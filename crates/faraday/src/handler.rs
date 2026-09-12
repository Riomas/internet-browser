//! Client CEF : cycle de vie du navigateur + blocage réseau des trackers.
//!
//! Le `ResourceRequestHandler::on_before_resource_load` refuse les requêtes
//! vers les domaines listés dans `configs/blocklist.txt` (moteur Phase 2) et
//! injecte l'en-tête Do-Not-Track (`DNT: 1`) sur chaque requête.

use cef::*;
use std::sync::{Arc, Mutex, OnceLock, Weak};

use crate::blocklist;
use crate::downloads::{
    self, DownloadEntry, DownloadNoticeKind, DownloadNotices, DownloadState, Downloads,
};
use crate::history::{self, History};

/// Tampon de pixels du rendu OSR d'un onglet (mode CEF windowless).
pub struct RenderBuffer {
    pub data: Vec<u8>,
    pub width: usize,
    pub height: usize,
    pub dirty: bool,
}

impl RenderBuffer {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            width: 0,
            height: 0,
            dirty: false,
        }
    }
}

/// Taille de la zone de rendu partagée par tous les onglets.
pub type ViewSize = Arc<Mutex<(usize, usize)>>;

/// Dernier message de statut (URL survolée) d'un onglet.
pub type StatusCell = Arc<Mutex<Option<String>>>;

static HANDLER: OnceLock<Weak<Mutex<FaradayHandler>>> = OnceLock::new();

pub struct FaradayHandler {
    browser: Option<Browser>,
    is_closing: bool,
}

impl FaradayHandler {
    #[allow(dead_code)]
    pub fn instance() -> Option<Arc<Mutex<Self>>> {
        HANDLER.get().and_then(Weak::upgrade)
    }

    pub fn new() -> Arc<Mutex<Self>> {
        let handler = Arc::new(Mutex::new(Self {
            browser: None,
            is_closing: false,
        }));
        HANDLER.set(Arc::downgrade(&handler)).ok();
        handler
    }

    fn on_after_created(&mut self, browser: Option<&mut Browser>) {
        let Some(browser) = browser else { return };
        self.browser = Some(browser.clone());
    }

    fn on_before_close(&mut self, _browser: Option<&mut Browser>) {
        self.is_closing = true;
    }
}

wrap_client! {
    pub struct FaradayClient {
        inner: Arc<Mutex<FaradayHandler>>,
        buffer: Arc<Mutex<RenderBuffer>>,
        view_size: ViewSize,
        history: History,
        downloads: Downloads,
        notices: DownloadNotices,
        status: StatusCell,
    }

    impl Client {
        fn display_handler(&self) -> Option<DisplayHandler> {
            Some(FaradayDisplayHandler::new(
                self.inner.clone(),
                self.history.clone(),
                self.status.clone(),
            ))
        }

        fn life_span_handler(&self) -> Option<LifeSpanHandler> {
            Some(FaradayLifeSpanHandler::new(self.inner.clone()))
        }

        fn load_handler(&self) -> Option<LoadHandler> {
            Some(FaradayLoadHandler::new(self.inner.clone()))
        }

        fn request_handler(&self) -> Option<RequestHandler> {
            Some(FaradayRequestHandler::new(self.inner.clone()))
        }

        fn render_handler(&self) -> Option<RenderHandler> {
            Some(FaradayRenderHandler::new(self.buffer.clone(), self.view_size.clone()))
        }

        fn download_handler(&self) -> Option<DownloadHandler> {
            Some(FaradayDownloadHandler::new(
                self.downloads.clone(),
                self.notices.clone(),
            ))
        }
    }
}

/// Convertit une `CefStringUserfree` (UTF-16) en `String` Rust.
fn userfree_to_string(raw: &CefStringUserfree) -> String {
    CefStringUtf16::from(raw).to_string()
}

