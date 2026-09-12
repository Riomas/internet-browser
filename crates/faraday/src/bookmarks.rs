//! Favoris (bookmarks) de Faraday.
//!
//! Stockés dans `%APPDATA%\Faraday\bookmarks.json`. Aucune donnée n'est
//! envoyée à l'extérieur : les favoris restent locaux.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Un favori (titre affiché + URL).
#[derive(Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub title: String,
    pub url: String,
}

/// Collection de favoris partagée (thread-safe).
pub type Bookmarks = Arc<Mutex<Vec<Bookmark>>>;

/// Chemin du fichier de favoris.
fn path() -> std::path::PathBuf {
    crate::privacy::appdata_dir().join("bookmarks.json")
}

/// Charge les favoris depuis le disque (liste vide si absent/invalide).
pub fn load() -> Bookmarks {
    Arc::new(Mutex::new(load_from(&path())))
}

fn load_from(file: &Path) -> Vec<Bookmark> {
    match std::fs::read_to_string(file) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

/// Sauvegarde les favoris sur le disque.
fn save_to(list: &Bookmarks, file: &Path) {
    let data = list.lock().unwrap().clone();
    if let Some(parent) = file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match serde_json::to_string_pretty(&data) {
        Ok(json) => {
            if let Err(e) = std::fs::write(file, json) {
                eprintln!("[faraday] favoris: écriture impossible ({e})");
            }
        }
        Err(e) => eprintln!("[faraday] favoris: sérialisation impossible ({e})"),
    }
}

/// Vrai si l'URL est déjà en favori.
pub fn contains(list: &Bookmarks, url: &str) -> bool {
    let url = url.trim();
    list.lock().unwrap().iter().any(|b| b.url == url)
}

/// Copie de la liste (pour l'affichage, sans tenir le verrou).
pub fn list_of(list: &Bookmarks) -> Vec<Bookmark> {
    list.lock().unwrap().clone()
}

/// Ajoute le favori s'il est absent, le retire sinon.
/// Renvoie `true` si la page est désormais **en favori**.
pub fn toggle(list: &Bookmarks, title: &str, url: &str) -> bool {
    toggle_at(list, title, url, &path())
}

fn toggle_at(list: &Bookmarks, title: &str, url: &str, file: &Path) -> bool {
    let url = url.trim();
    if url.is_empty() {
        return false;
    }
    let added = {
        let mut items = list.lock().unwrap();
        match items.iter().position(|b| b.url == url) {
            Some(pos) => {
                items.remove(pos);
                false
            }
            None => {
                let title = if title.trim().is_empty() {
                    crate::history::hostname(url)
                } else {
                    title.trim().to_string()
                };
                items.insert(
                    0,
                    Bookmark {
                        title,
                        url: url.to_string(),
                    },
                );
                true
            }
        }
    };
    save_to(list, file);
    added
}

/// Retire un favori par son URL.
pub fn remove(list: &Bookmarks, url: &str) {
    remove_at(list, url, &path())
}

fn remove_at(list: &Bookmarks, url: &str, file: &Path) {
    let url = url.trim();
    {
        let mut items = list.lock().unwrap();
        items.retain(|b| b.url != url);
    }
    save_to(list, file);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn temp_file(name: &str) -> std::path::PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("faraday_bm_{name}_{n}.json"))
    }

    #[test]
    fn toggle_add_and_remove() {
        let file = temp_file("toggle");
        let list: Bookmarks = Arc::new(Mutex::new(Vec::new()));

        assert!(toggle_at(&list, "EFF", "https://eff.org/", &file));
        assert!(contains(&list, "https://eff.org/"));
        assert_eq!(list_of(&list).len(), 1);
        assert_eq!(list_of(&list)[0].title, "EFF");

        // Le fichier doit exister après sauvegarde.
        assert!(file.exists());

        // Second appel = retrait.
        assert!(!toggle_at(&list, "EFF", "https://eff.org/", &file));
        assert!(!contains(&list, "https://eff.org/"));
        assert!(list_of(&list).is_empty());

        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn toggle_uses_hostname_when_title_missing() {
        let file = temp_file("title");
        let list: Bookmarks = Arc::new(Mutex::new(Vec::new()));
        assert!(toggle_at(&list, "   ", "https://duckduckgo.com/?q=x", &file));
        let items = list_of(&list);
        assert_eq!(items[0].title, "duckduckgo.com");
        // URL vide : ignorée.
        assert!(!toggle_at(&list, "t", "   ", &file));
        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn remove_by_url() {
        let file = temp_file("remove");
        let list: Bookmarks = Arc::new(Mutex::new(Vec::new()));
        toggle_at(&list, "A", "https://a.example/", &file);
        toggle_at(&list, "B", "https://b.example/", &file);
        assert_eq!(list_of(&list).len(), 2);
        remove_at(&list, "https://a.example/", &file);
        let items = list_of(&list);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].url, "https://b.example/");
        let _ = std::fs::remove_file(&file);
    }
}
