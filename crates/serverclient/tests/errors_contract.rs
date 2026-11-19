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
