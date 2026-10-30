<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-COMMON banner" width="100%">
</p>

# 🧱 EH3D-COMMON

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | 🇩🇪 <b>Deutsch</b> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Das gemeinsame Datenmodell der EH3D Fabrication Suite: Projektcontainer, Metadaten, Integritätsprüfungen und Maschinentelemetrie

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.4-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Ehrlichkeitscheck - was heute tatsächlich läuft:** eine Rust-Bibliothek mit dem Projektcontainer (`.eh3dproj`), den Metadaten eines Projekts, SHA-256-Integritätsprüfungen, den Typen der Maschinentelemetrie und den Farben der Suite, dazu die Firmware-Basis, die jeder ESP32-Knoten der Suite kopiert (70 Dateien und das Werkzeug, das die Kopien identisch hält). 115 Tests laufen auf einem Computer. Die Bohr- und Fräsprogramme und die moderne Oberfläche lesen ihre Dateien und zeichnen ihre Aufträge mit den gemeinsamen Crates; nichts davon lief auf einer Maschine oder einer Platine.

---

> **Stand von Sicherheit und IT-Sicherheit:** Diese Bibliothek ist kein Sicherheitssystem (Safety), und mit ihr wurde keine Maschine gesteuert. Der Container erkennt ein versehentlich beschädigtes oder verändertes Paket, ist aber weder signiert noch verschlüsselt: Er beweist nicht, wer ein Projekt geschrieben hat. Ein Paket von einer anderen Person ist nicht vertrauenswürdige Eingabe, darum sind Eintragsnamen, Größen und Anzahlen begrenzt.

## 📋 Auf einen Blick

| | |
|---|---|
| Version | `0.0.4` |
| Reifegrad | Gerüst |
| Tests | 115 |
| Technologie-Stack | Rust (Bibliotheks-Crate) und C++17 (Firmware-Basis) |
| Auf echter Hardware | Nicht zutreffend: eine Datenbibliothek, nichts hier spricht mit einer Maschine; die Firmware-Basis lief noch nie auf einer Platine |

