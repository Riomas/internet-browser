# 🔐 Faraday — Distribution avec un certificat auto-signé

> **En résumé** : Faraday peut être distribué avec un **certificat auto-signé** que
> l'installateur propose d'approuver dans le magasin de **l'utilisateur courant**
> (aucun droit administrateur). C'est un mode **fonctionnel et vérifié**, mais qui repose
> sur la confiance explicite de l'utilisateur : il remplace, en attendant, un certificat
> d'autorité de certification (AC) payant.

---

## 1. Pourquoi un certificat est nécessaire

Depuis CEF M138 (Chromium 152), `faraday.exe` est le `bootstrap.exe` de CEF. Au démarrage,
le bootstrap vérifie la signature Authenticode de trois fichiers :

| Fichier | Contrôlé par le bootstrap |
|---|---|
| `faraday.exe` | ✅ |
| `chrome_elf.dll` | ✅ |
| `faraday.dll` | ✅ |

La règle est **binaire** :

- **tous non signés** → démarrage normal ;
- **tous signés par le même certificat, et ce certificat approuvé sur la machine
  cible** → démarrage normal ;
- **tout autre cas** → refus de démarrer : aucune fenêtre, et un fichier `debug.log`
  apparaît à côté de `faraday.exe` :
  ```
  FATAL:...bootstrap_win.cc Failed C:\...\faraday.exe certificate checks:
  Certificate 0: WinVerifyTrust failed (-2146762487)
  ```
  `-2146762487` = `CERT_E_UNTRUSTEDROOT` (racine non approuvée par le fournisseur
  d'approbation).

D'où le choix de Faraday : **signer le lot applicatif** (`faraday.exe`, `faraday_helper.exe`,
`chrome_elf.dll`, `faraday.dll`) **et fournir le certificat** pour que la machine cible
l'approuve. `libcef.dll` reste non signé (binaire amont CEF) : voir la limite §6.

---

## 2. Comment ça marche (parcours utilisateur)

1. L'utilisateur lance `Faraday-Setup-0.1.1-x64.exe`.
2. À l'écran **« Certificat de signature Faraday »**, une case est cochée par défaut :

   > *« Approuver le certificat de signature Faraday pour mon compte utilisateur »*
   >
   > Installe le certificat public dans le magasin « Autorités de certification racines de
   > confiance » de **votre compte uniquement** (aucun droit administrateur). Il pourra être
   > retiré à la désinstallation.

3. Si la case reste cochée, l'installateur exécute l'utilitaire
   `faraday-certificat.ps1 -Install` **après** la copie des fichiers.
