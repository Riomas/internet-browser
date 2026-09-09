# 🛡️ Faraday — Guide de l'utilisateur

> **Faraday** est un navigateur web qui protège votre vie privée **par défaut** :
> rien à configurer, vous êtes protégé dès le premier clic.
>
> Slogan : *« La protection est le réglage par défaut. »*

---

## 1. Installation

### Version portable (aucune installation)
1. Décompressez `Faraday-0.1.0-x64-portable.zip`.
2. Double-cliquez sur `faraday.exe`.
3. (Optionnel) Créez un raccourci vers `faraday.exe` sur le bureau ou la barre des tâches.

Vos données (onglets, historique, réglages) sont enregistrées dans
`%APPDATA%\Faraday` — pas dans le dossier du programme. Vous pouvez donc
déplacer le dossier Faraday sans rien perdre.

### Version installateur (`.exe`)
1. Lancez `Faraday-Setup-0.1.0-x64.exe`.
2. Suivez l'assistant (installation **pour votre compte uniquement**, sans mot de
   passe administrateur).
3. Faraday est disponible dans le menu Démarrer et (si coché) sur le bureau.

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

Le **bouclier vert** en haut à droite indique que la protection est active, et le
chiffre à côté correspond au nombre de requêtes de suivi bloquées pendant la session.

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
| Session (onglets ouverts), historique | `%APPDATA%\Faraday\session.json` |
| Réglages de confidentialité | `%APPDATA%\Faraday\privacy.toml` |
| Téléchargements | Dossier `Téléchargements` habituel |

Aucun compte, aucune synchronisation, aucune donnée envoyée à un serveur.

---

## 6. Si un site ne fonctionne pas

Certains sites exigent des cookies tiers ou des bloqueurs désactivés (rare).
1. Ouvrez **Paramètres ⚙**.
2. Désactivez ponctuellement le réglage concerné (par ex. « Cookies tiers »),
   rechargez la page.
3. **Réactivez-le ensuite** avec « Rétablir les valeurs par défaut » puis
   « Enregistrer ».

> Faraday est conçu pour que la protection reste le réglage par défaut :
> pensez à la remettre après votre visite.

---

*Faraday v0.1.0 — Windows 10 / 11 (64 bits).*
