# The machining materials

The module `machining` of the crate `crates/materials` (`eh3d-materials`) is the second family of the materials database: the things that a CNC router or a laser cuts, mills or engraves. The first family (the board materials, see [MATERIALS.md](MATERIALS.md)) describes laminates for the design tool; this one describes workpieces for the machine programs. The two live in the same crate, have their own files and their own lists, and do not touch each other: the board stock of the second may name its laminate in the first.

It answers three questions for a material: what it is, **may a laser touch it and why not**, and **where to start** with a router or a laser. It never answers a question it has no data for, and it never goes outside the points it knows.

## The files

`crates/materials/data/machining/` holds one JSON file per group (`wood`, `plastic`, `metal`, `board`, `sheet`, `mineral`), embedded with `include_str!` through the explicit list in `src/machining/data.rs`; a test fails when a file of the folder is not in the list. A file is `{"schema_version": 1, "materials": [...]}`. The files keep their line ends on every platform (`-text` in `.gitattributes`).

## The record of a workpiece

| Field | Meaning |
|---|---|
| `id` | `<family>/<English name as a slug>`, for example `wood/birch-plywood`; the check refuses an identifier that does not follow from the family and the name |
| `family` | `wood`, `plastic`, `metal`, `paper`, `fabric`, `leather`, `rubber`, `foam`, `composite`, `mineral` or `pcb_stock` |
| `names` | the name in `en`, `es`, `de`, `fr`, `it`, `ja`, `zh` and `pt` (Portugal); all eight are required |
| `density_g_cm3` | the density, when it is a figure that does not vary much |
| `thicknesses_mm` | the commercial thicknesses |
| `safety` | what a laser may do (below) |
| `cnc` | starting points for a router (below) |
| `laser` | starting points for a laser (below) |
| `pcb_material_id` | the record of the board database that this board stock is (`bungard/fr4-copper-clad`); `MachiningDatabase::check_links` finds a link that points nowhere (`EH3D-MAT-015`) |
| `notes` | a word in English |
| `source` | where the physical figures come from, in a few words |

A field the schema does not know is refused when a file is read. Every field added after the first version is optional and is left out of the file when it is empty, so a file written before them still reads and is written back the same.

## What a laser may do

```json
"safety": {
  "laser_cut_allowed": false,
  "laser_engrave_allowed": false,
  "laser_unsafe": true,
  "hazards": ["chlorine", "toxic_fumes"]
}
```

* `laser_unsafe` means that the material must **never** be put under a laser: it gives off poisonous or corrosive fumes (PVC, vinyl, neoprene, ABS, acetal, nylon, PTFE, polycarbonate, polyurethane foam, chrome tanned and imitation leather, carbon fibre and the glass fibre boards). It implies that neither cutting nor engraving is allowed and that there are no laser points.
* A material can refuse the laser without being unsafe: a bare metal reflects the beam, polyethylene, polypropylene and PETG melt and burn instead of cutting, glass is cut by nothing a hobby laser has (a CO2 laser can mark it).
* `hazards` says why, and is never empty for a refusal. The hazards are `chlorine`, `cyanide`, `formaldehyde`, `fluorine`, `chromium`, `fibre_dust`, `toxic_fumes`, `reflective`, `flammable`, `melts_or_chars`, `fine_dust` and `transparent`; each has its sentence in the eight languages (`Hazard::text(lang)`), and `Workpiece::laser_reasons(lang)` gives them for a material. A material that a laser may cut can also list hazards: they are what to be careful with (the glue of plywood, the dust of MDF, a flame).

The data is checked when it is read: an unsafe material that allows a cut, a refusal that does not say why, a hazard listed twice and laser points for a refused process are errors (`EH3D-MAT-011` and `EH3D-MAT-014`).

## Starting points

A starting point is a place to start, not a setting that works. Every point has a `source` (a few words about where it comes from) and `verified`, which says whether anybody tried it on a machine. **No point of the seed is verified.** The seed is the maintainer's table of practical starting values for small machines (a router of 3 to 6 mm cutters, a diode laser of 10 W, CO2 lasers of 40 and 60 W), plus 24 laser figures from HobbyistLaserCalc (MIT, see `crates/materials/THIRD_PARTY_NOTICES`). It is not copied from the tables of a maker of materials, from a program or from a source with a licence that does not allow it, and where there is no figure to give the material has no point.

### Ranges: the single value and the two ends

A table gives a range ("feed 900-1100"). The single field of the point (`feed_mm_min`) holds the middle of it, which is what the queries interpolate; the ends are kept in `ranges` (`rpm`, `feed_mm_min`, `plunge_mm_min`, `depth_per_pass_mm`, `stepover_pct`, `flutes` for a CNC point; `power_pct`, `speed_mm_min`, `passes` for a laser point). The number of passes is the lowest of its range in the single field. A diameter that is a range (1.0-3.0) is two points, one at each end, with the same values. The check refuses a range that is reversed, outside the limits of its field or that does not hold the single value. Start at the conservative end of a range and use a test grid.

