# 🧪 Faraday — Test sur machine propre (VM)

> **But** : valider la distribution 0.1.0 sur une machine **sans** Smart App Control
> (qui bloque les binaires non signés) — c'est le point de contrôle final de la Phase 5.
>
> ⚠️ Sur ta machine hôte (Windows 11 Pro), **Smart App Control est activé**
> (`VerifiedAndReputablePolicyState = 1`) : il bloque `faraday.exe` et `libcef.dll`
> car ils ne sont **pas signés**. Ne désactive pas SAC sur ta machine de travail
> (c'est irréversible sans réinstaller Windows) → **utilise une VM**.

---

## 1. Ce qu'il faut copier dans la VM

Depuis `C:\Users\mario\Dev\internet-browser\dist\` :

| Fichier | Rôle |
|---|---|
| `Faraday-Setup-0.1.0-x64.exe` (131,8 Mo) | Installateur (à tester) |
| `Faraday-0.1.0-x64-portable.zip` (181,6 Mo) | Version portable (à tester) |
| `..\docs\GUIDE_UTILISATEUR.md` | Guide utilisateur (facultatif) |

Astuce : copie ces 2 fichiers via un **dossier partagé** VM ↔ hôte, une clé USB, ou
un partage réseau. (Le dossier `dist\Faraday\` est inutile à copier : c'est l'aperçu
décompressé.)

---

## 2. Installer un environnement de test — 3 options

### Option A — Hyper-V (recommandé, fidèle au plan « Windows 10 propre »)

*Windows 11 Pro inclut Hyper-V ; nécessite un passage en admin + redémarrage.*

1. **Activer Hyper-V** (PowerShell **en administrateur**) :
   ```powershell
   Enable-WindowsOptionalFeature -Online -FeatureName Microsoft-Hyper-V-All -All
   # puis redémarrer
   ```
   *(Alternative graphique : Panneau de configuration → Fonctionnalités Windows →
   cocher « Plateforme d'hyperviseur » / « Hyper-V ».)*

2. **Télécharger un ISO Windows** :
   - Windows 11 : <https://www.microsoft.com/software-download/windows11>
   - Windows 10 : <https://www.microsoft.com/software-download/windows10>
   - (Option dev : **Windows 11 development environment (VM préconfigurée)** :
     <https://developer.microsoft.com/windows/downloads/virtual-machines/>)

3. **Créer la VM** : Hyper-V Manager → *Nouvelle machine virtuelle* → **Génération 2**,
   mémoire 4 Go (ou plus), **vérifier que « Trusted Platform Module » est activé**
   (obligatoire pour Win11), disque 60 Go, réseau = *Default Switch*.

4. **Installer Windows** dans la VM (méthode recommandée : **sans compte Microsoft**,
   pour un environnement de test isolé).

> 💡 Cible du plan : **Windows 10 (1809+)**. ⚠️ **Préfère un ISO Windows 10** : Windows 11
> active **Smart App Control par défaut** sur les installations neuves, ce qui bloquerait
> de nouveau Faraday (non signé). Windows 10 n'a pas SAC → environnement de test idéal.
> (Si tu utilises un ISO Windows 11, **désactive SAC juste après l'installation** :
> Sécurité Windows → Contrôle des applications et du navigateur → Smart App Control.)

### Option B — VirtualBox (gratuit, plus simple si pas d'admin prolongé)

1. Installer **VirtualBox** (<https://www.virtualbox.org/>) — admin requis une fois.
2. Créer une VM (Windows 10/11, 4 Go RAM, 60 Go disque).
3. Installer Windows depuis l'ISO, puis copier les 2 fichiers via *Dossiers partagés*.

### Option C — Windows Sandbox (le plus rapide, intégré à Win11 Pro)

*Démarrage en ~30 s, jetable, **sans ISO ni installation** de Windows.*

1. **Activer la fonctionnalité** — PowerShell **EN ADMINISTRATEUR** :
   ```powershell
   Enable-WindowsOptionalFeature -Online -FeatureName "Containers-DisposableClientVM" -All
   # puis REDÉMARRER Windows
   ```
   *(Vérifier l'état : `(Get-CimInstance Win32_OptionalFeature -Filter "Name='Containers-DisposableClientVM'").InstallState` → doit valoir **1**.)*

2. **Lancer le test** : double-clique le fichier préparé
   `C:\Users\mario\Dev\internet-browser\dist\Faraday-Test.wsb`.
   Il ouvre le Sandbox avec le dossier `dist\` monté **en lecture seule** dans
   `C:\FaradayDist` (aucune écriture sur ta machine hôte).

3. **Dans le Sandbox** :
   - Une fenêtre Explorateur s'ouvre sur `C:\FaradayDist`.
   - **Copie** `Faraday-0.1.0-x64-portable.zip` sur le **Bureau du Sandbox**,
     puis **extrais-le** (clic droit → Extraire tout) et lance `faraday.exe`.
   - (Teste aussi `Faraday-Setup-0.1.0-x64.exe` : installation sans UAC.)

4. ⚠️ **Si Faraday est bloqué aussi dans le Sandbox** : la politique de contrôle
   d'application (Smart App Control / WDAC) est héritée de l'hôte. Dans ce cas, passe
   à l'**option A ou B** avec un **ISO Windows 10** (environnement totalement neuf,
   sans SAC).

5. Le Sandbox est **jetable** : fermer la fenêtre efface tout ce qui s'y trouvait.

---

## 3. Checklist de validation (dans la VM)

### 3.1 Version portable
- [ ] Décompresser le zip, double-cliquer `faraday.exe` → **la fenêtre s'ouvre**.
- [ ] 1er lancement : le dossier `%APPDATA%\Faraday\` est **créé**
      (avec `privacy.toml` et `session.json`).
- [ ] Onglet vide = **page d'accueil Faraday** (raccourcis + barre de recherche).
- [ ] Naviguer vers un site (ex. `fr.wikipedia.org`) → la page s'affiche.
- [ ] Afficher les **Paramètres ⚙** → tous les réglages confidentialité **activés**.

### 3.2 Confidentialité (le cœur du produit)
- [ ] Aller sur <https://coveryourtracks.eff.org> → *TEST MY BROWSER*.
      Résultat attendu : **« Blocking tracking ads? → Yes »** et
      **« Blocking invisible trackers? → Yes »**.
- [ ] Le **bouclier vert** en haut à droite affiche un compteur > 0 après navigation.

### 3.3 Fonctionnalités
- [ ] Multi-onglets (`+`, `Ctrl+T`), fermeture (`×`, `Ctrl+W`).
- [ ] Historique (`Ctrl+H`), effacement, clic pour re-naviguer.
- [ ] **Téléchargement** : télécharger un petit fichier → notification + fenêtre
      Téléchargements (`Ctrl+J`), puis « ouvrir le dossier ».
- [ ] **Clic droit** sur un lien → menu (copier le lien, ouvrir dans un nouvel onglet…).
- [ ] Survol d'un lien → l'URL s'affiche en **bas à gauche**.
- [ ] Fermer puis rouvrir → les **onglets sont restaurés** (session) et le **thème** conservé.

### 3.4 Installateur
- [ ] Lancer `Faraday-Setup-0.1.0-x64.exe` → assistant en français, **sans UAC**.
- [ ] Faraday apparaît dans le **menu Démarrer** (+ bureau si coché).
- [ ] L'app installée **démarre**.
- [ ] Désinstaller : l'app part, les **données `%APPDATA%\Faraday` restent** (sauf si
      la case « Supprimer aussi les données de navigation » est cochée).

### 3.5 Absence de dépendances de développement
- [ ] Déplacer le dossier portable sur le **Bureau** d'un autre utilisateur → il
      démarre toujours (aucune dépendance à `C:\Users\mario\Dev\...`).
- [ ] (Avancé) Chercher `Dev\internet-browser` dans les fichiers extraits → **aucune
      occurrence** ne devrait servir au runtime (les configs sont embarquées).

---

## 4. À noter / limites connues de la v0.1.0

- ✅ **Runtime C++ embarqué** *(corrigé et VÉRIFIÉ en Windows Sandbox)* : le premier essai
  échouait avec « `VCRUNTIME140.dll` est introuvable ». Le runtime Visual C++
  (`vcruntime140.dll`, `vcruntime140_1.dll`) est désormais **embarqué à côté de l'exe**
  (déploiement « app-local »). **Test de reprise réussi** : Faraday démarre dans un
  environnement propre (Windows Sandbox) sans VC++ Redistributable installé.
- ℹ️ **Windows Sandbox n'applique pas Smart App Control** : c'est donc un bon environnement
  de test « machine propre » (le blocage SAC observé sur l'hôte ne s'y reproduit pas).
- **Non signée** : sur une machine avec Smart App Control ou une politique WDAC stricte,
  le lancement sera bloqué → signature de code nécessaire pour une diffusion publique.
- **Sandbox Chromium : ACTIF** — la variante packagée avec `-Sandbox` lance l'application via
  `bootstrap.exe` (CEF) et `faraday.dll` : le sandbox est **validé** en Windows Sandbox
  (l'interface affiche « Sandbox Chromium : ACTIF »). Cf. `PHASE5_DISTRIBUTION.md` §4.
- **Empreinte navigateur « unique »** au test EFF : **normal** (cf. `PHASE4_AUDIT.md` §3).

---

## 5. Rapport de test (à remplir)

```
Machine de test : Windows __ ( __ bits )   SAC : désactivé / absent
Version testée  : Faraday 0.1.0 x64   (portable / installateur)

[ ] Lancement OK            [ ] Page d'accueil OK
[ ] EFF : ads = ___ , invisible trackers = ___
[ ] Multi-onglets OK        [ ] Historique OK
[ ] Téléchargement OK       [ ] Clic droit OK
[ ] Session restaurée OK    [ ] Paramètres OK
[ ] Installation OK         [ ] Désinstallation OK (+ données conservées ? O/N)

Anomalies rencontrées :
 -
```

---

*Faraday — procédure de test VM, Phase 5.*
