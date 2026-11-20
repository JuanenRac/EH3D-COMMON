//! Tests over the whole seed of the machining family: what is loaded, what must never change
//! (the materials a laser must not touch), the figures that carry a licence, the schema of the
//! files and a reader that is fed damaged text.

use super::data;
use super::model::{
    Evidence, Family, Hazard, LaserKind, LaserMode, LaserProcess, SpeedUnit, Workpiece,
};
use super::query::MachiningDatabase;
use super::validate::{check_workpiece, POISONING_HAZARDS};
use crate::error::MatCode;

fn db() -> MachiningDatabase {
    MachiningDatabase::embedded().unwrap()
}

/// The materials that a laser must never touch, as they were before the update of the figures.
/// This list may grow; a material may never leave it.
const NEVER_UNDER_A_LASER: [&str; 17] = [
    "composite/carbon-fibre-sheet",
    "foam/polyurethane-foam",
    "leather/chrome-tanned-leather",
    "leather/faux-leather-pu-or-pvc",
    "pcb_stock/cem-1-copper-clad-board",
    "pcb_stock/fr-2-copper-clad-board",
    "pcb_stock/fr-4-copper-clad-board",
    "pcb_stock/fr-4-unclad-sheet",
    "plastic/abs",
    "plastic/acetal-pom",
    "plastic/adhesive-vinyl",
    "plastic/foam-pvc-board",
    "plastic/nylon-pa6",
    "plastic/polycarbonate-pc",
    "plastic/ptfe",
    "plastic/rigid-pvc",
    "rubber/neoprene",
];

#[test]
fn the_seed_has_the_materials_of_the_updated_table_and_three_more() {
    let db = db();
    assert_eq!(db.len(), 55);
    let count = |f: Family| db.by_family(f).count();
    assert_eq!(count(Family::Wood), 14);
    assert_eq!(count(Family::Plastic), 14);
    assert_eq!(count(Family::PcbStock), 4);
    assert_eq!(count(Family::Fabric), 4);
    assert_eq!(count(Family::Foam), 4);
    assert_eq!(count(Family::Paper), 4);
    assert_eq!(count(Family::Metal), 3);
    assert_eq!(count(Family::Leather), 3);
    assert_eq!(count(Family::Mineral), 2);
    assert_eq!(count(Family::Rubber), 2);
    assert_eq!(count(Family::Composite), 1);
    let cnc: usize = db.materials().iter().map(|w| w.cnc.len()).sum();
    let laser: usize = db.materials().iter().map(|w| w.laser.len()).sum();
    let alternatives: usize = db
        .materials()
        .iter()
        .map(|w| w.laser.iter().filter(|p| p.alternative).count())
        .sum();
    assert_eq!((cnc, laser, alternatives), (137, 112, 12));
    for id in ["wood/cherry", "fabric/denim", "paper/mat-board"] {
        let w = db.get(id).unwrap_or_else(|| panic!("{id}"));
        assert!(w.laser_cut_allowed() && !w.laser.is_empty(), "{id}");
    }
}

#[test]
fn no_material_a_laser_must_not_touch_was_loosened() {
    let db = db();
    for id in NEVER_UNDER_A_LASER {
        let w = db.get(id).unwrap_or_else(|| panic!("{id} is gone"));
        assert!(w.laser_unsafe(), "{id}");
        assert!(!w.laser_cut_allowed() && !w.laser_engrave_allowed(), "{id}");
        assert!(w.laser.is_empty(), "{id} has laser points");
        for process in [LaserProcess::CO2_CUT, LaserProcess::DIODE_CUT] {
            assert!(w.laser_alternatives(process).is_empty(), "{id}");
            assert!(w.laser_start(40.0, 3.0, process).is_err(), "{id}");
        }
    }
    // the unsafe ones are exactly the list: nothing was added without a decision
    assert_eq!(db.laser_unsafe().count(), NEVER_UNDER_A_LASER.len());
}

