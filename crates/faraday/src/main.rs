//! Point d'entrée de Faraday (Phase 0).
//!
//! Démarre le moteur Chromium via CEF, applique la couche de confidentialité
//! puis affiche une fenêtre avec la page d'accueil.
//!
//! > ⚠️ Ce fichier suit l'exemple officiel `cefsimple` de `cef-rs`. Certains
//! > détails d'API (`browser_host_create_browser_sync`, params optionnels)
//! > seront ajustés lors du premier build une fois l'outillage installé.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod handler;
mod privacy;
mod win;

use cef::*;

#[allow(dead_code)]
fn run_main(main_args: &MainArgs, cmd_line: &CommandLine, sandbox_info: *mut u8) {
    let switch = CefString::from("type");
    let is_browser_process = cmd_line.has_switch(Some(&switch)) != 1;

    // Laisse CEF démarrer les processus enfants (helper).
    let ret = execute_process(Some(main_args), None, sandbox_info);

    if is_browser_process {
        println!("[faraday] processus browser : initialisation");
        assert_eq!(ret, -1, "impossible d'exécuter le processus browser");
    } else {
        println!("[faraday] processus helper");
        assert!(ret >= 0, "impossible d'exécuter un processus non-browser");
        return;
    }

    let mut app = app::FaradayApp::new();

    let settings = Settings {
        no_sandbox: !cfg!(feature = "sandbox") as _,
        ..Default::default()
    };
    assert_eq!(
        initialize(
            Some(main_args),
            Some(&settings),
            Some(&mut app),
            sandbox_info,
        ),
        1,
        "échec de l'initialisation CEF"
    );

    // Le navigateur est créé dans `on_context_initialized` (voir app.rs).
    // On lance alors la boucle de messages, puis on arrête proprement CEF.
    win::run_message_loop();

    cef::shutdown();
}

#[cfg(not(all(feature = "sandbox", target_os = "windows")))]
fn main() -> anyhow::Result<()> {
    win::load_cef();

    let args = cef::args::Args::new();
    let Some(cmd_line) = args.as_cmd_line() else {
        anyhow::bail!("Impossible de parser les arguments de la ligne de commande");
    };

    run_main(args.as_main_args(), &cmd_line, std::ptr::null_mut());
    Ok(())
}

#[cfg(all(feature = "sandbox", target_os = "windows"))]
fn main() -> anyhow::Result<()> {
    // Le mode sandbox sur Windows nécessite `bootstrap.exe` / `bootstrapc.exe`.
    anyhow::bail!("Le mode sandbox Windows nécessite bootstrap.exe — voir README");
}
