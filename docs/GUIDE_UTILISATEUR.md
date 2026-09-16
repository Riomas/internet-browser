# 🛡️ Faraday — Guide de l'utilisateur

> **Faraday** est un navigateur web qui protège votre vie privée **par défaut** :
> rien à configurer, vous êtes protégé dès le premier clic.
>
> Slogan : *« La protection est le réglage par défaut. »*

---

## 1. Installation

### Version portable (aucune installation)
1. Décompressez `Faraday-0.1.0-x64-portable.zip`.
2. **Approuvez le certificat de signature** (une seule fois, sans droits administrateur) —
   dans le dossier décompressé :
   ```powershell
   powershell -ExecutionPolicy Bypass -File .\faraday-certificat.ps1 -Install
   ```
   *(Le certificat est un certificat **auto-signé** ; Windows l'affichera comme non vérifié.
   Détails et points de sécurité : [`CERTIFICAT_AUTOSIGNE.md`](CERTIFICAT_AUTOSIGNE.md).)*
3. Double-cliquez sur `faraday.exe`.
4. (Optionnel) Créez un raccourci vers `faraday.exe` sur le bureau ou la barre des tâches.

> Sans l'étape 2, le navigateur **ne démarre pas** : aucune fenêtre ne s'ouvre et un fichier
> `debug.log` apparaît à côté de `faraday.exe` (règle de vérification de signature de CEF,
> détaillée dans [`CERTIFICAT_AUTOSIGNE.md`](CERTIFICAT_AUTOSIGNE.md) §1).

Vos données (onglets, historique, réglages) sont enregistrées dans
`%APPDATA%\Faraday` — pas dans le dossier du programme. Vous pouvez donc
déplacer le dossier Faraday sans rien perdre.

### Version installateur (`.exe`)
1. Lancez `Faraday-Setup-0.1.0-x64.exe`.
2. Suivez l'assistant (installation **pour votre compte uniquement**, sans mot de
   passe administrateur).
3. À l'écran **« Certificat de signature Faraday »**, laissez la case cochée : le certificat
   public est ajouté aux certificats de confiance de **votre compte** (c'est ce qui permet à
   Faraday de démarrer). Aucun droit administrateur n'est demandé et le certificat est retiré
   automatiquement à la désinstallation.
4. Windows peut demander une confirmation (« Voulez-vous installer ce certificat ? ») :
   répondez **Oui**.
5. Faraday est disponible dans le menu Démarrer et (si coché) sur le bureau.

