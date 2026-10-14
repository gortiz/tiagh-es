//! Reader and crash-safe in-place writer for Godot 4 PCK archives (pack format 3).
//!
//! Layout: a 40-byte header, the file payloads, and the directory at the end of the file.
//! The header stores the directory offset at byte 32, so a single 8-byte write switches
//! the archive from one complete directory to another.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use md5::{Digest, Md5};

const MAGIC: &[u8; 4] = b"GDPC";
const SUPPORTED_FORMAT: u32 = 3;
const HEADER_LEN: u64 = 40;
const DIR_OFFSET_POS: u64 = 32;
/// Pack flag: file offsets are relative to `file_base`. Every other pack flag is unsupported.
const PACK_REL_FILEBASE: u32 = 1 << 1;
const DATA_ALIGNMENT: u64 = 32;

#[derive(Clone, Debug)]
pub struct Entry {
    pub path: String,
    /// Offset relative to `file_base`.
    pub offset: u64,
    pub size: u64,
    pub md5: [u8; 16],
    pub flags: u32,
}

pub struct Pck {
    pub path: PathBuf,
    pub file_base: u64,
    pub dir_offset: u64,
    pub file_len: u64,
    pub entries: Vec<Entry>,
}

#[derive(Debug)]
pub enum PckError {
    Io(io::Error),
    Invalid(String),
}

impl From<io::Error> for PckError {
    fn from(e: io::Error) -> Self {
        PckError::Io(e)
    }
}

impl std::fmt::Display for PckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PckError::Io(e) => write!(f, "{e}"),
            PckError::Invalid(msg) => write!(f, "{msg}"),
        }
    }
}

pub type Result<T> = std::result::Result<T, PckError>;

fn invalid<T>(msg: impl Into<String>) -> Result<T> {
    Err(PckError::Invalid(msg.into()))
}

fn u32_at(buf: &[u8], pos: usize) -> u32 {
    u32::from_le_bytes(buf[pos..pos + 4].try_into().unwrap())
}

fn u64_at(buf: &[u8], pos: usize) -> u64 {
    u64::from_le_bytes(buf[pos..pos + 8].try_into().unwrap())
}

pub fn md5(data: &[u8]) -> [u8; 16] {
    Md5::digest(data).into()
}

/// Parses a raw directory block (file count followed by the entries).
pub fn parse_directory(buf: &[u8]) -> Result<Vec<Entry>> {
    let truncated = || PckError::Invalid("El directorio del PCK está truncado.".into());
    if buf.len() < 4 {
        return Err(truncated());
    }
    let count = u32_at(buf, 0) as usize;
    let mut pos = 4;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        if pos + 4 > buf.len() {
            return Err(truncated());
        }
        let path_len = u32_at(buf, pos) as usize;
        pos += 4;
        if pos + path_len + 36 > buf.len() {
            return Err(truncated());
        }
        let raw_path = &buf[pos..pos + path_len];
        let end = raw_path.iter().position(|&b| b == 0).unwrap_or(path_len);
        let path = String::from_utf8_lossy(&raw_path[..end]).into_owned();
        pos += path_len;
        let offset = u64_at(buf, pos);
        let size = u64_at(buf, pos + 8);
        let md5: [u8; 16] = buf[pos + 16..pos + 32].try_into().unwrap();
        let flags = u32_at(buf, pos + 32);
        pos += 36;
        entries.push(Entry {
            path,
            offset,
            size,
            md5,
            flags,
        });
    }
    if pos != buf.len() {
        return invalid("El PCK tiene datos inesperados después del directorio.");
    }
    Ok(entries)
}

/// Serializes a directory block the way Godot's `PCKPacker` does (paths padded to 4 bytes).
pub fn serialize_directory(entries: &[Entry]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for e in entries {
        let mut path = e.path.as_bytes().to_vec();
        path.push(0);
        while path.len() % 4 != 0 {
            path.push(0);
        }
        out.extend_from_slice(&(path.len() as u32).to_le_bytes());
        out.extend_from_slice(&path);
        out.extend_from_slice(&e.offset.to_le_bytes());
        out.extend_from_slice(&e.size.to_le_bytes());
        out.extend_from_slice(&e.md5);
        out.extend_from_slice(&e.flags.to_le_bytes());
    }
    out
}

