//! Moteur de blocage des trackers/publicités de Faraday (Phase 2).
//!
//! Utilise une liste de règles embarquée (`configs/blocklist.txt`) au format
//! proche d'Adblock Plus (sous-ensemble) :
//!   - `! commentaire`            (ignoré)
//!   - `||domaine.com^`           → bloque domaine.com + ses sous-domaines
//!   - `domaine.com`              → bloque ce domaine (idem ci-dessus)
//!   - `@@||domaine.com^`         → exception (toujours autorisé)
//!   - les options `$...` sont ignorées (on bloque la règle de base).
//!
//! Le `RequestHandler` demande à ce moteur s'il doit bloquer une URL ; un
//! compteur global permet d'afficher le nombre de blocages sur le bouclier.

use std::collections::HashSet;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Compteur global de requêtes bloquées (affiché sur le bouclier).
static BLOCKED: AtomicU64 = AtomicU64::new(0);

/// Le blocage réseau est-il activé ? (basculable depuis les Paramètres)
static ENABLED: AtomicBool = AtomicBool::new(true);

/// L'en-tête Do Not Track (`DNT: 1`) est-il envoyé ?
static DNT: AtomicBool = AtomicBool::new(true);

/// Nombre de requêtes bloquées depuis le démarrage.
pub fn blocked_count() -> u64 {
    BLOCKED.load(Ordering::Relaxed)
}

/// Incrémente le compteur de blocages (appelé quand on refuse une requête).
pub fn incr_blocked() {
    BLOCKED.fetch_add(1, Ordering::Relaxed);
}

/// Le blocage des trackers est-il actif ?
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Active/désactive le blocage réseau (paramètre en direct).
pub fn set_enabled(value: bool) {
    ENABLED.store(value, Ordering::Relaxed);
}

/// L'en-tête Do Not Track doit-il être envoyé ?
pub fn dnt_enabled() -> bool {
    DNT.load(Ordering::Relaxed)
}

