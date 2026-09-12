# 📦 Faraday — Phase 5 · Distribution & packaging

> Statut : **EN COURS** — Phase 5 (Distribution, sem. 15–17)
> Objectif : installer/portable Windows 10/11 x64, signature, auto-update, docs.
> Décisions : **portable + installateur** · **x64** · **non signé pour l'instant** (hooks SignTool prêts).

---

## 1. Ce qui est livré (dossier `packaging/`)

| Fichier | Rôle |
|---|---|
| `build-release.ps1` | Build `--release`, regroupe l'app dans `dist\Faraday\`, **signe** `faraday.exe`/`faraday_helper.exe`, crée le **zip portable**, compile et **signe l'installateur** (`-Inno`), écrit `dist\SHA256SUMS.txt`. |
| `faraday.iss` | Script **Inno Setup 6** → installateur `dist\Faraday-Setup-0.1.0-x64.exe` (installation **par utilisateur**, sans admin/UAC). |
| `make-icon.ps1` | Génère `crates\faraday\resources\icons\faraday.ico` (bouclier privacy, 16/32/48/256 px, format DIB compatible partout). |
| `make-testcert.ps1` | Crée un **certificat de test auto-signé** (+ `-Trust`, `-Remove`) pour valider le pipeline de signature sans acheter de certificat. |
| `build-signed.ps1` | Raccourci : reconstruit une distribution **signée** en une commande (utilise le certificat de test par défaut). |
| `docs/SIGNATURE.md` | **Guide de signature de code** (types de certificats, coûts, token/HSM, Azure Trusted Signing). |
| `PHASE5_DISTRIBUTION.md` | Ce guide. |

Artéfacts générés (racine `dist/`) :
```
dist/
  Faraday/                             <- dossier applicatif complet (à zipper/tester)
  Faraday-0.1.0-x64-portable.zip       <- version portable
  Faraday-Setup-0.1.0-x64.exe          <- installateur (après ISCC)
```

---

## 2. Contenu de l'application (runtime)

Le dossier `dist\Faraday\` contient **tout le nécessaire**, sans dépendance au chemin de dev :

- `faraday.exe` + `faraday_helper.exe` (icône + version embarquées via `winres`)
- `libcef.dll` (Chromium 152), `chrome_elf.dll`, `libEGL/libGLESv2.dll`, `vk_swiftshader*`
- `icudtl.dat`, `resources.pak`, `chrome_100/200_percent.pak`, `v8_context_snapshot.bin`
- `d3dcompiler_47.dll`, `dxcompiler.dll`, `dxil.dll`, `vulkan-1.dll`
- **`vcruntime140.dll` + `vcruntime140_1.dll`** — runtime Visual C++ embarqué « app-local »
  (nos `faraday.exe`/`faraday_helper.exe` l'importent ; `libcef.dll` non). Indispensable sur
  une machine **sans VC++ Redistributable** (VM/Sandbox propre) : sinon erreur
  « `VCRUNTIME140.dll` est introuvable ». Copié depuis le dossier redistribuable officiel
  Visual Studio (`…\VC\Redist\MSVC\…\x64\Microsoft.VC143.CRT`), repli `System32`.
- `locales\` (220 fichiers de langue Chromium)
- `CREDITS.html` (licences), `bootstrap*.exe` (sandbox, inoffensifs sinon)
- **Aucune config ni donnée à côté de l'exe** : la blocklist et la config privacy sont
  **embarquées dans le binaire** ; le profil utilisateur est créé au 1er lancement dans
  `%APPDATA%\Faraday\` (session.json, privacy.toml). → **0 écriture dans Program Files**,
  idéal pour une installation par utilisateur.

> ⚠️ Correctif clé apporté en Phase 5 : la config `privacy.toml` était lue depuis le
> **chemin absolu de compilation** (`CARGO_MANIFEST_DIR`), inexistant sur un PC propre.
> Elle est désormais **embarquée** (`include_str!`) et surchargée par
> `%APPDATA%\Faraday\privacy.toml` (créé au 1er lancement). Les réglages de
> confidentialité restent activés par défaut partout.

---

## 3. Générer une distribution

### Prérequis (une seule fois)
- Rust stable (MSVC) + CMake + Ninja (déjà en place pour le dev).
- [Inno Setup 6](https://jrsoftware.org/isinfo.php) — **seulement si** tu veux l'installateur .exe.
- (Optionnel) Windows SDK `signtool.exe` + certificat de code pour signer.

### Étapes
```powershell
# 1) Version portable + dossier app
powershell -ExecutionPolicy Bypass -File .\packaging\build-release.ps1

