//! Client CEF : cycle de vie du navigateur + blocage réseau des trackers.
//!
//! Le `RequestHandler::on_before_resource_load` rejette les requêtes vers des
//! domaines de tracking/publicité connus (liste simple en Phase 0 ; une vraie
//! liste type EasyList sera intégrée en Phase 2).

use cef::*;
use std::sync::{Arc, Mutex, OnceLock, Weak};

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
    }

    impl Client {
        fn display_handler(&self) -> Option<DisplayHandler> {
            Some(FaradayDisplayHandler::new(self.inner.clone()))
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
    }
}

wrap_display_handler! {
    struct FaradayDisplayHandler {
        inner: Arc<Mutex<FaradayHandler>>,
    }

    impl DisplayHandler {
        fn on_title_change(&self, browser: Option<&mut Browser>, title: Option<&CefString>) {
            // Défini plus tard (titre d'onglet, Phase 1).
            let _ = (browser, title);
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
