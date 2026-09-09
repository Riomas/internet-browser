//! Gestion des téléchargements (métadonnées partagées entre CEF et UI).
//!
//! Seules des données sérialisables sont stockées ici (thread-safe) : les
//! `DownloadItemCallback` de CEF restent côté handler, qui annule un
//! téléchargement quand l'UI pose le drapeau `cancel_requested`.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// État d'un téléchargement.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DownloadState {
    Starting,
    InProgress,
    Complete,
    Cancelled,
    Interrupted,
}

impl DownloadState {
    pub fn label(&self) -> &'static str {
        match self {
            DownloadState::Starting => "Démarrage",
            DownloadState::InProgress => "En cours",
            DownloadState::Complete => "Terminé",
            DownloadState::Cancelled => "Annulé",
            DownloadState::Interrupted => "Interrompu",
        }
    }
}

/// Une ligne de téléchargement affichée dans l'UI.
#[derive(Clone)]
pub struct DownloadEntry {
    pub id: u32,
    pub url: String,
    pub name: String,
    pub path: String,
    pub state: DownloadState,
    pub percent: i32,
    pub speed: i64,
    pub received: u64,
    pub total: u64,
    /// Drapeau posé par l'UI pour demander l'annulation.
    pub cancel_requested: bool,
}

/// Collection partagée de téléchargements (thread-safe).
pub type Downloads = Arc<Mutex<Vec<DownloadEntry>>>;

/// Événement de notification d'un téléchargement (démarré/terminé/...).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DownloadNoticeKind {
    Started,
    Complete,
    Cancelled,
    Interrupted,
}

/// Notification envoyée par le handler CEF vers l'UI.
#[derive(Clone)]
pub struct DownloadNotice {
    pub name: String,
    pub kind: DownloadNoticeKind,
}

/// File de notifications à afficher (consommée par l'UI egui).
pub type DownloadNotices = Arc<Mutex<Vec<DownloadNotice>>>;

/// Ajoute une notification (avec une taille max pour éviter l'accumulation).
pub fn notify(list: &mut Vec<DownloadNotice>, name: &str, kind: DownloadNoticeKind) {
    list.push(DownloadNotice {
        name: name.to_string(),
        kind,
    });
    if list.len() > 40 {
        list.remove(0);
    }
}

/// Répertoire de téléchargement par défaut : `%USERPROFILE%\Downloads`.
pub fn downloads_dir() -> PathBuf {
    let user = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(user).join("Downloads")
}

/// Nettoie un nom de fichier suggéré (anti-traversée de chemin + caractères
/// invalides Windows).
pub fn safe_file_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let cleaned: String = base
        .chars()
        .filter(|c| {
            !matches!(
                c,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
            ) && !c.is_control()
        })
        .take(180)
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "telechargement".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Chemin de destination unique : évite d'écraser un fichier existant en
/// ajoutant un suffixe « (1) », « (2) », etc.
pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let mut candidate = dir.join(name);
    let mut i = 1;
    while candidate.exists() {
        let stem = candidate
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "fichier".to_string());
        let ext = candidate
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();
        candidate = dir.join(format!("{stem} ({i}){ext}"));
        i += 1;
    }
    candidate
}

/// Crée la ligne de téléchargement, ou met à jour celle existante (par id).
pub fn upsert(list: &mut Vec<DownloadEntry>, entry: DownloadEntry) {
    if let Some(e) = list.iter_mut().find(|e| e.id == entry.id) {
        *e = entry;
    } else {
        list.push(entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_file_name_sanitizes() {
        assert_eq!(safe_file_name("../../evil<>.txt"), "evil.txt");
        assert_eq!(safe_file_name("fichier:*.txt"), "fichier.txt");
        assert_eq!(safe_file_name("a?b|c\u{0}.txt"), "abc.txt");
        assert_eq!(safe_file_name("   "), "telechargement");
    }

    #[test]
    fn unique_path_dedups() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("faraday_test_{unique}"));
        std::fs::create_dir_all(&dir).unwrap();

        let first = unique_path(&dir, "fichier.txt");
        std::fs::write(&first, b"1").unwrap();
        let second = unique_path(&dir, "fichier.txt");

        assert_ne!(first, second);
        assert!(first.to_string_lossy().ends_with("fichier.txt"));
        assert!(second.to_string_lossy().contains("fichier (1).txt"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
