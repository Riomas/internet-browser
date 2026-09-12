# 🔒 Faraday — Politique de confidentialité

**Dernière mise à jour : 13 septembre 2026** · Version du logiciel : 0.1.0

Faraday est un navigateur web « privacy-first ». Son principe de conception est simple :
**tout ce que fait Faraday reste sur votre machine**, et rien n'est transmis à qui que ce
soit sans votre action.

## 1. Ce que Faraday ne fait PAS

- ❌ **Aucune télémétrie** : pas de statistiques d'utilisation, pas de rapport de plantage
  automatique, pas de « ping » de démarrage.
- ❌ **Aucun compte** : vous n'avez rien à créer, aucune adresse e-mail à fournir.
- ❌ **Aucune synchronisation** : vos favoris, votre historique et vos réglages ne quittent
  jamais votre ordinateur (la fonction de synchronisation de Chromium est désactivée).
- ❌ **Aucune mise à jour de composants en arrière-plan** : Faraday n'interroge aucun
  serveur au démarrage ni pendant la navigation.
- ❌ **Aucun profilage publicitaire**, aucune revente de données (Faraday n'en collecte pas).

## 2. Ce qui est stocké, et où

Tout réside dans **votre profil utilisateur**, sur votre disque :

| Donnée | Emplacement | Transmission |
|---|---|---|
| Réglages de confidentialité | `%APPDATA%\Faraday\profiles\<profil>\privacy.toml` | locale uniquement |
| Onglets ouverts et historique | `…\session.json`, `…\history.json` | locale uniquement |
| Favoris | `…\bookmarks.json` | locale uniquement |
| Domaines débloqués (exceptions) | `…\exceptions.json` | locale uniquement |
| Cache et cookies Chromium | `…\cef\` | locale uniquement |
| Journal de démarrage | `faraday-startup.log` (à côté de l'exécutable) | locale uniquement |

Vous pouvez **effacer ces données** à tout moment en supprimant le dossier du profil,
ou en cochant l'option correspondante lors de la désinstallation.

> 🧪 **Journal de diagnostic (facultatif)** : si la variable d'environnement
> `FARADAY_DIAG=1` est positionnée, Faraday écrit `diag.log` dans le profil. Ce fichier
> contient **uniquement des noms d'hôtes** de requêtes déjà bloquées par la liste de
> règles, sert au dépannage, et **n'est pas activé par défaut**.

## 3. Les communications réseau, et pourquoi elles ont lieu

Faraday est un navigateur : lorsqu'il affiche un site web, il **envoie des requêtes à ce
site**, parce que c'est ce que vous lui avez demandé. Ces requêtes contiennent
naturellement votre adresse IP et un identifiant de navigateur. Faraday **réduit** ces
informations au minimum :

- blocage des domaines de suivi et de publicité (liste de règles embarquée) ;
- filtrage des **cookies tiers** ;
- envoi de l'en-tête **Do Not Track** ;
- suppression de l'en-tête **Referer** ;
- **HTTPS forcé** lorsque le site le propose ;
- **IP locale masquée** pour les appels WebRTC.

Faraday **n'est pas un proxy ni un VPN** : le trafic part directement de votre machine
vers les sites visités, comme avec tout navigateur.

**Moteur de recherche** : la barre d'adresse utilise par défaut **DuckDuckGo**
(`https://duckduckgo.com`). Vos recherches sont donc envoyées à ce moteur ; vous pouvez le
changer dans les Paramètres de Faraday.

**Téléchargements** : un fichier téléchargé vient du serveur que vous avez choisi, sans
intermédiaire.

## 4. Composants tiers

Faraday s'appuie sur le **Chromium Embedded Framework (CEF)** et sur Chromium, distribués
sous licence BSD. Faraday désactive la télémétrie, les suggestions, l'autofill personnel et
les mises à jour de composants de Chromium ; aucun de ces composants n'envoie de données
depuis Faraday.

## 5. Vos droits

- **Accès / portabilité** : vos données sont des fichiers locaux lisibles (JSON, TOML).
- **Effacement** : supprimez `%APPDATA%\Faraday` et le dossier d'installation.
- **Opposition** : désinstallez Faraday ; aucun compte ni transmission n'est à résilier.

## 6. Contact

Questions ou signalement d'un problème de confidentialité : ouvrez une *issue* sur
<https://github.com/Riomas/internet-browser/issues>.

---

> **Résumé en une phrase** : Faraday ne transfère aucune information vers des systèmes
> réseau tiers, sauf demande explicite de votre part (visiter un site, lancer une
> recherche, télécharger un fichier).
