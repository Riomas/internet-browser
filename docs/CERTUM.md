# 🔐 Certum — du paiement à la release signée

Guide pas-à-pas pour le certificat **Open Source Code Signing *in the Cloud*** (58 $ / 49 €),
la solution la moins chère qui permet de signer **tout** le lot Faraday (y compris
`chrome_elf.dll`, imposé par le bootstrap CEF).

> ⏱️ Comptez **1 à 5 jours ouvrés** entre l'achat et le certificat utilisable (vérification
> d'identité par Certum). Tout le reste est prêt côté Faraday : rien à coder.

---

## 0. Ce que l'on achète

| | |
|---|---|
| Produit | **Open Source Code Signing in the Cloud** (carte virtuelle SimplySign, **aucun matériel**) |
| Boutique EUR | <https://shop.certum.eu/open-source-code-signing-on-simplysign.html> — **49 €** |
| Boutique USD | <https://certum.store/open-source-code-signing-on-simplysign.html> — **58 $** |
| Durée | 1 an, renouvelable (même prix) |
| Condition | projet **open source** : gardez le dépôt public sous la main, Certum peut le vérifier |
| Titulaire | **votre nom légal** (le certificat est nominatif, pas au nom du projet) |

À éviter : la variante à 29 $ n'est valable que si vous possédez **déjà** une carte
cryptoCertum + un lecteur ; le pack à 89 $ en inclut, mais revient plus cher que le cloud.

## 1. Achat et vérification d'identité

1. Créer un compte sur la boutique et commander le produit.
2. Certum envoie une demande de **vérification** : fournir une pièce d'identité
   (liste officielle : <https://support.certum.eu/en/code-signing-required-documents/>).
   Une **visioconférence** ou un selfie peut être demandé.
3. Le **code d'activation** (e-code) apparaît dans le **Certum Panel** :
   <https://panel.certum.pl/> → *Data security products*.

## 2. Activer le certificat sur la carte cloud

1. Certum Panel → produit → **Activer le certificat**.
2. Installer **SimplySign Desktop** (Windows) — lien de téléchargement sur la page du produit.
3. Installer l'application mobile **SimplySign** (Android / iOS) : elle génère le
   **jeton TOTP** qui protège la carte.
4. Dans SimplySign Desktop : se connecter avec l'e-mail du compte + le jeton TOTP.
   ➜ Le certificat est alors inscrit dans le magasin Windows `Cert:\CurrentUser\My`.

## 3. Vérifier que le certificat est disponible

```powershell
Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert |
    Select-Object Subject, Thumbprint, NotAfter
```

Notez l'**empreinte** (`Thumbprint`) : c'est elle que l'on passe au script de build.

## 4. Signer la distribution

> ⚠️ **SimplySign Desktop doit être connecté** (session ouverte depuis la zone de
> notification) — sinon `signtool` ne trouve pas le certificat. La connexion est
> volontairement manuelle (TOTP) : la signature ne peut donc pas tourner en CI.

```powershell
# Build complet, installateur compris, et signature de TOUT le lot applicatif :
# faraday.exe, faraday.dll, faraday_helper.exe et chrome_elf.dll
.\packaging\build-release.ps1 -Sandbox -Inno -SignAppFiles `
    -CertThumbprint <EMPREINTE> `
    -TimestampUrl "http://time.certum.pl"
```

- Si `signtool` répond « *No certificates were found* », ajoutez le fournisseur :
  `-Csp "Certum SimplySign"` (et, si Certum l'indique, `-KeyContainer "<conteneur>"`).
- L'horodatage `http://time.certum.pl` est le service de Certum ; celui de DigiCert
  (`http://timestamp.digicert.com`, valeur par défaut) fonctionne aussi.
- `-SignAppFiles` signe **aussi `chrome_elf.dll`** : indispensable, le bootstrap CEF exige
  que `faraday.exe`, `chrome_elf.dll` et `faraday.dll` partagent le **même** certificat.
- Sans `-SignAppFiles`, seul l'installateur est signé (mode par défaut des certificats
  **non** approuvés).

## 5. Vérifier les signatures

```powershell
Get-AuthenticodeSignature .\dist\Faraday\faraday.exe          | Format-List Status, SignerCertificate
Get-AuthenticodeSignature .\dist\Faraday\chrome_elf.dll       | Format-List Status
Get-AuthenticodeSignature .\dist\Faraday\faraday.dll          | Format-List Status
Get-AuthenticodeSignature .\dist\Faraday-Setup-0.1.0-x64.exe  | Format-List Status
```

Attendu : **`Valid`** partout, avec **votre nom** comme signataire, et un horodatage
présent (`TimeStamperCertificate`).

## 6. Vérifier que l'application démarre toujours

Le bootstrap CEF refuse de démarrer si les trois fichiers ne sont pas signés par le même
certificat. Comme on vient de le faire avec un **certificat d'autorité reconnu** (approuvé
par Windows partout, contrairement à notre certificat de test auto-signé), le démarrage est
nominal — on peut même tester dans un **Windows Sandbox vierge** :

