//! Faraday — cœur de l'application.
//!
//! Ce crate est compilé à la fois :
//!   - en **binaire** (`faraday.exe`, `main.rs`) : mode classique, **sans** sandbox ;
//!   - en **DLL** (`faraday.dll`, cdylib) : mode **sandbox Chromium**, lancée par
//!     `bootstrap.exe` (fourni par CEF, renommé `faraday.exe`).
//!
//! Depuis Chromium M138, le sandbox Windows exige que l'application soit une DLL
//! exportant `RunWinMain`, exécutée par `bootstrap.exe` qui fournit le
//! `sandbox_info` (pointeur opaque créé par le bootstrap).
//! Voir : <https://chromiumembedded.github.io/cef/sandbox_setup>

mod app;
mod blocklist;
mod chrome;
mod downloads;
mod handler;
mod history;
mod icons;
mod privacy;
mod session;
mod win;

use cef::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Vrai si l'application tourne dans le **sandbox Chromium** (lancée par
/// `bootstrap.exe`, qui fournit le `sandbox_info`). Affiché dans l'interface.
pub static SANDBOX_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Point d'entrée commun à l'exécutable et à la DLL.
///
/// `windows_sandbox_info` :
///   - `null` → lancé directement (exe) : sandbox désactivé ;
///   - non-null → lancé par `bootstrap.exe` : sandbox activé.
pub fn browser_main(windows_sandbox_info: *mut u8) -> anyhow::Result<()> {
    let sandboxed = !windows_sandbox_info.is_null();
    SANDBOX_ACTIVE.store(sandboxed, Ordering::Relaxed);

    win::load_cef();

    let args = cef::args::Args::new();
    let Some(cmd_line) = args.as_cmd_line() else {
        anyhow::bail!("Impossible de parser les arguments de la ligne de commande");
    };

    let switch = CefString::from("type");
    let is_browser_process = cmd_line.has_switch(Some(&switch)) != 1;

    // Laisse CEF exécuter les processus enfants (renderer, GPU, ...).
    let ret = execute_process(Some(args.as_main_args()), None, windows_sandbox_info);

    if !is_browser_process {
        assert!(ret >= 0, "impossible d'exécuter un processus non-browser");
        return Ok(());
    }
    assert_eq!(ret, -1, "impossible d'exécuter le processus browser");

    // Trace locale (aucune donnée personnelle) : utile pour vérifier le mode.
    log_startup(sandboxed);

    // Taille de la zone de rendu OSR, partagée par tous les onglets.
    let view_size: handler::ViewSize = Arc::new(Mutex::new((0, 0)));

    let mut app = app::FaradayApp::new();

    // OSR : rendu hors-écran + pompe de messages externe (pour l'UI egui).
    // `no_sandbox` = true uniquement si aucun sandbox_info n'a été fourni.
    let settings = Settings {
        windowless_rendering_enabled: true as _,
        external_message_pump: true as _,
        no_sandbox: (!sandboxed) as _,
        ..Default::default()
    };
    assert_eq!(
        initialize(
            Some(args.as_main_args()),
            Some(&settings),
            Some(&mut app),
            windows_sandbox_info,
        ),
        1,
        "échec de l'initialisation CEF"
    );

    // UI chrome (egui) + page OSR, puis arrêt propre de CEF.
    let result = chrome::run(view_size);
    cef::shutdown();
    result
}

/// Écrit une trace minimale de démarrage dans `%APPDATA%\Faraday\startup.log`
/// (version + état du sandbox ; **aucune donnée personnelle**).
fn log_startup(sandboxed: bool) {
    let dir = privacy::appdata_dir();
    let _ = std::fs::create_dir_all(&dir);
    let content = format!(
        "version={}\nsandbox={}\n",
        env!("CARGO_PKG_VERSION"),
        if sandboxed { "active" } else { "desactive" }
    );
    let _ = std::fs::write(dir.join("startup.log"), content);
}

/// Export attendu par `bootstrap.exe` (application fenêtrée).
///
/// Doit impérativement s'appeler **`RunWinMain`** : le bootstrap le résout par
/// `GetProcAddress`. Le `sandbox_info` reçu est transmis à CEF
/// (`execute_process` / `initialize`).
#[cfg(target_os = "windows")]
#[no_mangle]
pub extern "system" fn RunWinMain(
    _hinstance: *mut std::ffi::c_void,
    _cmdline: *mut u16,
    _ncmdshow: i32,
    sandbox_info: *mut std::ffi::c_void,
    _version_info: *mut std::ffi::c_void,
) -> i32 {
    match browser_main(sandbox_info as *mut u8) {
        Ok(()) => 0,
        Err(e) => {
            // Pas de console dans le modèle bootstrap : on trace aussi l'erreur.
            let dir = privacy::appdata_dir();
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(dir.join("startup.log"), format!("erreur={e}\n"));
            1
        }
    }
}
