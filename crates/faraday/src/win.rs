//! Détails spécifiques à Windows (cible principale : Windows 10+).

use cef::{sys, *};

/// Prépare CEF pour Windows (déclare la version d'API utilisée).
pub fn load_cef() {
    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);
}

/// Crée une `WindowInfo` pour un browser en fenêtre native (popup CEF).
pub fn window_info() -> WindowInfo {
    // runtime_style DEFAULT (comme l'exemple officiel cefsimple).
    WindowInfo::default().set_as_popup(Default::default(), "Faraday")
}

/// Boucle de messages Windows requise par les fenêtres CEF natives.
pub fn run_message_loop() {
    cef::run_message_loop();
}
