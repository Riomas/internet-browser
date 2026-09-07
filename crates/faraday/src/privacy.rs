//! Couche de confidentialité de Faraday.
//!
//! Tous les trackings sont désactivés **par défaut**. Les réglages de
//! `configs/privacy.toml` sont injectés dans le moteur Chromium via :
//!   - les **switches** en ligne de commande CEF (au démarrage du processus
//!     browser, via `App::on_before_command_line_processing`),
//!   - le **blocage réseau** dans le `RequestHandler` (voir `handler.rs`).

use cef::{CommandLine, ImplCommandLine};
use serde::Deserialize;

/// Configuration de confidentialité chargée depuis `configs/privacy.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct PrivacyConfig {
    /// Désactive les mises à jour de composants (télémesure).
    pub disable_component_update: bool,
    /// Désactive les applications par défaut.
    pub disable_default_apps: bool,
    /// Désactive la synchronisation (compte Google/Edge).
    pub disable_sync: bool,
    /// Désactive les suggestions de recherche.
    pub disable_suggestions: bool,
    /// Désactive l'autofill personnel.
    pub disable_personal_autofill: bool,
    /// Force le passage en HTTPS quand disponible.
    pub force_https: bool,
    /// Features Chromium à désactiver (prefetch, preconnect, etc.).
    #[serde(default)]
    pub disable_features: Vec<String>,
    /// Moteur de recherche par défaut.
    pub default_search_engine: String,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            disable_component_update: true,
            disable_default_apps: true,
            disable_sync: true,
            disable_suggestions: true,
            disable_personal_autofill: true,
            force_https: true,
            disable_features: vec![
                "Preload".to_string(),
                "PrefetchPrivacyChanges".to_string(),
                "NetworkService".to_string(),
            ],
            default_search_engine: "https://duckduckgo.com".to_string(),
        }
    }
}

impl PrivacyConfig {
    /// Charge la configuration depuis le disque, ou retombe sur les valeurs
    /// par défaut si le fichier est absent/invalide.
    pub fn load() -> Self {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/configs/privacy.toml");
        match std::fs::read_to_string(path) {
            Ok(content) => toml::from_str(&content)
                .map(|cfg: FileConfig| cfg.privacy)
                .unwrap_or_else(|e| {
                    eprintln!("[faraday] config privacy invalide ({e}), valeurs par défaut");
                    PrivacyConfig::default()
                }),
            Err(_) => PrivacyConfig::default(),
        }
    }
}

/// Enveloppe du fichier TOML (`[privacy]`).
#[derive(Debug, Deserialize)]
struct FileConfig {
    privacy: PrivacyConfig,
}

/// Inscrit tous les switches « zéro tracking » sur la ligne de commande CEF.
pub fn apply_privacy_switches(config: &PrivacyConfig, command_line: &mut CommandLine) {
    if config.disable_component_update {
        command_line.append_switch(Some(&"--disable-component-update".into()));
    }
    if config.disable_default_apps {
        command_line.append_switch(Some(&"--disable-default-apps".into()));
    }
    if config.disable_sync {
        command_line.append_switch(Some(&"--disable-sync".into()));
    }
    if config.disable_suggestions {
        command_line.append_switch(Some(&"--disable-suggestions".into()));
    }
    if config.disable_personal_autofill {
        command_line.append_switch(Some(&"--disable-personal-autofill".into()));
    }
    if config.force_https {
        command_line.append_switch(Some(&"--force-https".into()));
    }
    if !config.disable_features.is_empty() {
        let value = config.disable_features.join(",");
        let switch = format!("--disable-features={value}");
        command_line.append_switch(Some(&switch.as_str().into()));
    }
    // WebRTC : on masque l'IP réelle (anti-empreinte).
    command_line.append_switch(Some(
        &"--webrtc-ip-handling-policy=disable_non_proxied_udp".into(),
    ));
}
