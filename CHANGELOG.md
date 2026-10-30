# Changelog

All notable changes to this project are documented here.

## [Unreleased]

- **One word more:** "Recent" in the eight languages of the interface.
- **The drawing of a board in DXF as a routing path:** `eh3d_bungard_formats::dxf` reads the lines, the light polylines with their bulges, the arcs and the circles of a text DXF file (in the unit its header says) into the paths of a plot, joining the strokes that meet end to end; the archives list `.dxf` files as routing files.
- **Two words for the count of the holes:** "Holes drilled in all" and "Reset the counts" in the eight languages of the interface.
- **The chord angle of a routing configuration as chords of a circle:** `hpgl::segments_for_chord` turns the angle of the routing configuration line (a minus sign only gives the direction) into the chords of a whole circle, for the arcs and circles of a plotter file.
- **The outline of a Gerber board as a routing path:** `eh3d_bungard_formats::gerber` reads the lines and the arcs of the profile layer (the `G01`, `G02` and `G03` strokes, in millimetres or inches, with the zeros left out at either end) into the paths of a plot, which can be written as an HP-GL file with `to_hpgl`; the archives list the usual outline extensions as routing files.
- **A message for a job that does not fit:** the interface words in eight languages have the sentence that tells the operator to move the board or make the table bigger.

## [0.0.2] - The shared crates for the drilling and routing programs and the modern interface: the job view, the ZIP archives, the machine files with the HP-GL arcs and circles, and the machine controllers

- **A lighter debug build:** the `dev` profile keeps the file and line of a panic (`debug = "line-tables-only"`) but not the full debug symbols, so the `target` folder of a test run is about a tenth of its size.
- **README in the universal layout, in eight languages:** the README has the same sections in the same order as the other repositories of the family (status, overview, architecture, features, security, interfaces, configuration, quick start, testing, structure, roadmap and known limits), with figures taken from the code and from the tests that were run. The Architecture and Repository Structure pictures are drawn from `docs/architecture.json` and `docs/structure.json` by `tools/make_architecture.py` and `tools/make_structure.py`, and the CI checks that they match.
- **The layout of the README is checked in the CI:** the validator runs `tools/universal_readme_check.py` and lists, as a warning, what differs from the universal layout of the README.
- The banner follows the animated style of the ecosystem banners, with the name, the kind and the technologies of the project.
- **Arcs, circles and scaling in the HP-GL reader:** `AA`, `AR` and `CI` are cut into chords (72 to a whole circle by default, or the number the machine configuration asks for through `parse_with`), and `SC` with `IP` lays the user units on the plotter points; the pen-up arcs only move the pen.
- **The files of the Bungard machines (`eh3d-bungard-formats`):** the Excellon drill file, the HP-GL routing file, the tool tables of both programs and the machine configuration, read and written back keeping their layout; DRILLPRO, ROUTEPRO and the modern interface read them from here.
- **The interface of the machine controllers (`eh3d-controller`):** one `Controller` trait (status, orders, lines of G-code) and the families of the market that a machine-control program can drive, with what each one needs to be reached (serial, network, USB or a library of the maker) and whether it is free software; the first controller written against it is GRBL (which grblHAL and FluidNC share), tested on a scripted line transport. The families without a controller yet are listed as not driven.
- **A workspace with shared crates:** `eh3d-view` (the scene of a job on the work table, the plan and the turnable isometric views, panel copies, the five themes and the interface words in eight languages) and `eh3d-archive` (jobs packed in ZIP archives: the drilling and routing files inside, names that could leave the work folder never listed, limits on the number and the size of the files).

## [0.0.1] - The project container, its metadata and the machine telemetry types

- **The `.eh3dproj` container:** a ZIP archive with `project.json`, `integrity.json` and the folders `layers/`, `toolpaths/` and `vision/`. The writer is deterministic and writes through a temporary file.
- **Integrity:** the SHA-256 and the size of every entry are recorded; a package with a changed, missing or unlisted entry does not open.
- **Limits:** entry names that could leave the package folder are refused, and so are oversized or too many entries.
- **Metadata:** name, identity, units, zero of the workpiece, layers, tools, toolpaths and fiducials, validated, with a semantic format version: a newer major version is refused and an older one needs a registered migration (there is none yet).
- **Telemetry types:** axes, spindle, machine states and GRBL alarm codes with their severity.
- **Theme:** the colours of the suite with a contrast check.
- **Firmware base of the nodes:** `firmware_base/`, the part of the ESP32 node firmware that every EH3D node shares (settings, network, panel server, users and sessions, update with rollback, log, self-protection, tests), written once with the project's name as placeholders, and `tools/sync_firmware_base.py` to write it into the nodes and check the copies. CI runs the check.
- 29 tests, `cargo clippy -D warnings` and `cargo fmt --check` in CI on Linux and Windows, and a repository check of the manifest, the versions, the changelog and the eight README files.
