//! Script de build de Faraday.
//! L'icône de l'application sera intégrée en Phase 3 ; ici on se contente
//! d'initialiser les ressources Windows sans icône.

#[cfg(target_os = "windows")]
fn main() {
    if std::env::var_os("CARGO_CFG_TARGET_OS").map(|s| s == "windows").unwrap_or(false) {
        let _res = winres::WindowsResource::new();
        // Pas d'icône pour l'instant (Phase 0).
        let _ = _res;
    }
}

#[cfg(not(target_os = "windows"))]
fn main() {}