/// What a commit writes after `data_start`: new payloads followed by a directory.
pub enum Tail<'a> {
    /// Append `payloads`, replace or add their entries in `base`, and drop the entries
    /// in `remove`.
    Patch {
        base: &'a [Entry],
        payloads: &'a [(String, Vec<u8>)],
        remove: &'a [String],
    },
    /// Write this exact directory block (used to restore the original bytes).
    RawDirectory(&'a [u8]),
}

impl Pck {
    pub fn open(path: &Path) -> Result<Pck> {
        let mut f = File::open(path)?;
        let file_len = f.metadata()?.len();
        let mut header = [0u8; HEADER_LEN as usize];
        if file_len < HEADER_LEN {
            return invalid("El archivo es demasiado pequeño para ser un PCK.");
        }
        f.read_exact(&mut header)?;
        if &header[0..4] != MAGIC {
            return invalid("El archivo no es un PCK de Godot (falta la firma GDPC).");
        }
        let format = u32_at(&header, 4);
        if format != SUPPORTED_FORMAT {
            return invalid(format!(
                "Formato de PCK {format} no soportado (se esperaba {SUPPORTED_FORMAT})."
            ));
        }
        let pack_flags = u32_at(&header, 20);
        if pack_flags != PACK_REL_FILEBASE {
            return invalid(format!(
                "Opciones de PCK no soportadas (flags = {pack_flags})."
            ));
        }
        let file_base = u64_at(&header, 24);
        let dir_offset = u64_at(&header, DIR_OFFSET_POS as usize);
        if dir_offset < HEADER_LEN || dir_offset >= file_len {
            return invalid("El desplazamiento del directorio del PCK no es válido.");
        }
        let mut dir = vec![0u8; (file_len - dir_offset) as usize];
        f.seek(SeekFrom::Start(dir_offset))?;
        f.read_exact(&mut dir)?;
        let entries = parse_directory(&dir)?;
        Ok(Pck {
            path: path.to_path_buf(),
            file_base,
            dir_offset,
            file_len,
            entries,
        })
    }

