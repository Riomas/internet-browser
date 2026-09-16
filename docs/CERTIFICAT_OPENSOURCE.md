# 🔏 Obtenir un certificat de signature « open source »

> Objectif : supprimer l'avertissement « Éditeur inconnu » et construire la réputation
> SmartScreen, **sans rien payer** si possible.

**État au 13/09/2026 — vérifié sur les sites officiels.**

---

## 0. Le constat (à connaître avant de s'engager)

| Piste | Gratuit ? | Réalité |
|---|---|---|
| **SignPath Foundation** | ✅ **oui** | Certificat émis au nom de *SignPath Foundation* (c'est **leur** nom qui apparaît comme éditeur), clé privée dans leur HSM, signature intégrée à la CI. Conditions strictes (voir §2). |
| **Certum « Open Source Code Signing »** | ❌ **plus gratuit** | Le programme historique a été remplacé par un produit **remisé** : **29 $** si vous possédez déjà une carte cryptoCertum + lecteur, sinon **≈ 69 €** (certificat + carte + lecteur). Vérification d'identité, **matériel obligatoire** (règles CA/Browser Forum 2023), renouvellement annuel. |
| Azure Trusted Signing | ❌ | Réservé aux organisations avec **3 ans d'existence vérifiable** (~10 €/mois au-delà). Pas accessible à un projet neuf. |
| Certificat OV / EV classique | ❌ | 200–400 €/an + token HSM. |

> ⚠️ Depuis juin 2023, **aucune autorité ne délivre plus de certificat de signature
> « fichier simple »** : la clé privée doit être sur un support matériel (carte, token
> USB) ou dans un HSM. C'est ce qui a fait disparaître les certificats gratuits
> « téléchargeables ». Les seules voies gratuites passent par un **tiers de confiance
> qui signe à votre place** (SignPath Foundation).

---

## 1. Prérequis communs (avant de candidater)

- [x] **Licence OSI** : `LICENSE-MIT` + `LICENSE-APACHE` à la racine (déclarée
      `license = "MIT OR Apache-2.0"` dans `Cargo.toml`).
- [ ] **Dépôt public** — ⚠️ **bloquant aujourd'hui** : `github.com/Riomas/internet-browser`
      répond *404* à l'API GitHub, donc il est **privé**. Les deux programmes exigent un
      dépôt **public**. → *Settings → General → Danger Zone → Change visibility*.
- [ ] **Projet documenté & publié** (README : fonctionnalités, installation, prérequis).
- [ ] **Politique de signature de code** visible sur la page d'accueil du projet
      (obligatoire pour SignPath, texte imposé — voir README, section dédiée).
- [ ] **Build reproductible en CI** : `.github/workflows/build.yml` fourni dans ce dépôt.
- [x] **Désinstallateur** : fourni par l'installateur Inno Setup.

---

## 2. Option A (recommandée, gratuite) — SignPath Foundation

### Conditions à remplir
- Licence OSI **sans double licence commerciale** ✔
- Code **entièrement open source** (les binaires amont non signés, comme `libcef.dll`,
  peuvent être **inclus** dans un paquet signé) ✔
- Projet **maintenu**, **déjà publié**, **documenté** ✔ (après publication)
- **Pas d'outil de hacking**, respect de la vie privée, désinstallation possible ✔
- Rôles d'équipe déclarés (auteurs / relecteurs / approbateurs) + **MFA** sur GitHub et SignPath
- **Politique de signature** sur la page du projet, avec la mention imposée :
  *« Free code signing provided by SignPath.io, certificate by SignPath Foundation »*
- Chaque version exige une **approbation manuelle** de signature

### Étapes
1. Passer le dépôt en **public** et y pousser la licence + la politique de signature.
2. Créer le workflow CI (déjà fourni : `.github/workflows/build.yml`) et vérifier qu'un
   build produit bien le zip + l'installateur en artefacts.
3. S'inscrire sur <https://signpath.io/> puis candidater : <https://signpath.org/apply>.
4. Décrire le projet et **demander explicitement** si `chrome_elf.dll` (DLL runtime de CEF,
   BSD, **imposée par le bootstrap CEF**) peut être signée avec le certificat du projet —
   voir §4, c'est le point à clarifier pour signer l'application elle-même.
5. Une fois accepté : ajouter `signpath/github-action` au workflow (l'action signe les
   artefacts côté SignPath, avec approbation manuelle de la release).

### Ce que SignPath peut signer — et la limite liée à CEF
| Fichier | Signable par SignPath ? |
|---|---|
| `Faraday-Setup-0.1.0-x64.exe` (installateur Inno, **notre** binaire) | ✅ oui |
| `faraday.exe` (bootstrap CEF renommé), `faraday.dll` (notre code) | ⚠️ **seulement avec `chrome_elf.dll`** (règle CEF du §4) |
| `chrome_elf.dll` (binaire **amont** CEF) | ❌ par défaut : les binaires amont ne doivent pas être signés avec l'abonnement |
| `libcef.dll` et autres DLL CEF | ❌ idem |

➡️ **Conséquence** : au minimum, SignPath supprime l'avertissement sur le **fichier
téléchargé** (l'installateur, qui est ce que l'utilisateur exécute en premier). Signer
aussi l'application dépend de leur réponse sur `chrome_elf.dll`.

---

## 3. Option B (payante) — Certum Open Source Code Signing

- Produit : <https://certum.store/open-source-code-signing-code.html> (29 $, activation sur
  une carte Certum que vous possédez déjà) ou pack ≈ 69 € avec carte + lecteur
  (<https://shop.certum.eu/open-source-code-signing.html>, ou variante « on SimplySign » =
  carte virtuelle, aucun matériel à recevoir).
