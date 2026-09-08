//! Couche de confidentialité de Faraday.
//!
//! Tous les trackings sont désactivés **par défaut**. Les réglages de
//! `configs/privacy.toml` sont injectés dans le moteur Chromium via :
//!   - les **switches** en ligne de commande CEF (au démarrage du processus
//!     browser, via `App::on_before_command_line_processing`),
//!   - le **blocage réseau** dans le `RequestHandler` (voir `handler.rs`).

use cef::{CommandLine, ImplCommandLine};
use serde::{Deserialize, Serialize};

use crate::blocklist;

/// Configuration de confidentialité chargée depuis `configs/privacy.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    /// Bloque les requêtes vers les domaines de la blocklist (en direct).
    pub enable_tracker_blocking: bool,
    /// Envoie l'en-tête Do Not Track (`DNT: 1`) (en direct).
    pub enable_do_not_track: bool,
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
    /// Bloque les cookies tiers (les sites ne peuvent pas se suivre entre eux).
    pub block_third_party_cookies: bool,
    /// N'envoie aucun en-tête `Referer` (anti-suivi).
    pub no_referrer: bool,
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
            enable_tracker_blocking: true,
            enable_do_not_track: true,
            disable_component_update: true,
            disable_default_apps: true,
            disable_sync: true,
            disable_suggestions: true,
            disable_personal_autofill: true,
            block_third_party_cookies: true,
            no_referrer: true,
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
        let config = match std::fs::read_to_string(path) {
            Ok(content) => toml::from_str(&content)
                .map(|cfg: FileConfig| cfg.privacy)
                .unwrap_or_else(|e| {
                    eprintln!("[faraday] config privacy invalide ({e}), valeurs par défaut");
                    PrivacyConfig::default()
                }),
            Err(_) => PrivacyConfig::default(),
        };
        // Applique immédiatement les réglages « en direct » (blocage, DNT).
        config.apply_runtime();
        config
    }

    /// Applique les réglages qui prennent effet sans redémarrage.
    pub fn apply_runtime(&self) {
        blocklist::set_enabled(self.enable_tracker_blocking);
        blocklist::set_dnt(self.enable_do_not_track);
    }

    /// Persiste la configuration dans `configs/privacy.toml`.
    pub fn save(&self) {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/configs/privacy.toml");
        let wrapper = FileConfig {
            privacy: self.clone(),
        };
        match toml::to_string_pretty(&wrapper) {
            Ok(content) => {
                if let Err(e) = std::fs::write(path, content) {
                    eprintln!("[faraday] config privacy: écriture impossible ({e})");
                }
            }
            Err(e) => eprintln!("[faraday] config privacy: sérialisation impossible ({e})"),
        }
    }
}

/// Enveloppe du fichier TOML (`[privacy]`).
#[derive(Debug, Serialize, Deserialize)]
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
    if config.block_third_party_cookies {
        command_line.append_switch(Some(&"--block-third-party-cookies".into()));
    }
    if config.no_referrer {
        command_line.append_switch(Some(&"--no-referrers".into()));
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
