<div align="center">
  <img src="src-tauri/icons/128x128.png" alt="Accolade Logo" width="96" />

  # Accolade

  **Donnez une seconde vie à vos succès.**

  [![Latest Release](https://img.shields.io/github/v/release/NotSerigne/Accolade?style=flat-square&color=c8a96e&label=version)](https://github.com/NotSerigne/Accolade/releases/latest)
  [![Downloads](https://img.shields.io/github/downloads/NotSerigne/Accolade/total?style=flat-square&color=c8a96e)](https://github.com/NotSerigne/Accolade/releases)
  [![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
  [![Tauri](https://img.shields.io/badge/Tauri-v2-FFC131?style=flat-square&logo=tauri&logoColor=white)](https://tauri.app/)
  [![Svelte](https://img.shields.io/badge/Svelte-5-FF3E00?style=flat-square&logo=svelte&logoColor=white)](https://svelte.dev/)
  [![Rust](https://img.shields.io/badge/Rust-2021-orange?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)

  _Accolade est une application Windows qui synchronise et affiche vos succès issus de jeux crackés, avec des notifications en overlay style PS5/Steam et une interface moderne._

  [Télécharger](#-télécharger) • [Fonctionnalités](#-fonctionnalités) • [Émulateurs supportés](#-émulateurs-supportés) • [Configuration](#-configuration) • [Contribuer](#-contribuer)

</div>

---

## ⬇️ Télécharger

Rendez-vous sur la page [**Releases**](https://github.com/NotSerigne/Accolade/releases/latest) et téléchargez l'installateur `.msi` ou le setup `.exe`.

> **Prérequis** : Windows 10 / 11 (64-bit). WebView2 est généralement déjà présent ; si ce n'est pas le cas, Windows vous proposera de l'installer automatiquement.

---

## ✨ Fonctionnalités

- 🔍 **Scan automatique** — Détection des fichiers de succès pour tous les émulateurs supportés.
- 🔔 **Overlay temps réel** — Notification en plein jeu dès qu'un succès est débloqué, sans alt-tab.
- 🖼️ **Métadonnées Steam** — Noms, descriptions, icônes et rareté récupérés depuis l'API Steam.
- 🎨 **Personnalisation** — Thème clair / sombre / système, couleur d'accentuation, sons de notification (PS4, PS5, Xbox, Steam…).
- 📊 **Statistiques & Journal** — Vue d'ensemble de votre progression, favoris, tags et objectifs personnels.
- 📸 **Screenshots automatiques** — Capture d'écran déclenchée à chaque succès débloqué, avec raccourci configurable.
- 🌍 **Multilingue** — Interface disponible en FR, EN, ES, DE, IT.
- 🚀 **Démarrage avec Windows** — Mode silencieux dans la barre des tâches.
- ⚡ **Ultra léger** — Cœur Rust via Tauri v2, ~10 Mo en RAM au repos.

---

## 🕹️ Émulateurs Supportés

| Émulateur | Emplacement par défaut |
| :--- | :--- |
| **Goldberg** | `%APPDATA%\Goldberg SteamEmu Saves` ou `%APPDATA%\GSE Saves` |
| **Empress** | `%APPDATA%\Empress-Emulator` |
| **CODEX** | `%APPDATA%\Steam\CODEX` |
| **OnlineFix** | `%PUBLIC%\Documents\OnlineFix` |
| **RUNE** | `%PUBLIC%\Documents\Steam\RUNE` |

---

## ⚙️ Configuration

Au premier lancement, l'assistant de configuration vous guidera. Vous aurez besoin de :

| Paramètre | Obligatoire | Comment l'obtenir |
| :--- | :---: | :--- |
| **Steam ID** (64-bit) | ✅ | [steamid.io](https://steamid.io) ou votre profil Steam → *Copier l'URL du profil* |
| **Steam API Key** | ✅ | [steamcommunity.com/dev/apikey](https://steamcommunity.com/dev/apikey) (compte Steam requis) |
| **SteamGridDB API Key** | ⬜ | [steamgriddb.com/profile/preferences/api](https://www.steamgriddb.com/profile/preferences/api) — améliore la qualité des icônes |

> **Dossiers de jeux** : ajoutez les dossiers racines où se trouvent vos jeux. Accolade cherchera automatiquement les fichiers de succès dans les emplacements connus des émulateurs.

---

## 🛠️ Build depuis les sources

Vous avez besoin de [Node.js 20+](https://nodejs.org/), [Rust](https://www.rust-lang.org/tools/install) et [Tauri CLI](https://tauri.app/start/prerequisites/).

```bash
git clone https://github.com/NotSerigne/Accolade.git
cd Accolade
npm install

# Développement (hot-reload)
npm run tauri dev

# Build release (génère l'installateur dans src-tauri/target/release/bundle/)
npm run tauri build
```

---

## 🤝 Contribuer

Les contributions sont les bienvenues ! Pour signaler un bug ou proposer une fonctionnalité, ouvrez une [Issue](https://github.com/NotSerigne/Accolade/issues/new/choose) en utilisant le template adapté.

Pour soumettre du code :

1. Fork le projet
2. Créez une branche (`git checkout -b feature/ma-feature`)
3. Commitez vos changements
4. Ouvrez une Pull Request

---

## 📄 Licence

Distribué sous licence [MIT](LICENSE). © 2025 NotSerigne.

---

<div align="center">
  <sub>Accolade n'est pas affilié à Valve, Steam ou tout autre éditeur de jeux vidéo.</sub>
</div>
