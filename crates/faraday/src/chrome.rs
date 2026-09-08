//! Chrome du navigateur : UI egui (onglets + barre d'adresse) et rendu de la
//! page web via CEF en rendu hors-écran (OSR).

use eframe::egui;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cef::*;

use crate::blocklist;
use crate::downloads::{
    DownloadEntry, DownloadNotice, DownloadNoticeKind, DownloadNotices, DownloadState, Downloads,
};
use crate::handler::{FaradayClient, FaradayHandler, RenderBuffer, StatusCell, ViewSize};
use crate::history::{History, HistoryEntry};
use crate::icons;
use crate::privacy::PrivacyConfig;
use crate::session;

// --- Transformation clavier / souris ---
const FLAG_SHIFT: u32 = 1 << 1;
const FLAG_CONTROL: u32 = 1 << 2;
const FLAG_ALT: u32 = 1 << 3;

fn cef_modifiers(m: egui::Modifiers) -> u32 {
    let mut f = 0;
    if m.shift { f |= FLAG_SHIFT; }
    if m.ctrl { f |= FLAG_CONTROL; }
    if m.alt { f |= FLAG_ALT; }
    f
}

/// Mapping `egui::Key` → code clavier Windows (VK).
fn vk_for_key(key: egui::Key) -> i32 {
    use egui::Key::*;
    match key {
        Enter => 0x0D,
        Backspace => 0x08,
        Tab => 0x09,
        Escape => 0x1B,
        Space => 0x20,
        ArrowUp => 0x26,
        ArrowDown => 0x28,
        ArrowLeft => 0x25,
        ArrowRight => 0x27,
        Delete => 0x2E,
        Home => 0x24,
        End => 0x23,
        PageUp => 0x21,
        PageDown => 0x22,
        A => 0x41, B => 0x42, C => 0x43, D => 0x44, E => 0x45, F => 0x46, G => 0x47,
        H => 0x48, I => 0x49, J => 0x4A, K => 0x4B, L => 0x4C, M => 0x4D, N => 0x4E,
        O => 0x4F, P => 0x50, Q => 0x51, R => 0x52, S => 0x53, T => 0x54, U => 0x55,
        V => 0x56, W => 0x57, X => 0x58, Y => 0x59, Z => 0x5A,
        Num0 => 0x30, Num1 => 0x31, Num2 => 0x32, Num3 => 0x33, Num4 => 0x34,
        Num5 => 0x35, Num6 => 0x36, Num7 => 0x37, Num8 => 0x38, Num9 => 0x39,
        _ => 0,
    }
}

/// Code clavier Windows (VK) pour un caractère saisi (lettres/chiffres/espace).
fn vk_for_char(c: char) -> i32 {
    match c {
        'a'..='z' => 0x41 + (c as i32 - 'a' as i32),
        'A'..='Z' => 0x41 + (c as i32 - 'A' as i32),
        '0'..='9' => 0x30 + (c as i32 - '0' as i32),
        ' ' => 0x20,
        '\n' | '\r' => 0x0D,
        '\t' => 0x09,
        _ => 0,
    }
}

/// Caractère produit par une touche `egui` (lettres/chiffres/espace).
fn key_to_char(key: egui::Key) -> Option<char> {
    use egui::Key::*;
    Some(match key {
        A => 'a', B => 'b', C => 'c', D => 'd', E => 'e', F => 'f', G => 'g',
        H => 'h', I => 'i', J => 'j', K => 'k', L => 'l', M => 'm', N => 'n',
        O => 'o', P => 'p', Q => 'q', R => 'r', S => 's', T => 't', U => 'u',
        V => 'v', W => 'w', X => 'x', Y => 'y', Z => 'z',
        Num0 => '0', Num1 => '1', Num2 => '2', Num3 => '3', Num4 => '4',
        Num5 => '5', Num6 => '6', Num7 => '7', Num8 => '8', Num9 => '9',
        Space => ' ',
        _ => return None,
    })
}



/// Nom d'hôte d'une URL (pour le libellé d'un onglet).
fn hostname(url: &str) -> String {
    let mut rest = url.trim();
    for p in ["https://", "http://", "ftp://"] {
        if let Some(r) = rest.strip_prefix(p) {
            rest = r;
            break;
        }
    }
    match rest.find('/') {
        Some(i) => rest[..i].to_string(),
        None => rest.to_string(),
    }
}

/// Un lien rapide proposé sur la page de nouvel onglet.
struct QuickLink {
    label: &'static str,
    url: &'static str,
    icon: &'static str,
    color: egui::Color32,
}

/// Raccourcis par défaut de la page d'accueil.
const QUICK_LINKS: &[QuickLink] = &[
    QuickLink { label: "DuckDuckGo", url: "https://duckduckgo.com", icon: icons::GLOBE, color: egui::Color32::from_rgb(222, 88, 51) },
    QuickLink { label: "Wikipedia", url: "https://fr.wikipedia.org", icon: icons::ARTICLE, color: egui::Color32::from_rgb(84, 89, 93) },
    QuickLink { label: "GitHub", url: "https://github.com", icon: icons::GITHUB_LOGO, color: egui::Color32::from_rgb(110, 84, 148) },
    QuickLink { label: "EFF", url: "https://www.eff.org", icon: icons::CERTIFICATE, color: egui::Color32::from_rgb(23, 23, 23) },
    QuickLink { label: "YouTube", url: "https://www.youtube.com", icon: icons::VIDEO, color: egui::Color32::from_rgb(255, 0, 0) },
    QuickLink { label: "Actualités", url: "https://www.lemonde.fr", icon: icons::NEWSPAPER, color: egui::Color32::from_rgb(58, 98, 181) },
    QuickLink { label: "Internet Archive", url: "https://archive.org", icon: icons::BOOKMARK, color: egui::Color32::from_rgb(90, 74, 140) },
    QuickLink { label: "Framasoft", url: "https://framasoft.org", icon: icons::FINGERPRINT, color: egui::Color32::from_rgb(51, 153, 255) },
];

/// Vrai si la saisie ressemble à une URL (sinon → moteur de recherche).
fn looks_like_url(input: &str) -> bool {
    let s = input.trim();
    s.contains("://") || (s.contains('.') && !s.chars().any(char::is_whitespace))
}

/// Encode une requête pour une URL de recherche (percent-encoding minimal).
fn url_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Construit l'URL cible depuis une saisie, avec le moteur de recherche par
/// défaut (`search_engine`) comme base pour les requêtes.
fn resolve_input(input: &str, search_engine: &str) -> String {
    let s = input.trim();
    if s.is_empty() {
        return String::new();
    }
    if looks_like_url(s) {
        if s.contains("://") {
            s.to_string()
        } else {
            format!("https://{s}")
        }
    } else {
        let base = search_engine.trim_end_matches('/');
        format!("{base}/?q={}", url_encode(s))
    }
}

