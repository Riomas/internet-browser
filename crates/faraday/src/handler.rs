//! Client CEF : cycle de vie du navigateur + blocage réseau des trackers.
//!
//! Le `RequestHandler::on_before_resource_load` rejette les requêtes vers des
//! domaines de tracking/publicité connus (liste simple en Phase 0 ; une vraie
//! liste type EasyList sera intégrée en Phase 2).

use cef::*;
use std::sync::{Arc, Mutex, OnceLock, Weak};

use crate::downloads::{self, DownloadEntry, DownloadState, Downloads};
use crate::history::{self, History};

/// Liste de domaines de tracking / publicité bloqués (extrait — Phase 0).
/// Sera remplacée par une vraie liste (EasyList + anti-track) en Phase 2.
const TRACKER_DOMAINS: &[&str] = &[
    "doubleclick.net",
    "googletagmanager.com",
    "google-analytics.com",
    "facebook.net",
    "scorecardresearch.com",
];

/// Domaines qui ne doivent **jamais** être bloqués (sites de confiance).
const ALLOWED_DOMAINS: &[&str] = &["duckduckgo.com"];

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
    }

    impl Client {
        fn display_handler(&self) -> Option<DisplayHandler> {
            Some(FaradayDisplayHandler::new(self.inner.clone(), self.history.clone()))
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
            Some(FaradayDownloadHandler::new(self.downloads.clone()))
        }
    }
}

/// Convertit une `CefStringUserfree` (UTF-16) en `String` Rust.
fn userfree_to_string(raw: &CefStringUserfree) -> String {
    CefStringUtf16::from(raw).to_string()
}

wrap_download_handler! {
    struct FaradayDownloadHandler {
        downloads: Downloads,
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
            {
                let mut list = self.downloads.lock().unwrap();
                if let Some(e) = list.iter_mut().find(|e| e.id == id) {
                    url = e.url.clone();
                    name = e.name.clone();
                    path = e.path.clone();
                    cancel = e.cancel_requested;
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

            // Annulation demandée par l'UI : on exécute le callback ici.
            if cancel {
                if let Some(cb) = callback {
                    cb.cancel();
                }
                let mut list = self.downloads.lock().unwrap();
                if let Some(e) = list.iter_mut().find(|e| e.id == id) {
                    e.state = DownloadState::Cancelled;
                }
            }
        }
    }
}

wrap_display_handler! {
    struct FaradayDisplayHandler {
        inner: Arc<Mutex<FaradayHandler>>,
        history: History,
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
            _frame: Option<&mut Frame>,
            request: Option<&mut Request>,
            _callback: Option<&mut Callback>,
        ) -> ReturnValue {
            // Bloque les requêtes vers les domaines de tracking/publicité.
            let Some(req) = request else {
                return ReturnValue::CONTINUE;
            };
            let url = {
                let raw = req.url();
                CefStringUtf16::from(&raw).to_string().to_lowercase()
            };

            if TRACKER_DOMAINS.iter().any(|d| url.contains(d)) {
                // Ne pas bloquer les domaines autorisés.
                if !ALLOWED_DOMAINS.iter().any(|d| url.contains(d)) {
                    return ReturnValue::CANCEL;
                }
            }
            ReturnValue::CONTINUE
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