```powershell
Start-Process .\dist\Faraday-Test-Sandbox.wsb
```

Puis, dans la session : double-clic sur `faraday.exe` → la fenêtre doit s'ouvrir.

## 7. Publier la version signée

Les binaires signés ne peuvent pas être produits par la CI (la clé reste dans le coffre
SimplySign) : **on signe localement, on publie ensuite**.

1. Incrémenter la version : `version = "0.1.1"` dans `Cargo.toml`.
2. Signer comme au §4, puis vérifier `dist\SHA256SUMS.txt` (il est régénéré **après**
   signature, donc les empreintes correspondent aux fichiers signés).
3. Publier la release — deux options :
   - **Automatique** : `git tag -a v0.1.1 -m "Faraday v0.1.1 (signée)" && git push origin v0.1.1`
     → la CI crée la release, puis **remplacer** les fichiers par les versions signées :
     ```powershell
     gh release upload v0.1.1 .\dist\Faraday-0.1.1-x64-portable.zip `
         .\dist\Faraday-Setup-0.1.1-x64.exe .\dist\SHA256SUMS.txt --clobber
     ```
   - **Manuelle** : page *Releases* → supprimer les fichiers non signés → glisser-déposer
     les fichiers signés de `dist\`.
4. Vérifier depuis un autre poste : télécharger le zip, `Get-AuthenticodeSignature` →
   `Valid`, et **aucun avertissement « Éditeur inconnu »**.

## 8. Bonnes pratiques / pièges

- **Ne jamais** partager le code d'activation ni le jeton TOTP ; ils donnent accès à la clé.
- La réputation **SmartScreen** se construit avec les téléchargements du **même** binaire :
  gardez le même certificat et des versions régulières (pas de re-signature inutile).
- `chrome_elf.dll` et `libcef.dll` sont des fichiers **amont** (CEF, licence BSD) : les
  re-signer est autorisé par la licence et nécessaire ici ; c'est documenté dans la
  politique de signature du projet.
- Sur une machine où **Smart App Control** est actif, pensez à signer **tout le jeu de DLL
  CEF** (`libcef.dll`, `libEGL.dll`, `libGLESv2.dll`, `vk_swiftshader.dll`, `vulkan-1.dll`,
  `dxcompiler.dll`, `dxil.dll`, `d3dcompiler_47.dll`) — demandez-moi de l'ajouter au script
  le jour venu.
- Renouvellement annuel : relancer l'activation dans le panel, l'empreinte du certificat
  change (le script prend l'empreinte en paramètre, rien à modifier).

---

*Voir aussi : `CERTIFICAT_OPENSOURCE.md` (comparatif des options) et `SIGNATURE.md`
(mécanique générale de signature).*