/// Formate une vitesse de téléchargement (octets/s) en texte lisible.
fn format_speed(bytes_per_sec: i64) -> String {
    let b = bytes_per_sec.max(0) as f64;
    if b >= 1024.0 * 1024.0 {
        format!("{:.1} Mo/s", b / (1024.0 * 1024.0))
    } else if b >= 1024.0 {
        format!("{:.0} Ko/s", b / 1024.0)
    } else {
        format!("{b:.0} o/s")
    }
}

/// Un onglet : son navigateur CEF (OSR), son tampon de pixels et sa texture egui.
pub struct Tab {
    url: String,
    buffer: Arc<Mutex<RenderBuffer>>,
    browser: Option<Browser>,
    texture: Option<egui::TextureHandle>,
    /// Dernier message de statut (URL survolée par le pointeur).
    status: StatusCell,
}

/// Une notification temporaire (toast) affichée en haut à droite.
#[derive(Clone)]
struct Toast {
    title: &'static str,
    detail: String,
    icon: &'static str,
    color: egui::Color32,
    start: Instant,
    life: f32,
}

/// Commande choisie dans le menu contextuel (clic droit).
#[derive(Clone, Copy, PartialEq)]
enum CtxCommand {
    CopyLink,
    OpenLinkTab,
    OpenLinkHere,
    CopyPage,
    Reload,
    Back,
    Forward,
}

/// État du menu contextuel (position + lien sous le pointeur).
struct CtxMenu {
    pos: egui::Pos2,
    link: String,
}

pub struct FaradayChrome {
    /// Taille de la zone de rendu (partagée par tous les onglets).
    view_size: ViewSize,
    tabs: Vec<Tab>,
    active: usize,
    /// Contenu de la barre d'adresse (onglet actif).
    url: String,
    /// Moteur de recherche par défaut (base des requêtes de la barre/NTP).
    search_engine: String,
    /// Historique de navigation partagé (rempli par les handlers CEF).
    history: History,
    /// Téléchargements partagés (remplis par le DownloadHandler CEF).
    downloads: Downloads,
    /// Notifications de téléchargement en attente d'affichage.
    notices: DownloadNotices,
    /// Toasts actuellement visibles.
    toasts: Vec<Toast>,
    /// La fenêtre « Historique » est ouverte ?
    show_history: bool,
    /// La fenêtre « Téléchargements » est ouverte ?
    show_downloads: bool,
    /// Requête saisie dans la barre de la page de nouvel onglet.
    ntp_query: String,
    left_down: bool,
    /// Le focus clavier est sur la page web (vs barre d'adresse).
    page_focused: bool,
    address_bar_id: Option<egui::Id>,
    /// Menu contextuel (clic droit) ouvert sur la page.
    ctx_menu: Option<CtxMenu>,
}

impl FaradayChrome {
    pub fn new(cc: &eframe::CreationContext<'_>, view_size: ViewSize) -> Self {
        // Enregistrer les polices d'icônes Phosphor (libres MIT)
        icons::setup_custom_fonts(&cc.egui_ctx);

        // Restaure la session précédente (onglets + historique).
        let session_data = session::load();
        let history: History = Arc::new(Mutex::new(session_data.history));
        let search_engine = PrivacyConfig::load().default_search_engine;
        let downloads: Downloads = Arc::new(Mutex::new(Vec::new()));
        let notices: DownloadNotices = Arc::new(Mutex::new(Vec::new()));

        let mut tabs: Vec<Tab> = session_data
            .tabs
            .iter()
            .map(|url| Tab {
                url: url.clone(),
                buffer: Arc::new(Mutex::new(RenderBuffer::new())),
                browser: None,
                texture: None,
                status: Arc::new(Mutex::new(None)),
            })
            .collect();
        if tabs.is_empty() {
            // Première utilisation : un nouvel onglet (page d'accueil).
            tabs.push(Tab {
                url: String::new(),
                buffer: Arc::new(Mutex::new(RenderBuffer::new())),
                browser: None,
                texture: None,
                status: Arc::new(Mutex::new(None)),
            });
        }
        let active = session_data.active.min(tabs.len() - 1);
        let url = tabs[active].url.clone();

        Self {
            view_size,
            tabs,
            active,
            url,
            search_engine,
            history,
            downloads,
            notices,
            toasts: Vec::new(),
            show_history: false,
            show_downloads: false,
            ntp_query: String::new(),
            left_down: false,
            page_focused: false,
            address_bar_id: None,
            ctx_menu: None,
        }
    }

    /// L'onglet actif n'a pas encore de navigateur CEF → page de nouvel onglet.
    fn active_is_new_tab(&self) -> bool {
        self.tabs
            .get(self.active)
            .map(|t| t.browser.is_none())
            .unwrap_or(true)
    }

    /// Navigue vers `target` dans l'onglet actif (crée le navigateur si NTP).
    fn navigate_to(&mut self, target: &str) {
        let resolved = resolve_input(target, &self.search_engine);
        if resolved.is_empty() {
            return;
        }
        self.url = resolved.clone();
        self.navigate();
        self.page_focused = true;
    }

    /// Crée le navigateur OSR de l'onglet `idx` (une seule fois par onglet).
    fn make_browser_for(&mut self, idx: usize) {
        if idx >= self.tabs.len() || self.tabs[idx].browser.is_some() {
            return;
        }
        let url = self.tabs[idx].url.clone();
        // Un onglet sans URL (page d'accueil) n'est matérialisé qu'à la
        // première navigation, sinon il resterait sur une page vide.
        if url.is_empty() {
            return;
        }
        let buffer = self.tabs[idx].buffer.clone();
        let client = FaradayClient::new(
            FaradayHandler::new(),
            buffer,
            self.view_size.clone(),
            self.history.clone(),
            self.downloads.clone(),
            self.notices.clone(),
            self.tabs[idx].status.clone(),
        );
        let settings = BrowserSettings {
            windowless_frame_rate: 60,
            ..Default::default()
        };
        let url = CefString::from(url.as_str());
        let window_info = WindowInfo {
            windowless_rendering_enabled: 1,
            ..Default::default()
        };

        let mut client = client;
        let browser = browser_host_create_browser_sync(
            Some(&window_info),
            Some(&mut client),
            Some(&url),
            Some(&settings),
            None,
            None,
        );
        if let Some(browser) = browser {
            if idx != self.active {
                if let Some(host) = browser.host() {
                    host.was_hidden(1);
                }
            }
            self.tabs[idx].browser = Some(browser);
        }
    }

