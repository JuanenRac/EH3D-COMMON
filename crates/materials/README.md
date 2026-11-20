# eh3d-materials

The database of materials of the EH3D suite, in two families. The **board materials** are laminates, prepregs, bonding films, flex, metal cores, PTFE and ceramic filled boards: the design tool reads the whole record of a material (RF simulation, impedance, stack-ups, thermal work) and the machine program that drills and mills boards reads a small view of it. The **machining materials** (module `machining`, described below and in `docs/MACHINING.md`) are the woods, plastics, metals, paper, leather, fabric, rubber, foam and board stock that a CNC router or a laser works: what a laser may do with them and why not, and starting points that are never extrapolated.

Nothing here is invented. Every value is copied from a manufacturer's sheet, every record names the PDF it comes from (`source` or `sources`, the file name only), and a value the sheet does not give is `null` or absent. A bound such as "more than 60 min" or a range such as "9-12 ppm/C" is not stored as a number: it goes into the `notes` of the record. The glass transition has fields for what a sheet gives in other ways: `tg_lower_bounds` (`[{"method": "DSC", "value_c": 280}]`) for "Tg above 280 C", never mixed with the measured `tg_dsc_c`, `tg_dma_c` and `tg_tma_c`; `tg_unspecified_c` for a Tg the sheet gives without a method; and `cte_unspecified_ppm` for an expansion given without a direction.

## What is in `data/`

One JSON file per manufacturer, `{"schema_version": 1, "manufacturer": "...", "materials": [...]}`, embedded in the crate with `include_str!` through the explicit list in `src/data.rs` (a test fails when a file of the folder is not in the list). The identifier of a material is `<manufacturer>/<name>` in lower case with hyphens (`rogers/ro4350b`); a variant of thickness or copper is not a new record, it goes in `thicknesses_mm` and `copper`.

The fields (all optional except `id`, `manufacturer`, `name`, `kind` and a source) are in `src/model.rs`: kind (`laminate`, `prepreg`, `bonding_film`, `flex_laminate`, `coverlay`, `metal_core`, `ptfe`, `ceramic_filled`, `copper_foil`, `other`), resin, reinforcement, filler, halogen free, UL 94, IPC slash sheet, Tg by DSC, DMA and TMA (and as a lower bound or without a method), Td, T260, T288, T300, CTE, Dk and Df as lists of points `{freq_ghz, value, method, resin_content_pct, tolerance}`, thermal coefficient of Dk, thermal conductivity, specific heat, density, peel strength, water absorption, resistivities, dielectric strength, CTI, flexural and tensile strength and modulus (lengthwise, crosswise or without a direction), thicknesses and their range, copper, panel sizes, glass styles of a prepreg (with their own Dk and Df when the sheet gives them), processing advice, safety data and notes. Units are in the name of the field and frequencies are in GHz. A field the schema does not know is refused when a file is read (`deny_unknown_fields`).

## Checks

Done when the data is loaded, and by a test over every file: known schema version, unique identifiers that agree with the manufacturer and the name, Dk between 1 and 100, Df between 0 and 1, Tg, Td and the bounds between 20 and 700 C (a measured Tg below the bound of the same method is refused), thicknesses, frequencies, copper and panel sizes above zero, and a source that is a plain file name. The errors are typed (`MatError`) and carry the stable codes `EH3D-MAT-001` to `EH3D-MAT-015` (010 to 015 belong to the machining family); the titles are in English, in the eight languages of the suite (`MatCode::title_in(lang)`), and the user interface may translate them by the key `diag.<code>`.

## Use

```rust
use eh3d_materials::{Criteria, Database, Kind};

let db = Database::embedded()?;
let ro4350b = db.get("rogers/ro4350b").unwrap();
let dk = ro4350b.dk_at(10.0)?;          // between the published points, never beyond them

let wanted = Criteria {
    kind: Some(Kind::Laminate),
    min_tg_c: Some(170.0),
    max_df: Some(0.012),
    at_ghz: Some(1.0),
    ..Criteria::default()
};
let found = db.search(&wanted);
let for_the_machine = db.bungard_summaries();   // name, thicknesses, copper, panels, Tg, Dk, Df, thermal conductivity
```

Dk and Df are interpolated linearly in the logarithm of the frequency. A frequency below the lowest or above the highest published point is an error; so is a list with no frequencies. When a sheet gives the same frequency twice (another resin content or another method) the first one listed is used.

## The machining family (`machining`, `data/machining/`)

The second family describes the things a CNC router or a laser cuts, mills or engraves: 55 workpieces (woods and boards, acrylic, engineering plastics, metals, board stock, paper, leather, fabric, rubber, foam, glass and ceramic), each with an identifier `<family>/<name>`, its name in the eight languages, density, commercial thicknesses, what a laser may do with it and starting points. It has its own files, its own explicit list (`src/machining/data.rs`) and its own checks; it does not touch the board family, and a board stock record names its laminate with `pcb_material_id`.

