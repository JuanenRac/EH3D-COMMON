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
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Ehrlichkeitscheck - was heute tatsächlich läuft:** eine Rust-Bibliothek mit dem Projektcontainer (`.eh3dproj`), den Metadaten eines Projekts, SHA-256-Integritätsprüfungen und den Typen der Maschinentelemetrie. Alles läuft in Tests auf dem Entwicklungsrechner; noch kein Programm der Suite nutzt sie, und nichts wurde auf einer Maschine ausgeführt.

---

> **Stand von Sicherheit und IT-Sicherheit:** Diese Bibliothek ist kein Sicherheitssystem, und mit ihr wurde noch keine Maschine gesteuert. Der Container erkennt ein versehentlich beschädigtes oder verändertes Paket, ist aber weder signiert noch verschlüsselt: Er beweist nicht, wer ein Projekt geschrieben hat. Ein Paket einer anderen Person ist eine nicht vertrauenswürdige Eingabe, deshalb sind Eintragsnamen, Größen und Anzahl der Einträge begrenzt.

## 📋 Auf einen Blick

| | |
|---|---|
| Version | `0.0.1` |
| Reifegrad | Gerüst |
| Tests | 29 |
| Technologie-Stack | Rust |
| Auf echter Hardware | Nicht zutreffend: eine Datenbibliothek, nichts hier spricht mit einer Maschine |

## 🚦 Projektstatus

Jede Zeile ist ein Teil dieses Repositorys, und die Ampel sagt, wie weit er heute ist. Sie beruht auf dem, was das Repository zeigen kann (Tests, Builds, Läufe), nicht auf dem, was geplant ist.

🟢 Fertig und überprüft · 🟡 Funktioniert, mit bekannten Grenzen · 🟠 Teilweise fertig oder noch nicht überprüft · 🔴 Nicht gemacht oder funktioniert nicht

| Bereich | Stand | Was das hier bedeutet |
|---|---|---|
| Software | 🟡 | Der Container, die Metadaten und die Telemetrietypen funktionieren als Rust-Bibliothek; noch kein Programm der Suite verwendet sie. |
| Tests | 🟢 | 29 Tests bestehen (`cargo test`): Hin- und Rückweg, geänderte, fehlende und nicht aufgeführte Einträge sowie unsichere Namen. |
| Datenformat | 🟡 | `.eh3dproj` ist ein Standard-ZIP mit dem SHA-256 jedes Eintrags, beschrieben in `docs/FORMAT.md`; eine neuere Hauptversion wird abgelehnt. Bisher lesen und schreiben es nur die eigenen Tests. |
| Sicherheit | 🟡 | Unsichere Eintragsnamen sowie zu große oder zu viele Einträge werden abgelehnt, und ein verändertes Paket lässt sich nicht öffnen; das Paket ist weder signiert noch verschlüsselt und wurde nicht unabhängig geprüft. |
| Gemeinsame Firmware-Basis | 🟠 | Der Teil der Knoten-Firmware, den alle Knoten teilen, wird nur über EH3D-NODE-BUNGARD getestet, den einzigen Knoten, der ihn verwendet; einen zweiten Knoten gibt es noch nicht. |
| Echte Hardware | 🟠 | Die Bibliothek braucht keine, aber die darin enthaltene Firmware-Basis ist noch nie auf einer Platine gelaufen. |
| Dokumentation | 🟢 | README in acht Sprachen, die Formatbeschreibung und die Anleitung zur Firmware-Basis; die Prüfungen vergleichen sie mit dem Manifest und dem Änderungsprotokoll. |

## 🎯 Überblick

* Der **Projektcontainer**: ein `.eh3dproj`-Paket ist ein Standard-ZIP-Archiv mit `project.json`, `integrity.json` und den Ordnern `layers/`, `toolpaths/` und `vision/`.
* Die **Metadaten** eines Projekts: Name, Identität, Einheiten, Werkstücknullpunkt, Lagen, Werkzeuge, Werkzeugwege und Passmarken, mit einer semantischen Formatversion. Eine neuere Hauptversion wird abgelehnt, statt halb verstanden zu werden.
* Das **Integritätsprotokoll**: SHA-256 und Größe jedes Eintrags. Ein Paket, in dem ein Eintrag verändert wurde, fehlt oder nicht aufgeführt ist, wird nicht geöffnet.
* Die **Grenzen**: Eintragsnamen, die den Paketordner verlassen könnten, werden abgelehnt, ebenso zu große oder zu viele Einträge.
* Die **Telemetrietypen**: Achsen, Spindel, Maschinenzustände und GRBL-Alarmcodes mit ihrem Schweregrad.
* Das **Thema**: die Farben der Suite, mit einer Kontrastprüfung.
* Die **Firmware-Basis der Knoten** (`firmware_base/`, mit `tools/sync_firmware_base.py`): der Teil der ESP32-Knoten-Firmware, den alle EH3D-Knoten teilen - das Einstellungsdokument, das Netzwerk, der Server des Panels, Benutzer und Sitzungen, das Update mit Rollback, das Log, der Selbstschutz und die zugehörigen Tests - einmal geschrieben, mit dem Projektnamen als Platzhalter. Jeder Knoten behält seine eigene Kopie und baut für sich; das Werkzeug schreibt die Basis in jeden Knoten und meldet, wenn eine Kopie abgewichen ist.

## 🧩 Rolle in EH3D

EH3D-COMMON ist die Grundlage der Suite: Die Bibliotheken und Programme, die ein Projekt öffnen oder speichern, sollen davon abhängen. Keines davon existiert bisher. Auch die Firmware-Basis der Knoten liegt hier: EH3D-NODE-BUNGARD ist der erste Knoten, der sie nutzt.

## 📂 Repository-Struktur

```text
EH3D-COMMON/
├── src/          container.rs, project.rs, telemetry.rs, theme.rs, error.rs
├── tests/        the package: round trips, changed, missing and unlisted entries, unsafe names
├── docs/         FORMAT.md: the .eh3dproj format
├── firmware_base/der gemeinsame Teil der Knoten-Firmware
├── tools/        ci_validate.py: manifest, versions, changelog and README checks
├── images/       banner
└── eh3d.project.json   the manifest of the project
```

## 🛠️ BAUEN UND AUSFÜHREN

Du brauchst eine stabile Rust-Toolchain.

```bash
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
python -I tools/ci_validate.py
```

## 🔗 Verwandte Projekte

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): geplant: Parser und Geometrie-Engines, die diese Typen nutzen werden
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): geplant: die Zentrale, die Projekte öffnen und speichern wird
* [HYDRA-UMC](https://github.com/JuanenRac/HYDRA-UMC): die Robotikplattform, deren Maschinen die Suite künftig steuern soll

## 📚 Dokumentation und Community

* [Projektformat](docs/FORMAT.md): die Einträge, die Metadaten, das Integritätsprotokoll und die Regeln für Leser.
* [Die Firmware-Basis](docs/FIRMWARE_BASE.md): was geteilt wird, wie es identisch gehalten wird und wie man einen Knoten hinzufügt.
* [Änderungsprotokoll](CHANGELOG.md)
* [Mitwirken](CONTRIBUTING.md)
* [Sicherheitsrichtlinie](SECURITY.md)
* [Support](SUPPORT.md)

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LIZENZ

Software: GPL-3.0-or-later. Siehe [LICENSE](LICENSE). Die Hardware-Entwürfe der Suite verwenden die CERN OHL-S.