### Optional fields of a point

| Field | Meaning |
|---|---|
| `tip_angle_deg` | CNC: the lowest and highest tip angle of a V-bit (only for `v_bit`) |
| `cutter_style` | CNC: `single_flute`, `o_flute`, `high_helix`, `upcut`, `downcut` or `corn_cob` |
| `coating` | CNC: the coating or material of the cutter, when the table says so |
| `coolant` | CNC: `none`, `air`, `lubricant` or `flood` |
| `spindle_w`, `machine` | the machine and the spindle that the point was given for |
| `air_assist_level` | laser: `off`, `low`, `medium`, `high` or `max`; it must agree with `air_assist` |
| `source_unit`, `source_speed` | laser: the speed as the source wrote it (`mm_min` or `mm_s`); the file always holds mm per minute and the check recomputes it |
| `alternative` | laser: a second figure from another source for a case that already has a main figure; it is never returned by `laser_start`, see `Workpiece::laser_alternatives` |
| `evidence` | `general_practice`, `public_reference`, `manufacturer_test` or `tested_on_machine` |
| `source_info` | `origin`, `date` (`YYYY-MM`), `license` and an optional `url` (`https://`) |

`verified: true` is refused (`EH3D-MAT-014`) unless the evidence is `manufacturer_test` or `tested_on_machine` and the point has its `source_info`; a test on a machine also names the machine. No point of the seed qualifies.

### Licences

A figure goes into the data only when its licence allows it. The maintainer's table is project data. HobbyistLaserCalc is MIT and keeps its notice. Tables of other sources whose licence is copyleft or not declared (maker tables, community calculators, wikis) were read for comparison only and none of their figures is in the data.

CNC (`cnc`):

| Field | Meaning |
|---|---|
| `op` | `contour`, `pocket`, `engrave` or `drill` |
| `tool` | `end_mill`, `compression`, `ball_nose`, `v_bit` or `drill` |
| `diameter_mm`, `flutes` | the cutter |
| `rpm` | the spindle speed |
| `feed_mm_min`, `plunge_mm_min` | the feed in the material and the plunge feed |
| `depth_per_pass_mm`, `stepover_pct` | the depth of a pass and the step over of a pocket |

Laser (`laser`):

| Field | Meaning |
|---|---|
| `kind` | `diode` or `co2` |
| `mode` | `cut` or `engrave` |
| `power_w` | the nominal (optical) power of the laser that the point is given for |
| `thickness_mm` | the thickness; always given for a cut and absent for an engraving |
| `power_pct`, `speed_mm_min`, `passes`, `air_assist` | the settings |

## The queries

```rust
use eh3d_materials::machining::{CncOp, LaserProcess, MachiningDatabase, Tool};

let db = MachiningDatabase::embedded()?;
db.laser_cut_allowed("plastic/polycarbonate-pc");                 // Some(false)
let wood = db.get("wood/birch-plywood").unwrap();
let cut = wood.laser_start(40.0, 4.5, LaserProcess::CO2_CUT)?;    // between the 3 mm and the 6 mm points
let notes = cut.notes();                                          // Interpolated, NotVerified
let mill = wood.cnc_start(CncOp::Contour, Tool::EndMill, 4.0)?;
```

* `Workpiece::cnc_start(op, tool, diameter_mm)` gives the point for the cutter itself, or a straight line between the two nearest known diameters of that operation and cutter.
* `Workpiece::laser_start(power_w, thickness_mm, process)` gives the point for the power and the thickness itself, or a straight line between the two nearest thicknesses of that power; when the power is between two known powers it blends the two curves, but only where both cover the thickness. For an engraving the thickness is ignored.
* The result says whether it was `interpolated` and whether it was `verified`; `notes()` turns that into `StartNote`s, whose text is in the eight languages (`StartNote::text(lang)`).
* Outside what is known there is no answer: `StartWarning::{NoPoints, NoSuchProcess, LaserForbidden, OutOfRange { quantity, requested, lowest, highest }}`, with `text(lang)`. The `_checked` forms and the methods of `MachiningDatabase` give the same warning as a typed error (`EH3D-MAT-011`, `012` and `013`).
* A laser that must not be used on the material is refused first, before any search.
* `Workpiece::laser_alternatives(process)` lists the second figures of other sources for a process; they are not interpolated and a material that a laser must not touch has none.

The limit of a machine is not in the data: the spindle limits live in `spindles.cfg` and the limits of a laser in the settings of its program. A program clamps what it gets to its machine.

