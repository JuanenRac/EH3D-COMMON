# The database of 3D printing materials

The crate `crates/printmaterials` (`eh3d-printmaterials`) holds the filaments (FDM) and the photopolymer resins (SLA, DLP, LCD, MSLA) that a slicer or any other program needs. Its README lists the fields; this page says how to use the data, how it is kept, what is in it, which licences apply and how a program that keeps its own tables moves to it.

## Using it

```rust
use eh3d_printmaterials::{Database, FilamentCriteria, Lang, ResinCriteria};

let db = Database::embedded()?;                       // checked: every rule below, every file

// a filament
let f = db.filament("polymaker/polylite-pla").unwrap();
f.nozzle_range();                                      // Some((min, max)) only when the record gives both
f.bed_range();
f.mm3s_limit();                                        // the volumetric flow limit, mm3/s
f.grams_per_meter(None);                               // density and diameter (1.75 if it comes in it)
f.drying_plan();                                       // temperature, hours, sensitivity - or None
f.needs_hardened_nozzle();                             // the record says; if not, the filler tells
f.fan_at_layer(1);                                     // by stretch of layers, else the recommended value

// a search
db.search_filaments(&FilamentCriteria { nozzle_c: Some(250.0), ..Default::default() });

// a resin: the settings for a printer and a layer height, never extrapolated
let r = db.resin("generic/generic-standard-resin").unwrap();
match r.exposure_for("generic-msla", 0.05) {
    Ok(e) => { let _ = (e.normal_s, e.base_s, e.base_layers, e.notes()); }
    Err(w) => eprintln!("{}", w.text(Lang::Es)),
}
db.search_resins(&ResinCriteria { printer: Some("generic-msla".into()), layer_mm: Some(0.05), ..Default::default() });
```

The cost is not in the data. `cost_per_gram(spool_price, spool_grams)` and `cost_per_meter(spool_price, spool_grams, density, diameter)` take the price the user paid.

### The exposure of a resin

`Resin::exposure_for(printer, layer_mm)` looks at the points of that printer in `exposures`:

* the layer height of a point: that point, as it is (`interpolated = false`);
* between two points: a linear interpolation of every number both points have (`interpolated = true`; a number that only one of them has is left empty, not guessed; the number of base layers is rounded);
* below the lowest or above the highest: `ExposureWarning::OutOfRange`, and nothing is extrapolated;
* a printer the resin has no point for: `ExposureWarning::NoPrinter`; a resin with no points at all: `ExposureWarning::NoPoints`.

`Exposure::notes()` says whether the values were interpolated and that they were not tried on a machine (`verified` is the conjunction of the points it comes from). The warnings and the notes read in the eight languages of the suite (`text(Lang)`).

The identifier of the printer is the one of the printer profiles of the slicer when it exists (`bambu-lab-a1`, `elegoo-mars-4`...). Settings that fit a monochrome screen printer in general use `generic-msla`.

## The schema

One JSON file per manufacturer (`{"schema_version": 1, "manufacturer": "...", "filaments": [...]}` or `"resins": [...]`), `deny_unknown_fields` everywhere. Units are in the name of the field. A value the datasheet does not publish is left out or `null`; a bound or a note goes in `notes`. The record has no price. Every record says `"verified": false` and so does every exposure point: the rule of the data is that nothing is claimed to have run on a machine (the check refuses a `true`).

### Filament

| Field | Meaning |
|---|---|
| `id`, `manufacturer`, `name` | `<manufacturer>/<name>` in lower case with hyphens; the name as the maker writes it |
| `material`, `grade` | the polymer (`pla`, `petg`, `abs`, `asa`, `tpu`, `tpe`, `pa`, `pc`, `pp`, `pva`, `bvoh`, `hips`, `pet`, `ppa`, `pps`, `peek`, `pei`, `other`) and its variant (`PLA+`, `PA12`, `silk`) |
| `fill` | `carbon_fiber`, `glass_fiber`, `wood`, `metal`, `glitter`, `glow`, `mineral`, `other` |
| `shore` | `{value, scale}`, scale `a` or `d`, for the soft ones |
| `diameters_mm`, `diameter_tolerance_mm`, `density_g_cm3` | 1.75 and 2.85 are the usual diameters |
| `colors` | `{name, hex, td}`: the name, `#RRGGBB` and the transmission distance when measured |
| `nozzle_c`, `bed_c`, `fan_pct` | `{min, max, recommended}`, each part optional, in order |
| `chamber_c` | the chamber temperature advised |
| `max_speed_mm_s`, `max_flow_mm3_s` | the limits of the print |
| `fan_spans` | `[{from_layer, pct}]`: the fan from a layer on, in order of layer |
| `retraction` | `{distance_mm, speed_mm_s, drive}`, drive `direct` or `bowden` |
| `drying`, `moisture` | `{temp_c, hours}` and `low`, `medium` or `high` |
| `abrasive`, `min_nozzle_mm`, `enclosure_advised`, `direct_drive_advised` | what a nozzle, an enclosure or an extruder must be |
| `spools` | `[{weight_g, diameter_mm, width_mm, hub_mm}]` |
| `tg_c`, `hdt_c`, `tensile_mpa`, `modulus_mpa`, `td` | the mechanical and optical values the sheet gives |
| `compatibility` | `{printers, nozzles_mm, notes}`: what the maker names |
| `links`, `notes` | `https` pages of the maker (data sheet, safety sheet) and a word |
| `source`, `source_ref`, `verified` | where it comes from, the place in it, and `false` |

