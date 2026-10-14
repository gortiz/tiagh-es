//! Installs and removes the Spanish translation in the game's PCK.
//!
//! The Spanish files are the translation (embedded in this installer), the Spanish
//! resource remaps (copies of the English ones), and the patched game scripts as source
//! text (see `scripts`), which replace the compiled ones through their `.remap` files.
//!
//! The installer keeps no backup file. On install, it stores the original directory block
//! inside the PCK as `ORIGINAL_DIRECTORY`, and the original directory offset as
//! `DATA_START`. The original payloads stay untouched before that offset, so writing the
//! original block back there and truncating gives the original file, byte for byte.

use crate::pck::{self, Entry, Pck, Tail};
use crate::scripts;

pub const TRANSLATION: &str = "text/galley.es.translation";
const TEXTURES_REMAP_EN: &str = "resources/localized-textures-en.tres.remap";
const TEXTURES_REMAP_ES: &str = "resources/localized-textures-es.tres.remap";
const SEARCH_REMAP_EN: &str = "db/search-index-en.tres.remap";
const SEARCH_REMAP_ES: &str = "db/search-index-es.tres.remap";
const ORIGINAL_DIRECTORY: &str = "tiagh-es/original-directory.bin";
const DATA_START: &str = "tiagh-es/data-start.bin";

const TRANSLATION_DATA: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../build/galley.es.translation"
));
pub enum State {
    NotInstalled,
    /// Installed by this installer. `up_to_date` is false when the installed files differ
    /// from the ones embedded in this installer.
    Installed {
        up_to_date: bool,
    },
    /// Spanish files exist, but this installer did not write them.
    Foreign,
}

pub enum Outcome {
    Installed,
    Updated,
    AlreadyUpToDate,
    Uninstalled,
}

pub type Result<T> = std::result::Result<T, String>;

pub fn state(pck: &Pck) -> Result<State> {
    if pck.find(ORIGINAL_DIRECTORY).is_none() {
        let foreign = pck.find(TRANSLATION).is_some() || pck.find(TEXTURES_REMAP_ES).is_some();
        return Ok(if foreign {
            State::Foreign
        } else {
            State::NotInstalled
        });
    }
    let original = original_entries(pck)?;
    let payloads = payloads(pck, &original)?;
    let written = payloads.iter().all(|(path, data)| {
        pck.find(path)
            .is_some_and(|e| e.size == data.len() as u64 && e.md5 == pck::md5(data))
    });
    let removed = removals().iter().all(|path| pck.find(path).is_none());
    let up_to_date = written && removed;
    Ok(State::Installed { up_to_date })
}

fn original_directory_bytes(pck: &Pck) -> Result<Vec<u8>> {
    let entry = pck
        .find(ORIGINAL_DIRECTORY)
        .ok_or("Falta el directorio original en el PCK.")?;
    let raw = pck.read_entry(entry).map_err(|e| e.to_string())?;
    if pck::md5(&raw) != entry.md5 {
        return Err("La copia del directorio original está dañada.".into());
    }
    Ok(raw)
}

fn original_entries(pck: &Pck) -> Result<Vec<Entry>> {
    pck::parse_directory(&original_directory_bytes(pck)?).map_err(|e| e.to_string())
}

/// Where the original data ends: the original directory offset. Stored as a little-endian
/// u64 in `DATA_START`, because our files can sit anywhere after it (see `Pck::commit`).
fn data_start(pck: &Pck) -> Result<u64> {
    let Some(entry) = pck.find(DATA_START) else {
        return Ok(pck.dir_offset);
    };
    let raw = pck.read_entry(entry).map_err(|e| e.to_string())?;
    let bytes: [u8; 8] = raw
        .try_into()
        .map_err(|_| "El marcador de instalación está dañado.")?;
    Ok(u64::from_le_bytes(bytes))
}

/// The `.remap` files of the patched scripts. Without them, the game loads our source
/// text instead of the compiled script.
fn removals() -> Vec<String> {
    scripts::PATCHES.iter().map(|p| p.remap()).collect()
}

/// The files to write. The remaps for Spanish reuse the English resources, whose exported
/// paths differ between platforms, so they come from the PCK itself.
fn payloads(pck: &Pck, original: &[Entry]) -> Result<Vec<(String, Vec<u8>)>> {
    let read = |path: &str| -> Result<Vec<u8>> {
        let entry = original
            .iter()
            .find(|e| e.path == path)
            .ok_or(format!("El juego no contiene {path}."))?;
        pck.read_entry(entry).map_err(|e| e.to_string())
    };
    let mut out = vec![
        (TRANSLATION.into(), TRANSLATION_DATA.to_vec()),
        (TEXTURES_REMAP_ES.into(), read(TEXTURES_REMAP_EN)?),
        (SEARCH_REMAP_ES.into(), read(SEARCH_REMAP_EN)?),
    ];
    for patch in scripts::PATCHES {
        let source = scripts::apply(patch, &read(patch.compiled)?)?;
        out.push((patch.source.into(), source.into_bytes()));
    }
    Ok(out)
}

pub fn install(pck: &Pck) -> Result<Outcome> {
    let (original, raw_dir, outcome) = match state(pck)? {
        State::Foreign => {
            return Err("El juego ya tiene archivos en español que no ha instalado este programa. \
                 Restaura el juego con «Verificar integridad de los archivos» en Steam y vuelve a intentarlo."
                .into());
        }
        State::Installed { up_to_date: true } => return Ok(Outcome::AlreadyUpToDate),
        State::Installed { up_to_date: false } => {
            let raw = original_directory_bytes(pck)?;
            (
                pck::parse_directory(&raw).map_err(|e| e.to_string())?,
                raw,
                Outcome::Updated,
            )
        }
        State::NotInstalled => {
            let raw = pck.read_directory_bytes().map_err(|e| e.to_string())?;
            (pck.entries.clone(), raw, Outcome::Installed)
        }
    };
    let start = data_start(pck)?;
    let mut payloads = payloads(pck, &original)?;
    payloads.push((ORIGINAL_DIRECTORY.into(), raw_dir));
    payloads.push((DATA_START.into(), start.to_le_bytes().to_vec()));
    pck.commit(
        start,
        &Tail::Patch {
            base: &original,
            payloads: &payloads,
            remove: &removals(),
        },
    )
    .map_err(crate::macos::write_error)?;
    Ok(outcome)
}

pub fn uninstall(pck: &Pck) -> Result<Outcome> {
    match state(pck)? {
        State::NotInstalled => Err("La traducción no está instalada.".into()),
        State::Foreign => Err("Este programa no instaló los archivos en español. \
             Usa «Verificar integridad de los archivos» en Steam para restaurar el juego."
            .into()),
        State::Installed { .. } => {
            let raw = original_directory_bytes(pck)?;
            pck.commit(data_start(pck)?, &Tail::RawDirectory(&raw))
                .map_err(crate::macos::write_error)?;
            Ok(Outcome::Uninstalled)
        }
    }
}
