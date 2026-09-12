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

## État du projet

- **v0.1.0** — phases 0 à 5 terminées : noyau CEF, blocage des trackers/publicités,
  interface complète (onglets, favoris, historique, téléchargements, profils,
  déblocage par site, suspension temporaire), **sandbox Chromium**, distribution
  portable + installateur.
- **28 tests unitaires** : `cargo test --package faraday --lib`.
- Validation externe : **EFF Cover Your Tracks** (*blocking tracking ads / invisible
  trackers : Yes*) sur une installation propre.

### Modules principaux (`crates/faraday/src/`)

| Module | Rôle |
|---|---|
| `lib.rs` | démarrage CEF, appel `RunWinMain` (variante sandbox) |
| `chrome.rs` | interface egui : onglets, barre d'outils, réglages |
| `handler.rs` | client CEF : blocage réseau, cookies tiers, DNT |
| `blocklist.rs` | moteur de règles + exceptions par site + statistiques |
| `privacy.rs`, `configs/privacy.toml` | configuration « zéro tracking » par défaut |
| `bookmarks.rs`, `history.rs`, `session.rs`, `profiles.rs`, `downloads.rs` | données locales |
| `packaging/` (`build-release.ps1`, `faraday.iss`) | zip portable, installateur, signature |

## Téléchargement

Les versions publiées sont sur la page **[Releases](https://github.com/Riomas/internet-browser/releases)** :

| Fichier | Description |
|---|---|
| `Faraday-0.1.0-x64-portable.zip` | version **portable** : décompresser puis lancer `faraday.exe` |
| `Faraday-Setup-0.1.0-x64.exe` | **installateur** (par utilisateur, sans droits administrateur) |
| `SHA256SUMS.txt` | empreintes SHA-256 pour vérifier l'intégrité |

> Windows 10/11 **64 bits**. Le sandbox Chromium est actif, et aucun composant tiers
> n'est téléchargé au démarrage. Le désinstallateur est fourni par l'installateur.

## Licence

**MIT _ou_ Apache-2.0**, au choix de l'utilisateur : voir [`LICENSE-MIT`](LICENSE-MIT) et
[`LICENSE-APACHE`](LICENSE-APACHE).

## Politique de signature de code

- **Free code signing provided by [SignPath.io](https://about.signpath.io/), certificate by [SignPath Foundation](https://signpath.org/).**
- Mainteneur (auteur, relecteur et approbateur) : [@Riomas](https://github.com/Riomas)
- Validation des changements : toute contribution externe passe par une *pull request* relue
  avant intégration.
- **Vie privée : ce programme ne transfère aucune information vers un système réseau tiers
  sans demande explicite de l'utilisateur ou de la personne qui l'installe.** Aucune
  télémétrie, aucun compte, aucun envoi de statistiques. Politique complète :
  [`docs/CONFIDENTIALITE.md`](docs/CONFIDENTIALITE.md) ; voir aussi
  `docs/GUIDE_UTILISATEUR.md`, « Vos données ».
- Composants amont inclus : `libcef.dll`, `chrome_elf.dll`, `libEGL.dll`, `libGLESv2.dll`,
  `vk_swiftshader.dll`, `d3dcompiler_47.dll`… proviennent de **CEF / Chromium** (licence
  BSD) et sont redistribués tels quels. `faraday.exe` est le *bootstrap* de CEF renommé ;
  le code propre à Faraday est compilé dans `faraday.dll`.

## Documentation

`docs/` contient le guide utilisateur, le guide de signature, la procédure de test en
machine propre et la marche à suivre pour un certificat open source.