#[test]
fn chrome_leather_imitation_leather_pvc_abs_and_the_fibres_have_no_laser_setting() {
    let db = db();
    for id in [
        "leather/chrome-tanned-leather",
        "leather/faux-leather-pu-or-pvc",
        "plastic/rigid-pvc",
        "plastic/foam-pvc-board",
        "plastic/adhesive-vinyl",
        "plastic/abs",
        "composite/carbon-fibre-sheet",
        "pcb_stock/fr-4-copper-clad-board",
        "pcb_stock/fr-4-unclad-sheet",
    ] {
        let w = db.get(id).unwrap();
        assert!(w.laser.is_empty() && w.laser_unsafe(), "{id}");
    }
}

#[test]
fn a_material_with_a_poisoning_hazard_is_never_cut_or_engraved() {
    for w in db().materials() {
        if w.safety
            .hazards
            .iter()
            .any(|h| POISONING_HAZARDS.contains(h))
        {
            assert!(
                !w.safety.laser_cut_allowed && !w.safety.laser_engrave_allowed,
                "{}",
                w.id
            );
            assert!(w.laser.is_empty(), "{}", w.id);
        }
    }
    assert!(POISONING_HAZARDS.contains(&Hazard::Chlorine));
    assert!(POISONING_HAZARDS.contains(&Hazard::Cyanide));
    assert!(POISONING_HAZARDS.contains(&Hazard::Fluorine));
    assert!(POISONING_HAZARDS.contains(&Hazard::Chromium));
    assert!(POISONING_HAZARDS.contains(&Hazard::ToxicFumes));
}

#[test]
fn nothing_in_the_seed_is_verified_and_no_evidence_is_above_a_public_reference() {
    for w in db().materials() {
        for p in &w.cnc {
            assert!(!p.verified, "{}", w.id);
            assert!(
                matches!(
                    p.evidence,
                    Some(Evidence::GeneralPractice | Evidence::PublicReference)
                ),
                "{}",
                w.id
            );
            assert!(p.source_info.is_some(), "{}", w.id);
        }
        for p in &w.laser {
            assert!(!p.verified, "{}", w.id);
        }
    }
}

#[test]
fn a_second_figure_is_a_co2_one_from_the_mit_calculator_with_its_attribution() {
    let db = db();
    let mut seen = 0;
    for w in db.materials() {
        for p in w.laser.iter().filter(|p| p.alternative) {
            seen += 1;
            assert_eq!(p.kind, LaserKind::Co2, "{}", w.id);
            let info = p.source_info.as_ref().unwrap();
            assert_eq!(info.origin, "HobbyistLaserCalc");
            assert_eq!(info.license, "MIT");
            assert!(info.url.as_deref().unwrap().starts_with("https://"));
            assert_eq!(p.evidence, Some(Evidence::PublicReference));
            assert_eq!(p.source_unit, Some(SpeedUnit::MmS));
            assert!(!p.verified);
        }
    }
    assert_eq!(seen, 12);
    // the main figure of the owner's table is what a start returns, the second one is listed
    let acrylic = db.get("plastic/cast-acrylic-pmma").unwrap();
    let main = acrylic
        .laser_start(40.0, 3.0, LaserProcess::CO2_CUT)
        .unwrap();
    assert_eq!(main.power_pct, 62.5);
    assert_eq!(main.speed_mm_min, 425.0);
    let alt = acrylic.laser_alternatives(LaserProcess::CO2_CUT);
    assert_eq!(alt.len(), 4);
    assert!(alt
        .iter()
        .any(|p| p.power_pct == 65.0 && p.speed_mm_min == 900.0));
    // 20 mm/s is 1200 mm/min
    assert!(alt
        .iter()
        .any(|p| p.power_pct == 55.0 && p.speed_mm_min == 1200.0));
}

#[test]
fn only_the_materials_that_allow_it_have_a_second_figure() {
    let db = db();
    for w in db.materials() {
        for p in w.laser.iter().filter(|p| p.alternative) {
            let allowed = match p.mode {
                LaserMode::Cut => w.laser_cut_allowed(),
                LaserMode::Engrave => w.laser_engrave_allowed(),
            };
            assert!(allowed, "{}", w.id);
        }
    }
}