/// Journal de diagnostic technique (aide au dépannage, **désactivé** par
/// défaut : activer avec la variable d'environnement `FARADAY_DIAG=1`).
///
/// Respect de la vie privée : on n'écrit **que des noms d'hôtes** de requêtes
/// déjà listées comme pistes potentielles, jamais d'URL complète, jamais de
/// chemin, de paramètre ni de cookie.
fn diag(line: &str) {
    use std::io::Write;
    static LOG: OnceLock<Mutex<Option<std::fs::File>>> = OnceLock::new();
    let log = LOG.get_or_init(|| {
        if std::env::var_os("FARADAY_DIAG").is_none() {
            return Mutex::new(None);
        }
        let dir = crate::privacy::appdata_dir();
        let _ = std::fs::create_dir_all(&dir);
        Mutex::new(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(dir.join("diag.log"))
                .ok(),
        )
    });
    if let Ok(mut guard) = log.lock() {
        if let Some(file) = guard.as_mut() {
            let _ = writeln!(file, "{line}");
        }
    }
}

/// Hôte du document de **plus haut niveau** (« first party » réel), obtenu en
/// remontant la chaîne des frames depuis la frame qui déclenche la requête.
///
/// Pourquoi ne pas se contenter de `Request::first_party_for_cookies()` ?
/// Ce champ n'est renseigné par CEF que pour les requêtes construites via
/// `CefURLRequest` ; pour les requêtes réseau ordinaires il peut être **vide**.
/// Or c'est lui qui sert à décider si une requête est « tierce partie » et si
/// le site courant est exempté : s'il est vide, le déblocage par site ne
/// s'applique jamais aux ressources tierces (et les cookies tiers ne sont plus
/// filtrés). On s'appuie donc d'abord sur la frame de plus haut niveau, que
/// CEF nous fournit toujours dans les rappels de requête.
fn top_level_host(frame: Option<&Frame>) -> String {
    let Some(frame) = frame else {
        return String::new();
    };
    let mut current = frame.clone();
    // Garde-fou : profondeur d'imbrication volontairement large.
    for _ in 0..32 {
        if current.is_main() != 0 {
            break;
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }
    blocklist::host_of(&userfree_to_string(&current.url()))
}

/// Domaine « enregistrable » approximatif : les deux derniers labels de l'hôte
/// (équivalent simplifié de eTLD+1, sans liste publique). Ex. `img.eff.org` →
/// `eff.org`, `eff.org` → `eff.org`.
fn registrable_domain(host: &str) -> String {
    let labels: Vec<&str> = host.split('.').filter(|s| !s.is_empty()).collect();
    if labels.len() <= 2 {
        host.to_string()
    } else {
        labels[labels.len() - 2..].join(".")
    }
}

/// Vrai si un hôte est « tierce partie » par rapport au first-party donné.
/// Même site (même domaine enregistrable, y compris sous-domaines) => 1re partie.
fn host_is_third_party(host: &str, first_party: &str) -> bool {
    if host.is_empty() || first_party.is_empty() {
        return false;
    }
    registrable_domain(host) != registrable_domain(first_party)
}

/// Hôte du site principal (« first party ») tel que rapporté par CEF pour la
/// requête (peut être vide : voir `top_level_host`).
fn request_first_party_host(request: &Request) -> String {
    let raw = request.first_party_for_cookies();
    blocklist::host_of(&userfree_to_string(&raw))
}

/// Contexte « site » d'une requête : (document de plus haut niveau, valeur
/// fournie par CEF). L'un des deux peut être vide.
fn site_hosts(request: &Request, frame: Option<&Frame>) -> (String, String) {
    (top_level_host(frame), request_first_party_host(request))
}

/// Vrai si la requête est « tierce partie » : son hôte diffère de celui du site
/// affiché (document de plus haut niveau, sinon le contexte de cookies CEF).
fn is_third_party_request(request: &Request, frame: Option<&Frame>) -> bool {
    let url = {
        let raw = request.url();
        userfree_to_string(&raw)
    };
    let (top, from_request) = site_hosts(request, frame);
    let first_party = if top.is_empty() { from_request } else { top };
    host_is_third_party(&blocklist::host_of(&url), &first_party)
}

/// Vrai si la protection est **désactivée pour le site courant**
/// (« déblocage ponctuel ») : on n'applique alors ni le blocage réseau, ni le
/// blocage des cookies tiers.
fn is_exempt_request(request: &Request, frame: Option<&Frame>) -> bool {
    let (top, from_request) = site_hosts(request, frame);
    (!top.is_empty() && blocklist::is_exempt(&top))
        || (!from_request.is_empty() && blocklist::is_exempt(&from_request))
}

// Filtre de cookies : refuse l'envoi/l'enregistrement des cookies tiers
// (blocage réel du suivi inter-sites, testé par EFF Cover Your Tracks).
wrap_cookie_access_filter! {
    struct FaradayCookieAccessFilter;

    impl CookieAccessFilter {
        fn can_send_cookie(
            &self,
            _browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            request: Option<&mut Request>,
            _cookie: Option<&Cookie>,
        ) -> ::std::os::raw::c_int {
            let Some(r) = request else { return 1 };
            // Protection suspendue : aucun filtrage des cookies tiers.
            if blocklist::paused() {
                return 1;
            }
            let frame = frame.as_deref();
            if !is_exempt_request(r, frame) && is_third_party_request(r, frame) {
                let (top, fp) = site_hosts(r, frame);
                diag(&format!(
                    "COOKIE tiers non envoye {} (site={} cef={})",
                    blocklist::host_of(&userfree_to_string(&r.url())),
                    if top.is_empty() { "-" } else { &top },
                    if fp.is_empty() { "-" } else { &fp },
                ));
                return 0;
            }
            1
        }

        fn can_save_cookie(
            &self,
            _browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            request: Option<&mut Request>,
            _response: Option<&mut Response>,
            _cookie: Option<&Cookie>,
        ) -> ::std::os::raw::c_int {
            let Some(r) = request else { return 1 };
            // Protection suspendue : aucun filtrage des cookies tiers.
            if blocklist::paused() {
                return 1;
            }
            let frame = frame.as_deref();
            if !is_exempt_request(r, frame) && is_third_party_request(r, frame) {
                diag(&format!(
                    "COOKIE tiers non enregistre {}",
                    blocklist::host_of(&userfree_to_string(&r.url()))
                ));
                return 0;
            }
            1
        }
    }
}

wrap_download_handler! {
    struct FaradayDownloadHandler {
        downloads: Downloads,
        notices: DownloadNotices,
    }

    impl DownloadHandler {
        fn can_download(
            &self,
            _browser: Option<&mut Browser>,
            _url: Option<&CefString>,
            _request_method: Option<&CefString>,
        ) -> ::std::os::raw::c_int {
            // Le défaut de cef-rs renvoie 0 (bloque le téléchargement).
            // Faraday autorise les téléchargements.
            1
        }

        fn on_before_download(
            &self,
            _browser: Option<&mut Browser>,
            download_item: Option<&mut DownloadItem>,
            suggested_name: Option<&CefString>,
            callback: Option<&mut BeforeDownloadCallback>,
        ) -> ::std::os::raw::c_int {
            // « Return true (1) and execute callback » : on choisit le chemin
            // de destination (dossier Téléchargements) puis on continue.
            let Some(item) = download_item else { return 1 };
            let Some(cb) = callback else { return 1 };

            let id = item.id();
            let url = userfree_to_string(&item.original_url());
            let mut name = suggested_name
                .map(|s| s.to_string())
                .unwrap_or_default();
            if name.trim().is_empty() {
                name = userfree_to_string(&item.suggested_file_name());
            }
            let safe = downloads::safe_file_name(if name.trim().is_empty() {
                "telechargement"
            } else {
                name.trim()
            });

            // Dossier de destination (créé si besoin) + nom unique.
            let dir = downloads::downloads_dir();
            let _ = std::fs::create_dir_all(&dir);
            let path = downloads::unique_path(&dir, &safe);

            {
                let mut list = self.downloads.lock().unwrap();
                downloads::upsert(
                    &mut list,
                    DownloadEntry {
                        id,
                        url: url.clone(),
                        name: safe.clone(),
                        path: path.to_string_lossy().to_string(),
                        state: DownloadState::Starting,
                        percent: 0,
                        speed: 0,
                        received: 0,
                        total: 0,
                        cancel_requested: false,
                    },
                );
            }

            // Notifie l'UI : le téléchargement démarre.
            downloads::notify(
                &mut self.notices.lock().unwrap(),
                &safe,
                DownloadNoticeKind::Started,
            );

            let path_str = path.to_string_lossy().to_string();
            let full = CefString::from(path_str.as_str());
            cb.cont(Some(&full), 0);
            1
        }

        fn on_download_updated(
            &self,
            _browser: Option<&mut Browser>,
            download_item: Option<&mut DownloadItem>,
            callback: Option<&mut DownloadItemCallback>,
        ) {
            let Some(item) = download_item else { return };
            let id = item.id();

            let state = if item.is_complete() != 0 {
                DownloadState::Complete
            } else if item.is_canceled() != 0 {
                DownloadState::Cancelled
            } else if item.is_interrupted() != 0 {
                DownloadState::Interrupted
            } else if item.is_in_progress() != 0 {
                DownloadState::InProgress
            } else {
                DownloadState::Starting
            };

            let full_path = {
                let raw = item.full_path();
                userfree_to_string(&raw)
            };

            // Récupère l'état existant (sans garder d'emprunt sur la liste).
            let (mut url, mut name, mut path, mut cancel) =
                (String::new(), String::new(), String::new(), false);
            let mut prev: Option<DownloadState> = None;
            {
                let mut list = self.downloads.lock().unwrap();
                if let Some(e) = list.iter_mut().find(|e| e.id == id) {
                    url = e.url.clone();
                    name = e.name.clone();
                    path = e.path.clone();
                    cancel = e.cancel_requested;
                    prev = Some(e.state);
                }
            }

            if name.trim().is_empty() {
                let raw = item.suggested_file_name();
                name = userfree_to_string(&raw);
            }
            if url.is_empty() {
                let raw = item.original_url();
                url = userfree_to_string(&raw);
            }
            if !full_path.is_empty() {
                path = full_path;
            }
            let notice_name = name.clone();

            {
                let mut list = self.downloads.lock().unwrap();
                downloads::upsert(
                    &mut list,
                    DownloadEntry {
                        id,
                        url,
                        name,
                        path,
                        state,
                        percent: item.percent_complete(),
                        speed: item.current_speed(),
                        received: item.received_bytes() as u64,
                        total: item.total_bytes() as u64,
                        cancel_requested: false,
                    },
                );
            }

            // Notifications de fin (terminé / annulé / interrompu).
            let mut notified_end = false;
            let notice_kind = if prev.is_none()
                && matches!(state, DownloadState::Starting | DownloadState::InProgress)
            {
                Some(DownloadNoticeKind::Started)
            } else if prev.is_some() && prev != Some(state) {
                match state {
                    DownloadState::Complete => Some(DownloadNoticeKind::Complete),
                    DownloadState::Cancelled => Some(DownloadNoticeKind::Cancelled),
                    DownloadState::Interrupted => Some(DownloadNoticeKind::Interrupted),
                    _ => None,
                }
            } else {
                None
            };
            if let Some(kind) = notice_kind {
                downloads::notify(&mut self.notices.lock().unwrap(), &notice_name, kind);
                notified_end = matches!(
                    kind,
                    DownloadNoticeKind::Complete
                        | DownloadNoticeKind::Cancelled
                        | DownloadNoticeKind::Interrupted
                );
            }

            // Annulation demandée par l'UI : on exécute le callback ici.
            if cancel {
                if let Some(cb) = callback {
                    cb.cancel();
                }
                let mut list = self.downloads.lock().unwrap();
                if let Some(e) = list.iter_mut().find(|e| e.id == id) {
                    e.state = DownloadState::Cancelled;
                }
                if !notified_end {
                    downloads::notify(
                        &mut self.notices.lock().unwrap(),
                        &notice_name,
                        DownloadNoticeKind::Cancelled,
                    );
                }
            }
        }
    }
}

wrap_display_handler! {
    struct FaradayDisplayHandler {
        inner: Arc<Mutex<FaradayHandler>>,
        history: History,
        status: StatusCell,
    }

    impl DisplayHandler {
        fn on_title_change(&self, browser: Option<&mut Browser>, title: Option<&CefString>) {
            // Défini plus tard (titre d'onglet).
            let _ = (browser, title);
        }

        fn on_address_change(
            &self,
            _browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            url: Option<&CefString>,
        ) {
            // N'enregistre que les navigations de la frame principale.
            let is_main = frame.map(|f| f.is_main() == 1).unwrap_or(false);
            if !is_main {
                return;
            }
            if let Some(url) = url {
                history::push(&self.history, &url.to_string());
            }
        }

        fn on_status_message(
            &self,
            _browser: Option<&mut Browser>,
            value: Option<&CefString>,
        ) {
            // CEF remplit ce message quand le pointeur survole un lien (URL
            // cible) et le vide quand il en sort → barre de statut en bas.
            let text = value.map(|v| v.to_string()).unwrap_or_default();
            let mut cell = self.status.lock().unwrap();
            if text.is_empty() {
                *cell = None;
            } else {
                *cell = Some(text);
            }
        }
    }
}

wrap_life_span_handler! {
    struct FaradayLifeSpanHandler {
        inner: Arc<Mutex<FaradayHandler>>,
    }

    impl LifeSpanHandler {
        fn on_after_created(&self, browser: Option<&mut Browser>) {
            let mut inner = self.inner.lock().unwrap();
            inner.on_after_created(browser);
        }

        fn do_close(&self, browser: Option<&mut Browser>) -> i32 {
            let mut inner = self.inner.lock().unwrap();
            inner.on_before_close(browser);
            0
        }
    }
}

wrap_load_handler! {
    struct FaradayLoadHandler {
        inner: Arc<Mutex<FaradayHandler>>,
    }

    impl LoadHandler {
        fn on_load_error(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _error_code: Errorcode,
            _error_text: Option<&CefString>,
            _failed_url: Option<&CefString>,
        ) {
            // Page d'erreur personnalisée (Phase 1).
        }
    }
}

wrap_request_handler! {
    struct FaradayRequestHandler {
        inner: Arc<Mutex<FaradayHandler>>,
    }

    impl RequestHandler {
        fn resource_request_handler(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _request: Option<&mut Request>,
            _is_navigation: ::std::os::raw::c_int,
            _is_download: ::std::os::raw::c_int,
            _request_initiator: Option<&CefString>,
            _disable_default_handling: Option<&mut ::std::os::raw::c_int>,
        ) -> Option<ResourceRequestHandler> {
            Some(FaradayResourceRequestHandler::new(self.inner.clone()))
        }
    }
}

wrap_resource_request_handler! {
    struct FaradayResourceRequestHandler {
        inner: Arc<Mutex<FaradayHandler>>,
    }

    impl ResourceRequestHandler {
        fn on_before_resource_load(
            &self,
            _browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            request: Option<&mut Request>,
            _callback: Option<&mut Callback>,
        ) -> ReturnValue {
            let Some(req) = request else {
                return ReturnValue::CONTINUE;
            };

            // Blocage des domaines de tracking/publicité (moteur Phase 2),
            // sauf si le site courant est exempté (« déblocage ponctuel »).
            {
                let url = {
                    let raw = req.url();
                    CefStringUtf16::from(&raw).to_string()
                };
                // Toute requete correspondant a une regle est journalisee (mode
                // FARADAY_DIAG), meme si elle est finalement autorisee : c'est ce qui
                // permet de verifier sans ambiguite le debocage par site ou la
                // suspension temporaire de la protection.
                if blocklist::matches_rules(&url) {
                    let (top, fp) = site_hosts(req, frame.as_deref());
                    let exempt = is_exempt_request(req, frame.as_deref());
                    let paused = blocklist::paused();
                    // Décision réelle : règles + réglage persistant + suspension,
                    // puis déblocage éventuel du site affiché.
                    let blocked = !exempt && blocklist::should_block(&url);
                    let raison = if paused {
                        "protection suspendue"
                    } else if !blocklist::enabled() {
                        "blocage desactive"
                    } else if exempt {
                        "site exempte"
                    } else {
                        "-"
                    };
                    diag(&format!(
                        "RESEAU {} (site={} cef={} exempt={} raison={}) -> {}",
                        blocklist::host_of(&url),
                        if top.is_empty() { "-" } else { &top },
                        if fp.is_empty() { "-" } else { &fp },
                        if exempt { "oui" } else { "non" },
                        raison,
                        if blocked { "BLOQUE" } else { "AUTORISE" },
                    ));
                    if blocked {
                        // Le blocage est attribue au site qui a declenche la requete
                        // (premiere partie). Ainsi le compteur du bouclier tombe a 0
                        // pour un site exempte, meme si la page fait charger d'autres
                        // sites qui, eux, restent bloques.
                        let site = if top.is_empty() { fp } else { top };
                        blocklist::incr_blocked_for(&site);
                        diag(&format!(
                            "  compteurs : {} = {}, total = {}",
                            if site.is_empty() { "-" } else { &site },
                            blocklist::blocked_for(&site),
                            blocklist::blocked_count()
                        ));
                        return ReturnValue::CANCEL;
                    }
                }
            }

            // Do-Not-Track : en-tête envoyé à chaque requête (si activé).
            if blocklist::dnt_enabled() {
                let dnt_name = CefString::from("DNT");
                let dnt_value = CefString::from("1");
                req.set_header_by_name(Some(&dnt_name), Some(&dnt_value), 1);
            }

            ReturnValue::CONTINUE
        }

        fn cookie_access_filter(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _request: Option<&mut Request>,
        ) -> Option<CookieAccessFilter> {
            Some(FaradayCookieAccessFilter::new())
        }
    }
}

wrap_render_handler! {
    struct FaradayRenderHandler {
        buffer: Arc<Mutex<RenderBuffer>>,
        view_size: ViewSize,
    }

    impl RenderHandler {
        fn view_rect(&self, _browser: Option<&mut Browser>, rect: Option<&mut Rect>) {
            // Taille de la zone de rendu (doit être non nulle en OSR sinon
            // CEF déclenche un DCHECK à la création du browser).
            if let Some(rect) = rect {
                let (w, h) = *self.view_size.lock().unwrap();
                if w > 0 && h > 0 {
                    rect.width = w as i32;
                    rect.height = h as i32;
                } else {
                    rect.width = 1280;
                    rect.height = 800;
                }
                rect.x = 0;
                rect.y = 0;
            }
        }

        fn on_paint(
            &self,
            _browser: Option<&mut Browser>,
            _type_: PaintElementType,
            _dirty_rects: Option<&[Rect]>,
            buffer: *const u8,
            width: ::std::os::raw::c_int,
            height: ::std::os::raw::c_int,
        ) {
            let mut buf = self.buffer.lock().unwrap();
            let size = (width * height * 4) as usize;
            if size > 0 && !buffer.is_null() {
                unsafe {
                    buf.data = std::slice::from_raw_parts(buffer, size).to_vec();
                }
                buf.width = width as usize;
                buf.height = height as usize;
                buf.dirty = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn third_party_detection() {
        // Domaines enregistrables différents => tierce partie.
        assert!(host_is_third_party("trackersimulator.org", "eff.org"));
        assert!(host_is_third_party("ads.doubleclick.net", "example.com"));
        // Même site, y compris entre sous-domaines frères => première partie.
        assert!(!host_is_third_party("www.eff.org", "eff.org"));
        assert!(!host_is_third_party("img.eff.org", "www.eff.org"));
        assert!(!host_is_third_party("cdn2.doubleclick.net", "ads.doubleclick.net"));
        assert!(!host_is_third_party("", "eff.org"));
    }
}
