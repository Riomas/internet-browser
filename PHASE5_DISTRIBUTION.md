# 📦 Faraday — Phase 5 · Distribution & packaging

> Statut : **EN COURS** — Phase 5 (Distribution, sem. 15–17)
> Objectif : installer/portable Windows 10/11 x64, signature, auto-update, docs.
> Décisions : **portable + installateur** · **x64** · **non signé pour l'instant** (hooks SignTool prêts).

---

## 1. Ce qui est livré (dossier `packaging/`)

| Fichier | Rôle |
|---|---|
| `build-release.ps1` | Build `--release`, regroupe l'app dans `dist\Faraday\`, crée le **zip portable**, signature optionnelle, lance Inno si dispo (`-Inno`). |
| `faraday.iss` | Script **Inno Setup 6** → installateur `dist\Faraday-Setup-0.1.0-x64.exe` (installation **par utilisateur**, sans admin/UAC). |
| `make-icon.ps1` | Génère `crates\faraday\resources\icons\faraday.ico` (bouclier privacy, 16/32/48/256 px, format DIB compatible partout). |
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

> ⚠️ **État actuel : NON PRÊT.** La feature `sandbox` du code est un **placeholder
> de la Phase 0** : `main.rs` contient une seconde `fn main` (gated
> `#[cfg(all(feature = "sandbox", windows))]`) qui **refuse de démarrer**
> (`bail!("Le mode sandbox Windows nécessite bootstrap.exe…")`). Un build
> `--features sandbox` produit donc un exécutable **non fonctionnel**.
> → **Ne pas distribuer** de build `-Sandbox` en l'état.

La distribution CEF embarque `bootstrap.exe` / `bootstrapc.exe` (copiés dans le
packaging), qui sont le support du sandbox Chromium sous Windows. Activer
réellement le sandbox nécessite :
1. Implémenter le lancement via `bootstrap` (d'après la mécanique CEF Windows), et
2. **valider sur une Windows 10 propre (VM)**.

**Recommandation** : rester sur le build non-sandbox (validé) pour la version
0.1.0, et traiter le sandbox comme une **tâche Phase 5+ dédiée** (sécurité,
nécessite VM + test d'intégration cef-rs).

---

## 5. Signature de code & SmartScreen

Sans signature, Windows affiche « Éditeur inconnu / Windows a protégé votre PC » au
téléchargement/lancement. Pour une diffusion publique il faudra :

1. Acheter un **certificat de signature de code** (OV/EV) auprès d'un émetteur (DigiCert,
   Sectigo…), ~150–400 €/an. L'EV lève aussi la réputation SmartScreen plus vite.
2. Signer `faraday.exe`, `faraday_helper.exe` et l'installateur (SHA-256 + horodatage).
   Le script `build-release.ps1 -CertPath …` fait le nécessaire via `signtool.exe`.

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

- [ ] Test sur **Windows 10 propre (VM)** : installation par utilisateur sans UAC.
- [ ] Au 1er lancement : `%APPDATA%\Faraday\` créé, `privacy.toml` généré (valeurs par défaut privées).
- [ ] EFF Cover Your Tracks : **Yes / Yes** par défaut.
- [ ] Aucune requête sortante vers Google/services tiers au démarrage (netstat / proxy).
- [ ] Désinstallation : supprime l'app, **option** de suppression des données.
- [ ] (Recommandé) Build `--features sandbox` validé sur la VM.
- [ ] (Avant diffusion publique) Signature de code + page de téléchargement HTTPS.

---

*Faraday — Phase 5 Distribution · document de référence.*
