//! Application CEF principale. C'est ici qu'on injecte les switches de
//! confidentialité avant même que Chromium ne démarre, et qu'on crée le
//! navigateur une fois le contexte initialisé.

use cef::*;
use std::cell::RefCell;

use crate::handler::{FaradayClient, FaradayHandler};
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
        }

        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            Some(FaradayBrowserProcessHandler::new(RefCell::new(None)))
        }
    }
}

wrap_browser_process_handler! {
    struct FaradayBrowserProcessHandler {
        client: RefCell<Option<Client>>,
    }

    impl BrowserProcessHandler {
        fn on_context_initialized(&self) {
            // Créer le client (cycle de vie + blocage trackers).
            let client = FaradayClient::new(FaradayHandler::new());
            *self.client.borrow_mut() = Some(client.clone());

            let settings = BrowserSettings::default();

            let config = PrivacyConfig::load();
            let url = CefString::from(config.default_search_engine.as_str());

            let window_info = crate::win::window_info();

            // Crée le navigateur dans la fenêtre native.
            let mut client_opt = self.client.borrow_mut();
            browser_host_create_browser(
                Some(&window_info),
                client_opt.as_mut(),
                Some(&url),
                Some(&settings),
                None,
                None,
            );
        }
    }
}
