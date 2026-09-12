//! Couche de confidentialité de Faraday.
//!
//! Tous les trackings sont désactivés **par défaut**. Les réglages sont
//! injectés dans le moteur Chromium via :
//!   - les **switches** en ligne de commande CEF (au démarrage du processus
//!     browser, via `App::on_before_command_line_processing`),
//!   - le **blocage réseau** dans le `RequestHandler` (voir `handler.rs`).
//!
//! La config par défaut est **embarquée dans le binaire** (fichier
//! `configs/privacy.toml` compilé via `include_str!`). L'utilisateur peut la
//! surcharger via `%APPDATA%\Faraday\privacy.toml` (créé au 1er lancement) —
//! aucune dépendance au chemin de compilation (indispensable une fois Faraday
//! installé sur une machine propre).

use cef::{CommandLine, ImplCommandLine};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::blocklist;

/// Contenu TOML par défaut, embarqué au moment de la compilation.
const DEFAULT_PRIVACY_TOML: &str = include_str!("../configs/privacy.toml");

/// Configuration de confidentialité (défaut embarqué / surcharge locale).
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

/// Répertoire des données Faraday (celui du **profil actif**).
pub fn appdata_dir() -> PathBuf {
    crate::profiles::active_dir()
}

/// Chemin du fichier de configuration utilisateur (`%APPDATA%\Faraday\privacy.toml`).
pub fn user_config_path() -> PathBuf {
    appdata_dir().join("privacy.toml")
}

/// Parse un contenu TOML `[privacy]` (retombe sur les défauts si invalide).
fn parse_toml(content: &str) -> PrivacyConfig {
    toml::from_str::<FileConfig>(content)
        .map(|cfg| cfg.privacy)
        .unwrap_or_else(|e| {
            eprintln!("[faraday] config privacy invalide ({e}), valeurs par défaut");
            PrivacyConfig::default()
        })
}

impl PrivacyConfig {
    /// Charge la configuration : défaut embarqué, surchargé par le fichier
    /// utilisateur `%APPDATA%\Faraday\privacy.toml` s'il existe (sinon créé).
    pub fn load() -> Self {
        // 1) Défaut embarqué (compile-time) — toujours valide.
        let mut config = parse_toml(DEFAULT_PRIVACY_TOML);

        // 2) Surcharge utilisateur si le fichier existe.
        let path = user_config_path();
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(parsed) = toml::from_str::<FileConfig>(&content) {
                config = parsed.privacy;
            } else {
                eprintln!("[faraday] config utilisateur invalide ({path:?}), défaut embarqué");
            }
        } else if let Some(parent) = path.parent() {
            // 3) 1er lancement : on matérialise la config par défaut pour que
            //    l'utilisateur puisse la modifier (et que save() fonctionne).
            let _ = std::fs::create_dir_all(parent);
            let _ = std::fs::write(&path, DEFAULT_PRIVACY_TOML);
        }

        // Applique immédiatement les réglages « en direct » (blocage, DNT).
        config.apply_runtime();
        config
    }

    /// Applique les réglages qui prennent effet sans redémarrage.
    pub fn apply_runtime(&self) {
        blocklist::set_enabled(self.enable_tracker_blocking);
        blocklist::set_dnt(self.enable_do_not_track);
        // Démarrage volontairement sans protection (dépannage, banc de test) :
        // `FARADAY_PAUSE=1` suspend tout pour la session en cours.
        if std::env::var_os("FARADAY_PAUSE").is_some() {
            blocklist::set_paused(true);
        }
    }

    /// Persiste la configuration utilisateur dans `%APPDATA%\Faraday\privacy.toml`.
    pub fn save(&self) {
        let path = user_config_path();
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("[faraday] config privacy: création répertoire impossible ({e})");
                return;
            }
        }
        let wrapper = FileConfig {
            privacy: self.clone(),
        };
        match toml::to_string_pretty(&wrapper) {
            Ok(content) => {
                if let Err(e) = std::fs::write(&path, content) {
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

/// Liste des switches « zéro tracking » correspondant à la configuration
/// (fonction pure, testable sans instance CEF).
pub fn privacy_switch_strings(config: &PrivacyConfig) -> Vec<String> {
    let mut out = Vec::new();
    if config.disable_component_update {
        out.push("--disable-component-update".into());
    }
    if config.disable_default_apps {
        out.push("--disable-default-apps".into());
    }
    if config.disable_sync {
        out.push("--disable-sync".into());
    }
    if config.disable_suggestions {
        out.push("--disable-suggestions".into());
    }
    if config.disable_personal_autofill {
        out.push("--disable-personal-autofill".into());
    }
    if config.block_third_party_cookies {
        out.push("--block-third-party-cookies".into());
    }
    if config.no_referrer {
        out.push("--no-referrers".into());
    }
    if config.force_https {
        out.push("--force-https".into());
    }
    if !config.disable_features.is_empty() {
        out.push(format!("--disable-features={}", config.disable_features.join(",")));
    }
    // WebRTC : on masque l'IP réelle (anti-empreinte) — toujours actif.
    out.push("--webrtc-ip-handling-policy=disable_non_proxied_udp".into());
    out
}

/// Inscrit tous les switches « zéro tracking » sur la ligne de commande CEF.
pub fn apply_privacy_switches(config: &PrivacyConfig, command_line: &mut CommandLine) {
    for switch in privacy_switch_strings(config) {
        command_line.append_switch(Some(&switch.as_str().into()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_privacy_first() {
        let cfg = PrivacyConfig::default();
        assert!(cfg.enable_tracker_blocking);
        assert!(cfg.enable_do_not_track);
        assert!(cfg.block_third_party_cookies);
        assert!(cfg.no_referrer);
        assert!(cfg.force_https);
        assert_eq!(cfg.default_search_engine, "https://duckduckgo.com");
    }

    #[test]
    fn toml_roundtrip() {
        let wrapper = FileConfig {
            privacy: PrivacyConfig::default(),
        };
        let text = toml::to_string(&wrapper).expect("sérialisation");
        let back: FileConfig = toml::from_str(&text).expect("parse");
        assert!(back.privacy.enable_tracker_blocking);
        assert_eq!(
            back.privacy.default_search_engine,
            "https://duckduckgo.com"
        );
    }

    #[test]
    fn switches_reflect_config() {
        let cfg = PrivacyConfig::default();
        let list = privacy_switch_strings(&cfg);
        assert!(list.iter().any(|s| s == "--block-third-party-cookies"));
        assert!(list.iter().any(|s| s == "--no-referrers"));
        assert!(list.iter().any(|s| s == "--force-https"));
        assert!(list.iter().any(|s| s.starts_with("--disable-features=")));
        assert!(list.iter().any(|s| s.contains("webrtc-ip-handling-policy")));

        // Quand on désactive un réglage, son switch disparaît.
        let mut relaxed = PrivacyConfig::default();
        relaxed.no_referrer = false;
        relaxed.force_https = false;
        let list2 = privacy_switch_strings(&relaxed);
        assert!(!list2.iter().any(|s| s == "--no-referrers"));
        assert!(!list2.iter().any(|s| s == "--force-https"));
    }
}