/// Active/désactive l'envoi de l'en-tête Do Not Track (en direct).
pub fn set_dnt(value: bool) {
    DNT.store(value, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Exceptions par site ("déblocage ponctuel")
//
// Certains sites (banques, messageries, portails) nécessitent le chargement de
// ressources tierces pour fonctionner. L'utilisateur peut désactiver la
// protection pour **un domaine donné** ; les sites ainsi exemptés ne subissent
// ni le blocage réseau ni le blocage des cookies tiers.
// ---------------------------------------------------------------------------

/// Domaines exemptés (protection désactivée), en minuscules et sans `www.`.
static EXCEPTIONS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn exceptions() -> &'static Mutex<HashSet<String>> {
    EXCEPTIONS.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Normalise un hôte pour comparaison : minuscules, sans schéma/chemin/port,
/// sans point final ni préfixe `www.` (si un domaine reste).
pub fn normalize_host(input: &str) -> String {
    let host = host_of(input).to_lowercase();
    let host = host.trim_end_matches('.').to_string();
    match host.strip_prefix("www.") {
        Some(rest) if rest.contains('.') => rest.to_string(),
        _ => host,
    }
}

/// Vrai si `host` est exempté (lui-même, `www.`, ou un domaine parent).
pub fn is_exempt(host: &str) -> bool {
    let host = normalize_host(host);
    if host.is_empty() {
        return false;
    }
    let set = exceptions().lock().unwrap();
    if set.is_empty() {
        return false;
    }
    if set.contains(&host) {
        return true;
    }
    // Ex. « secure.banque.fr » est exempté si « banque.fr » l'est.
    set.iter().any(|ex| host.ends_with(&format!(".{ex}")))
}

/// Ajoute/retire une exception pour un domaine.
pub fn set_exempt(host: &str, exempt: bool) {
    let host = normalize_host(host);
    if host.is_empty() {
        return;
    }
    let mut set = exceptions().lock().unwrap();
    if exempt {
        set.insert(host);
    } else {
        set.remove(&host);
    }
}

/// Liste triée des domaines exemptés (affichée dans les Paramètres).
pub fn exempt_list() -> Vec<String> {
    let set = exceptions().lock().unwrap();
    let mut list: Vec<String> = set.iter().cloned().collect();
    list.sort();
    list
}

fn exceptions_path() -> std::path::PathBuf {
    crate::privacy::appdata_dir().join("exceptions.json")
}

/// Charge les exceptions depuis `%APPDATA%\Faraday\exceptions.json`.
pub fn load_exceptions() {
    let path = exceptions_path();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return;
    };
    let Ok(list) = serde_json::from_str::<Vec<String>>(&text) else {
        eprintln!("[faraday] exceptions.json invalide, ignore");
        return;
    };
    let mut set = exceptions().lock().unwrap();
    set.clear();
    for item in list {
        let host = normalize_host(&item);
        if !host.is_empty() {
            set.insert(host);
        }
    }
}

/// Sauvegarde les exceptions sur disque.
pub fn save_exceptions() {
    let dir = crate::privacy::appdata_dir();
    let _ = std::fs::create_dir_all(&dir);
    match serde_json::to_string_pretty(&exempt_list()) {
        Ok(json) => {
            if let Err(e) = std::fs::write(exceptions_path(), json) {
                eprintln!("[faraday] exceptions: écriture impossible ({e})");
            }
        }
        Err(e) => eprintln!("[faraday] exceptions: sérialisation impossible ({e})"),
    }
}

/// Liste embarquée au moment de la compilation.
const BLOCKLIST_SOURCE: &str = include_str!("../configs/blocklist.txt");

struct Rule {
    /// Domaine racine de la règle (minuscules, sans `www.` obligatoire).
    host: String,
    /// `true` = exception (autoriser), `false` = bloquer.
    allow: bool,
}

/// Liste de règles chargée une seule fois.
pub struct BlockList {
    rules: Vec<Rule>,
}

impl BlockList {
    fn parse(text: &str) -> Self {
        let mut rules = Vec::new();
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('!') {
                continue;
            }
            // Exception ?
            let (allow, mut body) = if let Some(rest) = line.strip_prefix("@@") {
                (true, rest)
            } else {
                (false, line)
            };
            // Supprime d'éventuelles options (`$third-party`, etc.).
            if let Some(i) = body.find('$') {
                body = &body[..i];
            }
            // Règle réseau ABP : `||domaine^`.
            if let Some(rest) = body.strip_prefix("||") {
                body = rest;
            }
            // Séparateur ABP de fin `^`.
            body = body.trim_end_matches('^').trim();
            // On ne gère que les domaines « propres » (pas de chemins,
            // d'expressions complexes ou de regex).
            if body.is_empty()
                || body.contains('/')
                || body.contains('*')
                || body.contains('|')
                || body.contains('~')
            {
                continue;
            }
            let host = body.trim_start_matches('.').to_ascii_lowercase();
            if host.is_empty() {
                continue;
            }
            rules.push(Rule { host, allow });
        }
        Self { rules }
    }

    /// Vrai si l'hôte doit être bloqué (les exceptions ont priorité).
    fn blocks(&self, host: &str) -> bool {
        let host = host.trim().trim_start_matches('.').to_ascii_lowercase();
        if host.is_empty() {
            return false;
        }
        // Les exceptions passent toujours.
        for r in &self.rules {
            if r.allow && domain_matches(&r.host, &host) {
                return false;
            }
        }
        for r in &self.rules {
            if !r.allow && domain_matches(&r.host, &host) {
                return true;
            }
        }
        false
    }
}

/// `host` correspond à `domain` s'il est égal ou un sous-domaine.
fn domain_matches(domain: &str, host: &str) -> bool {
    host == domain || host.ends_with(&format!(".{domain}"))
}

fn list() -> &'static BlockList {
    static LIST: OnceLock<BlockList> = OnceLock::new();
    LIST.get_or_init(|| BlockList::parse(BLOCKLIST_SOURCE))
}

/// Extrait le nom d'hôte (minuscules) d'une URL.
pub fn host_of(url: &str) -> String {
    let mut rest = url.trim();
    for p in ["https://", "http://", "ftp://", "ws://", "wss://"] {
        if let Some(r) = rest.strip_prefix(p) {
            rest = r;
            break;
        }
    }
    // Supprime « user:pass@host ».
    if let Some(i) = rest.rfind('@') {
        rest = &rest[i + 1..];
    }
    let end = rest
        .find(['/', ':', '?', '#'])
        .unwrap_or(rest.len());
    rest[..end].trim().to_ascii_lowercase()
}