### Resin

| Field | Meaning |
|---|---|
| `id`, `manufacturer`, `name`, `kind` | `standard`, `hard`, `flexible`, `abs_like`, `castable`, `dental`, `water_washable`, `high_temperature`, `ceramic`, `clear`, `high_detail`, `other` |
| `wavelength_nm`, `viscosity_mpa_s`, `density_g_cm3` | 385 and 405 nm are the usual wavelengths |
| `shore_d`, `shore_a`, `tg_c`, `hdt_c`, `shrinkage_pct` | after curing |
| `tensile_mpa`, `flexural_mpa`, `impact_kj_m2`, `elongation_pct`, `water_absorption_pct` | mechanical values |
| `colors`, `translucent` | colours and whether the part is clear |
| `sds` | the file name of the safety data sheet |
| `post_cure`, `wash` | `{minutes, temp_c, device}` and `{solvent, minutes}`, solvent `ipa`, `alcohol`, `water`, `other` |
| `layer_mm` | `{min, max, recommended}`: the layer heights the maker advises |
| `exposures` | the settings by printer and layer height (below) |
| `source`, `source_ref`, `verified` | as in the filament |

An exposure point: `printer`, `layer_mm`, `normal_s` (with `normal_s_min` and `normal_s_max`), `base_s` (with its lowest and highest), `base_layers`, `lift_speed_mm_min`, `retract_speed_mm_min`, `lift_mm`, `lift2_mm`, `wait_before_s`, `wait_after_s` and `verified`. One point per printer and layer height.

## Adding data

1. Write the record in `crates/printmaterials/data/filaments/<manufacturer>.json` or `data/resins/<manufacturer>.json`, with the identifier from the manufacturer and the name, every value as the sheet publishes it, `source` set to `maker-datasheet` and `source_ref` to the name of the sheet (a file name, not a path on a disk). Do not copy tables or texts from another collection of data: read the sheet of the maker.
2. A new resin file goes into the list in `src/data.rs`. The list of the filament files is written by `tools/import_open_filament_db.py` (or by hand in `src/filament_files.rs` when a file is added by hand); a test fails when a file of a folder is missing from its list.
3. `cargo test -p eh3d-printmaterials` checks every file: schema version, unique identifiers that agree with the manufacturer and the name, every value inside the range it can have, ranges in order, a source, `https` links, one exposure per printer and layer height, and nothing verified. `cargo clippy -p eh3d-printmaterials --all-targets -- -D warnings` and `rustfmt --edition 2021 --check` are the same as for the other crates.
4. The data files are not text of the platform: `.gitattributes` keeps them as they are on every system, so a test that reads one with `include_str!` sees the same bytes on Windows and on Linux.

The seed of the filaments is written by the tool from a shallow clone of the Open Filament Database made in a temporary folder and deleted afterwards: `py -3 -I tools/import_open_filament_db.py <clone>/data`. The tool only reads the clone, takes the facts of the most used lines of the main brands (not images, long texts or purchase links), drops a value that is out of range instead of failing, and never marks a record verified. The crate does not depend on it.

## The codes

`PmCode` has the stable codes `EH3D-PMAT-001` to `EH3D-PMAT-009` at the start of every message; `PmCode::title_in(Lang)` gives the title in en, es, de, fr, it, ja, zh and pt (of Portugal).

| Code | Variant of `PmError` | What it is |
|---|---|---|
| `EH3D-PMAT-001` | `Parse` | A data file is not valid JSON of the schema. |
| `EH3D-PMAT-002` | `SchemaVersion` | A data file has a schema version that the crate does not read. |
| `EH3D-PMAT-003` | `DuplicateId` | Two records have the same identifier. |
| `EH3D-PMAT-004` | `IdMismatch` | The identifier does not match the manufacturer and the name, or the file. |
| `EH3D-PMAT-005` | `OutOfRange` | A value is outside the range it can have. |
| `EH3D-PMAT-006` | `Source` | The source is missing, or a reference or a link is not valid. |
| `EH3D-PMAT-007` | `FileList` | A data file of a folder is not in the list of embedded files. |
| `EH3D-PMAT-008` | `Exposure` | The exposure cannot be given for the printer and the layer height. |
| `EH3D-PMAT-009` | `Inconsistent` | The values of a record contradict each other (a lowest above a highest, a repeated point...). |