# 2) + installateur Inno (si Inno Setup installé)
powershell -ExecutionPolicy Bypass -File .\packaging\build-release.ps1 -Inno

# 3) + sandbox Chromium activé (à valider sur VM propre)
powershell -ExecutionPolicy Bypass -File .\packaging\build-release.ps1 -Sandbox -Inno

# 4) + signature (quand tu auras un certificat)
powershell -ExecutionPolicy Bypass -File .\packaging\build-release.ps1 -CertPath cert.pfx -CertPass "***"
```

---

## 4. Sandbox Chromium (sécurité)

> ✅ **IMPLÉMENTÉ** (2026-09-12) — modèle officiel CEF M138+.

Depuis **Chromium M138**, la bibliothèque statique `cef_sandbox` n'est plus
distribuée : le sandbox Windows exige désormais que **l'application soit une DLL**
exportant `RunWinMain`, lancée par `bootstrap.exe` (fourni par CEF) qui lui fournit
le `sandbox_info` (pointeur opaque). Référence :
<https://chromiumembedded.github.io/cef/sandbox_setup>

**Architecture mise en place** :

| Élément | Rôle |
|---|---|
| `faraday.exe` | copie de `bootstrap.exe` (CEF) — met en place le sandbox et charge la DLL |
| `faraday.dll` | l'application (`faraday_core.dll` renommée) — exporte **`RunWinMain`** |
| `src/lib.rs` | cœur partagé : `browser_main(sandbox_info)` + export `RunWinMain` |
| `src/main.rs` | binaire seul (mode **sans** sandbox) : appelle `browser_main(null)` |

Le code détecte le mode automatiquement :
`sandbox_info` **non nul** (lancé par le bootstrap) → `no_sandbox = false` et le
pointeur est transmis à `execute_process` / `initialize` ; **nul** (exe direct) →
sandbox désactivé. Aucune feature de compilation n'est nécessaire.

**Construire la variante sandbox** :
```powershell
.\packaging\build-release.ps1 -Sandbox [ -Inno ] [ -CertPath … -CertPass … ]
```

**Vérifier que le sandbox est actif** : l'interface l'affiche (pied de page de la
page d'accueil et infobulle du bouclier → « Sandbox Chromium : ACTIF »), et
`%APPDATA%\Faraday\startup.log` contient `sandbox=active`.

> ⚠️ La variante sandbox doit être **validée sur une machine propre** (le sandbox
> CEF échoue au démarrage s'il est mal configuré — c'est un bon indicateur).
> Le fichier `faraday.exe` (bootstrap CEF) porte l'icône/version de CEF : elle peut
> être personnalisée avec Resource Hacker si souhaité.

---

## 5. Signature de code & SmartScreen

> 📘 Guide détaillé : **`docs/SIGNATURE.md`** (choix du certificat, coûts, token/HSM,
> Azure Trusted Signing, vérification).
>
> ✅ **Pipeline implémenté et testé** (2026-09-12) : `build-release.ps1 -Inno -CertPath … -CertPass …`
> signe `faraday.exe`, `faraday_helper.exe` **avant** le zip/l'installateur, puis signe
> l'installateur, et vérifie chaque signature (`signtool verify /pa`). `signtool.exe`
> est repéré automatiquement (PATH → Windows Kits). Ordre correct garanti : les binaires
> signés se retrouvent **dans** le zip et l'installateur. `SHA256SUMS.txt` couvre les
> archives (un `.zip` ne peut pas être signé).

Sans signature, Windows affiche « Éditeur inconnu / Windows a protégé votre PC » au
téléchargement/lancement. Pour une diffusion publique il faudra :

1. Acheter un **certificat de signature de code** (OV/EV) auprès d'un émetteur (DigiCert,
   Sectigo…), ~200–600 €/an. L'EV lève aussi la réputation SmartScreen plus vite.
2. Signer `faraday.exe`, `faraday_helper.exe` et l'installateur (SHA-256 + horodatage).
   Le script `build-release.ps1 -CertPath …` fait le nécessaire via `signtool.exe`.

> ⚠️ Un certificat **auto-signé** (créé par `make-testcert.ps1`) permet de **valider
> l'outillage** mais **ne débloque pas** Smart App Control / SmartScreen : seul un
> certificat délivré par une AC de confiance le fait.

### ⚠️ Smart App Control (Windows 11) — bloque les binaires non signés

Symptôme observé en test : au lancement, « **Image incorrecte** » sur `libcef.dll`
(code `0xc0e90002`), ou « **Une stratégie de contrôle d'application a bloqué ce fichier** ».

**Cause** : Smart App Control (SAC) est **activé**. La clé
`HKLM\SYSTEM\CurrentControlSet\Control\CI\Policy\VerifiedAndReputablePolicyState`
vaut **1** (= application). SAC exige des binaires **signés réputés** ; or Faraday 0.1.0
n'est pas signé → Windows bloque l'exe et/ou le chargement de `libcef.dll`.
(Note : SAC démarre en mode *évaluation* — d'où des lancements réussis au début — puis
passe en mode *application* automatiquement.)

**Options** :
- **Signer** les binaires (solution pérenne, cf. ci-dessus) ;
- **Désactiver SAC** : Sécurité Windows → Contrôle des applications et du navigateur →
  Smart App Control → Désactivé ⚠️ **irréversible sans réinstallation de Windows** ;
- **Tester sur une VM / un PC sans SAC** (recommandé pour valider la distribution).

> Cette contrainte ne concerne pas le développement habituel sur une machine sans SAC.

---

## 6. Auto-update (piste, Phase 5+)

Le switch `--disable-component-update` empêche Chromium de se mettre à jour tout seul : toute
mise à jour doit passer par **notre** mécanisme.

**Stratégie recommandée (plus tard)** :
- **Serveur** : un simple JSON `latest.json` (`version`, `url_installer`, `sha256`) sur un
  hébergement HTTPS statique (Pages/Netlify/S3).
- **Côté client** : vérification périodique (ex. chaque lancement, différé de quelques
  secondes), comparaison de version, téléchargement + vérification SHA-256, invocation de
  l'installateur en mode silencieux (`/VERYSILENT /SUPPRESSMSGBOXES` pour Inno) et relance.
- **Signature obligatoire** des artefacts avant diffusion publique.

---

## 7. Vérifications avant publication (checklist)

> Procédure détaillée : **`docs/TEST_VM.md`** (options Hyper-V / VirtualBox / Windows Sandbox
> + checklist complète à remplir).

- [x] **Test machine propre** — ✅ validé le 2026-09-12 en **Windows Sandbox** (portable) :
      l'application démarre et s'affiche (après correction du runtime CRT, cf. §2).
- [x] **EFF Cover Your Tracks : Yes / Yes** — ✅ validé le 2026-09-12 sur la **build
      distribuée** (zip portable) dans l'environnement propre.
- [ ] Au 1er lancement : `%APPDATA%\Faraday\` créé, `privacy.toml` généré (valeurs par défaut privées).
- [ ] Aucune requête sortante vers Google/services tiers au démarrage (netstat / proxy).
- [ ] Désinstallation : supprime l'app, **option** de suppression des données.
- [x] **Sandbox Chromium validé** — ✅ 2026-09-12 : la variante sandbox (`-Sandbox`) s'ouvre
      dans Windows Sandbox et l'interface affiche **« Sandbox Chromium : ACTIF »**.
- [ ] (Avant diffusion publique) Signature de code + page de téléchargement HTTPS
      → sinon Smart App Control / WDAC bloquent les binaires non signés (cf. §5).

---

*Faraday — Phase 5 Distribution · document de référence.*