> 🔐 **Pourquoi un certificat ?** Faraday est signé, mais avec un certificat **auto-signé**
> (en attendant un certificat d'autorité payant) : Windows ne peut donc pas le vérifier
> seul. En savoir plus, et comment le retirer à tout moment :
> [`CERTIFICAT_AUTOSIGNE.md`](CERTIFICAT_AUTOSIGNE.md).

> À la désinstallation, vos données de navigation sont **conservées** par défaut.
> Cochez « Supprimer aussi les données de navigation » pour tout effacer.

---

## 2. Premiers pas

| Action | Comment faire |
|---|---|
| Aller sur un site | Tapez l'adresse dans la barre du haut puis `Entrée` |
| Rechercher | Tapez un texte (sans « .com ») : la recherche s'ouvre sur DuckDuckGo |
| Nouvel onglet | Bouton `+` ou `Ctrl+T` |
| Fermer un onglet | Bouton `×` sur l'onglet ou `Ctrl+W` |
| Revenir / avancer | Flèches `←` / `→` de la barre d'outils |
| Recharger | Bouton circulaire ou `Ctrl+R` |
| Accueil / raccourcis | Onglet vide : vos sites favoris (DuckDuckGo, Wikipédia, GitHub…) |
| Historique | Bouton horloge ou `Ctrl+H` |
| Téléchargements | Bouton flèche-bas ou `Ctrl+J` |
| **Ajouter aux favoris** | Étoile de la barre d'outils ou `Ctrl+D` |
| **Voir les favoris** | Bouton signet ou `Ctrl+B` |
| **Débloquer un site** | Bouton bouclier/attention dans la barre d'outils |
| Paramètres | Bouton engrenage ⚙ |

---

## 3. Confidentialité : protégé par défaut

Faraday bloque, **sans aucune action de votre part** :

- 🚫 **Les trackers publicitaires et traceurs invisibles** (liste intégrée,
  mise à jour à chaque version du navigateur).
- 🚫 **Les cookies tiers** : les sites ne peuvent pas vous suivre d'un site à l'autre.
- 🚫 **Le suivi intersites** : en-tête « Do Not Track » envoyé.
- 🚫 **Les référents** (`Referer`) : le site de destination ne voit pas d'où vous venez.
- 🚫 **La synchronisation / télémétrie Chromium** : rien n'est envoyé à Google.
- 🚫 **Les préchargements** de pages/liens non sollicités.
- 🌐 **HTTPS forcé** quand le site le propose.
- 🔇 **Votre IP locale masquée** pour les appels WebRTC.

Le **bouclier avec compteur** en haut à droite indique la protection du **site affiché** :
vert et chiffré tant que le site est protégé, **orange « off »** quand la protection est
levée (pour ce site ou globalement). Son infobulle détaille les blocages (site courant,
total de la session, autres sites concernés).

### Débloquer un site (protection par site)

Certains sites (banques, messageries, portails d'entreprise) ont besoin de
ressources tierces pour fonctionner. Faraday permet de **désactiver la protection
pour un seul site**, sans toucher au reste :

1. Ouvrez le site concerné.
2. Dans la barre d'outils, cliquez sur le bouton **bouclier** (vert) : il devient
   **attention** (orange) → la protection est coupée **pour ce domaine
   uniquement** (et ses sous-domaines).
3. Rechargez la page si nécessaire.
4. Pour **réactiver** : cliquez à nouveau sur le bouton (ou passez par
   **Paramètres ⚙ → Protection par site** → poubelle).

> 🔒 Le reste de votre navigation continue d'être protégé : seuls les domaines
> exemptés sont concernés. Consultez la liste à tout moment dans les Paramètres.

> 🔢 **Le compteur du bouclier indique les blocages du site affiché** : il tombe à zéro
> dès que la protection est désactivée pour ce site (le chiffre et l'icône passent en
> orange, comme le bouton de déblocage). L'infobulle donne le **total de la session**
> ainsi que **les autres sites qui déclenchent encore des blocages** — utile quand une
> page fait charger des ressources depuis d'autres sites (c'est le cas du test EFF, qui
> interroge trois domaines différents).

### Suspendre toute la protection (temporaire)

Pour dépanner un site récalcitrant, ou pour comparer un site **avec et sans** protection :

1. Cliquez sur le **bouclier‑compteur** (en haut à droite) : il passe en **orange « off »**
   → plus rien n'est bloqué (trackers, cookies tiers) et l'en-tête DNT n'est plus envoyé.
2. Cliquez à nouveau : la protection revient **immédiatement**.

La suspension vaut pour **tous les onglets** et n'est **pas enregistrée** : elle est levée
au redémarrage de Faraday. Même réglage dans **Paramètres ⚙ → Confidentialité →
« Suspendre toute la protection (cette session seulement) »**, à côté du bouton
**« Tester la protection sur EFF Cover Your Tracks »**.

> 🧪 **Le vérifier soi‑même** : sur <https://coveryourtracks.eff.org>, lancez le test →
> *Yes/Yes*. Suspendez la protection, relancez le test → *No/No*. Réactivez ensuite.
> Rappel : si vous préférez ne lever la protection que sur **un** site (pour le test EFF,
> il faut exempter ses trois domaines), utilisez le bouton de déblocage par site.

> 🧪 **Sites de test (EFF Cover Your Tracks…)** : certains tests chargent leurs
> pistes de suivi depuis **plusieurs domaines différents**. Le verdict ne change
> donc que si **tous** les domaines concernés sont exemptés. Exemple avec
> `coveryourtracks.eff.org` : le test interroge aussi `firstpartysimulator.net`
> et `firstpartysimulator.org` — tant que ces deux domaines restent bloqués,
> le résultat affiche toujours « bloqué : oui ». C'est le comportement attendu,
> pas un défaut du bouton : la protection fait son travail.

### Réglages (Paramètres ⚙)
Tout est activé par défaut. Vous pouvez désactiver un blocage si un site
fonctionne mal. Certains réglages (synchronisation, suggestions…) prennent
effet au prochain démarrage.

> 💡 **Votre empreinte de navigateur reste « unique »** — c'est normal et sans
> danger : c'est simplement parce que peu de visiteurs utilisent exactement le
> même navigateur que vous. Faraday ne collecte rien ; il n'y a donc rien à
> exploiter.

---

## 4. Fonctionnalités

- **Multi-onglets** : ouvrez autant d'onglets que nécessaire.
- **Page d'accueil** : raccourcis cliquables + barre de recherche intégrée.
- **Historique** : récent, consultable et effaçable (poubelle).
- **Favoris** : étoile dans la barre d'outils (ou `Ctrl+D`) pour enregistrer la
  page affichée ; bouton signet (ou `Ctrl+B`) pour retrouver vos favoris.
- **Profils** : plusieurs profils isolés (onglets, favoris, historique,
  exceptions, cache) — **Paramètres ⚙ → Profil**. Le changement prend effet au
  redémarrage.
- **Téléchargements** : suivi en direct, annulation, ouverture du dossier.
- **Notifications** discrètes en haut à droite (début/fin de téléchargement…).
- **Clic droit** : copier un lien, l'ouvrir dans un nouvel onglet, copier
  l'adresse de la page, recharger…
- **Survol d'un lien** : l'adresse s'affiche en bas à gauche (pour vérifier où
  vous allez avant de cliquer).