## Licences and sources

* The facts of a filament or a resin are the maker's. A collection of them can have conditions of its own, so the data is taken in three ways only: records written from the datasheet of the maker (named in `source_ref`), the generic profiles of the slicer, and the seed of filaments imported from a collection published under the MIT licence.
* The notice that the MIT licence asks for is in `crates/printmaterials/THIRD_PARTY_NOTICES`, and every record that comes from it says so in `source` (`open-filament-database`) and gives its place in that collection in `source_ref`.
* No table, text or image of another site is copied. Collections of resin settings that are not under a licence that allows it are not used: the resin data of the suite is the generic profiles and what the maker publishes in its own sheet.

## Moving a program that keeps its own tables (the slicer of the suite)

PHOENIX-3DP keeps 12 filaments and 7 resins in `crates/p3dp-profiles/data/filaments.tsv` and `resins.tsv`. Every column has a place in the schema, so the tables can be replaced without losing a value:

| Column of `filaments.tsv` | Field |
|---|---|
| id | `source_ref` (the new `id` is `generic/<name>`) |
| name, material | `name`, `material` (`PLA+` is `pla` with `grade` `PLA+`) |
| nozzle C: best, lowest, highest | `nozzle_c.recommended`, `.min`, `.max` |
| bed C: best, lowest useful, highest | `bed_c.recommended`, `.min`, `.max` |
| part fan %: best, lowest, highest | `fan_pct.recommended`, `.min`, `.max` |
| density, diameter | `density_g_cm3`, `diameters_mm` |
| smallest nozzle | `min_nozzle_mm` |
| abrasive, enclosure advised, direct drive advised | `abrasive`, `enclosure_advised`, `direct_drive_advised` |
| verified | `verified` |

| Column of `resins.tsv` | Field |
|---|---|
| id | `source_ref` (the new `id` is `generic/<name>`) |
| name, kind | `name`, `kind` (`tough` is `abs_like`; the water-washable resin is `water_washable`) |
| layer um: best, lowest, highest | `layer_mm.recommended`, `.min`, `.max` (in mm) and `exposures[].layer_mm` (the best) |
| normal exposure s: best, lowest, highest | `exposures[].normal_s`, `normal_s_min`, `normal_s_max` |
| bottom exposure s: best, lowest, highest | `exposures[].base_s`, `base_s_min`, `base_s_max` |
| bottom layers | `exposures[].base_layers` |
| lift mm, lift speed mm/min | `exposures[].lift_mm`, `lift_speed_mm_min` |
| wavelength, density | `wavelength_nm`, `density_g_cm3` |
| wash (`alcohol`, `water`) | `wash.solvent` |
| verified | `verified` |

The generic profiles are already in the crate (`data/filaments/generic.json`, `data/resins/generic.json`) with the values of those tables, and the printer `generic-msla` for the resins. The steps for the program:

1. Add the dependency on `eh3d-printmaterials` and read `Database::embedded()` instead of `include_str!("../data/filaments.tsv")`.
2. Look the profiles up by the new identifier, or keep the old identifier in a table of its own (it is in `source_ref`).
3. Where the old code took the temperatures to use from a printer limit, use `nozzle_range`, `bed_range` and the recommended values; where it checked the nozzle, use `needs_hardened_nozzle` and `min_nozzle_mm`.
4. Where the old code took the exposure of a resin as one row, ask `exposure_for` with the printer and the layer height; keep the old behaviour for a layer height outside the known ones by showing the warning instead of a guess. The ranges of the old table (lowest and highest) are in `normal_s_min` and `normal_s_max`.
5. Add the exposures of each resin for the printers of the profiles (`elegoo-mars-4`...) as data, from the sheet of the maker, when they are known.
6. The profile files of the user keep working in the slicer; the database is the starting point, and a profile file replaces any row, as before.

## Not in the data yet

* Resins of the makers beyond the generic profiles: they are added from the sheet of each maker, with the sheet named in `source_ref`.
* A filament of the seed has no recommended temperature unless the maker gives one; the slicer uses the middle of the range or the generic profile of the polymer.
* Nothing is verified on a machine. The day a profile is tried, the person who tried it says so in a profile file of their own; the data of the suite stays at `false`.
