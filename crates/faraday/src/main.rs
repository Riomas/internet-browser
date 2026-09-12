//! Point d'entrée du **binaire** `faraday.exe` (mode classique, sans sandbox).
//!
//! Le cœur de l'application vit dans `lib.rs` : il est partagé entre ce binaire
//! et la DLL `faraday.dll` utilisée en mode sandbox (lancée par `bootstrap.exe`).
//!
//! Ici, aucun `sandbox_info` n'est fourni → le sandbox est désactivé (mode dev /
//! version non sandboxée).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> anyhow::Result<()> {
    faraday_core::browser_main(std::ptr::null_mut())
}