    /// Ouvre un nouvel onglet sur `url` et le rend actif.
    fn open_tab(&mut self, url: String) {
        // Masquer l'onglet actuel avant d'en créer un nouveau.
        if let Some(b) = self.tabs[self.active].browser.as_ref() {
            if let Some(host) = b.host() {
                host.was_hidden(1);
                host.set_focus(0);
            }
        }
        let tab = Tab {
            url,
            buffer: Arc::new(Mutex::new(RenderBuffer::new())),
            browser: None,
            texture: None,
            status: Arc::new(Mutex::new(None)),
        };
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
        self.make_browser_for(self.active);
        self.url = self.tabs[self.active].url.clone();
        // Un onglet vide (page d'accueil) n'a pas de focus navigateur.
        self.page_focused = !self.url.is_empty();
    }

    /// Rend l'onglet `idx` actif (masque l'ancien, montre le nouveau).
    fn set_active(&mut self, idx: usize) {
        if idx >= self.tabs.len() || idx == self.active {
            return;
        }
        let old = self.active;
        if let Some(b) = self.tabs[old].browser.as_ref() {
            if let Some(host) = b.host() {
                host.was_hidden(1);
                host.set_focus(0);
            }
        }
        self.active = idx;
        self.make_browser_for(idx);
        if let Some(b) = self.tabs[idx].browser.as_ref() {
            if let Some(host) = b.host() {
                host.was_hidden(0);
                host.was_resized();
            }
        }
        self.url = self.tabs[idx].url.clone();
        self.page_focused = !self.url.is_empty();
    }

    /// Ferme l'onglet `idx` (jamais le dernier) et sélectionne un voisin.
    fn close_tab(&mut self, idx: usize) {
        if self.tabs.len() <= 1 || idx >= self.tabs.len() {
            return;
        }
        let was_active = idx == self.active;
        // Fermer proprement le navigateur CEF de l'onglet s'il existe.
        if let Some(b) = self.tabs[idx].browser.take() {
            if let Some(host) = b.host() {
                host.close_browser(1);
            }
        }
        self.tabs.remove(idx);
        if was_active {
            if self.active >= self.tabs.len() {
                self.active = self.tabs.len() - 1;
            }
            self.make_browser_for(self.active);
            if let Some(b) = self.tabs[self.active].browser.as_ref() {
                if let Some(host) = b.host() {
                    host.was_hidden(0);
                    host.was_resized();
                }
            }
            self.url = self.tabs[self.active].url.clone();
        } else if idx < self.active {
            self.active -= 1;
        }
    }

    /// Ferme tous les navigateurs CEF (appelé à la fermeture propre de l'app).
    fn close_all_browsers(&mut self) {
        for tab in &mut self.tabs {
            if let Some(browser) = tab.browser.take() {
                if let Some(host) = browser.host() {
                    host.close_browser(1);
                }
            }
        }
    }

    /// Convertit le tampon CEF (BGRA) de l'onglet actif en texture egui (RGBA).
    fn upload_active_texture(&mut self, ctx: &egui::Context) {
        let idx = self.active;
        let mut buf = self.tabs[idx].buffer.lock().unwrap();
        if !buf.dirty || buf.width == 0 || buf.height == 0 {
            return;
        }

        let w = buf.width;
        let h = buf.height;
        // CEF fournit du BGRA ; egui attend du RGBA.
        let mut rgba = Vec::with_capacity(w * h * 4);
        for px in buf.data.chunks_exact(4) {
            rgba.push(px[2]); // R
            rgba.push(px[1]); // G
            rgba.push(px[0]); // B
            rgba.push(px[3]); // A
        }
        let color = egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba);
        buf.dirty = false;
        drop(buf);

