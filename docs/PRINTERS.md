# The catalog of 3D printers (`eh3d-printers`)

One record per model, shared by the slicer, the server and the clients. The figures come from the public pages of each maker; none has been tried on a machine, and nothing in the crate talks to a printer.

## The data

One JSON file per maker in `crates/printers/data/printers/`, embedded in the crate, with `schema_version` 1. A field the schema does not know is refused.

| Field | Meaning |
|---|---|
| `id` | `<maker>/<model>` in lower case with hyphens; `Catalog::find` also takes `<maker>-<model>` |
| `manufacturer`, `model` | as the maker writes them |
| `volume_mm` | x, y and z of the build volume |
| `filament_diameter_mm`, `nozzle_default_mm`, `nozzles_mm` | the filament and the nozzles the maker names |
| `enclosure` | `open`, `enclosed` or `unknown` |
| `heated_chamber`, `chamber_max_c` | whether the chamber is heated on purpose, and how far |
| `nozzle_max_c`, `bed_max_c` | the highest temperatures |
| `max_speed_mm_s`, `max_acceleration_mm_s2` | the highest speed and acceleration of the head the maker states |
| `max_flow_mm3_s` | the highest flow of the hot end, as the maker measures it with one material; a ceiling, not a figure for every filament |
| `default_height_mm` | the height used by default where the maker says it is less than the volume |
| `multicolor` | `status` (`included`, `optional`, `none`, `unknown`), `system`, `max_colors` |
| `firmware` | `bambu`, `klipper`, `marlin` or `unknown` |
| `adapter` | the family of protocol: `bambu_lan`, `klipper_moonraker`, `marlin_serial`, `octoprint`, `prusalink`, `vendor_protocol`, `unknown` |
| `adapter_documented` | `false` when the family is taken from the firmware and the maker does not document it: check it on the machine |
| `send_formats` | `gcode`, `gcode_3mf`, `bgcode`; empty when the maker does not say |
| `sources` | the pages: an `https` address, the fields it is named for (`covers`) and, of those, the ones read on the page itself (`confirmed`) |
| `verified` | always `false` in the data of the suite: it means tried on a machine |

A figure that is not published is absent or `unknown`, never a guess.

## The rules

The data is checked when it is loaded. A record is refused when its identifier does not match, a figure is out of range or is not a number, a text has a control character or one of `| # =`, the values contradict each other, it says it was verified, or **a claim names no page**: `volume_mm`, `nozzle_max_c` and `bed_max_c` always need a source, and so does every other field that is given. A file over 1 MiB or with more than 500 printers is refused.

| Code | Title |
|---|---|
| `EH3D-PRN-001` | Data file not valid |
| `EH3D-PRN-002` | Schema version not supported |
| `EH3D-PRN-003` | Duplicate identifier |
| `EH3D-PRN-004` | Identifier does not match the record |
| `EH3D-PRN-005` | Value out of range |
| `EH3D-PRN-006` | Source missing or not valid |
| `EH3D-PRN-007` | Values contradict each other |
| `EH3D-PRN-008` | Text not accepted |
| `EH3D-PRN-009` | Data file too large |

## What was read and what is still to be checked

`confirmed` holds the fields that were read on the maker's own page and agree with it. `Printer::confirmed(field)` asks for one field and `Printer::unconfirmed()` lists the claims of a record that no page was read for: they are to be checked. A field is left out when the page no longer answers, does not say it, or could only be read through a summary. Reading a page is not trying a machine, so `verified` stays `false`.

## Where to start from

* `starting_filaments(printer, database, nozzle)` looks at every generic filament of `eh3d-printmaterials` and gives the reasons it cannot be printed (nozzle or bed not hot enough, nozzle too narrow, another diameter) and what to know (closed printer advised, hardened nozzle, advised temperature above the limit). Nothing is left out and no temperature is lowered.
* `Processes::for_nozzle(level, nozzle)` computes the layer height, the line width, the walls, the skins and the infill of `draft`, `standard`, `fine` and `strong` from the ratios in `data/processes.json`. The ratios were written for this suite; they are a place to start and have not been tried on a machine.

Every word a program shows (`title_in(Lang)`) is in English, Spanish, German, French, Italian, Japanese, Chinese and European Portuguese. The translations have not been reviewed by native speakers.

## Adding a printer

Read the maker's own pages, write the record with the address of each page and what it says, leave out what the pages do not say, and run the tests. A new file is added to the list in `crates/printers/src/catalog.rs`. Never copy the profiles of another program.
