//! Application CEF principale. C'est ici qu'on injecte les switches de
//! confidentialité avant même que Chromium ne démarre.

use cef::*;

use crate::privacy::{apply_privacy_switches, PrivacyConfig};

wrap_app! {
    pub struct FaradayApp;

    impl App {
        /// Applique tous les switches « zéro tracking » au lancement.
        fn on_before_command_line_processing(
            &self,
            _process_type: Option<&CefStringUtf16>,
            command_line: Option<&mut CommandLine>,
        ) {
            let Some(command_line) = command_line else {
                return;
            };

            let config = PrivacyConfig::load();
            apply_privacy_switches(&config, command_line);

            // Rendu logiciel : évite des DCHECK GPU sur les environnements
            // sans adaptateur graphique stable (à réévaluer pour la perf).
            command_line.append_switch(Some(&"--disable-gpu".into()));
        }

        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            Some(FaradayBrowserProcessHandler::new())
        }
    }
}

wrap_browser_process_handler! {
    struct FaradayBrowserProcessHandler;

    impl BrowserProcessHandler {
        fn on_context_initialized(&self) {
            // Le navigateur OSR est créé par l'UI egui (chrome.rs) une fois
            // que la boucle événementielle tourne.
        }
    }
}
