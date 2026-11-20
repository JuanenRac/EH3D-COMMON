# eh3d-printmaterials

The database of 3D printing materials of the EH3D suite: FDM filaments and photopolymer resins (SLA, DLP, LCD, MSLA). The slicer reads the whole record of a material (temperatures, flow, drying, exposure by printer and layer height); any other program can read the same data and ask the same questions. It is the twin of `eh3d-materials` (board laminates), written with the same rigour.

Nothing here is invented and nothing is claimed to be tried on a machine. Every value is a fact a maker publishes (or a typical range of a family, in the `Generic` profiles), every record names where it comes from (`source`, `source_ref`), a value the sheet does not give is `null` or absent, and every record, and every exposure point, says `verified: false`. There is no price in the data: the price is the user's, and the cost functions take it as an argument.

## What is in `data/`

* `data/filaments/<manufacturer>.json`: `{"schema_version": 1, "manufacturer": "...", "filaments": [...]}`. The seed holds the most used lines of the main brands (imported from the Open Filament Database, MIT, see `THIRD_PARTY_NOTICES`) and the twelve `Generic` profiles of the slicer. The list of embedded files is written by `tools/import_open_filament_db.py` into `src/filament_files.rs`.
* `data/resins/<manufacturer>.json`: `{"schema_version": 1, "manufacturer": "...", "resins": [...]}`, with the seven `Generic` profiles of the slicer. The list of embedded files is written out in `src/data.rs`. A test fails when a file of a folder is not in its list.

The identifier is `<manufacturer>/<name>` in lower case with hyphens. The fields (all optional except `id`, `manufacturer`, `name`, the kind or material, `source` and `verified`) are in `src/model.rs`. A field the schema does not know is refused.

**Filament:** material (`pla`, `petg`, `abs`, `asa`, `tpu`, `tpe`, `pa`, `pc`, `pp`, `pva`, `bvoh`, `hips`, `pet`, `ppa`, `pps`, `peek`, `pei`, `other`), `grade` (PLA+, PA12...), `fill` (carbon fibre, glass fibre, wood, metal, glitter, glow, mineral), Shore hardness, diameters and tolerance, density, colours (name, hex, optional transmission distance), temperatures of nozzle, bed and chamber (min, max, recommended), maximum speed and volumetric flow, part fan by range and by stretch of layers, retraction, drying and moisture sensitivity, abrasive, smallest nozzle, enclosure and direct drive advice, spools (weight, diameter, width, hub), Tg, HDT, tensile strength and modulus, compatibility (printers and nozzles), links of the maker, notes.

**Resin:** kind (`standard`, `hard`, `flexible`, `abs_like`, `castable`, `dental`, `water_washable`, `high_temperature`, `ceramic`, `clear`, `high_detail`, `other`), wavelength, viscosity, density, Shore D and A, Tg, HDT, shrinkage, tensile, flexural and impact strength, elongation, water absorption, colours, translucency, the file name of the safety data sheet, post-cure, wash, the recommended layer heights and the exposure settings by printer and layer height (`exposures`: normal and base exposure with their lowest and highest, base layers, lift and retract speeds, lift distances, waits).

## Checks

Done when the data is loaded, and by a test over every file: known schema version, unique identifiers that agree with the manufacturer and the name, every value inside the range it can have (a nozzle at 2000 C is a typing error), ranges with `min <= recommended <= max`, fan spans in order of layer, colours as `#RRGGBB`, a source and `https` links, one exposure per printer and layer height, and no record marked verified.

## Use

```rust
use eh3d_printmaterials::{Database, FilamentCriteria, Lang, Material};

let db = Database::embedded().expect("the embedded data is sound");
let pla = db.filament("bambu-lab/pla-basic").unwrap();
let (low, high) = pla.nozzle_range().unwrap();
let per_metre = pla.grams_per_meter(None).unwrap();   // from the density and the diameter
let drying = pla.drying_plan();                       // temperature and hours, when the record says
let wears_brass = pla.needs_hardened_nozzle();

let wanted = FilamentCriteria {
    material: Some(Material::Petg),
    nozzle_c: Some(240.0),
    needs_hardened_nozzle: Some(false),
    ..Default::default()
};
let found = db.search_filaments(&wanted);

let resin = db.resin("generic/generic-tough-abs-like-resin").unwrap();
match resin.exposure_for("generic-msla", 0.05) {
    Ok(exposure) => println!("{} s, {:?}", exposure.normal_s, exposure.notes()),
    Err(warning) => println!("{}", warning.text(Lang::Pt)),   // never an extrapolation
}
```

`exposure_for` gives the point itself when the printer and the layer height are in the data, a linear interpolation between the two nearest layer heights of that printer when it falls between them, and a warning (in eight languages) when it falls outside or the printer is unknown. The cost functions (`cost_per_gram`, `cost_per_meter`) take the price of a spool from the caller.

## Errors and languages

The errors carry the stable codes `EH3D-PMAT-001` to `EH3D-PMAT-009` (`PmCode`); `PmCode::title_in(Lang)` gives the title in the eight languages of the suite (en, es, de, fr, it, ja, zh and pt of Portugal), as do the warnings and the notes of an exposure.

The crate opens no window, no file and no network.