* **Laser safety:** `laser_cut_allowed`, `laser_engrave_allowed`, `laser_unsafe` (never put it under a laser: PVC, vinyl, ABS, acetal, nylon, PTFE, polycarbonate, neoprene, polyurethane foam, chrome tanned and imitation leather, carbon fibre, glass fibre boards) and the `hazards` behind a refusal or a care, each with its sentence in the eight languages.
* **Starting points:** `cnc` (operation, cutter, diameter, spindle speed, feed, plunge feed, depth of a pass, step over) and `laser` (kind of laser, cut or engraving, nominal power, thickness, power setting, speed, passes), each with its `source` and `verified`. The seed is the maintainer's table of practical starting values plus a few public figures under the MIT licence (see `THIRD_PARTY_NOTICES`), and none of it is verified. A table that gives a range keeps both ends in `ranges` and the middle in the single value; the points can also say the cutter style and coating, the coolant, the machine, the strength of the air assist, the unit the source gave the speed in, the `evidence` and a `source_info` with origin, date and licence. A second figure from another source is an `alternative`: it is listed (`Workpiece::laser_alternatives`) and never used for a start. `verified` is refused unless the evidence is a test of the maker or on a machine. See `docs/MACHINING.md`.
* **Queries:** `Workpiece::cnc_start(op, tool, diameter_mm)` and `Workpiece::laser_start(power_w, thickness_mm, process)` return the point itself or a straight line between the two nearest known ones, and between two known powers where both curves cover the thickness; outside what is known the answer is a `StartWarning` with its text in the eight languages, never an extrapolation. `MachiningDatabase` adds the lookups by identifier and by family and the same two queries by identifier.

```rust
use eh3d_materials::machining::{LaserProcess, MachiningDatabase};

let db = MachiningDatabase::embedded()?;
assert_eq!(db.laser_cut_allowed("plastic/rigid-pvc"), Some(false));
let start = db.laser_start("wood/birch-plywood", 40.0, 4.5, LaserProcess::CO2_CUT)?;
assert!(start.interpolated && !start.verified);
```

The codes `EH3D-MAT-010` to `EH3D-MAT-015` belong to this family. The guide `docs/MACHINING.md` has the schema field by field, the queries, the codes, how a material or a point is added and how PHOENIX-LASER and PHOENIX-CNC will read their materials from it.

## The catalogue of documents (`data/documents.json`)

One record per file of the reference folders (not only laminates): the file name, the folder, the number of pages, the manufacturer, the category, the kind of document, whether it has a text layer, and what it serves. It holds metadata only: no local paths and no text of the documents.

Category: `laminate`, `prepreg`, `bonding_film`, `flex` (flexible laminates and coverlays), `metal_core`, `ptfe_rf` (PTFE and RF materials), `copper_foil`, `solder_mask`, `photoresist`, `thermal_interface`, `foam_gasket`, `adhesive_encapsulant`, `process_guide`, `msds` (safety data sheets), `certificate`, `brochure`, `standard`, `equipment` and `other`.

Kind of document (`doc_type`): `datasheet`, `processing_guide`, `msds`, `certificate`, `brochure`, `comparison`, `standard`, `image` and `unknown`.

Relevance, given for each consumer: `cnc_machining` (a CNC that drills and mills boards: rigid laminates, metal cores, PTFE and RF materials, processing guides for drilling and milling) and `pcb_design` (the design tool: laminates, prepregs, bonding films, flex, solder mask, copper, metal cores, PTFE and RF). The values are `yes` (it serves), `reference` (supporting documentation only) and `no` (it does not serve). Foams, gaskets, adhesives and certificates are `no` or `reference`.

Status: `auto` (classified by keywords on the name and the text, not read) or `checked` (read by a person). Only `checked` is reliable. When a document defines materials of the database, `material_ids` lists them.

Queries: `documents_by_category`, `documents_by_manufacturer`, `documents_by_relevance`, `documents_useful_for_cnc`, `documents_useful_for_phoenix`, and the views `materials_useful_for_cnc` and `materials_useful_for_phoenix` (the materials named by a document that serves the consumer, or that cite such a document as a source).

## Not loaded yet

* Seven Isola processing guides of the reference folder are unreadable (919 bytes each, probably a failed download) and have to be downloaded again: the guides of 370HR, DE104, I-Speed, IS580G, Tachyon 100G, TerraGreen 400G and TerraGreen 400GE.
* Four multi-manufacturer overviews (a microwave laminate comparison chart, a laminate properties guide, a guide to the IPC slash sheets and a laminate guide) are classified in the catalogue of documents but their rows are not loaded as materials: they repeat, with fewer columns, what the datasheets of the makers give.
* The aluminium base laminates of Yuxinda have a resistivity whose exponent cannot be read in the scan; it is left empty.

The crate opens no window, no file and no network.
