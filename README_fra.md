<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-COMMON banner" width="100%">
</p>

# 🧱 EH3D-COMMON

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | 🇫🇷 <b>Français</b> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Le modèle de données partagé de l'EH3D Fabrication Suite : conteneur de projet, métadonnées, contrôles d'intégrité et télémétrie machine

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Vérification d'honnêteté - ce qui tourne aujourd'hui :** une bibliothèque Rust avec le conteneur de projet (`.eh3dproj`), les métadonnées d'un projet, des contrôles d'intégrité SHA-256 et les types de télémétrie machine. Tout s'exécute dans des tests sur l'ordinateur de développement ; aucun programme de la suite ne l'utilise encore et rien n'a été exécuté sur une machine.

---

> **État de la sûreté et de la sécurité :** cette bibliothèque n'est pas un système de sûreté et aucune machine n'a été pilotée avec elle. Le conteneur détecte un paquet endommagé ou modifié par accident, mais il n'est ni signé ni chiffré : il ne prouve pas qui a écrit un projet. Un paquet venant d'une autre personne est une entrée non fiable, c'est pourquoi les noms d'entrée, les tailles et le nombre d'entrées sont limités.

## 📋 En un coup d'œil

| | |
|---|---|
| Version | `0.0.1` |
| Maturité | Échafaudage |
| Tests | 29 |
| Pile technique | Rust |
| Sur du matériel réel | Sans objet : une bibliothèque de données, rien ici ne parle à une machine |

## 🎯 Vue d'ensemble

* Le **conteneur de projet** : un paquet `.eh3dproj` est une archive ZIP standard avec `project.json`, `integrity.json` et les dossiers `layers/`, `toolpaths/` et `vision/`.
* Les **métadonnées** d'un projet : nom, identité, unités, zéro de la pièce, couches, outils, trajectoires et repères fiduciaires, avec une version de format sémantique. Une version majeure plus récente est refusée au lieu d'être comprise à moitié.
* Le **registre d'intégrité** : le SHA-256 et la taille de chaque entrée. Un paquet dont une entrée a changé, manque ou n'est pas listée ne s'ouvre pas.
* Les **limites** : les noms d'entrée qui pourraient sortir du dossier du paquet sont refusés, ainsi que les entrées trop grandes ou trop nombreuses.
* Les **types de télémétrie** : axes, broche, états de la machine et codes d'alarme GRBL avec leur gravité.
* Le **thème** : les couleurs de la suite, avec un contrôle de contraste.

## 🧩 Rôle dans EH3D

EH3D-COMMON est la base de la suite : les bibliothèques et les programmes qui ouvrent ou enregistrent un projet sont prévus pour en dépendre. Aucun n'existe encore.

## 📂 Structure du dépôt

```text
EH3D-COMMON/
├── src/          container.rs, project.rs, telemetry.rs, theme.rs, error.rs
├── tests/        the package: round trips, changed, missing and unlisted entries, unsafe names
├── docs/         FORMAT.md: the .eh3dproj format
├── tools/        ci_validate.py: manifest, versions, changelog and README checks
├── images/       banner
└── eh3d.project.json   the manifest of the project
```

## 🛠️ COMPILER ET EXÉCUTER

Il faut une chaîne d'outils Rust stable.

```bash
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
python -I tools/ci_validate.py
```

## 🔗 Projets liés

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): prévu : analyseurs et moteurs de géométrie qui utiliseront ces types
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): prévu : le centre qui ouvrira et enregistrera les projets
* [HYDRA-UMC](https://github.com/JuanenRac/HYDRA-UMC): la plateforme de robotique dont la suite doit piloter les machines à l'avenir

## 📚 Documentation et communauté

* [Format de projet](docs/FORMAT.md) : les entrées, les métadonnées, le registre d'intégrité et les règles pour les lecteurs.
* [Journal des modifications](CHANGELOG.md)
* [Contribuer](CONTRIBUTING.md)
* [Politique de sécurité](SECURITY.md)
* [Assistance](SUPPORT.md)

## 👤 AUTEUR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENCE

Logiciel : GPL-3.0-or-later. Voir [LICENSE](LICENSE). Les conceptions matérielles de la suite utilisent la CERN OHL-S.
