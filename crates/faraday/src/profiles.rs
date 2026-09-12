//! Profils multi-utilisateurs.
//!
//! Chaque profil possède **son propre dossier de données** :
//! `%APPDATA%\Faraday\profiles\<profil>\` (onglets, historique, favoris,
//! exceptions, réglages de confidentialité et cache Chromium).
//!
//! Le profil actif est enregistré dans `%APPDATA%\Faraday\profiles.json`.
//! Changer de profil nécessite un **redémarrage** (le cache Chromium se fixe au
//! démarrage de CEF).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// Liste des profils + profil actif (fichier `profiles.json`).
#[derive(Clone, Serialize, Deserialize)]
pub struct ProfilesState {
    pub active: String,
    pub names: Vec<String>,
}

impl Default for ProfilesState {
    fn default() -> Self {
        Self {
            active: "default".to_string(),
            names: vec!["default".to_string()],
        }
    }
}

static STATE: OnceLock<Mutex<ProfilesState>> = OnceLock::new();

fn state() -> &'static Mutex<ProfilesState> {
    STATE.get_or_init(|| Mutex::new(ProfilesState::default()))
}

/// Racine des données Faraday (`%APPDATA%\Faraday`).
pub fn root() -> PathBuf {
    let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base).join("Faraday")
}

fn state_path() -> PathBuf {
    root().join("profiles.json")
}

/// Transforme un nom de profil en nom de dossier sûr (`Mon Profil` → `mon-profil`).
pub fn sanitize(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().chars() {
        if c.is_alphanumeric() || c == '-' || c == '_' {
            out.push(c);
        } else if c == ' ' {
            out.push('-');
        }
    }
    let out = out.trim_matches('-').to_lowercase();
    if out.is_empty() {
        "default".to_string()
    } else {
        out
    }
}

fn dir_for(slug: &str) -> PathBuf {
    root().join("profiles").join(slug)
}

/// Charge la liste des profils et prépare le dossier du profil actif.
pub fn init() {
    let mut loaded = match std::fs::read_to_string(state_path()) {
        Ok(text) => serde_json::from_str::<ProfilesState>(&text).unwrap_or_default(),
        Err(_) => ProfilesState::default(),
    };
    if loaded.names.is_empty() {
        loaded.names.push("default".to_string());
    }
    if !loaded.names.iter().any(|n| n == &loaded.active) {
        loaded.active = loaded.names[0].clone();
    }
    *state().lock().unwrap() = loaded;
    let _ = std::fs::create_dir_all(active_dir());
}

/// Dossier de données du profil actif.
pub fn active_dir() -> PathBuf {
    let name = active_name();
    dir_for(&sanitize(&name))
}

/// Nom du profil actif.
pub fn active_name() -> String {
    state().lock().unwrap().active.clone()
}

/// Liste des profils disponibles.
pub fn list() -> Vec<String> {
    state().lock().unwrap().names.clone()
}

/// Change le profil actif (nécessite un redémarrage pour être pleinement appliqué).
pub fn set_active(name: &str) {
    {
        let mut s = state().lock().unwrap();
        if !s.names.iter().any(|n| n == name) {
            return;
        }
        s.active = name.to_string();
    }
    save();
    let _ = std::fs::create_dir_all(active_dir());
}

/// Crée un profil (renvoie `false` si le nom est vide ou déjà utilisé).
pub fn create(name: &str) -> bool {
    let name = name.trim().to_string();
    if name.is_empty() {
        return false;
    }
    {
        let mut s = state().lock().unwrap();
        if s.names.iter().any(|n| n == &name) {
            return false;
        }
        s.names.push(name);
    }
    save();
    true
}

/// Supprime un profil (jamais le dernier, ni le profil actif) et ses données.
pub fn remove(name: &str) -> bool {
    {
        let mut s = state().lock().unwrap();
        if s.names.len() <= 1 || s.active == name {
            return false;
        }
        s.names.retain(|n| n != name);
    }
    save();
    let _ = std::fs::remove_dir_all(dir_for(&sanitize(name)));
    true
}

fn save() {
    let data = state().lock().unwrap().clone();
    let _ = std::fs::create_dir_all(root());
    match serde_json::to_string_pretty(&data) {
        Ok(json) => {
            if let Err(e) = std::fs::write(state_path(), json) {
                eprintln!("[faraday] profils: écriture impossible ({e})");
            }
        }
        Err(e) => eprintln!("[faraday] profils: sérialisation impossible ({e})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_profile_names() {
        assert_eq!(sanitize("Mon Profil"), "mon-profil");
        assert_eq!(sanitize("  Travail  "), "travail");
        assert_eq!(sanitize("a/b\\c:d*?"), "abcd");
        assert_eq!(sanitize("Pro"), "pro");
        assert_eq!(sanitize("///"), "default");
        assert_eq!(sanitize(""), "default");
    }
}
