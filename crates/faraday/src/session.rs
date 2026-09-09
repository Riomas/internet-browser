//! Persistance de session : onglets ouverts + onglet actif + historique.
//!
//! Fichier : `%APPDATA%\Faraday\session.json`
//! Sauvegardé proprement à la fermeture (on_exit), restauré au démarrage.

use crate::history::HistoryEntry;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Données persistées de la session.
#[derive(Serialize, Deserialize, Default)]
pub struct SessionData {
    /// Index de l'onglet actif.
    pub active: usize,
    /// URLs des onglets ouverts (dans l'ordre, une chaîne vide = nouvel onglet).
    pub tabs: Vec<String>,
    /// Préférence de thème : 0 = Système, 1 = Sombre, 2 = Clair.
    #[serde(default)]
    pub theme: u8,
    /// Historique de navigation (le plus récent en premier).
    pub history: Vec<HistoryEntry>,
}

/// Répertoire des données de session dans %APPDATA%.
fn session_dir() -> PathBuf {
    let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base).join("Faraday")
}

fn session_file() -> PathBuf {
    session_dir().join("session.json")
}

/// Charge la session sauvegardée (ou une session vide par défaut).
pub fn load() -> SessionData {
    let file = session_file();
    match std::fs::read_to_string(&file) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => SessionData::default(),
    }
}

/// Sauvegarde la session sur le disque (crée le répertoire si besoin).
pub fn save(data: &SessionData) {
    let dir = session_dir();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("[faraday] session: création du répertoire impossible: {e}");
        return;
    }
    match serde_json::to_string_pretty(data) {
        Ok(json) => {
            if let Err(e) = std::fs::write(session_file(), json) {
                eprintln!("[faraday] session: écriture impossible: {e}");
            }
        }
        Err(e) => eprintln!("[faraday] session: sérialisation impossible: {e}"),
    }
}
