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
mod chrome;
mod downloads;
mod handler;
mod history;
mod icons;
mod privacy;
mod session;
mod win;

use cef::*;
use std::sync::{Arc, Mutex};

#[cfg(not(all(feature = "sandbox", target_os = "windows")))]
fn main() -> anyhow::Result<()> {
    win::load_cef();

    let args = cef::args::Args::new();
    let Some(cmd_line) = args.as_cmd_line() else {
        anyhow::bail!("Impossible de parser les arguments de la ligne de commande");
    };

    let switch = CefString::from("type");
    let is_browser_process = cmd_line.has_switch(Some(&switch)) != 1;

    // Laisse CEF démarrer les processus enfants (helper).
    let ret = execute_process(Some(args.as_main_args()), None, std::ptr::null_mut());

    if !is_browser_process {
        println!("[faraday] processus helper");
        assert!(ret >= 0, "impossible d'exécuter un processus non-browser");
        return Ok(());
    }
    println!("[faraday] processus browser : initialisation");
    assert_eq!(ret, -1, "impossible d'exécuter le processus browser");

    // Taille de la zone de rendu OSR, partagée par tous les onglets.
    let view_size: handler::ViewSize = Arc::new(Mutex::new((0, 0)));

    let mut app = app::FaradayApp::new();

    // OSR : rendu hors-écran + pompe de messages externe (pour l'UI egui).
    let settings = Settings {
        windowless_rendering_enabled: true as _,
        external_message_pump: true as _,
        no_sandbox: !cfg!(feature = "sandbox") as _,
        ..Default::default()
    };
    assert_eq!(
        initialize(
            Some(args.as_main_args()),
            Some(&settings),
            Some(&mut app),
            std::ptr::null_mut(),
        ),
        1,
        "échec de l'initialisation CEF"
    );

    // UI chrome (egui) + page OSR, puis arrêt propre de CEF.
    let result = chrome::run(view_size);
    cef::shutdown();
    result
}

#[cfg(all(feature = "sandbox", target_os = "windows"))]
fn main() -> anyhow::Result<()> {
    anyhow::bail!("Le mode sandbox Windows nécessite bootstrap.exe — voir README");
}