        match &mut self.tabs[idx].texture {
            Some(tex) => tex.set(color, egui::TextureOptions::LINEAR),
            None => {
                self.tabs[idx].texture =
                    Some(ctx.load_texture("page", color, egui::TextureOptions::LINEAR));
            }
        }
    }

    /// Accède au navigateur CEF de l'onglet actif.
    fn with_browser<R>(&self, f: impl FnOnce(&Browser) -> R) -> Option<R> {
        self.tabs.get(self.active).and_then(|t| t.browser.as_ref()).map(f)
    }

    /// Accède au `BrowserHost` de l'onglet actif (entrées OSR).
    fn with_host<R>(&self, f: impl FnOnce(&BrowserHost) -> R) -> Option<R> {
        self.with_browser(|b| b.host().map(|h| f(&h))).flatten()
    }

    fn navigate(&mut self) {
        let url = self.url.trim().to_string();
        if url.is_empty() {
            return;
        }
        self.tabs[self.active].url = url.clone();
        // Premier chargement d'un onglet « accueil » : on crée son navigateur.
        if self.tabs[self.active].browser.is_none() {
            self.make_browser_for(self.active);
            return;
        }
        let _ = self.with_browser(|b| {
            if let Some(frame) = b.main_frame() {
                frame.load_url(Some(&CefString::from(url.as_str())));
            }
        });
    }

    fn back(&mut self) {
        let _ = self.with_browser(|b| b.go_back());
    }

    fn forward(&mut self) {
        let _ = self.with_browser(|b| b.go_forward());
    }

    fn reload(&mut self) {
        let _ = self.with_browser(|b| b.reload());
    }

    /// Synchronise la taille de la zone de rendu OSR avec la taille réelle.
    /// CEF a besoin de `was_resized()` quand la vue change (sinon clic décalé).
    fn sync_view_size(&mut self, rect: egui::Rect, scale: f32) {
        let w = (rect.width() * scale) as usize;
        let h = (rect.height() * scale) as usize;
        let changed = {
            let mut vs = self.view_size.lock().unwrap();
            if *vs != (w, h) {
                *vs = (w, h);
                true
            } else {
                false
            }
        };
        if changed {
            self.with_host(|host| host.was_resized());
        }
    }

    /// Transmet les événements souris/clavier à la page (rendu OSR).
    fn forward_input(&mut self, ctx: &egui::Context, rect: egui::Rect, response: &egui::Response) {
        let scale = ctx.pixels_per_point();

        // --- Souris : déplacement, clic gauche, molette ---
        if let Some(hover) = response.hover_pos() {
            let x = ((hover.x - rect.min.x) * scale) as i32;
            let y = ((hover.y - rect.min.y) * scale) as i32;
            let mods = cef_modifiers(ctx.input(|i| i.modifiers));

            self.with_host(|host| {
                host.send_mouse_move_event(Some(&MouseEvent { x, y, modifiers: mods }), 0)
            });

            // Clic GAUCHE uniquement : `is_pointer_button_down_on()` est aussi
            // vrai pour le bouton droit, ce qui cliquerait le lien sous le
            // pointeur au lieu d'ouvrir le menu contextuel.
            let pressed = ctx.input(|i| i.pointer.button_down(egui::PointerButton::Primary));
            if pressed && !self.left_down {
                self.left_down = true;
                self.with_host(|host| {
                    host.send_mouse_click_event(
                        Some(&MouseEvent { x, y, modifiers: mods }),
                        MouseButtonType::LEFT,
                        0,
                        1,
                    )
                });
            } else if !pressed && self.left_down {
                self.left_down = false;
                self.with_host(|host| {
                    host.send_mouse_click_event(
                        Some(&MouseEvent { x, y, modifiers: mods }),
                        MouseButtonType::LEFT,
                        1,
                        1,
                    )
                });
            }

            let scroll = ctx.input(|i| i.raw_scroll_delta);
            if scroll.y != 0.0 {
                let dy = (-scroll.y) as i32;
                self.with_host(|host| {
                    host.send_mouse_wheel_event(
                        Some(&MouseEvent { x, y, modifiers: mods }),
                        0,
                        dy,
                    )
                });
            }
        }

        // --- Clic droit : ouvre notre menu contextuel (remplace le menu
        // natif Chromium, que l'on n'envoie volontairement pas à CEF). ---
        if self.ctx_menu.is_none() && response.secondary_clicked() {
            if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                self.ctx_menu = Some(CtxMenu {
                    pos,
                    link: self.hovered_link(),
                });
                // Comme un clic gauche : la page garde le focus.
                self.page_focused = true;
                if let Some(id) = self.address_bar_id {
                    ctx.memory_mut(|m| m.surrender_focus(id));
                }
                self.with_host(|host| host.set_focus(1));
            }
        }

        // --- Garder le focus clavier sur le navigateur (OSR) tant que la page
        // --- est active, sinon les événements clavier sont ignorés par CEF.
        if self.page_focused {
            self.with_host(|host| host.set_focus(1));
        }

        // --- Clavier : transmis à la page quand elle a le focus ---
        if !self.page_focused {
            return;
        }
        let mods = cef_modifiers(ctx.input(|i| i.modifiers));
        let events = ctx.input(|i| i.events.clone());
        for ev in events {
            match ev {
                egui::Event::Key { key, pressed, .. } => {
                    let vk = vk_for_key(key);
                    let is_text = key_to_char(key).is_some();
                    // Les caractères imprimables sont gérés via Event::Text ;
                    // ici, uniquement navigation/contrôle + raccourcis Ctrl.
                    if vk != 0 && (!is_text || mods & FLAG_CONTROL != 0) {
                        let mut ke = KeyEvent::default();
                        ke.type_ = if pressed {
                            KeyEventType::KEYDOWN
                        } else {
                            KeyEventType::KEYUP
                        };
                        ke.modifiers = mods;
                        ke.windows_key_code = vk;
                        ke.native_key_code = vk;
                        self.with_host(|host| host.send_key_event(Some(&ke)));
                    }
                }
                egui::Event::Text(text) => {
                    // Caractère réellement saisi (casse correcte) :
                    // séquence KEYDOWN(char) + CHAR + KEYUP.
                    // Note CEF Windows : pour KEYEVENT_CHAR, `windows_key_code`
                    // DOIT être le code du caractère (`c as i32`, ex: 0x6B pour 'k'),
                    // et non le Virtual-Key (qui vaut toujours 0x4B 'K').
                    for c in text.chars() {
                        let vk = vk_for_char(c);
                        let char_code = c as i32;

                        let mut kd = KeyEvent::default();
                        kd.type_ = KeyEventType::KEYDOWN;
                        kd.modifiers = mods;
                        kd.windows_key_code = vk;
                        kd.native_key_code = vk;
                        kd.character = c as u16;
                        kd.unmodified_character = c as u16;
                        self.with_host(|host| host.send_key_event(Some(&kd)));

                        let mut ch = KeyEvent::default();
                        ch.type_ = KeyEventType::CHAR;
                        ch.modifiers = mods;
                        ch.windows_key_code = char_code;
                        ch.native_key_code = char_code;
                        ch.character = c as u16;
                        ch.unmodified_character = c as u16;
                        self.with_host(|host| host.send_key_event(Some(&ch)));

                        let mut ku = KeyEvent::default();
                        ku.type_ = KeyEventType::KEYUP;
                        ku.modifiers = mods;
                        ku.windows_key_code = vk;
                        ku.native_key_code = vk;
                        ku.character = c as u16;
                        ku.unmodified_character = c as u16;
                        self.with_host(|host| host.send_key_event(Some(&ku)));
                    }
                }
                _ => {}
            }
        }
    }

    /// Page de nouvel onglet : fond, recherche et raccourcis (rendu egui).
    fn new_tab_page(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(24, 26, 32));

        // Bandeau d'accent discret en haut (rappel de la marque).
        let accent = egui::Color32::from_rgb(52, 199, 89);
        painter.rect_filled(
            egui::Rect::from_min_max(rect.min, egui::pos2(rect.max.x, rect.min.y + 3.0)),
            0.0,
            accent,
        );

        let mut go: Option<String> = None;
        let engine_host = hostname(&self.search_engine);
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect), |ui| {
            ui.add_space((rect.height() * 0.20).max(32.0));
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("Faraday")
                        .size(40.0)
                        .strong()
                        .color(egui::Color32::from_rgb(235, 235, 240)),
                );
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("Votre navigation privée, sans traqueurs")
                        .size(14.0)
                        .color(egui::Color32::from_gray(150)),
                );
                ui.add_space(24.0);

                // Barre de recherche / adresse.
                let sw = (rect.width() * 0.55).clamp(260.0, 640.0);
                let search = ui.add_sized(
                    [sw, 38.0],
                    egui::TextEdit::singleline(&mut self.ntp_query)
                        .font(egui::TextStyle::Body)
                        .margin(egui::Margin::symmetric(16, 9))
                        .hint_text("Rechercher sur le web ou saisir une adresse"),
                );
                let submit =
                    search.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if submit && !self.ntp_query.trim().is_empty() {
                    go = Some(self.ntp_query.clone());
                }

                ui.add_space(34.0);

                // Raccourcis rapides.
                let label_hint = egui::RichText::new("Raccourcis")
                    .size(11.0)
                    .color(egui::Color32::from_gray(130));
                ui.label(label_hint);
                ui.add_space(10.0);

                egui::ScrollArea::horizontal().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for link in QUICK_LINKS {
                            let tile = ui.vertical(|ui| {
                                let b = ui.add(
                                    egui::Button::new(
                                        egui::RichText::new(link.icon).size(28.0).color(link.color),
                                    )
                                    .min_size(egui::vec2(64.0, 64.0)),
                                );
                                ui.label(
                                    egui::RichText::new(link.label)
                                        .size(11.0)
                                        .color(egui::Color32::from_gray(170)),
                                );
                                b
                            });
                            if tile.inner.clicked() {
                                go = Some(link.url.to_string());
                            }
                            ui.add_space(8.0);
                        }
                    });
                });

                ui.add_space(28.0);
                ui.label(
                    egui::RichText::new(format!(
                        "Recherche par défaut : {engine_host} — aucune donnée partagée"
                    ))
                    .size(11.0)
                    .color(egui::Color32::from_gray(110)),
                );
            });
        });

        if let Some(q) = go {
            self.navigate_to(&q);
            self.ntp_query.clear();
        }
    }

    /// Fenêtre flottante « Historique » (liste des visites + effacer).
    fn history_window(&mut self, ctx: &egui::Context) {
        if !self.show_history {
            return;
        }
        let mut open = true;
        let mut go: Option<String> = None;
        let mut clear = false;

        egui::Window::new("Historique")
            .open(&mut open)
            .default_width(460.0)
            .default_height(400.0)
            .collapsible(false)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.strong("Historique de navigation");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new(icons::TRASH).size(16.0))
                                    .frame(false),
                            )
                            .on_hover_text("Effacer l'historique")
                            .clicked()
                        {
                            clear = true;
                        }
                    });
                });
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(2.0);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let entries: Vec<HistoryEntry> = self.history.lock().unwrap().clone();
                    if entries.is_empty() {
                        ui.add_space(16.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new("Aucune visite enregistrée pour l'instant.")
                                    .color(egui::Color32::from_gray(150)),
                            );
                        });
                        return;
                    }
                    for e in &entries {
                        let row_h = 42.0;
                        let (rrect, resp) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), row_h),
                            egui::Sense::click(),
                        );
                        if ui.is_rect_visible(rrect) {
                            let p = ui.painter();
                            if resp.hovered() {
                                p.rect_filled(
                                    rrect,
                                    4.0,
                                    egui::Color32::from_gray(45),
                                );
                            }
                            let cy = rrect.center().y;
                            p.text(
                                egui::pos2(rrect.left() + 18.0, cy),
                                egui::Align2::LEFT_CENTER,
                                icons::CLOCK_COUNTER_CLOCKWISE,
                                egui::FontId::proportional(14.0),
                                egui::Color32::from_gray(160),
                            );
                            p.text(
                                egui::pos2(rrect.left() + 42.0, rrect.top() + 8.0),
                                egui::Align2::LEFT_TOP,
                                &e.title,
                                egui::FontId::proportional(13.0),
                                ui.visuals().text_color(),
                            );
                            p.text(
                                egui::pos2(rrect.left() + 42.0, rrect.top() + 24.0),
                                egui::Align2::LEFT_TOP,
                                &e.url,
                                egui::FontId::proportional(11.0),
                                egui::Color32::from_gray(140),
                            );
                        }
                        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            go = Some(e.url.clone());
                        }
                    }
                });
            });

        if clear {
            self.history.lock().unwrap().clear();
        }
        if let Some(url) = go {
            self.show_history = false;
            self.navigate_to(&url);
        }
        if !open {
            self.show_history = false;
        }
    }

    /// Fenêtre flottante « Téléchargements » : progression, annulation, dossier.
    fn downloads_window(&mut self, ctx: &egui::Context) {
        if !self.show_downloads {
            return;
        }
        let mut open = true;
        let mut cancel_ids: Vec<u32> = Vec::new();
        let mut open_paths: Vec<String> = Vec::new();
        let mut clear_finished = false;

        egui::Window::new("Téléchargements")
            .open(&mut open)
            .default_width(540.0)
            .default_height(380.0)
            .collapsible(false)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.strong("Téléchargements");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new(icons::TRASH).size(15.0))
                                    .frame(false),
                            )
                            .on_hover_text("Effacer les éléments terminés")
                            .clicked()
                        {
                            clear_finished = true;
                        }
                    });
                });
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(2.0);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let entries: Vec<DownloadEntry> = self.downloads.lock().unwrap().clone();
                    if entries.is_empty() {
                        ui.add_space(16.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new("Aucun téléchargement pour l'instant.")
                                    .color(egui::Color32::from_gray(150)),
                            );
                        });
                        return;
                    }
                    for e in &entries {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(&e.name).size(13.0).strong());
                                let sub = if e.state == DownloadState::InProgress && e.speed > 0 {
                                    format!("{} · {}", e.state.label(), format_speed(e.speed))
                                } else {
                                    e.state.label().to_string()
                                };
                                ui.label(
                                    egui::RichText::new(sub)
                                        .size(11.0)
                                        .color(egui::Color32::from_gray(150)),
                                );
                            });
                            ui.add_space(8.0);

                            let frac = if e.total > 0 {
                                (e.received as f32 / e.total as f32).clamp(0.0, 1.0)
                            } else {
                                0.0
                            };
                            let pct = e.percent.max(0);
                            let bar = ui.add(
                                egui::ProgressBar::new(frac)
                                    .desired_width((ui.available_width() - 72.0).max(60.0))
                                    .text(format!("{pct}%")),
                            );
                            let _ = bar;

                            // Bouton d'annulation si en cours.
                            if e.state == DownloadState::InProgress
                                || e.state == DownloadState::Starting
                            {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new(icons::X)
                                                .size(12.0)
                                                .color(egui::Color32::from_rgb(255, 90, 90)),
                                        )
                                        .min_size(egui::vec2(26.0, 26.0))
                                        .frame(false),
                                    )
                                    .on_hover_text("Annuler")
                                    .clicked()
                                {
                                    cancel_ids.push(e.id);
                                }
                            }
                            // Ouvrir le dossier si terminé.
                            if e.state == DownloadState::Complete && !e.path.is_empty() {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new(icons::FOLDER_OPEN)
                                                .size(13.0)
                                                .color(egui::Color32::from_gray(190)),
                                        )
                                        .min_size(egui::vec2(26.0, 26.0))
                                        .frame(false),
                                    )
                                    .on_hover_text("Ouvrir le dossier")
                                    .clicked()
                                {
                                    open_paths.push(e.path.clone());
                                }
                            }
                        });
                        ui.separator();
                    }
                });
            });

        // Demander l'annulation (le handler CEF l'exécutera à la prochaine
        // mise à jour du téléchargement).
        for id in cancel_ids {
            let mut list = self.downloads.lock().unwrap();
            if let Some(e) = list.iter_mut().find(|e| e.id == id) {
                e.cancel_requested = true;
            }
        }
        for path in open_paths {
            let _ = std::process::Command::new("explorer.exe")
                .arg("/select,")
                .arg(&path)
                .spawn();
        }
        if clear_finished {
            self.downloads
                .lock()
                .unwrap()
                .retain(|e| matches!(e.state, DownloadState::Starting | DownloadState::InProgress));
        }
        if !open {
            self.show_downloads = false;
        }
    }

    /// Barre de statut en bas à gauche : URL cible sous le pointeur.
    fn status_bar(&mut self, ctx: &egui::Context) {
        let raw = self
            .tabs
            .get(self.active)
            .and_then(|t| t.status.lock().ok())
            .and_then(|s| s.clone())
            .unwrap_or_default();
        if raw.is_empty() {
            return;
        }
        // N'affiche que les cibles de lien (URL), pas les messages de
        // chargement transitoires remontés par CEF.
        let trimmed = raw.trim();
        let is_target = looks_like_url(trimmed)
            || trimmed.contains("://")
            || trimmed.starts_with("mailto:")
            || trimmed.starts_with("tel:");
        if !is_target {
            return;
        }

        let mut shown = raw;
        if shown.chars().count() > 160 {
            let cut: String = shown.chars().take(160).collect();
            shown = format!("{cut}…");
        }
        let color = if shown.starts_with("https://") {
            egui::Color32::from_rgb(126, 208, 148)
        } else {
            egui::Color32::from_rgb(205, 210, 220)
        };

        egui::Area::new(egui::Id::new("faraday_status_bar"))
            .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(10.0, -14.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(egui::Color32::from_rgba_unmultiplied(24, 26, 33, 218))
                    .stroke(egui::Stroke::new(
                        1.0_f32,
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 20),
                    ))
                    .corner_radius(7.0)
                    .inner_margin(egui::Margin::symmetric(10, 6))
                    .show(ui, |ui| {
                        ui.set_max_width(560.0);
                        ui.label(
                            egui::RichText::new(shown)
                                .family(egui::FontFamily::Monospace)
                                .size(12.0)
                                .color(color),
                        );
                    });
            });
    }

    /// Lien actuellement sous le pointeur (d'après la barre de statut CEF).
    fn hovered_link(&self) -> String {
        let raw = self
            .tabs
            .get(self.active)
            .and_then(|t| t.status.lock().ok())
            .and_then(|s| s.clone())
            .unwrap_or_default();
        let trimmed = raw.trim().to_string();
        if trimmed.is_empty() {
            return String::new();
        }
        let is_url = looks_like_url(&trimmed)
            || trimmed.contains("://")
            || trimmed.starts_with("mailto:")
            || trimmed.starts_with("tel:");
        if is_url {
            trimmed
        } else {
            String::new()
        }
    }

    /// Dessine et gère le menu contextuel (clic droit) sur la page.
    fn draw_context_menu(&mut self, ctx: &egui::Context) {
        let Some(menu) = self.ctx_menu.as_ref() else { return };
        let pos = menu.pos;
        let link = menu.link.clone();
        let page = self
            .tabs
            .get(self.active)
            .map(|t| t.url.clone())
            .unwrap_or_default();
        let has_link = !link.is_empty();
        let has_page = !page.is_empty();

        let mut act: Option<CtxCommand> = None;
        let item = |ui: &mut egui::Ui, icon: &str, label: &str| -> bool {
            ui.add(
                egui::Button::new(
                    egui::RichText::new(format!("{icon}  {label}")).size(13.5),
                )
                .min_size(egui::vec2(230.0, 28.0))
                .frame(false),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        };

        egui::Area::new(egui::Id::new("faraday_ctx_menu"))
            .fixed_pos(pos)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                // Popup LUMINEUSE (texte foncé) pour bien ressortir sur la page.
                ui.style_mut().visuals = egui::Visuals::light();
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(252, 253, 255))
                    .stroke(egui::Stroke::new(
                        1.0_f32,
                        egui::Color32::from_rgb(160, 166, 178),
                    ))
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::symmetric(5, 5))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(340.0)
                            .show(ui, |ui| {
                                if has_link {
                                    if item(ui, icons::LINK, "Copier le lien") {
                                        act = Some(CtxCommand::CopyLink);
                                    }
                                    if item(ui, icons::PLUS, "Ouvrir dans un nouvel onglet") {
                                        act = Some(CtxCommand::OpenLinkTab);
                                    }
                                    if item(
                                        ui,
                                        icons::ARROW_SQUARE_UP_RIGHT,
                                        "Ouvrir le lien ici",
                                    ) {
                                        act = Some(CtxCommand::OpenLinkHere);
                                    }
                                    ui.separator();
                                }
                                if has_page {
                                    if item(ui, icons::COPY, "Copier l'adresse de la page") {
                                        act = Some(CtxCommand::CopyPage);
                                    }
                                    ui.separator();
                                }
                                if item(ui, icons::ARROWS_CLOCKWISE, "Recharger") {
                                    act = Some(CtxCommand::Reload);
                                }
                                if item(ui, icons::ARROW_LEFT, "Page précédente") {
                                    act = Some(CtxCommand::Back);
                                }
                                if item(ui, icons::ARROW_RIGHT, "Page suivante") {
                                    act = Some(CtxCommand::Forward);
                                }
                            });
                    });
            });

        let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let any_press = ctx.input(|i| i.pointer.any_pressed());
        if let Some(cmd) = act {
            self.ctx_menu = None;
            match cmd {
                CtxCommand::CopyLink => ctx.copy_text(link),
                CtxCommand::CopyPage => ctx.copy_text(page),
                CtxCommand::OpenLinkTab => self.open_tab(link),
                CtxCommand::OpenLinkHere => {
                    self.url = link;
                    self.navigate();
                    self.page_focused = true;
                }
                CtxCommand::Reload => self.reload(),
                CtxCommand::Back => self.back(),
                CtxCommand::Forward => self.forward(),
            }
        } else if esc || any_press {
            // Clic ailleurs ou Échap : on ferme le menu.
            self.ctx_menu = None;
        }
    }

    /// Convertit une notification de téléchargement en toast affiché.
    fn push_toast(&mut self, notice: DownloadNotice) {
        let (title, icon, color) = match notice.kind {
            DownloadNoticeKind::Started => (
                "Téléchargement démarré",
                icons::DOWNLOAD,
                egui::Color32::from_rgb(88, 166, 255),
            ),
            DownloadNoticeKind::Complete => (
                "Téléchargement terminé",
                icons::CHECK,
                egui::Color32::from_rgb(52, 199, 89),
            ),
            DownloadNoticeKind::Cancelled => (
                "Téléchargement annulé",
                icons::X,
                egui::Color32::from_rgb(200, 200, 210),
            ),
            DownloadNoticeKind::Interrupted => (
                "Téléchargement interrompu",
                icons::WARNING,
                egui::Color32::from_rgb(255, 159, 10),
            ),
        };
        self.toasts.push(Toast {
            title,
            detail: notice.name,
            icon,
            color,
            start: Instant::now(),
            life: 5.0,
        });
        if self.toasts.len() > 6 {
            self.toasts.remove(0);
        }
    }

    /// Consomme les notifications et dessine les toasts (haut à droite).
    fn update_toasts(&mut self, ctx: &egui::Context) {
        // Vide la file de notifications partagée.
        let pending: Vec<DownloadNotice> = {
            let mut queue = self.notices.lock().unwrap();
            queue.drain(..).collect()
        };
        for n in pending {
            self.push_toast(n);
        }

        // Retire les toasts expirés.
        let now = Instant::now();
        self.toasts
            .retain(|t| now.duration_since(t.start).as_secs_f32() < t.life);
        if self.toasts.is_empty() {
            return;
        }

        let toasts = self.toasts.clone();
        egui::Area::new(egui::Id::new("faraday_toasts"))
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-14.0, 12.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
                    for t in &toasts {
                        let age = now.duration_since(t.start).as_secs_f32();
                        let alpha = (1.0 - (age / t.life) * 0.75).clamp(0.25, 1.0);
                        let bg = egui::Color32::from_rgba_unmultiplied(
                            32,
                            36,
                            44,
                            (245.0 * alpha) as u8,
                        );
                        let border = egui::Color32::from_rgba_unmultiplied(
                            255,
                            255,
                            255,
                            (24.0 * alpha) as u8,
                        );
                        egui::Frame::new()
                            .fill(bg)
                            .stroke(egui::Stroke::new(1.0_f32, border))
                            .corner_radius(10.0)
                            .inner_margin(egui::Margin::symmetric(12, 10))
                            .show(ui, |ui| {
                                ui.set_max_width(330.0);
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(t.icon)
                                            .size(20.0)
                                            .color(t.color),
                                    );
                                    ui.vertical(|ui| {
                                        ui.label(
                                            egui::RichText::new(t.title)
                                                .size(13.0)
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new(&t.detail)
                                                .size(11.5)
                                                .color(egui::Color32::from_gray(165)),
                                        );
                                    });
                                });
                            });
                        ui.add_space(8.0);
                    }
                });
            });
    }

    /// Dessine la barre d'onglets + boutons nouveau/fermer.
    fn tab_strip(&mut self, ui: &mut egui::Ui) {
        let mut to_close: Option<usize> = None;
        let mut to_activate: Option<usize> = None;
        let mut new_tab = false;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);

            let mut i = 0;
            while i < self.tabs.len() {
                let active = i == self.active;
                let raw = &self.tabs[i].url;
                let title = if raw.is_empty() {
                    "Nouvel onglet".to_string()
                } else {
                    hostname(raw)
                };
                let label = egui::RichText::new(title).size(13.0);
                let selected = ui.selectable_label(active, label);
                if selected.clicked() {
                    to_activate = Some(i);
                }

                // Bouton de fermeture (croix) — discret sur l'onglet survolé.
                let close = ui.add(
                    egui::Button::new(
                        egui::RichText::new(icons::X)
                            .size(10.0)
                            .color(egui::Color32::from_gray(160)),
                    )
                    .min_size(egui::vec2(16.0, 16.0))
                    .frame(false),
                );
                if close.on_hover_text("Fermer l'onglet").clicked() {
                    to_close = Some(i);
                }
                i += 1;
            }

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            // Nouvel onglet (+)
            let plus = ui.add(
                egui::Button::new(
                    egui::RichText::new(icons::PLUS).size(14.0).strong(),
                )
                .min_size(egui::vec2(22.0, 22.0)),
            );
            if plus.on_hover_text("Nouvel onglet (Ctrl+T)").clicked() {
                new_tab = true;
            }
        });

        // Appliquer les actions après la boucle (éviter les emprunts).
        if let Some(i) = to_close {
            self.close_tab(i);
        }
        if let Some(i) = to_activate {
            self.set_active(i);
        }
        if new_tab {
            // Nouvel onglet = page d'accueil Faraday (vide → egui).
            self.open_tab(String::new());
        }
    }
}