- Public visé : **un mainteneur individuel** ; le certificat est délivré **à votre nom**
  (pas au projet).
- Étapes : achat → **vérification d'identité** (documents + éventuel appel) → réception de
  la carte ou activation SimplySign → signature via `signtool` (§5).
- Avantages : vous signez **tout** le lot (y compris `chrome_elf.dll`), donc l'application
  démarre signée et l'éditeur affiché est **vous**. Renouvellement annuel ≈ 29 $.
- Inconvénient : matériel/abonnement à conserver, et procédure d'identité.

---

## 4. La règle CEF à ne pas oublier (déjà rencontrée en test)

`bootstrap.exe` (notre `faraday.exe`) vérifie au démarrage la signature de :

1. `faraday.exe` lui-même,
2. `chrome_elf.dll`,
3. `faraday.dll` (DLL client).

Règle binaire : **soit tous non signés, soit tous signés par le même certificat, et ce
certificat approuvé sur la machine cible**. Sinon :

```
FATAL:…bootstrap_win.cc Failed …\faraday.exe certificate checks: WinVerifyTrust failed
```

Aucune fenêtre ne s'ouvre (un `debug.log` apparaît à côté de l'exe).

➡️ Conséquence pratique : **dès qu'on signe `faraday.exe`, `chrome_elf.dll` doit l'être
aussi**. Comme `chrome_elf.dll` est fourni par CEF, cela implique de **re-signer un binaire
amont** (autorisé par la licence BSD de CEF, à documenter dans la politique de signature).

➡️ Rappel **Smart App Control** : sur un PC où SAC est actif, `libcef.dll` (non signé par
CEF) peut être refusé par Windows. Avec un certificat d'AC, on peut signer **tout le lot
CEF** ; sinon, ces machines doivent désactiver SAC (réglage irréversible sans
réinstallation — à mentionner dans la documentation de diffusion).

---

## 5. La signature côté build (déjà prêt)

`packaging/build-release.ps1` gère les deux modes :

