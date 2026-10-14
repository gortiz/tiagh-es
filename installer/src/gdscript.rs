//! Turns a Godot 4.6 binary GDScript (`.gdc`, tokenizer buffer version 101) back into
//! source text that Godot can compile.
//!
//! The buffer holds the token stream without whitespace or comments. For each token that
//! starts a logical line it also stores the line and the column. Godot's parser does not
//! use columns, except to compute indentation, so the text only needs the same tokens in
//! the same lines, with the same indentation. See `GDScriptTokenizerBuffer` in Godot's
//! `modules/gdscript/gdscript_tokenizer_buffer.cpp`.

use std::collections::HashMap;
use std::io::Read;

const MAGIC: &[u8; 4] = b"GDSC";
const TOKENIZER_VERSION: u32 = 101;
const TOKEN_BYTE_MASK: u8 = 0x80;
const TOKEN_BITS: u32 = 8;
const TOKEN_MASK: u32 = (1 << (TOKEN_BITS - 1)) - 1;

/// Token types, in the order of `GDScriptTokenizer::Token::Type`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
enum Kind {
    Empty,
    Annotation,
    Identifier,
    Literal,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    EqualEqual,
    BangEqual,
    And,
    Or,
    Not,
    AmpersandAmpersand,
    PipePipe,
    Bang,
    Ampersand,
    Pipe,
    Tilde,
    Caret,
    LessLess,
    GreaterGreater,
    Plus,
    Minus,
    Star,
    StarStar,
    Slash,
    Percent,
    Equal,
    PlusEqual,
    MinusEqual,
    StarEqual,
    StarStarEqual,
    SlashEqual,
    PercentEqual,
    LessLessEqual,
    GreaterGreaterEqual,
    AmpersandEqual,
    PipeEqual,
    CaretEqual,
    If,
    Elif,
    Else,
    For,
    While,
    Break,
    Continue,
    Pass,
    Return,
    Match,
    When,
    As,
    Assert,
    Await,
    Breakpoint,
    Class,
    ClassName,
    Const,
    Enum,
    Extends,
    Func,
    In,
    Is,
    Namespace,
    Preload,
    SelfKw,
    Signal,
    Static,
    Super,
    Trait,
    Var,
    Void,
    Yield,
    BracketOpen,
    BracketClose,
    BraceOpen,
    BraceClose,
    ParenthesisOpen,
    ParenthesisClose,
    Comma,
    Semicolon,
    Period,
    PeriodPeriod,
    PeriodPeriodPeriod,
    Colon,
    Dollar,
    ForwardArrow,
    Underscore,
    Newline,
    Indent,
    Dedent,
    ConstPi,
    ConstTau,
    ConstInf,
    ConstNan,
    VcsConflictMarker,
    Backtick,
    QuestionMark,
    Error,
    Eof,
}

const ALL_KINDS: &[Kind] = {
    use Kind::*;
    &[
        Empty,
        Annotation,
        Identifier,
        Literal,
        Less,
        LessEqual,
        Greater,
        GreaterEqual,
        EqualEqual,
        BangEqual,
        And,
        Or,
        Not,
        AmpersandAmpersand,
        PipePipe,
        Bang,
        Ampersand,
        Pipe,
        Tilde,
        Caret,
        LessLess,
        GreaterGreater,
        Plus,
        Minus,
        Star,
        StarStar,
        Slash,
        Percent,
        Equal,
        PlusEqual,
        MinusEqual,
        StarEqual,
        StarStarEqual,
        SlashEqual,
        PercentEqual,
        LessLessEqual,
        GreaterGreaterEqual,
        AmpersandEqual,
        PipeEqual,
        CaretEqual,
        If,
        Elif,
        Else,
        For,
        While,
        Break,
        Continue,
        Pass,
        Return,
        Match,
        When,
        As,
        Assert,
        Await,
        Breakpoint,
        Class,
        ClassName,
        Const,
        Enum,
        Extends,
        Func,
        In,
        Is,
        Namespace,
        Preload,
        SelfKw,
        Signal,
        Static,
        Super,
        Trait,
        Var,
        Void,
        Yield,
        BracketOpen,
        BracketClose,
        BraceOpen,
        BraceClose,
        ParenthesisOpen,
        ParenthesisClose,
        Comma,
        Semicolon,
        Period,
        PeriodPeriod,
        PeriodPeriodPeriod,
        Colon,
        Dollar,
        ForwardArrow,
        Underscore,
        Newline,
        Indent,
        Dedent,
        ConstPi,
        ConstTau,
        ConstInf,
        ConstNan,
        VcsConflictMarker,
        Backtick,
        QuestionMark,
        Error,
        Eof,
    ]
};

