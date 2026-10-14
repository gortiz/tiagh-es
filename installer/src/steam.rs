//! Finds the game through Steam's library list, and the game's PCK inside a folder.

use std::fs;
use std::path::{Path, PathBuf};

const APP_ID: &str = "3641000";
const DEFAULT_INSTALL_DIR: &str = "The Incident at Galley House";
const LINUX_PCK: &str = "galleyhouse.pck";

/// Steam installation folders that exist on this machine.
fn steam_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    #[cfg(windows)]
    {
        use winreg::RegKey;
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
        let lookups = [
            (HKEY_CURRENT_USER, r"Software\Valve\Steam", "SteamPath"),
            (
                HKEY_LOCAL_MACHINE,
                r"SOFTWARE\WOW6432Node\Valve\Steam",
                "InstallPath",
            ),
            (HKEY_LOCAL_MACHINE, r"SOFTWARE\Valve\Steam", "InstallPath"),
        ];
        for (hive, key, value) in lookups {
            if let Ok(path) = RegKey::predef(hive)
                .open_subkey(key)
                .and_then(|k| k.get_value::<String, _>(value))
            {
                roots.push(PathBuf::from(path));
            }
        }
        for var in ["ProgramFiles(x86)", "ProgramFiles"] {
            if let Some(dir) = std::env::var_os(var) {
                roots.push(PathBuf::from(dir).join("Steam"));
            }
        }
    }

    #[cfg(not(windows))]
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        let candidates: &[&str] = if cfg!(target_os = "macos") {
            &["Library/Application Support/Steam"]
        } else {
            &[
                ".local/share/Steam",
                ".steam/steam",
                ".steam/root",
                ".var/app/com.valvesoftware.Steam/.local/share/Steam",
                ".var/app/com.valvesoftware.Steam/data/Steam",
                "snap/steam/common/.local/share/Steam",
            ]
        };
        roots.extend(candidates.iter().map(|c| home.join(c)));
    }

    dedup_existing(roots)
}

fn dedup_existing(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = Vec::new();
    let mut out = Vec::new();
    for p in paths {
        if let Ok(canonical) = fs::canonicalize(&p)
            && !seen.contains(&canonical)
        {
            seen.push(canonical);
            out.push(p);
        }
    }
    out
}

/// Returns the quoted strings of a Valve KeyValues (VDF/ACF) text, with escapes removed.
fn vdf_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '"' {
            continue;
        }
        let mut token = String::new();
        while let Some(c) = chars.next() {
            match c {
                '"' => break,
                '\\' => {
                    if let Some(next) = chars.next() {
                        token.push(next);
                    }
                }
                _ => token.push(c),
            }
        }
        tokens.push(token);
    }
    tokens
}

/// Values that follow `key` in a VDF/ACF text.
fn vdf_values(text: &str, key: &str) -> Vec<String> {
    let tokens = vdf_tokens(text);
    tokens
        .windows(2)
        .filter(|w| w[0].eq_ignore_ascii_case(key))
        .map(|w| w[1].clone())
        .collect()
}

fn library_folders(root: &Path) -> Vec<PathBuf> {
    let mut libs = vec![root.to_path_buf()];
    for vdf in ["steamapps/libraryfolders.vdf", "config/libraryfolders.vdf"] {
        if let Ok(text) = fs::read_to_string(root.join(vdf)) {
            libs.extend(vdf_values(&text, "path").into_iter().map(PathBuf::from));
        }
    }
    libs
}

/// Game folders that Steam knows about, from every library on this machine.
pub fn find_game_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let libs: Vec<PathBuf> = steam_roots()
        .iter()
        .flat_map(|r| library_folders(r))
        .collect();
    for lib in dedup_existing(libs) {
        let steamapps = lib.join("steamapps");
        let manifest = steamapps.join(format!("appmanifest_{APP_ID}.acf"));
        let install_dir = fs::read_to_string(&manifest)
            .ok()
            .and_then(|t| vdf_values(&t, "installdir").into_iter().next())
            .unwrap_or_else(|| DEFAULT_INSTALL_DIR.to_string());
        dirs.push(steamapps.join("common").join(install_dir));
    }
    dedup_existing(dirs)
}

fn pck_files_in(dir: &Path) -> Vec<PathBuf> {
    let Ok(read) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = read
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("pck")))
        .collect();
    out.sort();
    out
}

/// Finds the game's PCK from a path the user gives: the PCK itself, the game folder,
/// the macOS `.app` bundle, or the bundle's `Resources` folder.
pub fn find_pck(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        return path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("pck"))
            .then(|| path.to_path_buf());
    }
    if !path.is_dir() {
        return None;
    }
    let linux = path.join(LINUX_PCK);
    if linux.is_file() {
        return Some(linux);
    }
    let mut bundles = vec![path.to_path_buf()];
    if let Ok(read) = fs::read_dir(path) {
        bundles.extend(read.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| {
            p.is_dir() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("app"))
        }));
    }
    for bundle in &bundles {
        if let Some(pck) = pck_files_in(&bundle.join("Contents/Resources"))
            .into_iter()
            .next()
        {
            return Some(pck);
        }
    }
    pck_files_in(path).into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_library_paths_with_escapes() {
        let text = r#""libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
		"apps" { "3641000" "0" }
	}
	"1"
	{
		"path"		"/mnt/games/SteamLibrary"
	}
}"#;
        assert_eq!(
            vdf_values(text, "path"),
            vec![
                r"C:\Program Files (x86)\Steam".to_string(),
                "/mnt/games/SteamLibrary".to_string()
            ]
        );
    }

    #[test]
    fn finds_pck_in_mac_bundle() {
        let dir =
            std::env::temp_dir().join(format!("galley-house-es-steam-{}", std::process::id()));
        let res = dir.join("galleyhouse.app/Contents/Resources");
        fs::create_dir_all(&res).unwrap();
        fs::write(res.join("The Incident at Galley House.pck"), b"x").unwrap();
        assert_eq!(
            find_pck(&dir),
            Some(res.join("The Incident at Galley House.pck"))
        );
        assert_eq!(
            find_pck(&dir.join("galleyhouse.app")),
            Some(res.join("The Incident at Galley House.pck"))
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