## The codes

| Code | Variant of `MatError` | What it is |
|---|---|---|
| `EH3D-MAT-010` | `PointRange` | A starting value of a record is outside the range it can have (a spindle speed, a feed, a power setting, a number of passes...). |
| `EH3D-MAT-011` | `LaserRefused` | A laser is not allowed on the material, or the data lists laser points for a process the material does not allow. |
| `EH3D-MAT-012` | `NoStartPoints` | The material (or the identifier) has no starting points for the process asked. |
| `EH3D-MAT-013` | `OutsideKnown` | The diameter, the thickness or the power asked is outside the known points. |
| `EH3D-MAT-014` | `Workpiece` | A record is not valid (an empty name, an unsafe material that allows a cut, a refusal without a reason, a hazard that poisons on a material the laser may work, a repeated point, a range that is reversed or does not hold its value, an air level that disagrees with the flag, a `verified` point with no evidence, an alternative with no main figure). |
| `EH3D-MAT-015` | `BrokenLink` | A `pcb_material_id` names a material that is not in the board database. |

The codes `EH3D-MAT-001` to `009` of the board database are also used here for the reading of the files (`001` not valid JSON of the schema, `002` schema version, `003` repeated identifier, `004` identifier that does not follow from the family and the name, `006` source that is not text). `MatCode::title_in(lang)` gives the title of all fifteen codes in the eight languages.

## Adding a material or a point

1. Write the record in the file of its group, with the identifier that follows from the family and the English name and the name in the eight languages (Portuguese of Portugal).
2. Say what a laser may do and why not. When in doubt, refuse: `laser_cut_allowed: false` with a hazard.
3. Add a point only with a source for it (`source` and a `source_info` with the licence); leave `verified` as `false` until somebody has tried it on a machine or the maker documents it, and set the `evidence` to match. A point outside the ranges of `validate.rs` is refused. A material with chlorine, cyanide, fluorine, chromium or toxic fumes among its hazards cannot be cut or engraved by a laser, and a material that is on the list of `seed_tests.rs` never leaves it.
4. `cargo test -p eh3d-materials` checks every file, and the tests of the data (no point of the seed is verified, every unsafe material has its reason, every point can be asked for with its own values, the board stock links resolve).

## How the programs with a library of their own will read it

PHOENIX-LASER and PHOENIX-CNC keep their own text libraries today (`examples/materials.cfg` and `examples/materials.txt`). They are not changed by this crate. The move is in steps, each of which can be released on its own:

1. **Depend on the crate** as the other consumers do (`eh3d-materials = { version = ">=0.0.1", path = "../EH3D-COMMON/crates/materials" }`, `EH3D-COMMON` in `depends_on` of the manifest), and read the family with `MachiningDatabase::embedded()`. The crate opens no file, no window and no network.
2. **The shared data is the layer below the library of the person.** A material of the program's own file wins over a material of the shared database with the same name, and a value that the person tuned never changes. The shared record only fills what the file does not say. A program that has no entry for a material offers the shared one as a place to start, marked as not verified.
3. **PHOENIX-LASER.** A `[material NAME]` section maps to a workpiece, a kind of laser and a mode: `power` is `power_pct`, `speed` is `speed_mm_min`, `passes` is `passes`, `verified` is `verified`; the name of the material is chosen from `names[lang]`. The program asks `laser_start(power_w, thickness_mm, process)` with the nominal power of its laser from its settings and the thickness of the job. `max_power` and `max_speed` stay in the program, because they are the limits of the person's machine. The program asks `laser_cut_allowed` before it writes a cut, and says the reasons of `laser_reasons(lang)` when it refuses; a person who knows better can still use their own library, which is their decision and is not made by the shared data. `focus_step`, `kerf`, the hatch and the settings of a picture stay in the program: they are not material data.
4. **PHOENIX-CNC.** A line `material = NAME | feed=LOW,HIGH | rpm=LOW,HIGH` maps to the points of `cnc` of a workpiece: the range of feed and of speed that the checker (`lint`) holds a program to is the smallest and the largest `feed_mm_min` and `rpm` of the points of the material for the cutter of the program, and `cnc_start` gives the point for the exact cutter. The material chosen in the settings keeps its name; the names come from `names[lang]`. A range of the person's own file replaces the one of the database.
5. **The shared seed is enlarged, not forked.** A material or a point that a person found and wants to share is a record of the files above, with its source; the programs stop carrying a table of their own for a material that the family has.
6. **The board materials have already moved.** The stack-up tool of PHOENIX-PCB and the machine program of the Bungard machines read the board family through `materials_adapter.rs` and `boards.rs`; the machining family is not needed by them except for the board stock, which they find by `pcb_material_id`.