impl Kind {
    /// Source text of tokens that have a fixed spelling.
    fn text(self) -> Option<&'static str> {
        use Kind::*;
        Some(match self {
            Less => "<",
            LessEqual => "<=",
            Greater => ">",
            GreaterEqual => ">=",
            EqualEqual => "==",
            BangEqual => "!=",
            And => "and",
            Or => "or",
            Not => "not",
            AmpersandAmpersand => "&&",
            PipePipe => "||",
            Bang => "!",
            Ampersand => "&",
            Pipe => "|",
            Tilde => "~",
            Caret => "^",
            LessLess => "<<",
            GreaterGreater => ">>",
            Plus => "+",
            Minus => "-",
            Star => "*",
            StarStar => "**",
            Slash => "/",
            Percent => "%",
            Equal => "=",
            PlusEqual => "+=",
            MinusEqual => "-=",
            StarEqual => "*=",
            StarStarEqual => "**=",
            SlashEqual => "/=",
            PercentEqual => "%=",
            LessLessEqual => "<<=",
            GreaterGreaterEqual => ">>=",
            AmpersandEqual => "&=",
            PipeEqual => "|=",
            CaretEqual => "^=",
            If => "if",
            Elif => "elif",
            Else => "else",
            For => "for",
            While => "while",
            Break => "break",
            Continue => "continue",
            Pass => "pass",
            Return => "return",
            Match => "match",
            When => "when",
            As => "as",
            Assert => "assert",
            Await => "await",
            Breakpoint => "breakpoint",
            Class => "class",
            ClassName => "class_name",
            Const => "const",
            Enum => "enum",
            Extends => "extends",
            Func => "func",
            In => "in",
            Is => "is",
            Namespace => "namespace",
            Preload => "preload",
            SelfKw => "self",
            Signal => "signal",
            Static => "static",
            Super => "super",
            Trait => "trait",
            Var => "var",
            Void => "void",
            Yield => "yield",
            BracketOpen => "[",
            BracketClose => "]",
            BraceOpen => "{",
            BraceClose => "}",
            ParenthesisOpen => "(",
            ParenthesisClose => ")",
            Comma => ",",
            Semicolon => ";",
            Period => ".",
            PeriodPeriod => "..",
            PeriodPeriodPeriod => "...",
            Colon => ":",
            Dollar => "$",
            ForwardArrow => "->",
            Underscore => "_",
            ConstPi => "PI",
            ConstTau => "TAU",
            ConstInf => "INF",
            ConstNan => "NAN",
            QuestionMark => "?",
            _ => return None,
        })
    }

    /// Tokens after which a binary operator can follow (the end of an operand).
    fn ends_operand(self) -> bool {
        use Kind::*;
        matches!(
            self,
            Identifier
                | Literal
                | ParenthesisClose
                | BracketClose
                | BraceClose
                | SelfKw
                | ConstPi
                | ConstTau
                | ConstInf
                | ConstNan
                | Underscore
        )
    }
}

/// A literal constant, decoded from Godot's Variant encoding.
#[derive(Clone, Debug, PartialEq)]
enum Value {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    StringName(String),
    NodePath(String),
}

struct Token {
    kind: Kind,
    /// Identifier, annotation or constant index.
    index: usize,
}

pub type Result<T> = std::result::Result<T, String>;

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.pos.checked_add(n).filter(|&e| e <= self.buf.len());
        let end = end.ok_or("El script compilado está truncado.")?;
        let out = &self.buf[self.pos..end];
        self.pos = end;
        Ok(out)
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }

    /// A UTF-8 string with a u32 length, padded to 4 bytes.
    fn padded_string(&mut self) -> Result<String> {
        let len = self.u32()? as usize;
        let bytes = self.take(len)?;
        self.take((4 - len % 4) % 4)?;
        String::from_utf8(bytes.to_vec())
            .map_err(|_| "Texto no válido en el script compilado.".into())
    }
}

const VARIANT_FLAG_64: u32 = 1 << 16;

