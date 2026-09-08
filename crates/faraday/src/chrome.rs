//! Chrome du navigateur : UI egui (onglets + barre d'adresse) et rendu de la
//! page web via CEF en rendu hors-écran (OSR).

use eframe::egui;
use std::sync::{Arc, Mutex};

use cef::*;

use crate::handler::{FaradayClient, FaradayHandler, RenderBuffer, ViewSize};
use crate::icons;
use crate::privacy::PrivacyConfig;

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

/// Un onglet : son navigateur CEF (OSR), son tampon de pixels et sa texture egui.
pub struct Tab {
    url: String,
    buffer: Arc<Mutex<RenderBuffer>>,
    browser: Option<Browser>,
    texture: Option<egui::TextureHandle>,
}

pub struct FaradayChrome {
    /// Taille de la zone de rendu (partagée par tous les onglets).
    view_size: ViewSize,
    tabs: Vec<Tab>,
    active: usize,
    /// Contenu de la barre d'adresse (onglet actif).
    url: String,
    left_down: bool,
    /// Le focus clavier est sur la page web (vs barre d'adresse).
    page_focused: bool,
    address_bar_id: Option<egui::Id>,
}

impl FaradayChrome {
    pub fn new(cc: &eframe::CreationContext<'_>, view_size: ViewSize) -> Self {
        // Enregistrer les polices d'icônes Phosphor (libres MIT)
        icons::setup_custom_fonts(&cc.egui_ctx);

        let config = PrivacyConfig::load();
        let url = config.default_search_engine;
        let tab = Tab {
            url: url.clone(),
            buffer: Arc::new(Mutex::new(RenderBuffer::new())),
            browser: None,
            texture: None,
        };
        Self {
            view_size,
            tabs: vec![tab],
            active: 0,
            url,
            left_down: false,
            page_focused: false,
            address_bar_id: None,
        }
    }

    /// Crée le navigateur OSR de l'onglet `idx` (une seule fois par onglet).
    fn make_browser_for(&mut self, idx: usize) {
        if idx >= self.tabs.len() || self.tabs[idx].browser.is_some() {
            return;
        }
        let buffer = self.tabs[idx].buffer.clone();
        let client = FaradayClient::new(FaradayHandler::new(), buffer, self.view_size.clone());
        let settings = BrowserSettings {
            windowless_frame_rate: 60,
            ..Default::default()
        };
        let url = CefString::from(self.tabs[idx].url.as_str());
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
        };
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
        self.make_browser_for(self.active);
        self.url = self.tabs[self.active].url.clone();
        self.page_focused = true;
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
        self.page_focused = true;
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
        let url = self.url.clone();
        self.tabs[self.active].url = url.clone();
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

            let pressed = response.is_pointer_button_down_on();
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
                let title = hostname(&self.tabs[i].url);
                let label = egui::RichText::new(title).size(13.0);
                let selected = ui.selectable_label(active, label);
                if selected.clicked() {
                    to_activate = Some(i);
                }

                // Bouton de fermeture (croix) — discret sur l'onglet survolé.
                let close = ui.add(
                    egui::Button::new(egui::RichText::new("x").size(11.0).color(
                        egui::Color32::from_gray(160),
                    ))
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
                egui::Button::new(egui::RichText::new("+").size(16.0).strong())
                    .min_size(egui::vec2(22.0, 22.0)),
            );
            if plus.on_hover_text("Nouvel onglet").clicked() {
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
            let config = PrivacyConfig::load();
            self.open_tab(config.default_search_engine);
        }
    }
}

impl eframe::App for FaradayChrome {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Pomper les tâches CEF (mode external_message_pump).
        cef::do_message_loop_work();
        // Redessiner en continu pour recevoir les on_paint de CEF.
        ctx.request_repaint();

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
                    if nav(ui, icons::HOUSE, "Accueil DuckDuckGo") {
                        self.url = "https://duckduckgo.com".to_string();
                        self.navigate();
                    }

                    ui.add_space(4.0);

                    // Barre d'adresse : occupe l'espace restant après les
                    // boutons fixes de droite (Aller + bouclier).
                    let addr_w = (ui.available_width() - 120.0).max(80.0);
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

                    // Bouclier privacy compact (vert).
                    let shield = ui.add(
                        egui::Button::new(
                            egui::RichText::new(format!("{} 0", icons::SHIELD_CHECK))
                                .color(egui::Color32::from_rgb(52, 199, 89))
                                .strong()
                                .size(16.0),
                        )
                        .min_size(egui::vec2(44.0, 30.0))
                        .fill(egui::Color32::from_rgba_unmultiplied(52, 199, 89, 25)),
                    );
                    if shield
                        .on_hover_text(
                            "0 tracker bloqué - Faraday protège votre vie privée.\nCliquer : tester sur EFF Cover Your Tracks.",
                        )
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
