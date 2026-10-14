mod gdscript;
mod macos;
mod patch;
mod pck;
mod scripts;
mod steam;

use std::io::{self, BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};

const TITLE: &str = "The Incident at Galley House — Traducción al español";

const USAGE: &str = "\
Uso: galley-house-es [OPCIONES] [RUTA]

RUTA puede ser la carpeta del juego, galleyhouse.app (macOS) o el archivo .pck.
Sin RUTA, el instalador busca el juego en las bibliotecas de Steam.

Opciones:
  --desinstalar   Restaura los archivos originales del juego.
  --si            No pide confirmación.
  --extraer-scripts CARPETA
                  Para desarrolladores: guarda como texto los scripts del juego.
  --ayuda         Muestra esta ayuda.";

struct Args {
    path: Option<PathBuf>,
    uninstall: bool,
    yes: bool,
    extract_scripts: Option<PathBuf>,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        path: None,
        uninstall: false,
        yes: false,
        extract_scripts: None,
    };
    let mut argv = std::env::args_os().skip(1);
    while let Some(arg) = argv.next() {
        match arg.to_str() {
            Some("--desinstalar" | "--uninstall") => args.uninstall = true,
            Some("--si" | "--yes" | "-y") => args.yes = true,
            Some("--extraer-scripts") => {
                let dir = argv
                    .next()
                    .ok_or(format!("Falta la carpeta de destino.\n\n{USAGE}"))?;
                args.extract_scripts = Some(PathBuf::from(dir));
            }
            Some("--ayuda" | "--help" | "-h") => return Err(USAGE.into()),
            Some(s) if s.starts_with('-') => {
                return Err(format!("Opción desconocida: {s}\n\n{USAGE}"));
            }
            _ if args.path.is_some() => return Err(format!("Sobra un argumento.\n\n{USAGE}")),
            _ => args.path = Some(PathBuf::from(arg)),
        }
    }
    Ok(args)
}

/// Talks to the user through the terminal, or through dialogs when there is no terminal
/// (for example after a double click in a Linux file manager).
struct Ui {
    gui: bool,
}

impl Ui {
    fn info(&self, msg: &str) {
        if self.gui {
            MessageDialog::new()
                .set_title(TITLE)
                .set_description(msg)
                .set_level(MessageLevel::Info)
                .show();
        } else {
            println!("{msg}");
        }
    }

    fn error(&self, msg: &str) {
        if self.gui {
            MessageDialog::new()
                .set_title(TITLE)
                .set_description(msg)
                .set_level(MessageLevel::Error)
                .show();
        } else {
            eprintln!("Error: {msg}");
        }
    }

    fn confirm(&self, msg: &str) -> bool {
        if self.gui {
            let result = MessageDialog::new()
                .set_title(TITLE)
                .set_description(msg)
                .set_buttons(MessageButtons::YesNo)
                .show();
            return result == MessageDialogResult::Yes;
        }
        print!("{msg} [S/n] ");
        let answer = read_line().unwrap_or_default().to_lowercase();
        answer.is_empty() || answer.starts_with('s') || answer.starts_with('y')
    }

    /// Asks for the game folder: a folder dialog first, then the terminal.
    fn ask_game_path(&self) -> Option<PathBuf> {
        if let Some(dir) = FileDialog::new()
            .set_title("Elige la carpeta de The Incident at Galley House")
            .pick_folder()
        {
            return Some(dir);
        }
        if self.gui {
            return None;
        }
        println!("Introduce la ruta de la carpeta del juego (puedes arrastrarla aquí):");
        print!("> ");
        let line = read_line()?;
        let cleaned = line
            .trim()
            .trim_matches(|c| c == '"' || c == '\'')
            .replace("\\ ", " ");
        (!cleaned.is_empty()).then(|| PathBuf::from(cleaned))
    }
}

fn read_line() -> Option<String> {
    io::stdout().flush().ok()?;
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line).ok()?;
    Some(line.trim().to_string())
}

