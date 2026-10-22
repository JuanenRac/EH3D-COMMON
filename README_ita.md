<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-COMMON banner" width="100%">
</p>

# 🧱 EH3D-COMMON

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | 🇮🇹 <b>Italiano</b> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Il modello di dati condiviso della EH3D Fabrication Suite: contenitore di progetto, metadati, controlli di integrità e telemetria della macchina

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Verifica di onestà - cosa funziona oggi:** una libreria Rust con il contenitore di progetto (`.eh3dproj`), i metadati di un progetto, controlli di integrità SHA-256 e i tipi di telemetria della macchina. Tutto viene eseguito nei test sul computer di sviluppo; nessun programma della suite la usa ancora e nulla è stato eseguito su una macchina.

---

> **Stato di sicurezza funzionale e informatica:** questa libreria non è un sistema di sicurezza e nessuna macchina è stata controllata con essa. Il contenitore rileva un pacchetto danneggiato o alterato per errore, ma non è firmato né cifrato: non dimostra chi ha scritto un progetto. Un pacchetto di un'altra persona è un input non attendibile, per questo i nomi delle voci, le dimensioni e il numero di voci sono limitati.

## 📋 In breve

| | |
|---|---|
| Versione | `0.0.1` |
| Maturità | Impalcatura |
| Test | 29 |
| Stack tecnologico | Rust |
| Su hardware reale | Non applicabile: una libreria di dati, nulla qui parla con una macchina |

## 🎯 Panoramica

* Il **contenitore di progetto**: un pacchetto `.eh3dproj` è un archivio ZIP standard con `project.json`, `integrity.json` e le cartelle `layers/`, `toolpaths/` e `vision/`.
* I **metadati** di un progetto: nome, identità, unità, zero del pezzo, livelli, utensili, percorsi utensile e fiduciali, con una versione di formato semantica. Una versione maggiore più recente viene rifiutata invece di essere capita a metà.
* Il **registro di integrità**: lo SHA-256 e la dimensione di ogni voce. Un pacchetto in cui una voce è cambiata, manca o non è elencata non si apre.
* I **limiti**: i nomi di voce che potrebbero uscire dalla cartella del pacchetto vengono rifiutati, come le voci troppo grandi o troppo numerose.
* I **tipi di telemetria**: assi, mandrino, stati della macchina e codici di allarme GRBL con la loro gravità.
* Il **tema**: i colori della suite, con un controllo del contrasto.

## 🧩 Ruolo in EH3D

EH3D-COMMON è la base della suite: le librerie e i programmi che aprono o salvano un progetto sono previsti come dipendenti da essa. Nessuno esiste ancora.

## 📂 Struttura del repository

```text
EH3D-COMMON/
├── src/          container.rs, project.rs, telemetry.rs, theme.rs, error.rs
├── tests/        the package: round trips, changed, missing and unlisted entries, unsafe names
├── docs/         FORMAT.md: the .eh3dproj format
├── tools/        ci_validate.py: manifest, versions, changelog and README checks
├── images/       banner
└── eh3d.project.json   the manifest of the project
```

## 🛠️ COMPILARE ED ESEGUIRE

Serve una toolchain Rust stabile.

```bash
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
python -I tools/ci_validate.py
```

## 🔗 Progetti correlati

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): previsto: parser e motori geometrici che useranno questi tipi
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): previsto: il centro che aprirà e salverà i progetti
* [HYDRA-UMC](https://github.com/JuanenRac/HYDRA-UMC): la piattaforma robotica le cui macchine la suite dovrà pilotare in futuro

## 📚 Documentazione e community

* [Formato del progetto](docs/FORMAT.md): le voci, i metadati, il registro di integrità e le regole per chi legge.
* [Registro delle modifiche](CHANGELOG.md)
* [Contribuire](CONTRIBUTING.md)
* [Politica di sicurezza](SECURITY.md)
* [Supporto](SUPPORT.md)

## 👤 AUTORE

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENZA

Software: GPL-3.0-or-later. Vedi [LICENSE](LICENSE). I progetti hardware della suite usano la CERN OHL-S.