#[test]
fn the_ranges_of_the_table_keep_both_ends_and_hold_the_single_value() {
    let db = db();
    // a feed range of the owner's table: birch plywood, contour, 3 mm
    let birch = db.get("wood/birch-plywood").unwrap();
    let p = birch
        .cnc
        .iter()
        .find(|p| p.diameter_mm == 3.0 && p.op == super::model::CncOp::Contour)
        .unwrap();
    assert_eq!(p.rpm, 19000.0);
    let r = p.ranges.as_ref().unwrap();
    assert_eq!(r.rpm, Some([18000.0, 20000.0]));
    assert_eq!(r.feed_mm_min, Some([900.0, 1100.0]));
    assert_eq!(p.feed_mm_min, 1000.0);
    // a diameter that is a range becomes a point at each end
    let carbon = db.get("composite/carbon-fibre-sheet").unwrap();
    let diameters: Vec<f64> = carbon.cnc.iter().map(|p| p.diameter_mm).collect();
    assert_eq!(diameters, [1.0, 3.0]);
    assert_eq!(
        carbon.cnc[0].coating.as_deref(),
        Some("diamond coated or PCD")
    );
    // a V-bit of the board stock has its tip angle
    let fr4 = db.get("pcb_stock/fr-4-copper-clad-board").unwrap();
    let vbit = fr4.cnc.iter().find(|p| p.tip_angle_deg.is_some()).unwrap();
    assert_eq!(vbit.tip_angle_deg, Some([30.0, 60.0]));
    // aluminium is cut with a lubricant
    let al = db.get("metal/aluminium-6061").unwrap();
    assert!(al
        .cnc
        .iter()
        .all(|p| p.coolant == Some(super::model::Coolant::Lubricant)));
}

#[test]
fn a_speed_given_in_mm_per_second_is_stored_in_mm_per_minute() {
    for w in db().materials() {
        for p in w.laser.iter().filter(|p| p.source_unit.is_some()) {
            let unit = p.source_unit.unwrap();
            let speed = p.source_speed.unwrap();
            assert!(
                (speed * unit.to_mm_min() - p.speed_mm_min).abs() < 0.01,
                "{}",
                w.id
            );
        }
    }
}

#[test]
fn the_names_of_the_new_materials_exist_in_the_eight_languages() {
    let db = db();
    for id in ["wood/cherry", "fabric/denim", "paper/mat-board"] {
        let w = db.get(id).unwrap();
        let names = [
            &w.names.en,
            &w.names.es,
            &w.names.de,
            &w.names.fr,
            &w.names.it,
            &w.names.ja,
            &w.names.zh,
            &w.names.pt,
        ];
        assert!(names.iter().all(|n| !n.trim().is_empty()), "{id}");
        assert!(!w.names.ja.is_ascii() && !w.names.zh.is_ascii(), "{id}");
    }
}

#[test]
fn every_workpiece_survives_a_round_trip_through_json() {
    for w in db().materials() {
        let json = serde_json::to_string(w).unwrap();
        let again: Workpiece = serde_json::from_str(&json).unwrap();
        assert_eq!(&again, w, "{}", w.id);
        assert!(check_workpiece(&again).is_empty(), "{}", w.id);
    }
}

#[test]
fn a_record_written_before_the_new_fields_still_reads_and_checks() {
    let old = r#"{"id":"wood/old","family":"wood",
        "names":{"en":"Old","es":"Old","de":"Old","fr":"Old","it":"Old","ja":"Old","zh":"Old","pt":"Old"},
        "density_g_cm3":0.5,"thicknesses_mm":[3.0],
        "safety":{"laser_cut_allowed":true,"laser_engrave_allowed":true,"laser_unsafe":false,"hazards":["flammable"]},
        "cnc":[{"op":"contour","tool":"end_mill","diameter_mm":3.0,"flutes":2,"rpm":18000,
            "feed_mm_min":1000,"plunge_mm_min":300,"depth_per_pass_mm":1.5,"stepover_pct":null,
            "verified":false,"source":"s"}],
        "laser":[{"kind":"co2","mode":"cut","power_w":40,"thickness_mm":3,"power_pct":60,
            "speed_mm_min":500,"passes":1,"air_assist":true,"verified":false,"source":"s"}],
        "pcb_material_id":null,"notes":null,"source":"s"}"#;
    let w: Workpiece = serde_json::from_str(old).unwrap();
    assert!(check_workpiece(&w).is_empty());
    assert!(w.cnc[0].ranges.is_none() && !w.laser[0].alternative);
    // and it is written back without the new fields
    let out = serde_json::to_string(&w).unwrap();
    for field in [
        "ranges",
        "alternative",
        "evidence",
        "source_info",
        "coolant",
    ] {
        assert!(!out.contains(field), "{field}");
    }
}

