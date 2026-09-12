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

/// Répertoire des données de session (celui du **profil actif**).
fn session_dir() -> PathBuf {
    crate::profiles::active_dir()
}

fn session_file() -> PathBuf {
    session_dir().join("session.json")
}

/// Charge la session sauvegardée (ou une session vide par défaut).
pub fn load() -> SessionData {
    load_from(&session_file())
}

/// Charge une session depuis un chemin donné (tests inclus).
fn load_from(file: &std::path::Path) -> SessionData {
    match std::fs::read_to_string(file) {
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
    save_to(data, &session_file());
}

/// Sauvegarde une session vers un chemin donné (tests inclus).
fn save_to(data: &SessionData, file: &std::path::Path) {
    match serde_json::to_string_pretty(data) {
        Ok(json) => {
            if let Some(parent) = file.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    eprintln!("[faraday] session: création du répertoire impossible: {e}");
                    return;
                }
            }
            if let Err(e) = std::fs::write(file, json) {
                eprintln!("[faraday] session: écriture impossible: {e}");
            }
        }
        Err(e) => eprintln!("[faraday] session: sérialisation impossible: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn session_roundtrip() {
        let data = SessionData {
            active: 2,
            tabs: vec!["https://eff.org".into(), String::new(), "about:blank".into()],
            theme: 1,
            history: vec![
                HistoryEntry {
                    url: "https://eff.org".into(),
                    title: "EFF".into(),
                    ts: 1_700_000_000,
                },
                HistoryEntry {
                    url: "https://ddg.co".into(),
                    title: "DuckDuckGo".into(),
                    ts: 1_700_000_001,
                },
            ],
        };

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let file = std::env::temp_dir().join(format!("faraday_session_{unique}.json"));

        save_to(&data, &file);
        let back = load_from(&file);

        assert_eq!(back.active, 2);
        assert_eq!(back.theme, 1);
        assert_eq!(back.tabs, data.tabs);
        assert_eq!(back.history.len(), 2);
        assert_eq!(back.history[0].title, "EFF");
        assert_eq!(back.history[1].url, "https://ddg.co");

        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn session_missing_file_is_default() {
        let file = std::env::temp_dir().join("faraday_session_absent.json");
        let _ = std::fs::remove_file(&file);
        let data = load_from(&file);
        assert!(data.tabs.is_empty());
        assert_eq!(data.active, 0);
    }
}