fn locate_pck(ui: &Ui, given: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(path) = given {
        return steam::find_pck(path)
            .ok_or(format!("No se encontró el juego en {}.", path.display()));
    }
    let found: Vec<PathBuf> = steam::find_game_dirs()
        .iter()
        .filter_map(|d| steam::find_pck(d))
        .collect();
    if let Some(pck) = found.into_iter().next() {
        return Ok(pck);
    }
    ui.info("No se encontró el juego en las bibliotecas de Steam. Elige la carpeta del juego.");
    let path = ui.ask_game_path().ok_or("No se eligió ninguna carpeta.")?;
    steam::find_pck(&path).ok_or(format!("No se encontró el juego en {}.", path.display()))
}

/// Writes every compiled script of the game as source text, for writing script patches.
fn extract_scripts(pck: &pck::Pck, dir: &Path) -> Result<usize, String> {
    let mut count = 0;
    for entry in pck.entries.iter().filter(|e| e.path.ends_with(".gdc")) {
        let gdc = pck.read_entry(entry).map_err(|e| e.to_string())?;
        let source = gdscript::detokenize(&gdc).map_err(|e| format!("{}: {e}", entry.path))?;
        let out = dir.join(entry.path.trim_end_matches('c'));
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&out, source).map_err(|e| format!("{}: {e}", out.display()))?;
        count += 1;
    }
    Ok(count)
}

fn run(ui: &Ui, args: &Args) -> Result<(), String> {
    let pck_path = locate_pck(ui, args.path.as_deref())?;
    let pck = pck::Pck::open(&pck_path)
        .map_err(|e| format!("No se pudo leer {}: {e}", pck_path.display()))?;

    if let Some(dir) = &args.extract_scripts {
        let count = extract_scripts(&pck, dir)?;
        ui.info(&format!("{count} scripts guardados en {}.", dir.display()));
        return Ok(());
    }

    let state = patch::state(&pck)?;
    let action = match (&state, args.uninstall) {
        (patch::State::Installed { up_to_date: true }, false) => {
            let mut msg = format!(
                "La traducción ya está instalada y al día en:\n{}",
                pck_path.display()
            );
            if macos::fix_signature(&pck_path)? {
                msg.push_str(macos::RESIGNED_NOTE);
            }
            ui.info(&msg);
            return Ok(());
        }
        (patch::State::Installed { up_to_date: false }, false) => "actualizar la traducción en",
        (_, false) => "instalar la traducción en",
        (_, true) => "desinstalar la traducción de",
    };
    let question = format!(
        "Se va a {action}:\n{}\n\nCierra el juego antes de continuar. ¿Continuar?",
        pck_path.display()
    );
    if !args.yes && !ui.confirm(&question) {
        ui.info("Cancelado. No se ha modificado nada.");
        return Ok(());
    }

    let outcome = if args.uninstall {
        patch::uninstall(&pck)?
    } else {
        patch::install(&pck)?
    };
    let msg = match outcome {
        patch::Outcome::Installed => {
            "¡Traducción instalada! Abre el juego y elige «Español» en Options > Game > Language."
        }
        patch::Outcome::Updated => "¡Traducción actualizada!",
        patch::Outcome::AlreadyUpToDate => "La traducción ya estaba al día.",
        patch::Outcome::Uninstalled => {
            "Traducción desinstalada. Los datos del juego vuelven a estar como los originales."
        }
    };
    let mut msg = msg.to_string();
    if macos::fix_signature(&pck_path)? {
        msg.push_str(macos::RESIGNED_NOTE);
    }
    ui.info(&msg);
    Ok(())
}

fn main() -> ExitCode {
    // macOS and Windows open a terminal on double click. Linux file managers do not.
    let desktop =
        std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some();
    let no_terminal = !io::stdout().is_terminal() && !io::stdin().is_terminal();
    let ui = Ui {
        gui: cfg!(target_os = "linux") && desktop && no_terminal,
    };
    let args = match parse_args() {
        Ok(args) => args,
        Err(msg) => {
            ui.info(&msg);
            return ExitCode::from(2);
        }
    };
    // Started without arguments: probably a double click, so keep the window open at the end.
    let pause = !ui.gui && std::env::args_os().len() == 1;
    if !ui.gui {
        println!("{TITLE} (v{})\n", env!("CARGO_PKG_VERSION"));
    }
    let code = match run(&ui, &args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            ui.error(&msg);
            ExitCode::FAILURE
        }
    };
    if pause {
        print!("\nPulsa Intro para salir.");
        read_line();
    }
    code
}