#[test]
fn the_files_of_the_seed_are_written_the_way_the_tool_that_wrote_them_writes() {
    // one flat file per group, same order as the list, identifiers in order inside a file
    for (name, json) in data::FILES {
        let file = super::query::parse_file(name, json).unwrap();
        let ids: Vec<&str> = file.materials.iter().map(|w| w.id.as_str()).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(ids, sorted, "{name}");
        assert!(!json.contains('\r'), "{name}");
    }
}

/// A small generator with a fixed seed, so that the damaged texts are the same on every run.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

#[test]
fn a_reader_fed_damaged_text_answers_with_an_error_and_never_panics() {
    let (name, json) = data::FILES[0];
    let bytes = json.as_bytes();
    let mut rng = Lcg(0x5eed_2026);
    let mut refused = 0;
    for _ in 0..400 {
        let mut b = bytes.to_vec();
        for _ in 0..1 + rng.below(4) {
            let at = rng.below(b.len());
            match rng.below(4) {
                0 => b[at] = b"{}[],:\"0-.eE9xX "[rng.below(16)],
                1 => {
                    b.remove(at);
                }
                2 => b.insert(at, b'"'),
                _ => b.truncate(at.max(1)),
            }
        }
        let text = String::from_utf8_lossy(&b).into_owned();
        match MachiningDatabase::from_sources(&[(name, &text)]) {
            Ok(_) => {}
            Err(e) => {
                refused += 1;
                assert!(e.to_string().starts_with("EH3D-MAT-"));
            }
        }
    }
    assert!(refused > 300, "{refused}");
}

#[test]
fn values_pushed_out_of_their_ranges_are_refused_with_their_codes() {
    let (name, json) = data::FILES[5];
    let good = super::query::parse_file(name, json).unwrap();
    let refused = |f: &dyn Fn(&mut super::model::MachiningFile)| {
        let mut file = good.clone();
        f(&mut file);
        let text = serde_json::to_string(&file).unwrap();
        MachiningDatabase::from_sources(&[(name, &text)]).unwrap_err()
    };
    let e = refused(&|f| f.materials[0].cnc[0].rpm = 9_000_000.0);
    assert_eq!(e.code(), MatCode::PointRange);
    let e = refused(&|f| f.materials[0].laser[0].power_pct = 162.5);
    assert_eq!(e.code(), MatCode::PointRange);
    // a range that is reversed
    let e = refused(&|f| {
        let r = f.materials[0].cnc[0].ranges.as_mut().unwrap();
        r.rpm = Some([20000.0, 18000.0]);
    });
    assert_eq!(e.code(), MatCode::Workpiece);
    // a second figure of a material that a laser must not touch
    let e = refused(&|f| f.materials[0].safety.laser_unsafe = true);
    assert!(matches!(
        e.code(),
        MatCode::Workpiece | MatCode::LaserRefused
    ));
}

#[test]
fn every_figure_of_the_calculator_carries_the_notice_of_its_licence() {
    let notice = include_str!("../../THIRD_PARTY_NOTICES");
    assert!(notice.contains("Copyright (c) 2026 hobbyistlasercalc.com"));
    assert!(notice.contains("MIT License"));
    let mut taken = 0;
    for w in db().materials() {
        for p in &w.laser {
            let Some(info) = &p.source_info else { continue };
            if p.source.contains("HobbyistLaserCalc") || info.origin == "HobbyistLaserCalc" {
                taken += 1;
                assert_eq!(info.origin, "HobbyistLaserCalc", "{}", w.id);
                assert_eq!(info.license, "MIT", "{}", w.id);
                assert!(info.url.is_some(), "{}", w.id);
            } else {
                // the rest is the table of the maintainer of the data
                assert_eq!(p.evidence, Some(Evidence::GeneralPractice), "{}", w.id);
            }
        }
    }
    assert_eq!(taken, 24);
}
