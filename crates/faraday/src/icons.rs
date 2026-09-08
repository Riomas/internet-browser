//! Définitions et chargement des icônes Phosphor (licence MIT libre de droits).
//! Phosphor Icons: https://phosphoricons.com/
//! Codes Unicode vérifiés dans la police Phosphor.ttf (variant Regular).

use eframe::egui;
use std::sync::Arc;

/// Précédent : flèche gauche.
pub const ARROW_LEFT: &str = "\u{E058}";
/// Suivant : flèche droite.
pub const ARROW_RIGHT: &str = "\u{E06C}";
/// Recharger : flèches circulaires.
pub const ARROWS_CLOCKWISE: &str = "\u{E094}";
/// Accueil : maison.
pub const HOUSE: &str = "\u{E2C2}";
/// Aller : flèche dans un cercle.
pub const ARROW_CIRCLE_RIGHT: &str = "\u{E02E}";
/// Protection / privacy : bouclier validé.
pub const SHIELD_CHECK: &str = "\u{E40C}";
/// Historique : horloge anti-horaire.
pub const CLOCK_COUNTER_CLOCKWISE: &str = "\u{E1A0}";
/// Nouvel onglet / plus.
pub const PLUS: &str = "\u{E3D4}";
/// Fermer : croix.
pub const X: &str = "\u{E4F6}";
/// Poubelle (effacer).
pub const TRASH: &str = "\u{E4A6}";
/// Globe (web générique).
pub const GLOBE: &str = "\u{E288}";
/// Article / document.
pub const ARTICLE: &str = "\u{E0A8}";
/// Journal (actualités).
pub const NEWSPAPER: &str = "\u{E344}";
/// Vidéo.
pub const VIDEO: &str = "\u{E740}";
/// Empreinte (sécurité/privacy).
pub const FINGERPRINT: &str = "\u{E23E}";
/// Logo GitHub (développement).
pub const GITHUB_LOGO: &str = "\u{E576}";
/// Certificat (sécurité).
pub const CERTIFICATE: &str = "\u{E766}";
/// Signet (favoris / archive).
pub const BOOKMARK: &str = "\u{E0E8}";

const PHOSPHOR_TTF: &[u8] = include_bytes!("../resources/fonts/Phosphor.ttf");

/// Injecte la police d'icônes Phosphor dans le système de polices d'egui.
pub fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        "phosphor".to_owned(),
        Arc::new(egui::FontData::from_static(PHOSPHOR_TTF)),
    );

    if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        family.insert(1, "phosphor".to_owned());
    }

    ctx.set_fonts(fonts);
}
