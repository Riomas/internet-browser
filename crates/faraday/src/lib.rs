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
mod bookmarks;
mod chrome;
mod downloads;
mod handler;
mod history;
mod icons;
mod privacy;
mod profiles;
mod session;
mod win;

use cef::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Vrai si l'application tourne dans le **sandbox Chromium** (lancée par
/// `bootstrap.exe`, qui fournit le `sandbox_info`). Affiché dans l'interface.
pub static SANDBOX_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Chemin du journal de démarrage (diagnostic local, hors profils, pour rester
/// consultable même si l'initialisation échoue).
fn trace_path() -> std::path::PathBuf {
    let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(base).join("Faraday").join("startup.log")
}

/// Journal de démarrage à côté de l'exécutable (si le dossier est inscriptible) :
/// pratique pour diagnostiquer depuis l'hôte.
fn exe_trace_path() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.join("faraday-startup.log"))
}

fn append_line(path: &std::path::Path, msg: &str) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut content = std::fs::read_to_string(path).unwrap_or_default();
    content.push_str(msg);
    content.push('\n');
    let _ = std::fs::write(path, content);
}

/// Ajoute une ligne aux journaux de démarrage (**aucune donnée personnelle**).
fn trace(msg: &str) {
    append_line(&trace_path(), msg);
    if let Some(p) = exe_trace_path() {
        append_line(&p, msg);
    }
}

/// Réinitialise les journaux au lancement (seulement pour le processus
/// principal : les processus enfants ne doivent pas effacer le journal).
fn trace_reset() {
    let _ = std::fs::write(trace_path(), String::new());
    if let Some(p) = exe_trace_path() {
        let _ = std::fs::write(p, String::new());
    }
}

/// Vrai si ce processus est un **processus enfant** CEF (renderer, GPU, …).
///
/// On teste les arguments bruts (`args_os`, sans panic) plutôt que
/// `Args::as_cmd_line()` qui, sous Windows, reconstruit la ligne de commande via
/// `std::env::args()` — ce qui **panique** si un argument n'est pas de l'UTF-8
/// valide (cas du `bootstrap.exe` de CEF en mode sandbox).
fn is_child_process() -> bool {
    std::env::args_os()
        .skip(1)
        .any(|arg| arg.to_string_lossy().starts_with("--type="))
}

/// Point d'entrée commun à l'exécutable et à la DLL.
///
/// `windows_sandbox_info` :
///   - `null` → lancé directement (exe) : sandbox désactivé ;
///   - non-null → lancé par `bootstrap.exe` : sandbox activé.
pub fn browser_main(windows_sandbox_info: *mut u8) -> anyhow::Result<()> {
    let sandboxed = !windows_sandbox_info.is_null();
    SANDBOX_ACTIVE.store(sandboxed, Ordering::Relaxed);

    // Nouveau journal à chaque lancement (uniquement pour le processus
    // principal : les enfants ne doivent pas effacer ce journal).
    let child = is_child_process();
    if !child {
        trace_reset();
    }

    // Capte les panics (sinon le processus meurt sans laisser de trace).
    std::panic::set_hook(Box::new(|info| {
        trace(&format!("PANIC: {info}"));
    }));

    if child {
        trace(&format!(
            "[enfant] faraday {} demarre (processus CEF secondaire)",
            env!("CARGO_PKG_VERSION")
        ));
    } else {
        trace(&format!(
            "faraday {} - demarrage (sandbox={})",
            env!("CARGO_PKG_VERSION"),
            if sandboxed { "active" } else { "desactive" }
        ));
    }

    win::load_cef();
    trace("libcef chargee");

    // Profils : détermine le dossier de données (%APPDATA%\Faraday\profiles\<profil>).
    // Doit être fait AVANT tout accès aux données (session, favoris, exceptions…).
    profiles::init();
    trace(&format!("profil actif = {}", profiles::active_name()));

    // Exceptions par site (« déblocage ponctuel ») : chargées avant CEF.
    // Protégé : une erreur ici ne doit pas empêcher le navigateur de démarrer.
    let loaded = std::panic::catch_unwind(blocklist::load_exceptions);
    trace(&format!(
        "exceptions chargees (ok={})",
        loaded.is_ok()
    ));
    if loaded.is_err() {
        trace("ATTENTION: chargement des exceptions en echec (ignore)");
    }

    let args = cef::args::Args::new();
    trace("arguments de ligne de commande lus");

    // Détection du type de processus : voir `is_child_process()` (sans panic).
    let is_browser_process = !is_child_process();
    trace(&format!("processus browser = {is_browser_process}"));

    // Laisse CEF exécuter les processus enfants (renderer, GPU, ...).
    trace("execute_process ...");
    let ret = execute_process(Some(args.as_main_args()), None, windows_sandbox_info);
    trace(&format!("execute_process = {ret}"));

    if !is_browser_process {
        trace(&format!("processus enfant (execute_process={ret})"));
        if ret < 0 {
            anyhow::bail!("impossible d'exécuter un processus non-browser (code {ret})");
        }
        return Ok(());
    }
    trace(&format!("processus browser (execute_process={ret})"));

    // Taille de la zone de rendu OSR, partagée par tous les onglets.
    let view_size: handler::ViewSize = Arc::new(Mutex::new((0, 0)));

    let mut app = app::FaradayApp::new();

    // Cache Chromium du profil actif (cookies, localStorage, cache HTTP) :
    // c'est ce qui isole réellement les profils entre eux.
    let cache_dir = profiles::active_dir().join("cef");
    let _ = std::fs::create_dir_all(&cache_dir);
    let cache_dir_str = cache_dir.to_string_lossy().to_string();
    trace(&format!("cache_path={cache_dir_str}"));

    // OSR : rendu hors-écran + pompe de messages externe (pour l'UI egui).
    // `no_sandbox` = true uniquement si aucun sandbox_info n'a été fourni.
    let settings = Settings {
        windowless_rendering_enabled: true as _,
        external_message_pump: true as _,
        no_sandbox: (!sandboxed) as _,
        // Isolation des profils : cache/cookies/localStorage dans le profil actif.
        cache_path: CefString::from(cache_dir_str.as_str()),
        ..Default::default()
    };

    let init_code = initialize(
        Some(args.as_main_args()),
        Some(&settings),
        Some(&mut app),
        windows_sandbox_info,
    );
    trace(&format!("initialize (avec cache_path) = {init_code}"));

    // Repli : si l'initialisation échoue avec le cache du profil (chemin
    // inutilisable selon l'environnement), on réessaie sans chemin de cache
    // (cache en mémoire) pour que le navigateur démarre malgré tout.
    let init_code = if init_code != 1 {
        trace("echec avec cache_path -> nouvel essai sans cache_path");
        let fallback = Settings {
            windowless_rendering_enabled: true as _,
            external_message_pump: true as _,
            no_sandbox: (!sandboxed) as _,
            ..Default::default()
        };
        let code = initialize(
            Some(args.as_main_args()),
            Some(&fallback),
            Some(&mut app),
            windows_sandbox_info,
        );
        trace(&format!("initialize (sans cache_path) = {code}"));
        code
    } else {
        init_code
    };

    if init_code != 1 {
        anyhow::bail!("échec de l'initialisation CEF (code {init_code})");
    }
    trace("CEF pret - ouverture de la fenetre");

    // UI chrome (egui) + page OSR, puis arrêt propre de CEF.
    let result = chrome::run(view_size);
    cef::shutdown();
    trace("arret");
    result
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
            trace(&format!("ERREUR: {e}"));
            1
        }
    }
}