fn read_value(r: &mut Reader) -> Result<Value> {
    let header = r.u32()?;
    Ok(match header & 0xff {
        0 => Value::Nil,
        1 => Value::Bool(r.u32()? != 0),
        2 if header & VARIANT_FLAG_64 != 0 => Value::Int(r.u64()? as i64),
        2 => Value::Int(r.u32()? as i32 as i64),
        3 if header & VARIANT_FLAG_64 != 0 => Value::Float(f64::from_bits(r.u64()?)),
        3 => Value::Float(f32::from_bits(r.u32()?) as f64),
        4 => Value::String(r.padded_string()?),
        21 => Value::StringName(r.padded_string()?),
        22 => {
            let names = r.u32()?;
            if names & 0x8000_0000 == 0 {
                return Err("Formato antiguo de NodePath no soportado.".into());
            }
            let names = (names & 0x7fff_ffff) as usize;
            let mut subnames = r.u32()? as usize;
            let flags = r.u32()?;
            if flags & 2 != 0 {
                subnames += 1;
            }
            let mut parts = Vec::new();
            for _ in 0..names {
                parts.push(r.padded_string()?);
            }
            let mut path = parts.join("/");
            if flags & 1 != 0 {
                path.insert(0, '/');
            }
            for _ in 0..subnames {
                path.push(':');
                path.push_str(&r.padded_string()?);
            }
            Value::NodePath(path)
        }
        other => {
            return Err(format!(
                "Tipo de constante {other} no soportado en el script compilado."
            ));
        }
    })
}

fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\u{:04x}", c as u32))
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn value_text(v: &Value) -> Result<String> {
    Ok(match v {
        Value::Nil => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) if f.is_finite() => {
            // `{:?}` gives the shortest text that reads back as the same f64, always with
            // a `.` or an exponent, so GDScript reads it as a float.
            format!("{f:?}")
        }
        Value::Float(_) => return Err("Constante decimal no finita en el script compilado.".into()),
        Value::String(s) => quote(s),
        Value::StringName(s) => format!("&{}", quote(s)),
        Value::NodePath(s) => format!("^{}", quote(s)),
    })
}

struct Script {
    identifiers: Vec<String>,
    constants: Vec<Value>,
    /// Logical line starts: token index -> (line, column).
    line_starts: HashMap<usize, (u32, u32)>,
    tokens: Vec<Token>,
}

fn parse(gdc: &[u8]) -> Result<Script> {
    if gdc.len() < 12 || &gdc[0..4] != MAGIC {
        return Err("El archivo no es un script compilado de Godot.".into());
    }
    let version = u32::from_le_bytes(gdc[4..8].try_into().unwrap());
    if version != TOKENIZER_VERSION {
        return Err(format!(
            "Versión de script compilado {version} no soportada (se esperaba {TOKENIZER_VERSION})."
        ));
    }
    let size = u32::from_le_bytes(gdc[8..12].try_into().unwrap()) as usize;
    let contents = if size == 0 {
        gdc[12..].to_vec()
    } else {
        let mut decoder = ruzstd::decoding::StreamingDecoder::new(&gdc[12..])
            .map_err(|e| format!("No se pudo descomprimir el script: {e}"))?;
        let mut out = Vec::with_capacity(size);
        decoder
            .read_to_end(&mut out)
            .map_err(|e| format!("No se pudo descomprimir el script: {e}"))?;
        if out.len() != size {
            return Err("El script descomprimido no tiene el tamaño esperado.".into());
        }
        out
    };

    let mut r = Reader {
        buf: &contents,
        pos: 0,
    };
    let identifier_count = r.u32()? as usize;
    let constant_count = r.u32()? as usize;
    let line_count = r.u32()? as usize;
    let token_count = r.u32()? as usize;

    let mut identifiers = Vec::with_capacity(identifier_count);
    for _ in 0..identifier_count {
        let len = r.u32()? as usize;
        let raw = r.take(len * 4)?;
        let mut s = String::with_capacity(len);
        for chunk in raw.chunks_exact(4) {
            let bytes: Vec<u8> = chunk.iter().map(|b| b ^ 0xb6).collect();
            let code = u32::from_le_bytes(bytes.try_into().unwrap());
            s.push(char::from_u32(code).ok_or("Identificador no válido en el script compilado.")?);
        }
        identifiers.push(s);
    }

    let mut constants = Vec::with_capacity(constant_count);
    for _ in 0..constant_count {
        constants.push(read_value(&mut r)?);
    }

    let mut lines = Vec::with_capacity(line_count);
    for _ in 0..line_count {
        lines.push((r.u32()? as usize, r.u32()?));
    }
    let mut line_starts = HashMap::with_capacity(line_count);
    for (index, line) in lines {
        let (column_index, column) = (r.u32()? as usize, r.u32()?);
        if column_index != index {
            // Godot writes both maps from the same HashMap order, so the keys match.
            return Err("Las tablas de líneas y columnas del script no coinciden.".into());
        }
        line_starts.insert(index, (line, column));
    }

    let mut tokens = Vec::with_capacity(token_count);
    for _ in 0..token_count {
        let first = *contents
            .get(r.pos)
            .ok_or("El script compilado está truncado.")?;
        let raw = if first & TOKEN_BYTE_MASK != 0 {
            r.u32()? & !(TOKEN_BYTE_MASK as u32)
        } else {
            r.take(1)?[0] as u32
        };
        let _line = r.u32()?;
        let kind = *ALL_KINDS
            .get((raw & TOKEN_MASK) as usize)
            .ok_or("Tipo de token desconocido en el script.")?;
        tokens.push(Token {
            kind,
            index: (raw >> TOKEN_BITS) as usize,
        });
    }
    if r.pos != contents.len() {
        return Err("El script compilado tiene datos sobrantes.".into());
    }
    Ok(Script {
        identifiers,
        constants,
        line_starts,
        tokens,
    })
}

