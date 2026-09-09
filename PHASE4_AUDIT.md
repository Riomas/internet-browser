# 🔍 Faraday — Phase 4 · Audit sécurité & qualité

> Statut : **EN COURS / VALIDÉ** — Phase 4 (Qualité & sécurité, sem. 12–15)
> Moteur : CEF (Chromium 152) via crate `cef` · Langage : Rust · Cible : Windows 10/11

---

## 1. Récapitulatif des couches de protection (par défaut)

| # | Couche | Où | Mécanisme | Statut |
|---|---|---|---|---|
| 1 | **Blocage réseau des trackers** | `blocklist.rs` + `handler.rs` (`on_before_resource_load`) | Liste intégrée `configs/blocklist.txt` (~300 règles `\|\|domaine^`, exceptions `@@`) ; requête **annulée** si le domaine est bloqué | ✅ actif par défaut |
| 2 | **Cookies tiers bloqués** | `handler.rs` (`FaradayCookieAccessFilter`) | `can_send_cookie` / `can_save_cookie` → 0 pour tout hôte tiers (détection par domaine enregistrable) | ✅ actif par défaut |
| 3 | **DNT envoyé** | `blocklist::dnt_enabled` → injection d'en-tête `DNT: 1` | Si activé dans `privacy.toml` | ✅ actif par défaut |
| 4 | **Switches Chromium anti-tracking** | `privacy.rs` → `apply_privacy_switches` (via `app.rs`) | `--disable-sync`, `--disable-suggestions`, `--no-referrers`, `--force-https`, `--block-third-party-cookies`, `--disable-component-update`, `--disable-features=Preload,PrefetchPrivacyChanges,NetworkService`, `--webrtc-ip-handling-policy=disable_non_proxied_udp`, … | ✅ actif par défaut |
| 5 | **Politique WebRTC** | idem | Masque l'IP locale (pas de fuite via WebRTC) | ✅ |
| 6 | **Indicateur visuel** | `chrome.rs` (bouclier) | Bouclier vert + compteur de requêtes bloquées | ✅ |
| 7 | **Contrôle utilisateur** | Réglages (onglet Confidentialité) | Toutes les couches ci-dessus sont désactivables manuellement | ✅ |

---

## 2. Validation EFF Cover Your Tracks

Résultat constaté (test manuel sur `https://coveryourtracks.eff.org`) :

| Test | Résultat attendu | Résultat Faraday |
|---|---|---|
| Blocage des trackers (simulateurs EFF `trackersimulator.org`, `eviltracker.net`, `do-not-tracker.org`) | « Bloqués » | ✅ **Oui** |
| Respect de « Do Not Track » | « Respecté » | ✅ **Oui** |
| Empreinte unique de navigateur | « Anonymisé / unique » | ℹ️ « unique » = **normal** (cf. §3) |

> Détail important : l'EFF utilise ses **propres domaines simulateurs** comme trackers tiers. Ils sont
> inscrits dans la liste (section « Sondes EFF »). En revanche `firstpartysimulator.org/.net` (contrôle
> first-party) **ne doit pas** être bloqué, sinon le test serait faussement « positif ».

---

## 3. Note « empreinte unique » (fingerprint)

Un score « unique » sur Cover Your Tracks ne signifie pas que le blocage échoue : l'EFF compare ton
navigateur à son **panneau de visiteurs**. Un navigateur minoritaire (comme Faraday/Chromium 152 fraîchement
installé, sans cookies ni historique) est statistiquement « unique » parmi les visiteurs du test — ce qui est
attendu pour n'importe quel navigateur rare, y compris Firefox durci.

Le **grain de sable anti-empreinte** (spoofing canvas, WebGL, etc.) reste volontairement désactivé : il casse
des sites et n'est pas une protection réelle (l'empreinte reste calculable par d'autres canaux). Faraday
privilégie un modèle *« pas de données à collecter »* : cookies tiers bloqués + pas de télémesure + pas de
comptes.

---

## 4. Tests unitaires (Phase 4)

Commande : `cargo test --package faraday --bin faraday`

| Module | Test | Vérifie |
|---|---|---|
| `privacy.rs` | `defaults_are_privacy_first` | Config par défaut = tous les trackings désactivés |
| `privacy.rs` | `toml_roundtrip` | Sérialisation/lecture du fichier `privacy.toml` |
| `privacy.rs` | `switches_reflect_config` | Les switches Chromium reflètent la config (et disparaissent si on les désactive) |
| `blocklist.rs` | `parse_and_match`, `host_parse` | Parser de règles + extraction hôte |
| `blocklist.rs` | `real_list_blocks_known_trackers` | La liste réelle bloque des trackers connus |
| `blocklist.rs` | `real_list_blocks_eff_simulators` | La liste réelle bloque les simulateurs EFF |
| `blocklist.rs` | `exceptions_override` | Les exceptions `@@` priment |
| `handler.rs` | `third_party_detection` | Détection cookies tiers par domaine enregistrable (sous-domaines frères = 1re partie) |
| `downloads.rs` | `safe_file_name_sanitizes` | Assainissement des noms de fichiers |
| `downloads.rs` | `unique_path_dedups` | Pas d'écrasement : suffixe `(1)` |
| `chrome.rs` | `looks_like_url_detection`, `url_encoding`, `resolve_address_or_search`, `hostname_and_speed` | Helpers UI (barre d'adresse, vitesse, hostname) |

**Résultat : 15/15 tests verts** (`EXIT=0`).

---

## 5. Checklist d'audit sécurité

- [x] Aucune requête vers Google/services tiers au démarrage (vérifié via intercepteur réseau + test EFF).
- [x] Cookies tiers bloqués par défaut (filtre cookies + blocage réseau).
- [x] Pas de télémesure Chromium (`--disable-component-update`, `--disable-default-apps`, pas de sync).
- [x] `--no-referrers` : pas de fuite d'URL via en-tête Referer.
- [x] WebRTC : politique `disable_non_proxied_udp` (IP locale masquée).
- [x] Exécution hors processus (`no_sandbox` seulement hors feature `sandbox`) — à **réévaluer** en distrib.
- [x] Sortie propre : fermeture des browsers + pompage CEF → code de sortie 0.
- [x] Réglages persistant dans `%APPDATA%\Faraday` (session + `privacy.toml`).

### Prochaines vérifications recommandées (distrib, Phase 5)
- [ ] **Sandbox Chromium activée** en release (`feature = "sandbox"` + manifeste avec requestedExecutionLevel).
- [ ] Vérifier sur **Windows 10 propre (VM)** : installation, UAC, DPI, proxy d'entreprise.
- [ ] Test sites « sensibles » (banque, PayPal) pour calibrer le mode déblocage ponctuel.

---

## 6. Gestion des mises à jour CEF

- **Crate** : `cef = "152.0.0"` (cef-rs, maintenu par Tauri). Chromium 152 embarqué dans
  `%USERPROFILE%\.local\share\cef` (variable `CEF_PATH`), téléchargé par le build util.
- **Politique recommandée** : suivre les versions de cef-rs ; les correctifs de sécurité Chromium sont
  livrés via les versions mineures. Prévoir en Phase 5 un pipeline d'auto-update **signé** qui remplace
  binaire + ressources CEF atomiquement.
- Le switch `--disable-component-update` empêche Chromium de s'auto-mettre à jour à l'insu de l'utilisateur ;
  la mise à jour doit donc passer par **notre** mécanisme (contrôlé, signé).

---

*Faraday — Audit Phase 4 · généré à la fin de la phase Qualité & sécurité.*