```powershell
# A) certificat sous forme de fichier .pfx (pratique pour les tests)
.\packaging\build-release.ps1 -Sandbox -Inno -CertPath .\certs\faraday-test.pfx -CertPass "faraday-test"

# B) certificat du magasin : carte cryptoCertum, token USB ou SimplySign (HSM)
.\packaging\build-release.ps1 -Sandbox -Inno -CertThumbprint <EMPREINTE>
#    avec SimplySign, ajouter le fournisseur :
.\packaging\build-release.ps1 -Sandbox -Inno -CertThumbprint <EMPREINTE> -Csp "Certum SimplySign"

# Pour signer AUSSI le lot applicatif (et donc chrome_elf.dll, cf. §4) :
.\packaging\build-release.ps1 -Sandbox -Inno -CertThumbprint <EMPREINTE> -SignAppFiles
```

- L'**installateur est toujours signé** (aucune contrainte CEF sur cet exécutable).
- Le **lot applicatif** n'est signé que sur demande (`-SignAppFiles`) — parce qu'un lot
  partiellement signé, ou signé par un certificat non approuvé, **ne démarre pas**.
- La voie « magasin + `/sha1` + horodatage DigiCert » a été **testée le 13/09/2026** :
  signature `Valid` avec horodatage.

---

## 6. Plan d'action recommandé

1. **Décision** : rendre le dépôt public ? (prérequis absolu des deux voies)
2. Si oui → pousser `LICENSE-MIT`, `LICENSE-APACHE`, le README « politique de signature »
   et `.github/workflows/build.yml`, puis vérifier qu'un build CI passe.
3. **Candidater à SignPath Foundation** (gratuit) en posant la question de `chrome_elf.dll`.
4. Si SignPath refuse la signature de l'application : soit on se contente de
   l'installateur signé, soit on passe à **Certum** (≈ 69 € la première année) pour signer
   tout le lot avec votre nom.
5. Mettre à jour `docs/SIGNATURE.md` avec les identifiants définitifs (empreinte,
   fournisseur CSP) et **régénérer** `dist\` avec `-SignAppFiles` le jour venu.

---

## 7. Dossier de candidature SignPath (texte prêt à coller)

Formulaire : <https://signpath.org/apply> (le site est en anglais → texte ci-dessous en anglais).

### Project

```
Project name: Faraday
Repository:   https://github.com/Riomas/internet-browser
License:      MIT OR Apache-2.0 (files LICENSE-MIT and LICENSE-APACHE)
Platform:     Windows 10/11 x64
Language:     Rust (with the Chromium Embedded Framework, CEF)
```

### Short description

```
Faraday is a privacy-first web browser for Windows, written in Rust on top of the
Chromium Embedded Framework. It blocks tracking and advertising domains from an
embedded rule list, filters third-party cookies, sends Do Not Track, and enables the
real Chromium sandbox. Everything is local: no telemetry, no account, no data sent
anywhere. It ships as a portable ZIP and a per-user installer, with a built-in
uninstaller.
```

### What we would like to sign

```
1. Faraday-Setup-<version>-x64.exe  (our Inno Setup installer)

We would also like to sign the application itself, but the CEF bootstrap imposes a
constraint: a signed faraday.exe only starts if chrome_elf.dll and faraday.dll are
signed with the SAME certificate. chrome_elf.dll is an upstream CEF/Chromium runtime
file (BSD licensed) that we redistribute unchanged. Could you confirm whether
chrome_elf.dll may be signed together with our own binaries under this subscription?
If not, we will sign the installer only.
```

### Privacy / uninstall statements (they are part of your conditions)

```
Privacy: the program does not transfer any information to third-party networked
systems unless specifically requested by the user. There is no telemetry, no account,
no background reporting. See the 'Vos donnees' section of docs/GUIDE_UTILISATEUR.md.
Uninstall: a full uninstaller is included (data deletion is an option offered during
uninstallation).
System changes: none. Faraday writes only inside its own folder and %APPDATA%\Faraday.
Team: single maintainer (@Riomas); MFA enabled on GitHub; every external contribution
goes through a reviewed pull request.
```

### Avant d'envoyer le formulaire

- [ ] Dépôt **public** ✔
- [ ] `LICENSE-MIT` / `LICENSE-APACHE` **poussés** sur GitHub
- [ ] Section « Politique de signature de code » visible dans le README public
- [ ] Workflow `.github/workflows/build.yml` **vert** (onglet *Actions*)
- [ ] **Release v0.1.0** publiée avec le zip + l'installateur (obligation « Released »)
- [ ] **2FA activée** sur le compte GitHub (exigence de leurs conditions)

---

## 8. Publier la release v0.1.0 (obligatoire avant de candidater)

**Automatique (recommandé)** — le workflow `.github/workflows/build.yml` construit, teste,
empaquette **puis publie la release** avec les binaires dès qu'un tag `v*` est poussé
(jeton intégré de GitHub Actions : aucun secret à configurer) :

```powershell
git tag v0.1.0
git push origin v0.1.0
```

**Manuel (repli)** : `\packaging\build-release.ps1 -Sandbox -Inno`, puis GitHub →
*Releases* → *Draft a new release* → tag `v0.1.0`, en joignant
`Faraday-0.1.0-x64-portable.zip`, `Faraday-Setup-0.1.0-x64.exe` et `SHA256SUMS.txt`.

Les notes de version publiées vivent dans **`packaging/release-notes.md`** (version longue
reproduite ci-dessous, conservée pour référence) :

```markdown
Première version publique de Faraday, navigateur « privacy-first » pour Windows.

