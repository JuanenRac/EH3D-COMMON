//! The data of the catalog, the rules it follows and what it answers, with the hostile cases.

use eh3d_printers::catalog::{parse_printer_file, MAX_FILE_BYTES, PRINTER_FILES};
use eh3d_printers::validate::{check_printer, claimed_fields, FIELDS};
use eh3d_printers::{
    starting_filaments, Adapter, Block, Catalog, Caution, Enclosure, Firmware, Label, Lang, Level,
    Multicolor, MulticolorStatus, Printer, PrnCode, Processes, SendFormat,
};
use eh3d_printmaterials::Database;

fn catalog() -> Catalog {
    Catalog::embedded().unwrap()
}

fn a1() -> Printer {
    catalog().find("bambu-lab/a1").unwrap().clone()
}

fn codes(p: &Printer) -> Vec<PrnCode> {
    check_printer(p, None).iter().map(|e| e.code).collect()
}

fn one_file(printer_json: &str) -> String {
    format!(r#"{{"schema_version":1,"manufacturer":"V","printers":[{printer_json}]}}"#)
}

const SMALL: &str = r#"{"id":"v/m","manufacturer":"V","model":"M","volume_mm":[100,100,100],"nozzle_max_c":250,"bed_max_c":90,"adapter_documented":false,"sources":[{"url":"https://example.com/m","covers":["volume_mm","nozzle_max_c","bed_max_c"]}],"verified":false}"#;

fn load(text: &str) -> Vec<PrnCode> {
    match Catalog::from_texts(&[("v.json", text)]) {
        Ok(_) => Vec::new(),
        Err(errors) => errors.iter().map(|e| e.code).collect(),
    }
}

#[test]
fn the_embedded_catalog_is_sound_and_nothing_is_verified() {
    let c = catalog();
    assert_eq!(c.printers().len(), 8);
    for p in c.printers() {
        assert!(!p.verified, "{}", p.id);
        assert!(
            check_printer(p, Some(&p.manufacturer)).is_empty(),
            "{}",
            p.id
        );
        for field in claimed_fields(p) {
            assert!(p.covers(field), "{}: {field}", p.id);
        }
        for s in &p.sources {
            assert!(s.url.starts_with("https://"), "{}", s.url);
            assert!(s.covers.iter().all(|f| FIELDS.contains(&f.as_str())));
        }
    }
}

#[test]
fn every_page_is_on_the_site_of_the_maker() {
    for p in catalog().printers() {
        let site = match p.manufacturer.as_str() {
            "Bambu Lab" => "bambulab.com",
            "Creality" => "creality.com",
            "Artillery" => "artillery3d.com",
            other => panic!("a maker with no site in this test: {other}"),
        };
        for s in &p.sources {
            let host = s
                .url
                .trim_start_matches("https://")
                .split('/')
                .next()
                .unwrap();
            assert!(
                host == site || host.ends_with(&format!(".{site}")),
                "{}: {host}",
                p.id
            );
        }
    }
}

#[test]
fn every_file_of_the_folder_is_embedded() {
    let dir = format!("{}/data/printers", env!("CARGO_MANIFEST_DIR"));
    let mut on_disk: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    on_disk.sort();
    let listed: Vec<String> = PRINTER_FILES.iter().map(|f| f.0.to_string()).collect();
    assert_eq!(on_disk, listed);
}

#[test]
fn the_two_bambu_printers_say_what_their_maker_publishes() {
    let c = catalog();
    let a1 = c.find("bambu-lab/a1").unwrap();
    assert_eq!(a1.volume_mm, [256.0, 256.0, 256.0]);
    assert_eq!((a1.nozzle_max_c, a1.bed_max_c), (300.0, 100.0));
    assert_eq!(a1.enclosure, Enclosure::Open);
    assert_eq!(a1.adapter, Adapter::BambuLan);
    assert!(a1.adapter_documented);
    assert_eq!(a1.multicolor.max_colors, Some(4));
    assert!(a1.send_formats.contains(&SendFormat::Gcode3mf));
    let p1s = c.find("bambu-lab-p1s").unwrap();
    assert_eq!(p1s.enclosure, Enclosure::Enclosed);
    assert_eq!(p1s.heated_chamber, Some(false));
    assert_eq!(p1s.chamber_max_c, None);
    assert_eq!(p1s.multicolor.max_colors, Some(16));
}

#[test]
fn an_adapter_the_maker_does_not_document_is_said_to_be_so() {
    for p in catalog().printers() {
        if p.firmware == Firmware::Klipper {
            assert_eq!(p.adapter, Adapter::KlipperMoonraker, "{}", p.id);
            assert!(!p.adapter_documented, "{}", p.id);
        }
    }
}

#[test]
fn a_printer_is_found_in_both_forms_of_its_identifier() {
    let c = catalog();
    for id in ["creality/k1c", "creality-k1c", " CREALITY/K1C "] {
        assert_eq!(c.find(id).unwrap().model, "K1C", "{id}");
    }
    assert!(c.find("creality").is_none());
    assert!(c.find("").is_none());
    assert_eq!(c.search("artillery").len(), 2);
    assert_eq!(c.search("").len(), c.printers().len());
    assert!(c.search("no such thing").is_empty());
    assert_eq!(c.by_adapter(Adapter::BambuLan).len(), 3);
    assert_eq!(c.by_adapter(Adapter::MarlinSerial).len(), 1);
    assert_eq!(c.by_adapter(Adapter::KlipperMoonraker).len(), 4);
}

#[test]
fn the_small_record_of_the_tests_is_sound() {
    assert_eq!(load(&one_file(SMALL)), Vec::<PrnCode>::new());
}

#[test]
fn text_that_is_not_the_schema_is_refused() {
    let cases = [
        SMALL.replace("100,100,100", "NaN,100,100"),
        SMALL.replace("100,100,100", "1e999,100,100"),
        SMALL.replace("100,100,100", "100,100"),
        SMALL.replace("100,100,100", "100,100,100,100"),
        SMALL.replace(r#""verified":false"#, r#""verified":false,"price":3"#),
        SMALL.replace(
            r#""verified":false"#,
            r#""verified":false,"verified":false"#,
        ),
        SMALL.replace(r#""bed_max_c":90,"#, ""),
        SMALL.replace(r#""bed_max_c":90"#, r#""bed_max_c":null"#),
    ];
    for bad in &cases {
        assert_eq!(load(&one_file(bad)), vec![PrnCode::Parse], "{bad}");
    }
    assert_eq!(load("not json"), vec![PrnCode::Parse]);
    let deep = "[".repeat(10_000);
    assert_eq!(load(&deep), vec![PrnCode::Parse]);
}

#[test]
fn a_version_or_a_size_that_is_not_read_is_refused() {
    let v2 = one_file(SMALL).replace(r#""schema_version":1"#, r#""schema_version":2"#);
    assert_eq!(load(&v2), vec![PrnCode::SchemaVersion]);
    let big = " ".repeat(MAX_FILE_BYTES + 1);
    assert_eq!(load(&big), vec![PrnCode::TooLarge]);
    let many = one_file(&vec![SMALL; 501].join(","));
    let e = parse_printer_file("v.json", &many).unwrap_err();
    assert_eq!(e.code, PrnCode::TooLarge);
}

#[test]
fn the_same_printer_twice_is_refused() {
    let twice = one_file(&format!("{SMALL},{SMALL}"));
    assert_eq!(load(&twice), vec![PrnCode::DuplicateId]);
}

#[test]
fn an_identifier_that_does_not_match_is_refused() {
    let mut p = a1();
    p.id = "bambu-lab/a2".to_string();
    assert_eq!(codes(&p), vec![PrnCode::IdMismatch]);
    let errors = check_printer(&a1(), Some("Another"));
    assert_eq!(errors[0].code, PrnCode::IdMismatch);
}

#[test]
fn figures_out_of_range_and_not_numbers_are_refused() {
    for bad in [f64::NAN, f64::INFINITY, -1.0, 0.0, 1.0e12] {
        let mut p = a1();
        p.volume_mm[2] = bad;
        assert!(codes(&p).contains(&PrnCode::OutOfRange), "{bad}");
        let mut p = a1();
        p.nozzle_max_c = bad;
        assert!(codes(&p).contains(&PrnCode::OutOfRange), "{bad}");
        let mut p = a1();
        p.bed_max_c = bad;
        assert!(codes(&p).contains(&PrnCode::OutOfRange), "{bad}");
        let mut p = a1();
        p.filament_diameter_mm = Some(bad);
        assert!(codes(&p).contains(&PrnCode::OutOfRange), "{bad}");
    }
}

#[test]
fn contradictions_are_refused() {
    let changes: [fn(&mut Printer); 7] = [
        |p| p.nozzles_mm = vec![0.4, 0.2],
        |p| p.nozzle_default_mm = Some(0.5),
        |p| {
            p.heated_chamber = Some(true);
        },
        |p| p.multicolor.status = MulticolorStatus::None,
        |p| p.send_formats = vec![SendFormat::Gcode, SendFormat::Gcode],
        |p| p.verified = true,
        |p| p.adapter = Adapter::Unknown,
    ];
    for (i, change) in changes.iter().enumerate() {
        let mut p = a1();
        change(&mut p);
        let found = codes(&p);
        assert!(
            found.contains(&PrnCode::Inconsistent) || found.contains(&PrnCode::Source),
            "change {i}: {found:?}"
        );
    }
}

#[test]
fn a_name_that_would_break_a_settings_file_is_refused() {
    for bad in [
        "A1\nverified = true",
        "A1 #x",
        "A1|x",
        "A1=x",
        " A1",
        "",
        "A\u{0}1",
        "A\u{2028}1",
    ] {
        let mut p = a1();
        p.model = bad.to_string();
        assert!(codes(&p).contains(&PrnCode::Text), "{bad:?}");
    }
    let mut p = a1();
    p.model = "x".repeat(81);
    assert!(codes(&p).contains(&PrnCode::Text));
    let mut p = a1();
    p.multicolor = Multicolor {
        status: MulticolorStatus::Optional,
        system: Some("AMS\r".to_string()),
        max_colors: None,
    };
    assert!(codes(&p).contains(&PrnCode::Text));
}

#[test]
fn a_page_that_is_not_a_plain_https_address_is_refused() {
    for bad in [
        "http://example.com/a",
        "javascript:alert(1)",
        "file:///etc/passwd",
        "https://",
        "https://localhost/a",
        "https://user@example.com/a",
        "https://example.com/a b",
        "https://example.com/a\n",
    ] {
        let mut p = a1();
        p.sources[0].url = bad.to_string();
        assert!(codes(&p).contains(&PrnCode::Source), "{bad:?}");
    }
}

#[test]
fn a_claim_with_no_page_is_refused() {
    let mut p = a1();
    p.sources
        .retain(|s| !s.covers.iter().any(|c| c == "send_formats"));
    assert_eq!(codes(&p), vec![PrnCode::Source]);
    let mut p = a1();
    p.sources[0].covers.push("price".to_string());
    assert_eq!(codes(&p), vec![PrnCode::Source]);
    let mut p = a1();
    p.sources.clear();
    assert!(codes(&p).iter().all(|c| *c == PrnCode::Source));
    // what is not claimed needs no page
    let mut p = a1();
    p.enclosure = Enclosure::Unknown;
    p.sources.retain(|s| s.covers != ["enclosure"]);
    assert!(codes(&p).is_empty());
}

#[test]
fn the_errors_of_every_file_come_back_together() {
    let bad = one_file(&SMALL.replace("100,100,100", "1,100,100"));
    let errors = Catalog::from_texts(&[("a.json", "x"), ("v.json", &bad)]).unwrap_err();
    let found: Vec<PrnCode> = errors.iter().map(|e| e.code).collect();
    assert_eq!(found, vec![PrnCode::Parse, PrnCode::OutOfRange]);
    assert!(errors[1].to_string().starts_with("EH3D-PRN-005: v/m: "));
}

#[test]
fn the_processes_follow_the_nozzle() {
    let rules = Processes::embedded().unwrap();
    let p = rules.for_nozzle(Level::Standard, 0.4).unwrap();
    assert_eq!(
        (p.layer_mm, p.first_layer_mm, p.line_width_mm),
        (0.2, 0.2, 0.45)
    );
    for nozzle in [0.2, 0.4, 0.6, 0.8] {
        let mut last = f64::MAX;
        for level in [Level::Draft, Level::Standard, Level::Fine] {
            let p = rules.for_nozzle(level, nozzle).unwrap();
            assert!(p.layer_mm < last && p.layer_mm > 0.0 && p.layer_mm < nozzle);
            assert!(p.first_layer_mm < nozzle && p.line_width_mm >= nozzle * 0.9);
            last = p.layer_mm;
        }
    }
    for bad in [f64::NAN, f64::INFINITY, 0.0, -0.4, 0.05, 2.5] {
        assert_eq!(
            rules.for_nozzle(Level::Fine, bad).unwrap_err().code,
            PrnCode::OutOfRange,
            "{bad}"
        );
    }
    for level in Level::ALL {
        assert_eq!(Level::from_name(level.as_str()), Some(level));
    }
    assert_eq!(Level::from_name(" Fine "), Some(Level::Fine));
    assert_eq!(Level::from_name("finest"), None);
}

#[test]
fn rules_of_processes_that_are_not_whole_or_not_believable_are_refused() {
    let text = eh3d_printers::start::PROCESS_DATA;
    for (bad, code) in [
        (
            text.replace("\"draft\"", "\"standard\""),
            PrnCode::Inconsistent,
        ),
        (
            text.replacen("\"walls\": 2", "\"walls\": 0", 1),
            PrnCode::OutOfRange,
        ),
        (
            text.replacen("\"layer_ratio\": 0.6", "\"layer_ratio\": 3", 1),
            PrnCode::OutOfRange,
        ),
        (
            text.replace("\"schema_version\": 1", "\"schema_version\": 9"),
            PrnCode::SchemaVersion,
        ),
        ("{}".to_string(), PrnCode::Parse),
    ] {
        assert_eq!(Processes::from_text(&bad).unwrap_err().code, code, "{bad}");
    }
}

#[test]
fn every_generic_filament_is_seen_against_the_printer_with_its_reasons() {
    let db = Database::embedded().unwrap();
    let c = catalog();
    let open = c.find("bambu-lab/a1").unwrap();
    let closed = c.find("bambu-lab/p1s").unwrap();
    let on_open = starting_filaments(open, &db, 0.4);
    let on_closed = starting_filaments(closed, &db, 0.4);
    let generic = db
        .filaments()
        .iter()
        .filter(|f| f.manufacturer == "Generic")
        .count();
    assert!(generic >= 10);
    assert_eq!(on_open.len(), generic);
    assert_eq!(on_closed.len(), generic);
    let find = |list: &[eh3d_printers::StartFilament<'_>], id: &str| {
        let s = list.iter().find(|s| s.filament.id == id).unwrap();
        (s.blocks.clone(), s.cautions.clone(), s.usable())
    };
    let abs = "generic/generic-abs";
    assert!(find(&on_open, abs).1.contains(&Caution::Enclosure));
    assert!(!find(&on_closed, abs).1.contains(&Caution::Enclosure));
    assert!(find(&on_open, abs).2);
    let cf = "generic/generic-pla-with-carbon-fibre";
    assert!(find(&on_open, cf).1.contains(&Caution::HardenedNozzle));
    let thin = starting_filaments(open, &db, 0.2);
    assert!(find(&thin, "generic/generic-tpu")
        .0
        .contains(&Block::NozzleTooSmall));
    assert!(find(&thin, "generic/generic-pla").2);
}

#[test]
fn a_printer_that_is_too_cold_blocks_and_never_lowers_a_temperature() {
    let db = Database::embedded().unwrap();
    let mut cold = a1();
    cold.nozzle_max_c = 205.0;
    cold.bed_max_c = 50.0;
    cold.filament_diameter_mm = Some(2.85);
    cold.enclosure = Enclosure::Unknown;
    let seen = starting_filaments(&cold, &db, 0.4);
    let petg = seen
        .iter()
        .find(|s| s.filament.id == "generic/generic-petg")
        .unwrap();
    assert_eq!(
        petg.blocks,
        vec![
            Block::NozzleTemperature,
            Block::BedTemperature,
            Block::Diameter
        ]
    );
    assert!(!petg.usable());
    let asa = seen
        .iter()
        .find(|s| s.filament.id == "generic/generic-asa")
        .unwrap();
    assert!(asa.cautions.contains(&Caution::EnclosureUnknown));
    // the record of the filament is the one of the database, with its own figures
    assert_eq!(petg.filament.nozzle_c.unwrap().recommended, Some(235.0));
}

#[test]
fn every_word_is_there_in_the_eight_languages() {
    for lang in Lang::ALL {
        for code in PrnCode::ALL {
            assert!(!code.title_in(lang).is_empty());
        }
        for v in Enclosure::ALL {
            assert!(!v.title_in(lang).is_empty());
        }
        for v in Firmware::ALL {
            assert!(!v.title_in(lang).is_empty());
        }
        for v in Adapter::ALL {
            assert!(!v.title_in(lang).is_empty());
        }
        for v in MulticolorStatus::ALL {
            assert!(!v.title_in(lang).is_empty());
        }
        for v in Block::ALL {
            assert!(!v.title_in(lang).is_empty());
        }
        for v in Caution::ALL {
            assert!(!v.title_in(lang).is_empty());
        }
        for v in Level::ALL {
            assert!(!v.title_in(lang).is_empty());
        }
        for v in Label::ALL {
            assert!(!v.title_in(lang).is_empty());
        }
    }
    // the sentences are translated, not left in English
    for lang in Lang::ALL.into_iter().filter(|l| *l != Lang::En) {
        for v in Caution::ALL {
            assert_ne!(v.title_in(lang), v.title_in(Lang::En));
        }
        for v in Block::ALL {
            assert_ne!(v.title_in(lang), v.title_in(Lang::En));
        }
        assert_ne!(
            Label::NotTried.title_in(lang),
            Label::NotTried.title_in(Lang::En)
        );
    }
}

#[test]
fn what_was_read_on_a_page_is_told_apart_from_what_is_still_to_be_checked() {
    let c = catalog();
    let a1 = c.find("bambu-lab/a1").unwrap();
    assert_eq!(a1.unconfirmed(), Vec::<&str>::new());
    assert!(!a1.verified, "read on a page is not tried on a machine");
    assert_eq!(a1.max_speed_mm_s, Some(500.0));
    assert_eq!(a1.max_acceleration_mm_s2, Some(10_000.0));
    assert_eq!(a1.max_flow_mm3_s, Some(28.0));
    assert_eq!(a1.default_height_mm, None);
    let p1s = c.find("bambu-lab/p1s").unwrap();
    assert_eq!(p1s.default_height_mm, Some(250.0));
    assert_eq!(p1s.unconfirmed(), vec!["filament_diameter_mm"]);
    // a page that no longer answers confirms nothing
    let plus = c.find("artillery/sidewinder-x4-plus-s1").unwrap();
    assert!(plus.unconfirmed().contains(&"volume_mm"));
    assert!(plus.confirmed("nozzles_mm"));
    for p in c.printers() {
        for s in &p.sources {
            assert!(s.confirmed.iter().all(|f| s.covers.contains(f)), "{}", p.id);
        }
    }
}

#[test]
fn a_confirmation_of_what_is_not_covered_and_motion_figures_out_of_range_are_refused() {
    let mut p = a1();
    p.sources[0].confirmed.push("chamber_max_c".to_string());
    assert_eq!(codes(&p), vec![PrnCode::Source]);
    for bad in [f64::NAN, -1.0, 0.0, 1.0e9] {
        let mut p = a1();
        p.max_speed_mm_s = Some(bad);
        assert!(codes(&p).contains(&PrnCode::OutOfRange), "{bad}");
        let mut p = a1();
        p.max_flow_mm3_s = Some(bad);
        assert!(codes(&p).contains(&PrnCode::OutOfRange), "{bad}");
    }
    let mut p = a1();
    p.default_height_mm = Some(300.0);
    assert!(codes(&p).contains(&PrnCode::Inconsistent));
    // a figure with no page is a claim with no source
    let mut p = a1();
    p.sources
        .retain(|s| !s.covers.iter().any(|f| f == "max_flow_mm3_s"));
    assert!(codes(&p).contains(&PrnCode::Source));
}
