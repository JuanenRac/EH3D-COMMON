<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-COMMON banner" width="100%">
</p>

# 🧱 EH3D-COMMON

<p align="center">🇺🇸 <b>English</b> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### The shared data model of the EH3D Fabrication Suite: project container, metadata, integrity checks and machine telemetry

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Honesty check - what actually runs today:** a Rust library with the project container (`.eh3dproj`), the metadata of a project, SHA-256 integrity checks, the machine telemetry types and the colours of the suite, plus the firmware base that every ESP32 node of the suite copies (49 files and the tool that keeps the copies identical). 29 tests run on a computer. No program of the suite uses the library yet, and nothing here has run on a machine or on a board.

---

> **Safety and security status:** this library is not a safety system and no machine has been controlled with it. The container detects a package that was damaged or altered by accident, but it is not signed or encrypted: it does not prove who wrote a project. A package from another person is untrusted input, which is why the entry names, sizes and counts are limited.

## 📋 At a glance

| | |
|---|---|
| Version | `0.0.1` |
| Maturity | Scaffolding |
| Tests | 29 |
| Stack | Rust (library crate) and C++17 (firmware base) |
| On real hardware | Not applicable: a data library, nothing here talks to a machine; the firmware base has never run on a board |

**Jump to:** [Overview](#-overview) · [Architecture](#-architecture) · [Quick start](#-quick-start) · [Testing](#-testing) · [Structure](#-repository-structure) · [Related](#-related-projects)

## 🚦 Project status

Each line is one part of this repository and the light says how far it is today. It is written from what the repository can show (tests, builds, runs), not from what is planned.

🟢 Done and verified · 🟡 Works, with known limits · 🟠 Partly done or not verified yet · 🔴 Not done or not working

| Area | State | What it means here |
|---|---|---|
| Container | 🟡 | `.eh3dproj` is a standard ZIP with a SHA-256 record of every entry; it reads, writes, and refuses damaged, altered, unlisted and unsafe entries. Only its own tests read and write it so far. |
| Metadata | 🟡 | The metadata of a project (units, zero, layers, tools, toolpaths, fiducials) is validated and carries a semantic format version; there is no migration yet because 1.0.0 is the first format. |
| Telemetry and theme | 🟢 | Axes, spindle, machine states, the GRBL alarm codes with their severity and the colours of the suite with a contrast check, all tested. |
| Tests | 🟢 | 29 tests pass (`cargo test`): 15 in the modules and 14 on the container. |
| Security | 🟡 | Unsafe names, oversized or too many entries and any altered entry are refused; the package is not signed or encrypted and has had no independent review. |
| Firmware base | 🟠 | 49 files shared by the ESP32 nodes, kept identical by `tools/sync_firmware_base.py`; tested only through EH3D-NODE-BUNGARD, the one node that uses it, and never run on a board. |
| Use by other programs | 🔴 | No program of the suite depends on the crate yet. |
| Documentation | 🟢 | README in eight languages, the format description and the firmware-base guide; the CI compares them with the manifest and the changelog. |

## 🎯 Overview

* The **project container**: a `.eh3dproj` package is a standard ZIP archive with `project.json`, `integrity.json` and the folders `layers/`, `toolpaths/` and `vision/`. The writer is deterministic (the same content gives the same bytes) and writes through a temporary file.
* The **metadata** of a project: name, identity, units, zero of the workpiece, layers, tools, toolpaths and fiducials, with a semantic format version (today 1.0.0). A newer major version is refused instead of being half understood; an older one needs a registered migration, and none exists yet.
* The **integrity record**: the SHA-256 and the size of every entry. A package whose entry changed, is missing or is not listed does not open.
* The **limits**: entry names that could leave the package folder are refused, and so are oversized or too many entries.
* The **telemetry types**: axes, spindle, machine states read from status words, and the nine GRBL alarm codes with their severity (warning or critical; an unknown code counts as critical).
* The **theme**: the colours of the suite, with a WCAG contrast check that the tests run on every text and state colour.
* The **firmware base of the nodes** (`firmware_base/`, with `tools/sync_firmware_base.py`): the part of the ESP32 node firmware that every EH3D node shares - the settings document, the network, the panel's server, users and sessions, the update with rollback, the log, the self-protection and the tests that go with them - written once with the name of the project as placeholders. Each node keeps its own copy and builds on its own; the tool writes the base into every node and says when a copy has drifted.

## 🧩 Role in EH3D

EH3D-COMMON is the base of the suite: the libraries and the programs that open or save a project are planned to depend on it. None of them exists yet. The firmware base of the nodes also lives here: EH3D-NODE-BUNGARD is the first node that uses it.

## 🏛️ Architecture

<p align="center">
  <img src="images/architecture.svg" alt="EH3D-COMMON architecture: who uses it, what it holds and what it depends on" width="100%">
</p>

How the parts of this repository fit together: who uses it, what it holds and what it depends on. The picture is drawn from `docs/architecture.json` and checked against the code.

## ✨ Features

* **Deterministic, atomic writing:** the same project always gives the same bytes, and the file is written through a temporary file, so a failed write leaves no half package.
* **Everything checked on read:** names, count and sizes first, then the integrity record against every entry, then the metadata; the first problem stops the read with a typed error.
* **A format any tool can open:** plain ZIP (deflate only) and plain JSON, readable without this crate; `docs/FORMAT.md` is the written contract.
* **Readers stay compatible:** the same major version is read ignoring what it does not know; a newer major version is refused.
* **Alarm codes that never get lost:** a code this crate does not know is kept as `Unknown(n)` and treated as critical.
* **One firmware base for all nodes:** edit it where the need appeared, `import` it, `sync` it into the others, and `check` in the CI says when a copy drifted.

## 🔒 Security & Safety

The container reads files that other people may send, so every check below is a defense against a hostile or damaged package, and each one has a test.

| Threat | Defense |
|---|---|
| A name that climbs out of the package folder (`../x`, `/etc/x`, `C:\x`) | `check_entry_name` refuses an empty, absolute or over-long name (more than 240 bytes), a backslash, a colon, a control character, and any empty, `.` or `..` part, when writing and again for every name found in a ZIP |
| A zip bomb or a package with a huge number of entries | at most 10,000 entries, 256 MiB per entry and 1 GiB in total; the read of an entry stops one byte over its limit |
| An entry changed on the way (by accident or on purpose) | the SHA-256 and the size of every entry are compared with `integrity.json`; a different, missing or unlisted entry refuses the whole package |
| Two entries with the same name | refused as a duplicate on reading, and a replaced entry on writing keeps one copy |
| A package from a newer program | a newer major format version is refused instead of being half understood |
| Inconsistent metadata (empty name, repeated tool id, NaN, negative diameter, a layer outside `layers/`) | `validate` runs before writing and after reading; an invalid project cannot be written |
| No unsafe code | the crate is `#![forbid(unsafe_code)]` |

**What is not defended:** the package is not signed or encrypted. Whoever can rewrite the entries can also rewrite the integrity record, so it detects damage and accident, not an author. The library has had no independent review. Report a vulnerability privately (see [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

| Interface | What it is | Where |
|---|---|---|
| Rust API | `Package` (`new`, `add_entry`, `entry`, `entry_names`, `integrity`, `write`, `write_to_path`, `to_bytes`, `read`, `read_from_path`), `ProjectMetadata` (`new`, `validate`, `to_json`, `read_metadata`), the telemetry types and `theme` | `src/` |
| File format | `.eh3dproj`: a ZIP with `project.json`, `integrity.json` and the data folders `layers/`, `toolpaths/` and `vision/` | [docs/FORMAT.md](docs/FORMAT.md) |
| Errors | one `Error` enum: `Io`, `Json`, `Zip`, `UnsupportedFormat`, `MissingEntry`, `Integrity`, `UnsafePath`, `DuplicateEntry`, `TooLarge`, `InvalidMetadata` | `src/error.rs` |
| Firmware base tool | `python tools/sync_firmware_base.py check`, `sync`, `import <node>` and `status` | [docs/FIRMWARE_BASE.md](docs/FIRMWARE_BASE.md) |

There is no command-line program, no server and no network access in this repository.

## ⚙️ Configuration

The library has no configuration file: its limits are constants of the code, listed here so that a reader knows what a package may contain. The format version is part of the file, not a setting.

| Value | Limit | Where |
|---|---|---|
| Entries in a package | 1 to 10,000 | `MAX_ENTRIES` in `src/container.rs` |
| Size of one entry | up to 256 MiB (268,435,456 bytes) | `MAX_ENTRY_BYTES` |
| Size of all entries together | up to 1 GiB | `MAX_TOTAL_BYTES` |
| Length of an entry name | 1 to 240 bytes | `check_entry_name` |
| Data folders | `layers/`, `toolpaths/`, `vision/` only | `DATA_FOLDERS` in `src/project.rs` |
| Format version | 1.0.0 (same major is read, newer major refused) | `FORMAT_VERSION` |
| Tool diameter | finite and greater than 0 | `ProjectMetadata::validate` |

The firmware base has its own settings document (`node_settings.hpp`, `config_common.hpp`); its values belong to each node and are described in the README of EH3D-NODE-BUNGARD.

## 🚀 Quick start

You need a stable Rust toolchain (1.75 or newer).

```rust
use eh3d_common::{Package, ProjectMetadata};

fn main() -> eh3d_common::Result<()> {
    let mut package = Package::new(ProjectMetadata::new("Test board"));
    package.add_entry("layers/top.gbr", b"G04 test*".to_vec())?;
    package.write_to_path(std::path::Path::new("board.eh3dproj"))?;
    let again = Package::read_from_path(std::path::Path::new("board.eh3dproj"))?;   // checks every entry
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

## 🧪 Testing

The suite has 29 tests (`cargo test`, run on Windows and Linux in the CI): 15 inside the modules (the metadata and its validation, the alarm codes and machine states, the colours and their WCAG contrast) and 14 on the container (round trips through memory and through a file, the same bytes for the same content, the integrity record, a changed entry, a changed size, an unlisted entry, a missing entry, edited metadata, a file that is not a package, unsafe and hostile entry names, a replaced entry and an invalid project). The CI also runs `cargo fmt --check` and `cargo clippy` with warnings as errors, `python -I tools/ci_validate.py` (manifest, versions, changelog and the eight READMEs) and `tools/sync_firmware_base.py check` (the copies of the firmware base). The host tests of the firmware base (`firmware_base/tests/`) run inside EH3D-NODE-BUNGARD, not here.

## 🏗️ Build & Release

The version lives in `Cargo.toml` and in `eh3d.project.json`, and each version has its entry in [CHANGELOG.md](CHANGELOG.md). Releases are published on GitHub; the library is consumed as a Rust crate from this repository, and it is not on crates.io.

## 📂 Repository Structure

<p align="center">
  <img src="images/repository-structure.svg" alt="Repository structure of EH3D-COMMON" width="100%">
</p>

<details>
<summary>Text version of the tree</summary>

```text
EH3D-COMMON/
├── src/
│   ├── container.rs                             the package: reading, writing, integrity and limits
│   ├── project.rs                               the metadata and its validation
│   ├── telemetry.rs                             axes, spindle, states and alarm codes
│   └── theme.rs error.rs lib.rs                 the colours with contrast, the errors and the crate root
├── crates/                                      shared crates: the job view, the ZIP archives and the machine controllers
├── firmware_base/
│   ├── core/                                    no hardware: settings, auth, network plan
│   ├── main/                                    ESP-IDF code: network, store, panel API
│   ├── tests/                                   the host tests of all of the above
│   ├── tools/                                   pack the panel; list ignored sdkconfig lines
│   └── manifest.json                            which file goes to which node
├── tests/                                       the package: round trips, changed, missing and unlisted entries, unsafe names
├── docs/                                        the format, the firmware base and the data of the two pictures
├── tools/                                       CI check, README check, firmware-base sync and the picture tools
├── images/                                      banner and the Architecture and Repository Structure pictures
├── .github/                                     CI and release workflows, issue and pull-request templates
├── eh3d.project.json                            manifest: version, role, maturity and build command
├── Cargo.toml Cargo.lock                        the Rust package and its locked dependencies
├── CHANGELOG.md                                 what changed in each version
├── SECURITY.md SUPPORT.md                       how to report a vulnerability and where to ask
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   how to contribute, the code of conduct and the licence
```

</details>

## 🗺️ Roadmap & Known limits

**Known limits today**

* No program of the suite uses the library yet, so the format has only been read and written by its own tests.
* The package is not signed or encrypted, and the library has had no independent review.
* There is no migration, because 1.0.0 is the first format.
* The firmware base has only been exercised through EH3D-NODE-BUNGARD and has never run on a board; a second node does not exist yet.
* The library is not published on crates.io.

**Next**

* Make the first program of the suite open and save a project with it (EH3D-CORE-LIBS first).
* Add the second node to the firmware base and check that the copies stay identical.
* Decide whether packages need an optional signature.

## 🌍 Languages

README and texts exist in English, Spanish, French, Italian, German, Chinese, Japanese and European Portuguese.

## 🔗 Related Projects

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): the parsers (Excellon, Gerber) and geometry that will use these types
* [EH3D-NODE-BUNGARD](https://github.com/JuanenRac/EH3D-NODE-BUNGARD): the first node that keeps a copy of the firmware base
* [EH3D-SERVER](https://github.com/JuanenRac/EH3D-SERVER): the center of the family; it does not use the project container yet
* [EH3D-DOCS](https://github.com/JuanenRac/EH3D-DOCS): the documentation hub of the suite and its map of the projects
* [HYDRA-UMC](https://github.com/JuanenRac/HYDRA-UMC): the robotics platform whose machines the suite is meant to drive in the future

## 📚 Documentation

* [Project format](docs/FORMAT.md): the entries, the metadata, the integrity record and the rules for readers.
* [The firmware base](docs/FIRMWARE_BASE.md): what is shared, how it is kept identical and how to add a node.
* [Changelog](CHANGELOG.md)

## 🤝 Contributing & Community

* Before a pull request run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `python -I tools/ci_validate.py` and `python -I tools/sync_firmware_base.py check`: all must pass. See [CONTRIBUTING.md](CONTRIBUTING.md) and the [code of conduct](CODE_OF_CONDUCT.md).
* The README exists in eight languages: a change to the text is made in all eight files, and the two pictures are drawn from `docs/architecture.json` and `docs/structure.json` by `tools/make_architecture.py` and `tools/make_structure.py`.
* A change to the firmware base is made in a node, imported with `tools/sync_firmware_base.py import`, and synced into the others; never edit one copy by hand.
* Report a vulnerability privately: [SECURITY.md](SECURITY.md). Questions, ideas and reports: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTHOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENSE

Software: GPL-3.0-or-later. See [LICENSE](LICENSE). Hardware designs of the suite use CERN OHL-S.
