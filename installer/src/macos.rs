//! Keeps the macOS app bundle launchable after the PCK changes.
//!
//! On macOS the PCK lives inside `galleyhouse.app/Contents/Resources`, and the developer's
//! code signature seals every file in the bundle. A changed PCK breaks the seal, and macOS
//! then refuses to open the game ("the app is damaged"). We cannot re-create the
//! developer's signature, so we replace it with an ad-hoc one (`codesign --sign -`).
//! Steam's "Verify integrity of game files" restores the original signature.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The `.app` bundle that contains `pck`, if any.
fn app_bundle(pck: &Path) -> Option<PathBuf> {
    let resources = pck.parent()?;
    let contents = resources.parent()?;
    let app = contents.parent()?;
    let is_app = app
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("app"));
    (resources.file_name()? == "Resources" && contents.file_name()? == "Contents" && is_app)
        .then(|| app.to_path_buf())
}

/// Explains macOS App Management: once a signed app has been opened, macOS lets only
/// processes with that permission change it, and reports "Operation not permitted".
pub const APP_MANAGEMENT_HINT: &str = "macOS no deja modificar el juego. Abre Ajustes del Sistema > \
     Privacidad y seguridad > Gestión de apps, activa la app de terminal que estás usando, \
     ciérrala, ábrela de nuevo y vuelve a ejecutar el instalador.";

/// What the user must do after we sign the bundle again: Gatekeeper blocks ad-hoc
/// signed apps that Steam installed until the user approves them.
pub const RESIGNED_NOTE: &str = "\n\nSe ha vuelto a firmar la aplicación. La primera vez que abras el juego, \
     macOS lo bloqueará. Ve a Ajustes del Sistema > Privacidad y seguridad y, en la sección \
     Seguridad, pulsa «Abrir igualmente» junto al aviso de «galleyhouse». \
     Para recuperar la firma original, usa «Verificar integridad de los archivos» en Steam.";

/// Runs codesign and returns whether it succeeded, with its error output.
fn codesign(args: &[&str], app: &Path) -> Result<(bool, String), String> {
    Command::new("/usr/bin/codesign")
        .args(args)
        .arg(app)
        .output()
        .map(|out| {
            (
                out.status.success(),
                String::from_utf8_lossy(&out.stderr).trim().to_string(),
            )
        })
        .map_err(|e| format!("No se pudo ejecutar codesign: {e}"))
}

/// Signs the bundle again when its seal no longer matches its files.
/// Returns true if it signed the bundle.
pub fn fix_signature(pck: &Path) -> Result<bool, String> {
    if !cfg!(target_os = "macos") {
        return Ok(false);
    }
    let Some(app) = app_bundle(pck) else {
        return Ok(false);
    };
    if codesign(&["--verify", "--strict"], &app)?.0 {
        return Ok(false);
    }
    let (signed, stderr) = codesign(&["--force", "--sign", "-"], &app)?;
    if !signed {
        if stderr.contains("Operation not permitted") {
            return Err(APP_MANAGEMENT_HINT.into());
        }
        return Err(format!(
            "No se pudo volver a firmar {}:\n{stderr}",
            app.display()
        ));
    }
    let (valid, stderr) = codesign(&["--verify", "--strict"], &app)?;
    if !valid {
        return Err(format!(
            "La nueva firma de {} no es válida:\n{stderr}",
            app.display()
        ));
    }
    Ok(true)
}

/// Turns a failed write to the PCK into a message for the user.
pub fn write_error(e: crate::pck::PckError) -> String {
    match e {
        crate::pck::PckError::Io(io)
            if cfg!(target_os = "macos") && io.kind() == std::io::ErrorKind::PermissionDenied =>
        {
            APP_MANAGEMENT_HINT.into()
        }
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_bundle_only_for_resources_pck() {
        let pck =
            Path::new("/g/galleyhouse.app/Contents/Resources/The Incident at Galley House.pck");
        assert_eq!(app_bundle(pck), Some(PathBuf::from("/g/galleyhouse.app")));
        assert_eq!(app_bundle(Path::new("/g/galleyhouse.pck")), None);
    }
}
