//! Détails spécifiques à Windows.

use cef::{sys, *};

/// Prépare CEF pour Windows (déclare la version d'API utilisée).
pub fn load_cef() {
    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);
}