- **Thème** : clair / sombre / selon le système (Paramètres).
- **Restauration de session** : vos onglets ouverts sont retrouvés au prochain
  lancement.

---

## 5. Vos données

Tout est stocké **localement** sur votre ordinateur :

| Donnée | Emplacement |
|---|---|
| Session (onglets ouverts), historique | `%APPDATA%\Faraday\profiles\<profil>\session.json` |
| Favoris | `%APPDATA%\Faraday\profiles\<profil>\bookmarks.json` |
| Sites débloqués (exceptions) | `%APPDATA%\Faraday\profiles\<profil>\exceptions.json` |
| Réglages de confidentialité | `%APPDATA%\Faraday\profiles\<profil>\privacy.toml` |
| Liste des profils | `%APPDATA%\Faraday\profiles.json` |
| Téléchargements | Dossier `Téléchargements` habituel |

> 🗂️ Chaque **profil** possède son propre dossier : changer de profil change
> entièrement vos onglets, favoris, historique, exceptions et cache.

Aucun compte, aucune synchronisation, aucune donnée envoyée à un serveur.

---

## 6. Si un site ne fonctionne pas

**Méthode recommandée (rapide et ciblée)** :
1. Ouvrez le site concerné.
2. Cliquez sur le bouton **bouclier** de la barre d'outils (il devient orange
   « attention ») → la protection est désactivée **pour ce site seulement**.
3. Rechargez la page.
4. Une fois terminé, **réactivez** la protection (même bouton) ou gérez la liste
   dans **Paramètres ⚙ → Protection par site**.

**Autre méthode (globale)** : dans **Paramètres ⚙ → Confidentialité**,

1. Désactivez ponctuellement le réglage concerné (par ex. « Cookies tiers »),
   rechargez la page.
2. **Réactivez-le ensuite** avec « Rétablir les valeurs par défaut » puis
   « Enregistrer ».

> Faraday est conçu pour que la protection reste le réglage par défaut :
> pensez à la remettre après votre visite.

### Diagnostic avancé (aide au dépannage)

Pour comprendre précisément ce qui est bloqué, lancez Faraday avec la variable
d'environnement `FARADAY_DIAG=1` :

```
set FARADAY_DIAG=1
faraday.exe
```

Un fichier `diag.log` est alors écrit dans le dossier de votre profil
(`%APPDATA%\Faraday\profiles\<profil>\`). Il indique, pour chaque requête
concernée par la liste de blocage, si elle a été **BLOQUÉE** ou **AUTORISÉE**
(parce que le site est exempté), avec le site de rattachement :

```
RESEAU trackersimulator.org (site=example.com exempt=non) -> BLOQUE
```

> 🔒 Respect de la vie privée : seuls des **noms d'hôtes** sont écrits — jamais
> d'URL complète, de chemin, de paramètre ni de cookie. Le journal est
> **désactivé par défaut** et n'est créé que si vous activez `FARADAY_DIAG`.

> 🧰 En cas de **non-démarrage** de l'application, un fichier
> `faraday-startup.log` est écrit à côté de `faraday.exe` (version, chargement
> de `libcef.dll`, profil actif, état du sandbox). C'est le premier élément à
> transmettre pour un diagnostic. Dans ce cas précis, aucune variable
> d'environnement n'est nécessaire.

---

*Faraday v0.1.0 — Windows 10 / 11 (64 bits).*