impl eframe::App for FaradayChrome {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Pomper les tâches CEF (mode external_message_pump).
        cef::do_message_loop_work();
        // Redessiner en continu pour recevoir les on_paint de CEF.
        ctx.request_repaint();

        // Raccourcis clavier du chrome (avant tout envoi à la page).
        {
            let (t, l, h, j) = ctx.input(|i| {
                let c = i.modifiers.ctrl;
                (
                    c && i.key_pressed(egui::Key::T),
                    c && i.key_pressed(egui::Key::L),
                    c && i.key_pressed(egui::Key::H),
                    c && i.key_pressed(egui::Key::J),
                )
            });
            if t {
                self.open_tab(String::new());
            }
            if h {
                self.show_history = !self.show_history;
            }
            if j {
                self.show_downloads = !self.show_downloads;
            }
            if l {
                if let Some(id) = self.address_bar_id {
                    ctx.memory_mut(|m| m.request_focus(id));
                }
                self.page_focused = false;
            }
        }

        self.make_browser_for(self.active);
        self.upload_active_texture(ctx);

        // --- Barre d'onglets ---
        egui::TopBottomPanel::top("tabs")
            .frame(
                egui::Frame::side_top_panel(&ctx.style())
                    .fill(egui::Color32::from_rgb(34, 36, 43))
                    .inner_margin(egui::Margin::symmetric(8, 4)),
            )
            .show(ctx, |ui| {
                self.tab_strip(ui);
            });

