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