### Protection
- Blocage des domaines de tracking et de publicité (liste embarquée)
- Filtrage des cookies tiers et en-tête Do Not Track
- Déblocage ponctuel par site, et suspension temporaire de toute la protection
- Sandbox Chromium actif (isolation des processus)
- 28 tests unitaires

### Vie privée
- Aucune télémétrie, aucun compte, aucune donnée envoyée : tout reste sur votre machine
- Données locales dans `%APPDATA%\Faraday` (profils séparés, historique effaçable)

### Installation
- `Faraday-0.1.0-x64-portable.zip` : décompresser, lancer `faraday.exe`
- `Faraday-Setup-0.1.0-x64.exe` : installation par utilisateur, sans droits administrateur
- Vérifier l'intégrité avec `SHA256SUMS.txt`
```

---

## 9. Formulaire SignPath : réponses prêtes à coller

Formulaire : <https://signpath.org/apply> (HubSpot, une page, ~5 minutes). Tout est prêt
ci-dessous : **il ne reste que vos prénom, nom et adresse e-mail** (données personnelles,
que vous seul pouvez saisir) et les cases d'acceptation.

| Champ | Réponse |
|---|---|
| **Project Name\*** | `Faraday` |
| **Repository URL\*** | `https://github.com/Riomas/internet-browser` |
| **Homepage URL\*** | `https://github.com/Riomas/internet-browser` |
| **Download URL** | `https://github.com/Riomas/internet-browser/releases` |
| **Privacy Policy URL** | `https://github.com/Riomas/internet-browser/blob/main/docs/CONFIDENTIALITE.md` |
| **Wikipedia URL** | *(vide)* |
| **Tagline\*** | `Privacy-first web browser for Windows, written in Rust on Chromium (CEF)` |
| **Description\*** | voir texte ci-dessous |
| **Reputation\*** | voir texte ci-dessous |
| **Maintainer Type** | `Individual maintainer(s)` *(options : Independent community project / Non-profit / For-profit / **Individual maintainer(s)** / Other)* |
| **Build System** | `GitHub Actions` |
| **First Name / Last Name / Email\*** | ← **vos informations** |
| **Company Name** | *(vide)* |
| **Primary Discovery Channel\*** | `Organic search` *(options : Organic search / AI-LLM tools / Developer platforms / Community platforms / Social media / Events / Referral / Direct contact / Other)* |
| **Please specify the exact source** | `Search while looking for free code signing for open source projects` |
| Cases à cocher | à lire et accepter par vous — la 2ᵉ (communications marketing) est **facultative**, laissez-la décochée |

### Description (à coller)

