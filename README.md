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

- **v0.1.1** — distribution **signée** : lot applicatif et installateur signés par un
  certificat auto-signé (`CN=Faraday`), que l'installateur propose d'approuver dans les
  certificats de confiance du compte utilisateur ([doc](docs/CERTIFICAT_AUTOSIGNE.md)).
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
| `Faraday-0.1.1-x64-portable.zip` | version **portable** : approuver le certificat, puis lancer `faraday.exe` |
| `Faraday-Setup-0.1.1-x64.exe` | **installateur** (par utilisateur, sans droits administrateur) |
| `SHA256SUMS.txt` | empreintes SHA-256 pour vérifier l'intégrité |

> 🔐 Les binaires sont signés par un **certificat auto-signé** fourni dans le lot :
> l'installateur propose de l'approuver pour votre compte (aucun droit administrateur),
> et la version portable nécessite une approbation unique :
> `powershell -ExecutionPolicy Bypass -File .\faraday-certificat.ps1 -Install`.
> Sans cette étape, l'application **ne démarre pas** (le *bootstrap* de CEF vérifie la
> signature). Détails : [`docs/CERTIFICAT_AUTOSIGNE.md`](docs/CERTIFICAT_AUTOSIGNE.md).

> Windows 10/11 **64 bits**. Le sandbox Chromium est actif, et aucun composant tiers
> n'est téléchargé au démarrage. Le désinstallateur est fourni par l'installateur.

## Licence

**MIT _ou_ Apache-2.0**, au choix de l'utilisateur : voir [`LICENSE-MIT`](LICENSE-MIT) et
[`LICENSE-APACHE`](LICENSE-APACHE).

## Politique de signature de code

- **Signé par certificat auto-signé (procédure [`docs/CERTIFICAT_AUTOSIGNE.md`](docs/CERTIFICAT_AUTOSIGNE.md)).**
  Le certificat public est joint aux artefacts (`faraday-certificat.cer`) et l'installateur
  propose de l'approuver dans les certificats de confiance du **compte de l'utilisateur**
  (aucun droit administrateur, retiré à la désinstallation). Cette étape est nécessaire au
  démarrage de l'application, le *bootstrap* de CEF vérifiant la signature de
  `faraday.exe`, `chrome_elf.dll` et `faraday.dll`.
- **Objectif à terme** : un certificat d'autorité de certification (AC) pour supprimer tout
  avertissement Windows. Parcours prêts : [`docs/CERTUM.md`](docs/CERTUM.md) et
  [`docs/CERTIFICAT_OPENSOURCE.md`](docs/CERTIFICAT_OPENSOURCE.md) (⏸️ en pause).
- **Signature gratuite (SignPath Foundation) : refusée** le 13/09/2026 — motif : réputation du
  projet jugée insuffisante. Réessayable ultérieurement.
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

`docs/` contient le guide utilisateur, le guide de signature, la **distribution avec
certificat auto-signé** (**`CERTIFICAT_AUTOSIGNE.md`**, mode actuel), la procédure de test en
machine propre, le parcours de certification (**`CERTUM.md`**, en pause) et la marche à
suivre pour un certificat open source.
