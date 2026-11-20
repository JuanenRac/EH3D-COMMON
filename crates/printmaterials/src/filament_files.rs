//! The list of the embedded filament files (written by `tools/import_open_filament_db.py`).

/// The embedded files as `(file name, JSON text)`.
pub const FILES: &[(&str, &str)] = &[
    (
        "3dxtech.json",
        include_str!("../data/filaments/3dxtech.json"),
    ),
    (
        "add-north.json",
        include_str!("../data/filaments/add-north.json"),
    ),
    ("amolen.json", include_str!("../data/filaments/amolen.json")),
    (
        "anycubic.json",
        include_str!("../data/filaments/anycubic.json"),
    ),
    (
        "bambu-lab.json",
        include_str!("../data/filaments/bambu-lab.json"),
    ),
    (
        "colorfabb.json",
        include_str!("../data/filaments/colorfabb.json"),
    ),
    (
        "creality.json",
        include_str!("../data/filaments/creality.json"),
    ),
    ("elegoo.json", include_str!("../data/filaments/elegoo.json")),
    ("eryone.json", include_str!("../data/filaments/eryone.json")),
    (
        "esun-3d.json",
        include_str!("../data/filaments/esun-3d.json"),
    ),
    (
        "fiberlogy.json",
        include_str!("../data/filaments/fiberlogy.json"),
    ),
    (
        "formfutura.json",
        include_str!("../data/filaments/formfutura.json"),
    ),
    (
        "geeetech.json",
        include_str!("../data/filaments/geeetech.json"),
    ),
    (
        "generic.json",
        include_str!("../data/filaments/generic.json"),
    ),
    (
        "hatchbox.json",
        include_str!("../data/filaments/hatchbox.json"),
    ),
    ("jayo.json", include_str!("../data/filaments/jayo.json")),
    (
        "kingroon.json",
        include_str!("../data/filaments/kingroon.json"),
    ),
    (
        "overture.json",
        include_str!("../data/filaments/overture.json"),
    ),
    (
        "polymaker.json",
        include_str!("../data/filaments/polymaker.json"),
    ),
    (
        "protopasta.json",
        include_str!("../data/filaments/protopasta.json"),
    ),
    (
        "prusament.json",
        include_str!("../data/filaments/prusament.json"),
    ),
    (
        "qidi-tech.json",
        include_str!("../data/filaments/qidi-tech.json"),
    ),
    (
        "sainsmart.json",
        include_str!("../data/filaments/sainsmart.json"),
    ),
    (
        "spectrum.json",
        include_str!("../data/filaments/spectrum.json"),
    ),
    ("sunlu.json", include_str!("../data/filaments/sunlu.json")),
];
