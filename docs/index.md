# 🛡️ Faraday

**Navigateur web « privacy-first » pour Windows.** La protection est le réglage par défaut :
rien à configurer, vous êtes protégé dès le premier clic.

Écrit en **Rust** sur **Chromium embarqué (CEF)**, avec le **sandbox Chromium actif** et
**aucune télémétrie**.

[⬇️ Télécharger la dernière version](https://github.com/Riomas/internet-browser/releases/latest){: .btn }
[📖 Guide de l'utilisateur](GUIDE_UTILISATEUR.html){: .btn }
[💻 Code source](https://github.com/Riomas/internet-browser){: .btn }

---

## Téléchargement

| Fichier | Utilisation |
|---|---|
| `Faraday-Setup-0.1.2-x64.exe` | **installateur** — installation pour votre compte, **sans droits administrateur** |
| `Faraday-0.1.2-x64-portable.zip` | **version portable** — décompresser, approuver le certificat, lancer `faraday.exe` |
| `SHA256SUMS.txt` | empreintes SHA-256 pour vérifier l'intégrité |

**[→ Page des versions (releases)](https://github.com/Riomas/internet-browser/releases)**

**Prérequis** : Windows 10 (1809+) ou Windows 11, 64 bits.

## Installation en deux étapes

1. Lancez `Faraday-Setup-0.1.2-x64.exe` et suivez l'assistant (aucun mot de passe
   administrateur demandé).
2. À l'écran **« Certificat de signature Faraday »**, laissez la case cochée : le certificat
   public de Faraday est ajouté aux certificats de confiance de **votre compte**
   (il est retiré automatiquement à la désinstallation).

> 🔐 Faraday est signé par un **certificat auto-signé** (en attendant un certificat
> d'autorité de certification). Cette approbation est **nécessaire au démarrage** : le
> *bootstrap* de Chromium vérifie la signature des binaires.
> Détails, retrait et points de sécurité : [certificat auto-signé](CERTIFICAT_AUTOSIGNE.html).

## Protection par défaut

- **Blocage des domaines de tracking et de publicité** (liste de règles embarquée)
- **Filtrage des cookies tiers** et en-tête **Do Not Track**
- **Déblocage ponctuel par site** (bouton bouclier) et **suspension temporaire** de toute la
  protection en un clic — elle revient au redémarrage
- **Compteur de blocages par site**, avec le détail des autres sites dans l'infobulle
- **Sandbox Chromium actif** : isolation des processus du navigateur
- Référents (`Referer`) supprimés, HTTPS forcé, IP locale masquée pour WebRTC
- Validation externe : **[EFF Cover Your Tracks](https://coveryourtracks.eff.org/)** —
  *blocking tracking ads / invisible trackers* : protection effective

## Vie privée

- **Aucune télémétrie, aucun compte, aucune donnée envoyée** : tout reste sur votre machine
- Aucun composant tiers téléchargé au démarrage ; mises à jour de composants désactivées
- Données locales dans `%APPDATA%\Faraday` (profils séparés, historique effaçable)
- Politique complète : [confidentialité](CONFIDENTIALITE.html)

## Fonctionnalités

- Onglets, favoris (`Ctrl+D`), historique (`Ctrl+H`), téléchargements (`Ctrl+J`)
- Profils séparés, thème clair/sombre/système, session restaurée au démarrage
- Interface en français

## Documentation

| Document | Contenu |
|---|---|
| [Guide de l'utilisateur](GUIDE_UTILISATEUR.html) | installation, prise en main, dépannage |
| [Confidentialité](CONFIDENTIALITE.html) | ce qui est collecté (rien) et ce qui reste local |
| [Certificat auto-signé](CERTIFICAT_AUTOSIGNE.html) | pourquoi Windows demande d'approuver le certificat |
| [Signature de code](SIGNATURE.html) | comment Faraday est signé, et les pistes de certification |

## Licence

**MIT _ou_ Apache-2.0**, au choix : voir [`LICENSE-MIT`](https://github.com/Riomas/internet-browser/blob/main/LICENSE-MIT)
et [`LICENSE-APACHE`](https://github.com/Riomas/internet-browser/blob/main/LICENSE-APACHE).

*Faraday — Windows 10 / 11 (64 bits).*
