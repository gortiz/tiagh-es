//! Our changes to the game's scripts, applied at install time.
//!
//! The repository holds no game code. The installer reads each compiled script from the
//! player's own copy of the game, turns it into source text (`gdscript::detokenize`),
//! applies these edits, and writes the result as a text script. Godot compiles it when
//! the game starts. The `find` texts match the output of `detokenize`; run the installer
//! with `--extraer-scripts` to see that output.

use crate::gdscript;

pub enum Edit {
    /// Replace the only occurrence of `find`.
    Replace {
        find: &'static str,
        with: &'static str,
    },
    /// Add top-level code at the end of the script.
    Append(&'static str),
}

pub struct ScriptPatch {
    /// The compiled script in the PCK.
    pub compiled: &'static str,
    /// The path the game loads. Its `.remap` file points to `compiled`.
    pub source: &'static str,
    pub edits: &'static [Edit],
}

impl ScriptPatch {
    pub fn remap(&self) -> String {
        format!("{}.remap", self.source)
    }
}

pub const PATCHES: &[ScriptPatch] = &[
    ScriptPatch {
        compiled: "scripts/game-config.gdc",
        source: "scripts/game-config.gd",
        edits: &[
            // Add Spanish to the language menu.
            Edit::Replace {
                find: r#"5: &"Deutsch""#,
                with: r#"5: &"Deutsch", 6: &"Español""#,
            },
            Edit::Replace {
                find: r#"5: &"de""#,
                with: r#"5: &"de", 6: &"es""#,
            },
            // Register the Spanish translation when the GameConfig autoload is created.
            Edit::Append(
                r#"
var _tiagh_es_translation_loaded := _tiagh_es_load_translation()

func _tiagh_es_load_translation() -> bool:
    var translation := load("res://text/galley.es.translation") as Translation
    if translation == null:
        return false
    TranslationServer.add_translation(translation)
    return true
"#,
            ),
        ],
    },
    ScriptPatch {
        compiled: "scripts/lang-consts.gdc",
        source: "scripts/lang-consts.gd",
        edits: &[
            // The message shown after the player changes the language.
            Edit::Replace {
                find: "const LANG_WARNING_BY_LOCALE: Dictionary[StringName, StringName] = {",
                with: "const LANG_WARNING_BY_LOCALE: Dictionary[StringName, StringName] = {\n    \
                       &\"es\": &\"Por favor, reinicia el juego para aplicar el nuevo idioma.\",",
            },
        ],
    },
];

/// Returns the patched source text of a compiled script.
pub fn apply(patch: &ScriptPatch, compiled: &[u8]) -> Result<String, String> {
    let mut source =
        gdscript::detokenize(compiled).map_err(|e| format!("{}: {e}", patch.compiled))?;
    for edit in patch.edits {
        match edit {
            Edit::Replace { find, with } => {
                let count = source.matches(find).count();
                if count != 1 {
                    return Err(format!(
                        "Esta versión del juego no es compatible con la traducción: \
                         se esperaba encontrar una vez «{find}» en {}, y aparece {count} veces. \
                         Busca una versión más reciente del instalador.",
                        patch.source
                    ));
                }
                source = source.replacen(find, with, 1);
            }
            Edit::Append(code) => source.push_str(code),
        }
    }
    Ok(source)
}
