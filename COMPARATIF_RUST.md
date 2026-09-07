# ⚖️ Faraday — Comparatif & Tableau de décision : « Tout en Rust » ?

> **Question :** Est-il possible de coder entièrement Faraday en Rust ?
>
> **Réponse courte : OUI.** Mais avec une nuance importante 👇

## ⚠️ Nuance essentielle à comprendre

Le **moteur Chromium** est écrit en **C++** (~20 millions de lignes) et **ne peut pas être « codé en Rust »**.
- On **embarque** Chromium (compilé en C++, fourni en bibliothèque précompilée).
- Tout le reste — *l'application, l'interface, la logique de confidentialité, le blocage des trackers* — peut être **100 % en Rust**.

> 🔎 *Servo* (Mozilla) est un moteur web en Rust, mais ce n'est **pas Chromium**. Il est donc **exclu** de ton cahier des charges.

---

## 🧰 Les 3 vraies options Rust pour embarquer Chromium sur Windows

### Option A — **CEF en Rust** (`cef-rs`, crate `cef`)
- C'est le même moteur que la version .NET : **Chromium pur via CEF**, mais en Rust.
- Géré par l'équipe **Tauri** (Wu Yuwei / Bill Avery), **très actif** : version `152.0.0` (≈ Chromium 152), dernière release il y a quelques heures.
- **Tu embarques ton propre Chromium** → pas de dépendance au runtime Edge de Microsoft.
- Contrôle total : flags Chromium, filtre de requêtes, content settings.

### Option B — **WebView2 via `wry` / Tauri**
- Sur Windows, `wry` utilise **WebView2 = Edge (donc Chromium)**.
- Très simple, écosystème Tauri énorme.
- ⚠️ Dépend du **runtime WebView2 (Microsoft Edge)** installé sur le système (Evergreen).

### Option C — **`webview2-com`** (bindings WebView2 bas niveau)
- Accès à 100 % de l'API COM WebView2, mais **beaucoup de `unsafe`**, maintenance lourde.

---

## 🆚 Tableau comparatif

| Critère | CefSharp (.NET/WPF) | **A. CEF Rust (`cef-rs`)** | B. WebView2 (`wry`/Tauri) | C. `webview2-com` |
|---|---|---|---|---|
| **Moteur** | CEF (Chromium pur) | CEF (Chromium pur) | WebView2 (Edge/Chromium) | WebView2 (Edge/Chromium) |
| **Langage app** | C# | **Rust** | **Rust** | **Rust** |
| **Chromium « pur »** | ✅ | ✅ | ⚠️ (Edge runtime) | ⚠️ (Edge runtime) |
| **Contrôle flags Chromium** | ✅ Total | ✅ Total | ⚠️ Partiel | ⚠️ Partiel |
| **Blocage trackers** | `IRequestHandler` | `OnBeforeResourceLoad` | `WebResourceRequested` | `WebResourceRequested` |
| **Maturité bindings Rust** | — | ✔️ Active (v152, équipe Tauri) | ✅ Très mature | ⚠️ Bas niveau |
| **Vitesse dev UI Windows** | ✅ WPF mature | ⚠️ Fraîche (egui/iced) | ✔️ Facile (UI web) | ❌ Lente |
| **Performance / empreinte** | ⚠️ Runtime .NET | ✅ Léger, rapide | ✔️ Léger | ✔️ Léger |
| **Dépendance runtime externe** | Aucune | **Aucune** | Edge WebView2 | Edge WebView2 |
| **Écosystème / recrutement** | ✅ Large | 🔶 En croissance | ✅ Très large | 🔶 Restreint |
| **Windows 10 minimum** | ✅ (1809+) | ✅ | ✅ | ✅ |

---

## 🧮 Tableau de décision (score pondéré /5)

### Pondération (importance pour Faraday = grand public + zéro tracking)
- 🔒 **Confidentialité / Chromium pur** : 30 %
- 🛠 **Maturité & stabilité** : 20 %
- 🚀 **Vitesse de livraison** : 15 %
- ⚡ **Performance & empreinte** : 10 %
- 🎛 **Contrôle fin de Chromium** : 15 %
- 🌍 **Écosystème** : 10 %

### Scores
| Critère (poids) | CefSharp (.NET/WPF) | A. CEF Rust | B. WebView2 (wry) | C. `webview2-com` |
|---|:---:|:---:|:---:|:---:|
| Confidentialité (30 %) | 5 | 5 | 3 | 3 |
| Maturité (20 %) | 5 | 3 | 4 | 3 |
| Vitesse dev (15 %) | 5 | 3 | 4 | 2 |
| Performance (10 %) | 3 | 5 | 4 | 4 |
| Contrôle Chromium (15 %) | 5 | 5 | 3 | 3 |
| Écosystème (10 %) | 4 | 3 | 5 | 3 |
| **TOTAL PONDÉRÉ** | **4,70** | **4,10** | **3,65** | **2,95** |

---

## 🎯 Recommandation

| Priorité | Choix | Pourquoi |
|---|---|---|
| **Tu veux du 100 % Rust** 🦀 | **A. CEF Rust (`cef-rs`)** | Seule option qui garde **Chromium pur**, un contrôle total des flags, **sans** runtime Edge de Microsoft, et sans dépendance C#. 👉 Le bon choix si l'objectif « tout en Rust » est ferme. |
| **Tu veux livrer vite, UI Windows nickel** | CefSharp (.NET/WPF) | Plus mature côté UI et recrutement, plus rapide à développer, mais en C#. |
| **Tu veux du Rust minimale effort** | B. WebView2 (wry/Tauri) | Très simple, mais dépend du runtime Edge et moins de contrôle « Chromium pur ». |

> **Conclusion :** Oui, Faraday peut être **entièrement en Rust**. Le meilleur compromis pour ton cahier des charges (Chromium + zéro tracking + Windows 10) est **`cef-rs`** — avec une **UI en Rust** (par ex. `egui` ou `iced`) pour la barre d'outils et les onglets.

---

## 🛠 UI Rust recommandée pour le chrome du navigateur
- **`egui`** : rapide à prototyper, style moderne léger.
- **`iced`** : plus structurée, bon pour une UI « produit ».
- **`tao`/`winit`** : couche fenêtre (utilisée par `wry`/CEF).
- Option : la barre d'onglets/URL peut aussi être une **page web locale** servie par CEF (`custom protocol`), ce qui simplifie grandement l'UI.

---

## 🔄 Impact sur le plan initial
Si on passe en Rust, le plan reste **identique** dans les phases, seul le stack change :
| Domaine | Avant (.NET) | Après (Rust) |
|---|---|---|
| Moteur | CEF (CefSharp) | CEF (`cef` crate) |
| Runtime | .NET 8 | Rust (cargo) |
| UI | WPF | `egui` / `iced` / webview local |
| Base locale | SQLite (encrypté) | `rusqlite` + chiffrement |
| CI | GitHub Actions | GitHub Actions (cargo) |
| Packaging | WiX / Inno Setup | WiX / Inno Setup + `cargo` |

*© Faraday — comparatif v1.0*