    pub fn find(&self, path: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.path == path)
    }

    pub fn read_entry(&self, entry: &Entry) -> Result<Vec<u8>> {
        let mut f = File::open(&self.path)?;
        f.seek(SeekFrom::Start(self.file_base + entry.offset))?;
        let mut buf = vec![0u8; entry.size as usize];
        f.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// Reads the raw directory block that the header currently points to.
    pub fn read_directory_bytes(&self) -> Result<Vec<u8>> {
        let mut f = File::open(&self.path)?;
        f.seek(SeekFrom::Start(self.dir_offset))?;
        let mut buf = vec![0u8; (self.file_len - self.dir_offset) as usize];
        f.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// Builds the bytes of a tail that starts at absolute file position `start`.
    /// Returns the bytes and the absolute offset of the directory inside them.
    fn build_tail(&self, start: u64, tail: &Tail) -> Result<(Vec<u8>, u64)> {
        match tail {
            Tail::RawDirectory(dir) => Ok((dir.to_vec(), start)),
            Tail::Patch {
                base,
                payloads,
                remove,
            } => {
                let mut entries: Vec<Entry> = base
                    .iter()
                    .filter(|e| !remove.contains(&e.path))
                    .cloned()
                    .collect();
                let mut buf = Vec::new();
                for (path, data) in payloads.iter() {
                    while !(start + buf.len() as u64).is_multiple_of(DATA_ALIGNMENT) {
                        buf.push(0);
                    }
                    let abs = start + buf.len() as u64;
                    if abs < self.file_base {
                        return invalid("El PCK tiene una base de datos inesperada.");
                    }
                    let entry = Entry {
                        path: path.clone(),
                        offset: abs - self.file_base,
                        size: data.len() as u64,
                        md5: md5(data),
                        flags: 0,
                    };
                    match entries.iter_mut().find(|e| e.path == *path) {
                        Some(existing) => *existing = entry,
                        None => entries.push(entry),
                    }
                    buf.extend_from_slice(data);
                }
                let dir_abs = start + buf.len() as u64;
                buf.extend_from_slice(&serialize_directory(&entries));
                Ok((buf, dir_abs))
            }
        }
    }

    /// Replaces everything after `data_start` with `tail` without leaving the file in a
    /// broken state at any point:
    ///
    /// 1. Write the tail at the current end of the file, then point the header at it.
    /// 2. If the tail also fits in the region between `data_start` and the old end of the
    ///    file, write it again there, point the header at that copy, and truncate.
    ///
    /// The header always points to a complete directory. If step 2 does not fit, the old
    /// region stays in the file as unused bytes until the next commit.
    pub fn commit(&self, data_start: u64, tail: &Tail) -> Result<()> {
        if data_start < HEADER_LEN || data_start > self.file_len {
            return invalid("Punto de escritura no válido en el PCK.");
        }
        let mut f = OpenOptions::new().read(true).write(true).open(&self.path)?;

        let old_end = self.file_len;
        let (bytes, dir_abs) = self.build_tail(old_end, tail)?;
        write_at(&mut f, old_end, &bytes)?;
        f.sync_all()?;
        write_at(&mut f, DIR_OFFSET_POS, &dir_abs.to_le_bytes())?;
        f.sync_all()?;

        let (bytes, dir_abs) = self.build_tail(data_start, tail)?;
        if data_start + bytes.len() as u64 <= old_end {
            write_at(&mut f, data_start, &bytes)?;
            f.sync_all()?;
            write_at(&mut f, DIR_OFFSET_POS, &dir_abs.to_le_bytes())?;
            f.sync_all()?;
            f.set_len(data_start + bytes.len() as u64)?;
            f.sync_all()?;
        }
        Ok(())
    }
}

fn write_at(f: &mut File, pos: u64, data: &[u8]) -> io::Result<()> {
    f.seek(SeekFrom::Start(pos))?;
    f.write_all(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a small valid pack: header, two payloads, directory at the end.
    pub fn sample_pck(dir: &Path) -> PathBuf {
        let file_base = HEADER_LEN;
        let payloads: [(&str, &[u8]); 2] = [("a.txt", b"hello"), ("scripts/x.gdc", b"original")];
        let mut data = Vec::new();
        let mut entries = Vec::new();
        for (path, bytes) in payloads {
            entries.push(Entry {
                path: path.into(),
                offset: data.len() as u64,
                size: bytes.len() as u64,
                md5: md5(bytes),
                flags: 0,
            });
            data.extend_from_slice(bytes);
        }
        let dir_offset = file_base + data.len() as u64;
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        for v in [SUPPORTED_FORMAT, 4, 6, 2, PACK_REL_FILEBASE] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&file_base.to_le_bytes());
        out.extend_from_slice(&dir_offset.to_le_bytes());
        out.extend_from_slice(&data);
        out.extend_from_slice(&serialize_directory(&entries));
        let path = dir.join("game.pck");
        std::fs::write(&path, out).unwrap();
        path
    }

    pub fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("galley-house-es-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn patch_then_restore_is_byte_identical() {
        let dir = temp_dir("pck");
        let path = sample_pck(&dir);
        let original = std::fs::read(&path).unwrap();

        let pck = Pck::open(&path).unwrap();
        let raw_dir = pck.read_directory_bytes().unwrap();
        let payloads = vec![
            ("scripts/x.gdc".to_string(), b"patched!".to_vec()),
            ("text/new.translation".to_string(), vec![7u8; 100]),
        ];
        pck.commit(
            pck.dir_offset,
            &Tail::Patch {
                base: &pck.entries,
                payloads: &payloads,
                remove: &["a.txt".to_string()],
            },
        )
        .unwrap();

        let patched = Pck::open(&path).unwrap();
        assert_eq!(patched.entries.len(), 2);
        assert!(patched.find("a.txt").is_none());
        let x = patched.find("scripts/x.gdc").unwrap();
        assert_eq!(patched.read_entry(x).unwrap(), b"patched!");

        patched
            .commit(pck.dir_offset, &Tail::RawDirectory(&raw_dir))
            .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), original);
    }
}
