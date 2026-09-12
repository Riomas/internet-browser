# 🔏 Faraday — Signature de code (guide)

> Objectif : signer les binaires Faraday pour que Windows **fasse confiance** à la
> distribution (supprime les alertes SmartScreen et débloque **Smart App Control**).

---

## 1. Comprendre : quel certificat fait « confiance » à Windows ?

| Type de certificat | Coût | Windows fait-il confiance ? | Remarques |
|---|---|---|---|
| **Auto-signé** (self-signed) | Gratuit | ❌ **Non** | Utile **uniquement** pour tester le pipeline de signature. SAC/SmartScreen bloquent toujours. |
| **OV** (Organization Validation) d'une AC | ~200–400 €/an | ✅ Oui (réputation progressive) | Peut demander un token USB/HSM pour la clé privée. |
| **EV** (Extended Validation) d'une AC | ~300–600 €/an | ✅ Oui, réputation **immédiate** | Le plus efficace contre SmartScreen/SAC. |
| **Azure Trusted Signing** (Microsoft) | ~10 $/mois | ✅ Oui, sans token | Signature « dans le cloud » ; éligibilité (validation d'identité) requise. |
| **Certum « Open Source Code Signing »** | Gratuit / très bas coût | ✅ Oui | Réservé aux projets **open source** (dépôt public requis). |

**AC possibles** : DigiCert, Sectigo, SSL.com, GlobalSign, Certum (Asseco), etc.

> ⚠️ Un certificat **auto-signé ne débloque PAS Smart App Control** : c'est parfait pour
> valider l'outillage, pas pour distribuer.

---

## 2. Ce qui doit être signé

| Fichier | Pourquoi |
|---|---|
| `faraday.exe` | Initialise le processus Chromium (`bootstrap.exe` de CEF) |
| `faraday.dll` | Code de l'application (variante sandbox) |
| `faraday_helper.exe` | Processus enfant CEF |
| `chrome_elf.dll` | Préchargé par le bootstrap : **contrôlé** pour la signature |
| `Faraday-Setup-0.1.0-x64.exe` | Installateur (évite l'alerte « Éditeur inconnu ») |

> ⚠️ **Contrainte CEF (à connaitre absolument)** : au démarrage, `bootstrap.exe` vérifie la
> signature de `faraday.exe`, puis de `chrome_elf.dll`, puis de `faraday.dll`. La règle est
> binaire :
>
> - **soit tous non signés** → démarrage normal ;
> - **soit tous signés par le même certificat, et ce certificat approuvé sur la machine
>   cible** → démarrage normal ;
> - tout autre cas → le bootstrap **refuse de démarrer** : aucune fenêtre ne s'ouvre et un
>   `debug.log` apparaît à côté de `faraday.exe` avec
>   `Failed <chemin> certificate checks: WinVerifyTrust failed`.
>
> C'est pour cela que la stratégie par défaut de Faraday est : **lot applicatif non signé**
> (démarre partout, même sur une machine vierge) **+ installateur signé**. `libcef.dll`,
> fourni par CEF, restant non signé, le lot ne peut de toute façon pas être « intégralement »
> signé par le même certificat.

> Le `.zip` portable **ne peut pas être signé** (ce n'est pas un fichier PE) → on fournit
> un **`SHA256SUMS.txt`** pour vérifier l'intégrité.

**Ordre correct** (déjà implémenté dans `packaging/build-release.ps1`) :
`cargo build` → `dist\Faraday\` → *(signature du lot si `-SignAppFiles`)* → zip portable →
installateur compilé puis **signé** → `SHA256SUMS.txt`.

---

## 3. Signer avec le script (recommandé)

```powershell
# Depuis la racine du dépôt (installateur signé, lot applicatif non signé : voir §2)
.\packaging\build-release.ps1 -Inno -CertPath "C:\chemin\mon-certificat.pfx" -CertPass "motdepasse"
```

Le script :
1. compile en Release (+ helper, + DLL en mode sandbox),
2. regroupe l'application dans `dist\Faraday\`,
3. signe le **lot applicatif** *uniquement* avec `-SignAppFiles` (`faraday.exe`,
   `faraday.dll`, `faraday_helper.exe`, **`chrome_elf.dll`** — voir la contrainte §2),
4. crée le zip portable,
5. compile l'installateur Inno et **le signe** (avec vérification `signtool verify /pa`),
6. écrit `dist\SHA256SUMS.txt` et copie le certificat public (`certs\*.cer`) à côté des
   artefacts quand il existe.

> `signtool.exe` est repéré automatiquement (PATH, puis Windows Kits).
> Si le certificat est un **token/HSM**, utilise plutôt `/sha1 <thumbprint>` (voir §5).

---

## 4. Vérifier une signature

```powershell
# L'installateur est toujours signé
signtool verify /pa /v "dist\Faraday-Setup-0.1.0-x64.exe"
Get-AuthenticodeSignature "dist\Faraday-Setup-0.1.0-x64.exe" | Format-List Status, SignerCertificate

# Le lot applicatif : "NotSigned" par défaut (voir §2), ou "Valid" avec -SignAppFiles
Get-AuthenticodeSignature "dist\Faraday\faraday.exe" | Format-List Status
```

Attendu : `Status = Valid` sur l'installateur, avec le nom de ton organisation comme signataire.

---

## 5. Cas particulier : clé privée sur token ou HSM

`signtool` ne lit alors pas de `.pfx` ; il faut désigner le certificat par son empreinte
(thumbprint) présent dans le magasin :

```powershell
# Lister les certificats de signature disponibles
Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert | Select-Object Subject, Thumbprint, NotAfter

# Signature via le magasin (token inséré)
signtool sign /sha1 <THUMBPRINT> /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 `
  "dist\Faraday-Setup-0.1.0-x64.exe"
```

*(Le script propose `-CertPath` ; pour un token, on peut aussi exporter un `.pfx` non exportable
ou adapter la commande ci-dessus.)*

---

## 6. Azure Trusted Signing (alternative sans token)

```powershell
signtool sign /v /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 `
  /dlib "C:\Program Files\Microsoft Trusted Signing Client\Azure.CodeSigning.Dlib.dll" `
  /dmdf "signing-metadata.json" `
  "dist\Faraday\faraday.exe"
```
Le fichier `signing-metadata.json` décrit le compte Trusted Signing et le profil de certificat.
*(Nécessite un compte Azure + validation d'identité.)*

---

## 7. Tester le pipeline dès maintenant (certificat auto-signé)

Pour valider toute la chaîne **sans acheter de certificat** :

```powershell
# 1) Créer un certificat de test (magasin utilisateur, aucun droit admin)
.\packaging\make-testcert.ps1 -Trust      # -Trust ajoute le cert aux magasins de confiance utilisateur

# 2) Build + installateur signe (le lot applicatif reste non signe : voir §2)
.\packaging\build-release.ps1 -Inno -CertPath .\certs\faraday-test.pfx -CertPass "faraday-test"

# 3) Nettoyer (supprimer le certificat de test de la machine)
.\packaging\make-testcert.ps1 -Remove
```

> ⚠️ Rappel : signé avec un certificat auto-signé, Faraday **restera bloqué par Smart App
> Control** sur cette machine. Le but est de prouver que la signature fonctionne
> (le pipeline) avant d'investir. La confiance réelle viendra d'un certificat d'AC (§1).

### Signer aussi le lot applicatif (cas particulier)

Depuis le passage au sandbox Chromium (CEF M138+), signer `faraday.exe` **sans** que le
certificat soit approuvé sur la machine de test **empêche le démarrage** (règle détaillée au
§2). Si vous voulez tester cette configuration :

```powershell
# 1) Le certificat doit être approuvé sur la machine de test :
#    - hôte : .\packaging\make-testcert.ps1 -Trust
#    - Windows Sandbox (machine vierge) : importer le certificat public AVANT de lancer
Import-Certificate -FilePath C:\FaradayApp\..\Faraday-0.1.0-certificat.cer `
                   -CertStoreLocation Cert:\LocalMachine\Root

# 2) Construire avec le lot applicatif signe
.\packaging\build-release.ps1 -Sandbox -Inno -SignAppFiles `
    -CertPath .\certs\faraday-test.pfx -CertPass "faraday-test"
```

> En cas de doute : laissez le **lot applicatif non signé** (option par défaut). C'est la
> configuration qui démarre partout, y compris sur une machine vierge.

---

## 8. Bonnes pratiques

- **Horodatage** systématique (`/tr http://timestamp.digicert.com /td SHA256`) : la signature
  reste valide après expiration du certificat.
- **Ne jamais committer** le `.pfx` ni son mot de passe (ajoute `certs/` au `.gitignore`).
- **Signer de façon consistante** : conserver le même certificat améliore la réputation SmartScreen.
- Signer **aussi les versions suivantes** avec le même certificat (auto-update).

---

*Faraday — guide de signature de code.*
