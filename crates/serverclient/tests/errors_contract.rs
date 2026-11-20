//! The table of errors against the catalog of the server. The contract test is skipped when the sibling repository is not next to this one.

use eh3d_serverclient::{all_codes, lookup, text_for, Json, Lang};
use std::path::PathBuf;

fn catalog() -> Option<String> {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // crates/serverclient -> crates -> the repository -> the folder that holds the siblings
    let sibling = here
        .parent()?
        .parent()?
        .parent()?
        .join("EH3D-SERVER")
        .join("docs")
        .join("error-codes.json");
    std::fs::read_to_string(sibling).ok()
}

#[test]
fn the_table_is_sorted_and_has_no_repeated_code() {
    let codes: Vec<&str> = all_codes().map(|e| e.code).collect();
    assert!(codes.len() > 100);
    let mut sorted = codes.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(codes, sorted, "the lookup is a binary search");
}

#[test]
fn every_entry_has_eight_texts_that_are_not_empty() {
    for e in all_codes() {
        assert!((400..600).contains(&e.status), "{}", e.code);
        for lang in Lang::ALL {
            assert!(!e.text(lang).trim().is_empty(), "{} {:?}", e.code, lang);
        }
    }
}

#[test]
fn the_common_codes_are_there_with_their_status() {
    for (code, status) in [
        ("bad_credentials", 401),
        ("unauthorized", 401),
        ("forbidden", 403),
        ("unknown_session", 404),
        ("throttled", 429),
        ("bad_app", 422),
    ] {
        let e = lookup(code).unwrap_or_else(|| panic!("{code} is missing"));
        assert_eq!(e.status, status, "{code}");
    }
    assert!(lookup("zzz_not_a_code").is_none());
    assert!(text_for("bad_credentials", Lang::Ja).contains('\u{30d1}'));
}

#[test]
fn the_table_is_the_catalog_of_the_server() {
    let Some(text) = catalog() else {
        eprintln!("skipped: EH3D-SERVER is not next to this repository");
        return;
    };
    let json = Json::parse(&text).expect("error-codes.json reads");
    let items = json.as_array().expect("a list");
    let mut server: Vec<(String, u64)> = items
        .iter()
        .map(|i| {
            (
                i.get("code").and_then(Json::as_str).unwrap().to_string(),
                i.get("status").and_then(Json::as_u64).unwrap(),
            )
        })
        .collect();
    server.sort();
    let ours: Vec<(String, u64)> = all_codes()
        .map(|e| (e.code.to_string(), u64::from(e.status)))
        .collect();
    assert_eq!(
        ours, server,
        "the table of errors is not the catalog of the server: run tools/gen_server_errors.py"
    );
}

/// The codes the server added for the laser, the pick-and-place, the printers, the farm, the slicer and the Gerber reader, with their status.
const NEW_CODES: [(&str, u16); 36] = [
    ("air_assist_off", 409),
    ("bad_axis", 422),
    ("bad_feed", 422),
    ("bad_gerber", 422),
    ("bad_heater", 422),
    ("bad_mesh", 422),
    ("bad_operation", 422),
    ("bad_placement_list", 422),
    ("bad_rules", 422),
    ("compat_off", 404),
    ("compat_write_off", 404),
    ("enclosure_open", 409),
    ("farm_no_free_printer", 409),
    ("focus_not_checked", 409),
    ("heater_timeout", 409),
    ("input_too_large", 413),
    ("jog_while_running", 409),
    ("laser_estop_latched", 409),
    ("laser_feed_limit", 422),
    ("laser_power_limit", 422),
    ("machine_alarm", 409),
    ("missing_layer", 422),
    ("out_of_travel", 422),
    ("pnp_feeder_short", 409),
    ("printer_disconnected", 502),
    ("printer_latched", 409),
    ("printer_untrusted", 502),
    ("program_over_limit", 422),
    ("sensor_fault", 409),
    ("slice_failed", 502),
    ("slice_limit", 422),
    ("slice_off", 404),
    ("temperature_over_limit", 422),
    ("test_fire_refused", 422),
    ("thermal_runaway", 409),
    ("unit_unknown", 422),
];

#[test]
fn the_codes_of_the_laser_the_printers_the_farm_and_the_slicer_are_there_in_eight_languages() {
    for (code, status) in NEW_CODES {
        let e = lookup(code).unwrap_or_else(|| panic!("{code} is missing"));
        assert_eq!(e.status, status, "{code}");
        let english = e.text(Lang::En);
        assert!(english.ends_with('.'), "{code}: {english}");
        for lang in Lang::ALL.into_iter().skip(1) {
            let t = e.text(lang);
            assert!(!t.trim().is_empty(), "{code} {lang:?}");
            assert_ne!(t, english, "{code} {lang:?} is the English text");
            assert_eq!(t, t.trim(), "{code} {lang:?}");
            assert!(!t.contains('\n') && !t.contains("TODO") && !t.contains("???"));
        }
        // Chinese and Japanese are written in their own letters
        for lang in [Lang::Zh, Lang::Ja] {
            assert!(e.text(lang).chars().any(|c| (c as u32) > 0x2e80), "{code}");
        }
    }
}

#[test]
fn the_portuguese_of_the_new_texts_is_the_one_of_portugal() {
    for (code, _) in NEW_CODES {
        let t = lookup(code).unwrap().text(Lang::Pt).to_lowercase();
        for brazilian in ["arquivo", "tela", "você", "celular", "usuário", "gerenciar"] {
            assert!(!t.contains(brazilian), "{code}: {t}");
        }
    }
    assert!(lookup("bad_gerber")
        .unwrap()
        .text(Lang::Pt)
        .contains("ficheiro"));
    assert!(lookup("printer_latched")
        .unwrap()
        .text(Lang::Pt)
        .contains("impressora"));
}

#[test]
fn the_two_refusals_of_the_rule_of_the_tag_have_their_codes_and_texts() {
    for (code, status) in [("precondition_required", 428), ("precondition_failed", 412)] {
        let e = lookup(code).unwrap_or_else(|| panic!("{code} is missing"));
        assert_eq!(e.status, status);
        let mut texts: Vec<&str> = Lang::ALL.iter().map(|l| e.text(*l)).collect();
        texts.sort_unstable();
        texts.dedup();
        assert_eq!(texts.len(), 8, "{code}");
    }
}

#[test]
fn every_new_code_of_the_server_catalog_has_a_text_when_the_sibling_is_there() {
    let Some(text) = catalog() else {
        eprintln!("skipped: EH3D-SERVER is not next to this repository");
        return;
    };
    let json = Json::parse(&text).expect("error-codes.json reads");
    for item in json.as_array().expect("a list") {
        let code = item.get("code").and_then(Json::as_str).unwrap();
        let e = lookup(code).unwrap_or_else(|| {
            panic!("{code} has no row: run tools/gen_server_errors.py (and add its texts)")
        });
        for lang in Lang::ALL {
            assert!(!e.text(lang).is_empty(), "{code} {lang:?}");
        }
    }
}
