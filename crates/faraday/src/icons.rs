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