/// Décide si une URL doit être bloquée (liste de règles) ou non.
pub fn should_block(url: &str) -> bool {
    if !enabled() {
        return false;
    }
    let host = host_of(url);
    list().blocks(&host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_match() {
        let bl = BlockList::parse("! test\n||ads.example^\n@@||ok.example^\nexample.org\n");
        assert!(bl.blocks("ads.example"));
        assert!(bl.blocks("sub.ads.example"));
        assert!(!bl.blocks("ok.example"));
        assert!(!bl.blocks("ok.sub.example"));
        assert!(!bl.blocks("ok.example.com"));
        assert!(bl.blocks("example.org"));
        assert!(bl.blocks("www.example.org"));
        assert!(!bl.blocks("other.com"));
    }

    #[test]
    fn host_parse() {
        assert_eq!(host_of("https://Sub.Example.com/path?x=1"), "sub.example.com");
        assert_eq!(host_of("http://user@host.io:8080/a"), "host.io");
    }

    #[test]
    fn real_list_blocks_known_trackers() {
        // Vérifie que la liste embarquée couvre bien des domaines types.
        assert!(should_block("https://www.google-analytics.com/collect?v=1"));
        assert!(should_block("https://sb.scorecardresearch.com/b?c=1"));
        assert!(should_block("http://pixel.quantserve.com/pixel?a=1"));
        assert!(should_block("https://ad.doubleclick.net/ddm/"));
        assert!(should_block("https://ib.adnxs.com/ut/v2"));
        assert!(should_block("https://stats.g.doubleclick.net/r/collect"));
        assert!(!should_block("https://duckduckgo.com/"));
        assert!(!should_block("https://fr.wikipedia.org/wiki/Faraday"));
    }

    #[test]
    fn real_list_blocks_eff_simulators() {
        // Domaines de test EFF Cover Your Tracks (trackers simulés) :
        // bloqués, tandis que le « first-party simulator » reste autorisé.
        assert!(should_block("https://trackersimulator.org/pixel?a=1"));
        assert!(should_block("http://eviltracker.net/pixel?a=1"));
        assert!(should_block("https://www.do-not-tracker.org/collect"));
        assert!(!should_block("https://firstpartysimulator.net/x"));
        assert!(!should_block("https://firstpartysimulator.org/y"));
    }

    #[test]
    fn exceptions_override() {
        // `@@||...` doit toujours autoriser le domaine et ses sous-domaines.
        let bl = BlockList::parse("||example.com^\n@@||allow.example.com^\n");
        assert!(bl.blocks("ads.example.com"));
        assert!(!bl.blocks("allow.example.com"));
        assert!(!bl.blocks("sub.allow.example.com"));
    }

    #[test]
    fn host_normalization_for_exceptions() {
        assert_eq!(normalize_host("https://www.Example.com/path"), "example.com");
        assert_eq!(normalize_host("sub.example.com"), "sub.example.com");
        assert_eq!(normalize_host("http://example.com:8080/x?y=1"), "example.com");
        assert_eq!(normalize_host("banque.fr."), "banque.fr");
        // « www.com » ne doit pas devenir « com »
        assert_eq!(normalize_host("www.com"), "www.com");
    }

    #[test]
    fn per_site_exceptions() {
        // Déblocage ponctuel : un domaine exempté couvre ses sous-domaines.
        assert!(!is_exempt("banque.fr"));
        set_exempt("https://www.banque.fr/connexion", true);
        assert!(is_exempt("banque.fr"));
        assert!(is_exempt("www.banque.fr"));
        assert!(is_exempt("secure.banque.fr"));
        assert!(!is_exempt("autre.fr"));
        assert!(exempt_list().contains(&"banque.fr".to_string()));
        set_exempt("banque.fr", false);
        assert!(!is_exempt("banque.fr"));
        assert!(exempt_list().is_empty());
    }
}
