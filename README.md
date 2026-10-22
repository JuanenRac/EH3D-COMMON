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

**Honesty check - what actually runs today:** a Rust library with the project container (`.eh3dproj`), the metadata of a project, SHA-256 integrity checks and the machine telemetry types. All of it runs in tests on the development computer; no program of the suite uses it yet and nothing has been run on a machine.

---

> **Safety and security status:** this library is not a safety system and no machine has been controlled with it. The container detects a package that was damaged or altered by accident, but it is not signed or encrypted: it does not prove who wrote a project. A package from another person is untrusted input, which is why the entry names, sizes and counts are limited.

## 📋 At a glance

| | |
|---|---|
| Version | `0.0.1` |
| Maturity | Scaffolding |
| Tests | 29 |
| Stack | Rust |
| On real hardware | Not applicable: a data library, nothing here talks to a machine |

## 🎯 Overview

* The **project container**: a `.eh3dproj` package is a standard ZIP archive with `project.json`, `integrity.json` and the folders `layers/`, `toolpaths/` and `vision/`.
* The **metadata** of a project: name, identity, units, zero of the workpiece, layers, tools, toolpaths and fiducials, with a semantic format version. A newer major version is refused instead of being half understood.
* The **integrity record**: the SHA-256 and the size of every entry. A package whose entry changed, is missing or is not listed does not open.
* The **limits**: entry names that could leave the package folder are refused, and so are oversized or too many entries.
* The **telemetry types**: axes, spindle, machine states and GRBL alarm codes with their severity.
* The **theme**: the colours of the suite, with a contrast check.

## 🧩 Role in EH3D

EH3D-COMMON is the base of the suite: the libraries and the programs that open or save a project are planned to depend on it. None of them exists yet.

## 📂 Repository Structure

```text
EH3D-COMMON/
├── src/          container.rs, project.rs, telemetry.rs, theme.rs, error.rs
├── tests/        the package: round trips, changed, missing and unlisted entries, unsafe names
├── docs/         FORMAT.md: the .eh3dproj format
├── tools/        ci_validate.py: manifest, versions, changelog and README checks
├── images/       banner
└── eh3d.project.json   the manifest of the project
```

## 🛠️ BUILD & RUN

You need a stable Rust toolchain.

```bash
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
python -I tools/ci_validate.py
```

## 🔗 Related Projects

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): planned: parsers and geometry engines that will use these types
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): planned: the hub that will open and save projects
* [HYDRA-UMC](https://github.com/JuanenRac/HYDRA-UMC): the robotics platform whose machines the suite is meant to drive in the future

## 📚 Documentation & Community

* [Project format](docs/FORMAT.md): the entries, the metadata, the integrity record and the rules for readers.
* [Changelog](CHANGELOG.md)
* [Contributing](CONTRIBUTING.md)
* [Security policy](SECURITY.md)
* [Support](SUPPORT.md)

## 👤 AUTHOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENSE

Software: GPL-3.0-or-later. See [LICENSE](LICENSE). Hardware designs of the suite use CERN OHL-S.
