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

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

/// Compteur global de requêtes bloquées (affiché sur le bouclier).
static BLOCKED: AtomicU64 = AtomicU64::new(0);

/// Liste embarquée au moment de la compilation.
const BLOCKLIST_SOURCE: &str = include_str!("../configs/blocklist.txt");

/// Nombre de requêtes bloquées depuis le démarrage.
pub fn blocked_count() -> u64 {
    BLOCKED.load(Ordering::Relaxed)
}

/// Incrémente le compteur de blocages (appelé quand on refuse une requête).
pub fn incr_blocked() {
    BLOCKED.fetch_add(1, Ordering::Relaxed);
}

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
}
