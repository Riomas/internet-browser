//! Processus helper de Faraday (renderer / GPU).
//!
//! CEF lance ce binaire pour les processus enfants (renderer, GPU, etc.).
//! Il ne fait qu'appeler `execute_process`, qui délègue au bon type de
//! processus en fonction du flag `--type` passé par CEF.

use cef::{args::Args, *};

fn main() {
    let args = Args::new();

    // Déclare la version d'API CEF utilisée.
    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);

    // Laisse CEF exécuter ce processus comme enfant (renderer/GPU/etc.).
    execute_process(
        Some(args.as_main_args()),
        None::<&mut App>,
        std::ptr::null_mut(),
    );
}