        egui::TopBottomPanel::top("chrome")
            .frame(
                egui::Frame::side_top_panel(&ctx.style())
                    .fill(egui::Color32::from_rgb(28, 30, 36))
                    .inner_margin(egui::Margin::symmetric(10, 8)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);

                    // Boutons de navigation : icône claire + infobulle.
                    let nav = |ui: &mut egui::Ui, icon: &str, tip: &str| {
                        ui.add(
                            egui::Button::new(egui::RichText::new(icon).size(20.0))
                                .min_size(egui::vec2(30.0, 30.0)),
                        )
                        .on_hover_text(tip)
                        .clicked()
                    };

                    if nav(ui, icons::ARROW_LEFT, "Page précédente") {
                        self.back();
                    }
                    if nav(ui, icons::ARROW_RIGHT, "Page suivante") {
                        self.forward();
                    }
                    if nav(ui, icons::ARROWS_CLOCKWISE, "Recharger") {
                        self.reload();
                    }
                    let home = self.search_engine.clone();
                    if nav(ui, icons::HOUSE, "Aller au moteur de recherche") {
                        self.url = home;
                        self.navigate();
                    }

                    ui.add_space(4.0);

                    // Barre d'adresse : occupe l'espace restant après les
                    // boutons fixes de droite (Tél. + Hist. + Aller + bouclier).
                    let addr_w = (ui.available_width() - 200.0).max(80.0);
                    let addr = ui.add_sized(
                        [addr_w, 30.0],
                        egui::TextEdit::singleline(&mut self.url)
                            .font(egui::TextStyle::Body)
                            .margin(egui::Margin::symmetric(8, 6))
                            .hint_text("Rechercher ou entrer une adresse..."),
                    );
                    self.address_bar_id = Some(addr.id);
                    if addr.gained_focus() {
                        self.page_focused = false;
                        self.with_host(|host| host.set_focus(0));
                    }
                    if addr.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.navigate();
                        self.page_focused = true;
                        self.with_host(|host| host.set_focus(1));
                    }