4. Windows affiche sa boîte de dialogue de sécurité habituelle (« Voulez-vous installer ce
   certificat ? ») ; si elle est refusée, un message d'aide s'affiche et l'installation se
   poursuit (l'application sera alors dans la situation du §1, troisième cas).
5. À la **désinstallation**, l'utilitaire est rappelé avec `-Remove` : le certificat est
   retiré du magasin de l'utilisateur.

> Décocher la case laisse la machine intacte : rien n'est ajouté aux magasins de certificats.

### Installation silencieuse (`/VERYSILENT`, `/SILENT`)

La page de choix n'existe pas dans ce mode : l'installateur applique **le défaut**, c'est-à-dire
l'approbation du certificat (comme la case cochée). Sans cela, une installation non surveillée
laisserait une application incapable de démarrer.

```powershell
# Déploiement silencieux : certificat approuvé (défaut)
Faraday-Setup-0.1.1-x64.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART

# Déploiement silencieux SANS approuver le certificat
Faraday-Setup-0.1.1-x64.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /NOCERT=1
```

> Avec `/NOCERT=1`, l'application est installée mais **ne démarrera pas** avant approbation
> manuelle : `powershell -ExecutionPolicy Bypass -File "%LOCALAPPDATA%\Faraday\faraday-certificat.ps1"`.
> C'est le comportement recommandé lorsque l'administrateur veut contrôler la confiance.

### Ce que fait exactement l'utilitaire

```powershell
# packaging/installer/faraday-certificat.ps1 (extrait)
$cert  = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2($cerPath)
$store = New-Object System.Security.Cryptography.X509Certificates.X509Store('Root', 'CurrentUser')
$store.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
$store.Add($cert)
```

- Magasin `Root` / **`CurrentUser`** → aucun `UAC`, aucun droit administrateur,
  aucun impact sur les autres comptes Windows.
- Le certificat **privé** (`.pfx`) n'est **jamais** distribué : seul le certificat
  **public** (`.cer`) voyage dans le lot.

### Paramètres de l'utilitaire

| Appel | Effet | Code de sortie |
|---|---|---|
| `-Install` (défaut) | Installe/approuve le certificat | `0` approuvé, `1` refus/échec, `2` fichier `.cer` absent ou illisible |
| `-Status` | Affiche l'état (sujet + empreinte) | `0` approuvé, `1` non approuvé, `2` `.cer` manquant |
| `-Remove` | Retire le certificat du magasin utilisateur | `0` retiré |

Exemples :

```powershell
# Version portable : approuver manuellement (une seule fois)
powershell -ExecutionPolicy Bypass -File .\faraday-certificat.ps1 -Install

# Vérifier
powershell -ExecutionPolicy Bypass -File .\faraday-certificat.ps1 -Status

# Retirer
powershell -ExecutionPolicy Bypass -File .\faraday-certificat.ps1 -Remove
```

---

## 3. Construire un lot conforme

Le certificat de distribution est créé (une seule fois) par
`packaging/make-signing-cert.ps1` :

| Champ | Valeur |
|---|---|
| Sujet | `CN=Faraday, O=Faraday Project` |
| Nature | auto-signé, RSA 4096 / SHA-256, EKU *Code Signing* |
| Validité | 10 ans (16/09/2026 → 16/09/2036) |
| Empreinte (SHA-1) | `EFB4E86D4523514A5672CF4879ED88D96E395EBE` |
| Fichiers | `certs\faraday.pfx` (clé privée, **SECRÈTE**), `certs\faraday.cer` (clé publique) |

```powershell
# 1) Créer le certificat + l'approuver sur cette machine (magasin utilisateur, aucun droit admin)
.\packaging\make-signing-cert.ps1 -Trust

# 2) Build + installateur, avec LE LOT APPLICATIF SIGNÉ et le certificat public joint
.\packaging\build-release.ps1 -Sandbox -Inno -SignAppFiles `
    -CertPath .\certs\faraday.pfx -CertPass "<mot de passe>"
```

> 🔑 Le mot de passe du `.pfx` est écrit une fois dans `certs\faraday-password.txt`
> (ignoré par git) : à recopier dans un gestionnaire de mots de passe, puis à supprimer.
> Le `.pfx` **ne doit jamais être publié** (`make-signing-cert.ps1 -Remove` nettoie tout).
> Pour un build scripté, lire le mot de passe depuis ce fichier plutôt que de l'écrire en
> clair dans une commande :
> `$p = (Get-Content .\certs\faraday-password.txt -Raw).Trim()`

Le script :

1. compile en Release (application + helper + DLL) ;
2. **signe le lot applicatif** : `faraday.exe`, `faraday_helper.exe`, `chrome_elf.dll`,
   `faraday.dll` — la signature est **vérifiée** après coup (`signtool verify /pa`) ;
3. **joint le certificat public** au lot : `dist\Faraday\faraday-certificat.cer` et
   `dist\Faraday-<version>-certificat.cer`, ainsi que l'utilitaire
   `faraday-certificat.ps1` ;
4. compile l'installateur Inno (qui embarque ces fichiers et la page de consentement) puis
   **le signe** ;
5. écrit `dist\SHA256SUMS.txt`.

> ⚠️ `-SignAppFiles` n'est utilisable **que** si le certificat est approuvé sur la machine
> cible (c'est justement le rôle de la page de l'installateur). Sans approbation, mieux vaut
> **ne pas** signer le lot (comportement par défaut du script) : il démarrera partout.

---

## 4. Vérification effectuée (Windows Sandbox, machine vierge)

Le parcours a été validé de bout en bout dans un Windows Sandbox **sans aucun certificat
approuvé au départ**, avec les fichiers de `dist\Faraday` (lot signé) :

| Phase | Condition | Résultat mesuré |
|---|---|---|
| **A** — lancement **sans** approbation | `faraday.exe` signé, racine inconnue | ❌ ne démarre pas : sortie immédiate (code `-2147483645`), `debug.log` avec `WinVerifyTrust failed (-2146762487)`, 0 processus |
| **B** — approbation puis lancement | `faraday-certificat.ps1 -Install` puis `faraday.exe` | ✅ démarre : `faraday 0.1.0 - demarrage (sandbox=active)`, **4 processus CEF vivants**, aucun `debug.log` |

Ce parcours a été rejoué avec le **certificat de distribution**
(`CN=Faraday, O=Faraday Project`, empreinte `EFB4E86D4523514A5672CF4879ED88D96E395EBE`) :
mêmes résultats (phase A refusée, phase B démarrée, 4 processus).

Le test est reproductible : `tools\diag\launch-cert.wsb` (double-clic).

### Parcours complet de l'installateur

Le même parcours a été rejoué **via l'installateur**, sur un Windows Sandbox vierge
(`tools\diag\install-e2e.wsb`) :

| Phase | Scénario | Résultat mesuré |
|---|---|---|
| **A** | `Faraday-Setup-0.1.1-x64.exe /VERYSILENT` | ✅ installé dans `%LOCALAPPDATA%\Faraday`, certificat **approuvé par l'installateur**, application démarrée (**4 processus CEF**) |
| **B** | désinstallation silencieuse (`unins000.exe /VERYSILENT`) | ✅ certificat **retiré** du magasin de l'utilisateur |
| **C** | installation avec `/NOCERT=1` | ✅ installé, certificat non approuvé → l'application **refuse de démarrer** (comportement voulu) |

> 💡 **Attente possible hors connexion** : l'ajout d'une racine déclenche la mise à jour
> automatique des certificats racine par Windows. Sur une machine **sans réseau** (cas du
> Sandbox de test, `Networking Disable`), l'approbation peut prendre **jusqu'à ~1 minute**
> avant d'afficher son message. C'est normal ; avec une connexion, l'opération est immédiate.

---

## 5. Sécurité : ce que l'utilisateur accepte

Ajouter une racine au magasin de confiance est une opération **sensible** — il faut le dire
clairement :

- **Confiance accordée** : toute application signée par ce certificat pourra être vue comme
  « de confiance » par Windows sur ce compte. C'est le principe même d'une racine.
- **Portée limitée** : compte utilisateur courant uniquement ; supprimable à tout moment
  (`-Remove`, ou `certmgr.msc` → *Autorités de certification racines de confiance* →
  *Certificats* du compte utilisateur).
- **Réversible** : la désinstallation de Faraday retire automatiquement le certificat.
- **Retirable manuellement** : par l'utilisateur, à tout moment, sans impact sur
  l'installation de Faraday (l'application cessera simplement de démarrer, voir §6).
- **Pourquoi c'est acceptable ici** : l'utilisateur installe un logiciel qu'il a choisi, à
  partir d'un dépôt public vérifiable, et le certificat reste confiné à son compte. C'est le
  compromis retenu **en attendant** un certificat d'AC (voir [`CERTUM.md`](CERTUM.md)).

> Le certificat privé n'est **jamais** publié ; le dépôt public ne contient que le `.cer`.

---

## 6. Limites à connaître

1. **Smart App Control (SAC)** n'est **pas** débloqué : SAC s'appuie sur la réputation
   Microsoft associée au signataire, qu'un certificat auto-signé n'a pas. `libcef.dll`
   (binaire amont CEF, non signé) reste de toute façon concerné. Sur une machine avec SAC
   actif, Faraday doit toujours être lancé depuis un dossier exclu / une machine où SAC est
   désactivé.
2. **SmartScreen** (« Éditeur inconnu ») peut continuer d'avertir : la réputation se construit
   avec un certificat d'AC et un historique de téléchargements.
3. **Chaque machine** doit approuver le certificat (une fois). Sans cette étape, le
   navigateur ne démarre pas (§1) — c'est le prix d'une application signée sans AC.
4. **Le certificat expire** : à son renouvellement, une nouvelle approbation est nécessaire
   (le lot signé par l'ancien certificat continue de fonctionner, grâce à l'horodatage).
5. **Ne pas réutiliser** ce certificat pour d'autres logiciels : un certificat par usage.

### Résolution de problème

| Symptôme | Cause | Solution |
|---|---|---|
| Aucune fenêtre au lancement, `debug.log` à côté de `faraday.exe` | Certificat non approuvé | `powershell -ExecutionPolicy Bypass -File faraday-certificat.ps1 -Install` |
| L'installateur demande le certificat mais l'utilisateur refuse | UAC/consentement refusé | Relancer l'installateur, ou approuver manuellement (commande ci-dessus) |
| Le navigateur démarre puis SAC bloque d'autres DLL | Smart App Control actif | Voir [`GUIDE_UTILISATEUR.md`](GUIDE_UTILISATEUR.md) §6 (machine/sandbox) |
| « Le fichier certificat est introuvable » | Lot incomplet | Les fichiers `.cer` doivent être à côté de `faraday.exe` |

---

## 7. Et ensuite ?

Ce mode est **provisoire par conception** : il rend Faraday utilisable et signé partout dès
maintenant, sans dépendre d'un tiers. Le parcours vers une vraie AC reste documenté :

| Piste | Prix | État |
|---|---|---|
| Certum « Open Source Code Signing in the Cloud » | 58 $ / an | ⏸️ **En pause** (décision utilisateur) — procédure prête : [`CERTUM.md`](CERTUM.md) |
| Azure Artifact Signing (ex-Trusted Signing) | ~10 $/mois | ⏸️ En pause — comparatif : [`CERTIFICAT_OPENSOURCE.md`](CERTIFICAT_OPENSOURCE.md) §10 |
| SignPath Foundation | gratuit | ❌ Refusé (réputation insuffisante) — réessayable |
| Auto-signé + approbation par l'installateur | 0 € | ✅ **Mode actuel, vérifié** (§4) |

---

*Faraday — distribution avec certificat auto-signé.*
