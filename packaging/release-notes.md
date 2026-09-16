# Faraday v0.1.1

Version de **distribution** : mêmes fonctions que la v0.1.0, mais les binaires sont
**signés** et l'installateur prend en charge l'approbation du certificat.

## Signature (nouveau)

- Lot applicatif **signé** (`faraday.exe`, `faraday_helper.exe`, `chrome_elf.dll`,
  `faraday.dll`) et **installateur signé**, avec un certificat **auto-signé**
  `CN=Faraday, O=Faraday Project` (valide jusqu'au 16/09/2036, horodaté).
- L'installateur propose d'approuver le certificat dans les *Autorités de certification
  racines de confiance* du **compte utilisateur** (aucun droit administrateur) ; il est
  retiré automatiquement à la désinstallation.
- Cette approbation est **nécessaire au démarrage** : le *bootstrap* de CEF vérifie la
  signature de `faraday.exe`, `chrome_elf.dll` et `faraday.dll`.
- **Version portable** : approuver le certificat une fois avant de lancer :
  ```powershell
  powershell -ExecutionPolicy Bypass -File .\faraday-certificat.ps1 -Install
  ```
- Détails, retrait et points de sécurité : `docs/CERTIFICAT_AUTOSIGNE.md`.

> ⚠️ Un certificat auto-signé n'a **pas** la réputation Microsoft : **Smart App Control**
> reste bloquant et **SmartScreen** peut afficher un avertissement. Un certificat
> d'autorité (en préparation) lèvera ces deux points.

## Installation

| Fichier | Utilisation |
|---|---|
| `Faraday-0.1.1-x64-portable.zip` | version portable : approuver le certificat, puis lancer `faraday.exe` |
| `Faraday-Setup-0.1.1-x64.exe` | installateur par utilisateur (**sans droits administrateur**) |
| `SHA256SUMS.txt` | empreintes SHA-256 pour vérifier l'intégrité |

**Prérequis** : Windows 10 (1809+) ou Windows 11, 64 bits.

---

# Faraday v0.1.0

Première version publique de **Faraday**, navigateur « privacy-first » pour Windows,
écrit en Rust sur Chromium embarqué (CEF).

## Protection

- Blocage des domaines de tracking et de publicité (liste de règles embarquée)
- Filtrage des cookies tiers et en-tête **Do Not Track**
- **Déblocage ponctuel par site** (bouton bouclier) et **suspension temporaire** de toute
  la protection en un clic — la protection revient au redémarrage
- Compteur de blocages **par site**, avec le détail des autres sites dans l'infobulle
- **Sandbox Chromium actif** : isolation des processus du navigateur
- 28 tests unitaires (`cargo test --package faraday --lib`)

## Vie privée

- **Aucune télémétrie, aucun compte, aucune donnée envoyée** : tout reste sur votre machine
- Aucun composant tiers téléchargé au démarrage ; mises à jour de composants désactivées
- Données locales dans `%APPDATA%\Faraday` (profils séparés, historique effaçable)
- Référents (`Referer`) supprimés, HTTPS forcé, IP locale masquée pour WebRTC

## Fonctionnalités

- Onglets, favoris (`Ctrl+D`), historique (`Ctrl+H`), téléchargements (`Ctrl+J`),
  profils séparés, thème clair/sombre/système
- Session restaurée au démarrage (onglets + historique)

## Installation

| Fichier | Utilisation |
|---|---|
| `Faraday-0.1.0-x64-portable.zip` | version portable : décompresser puis lancer `faraday.exe` |
| `Faraday-Setup-0.1.0-x64.exe` | installateur par utilisateur (**sans droits administrateur**) |
| `SHA256SUMS.txt` | empreintes SHA-256 pour vérifier l'intégrité |

**Prérequis** : Windows 10 (1809+) ou Windows 11, 64 bits.

Vérification de l'intégrité (PowerShell) :

```powershell
Get-FileHash .\Faraday-0.1.0-x64-portable.zip -Algorithm SHA256
# comparer avec la ligne correspondante de SHA256SUMS.txt
```

## Notes

- Ces binaires ne sont **pas encore signés numériquement** : Windows peut afficher
  « Éditeur inconnu ». La signature de code gratuite (SignPath Foundation) est en cours
  de demande — voir `docs/CERTIFICAT_OPENSOURCE.md`.
- Sur un PC où **Smart App Control** est actif, Windows peut refuser `libcef.dll`
  (composant Chromium amont non signé) : utilisez un Windows Sandbox ou une VM pour tester.

## Licence

MIT **ou** Apache-2.0, au choix : voir `LICENSE-MIT` et `LICENSE-APACHE`.
Les composants amont (CEF / Chromium) sont distribués sous licence BSD.
