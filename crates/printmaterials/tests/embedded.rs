//! Tests over the data embedded in the crate: they run on every file of `data/`.

use eh3d_printmaterials::{
    validate, Database, FilamentCriteria, Lang, Material, PmCode, ResinCriteria, ResinKind,
};

fn db() -> Database {
    match Database::embedded() {
        Ok(db) => db,
        Err(errors) => {
            let text: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
            panic!("the embedded data is not sound:\n{}", text.join("\n"));
        }
    }
}

#[test]
fn the_embedded_data_loads_and_passes_every_rule() {
    let db = db();
    assert!(db.filaments().len() >= 300, "{}", db.filaments().len());
    assert!(db.resins().len() >= 7);
}

#[test]
fn every_record_names_its_source_and_none_is_verified() {
    let db = db();
    for f in db.filaments() {
        assert!(!f.source.is_empty() && !f.verified, "{}", f.id);
        if f.source == "open-filament-database" {
            let path = f.source_ref.as_deref().unwrap_or("");
            assert_eq!(path.split('/').count(), 3, "{}: {path}", f.id);
        }
        assert_eq!(f.id, validate::make_id(&f.manufacturer, &f.name));
    }
    for r in db.resins() {
        assert!(!r.source.is_empty() && !r.verified, "{}", r.id);
        assert!(r.exposures.iter().all(|e| !e.verified), "{}", r.id);
    }
}

#[test]
fn the_third_party_notice_carries_the_licence_of_the_seed() {
    let notice = include_str!("../THIRD_PARTY_NOTICES");
    assert!(notice.contains("Permission is hereby granted, free of charge"));
    assert!(notice.contains("OpenFilamentCollective"));
    let db = db();
    let from_the_seed = db
        .filaments()
        .iter()
        .filter(|f| f.source == "open-filament-database")
        .count();
    assert!(from_the_seed > 0);
}

#[test]
fn the_generic_profiles_of_the_slicer_are_all_there() {
    let db = db();
    let generic: Vec<_> = db
        .filaments()
        .iter()
        .filter(|f| f.manufacturer == "Generic")
        .collect();
    assert_eq!(generic.len(), 12);
    let pla = db.filament("generic/generic-pla").unwrap();
    assert_eq!(pla.nozzle_range(), Some((190.0, 230.0)));
    assert_eq!(pla.nozzle_c.unwrap().recommended, Some(210.0));
    assert_eq!(pla.source_ref.as_deref(), Some("generic-pla"));
    assert!(!pla.needs_hardened_nozzle());
    assert!(db
        .filament("generic/generic-pla-with-carbon-fibre")
        .unwrap()
        .needs_hardened_nozzle());
    assert!(
        db.filament("generic/generic-abs")
            .unwrap()
            .enclosure_advised
            == Some(true)
    );
    let resins: Vec<_> = db
        .resins()
        .iter()
        .filter(|r| r.manufacturer == "Generic")
        .collect();
    assert_eq!(resins.len(), 7);
    let tough = db.resin("generic/generic-tough-abs-like-resin").unwrap();
    assert_eq!(tough.kind, ResinKind::AbsLike);
    let e = tough.exposure_for("generic-msla", 0.05).unwrap();
    assert_eq!(
        (e.normal_s, e.base_s, e.base_layers),
        (3.0, Some(28.0), Some(5))
    );
    assert!(!e.verified && !e.interpolated);
    // the generic profiles are for 0.05 mm (0.025 for high detail): nothing outside is invented
    assert!(tough.exposure_for("generic-msla", 0.1).is_err());
    let detail = db.resin("generic/generic-high-detail-resin").unwrap();
    assert!(detail.exposure_for("generic-msla", 0.025).is_ok());
    assert!(detail.exposure_for("generic-msla", 0.05).is_err());
}

#[test]
fn the_searches_work_on_the_real_data() {
    let db = db();
    let abrasive = db.search_filaments(&FilamentCriteria {
        needs_hardened_nozzle: Some(true),
        ..Default::default()
    });
    assert!(!abrasive.is_empty());
    assert!(abrasive.iter().all(|f| f.needs_hardened_nozzle()));
    let petg = db.search_filaments(&FilamentCriteria {
        material: Some(Material::Petg),
        nozzle_c: Some(240.0),
        ..Default::default()
    });
    assert!(!petg.is_empty());
    assert!(petg
        .iter()
        .all(|f| f.material == Material::Petg && f.nozzle_range().is_some()));
    let on_printer = db.search_resins(&ResinCriteria {
        printer: Some("generic-msla".into()),
        layer_mm: Some(0.05),
        ..Default::default()
    });
    assert!(on_printer.len() >= 5);
}

#[test]
fn real_filaments_weigh_what_physics_says() {
    let db = db();
    for f in db.filaments() {
        if let Some(g) = f.grams_per_meter(None) {
            // between the lightest and the heaviest filament of 1.75 or 2.85 mm
            assert!((0.5..=60.0).contains(&g), "{}: {g}", f.id);
        }
    }
}

#[test]
fn the_ranges_of_the_data_are_in_order_and_the_manufacturers_are_distinct() {
    let db = db();
    for f in db.filaments() {
        if let Some((lo, hi)) = f.nozzle_range() {
            assert!(lo <= hi, "{}", f.id);
        }
    }
    let names = db.filament_manufacturers();
    assert!(names.len() >= 20, "{names:?}");
    assert!(db.resin_manufacturers().contains(&"Generic"));
}

#[test]
fn every_code_has_a_title_in_the_eight_languages() {
    for lang in Lang::ALL {
        for code in PmCode::ALL {
            assert!(!code.title_in(lang).is_empty());
        }
    }
}
