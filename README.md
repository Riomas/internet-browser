# 🛡️ Faraday

> Navigateur web « privacy-first » basé sur **Chromium embarqué (CEF)**, codé en **Rust**.
> **Tous les trackings sont désactivés par défaut.** Cible : **Windows 10+**.

## Structure du projet

```
internet-browser/
├── Cargo.toml                 # Workspace Cargo
├── setup.ps1                  # Install Rust + CEF_PATH + build
├── crates/
│   └── faraday/
│       ├── Cargo.toml
│       ├── build.rs
│       ├── configs/
│       │   └── privacy.toml   # Configuration « zéro tracking »
│       └── src/
│           ├── main.rs        # Point d'entrée (boot CEF)
│           ├── app.rs         # Application CEF (flags privacy)
│           ├── handler.rs     # Client + blocage trackers
│           ├── privacy.rs     # Switches Chromium
│           └── win.rs         # Détails Windows
```

## Prérequis
- **Windows 10 (1809+) / Windows 11**
- **Rust** (installé automatiquement par `setup.ps1`)
- Connexion internet pour le **premier build** (télécharge les binaires CEF)

## Installation & build

```powershell
.\setup.ps1
```

Le premier build télécharge les binaires CEF (~150 Mo) via le `build.rs` du
crate `cef`. Cela peut prendre plusieurs minutes.

## Lancer

```powershell
cargo run --package faraday
```

Une fenêtre Chromium doit s'ouvrir sur **DuckDuckGo** (moteur de recherche par
défaut, respectueux de la vie privée).

## Confidentialité (« zéro tracking » par défaut)

Les switches appliqués au démarrage (voir `crates/faraday/configs/privacy.toml`) :
- `--disable-component-update` — pas de télémesure de composants.
- `--disable-default-apps` — pas d'applications par défaut.
- `--disable-sync` — pas de synchronisation de compte.
- `--disable-suggestions` — pas de suggestions de recherche.
- `--disable-personal-autofill` — pas d'autofill personnel.
- `--force-https` — HTTPS forcé quand disponible.
- `--disable-features=Preload,PrefetchPrivacyChanges,NetworkService`.
- `--webrtc-ip-handling-policy=disable_non_proxied_udp` — masque l'IP réelle.

Le blocage réseau des trackers est géré dans `handler.rs` (domaines de tracking +
publicité rejetés avant chargement).

## Étapes suivantes (Phase 1+)
- UI du navigateur (onglets, barre d'adresse) — `egui`/`iced`.
- Page d'accueil Faraday locale.
- Liste de règles complète (EasyList + anti-track) en Phase 2.