```
Faraday is a privacy-first web browser for Windows 10/11, written in Rust on top of the
Chromium Embedded Framework (CEF). It is a complete desktop browser (tabs, bookmarks,
history, downloads, separated profiles, light/dark theme) whose default behaviour is to
protect the user:

- blocks tracking and advertising domains using an embedded rule list;
- filters third-party cookies and sends the Do Not Track header;
- strips the Referer header, forces HTTPS where the site supports it, and hides the local
  IP address from WebRTC;
- runs the real Chromium sandbox for process isolation;
- provides per-site unblocking (one click on the shield button) and a temporary global
  pause of all protections, both reversible, plus a per-site counter showing what was
  blocked and by which site.

Everything stays on the user's machine: no telemetry, no account, no crash reporting,
nothing is ever sent anywhere. The program writes only inside its own folder and
%APPDATA%\Faraday, and it ships with a full uninstaller (data deletion is offered during
uninstallation). The project is licensed under MIT OR Apache-2.0, with the complete
source, documentation and build scripts in the public repository. Releases are produced
by a public GitHub Actions workflow (build + 28 unit tests + packaging) from the tagged
commit, and distributed as a portable ZIP and a per-user installer that needs no
administrator rights.

Repository: https://github.com/Riomas/internet-browser

One question, if I may: the CEF bootstrap we use refuses to start unless faraday.exe,
chrome_elf.dll and faraday.dll are all signed by the same certificate. chrome_elf.dll is
an upstream CEF/Chromium runtime file (BSD licence) that we redistribute unchanged.
Would it be possible to sign it together with our own binaries under this subscription?
If that goes against your policy, we will sign the installer only - we completely
understand, and we would rather ask than assume.
```

### Reputation (à coller — factuel et honnête)

```
Faraday is a new project: version 0.1.0, first public release in September 2026.

Facts that anyone can verify today:

- the repository is public and contains the whole source code, the Windows metadata and
  packaging scripts, the test suite and the documentation;
- every release is built by the public GitHub Actions workflow from the tagged commit
  (build, 28 unit tests, packaging), so a signed binary can always be traced back to an
  exact commit;
- the protection was validated on a clean Windows Sandbox (a freshly installed Windows
  virtual machine) with EFF Cover Your Tracks: "Blocking tracking ads: Yes" and
  "Blocking invisible trackers: Yes";
- "no telemetry, no account, no data collection" is a design rule of the project, and the
  privacy policy is published in the repository.

We understand that software which users download and execute needs a verifiable
reputation, and that a project with a single release does not have much of it yet. We are
committed to publishing every release from the public repository, to reviewing all external
contributions through pull requests, and to keeping the code signing policy up to date.
We will gladly provide anything else you may need: build logs, checksums, extra
documentation, or a walkthrough of the build.
```

> ❓ **La question sur `chrome_elf.dll` est déjà incluse à la fin du texte de
> *Description*** ci-dessus. Si vous préférez l'envoyer séparément, écrivez à
> `support@signpath.io` : le *bootstrap* de CEF exige que `faraday.exe`, `chrome_elf.dll`
> et `faraday.dll` soient signés par **le même certificat** ; `chrome_elf.dll` étant un
> fichier **amont** de CEF, il faut leur accord pour le signer avec notre paquet (sinon on
> se limite à l'installateur).

### Après acceptation

- SignPath.io fournira un **jeton d'API** et un **identifiant d'organisation** : à
  enregistrer comme *secrets* GitHub du dépôt (`SIGNPATH_API_TOKEN`,
  `SIGNPATH_ORGANIZATION_ID`) — **jamais** dans un fichier ni dans une conversation.
- Il faudra créer sur leur portail : un **projet** (`faraday`), un **certificat**, un
  **signing policy** (`release-signing`) et une **artifact configuration** (fichiers à
  signer + contraintes de métadonnées).
- L'étape de signature, déjà écrite en commentaire dans `.github/workflows/build.yml`,
  pourra alors être activée.

---

## 10. Plan B — après un refus de SignPath (relevé le 16/09/2026)

SignPath Foundation réserve son certificat aux projets dont la **réputation est déjà
établie** et refuse de signer des binaires **amont** (ce qui, chez nous, bloque
`chrome_elf.dll`). Un refus n'est pas un échec : il existe des alternatives payantes mais
**peu coûteuses**, qui nous donnent en plus le droit de signer **tout** le lot — y compris
`chrome_elf.dll` — donc de produire une application entièrement signée.

| Option | Prix (1re année) | Matériel | Signer tout le lot (dont `chrome_elf.dll`) | Remarques |
|---|---|---|---|---|
| **Certum — Open Source Code Signing *in the Cloud*** | **58 $ / 49 €** | **aucun** (SimplySign, carte virtuelle) | ✅ | **Le moins cher.** Réservé aux projets open source (dépôt public exigé). Vérification d'identité. |
| Certum — Open Source *code* | 29 $ | il faut **déjà posséder** une carte cryptoCertum + lecteur | ✅ | Le prix le plus bas, mais le matériel coûte ~60 $. |
| Certum — Open Source *set* | 89 $ | carte + lecteur **inclus** | ✅ | Intéresse si l'on veut du matériel physique. |
| **Azure Artifact Signing** (ex-Trusted Signing) | **9,99 $/mois ≈ 120 $/an** | aucun (HSM Microsoft) | ✅ | **Validation d'identité « Individual » acceptée** (pièce d'identité + selfie). Abonnement Azure payant obligatoire. Certificats de 3 jours renouvelés automatiquement, horodatage Microsoft. Intégration GitHub Actions officielle. |
| Certum — Standard OV *in the Cloud* | 249 $ | aucun | ✅ | Si le statut « open source » ne convient pas. |
| Certum — EV *in the Cloud* | 459 $ | aucun | ✅ | Réputation SmartScreen quasi immédiate. |
| SignPath Foundation | 0 $ | aucun | ❌ | Soumise à réputation ; refusée le 13/09/2026. Réessayable plus tard. |