                    // Bouton Aller (compact, icône flèche).
                    let go = ui.add(
                        egui::Button::new(egui::RichText::new(icons::ARROW_CIRCLE_RIGHT).size(18.0))
                            .min_size(egui::vec2(30.0, 30.0)),
                    );
                    if go.on_hover_text("Ouvrir cette adresse").clicked() {
                        self.navigate();
                        self.page_focused = true;
                        self.with_host(|host| host.set_focus(1));
                    }

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Téléchargements (fenêtre flottante) + indicateur d'activité.
                    let (dl_active, dl_total) = {
                        let list = self.downloads.lock().unwrap();
                        let active = list
                            .iter()
                            .filter(|e| {
                                matches!(
                                    e.state,
                                    DownloadState::Starting | DownloadState::InProgress
                                )
                            })
                            .count();
                        (active, list.len())
                    };
                    let mut dl_btn = egui::Button::new(
                        egui::RichText::new(icons::DOWNLOAD).size(18.0),
                    )
                    .min_size(egui::vec2(30.0, 30.0));
                    if dl_active > 0 {
                        dl_btn = dl_btn
                            .fill(egui::Color32::from_rgba_unmultiplied(88, 166, 255, 45));
                    }
                    let dl = ui.add(dl_btn);
                    let dl_tip = if dl_active > 0 {
                        format!("Téléchargements (Ctrl+J) — {dl_active} en cours")
                    } else if dl_total > 0 {
                        format!("Téléchargements (Ctrl+J) — {dl_total} élément(s)")
                    } else {
                        "Téléchargements (Ctrl+J)".to_string()
                    };
                    if dl.on_hover_text(dl_tip).clicked() {
                        self.show_downloads = !self.show_downloads;
                    }

