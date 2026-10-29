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

**Vérification d'honnêteté - ce qui tourne aujourd'hui :** une bibliothèque Rust avec le conteneur de projet (`.eh3dproj`), les métadonnées d'un projet, les contrôles d'intégrité SHA-256, les types de télémétrie machine et les couleurs de la suite, plus la base de firmware que chaque nœud ESP32 de la suite copie (49 fichiers et l'outil qui garde les copies identiques). 29 tests tournent sur un ordinateur. Aucun programme de la suite n'utilise encore la bibliothèque, et rien ici n'a tourné sur une machine ni sur une carte.

---

> **État de la sûreté et de la sécurité :** cette bibliothèque n'est pas un système de sûreté et aucune machine n'a été pilotée avec elle. Le conteneur détecte un paquet endommagé ou modifié par accident, mais il n'est ni signé ni chiffré : il ne prouve pas qui a écrit un projet. Un paquet venant d'une autre personne est une entrée non fiable, ce qui explique que les noms, les tailles et le nombre d'entrées soient limités.

## 📋 En un coup d'œil

| | |
|---|---|
| Version | `0.0.1` |
| Maturité | Échafaudage |
| Tests | 29 |
| Pile technique | Rust (crate de bibliothèque) et C++17 (base de firmware) |
| Sur du matériel réel | Sans objet : une bibliothèque de données, rien ici ne parle à une machine ; la base de firmware n'a jamais tourné sur une carte |

**Aller à  :** [Présentation](#-présentation) · [Architecture](#-architecture) · [Démarrage rapide](#-démarrage-rapide) · [Tests](#-tests) · [Structure](#-structure-du-dépôt) · [Liés](#-projets-liés)

## 🚦 État du projet

Chaque ligne est une partie de ce dépôt et le feu dit où elle en est aujourd'hui. Elle est écrite d'après ce que le dépôt peut montrer (tests, compilations, exécutions), pas d'après ce qui est prévu.

🟢 Fait et vérifié · 🟡 Fonctionne, avec des limites connues · 🟠 À moitié fait ou pas encore vérifié · 🔴 Pas fait ou ne fonctionne pas

| Domaine | État | Ce que cela veut dire ici |
|---|---|---|
| Conteneur | 🟡 | `.eh3dproj` est un ZIP standard avec un enregistrement SHA-256 de chaque entrée ; il lit, écrit et refuse les entrées endommagées, modifiées, non listées ou dangereuses. Pour l'instant, seuls ses propres tests le lisent et l'écrivent. |
| Métadonnées | 🟡 | Les métadonnées d'un projet (unités, zéro, couches, outils, parcours, repères) sont validées et portent une version de format sémantique ; il n'y a pas encore de migration, car 1.0.0 est le premier format. |
| Télémétrie et thème | 🟢 | Axes, broche, états de la machine, codes d'alarme GRBL avec leur gravité et couleurs de la suite avec contrôle du contraste, le tout testé. |
| Tests | 🟢 | 29 tests réussissent (`cargo test`) : 15 dans les modules et 14 sur le conteneur. |
| Sécurité | 🟡 | Les noms dangereux, les entrées trop grosses ou trop nombreuses et toute entrée modifiée sont refusés ; le paquet n'est ni signé ni chiffré et n'a pas eu de revue indépendante. |
| Base de firmware | 🟠 | 49 fichiers partagés par les nœuds ESP32, gardés identiques par `tools/sync_firmware_base.py` ; testée seulement via EH3D-NODE-BUNGARD, le seul nœud qui l'utilise, et jamais exécutée sur une carte. |
| Utilisation par d'autres programmes | 🔴 | Aucun programme de la suite ne dépend encore du crate. |
| Documentation | 🟢 | README en huit langues, la description du format et le guide de la base de firmware ; la CI les compare au manifeste et au journal des modifications. |

## 🎯 Présentation

* Le **conteneur de projet** : un paquet `.eh3dproj` est une archive ZIP standard avec `project.json`, `integrity.json` et les dossiers `layers/`, `toolpaths/` et `vision/`. L'écriture est déterministe (le même contenu donne les mêmes octets) et passe par un fichier temporaire.
* Les **métadonnées** d'un projet : nom, identité, unités, zéro de la pièce, couches, outils, parcours et repères, avec une version de format sémantique (aujourd'hui 1.0.0). Une version majeure plus récente est refusée plutôt que comprise à moitié ; une plus ancienne exige une migration enregistrée, et il n'en existe pas encore.
* L'**enregistrement d'intégrité** : le SHA-256 et la taille de chaque entrée. Un paquet dont une entrée a changé, manque ou n'est pas listée ne s'ouvre pas.
* Les **limites** : les noms d'entrée qui pourraient sortir du dossier du paquet sont refusés, tout comme les entrées trop grosses ou trop nombreuses.
* Les **types de télémétrie** : axes, broche, états de la machine lus dans les mots d'état, et les neuf codes d'alarme GRBL avec leur gravité (avertissement ou critique ; un code inconnu compte comme critique).
* Le **thème** : les couleurs de la suite, avec un contrôle de contraste WCAG que les tests appliquent à chaque couleur de texte et d'état.
* La **base de firmware des nœuds** (`firmware_base/`, avec `tools/sync_firmware_base.py`) : la partie du firmware des nœuds ESP32 que partage chaque nœud EH3D - le document de réglages, le réseau, le serveur du panneau, utilisateurs et sessions, la mise à jour avec retour arrière, le journal, l'autoprotection et les tests qui vont avec - écrite une fois avec le nom du projet en marqueurs. Chaque nœud garde sa propre copie et se compile seul ; l'outil écrit la base dans chaque nœud et signale quand une copie a dérivé.

## 🧩 Rôle dans EH3D

EH3D-COMMON est la base de la suite : les bibliothèques et les programmes qui ouvrent ou enregistrent un projet sont prévus pour en dépendre. Aucun n'existe encore. La base de firmware des nœuds vit aussi ici : EH3D-NODE-BUNGARD est le premier nœud qui l'utilise.

## 🏛️ Architecture

<p align="center">
  <img src="images/architecture.svg" alt="Architecture de EH3D-COMMON : qui l'utilise, ce qu'il contient et de quoi il dépend" width="100%">
</p>

Comment les parties de ce dépôt s'articulent : qui l'utilise, ce qu'il contient et de quoi il dépend. L'image est dessinée à partir de `docs/architecture.json` et vérifiée par rapport au code.

## ✨ Fonctionnalités

* **Écriture déterministe et atomique :** le même projet donne toujours les mêmes octets et le fichier passe par un fichier temporaire, donc une écriture ratée ne laisse pas de paquet à moitié écrit.
* **Tout est contrôlé à la lecture :** d'abord les noms, le nombre et les tailles, puis l'enregistrement d'intégrité contre chaque entrée, puis les métadonnées ; le premier problème arrête la lecture avec une erreur typée.
* **Un format que tout outil ouvre :** ZIP simple (deflate seulement) et JSON simple, lisibles sans ce crate ; `docs/FORMAT.md` est le contrat écrit.
* **Les lecteurs restent compatibles :** la même version majeure est lue en ignorant ce qu'elle ne connaît pas ; une version majeure plus récente est refusée.
* **Des codes d'alarme jamais perdus :** un code que ce crate ne connaît pas est conservé comme `Unknown(n)` et traité comme critique.
* **Une base de firmware pour tous les nœuds :** on la modifie là où le besoin est apparu, on fait `import`, on fait `sync` dans les autres, et le `check` de la CI signale une copie qui a dérivé.

## 🔒 Sécurité et protection

Le conteneur lit des fichiers que d'autres personnes peuvent envoyer ; chaque contrôle ci-dessous est donc une défense contre un paquet hostile ou endommagé, et chacun a son test.

| Menace | Défense |
|---|---|
| Un nom qui sort du dossier du paquet (`../x`, `/etc/x`, `C:\x`) | `check_entry_name` refuse un nom vide, absolu ou trop long (plus de 240 octets), une barre oblique inverse, un deux-points, un caractère de contrôle et toute partie vide, `.` ou `..`, à l'écriture et de nouveau pour chaque nom trouvé dans un ZIP |
| Une bombe zip ou un paquet avec un nombre énorme d'entrées | au plus 10 000 entrées, 256 Mio par entrée et 1 Gio au total ; la lecture d'une entrée s'arrête un octet au-delà de sa limite |
| Une entrée modifiée en route (par accident ou exprès) | le SHA-256 et la taille de chaque entrée sont comparés à `integrity.json` ; une entrée différente, absente ou non listée fait refuser tout le paquet |
| Deux entrées de même nom | refusées comme doublon à la lecture, et une entrée remplacée à l'écriture ne garde qu'une copie |
| Un paquet d'un programme plus récent | une version majeure de format plus récente est refusée plutôt que comprise à moitié |
| Métadonnées incohérentes (nom vide, id d'outil répété, NaN, diamètre négatif, une couche hors de `layers/`) | `validate` s'exécute avant l'écriture et après la lecture ; un projet invalide ne peut pas être écrit |
| Aucun code unsafe | le crate est `#![forbid(unsafe_code)]` |

**Ce qui n'est pas défendu :** le paquet n'est ni signé ni chiffré. Qui peut réécrire les entrées peut aussi réécrire l'enregistrement d'intégrité : il détecte les dégâts et les accidents, pas un auteur. La bibliothèque n'a pas eu de revue indépendante. Signale une vulnérabilité en privé (voir [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

| Interface | Ce que c'est | Où |
|---|---|---|
| API Rust | `Package` (`new`, `add_entry`, `entry`, `entry_names`, `integrity`, `write`, `write_to_path`, `to_bytes`, `read`, `read_from_path`), `ProjectMetadata` (`new`, `validate`, `to_json`, `read_metadata`), les types de télémétrie et `theme` | `src/` |
| Format de fichier | `.eh3dproj` : un ZIP avec `project.json`, `integrity.json` et les dossiers de données `layers/`, `toolpaths/` et `vision/` | [docs/FORMAT.md](docs/FORMAT.md) |
| Erreurs | un seul enum `Error` : `Io`, `Json`, `Zip`, `UnsupportedFormat`, `MissingEntry`, `Integrity`, `UnsafePath`, `DuplicateEntry`, `TooLarge`, `InvalidMetadata` | `src/error.rs` |
| Outil de la base de firmware | `python tools/sync_firmware_base.py check`, `sync`, `import <nœud>` et `status` | [docs/FIRMWARE_BASE.md](docs/FIRMWARE_BASE.md) |

Ce dépôt n'a ni programme en ligne de commande, ni serveur, ni accès réseau.

## ⚙️ Configuration

La bibliothèque n'a pas de fichier de configuration : ses limites sont des constantes du code, listées ici pour qu'on sache ce qu'un paquet peut contenir. La version du format fait partie du fichier, ce n'est pas un réglage.

| Valeur | Limite | Où |
|---|---|---|
| Entrées dans un paquet | de 1 à 10 000 | `MAX_ENTRIES` dans `src/container.rs` |
| Taille d'une entrée | jusqu'à 256 Mio (268 435 456 octets) | `MAX_ENTRY_BYTES` |
| Taille de toutes les entrées ensemble | jusqu'à 1 Gio | `MAX_TOTAL_BYTES` |
| Longueur d'un nom d'entrée | de 1 à 240 octets | `check_entry_name` |
| Dossiers de données | seulement `layers/`, `toolpaths/`, `vision/` | `DATA_FOLDERS` dans `src/project.rs` |
| Version du format | 1.0.0 (la même version majeure est lue, une plus récente est refusée) | `FORMAT_VERSION` |
| Diamètre d'outil | fini et supérieur à 0 | `ProjectMetadata::validate` |

La base de firmware a son propre document de réglages (`node_settings.hpp`, `config_common.hpp`) ; ses valeurs appartiennent à chaque nœud et sont décrites dans le README d'EH3D-NODE-BUNGARD.

## 🚀 Démarrage rapide

Il faut une chaîne d'outils Rust stable (1.75 ou plus récente).

```rust
use eh3d_common::{Package, ProjectMetadata};

fn main() -> eh3d_common::Result<()> {
    let mut package = Package::new(ProjectMetadata::new("Test board"));
    package.add_entry("layers/top.gbr", b"G04 test*".to_vec())?;
    package.write_to_path(std::path::Path::new("board.eh3dproj"))?;
    let again = Package::read_from_path(std::path::Path::new("board.eh3dproj"))?;   // contrôle chaque entrée
    println!("{} data entries", again.entry_names().count());
    Ok(())
}
```

```bash
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
python -I tools/ci_validate.py
python -I tools/sync_firmware_base.py check
```

## 🧪 Tests

La suite compte 29 tests (`cargo test`, exécutés sous Windows et Linux dans la CI) : 15 dans les modules (les métadonnées et leur validation, les codes d'alarme et les états de la machine, les couleurs et leur contraste WCAG) et 14 sur le conteneur (aller-retour en mémoire et par fichier, mêmes octets pour le même contenu, l'enregistrement d'intégrité, une entrée modifiée, une taille modifiée, une entrée non listée, une entrée manquante, des métadonnées éditées, un fichier qui n'est pas un paquet, des noms d'entrée dangereux et hostiles, une entrée remplacée et un projet invalide). La CI exécute aussi `cargo fmt --check` et `cargo clippy` avec les avertissements en erreurs, `python -I tools/ci_validate.py` (manifeste, versions, journal des modifications et les huit README) et `tools/sync_firmware_base.py check` (les copies de la base de firmware). Les tests sur ordinateur de la base de firmware (`firmware_base/tests/`) s'exécutent dans EH3D-NODE-BUNGARD, pas ici.

## 🏗️ Compilation et publication

La version se trouve dans `Cargo.toml` et dans `eh3d.project.json`, et chaque version a son entrée dans [CHANGELOG.md](CHANGELOG.md). Les versions sont publiées sur GitHub ; la bibliothèque s'utilise comme crate Rust depuis ce dépôt et n'est pas sur crates.io.

## 📂 Structure du dépôt

<p align="center">
  <img src="images/repository-structure.svg" alt="Structure du dépôt EH3D-COMMON" width="100%">
</p>

<details>
<summary>Version texte de l'arborescence</summary>

```text
EH3D-COMMON/
├── src/
│   ├── container.rs                             le paquet : lecture, écriture, intégrité et limites
│   ├── project.rs                               les métadonnées et leur validation
│   ├── telemetry.rs                             axes, broche, états et codes d'alarme
│   └── theme.rs error.rs lib.rs                 les couleurs avec contraste, les erreurs et la racine du crate
├── crates/                                      crates partagés : vue, ZIP, fichiers machine, contrôleurs
├── firmware_base/
│   ├── core/                                    sans matériel : réglages, auth, plan réseau
│   ├── main/                                    code ESP-IDF : réseau, stockage, API du panneau
│   ├── tests/                                   les tests sur ordinateur de tout cela
│   ├── tools/                                   empaquette le panneau ; liste les lignes sdkconfig ignorées
│   └── manifest.json                            quel fichier va dans quel nœud
├── tests/                                       le paquet : allers-retours, entrées modifiées, manquantes et non listées, noms dangereux
├── docs/                                        le format, la base de firmware et les données des deux images
├── tools/                                       contrôle de la CI et du README, synchronisation de la base de firmware et outils d'images
├── images/                                      bannière et images d'Architecture et de Structure du dépôt
├── .github/                                     workflows de CI et de publication, modèles d'issues et de pull requests
├── eh3d.project.json                            manifeste : version, rôle, maturité et commande de compilation
├── Cargo.toml Cargo.lock                        le paquet Rust et ses dépendances figées
├── CHANGELOG.md                                 ce qui a changé dans chaque version
├── SECURITY.md SUPPORT.md                       signaler une vulnérabilité et poser des questions
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   contribuer, code de conduite et licence
```

</details>

## 🗺️ Feuille de route et limites connues

**Limites connues aujourd'hui**

* Aucun programme de la suite n'utilise encore la bibliothèque : le format n'a donc été lu et écrit que par ses propres tests.
* Le paquet n'est ni signé ni chiffré, et la bibliothèque n'a pas eu de revue indépendante.
* Il n'y a pas de migration, car 1.0.0 est le premier format.
* La base de firmware n'a été exercée que via EH3D-NODE-BUNGARD et n'a jamais tourné sur une carte ; un second nœud n'existe pas encore.
* La bibliothèque n'est pas publiée sur crates.io.

**Ensuite**

* Faire ouvrir et enregistrer un projet avec elle par le premier programme de la suite (EH3D-CORE-LIBS d'abord).
* Ajouter le second nœud à la base de firmware et vérifier que les copies restent identiques.
* Décider si les paquets ont besoin d'une signature facultative.

## 🌍 Langues

Le README et les textes existent en anglais, espagnol, français, italien, allemand, chinois, japonais et portugais européen.

## 🔗 Projets liés

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS) : les analyseurs (Excellon, Gerber) et la géométrie qui utiliseront ces types
* [EH3D-NODE-BUNGARD](https://github.com/JuanenRac/EH3D-NODE-BUNGARD) : le premier nœud qui garde une copie de la base de firmware
* [EH3D-SERVER](https://github.com/JuanenRac/EH3D-SERVER) : le centre de la famille ; il n'utilise pas encore le conteneur de projet
* [EH3D-DOCS](https://github.com/JuanenRac/EH3D-DOCS) : le centre de documentation de la suite et sa carte des projets
* [HYDRA-UMC](https://github.com/JuanenRac/HYDRA-UMC) : la plateforme de robotique dont la suite doit piloter les machines à l'avenir

## 📚 Documentation

* [Format de projet](docs/FORMAT.md) : les entrées, les métadonnées, l'enregistrement d'intégrité et les règles pour les lecteurs.
* [La base de firmware](docs/FIRMWARE_BASE.md) : ce qui est partagé, comment cela reste identique et comment ajouter un nœud.
* [Journal des modifications](CHANGELOG.md)

## 🤝 Contribuer et communauté

* Avant une pull request, exécute `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `python -I tools/ci_validate.py` et `python -I tools/sync_firmware_base.py check` : tout doit réussir. Voir [CONTRIBUTING.md](CONTRIBUTING.md) et le [code de conduite](CODE_OF_CONDUCT.md).
* Le README existe en huit langues : une modification du texte se fait dans les huit fichiers, et les deux images sont dessinées à partir de `docs/architecture.json` et `docs/structure.json` par `tools/make_architecture.py` et `tools/make_structure.py`.
* Une modification de la base de firmware se fait dans un nœud, s'importe avec `tools/sync_firmware_base.py import` et se synchronise dans les autres ; ne modifie jamais une copie à la main.
* Signalez une vulnérabilité en privé : [SECURITY.md](SECURITY.md). Questions, idées et rapports : electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTEUR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENCE

Logiciel : GPL-3.0-or-later. Voir [LICENSE](LICENSE). Les conceptions matérielles de la suite utilisent la CERN OHL-S.