### Recommandation

> ✅ **Choix retenu : Certum *Open Source Code Signing in the Cloud*. La procédure complète,
> pas-à-pas (achat → activation → signature → publication) est dans
> [`CERTUM.md`](CERTUM.md).**

1. **Certum Open Source Code Signing in the Cloud — 58 $ / 49 €** :
   <https://certum.store/open-source-code-signing-on-simplysign.html> (ou la boutique € :
   <https://shop.certum.eu/open-source-code-signing-on-simplysign.html>).
   Aucun matériel à recevoir, et **notre script gère déjà ce mode** :
   ```powershell
   .\packaging\build-release.ps1 -Sandbox -Inno -SignAppFiles `
       -CertThumbprint <EMPREINTE> -Csp "Certum SimplySign"
   ```
2. **Azure Artifact Signing** (9,99 $/mois) si l'on préfère le cloud Microsoft, la rotation
   automatique des certificats et l'action GitHub officielle — éligible en **particulier**.
   À noter : le CN du certificat est le **nom légal validé** (pas de personnalisation), et
   la signature passe par `signtool /dlib …/Azure.CodeSigning.Dlib.dll /dmdf metadata.json`
   (il faudra ajouter ce mode au script).

### À faire une fois le certificat obtenu

- Signer **le lot applicatif** (`-SignAppFiles`) : le bootstrap CEF exige `faraday.exe`,
  `chrome_elf.dll` et `faraday.dll` signés par **le même certificat** (aucun souci avec un
  certificat d'AC, approuvé partout).
- Envisager de signer aussi **le reste des DLL CEF** (`libcef.dll`, `libEGL.dll`,
  `vk_swiftshader.dll`…) pour les machines où **Smart App Control** est actif.
- Republier une version signée (nouveau tag, p. ex. `v0.1.1`) et mettre à jour
  `docs/SIGNATURE.md` avec l'empreinte et le fournisseur retenus.

---

## 11. Récapitulatif des fichiers fournis pour la candidature

| Fichier | Rôle |
|---|---|
| `LICENSE-MIT`, `LICENSE-APACHE` | licence OSI (exigée) |
| `README.md` | description, téléchargement, **politique de signature**, lien confidentialité |
| `docs/CONFIDENTIALITE.md` | politique de confidentialité (exigée) |
| `.github/workflows/build.yml` | build vérifiable en CI + release automatique sur tag |
| `packaging/release-notes.md` | notes de version publiées |