                    ui.add_space(4.0);

                    // Historique (fenêtre flottante).
                    let hist = ui.add(
                        egui::Button::new(
                            egui::RichText::new(icons::CLOCK_COUNTER_CLOCKWISE).size(18.0),
                        )
                        .min_size(egui::vec2(30.0, 30.0)),
                    );
                    if hist.on_hover_text("Historique (Ctrl+H)").clicked() {
                        self.show_history = !self.show_history;
                    }

                    ui.add_space(4.0);

                    // Bouclier privacy : compteur de blocages en direct.
                    let blocked = blocklist::blocked_count();
                    let shield = ui.add(
                        egui::Button::new(
                            egui::RichText::new(format!("{} {blocked}", icons::SHIELD_CHECK))
                                .color(egui::Color32::from_rgb(52, 199, 89))
                                .strong()
                                .size(16.0),
                        )
                        .min_size(egui::vec2(44.0, 30.0))
                        .fill(egui::Color32::from_rgba_unmultiplied(52, 199, 89, 25)),
                    );
                    let tip = if blocked == 0 {
                        "Aucune requête de tracking bloquée - Faraday protège votre vie privée."
                            .to_string()
                    } else {
                        format!(
                            "{blocked} requête{} de tracking bloquée{} - Faraday protège votre vie privée.",
                            if blocked > 1 { "s" } else { "" },
                            if blocked > 1 { "s" } else { "" }
                        )
                    };
                    if shield
                        .on_hover_text(format!(
                            "{tip}\nCliquer : tester sur EFF Cover Your Tracks."
                        ))
                        .clicked()
                    {
                        self.url = "https://coveryourtracks.eff.org".to_string();
                        self.navigate();
                        self.page_focused = true;
                        self.with_host(|host| host.set_focus(1));
                    }
                });
            });

        let tex_id = self
            .tabs
            .get(self.active)
            .and_then(|t| t.texture.as_ref())
            .map(|t| t.id());
        egui::CentralPanel::default().show(ctx, |ui| {
            let avail = ui.available_size();
            let (rect, response) = ui.allocate_exact_size(avail, egui::Sense::click_and_drag());
            self.sync_view_size(rect, ctx.pixels_per_point());

            // Onglet sans navigateur (page d'accueil) → rendu egui.
            if self.active_is_new_tab() {
                self.new_tab_page(ui, rect);
                return;
            }

            // Cliquer sur la page lui donne le focus clavier.
            if response.is_pointer_button_down_on() {
                self.page_focused = true;
                if let Some(id) = self.address_bar_id {
                    ctx.memory_mut(|m| m.surrender_focus(id));
                }
                self.with_host(|host| host.set_focus(1));
            }
            match tex_id {
                Some(id) => {
                    let uv = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0));
                    ui.painter().image(id, rect, uv, egui::Color32::WHITE);
                }
                None => {
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Chargement de la page...",
                        egui::FontId::proportional(18.0),
                        ui.visuals().text_color(),
                    );
                }
            }
            self.forward_input(ctx, rect, &response);
        });

        // Fenêtres flottantes (historique, téléchargements) par-dessus la page.
        self.history_window(ctx);
        self.downloads_window(ctx);

        // Notifications toast des téléchargements.
        self.update_toasts(ctx);

        // URL survolée (bas à gauche).
        self.status_bar(ctx);

        // Menu contextuel (clic droit).
        self.draw_context_menu(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Persistance : onglets ouverts, onglet actif et historique.
        let data = session::SessionData {
            active: self.active,
            tabs: self.tabs.iter().map(|t| t.url.clone()).collect(),
            history: self.history.lock().unwrap().clone(),
        };
        session::save(&data);

        // Fermeture propre des navigateurs CEF avant `cef::shutdown()` :
        // évite une sortie avec code 1 et des reliquats de processus au
        // prochain démarrage. On pompe les messages CEF un court instant
        // pour laisser les fermetures se terminer.
        self.close_all_browsers();
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(1500);
        while std::time::Instant::now() < deadline {
            cef::do_message_loop_work();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
}

/// Lance la fenêtre egui (chrome + page OSR).
pub fn run(view_size: ViewSize) -> anyhow::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([600.0, 400.0])
            .with_title("Faraday Browser - Privacy First"),
        ..Default::default()
    };
    eframe::run_native(
        "Faraday",
        options,
        Box::new(move |cc| Ok(Box::new(FaradayChrome::new(cc, view_size)))),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}
