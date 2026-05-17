<div align="center">
  <img src="src-tauri/icons/128x128.png" alt="Accolade Logo" width="128" />
  
  # Accolade
  
  **Donnez une seconde vie à vos succès.**
  
  [![Tauri](https://img.shields.io/badge/Tauri-v2-FFC131?logo=tauri&logoColor=white)](https://tauri.app/)
  [![Svelte](https://img.shields.io/badge/Svelte-5-FF3E00?logo=svelte&logoColor=white)](https://svelte.dev/)
  [![Rust](https://img.shields.io/badge/Rust-2021-000000?logo=rust&logoColor=white)](https://www.rust-lang.org/)
  [![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

_Accolade est une application de bureau moderne qui synchronise et affiche vos succès issus de jeux "cracked" avec de l'élégance._

[Fonctionnalités](#-fonctionnalités) • [Émulateurs Supportés](#-émulateurs-supportés) • [Installation](#-installation) • [Captures](#-captures-décran)

</div>

---

## ✨ Fonctionnalités

Accolade comble le fossé entre vos jeux locaux et l'expérience sociale des succès :

- 🔍 **Scan Intelligent** : Détection automatique des fichiers de succès locaux.
- 🖼️ **Métadonnées Steam** : Enrichissement automatique avec les noms, descriptions et icônes officiels.
- 🔔 **Overlay Real-time** : Notifications de succès en plein jeu (style PS5/Steam).
- 🎨 **Personnalisation Totale** : Thèmes (Clair/Sombre), couleurs d'accentuation, et sons de notification (PS4, PS5, Xbox, Steam).
- 🌍 **Multilingue** : Support complet pour FR, EN, ES, DE, IT.
- ⚡ **Performance Rust** : Coeur ultra-léger et rapide grâce à Tauri v2.

## 🕹️ Émulateurs Supportés

Accolade supporte nativement les formats de succès les plus courants :

| Émulateur     | Emplacement par défaut              |
| :------------ | :---------------------------------- |
| **Goldberg**  | `%APPDATA%\Goldberg SteamEmu Saves` |
| **Empress**   | `%APPDATA%\Empress-Emulator`        |
| **CODEX**     | `%APPDATA%\Steam\CODEX`             |
| **OnlineFix** | `%PUBLIC%\Documents\OnlineFix`      |
| **RUNE**      | `%PUBLIC%\Documents\Steam\RUNE`     |

## 🚀 Installation

### Prérequis (Windows)

- [Node.js](https://nodejs.org/) (v20+)
- [Rust](https://www.rust-lang.org/tools/install)
- [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (souvent déjà présent sur Windows 10/11)

### Configuration rapide

1.  **Clonage et dépendances**

    ```bash
    git clone https://github.com/votre-repo/accolade.git
    cd accolade
    npm install
    ```

2.  **Lancement en développement**

    ```bash
    npm run tauri dev
    ```

3.  **Compilation finale**
    ```bash
    npm run tauri build
    ```

## ⚙️ Configuration de l'App

Pour une expérience optimale, configurez les éléments suivants dans les paramètres :

- **Steam ID** : Votre ID 64-bit pour synchroniser votre bibliothèque.
- **Steam API Key** : Nécessaire pour récupérer les succès et métadonnées.
- **SteamGridDB API Key** (Optionnel) : Pour des icônes de haute qualité.

## 🛠️ Stack Technique

- **Frontend** : Svelte 5 + SvelteKit + Tailwind-ish CSS (Vanilla)
- **Backend** : Rust + Tauri v2
- **Store** : Tauri Plugin Store (Persistance locale)
- **Watcher** : `notify` (Rust) pour le suivi des fichiers en temps réel

---
