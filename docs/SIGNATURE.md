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
| `faraday.exe` | Exécutable principal |
| `faraday_helper.exe` | Processus enfant CEF (soumis aux mêmes politiques) |
| `Faraday-Setup-0.1.0-x64.exe` | Installateur (évite l'alerte « Éditeur inconnu ») |

> Le `.zip` portable **ne peut pas être signé** (ce n'est pas un fichier PE) → on fournit
> un **`SHA256SUMS.txt`** pour vérifier l'intégrité.

**Ordre correct** (déjà implémenté dans `packaging/build-release.ps1`) :
`faraday.exe` + `faraday_helper.exe` **(signés) → zip + installateur → installateur signé`.

---

## 3. Signer avec le script (recommandé)

```powershell
# Depuis la racine du dépôt
.\packaging\build-release.ps1 -Inno -CertPath "C:\chemin\mon-certificat.pfx" -CertPass "motdepasse"
```

Le script :
1. compile en Release (+ helper),
2. regroupe l'application dans `dist\Faraday\`,
3. **signe** `faraday.exe` et `faraday_helper.exe` (SHA-256 + horodatage DigiCert),
4. **vérifie** chaque signature (`signtool verify /pa`),
5. crée le zip portable,
6. compile l'installateur Inno et **le signe**,
7. écrit `dist\SHA256SUMS.txt`.

> `signtool.exe` est repéré automatiquement (PATH, puis Windows Kits).
> Si le certificat est un **token/HSM**, utilise plutôt `/sha1 <thumbprint>` (voir §5).

---

## 4. Vérifier une signature

```powershell
signtool verify /pa /v "dist\Faraday\faraday.exe"
Get-AuthenticodeSignature "dist\Faraday\faraday.exe" | Format-List Status, SignerCertificate
```

Attendu : `Status = Valid`, avec le nom de ton organisation comme signataire.

---

## 5. Cas particulier : clé privée sur token ou HSM

`signtool` ne lit alors pas de `.pfx` ; il faut désigner le certificat par son empreinte
(thumbprint) présent dans le magasin :

```powershell
# Lister les certificats de signature disponibles
Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert | Select-Object Subject, Thumbprint, NotAfter

# Signature via le magasin (token inséré)
signtool sign /sha1 <THUMBPRINT> /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 `
  "dist\Faraday\faraday.exe"
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

# 2) Signer avec ce certificat de test
.\packaging\build-release.ps1 -Inno -CertPath .\certs\faraday-test.pfx -CertPass "faraday-test"

# 3) Nettoyer (supprimer le certificat de test de la machine)
.\packaging\make-testcert.ps1 -Remove
```

> ⚠️ Rappel : signé avec un certificat auto-signé, Faraday **restera bloqué par Smart App
> Control** sur cette machine. Le but est de prouver que la signature fonctionne
> (le pipeline) avant d'investir. La confiance réelle viendra d'un certificat d'AC (§1).

---

## 8. Bonnes pratiques

- **Horodatage** systématique (`/tr http://timestamp.digicert.com /td SHA256`) : la signature
  reste valide après expiration du certificat.
- **Ne jamais committer** le `.pfx` ni son mot de passe (ajoute `certs/` au `.gitignore`).
- **Signer de façon consistante** : conserver le même certificat améliore la réputation SmartScreen.
- Signer **aussi les versions suivantes** avec le même certificat (auto-update).

---

*Faraday — guide de signature de code.*