impl Script {
    fn token_text(&self, t: &Token) -> Result<String> {
        match t.kind {
            Kind::Identifier | Kind::Annotation => self
                .identifiers
                .get(t.index)
                .cloned()
                .ok_or("Índice de identificador no válido.".into()),
            Kind::Literal => value_text(
                self.constants
                    .get(t.index)
                    .ok_or("Índice de constante no válido.")?,
            ),
            kind => kind
                .text()
                .map(str::to_string)
                .ok_or(format!("Token {kind:?} inesperado en el script.")),
        }
    }

    fn to_source(&self) -> Result<String> {
        let mut out = String::new();
        let mut prev: Option<Kind> = None;
        let mut prev_unary = false;
        for (i, t) in self.tokens.iter().enumerate() {
            let text = self.token_text(t)?;
            if let Some(&(_, column)) = self.line_starts.get(&i).filter(|_| i > 0) {
                // Godot counts a tab as 4 columns. Spaces give the same indentation levels.
                out.push('\n');
                out.push_str(&" ".repeat(column.saturating_sub(1) as usize));
            } else if let Some(p) = prev
                && needs_space(p, prev_unary, t.kind) {
                    out.push(' ');
                }
            let unary = matches!(t.kind, Kind::Minus | Kind::Plus | Kind::Tilde | Kind::Bang)
                && !prev.is_some_and(Kind::ends_operand);
            out.push_str(&text);
            prev = Some(t.kind);
            prev_unary = unary;
        }
        out.push('\n');
        Ok(out)
    }
}

/// Whether to put a space between two tokens on the same line. Spaces never change the
/// meaning of GDScript; this only keeps the text readable and keeps tokens from merging.
fn needs_space(prev: Kind, prev_unary: bool, next: Kind) -> bool {
    use Kind::*;
    if prev == Literal && next == Period {
        // `1 .x`, not `1.x`, which would read as a float.
        return true;
    }
    if prev_unary || matches!(prev, ParenthesisOpen | BracketOpen | Period | Dollar) {
        return false;
    }
    if matches!(
        next,
        Comma | ParenthesisClose | BracketClose | Colon | Period
    ) {
        return false;
    }
    if matches!(next, ParenthesisOpen | BracketOpen) {
        return !matches!(
            prev,
            Identifier | Annotation | Preload | Super | ParenthesisClose | BracketClose
        );
    }
    true
}

/// Converts a binary GDScript into source text.
pub fn detokenize(gdc: &[u8]) -> Result<String> {
    parse(gdc)?.to_source()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spacing() {
        use Kind::*;
        assert!(!needs_space(Identifier, false, ParenthesisOpen));
        assert!(needs_space(If, false, ParenthesisOpen));
        assert!(!needs_space(Identifier, false, Colon));
        assert!(!needs_space(Minus, true, Literal));
        assert!(needs_space(Literal, false, Period));
    }

    #[test]
    fn literals() {
        assert_eq!(value_text(&Value::Float(1.0)).unwrap(), "1.0");
        assert_eq!(value_text(&Value::Float(0.1)).unwrap(), "0.1");
        assert_eq!(
            value_text(&Value::StringName("a\"b".into())).unwrap(),
            "&\"a\\\"b\""
        );
        assert_eq!(value_text(&Value::Nil).unwrap(), "null");
    }
}