**Springe zu:** [Überblick](#-überblick) · [Architektur](#-architektur) · [Schnellstart](#-schnellstart) · [Tests](#-tests) · [Struktur](#-struktur-des-repositorys) · [Verwandte](#-verwandte-projekte)

## 🚦 Projektstatus

Jede Zeile ist ein Teil dieses Repositorys, und die Ampel sagt, wie weit er heute ist. Sie beruht auf dem, was das Repository zeigen kann (Tests, Builds, Läufe), nicht auf dem, was geplant ist.

🟢 Fertig und überprüft · 🟡 Funktioniert, mit bekannten Grenzen · 🟠 Teilweise fertig oder noch nicht überprüft · 🔴 Nicht gemacht oder funktioniert nicht

| Bereich | Stand | Was das hier bedeutet |
|---|---|---|
| Container | 🟡 | `.eh3dproj` ist ein Standard-ZIP mit einem SHA-256-Eintrag je Eintrag; es liest, schreibt und weist beschädigte, veränderte, nicht gelistete und unsichere Einträge ab. Bisher lesen und schreiben es nur die eigenen Tests. |
| Metadaten | 🟡 | Die Metadaten eines Projekts (Einheiten, Nullpunkt, Ebenen, Werkzeuge, Werkzeugwege, Passmarken) werden geprüft und tragen eine semantische Formatversion; eine Migration gibt es noch nicht, da 1.0.0 das erste Format ist. |
| Telemetrie und Theme | 🟢 | Achsen, Spindel, Maschinenzustände, die GRBL-Alarmcodes mit ihrer Schwere und die Farben der Suite mit Kontrastprüfung, alles getestet. |
| Tests | 🟢 | 115 Tests bestehen (`cargo test --workspace`): 16 in den Modulen und 14 am Container des Hauptcrates sowie 85 in den acht gemeinsamen Crates. |
| Sicherheit | 🟡 | Unsichere Namen, zu große oder zu viele Einträge und jeder veränderte Eintrag werden abgewiesen; das Paket ist weder signiert noch verschlüsselt und wurde nicht unabhängig geprüft. |
| Firmware-Basis | 🟠 | 49 Dateien, die die ESP32-Knoten teilen und die `tools/sync_firmware_base.py` identisch hält; nur über EH3D-NODE-BUNGARD getestet, den einzigen Knoten, der sie nutzt, und nie auf einer Platine gelaufen. |
| Nutzung durch andere Programme | 🟡 | Die Crates des Workspace werden von EH3D-DRILLPRO, EH3D-ROUTEPRO, EH3D-BUNGARD-GUI, EH3D-MOTIONCNC und EH3D-SERVER benutzt (per Pfad, die Repositories liegen nebeneinander); den Projektcontainer selbst benutzt noch kein Programm. |
| Dokumentation | 🟢 | README in acht Sprachen, die Formatbeschreibung und der Leitfaden zur Firmware-Basis; die CI vergleicht sie mit Manifest und Änderungsprotokoll. |

## 🎯 Überblick

* Der **Projektcontainer**: Ein `.eh3dproj`-Paket ist ein Standard-ZIP-Archiv mit `project.json`, `integrity.json` und den Ordnern `layers/`, `toolpaths/` und `vision/`. Der Schreiber ist deterministisch (gleicher Inhalt ergibt gleiche Bytes) und schreibt über eine temporäre Datei.
* Die **Metadaten** eines Projekts: Name, Identität, Einheiten, Werkstücknullpunkt, Ebenen, Werkzeuge, Werkzeugwege und Passmarken, mit einer semantischen Formatversion (heute 1.0.0). Eine neuere Hauptversion wird abgewiesen, statt halb verstanden zu werden; eine ältere braucht eine registrierte Migration, und es gibt noch keine.
* Der **Integritätseintrag**: SHA-256 und Größe jedes Eintrags. Ein Paket, in dem ein Eintrag verändert wurde, fehlt oder nicht gelistet ist, öffnet sich nicht.
* Die **Grenzen**: Eintragsnamen, die den Paketordner verlassen könnten, werden abgewiesen, ebenso zu große oder zu viele Einträge.
* Die **Telemetrietypen**: Achsen, Spindel, aus Statuswörtern gelesene Maschinenzustände und die neun GRBL-Alarmcodes mit ihrer Schwere (Warnung oder kritisch; ein unbekannter Code gilt als kritisch).
* Das **Theme**: die Farben der Suite, mit einer WCAG-Kontrastprüfung, die die Tests für jede Text- und Zustandsfarbe ausführen.
* Die **Firmware-Basis der Knoten** (`firmware_base/`, mit `tools/sync_firmware_base.py`): der Teil der ESP32-Knoten-Firmware, den jeder EH3D-Knoten teilt - das Einstellungsdokument, das Netzwerk, der Server des Panels, Benutzer und Sitzungen, das Update mit Rückfall, das Protokoll, der Selbstschutz und die zugehörigen Tests - einmal geschrieben, mit dem Projektnamen als Platzhalter. Jeder Knoten behält seine eigene Kopie und baut für sich; das Werkzeug schreibt die Basis in jeden Knoten und meldet, wenn eine Kopie abgewichen ist.
* Die **gemeinsamen Crates** des Workspace: `eh3d-view` (der Auftrag auf dem Tisch, elf Themen, die Wörter der Oberfläche in acht Sprachen), `eh3d-archive` (Aufträge in ZIP-Archiven), `eh3d-bungard-formats` (die Dateien Excellon, HP-GL, Werkzeugtabellen und Konfiguration, die Gerber- und DXF-Umrisse), `eh3d-controller` (die Schnittstelle der Maschinensteuerungen, GRBL und eine simulierte), `eh3d-filedialog` (die Fenster Öffnen und Speichern) und `eh3d-fleet` (die Maschinen und Stationen des Servers).
* **Die Höhe des Tisches und der Knoten am Bohrkopf:** `eh3d-levelling` hält die Höhenkarte des Maschinentisches (ein Raster gemessener Punkte, die Fläche dazwischen durch Interpolation, Neigung und Spanne, die Korrektur des Z eines Auftrags und eine Textform zum Aufbewahren der Karte), und `eh3d-aux` ist der Client von EH3D-NODE-BUNGARD-AUX (sein Status, seine Messung des Tisches, seine Kamera als MJPEG, die Anfragen mit dem Token und ein kleiner HTTP-Client).

## 🧩 Rolle in EH3D

EH3D-COMMON ist die Basis der Suite. Die Crates seines Workspace (die Ansicht und die Themen, die ZIP-Archive, die Dateien der Bungard-Maschinen, die Maschinensteuerungen, die Dateifenster und die Flotte des Servers) benutzen heute EH3D-DRILLPRO, EH3D-ROUTEPRO, EH3D-BUNGARD-GUI, EH3D-MOTIONCNC und EH3D-SERVER; die Bibliotheken und Programme, die ein Projekt öffnen oder speichern, sollen vom Container abhängen, und keines davon existiert noch. Auch die Firmware-Basis der Knoten liegt hier: EH3D-NODE-BUNGARD ist der erste Knoten, der sie benutzt.

## 🏛️ Architektur

<p align="center">
  <img src="images/architecture.svg" alt="Architektur von EH3D-COMMON: wer es nutzt, was es enthält und wovon es abhängt" width="100%">
</p>

Wie die Teile dieses Repositorys zusammenpassen: wer es nutzt, was es enthält und wovon es abhängt. Das Bild wird aus `docs/architecture.json` gezeichnet und gegen den Code geprüft.

## ✨ Funktionen

* **Deterministisches, atomares Schreiben:** Dasselbe Projekt ergibt immer dieselben Bytes, und die Datei wird über eine temporäre Datei geschrieben, sodass ein fehlgeschlagener Schreibvorgang kein halbes Paket hinterlässt.
* **Alles wird beim Lesen geprüft:** zuerst Namen, Anzahl und Größen, dann der Integritätseintrag gegen jeden Eintrag, dann die Metadaten; das erste Problem beendet das Lesen mit einem typisierten Fehler.
* **Ein Format, das jedes Werkzeug öffnet:** einfaches ZIP (nur Deflate) und einfaches JSON, ohne dieses Crate lesbar; `docs/FORMAT.md` ist der geschriebene Vertrag.
* **Leser bleiben kompatibel:** Dieselbe Hauptversion wird gelesen, wobei Unbekanntes ignoriert wird; eine neuere Hauptversion wird abgewiesen.
* **Alarmcodes gehen nicht verloren:** Ein Code, den dieses Crate nicht kennt, bleibt als `Unknown(n)` erhalten und gilt als kritisch.
* **Eine Firmware-Basis für alle Knoten:** dort ändern, wo der Bedarf entstand, `import`, per `sync` in die anderen schreiben, und das `check` der CI meldet, wenn eine Kopie abweicht.

## 🔒 Sicherheit und Schutz

Der Container liest Dateien, die andere Personen schicken können; jede Prüfung unten ist daher eine Abwehr gegen ein feindliches oder beschädigtes Paket, und jede hat einen Test.

| Bedrohung | Abwehr |
|---|---|
| Ein Name, der aus dem Paketordner herausklettert (`../x`, `/etc/x`, `C:\x`) | `check_entry_name` weist einen leeren, absoluten oder zu langen Namen (über 240 Bytes), einen Backslash, einen Doppelpunkt, ein Steuerzeichen und jeden leeren, `.`- oder `..`-Teil ab, beim Schreiben und erneut für jeden in einem ZIP gefundenen Namen |
| Eine Zip-Bombe oder ein Paket mit riesiger Eintragszahl | höchstens 10.000 Einträge, 256 MiB je Eintrag und 1 GiB insgesamt; das Lesen eines Eintrags stoppt ein Byte über seiner Grenze |
| Ein unterwegs veränderter Eintrag (aus Versehen oder absichtlich) | SHA-256 und Größe jedes Eintrags werden mit `integrity.json` verglichen; ein abweichender, fehlender oder nicht gelisteter Eintrag lehnt das ganze Paket ab |
| Zwei Einträge mit demselben Namen | beim Lesen als Duplikat abgewiesen, und ein beim Schreiben ersetzter Eintrag behält nur eine Kopie |
| Ein Paket eines neueren Programms | eine neuere Hauptversion des Formats wird abgewiesen, statt halb verstanden zu werden |
| Inkonsistente Metadaten (leerer Name, doppelte Werkzeug-ID, NaN, negativer Durchmesser, eine Ebene außerhalb von `layers/`) | `validate` läuft vor dem Schreiben und nach dem Lesen; ein ungültiges Projekt lässt sich nicht schreiben |
| Kein unsicherer Code | das Crate ist `#![forbid(unsafe_code)]` |

**Was nicht abgewehrt wird:** Das Paket ist weder signiert noch verschlüsselt. Wer die Einträge neu schreiben kann, kann auch den Integritätseintrag neu schreiben; er erkennt also Schäden und Versehen, keinen Urheber. Die Bibliothek wurde nicht unabhängig geprüft. Melde eine Schwachstelle privat (siehe [SECURITY.md](SECURITY.md)).

## 🔌 Schnittstellen

| Schnittstelle | Was es ist | Wo |
|---|---|---|
| Rust-API | `Package` (`new`, `add_entry`, `entry`, `entry_names`, `integrity`, `write`, `write_to_path`, `to_bytes`, `read`, `read_from_path`), `ProjectMetadata` (`new`, `validate`, `to_json`, `read_metadata`), die Telemetrietypen und `theme` | `src/` |
| Dateiformat | `.eh3dproj`: ein ZIP mit `project.json`, `integrity.json` und den Datenordnern `layers/`, `toolpaths/` und `vision/` | [docs/FORMAT.md](docs/FORMAT.md) |
| Fehler | ein `Error`-Enum: `Io`, `Json`, `Zip`, `UnsupportedFormat`, `MissingEntry`, `Integrity`, `UnsafePath`, `DuplicateEntry`, `TooLarge`, `InvalidMetadata` | `src/error.rs` |
| Werkzeug der Firmware-Basis | `python tools/sync_firmware_base.py check`, `sync`, `import <Knoten>` und `status` | [docs/FIRMWARE_BASE.md](docs/FIRMWARE_BASE.md) |

Dieses Repository enthält kein Kommandozeilenprogramm, keinen Server und keinen Netzwerkzugriff.

## ⚙️ Konfiguration

Die Bibliothek hat keine Konfigurationsdatei: Ihre Grenzen sind Konstanten im Code, hier aufgelistet, damit man weiß, was ein Paket enthalten darf. Die Formatversion ist Teil der Datei, keine Einstellung.

| Wert | Grenze | Wo |
|---|---|---|
| Einträge in einem Paket | 1 bis 10.000 | `MAX_ENTRIES` in `src/container.rs` |
| Größe eines Eintrags | bis 256 MiB (268.435.456 Bytes) | `MAX_ENTRY_BYTES` |
| Größe aller Einträge zusammen | bis 1 GiB | `MAX_TOTAL_BYTES` |
| Länge eines Eintragsnamens | 1 bis 240 Bytes | `check_entry_name` |
| Datenordner | nur `layers/`, `toolpaths/`, `vision/` | `DATA_FOLDERS` in `src/project.rs` |
| Formatversion | 1.0.0 (dieselbe Hauptversion wird gelesen, eine neuere abgewiesen) | `FORMAT_VERSION` |
| Werkzeugdurchmesser | endlich und größer als 0 | `ProjectMetadata::validate` |

Die Firmware-Basis hat ein eigenes Einstellungsdokument (`node_settings.hpp`, `config_common.hpp`); seine Werte gehören zum jeweiligen Knoten und stehen im README von EH3D-NODE-BUNGARD.

## 🚀 Schnellstart

Du brauchst eine stabile Rust-Toolchain (1.75 oder neuer).

```rust
use eh3d_common::{Package, ProjectMetadata};

fn main() -> eh3d_common::Result<()> {
    let mut package = Package::new(ProjectMetadata::new("Test board"));
    package.add_entry("layers/top.gbr", b"G04 test*".to_vec())?;
    package.write_to_path(std::path::Path::new("board.eh3dproj"))?;
    let again = Package::read_from_path(std::path::Path::new("board.eh3dproj"))?;   // prüft jeden Eintrag
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

Die Suite hat 115 Tests (`cargo test --workspace`, in der CI unter Windows und Linux ausgeführt): 16 in den Modulen (die Metadaten und ihre Prüfung, die Alarmcodes und Maschinenzustände, die Farben und ihr WCAG-Kontrast) und 14 am Container (Hin- und Rückweg über Speicher und Datei, gleiche Bytes bei gleichem Inhalt, der Integritätseintrag, ein geänderter Eintrag, eine geänderte Größe, ein nicht gelisteter Eintrag, ein fehlender Eintrag, bearbeitete Metadaten, eine Datei, die kein Paket ist, unsichere und feindliche Eintragsnamen, ein ersetzter Eintrag und ein ungültiges Projekt). Die CI führt außerdem `cargo fmt --check` und `cargo clippy` mit Warnungen als Fehlern, `python -I tools/ci_validate.py` (Manifest, Versionen, Änderungsprotokoll und die acht READMEs) und `tools/sync_firmware_base.py check` (die Kopien der Firmware-Basis) aus. Die Host-Tests der Firmware-Basis (`firmware_base/tests/`) laufen in EH3D-NODE-BUNGARD, nicht hier. Die acht gemeinsamen Crates fügen 85 weitere hinzu: die Auftragsarchive (6), die Dateien der Bungard-Maschinen mit den HP-GL-Bögen, -Kreisen und der Skalierung und den Gerber- und DXF-Umrissen (16), die Maschinensteuerungen auf einer geskripteten Leitung (19) sowie die Szene, die Ansichten, die Themen und die Wörter der Oberfläche in acht Sprachen (20), die Fenster Öffnen und Speichern (7), die Maschinen und Stationen des Servers (5) und die Höhenkarte des Tisches (8) und der Client des Knotens am Bohrkopf (4).

## 🏗️ Build und Veröffentlichung

Die Version steht in `Cargo.toml` und in `eh3d.project.json`, und jede Version hat ihren Eintrag in [CHANGELOG.md](CHANGELOG.md). Releases werden auf GitHub veröffentlicht; die Bibliothek wird als Rust-Crate aus diesem Repository genutzt und liegt nicht auf crates.io.

## 📂 Struktur des Repositorys

<p align="center">
  <img src="images/repository-structure.svg" alt="Struktur des Repositorys EH3D-COMMON" width="100%">
</p>

<details>
<summary>Textversion des Baums</summary>

```text
EH3D-COMMON/
├── src/
│   ├── container.rs                             das Paket: Lesen, Schreiben, Integrität und Grenzen
│   ├── project.rs                               die Metadaten und ihre Prüfung
│   ├── telemetry.rs                             Achsen, Spindel, Zustände und Alarmcodes
│   └── theme.rs error.rs lib.rs                 die Farben mit Kontrast, die Fehler und die Crate-Wurzel
├── crates/                                      gemeinsame Crates: Ansicht, ZIP, Maschinendateien, Steuerungen
├── firmware_base/
│   ├── core/                                    ohne Hardware: Einstellungen, Auth, Netzplan
│   ├── main/                                    ESP-IDF-Code: Netzwerk, Speicher, Panel-API
│   ├── tests/                                   die Host-Tests von allem oben
│   ├── tools/                                   packt das Panel; listet ignorierte sdkconfig-Zeilen
│   └── manifest.json                            welche Datei in welchen Knoten geht
├── tests/                                       das Paket: Hin- und Rückweg, geänderte, fehlende und nicht gelistete Einträge, unsichere Namen
├── docs/                                        das Format, die Firmware-Basis und die Daten der beiden Bilder
├── tools/                                       CI-Prüfung, README-Prüfung, Firmware-Basis-Sync und die Bildwerkzeuge
├── images/                                      Banner und die Bilder von Architektur und Repository-Struktur
├── .github/                                     CI- und Release-Workflows, Vorlagen für Issues und Pull Requests
├── eh3d.project.json                            Manifest: Version, Rolle, Reifegrad und Build-Befehl
├── Cargo.toml Cargo.lock                        das Rust-Paket und seine festgelegten Abhängigkeiten
├── CHANGELOG.md                                 was sich in jeder Version geändert hat
├── SECURITY.md SUPPORT.md                       Schwachstellen melden und Fragen stellen
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   Mitwirken, Verhaltenskodex und Lizenz
```

</details>

## 🗺️ Roadmap und bekannte Grenzen

**Bekannte Grenzen heute**

* Noch kein Programm benutzt den Projektcontainer, deshalb wurde das Format nur von seinen eigenen Tests gelesen und geschrieben.
* Das Paket ist weder signiert noch verschlüsselt, und die Bibliothek wurde nicht unabhängig geprüft.
* Es gibt keine Migration, weil 1.0.0 das erste Format ist.
* Die Firmware-Basis wurde nur über EH3D-NODE-BUNGARD erprobt und lief nie auf einer Platine; einen zweiten Knoten gibt es noch nicht.
* Die Bibliothek ist nicht auf crates.io veröffentlicht.

**Als Nächstes**

* Das erste Programm der Suite damit ein Projekt öffnen und speichern lassen (zuerst EH3D-CORE-LIBS).
* Den zweiten Knoten zur Firmware-Basis hinzufügen und prüfen, dass die Kopien identisch bleiben.
* Entscheiden, ob Pakete eine optionale Signatur brauchen.

## 🌍 Sprachen

README und Texte gibt es auf Englisch, Spanisch, Französisch, Italienisch, Deutsch, Chinesisch, Japanisch und europäischem Portugiesisch.

## 🔗 Verwandte Projekte

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): die Parser (Excellon, Gerber) und die Geometrie, die diese Typen nutzen werden
* [EH3D-NODE-BUNGARD](https://github.com/JuanenRac/EH3D-NODE-BUNGARD): der erste Knoten, der eine Kopie der Firmware-Basis hält
* [EH3D-SERVER](https://github.com/JuanenRac/EH3D-SERVER): das Zentrum der Familie; es nutzt den Projektcontainer noch nicht
* [EH3D-DOCS](https://github.com/JuanenRac/EH3D-DOCS): das Dokumentationszentrum der Suite und ihre Karte der Projekte
* [HYDRA-UMC](https://github.com/JuanenRac/HYDRA-UMC): die Robotik-Plattform, deren Maschinen die Suite künftig ansteuern soll

## 📚 Dokumentation

* [Projektformat](docs/FORMAT.md): die Einträge, die Metadaten, der Integritätseintrag und die Regeln für Leser.
* [Die Firmware-Basis](docs/FIRMWARE_BASE.md): was geteilt wird, wie es identisch gehalten wird und wie man einen Knoten hinzufügt.
* [Änderungsprotokoll](CHANGELOG.md)

## 🤝 Mitwirken und Community

* Führe vor einem Pull Request `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `python -I tools/ci_validate.py` und `python -I tools/sync_firmware_base.py check` aus: alles muss bestehen. Siehe [CONTRIBUTING.md](CONTRIBUTING.md) und den [Verhaltenskodex](CODE_OF_CONDUCT.md).
* Das README gibt es in acht Sprachen: Eine Textänderung wird in allen acht Dateien gemacht, und die beiden Bilder werden aus `docs/architecture.json` und `docs/structure.json` mit `tools/make_architecture.py` und `tools/make_structure.py` gezeichnet.
* Eine Änderung an der Firmware-Basis wird in einem Knoten gemacht, mit `tools/sync_firmware_base.py import` übernommen und in die anderen synchronisiert; bearbeite nie eine Kopie von Hand.
* Melde eine Schwachstelle privat: [SECURITY.md](SECURITY.md). Fragen, Ideen und Meldungen: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LIZENZ

Software: GPL-3.0-or-later. Siehe [LICENSE](LICENSE). Die Hardware-Entwürfe der Suite verwenden die CERN OHL-S.
