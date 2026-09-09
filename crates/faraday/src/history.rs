//! Historique de navigation (en mémoire, partagé entre threads CEF/UI).
//!
//! Les entrées sont poussées depuis les handlers CEF (thread de navigation)
//! et lues par l'UI egui (thread principal) via un simple `Mutex`.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Une visite enregistrée dans l'historique.
#[derive(Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
    /// Horodatage Unix (secondes).
    pub ts: u64,
}

/// Collection d'historique partagée (thread-safe).
pub type History = Arc<Mutex<Vec<HistoryEntry>>>;

/// Taille maximale de l'historique conservée en mémoire.
const MAX_ENTRIES: usize = 500;

/// Nom d'hôte d'une URL (utilisé comme titre par défaut).
pub fn hostname(url: &str) -> String {
    let mut rest = url.trim();
    for p in ["https://", "http://", "ftp://", "file://", "about:"] {
        if let Some(r) = rest.strip_prefix(p) {
            rest = r;
            break;
        }
    }
    match rest.find(['/', '?', '#']) {
        Some(i) => rest[..i].to_string(),
        None => rest.to_string(),
    }
}

/// Ajoute une visite en tête de liste (sans doublon consécutif récent).
pub fn push(history: &History, url: &str) {
    let url = url.trim().to_string();
    if url.is_empty() || url.starts_with("about:") {
        return;
    }
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let title = hostname(&url);
    let mut list = history.lock().unwrap();
    // Évite les doublons consécutifs (déjà en tête).
    if let Some(head) = list.first() {
        if head.url == url {
            return;
        }
    }
    list.insert(
        0,
        HistoryEntry {
            url,
            title,
            ts,
        },
    );
    if list.len() > MAX_ENTRIES {
        list.truncate(MAX_ENTRIES);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_history() -> History {
        Arc::new(Mutex::new(Vec::new()))
    }

    #[test]
    fn push_inserts_at_front_and_dedups() {
        let h = new_history();
        push(&h, "https://eff.org/");
        push(&h, "https://ddg.co/");
        // Doublon consécutif : ignoré.
        push(&h, "https://ddg.co/");
        // Nouvelle visite d'un site déjà vu : remonte en tête.
        push(&h, "https://eff.org/deep");

        let list = h.lock().unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].url, "https://eff.org/deep");
        assert_eq!(list[0].title, "eff.org");
        assert_eq!(list[1].url, "https://ddg.co/");
        assert_eq!(list[2].url, "https://eff.org/");
    }

    #[test]
    fn push_ignores_empty_and_about() {
        let h = new_history();
        push(&h, "");
        push(&h, "   ");
        push(&h, "about:blank");
        assert!(h.lock().unwrap().is_empty());
    }

    #[test]
    fn hostname_extraction() {
        assert_eq!(hostname("https://www.example.com/path?x=1"), "www.example.com");
        assert_eq!(hostname("https://eff.org"), "eff.org");
        assert_eq!(hostname("duckduckgo.com"), "duckduckgo.com");
    }
}
