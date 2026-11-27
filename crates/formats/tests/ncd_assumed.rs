//! A drill file that does not say how its numbers are written is read as before, and the supposition is told.

use eh3d_bungard_formats::ncd::{
    diagnose, how_to_choose, parse, parse_as, parse_declared, NumberFormat, MILS_DECLARATION,
};

fn file(header: &str, line: &str) -> String {
    format!("M48\n{header}\nT1C0.8\n%\nT1\n{line}\nM30\n")
}

#[test]
fn an_inch_header_with_nothing_else_keeps_its_mils_and_says_that_it_was_supposed() {
    for header in ["INCH", "M72", ";FILE_FORMAT=2:4\nINCH"] {
        let text = file(header, "X1000Y2000");
        let holes = parse(&text).unwrap().holes;
        assert_eq!((holes[0].x_mil, holes[0].y_mil), (1000, 2000), "{header}");
        let found = diagnose(&text);
        assert_eq!(found.len(), 1, "{header}");
        assert_eq!(found[0].code.id(), "EH3D-FORMATS-014", "{header}");
        assert_eq!(found[0].detail, "X1000Y2000", "{header}");
        assert_eq!(parse_declared(&text).unwrap_err(), found[0], "{header}");
    }
    // the line is that of the first coordinate, and it is told once
    let two = "M48\nINCH\n%\nT1\nX1Y1\nX2Y2\n";
    assert_eq!(
        diagnose(two).iter().map(|e| e.line).collect::<Vec<_>>(),
        vec![Some(5)]
    );
}

#[test]
fn a_format_that_has_to_be_supposed_is_told_whatever_is_missing() {
    for text in [
        // no unit at all, as the files of the machine
        "%\nT06\nX1000Y2000\n".to_string(),
        file(";FILE_FORMAT=2:4", "X1000Y2000"),
        // a unit with no digits
        file("INCH,TZ", "X5850Y10000"),
        file("METRIC", "X12345Y20000"),
        // digits with no word about the zeros, and a number that does not fill them
        file("METRIC\n;FILE_FORMAT=3:3", "X1000Y2000"),
        file("INCH\n;FILE_FORMAT=2:4", "X5850Y10000"),
    ] {
        assert_eq!(diagnose(&text).len(), 1, "{text:?}");
        assert!(parse_declared(&text).is_err(), "{text:?}");
    }
}

#[test]
fn a_file_that_says_its_format_has_nothing_to_tell() {
    for text in [
        file("METRIC", "X12.7Y-25.4"),
        file("INCH", "X1000Y1000").replace("X1000Y1000\n", ""),
        file("METRIC,TZ,000.000", "X12700Y0"),
        file("INCH,LZ\n;FILE_FORMAT=2:4", "X05Y1"),
        // every number fills the digits, so the zeros do not matter
        file("METRIC\n;FILE_FORMAT=3:3", "X012700Y-001000"),
    ] {
        assert!(diagnose(&text).is_empty(), "{text:?}");
        assert_eq!(parse_declared(&text).unwrap(), parse(&text).unwrap());
    }
}

#[test]
fn a_file_that_cannot_be_read_gives_its_error_to_the_check_too() {
    let text = "%\nX1000Y1000\n";
    assert_eq!(diagnose(text)[0].code.id(), "EH3D-FORMATS-002");
    assert_eq!(
        parse_declared(text).unwrap_err().code.id(),
        "EH3D-FORMATS-002"
    );
}

fn first_hole(text: &str, chosen: &str) -> (i32, i32) {
    let f = parse_as(text, NumberFormat::from_name(chosen)).unwrap();
    (f.holes[0].x_mil, f.holes[0].y_mil)
}

#[test]
fn an_inch_file_with_no_declared_format_is_refused_until_one_is_chosen() {
    let text = file("INCH", "X1000Y2000");
    let refused = parse_as(&text, None).unwrap_err();
    assert_eq!(refused.code.id(), "EH3D-FORMATS-014");
    // the two readings that are ten times apart, each only when it is asked for
    assert_eq!(first_hole(&text, "mils"), (1000, 2000));
    assert_eq!(first_hole(&text, "inch-2.4-lz"), (10_000, 20_000));
    assert_eq!(first_hole(&text, "inch-2.4-tz"), (100, 200));
    // the files of the machine have no header at all
    let bare = "%\nT06\nX1000Y2000\n";
    assert!(parse_as(bare, None).is_err());
    assert_eq!(first_hole(bare, "mils"), (1000, 2000));
    assert_eq!(first_hole(bare, "metric-3.3-tz"), (39, 79));
    assert!(
        parse_as(bare, NumberFormat::from_name("metric-3.3-tz"))
            .unwrap()
            .metric
    );
    // numbers with a point need the unit only
    let decimal = "%\nT06\nX1.0Y2.0\n";
    assert_eq!(first_hole(decimal, "inch-2.4-tz"), (1000, 2000));
}

#[test]
fn what_a_file_declares_wins_and_a_choice_that_cannot_be_it_is_refused() {
    let metric = file("METRIC,TZ,000.000", "X12700Y0");
    assert_eq!(parse_as(&metric, None).unwrap(), parse(&metric).unwrap());
    assert_eq!(
        first_hole(&metric, "inch-2.4-lz"),
        first_hole(&metric, "mils")
    );
    for (header, chosen) in [
        ("METRIC", "inch-2.4-tz"),
        ("METRIC", "mils"),
        ("INCH", "metric-3.3-tz"),
        ("INCH,LZ", "mils"),
        ("INCH\n;FILE_FORMAT=2:4", "mils"),
    ] {
        let refused =
            parse_as(&file(header, "X1000Y2000"), NumberFormat::from_name(chosen)).unwrap_err();
        assert_eq!(refused.code.id(), "EH3D-FORMATS-014", "{header} {chosen}");
    }
    // the zeros the header says win over those of the choice
    assert_eq!(
        first_hole(&file("INCH,LZ", "X05Y1"), "inch-2.4-tz"),
        (5000, 10_000)
    );
}

#[test]
fn the_header_the_suite_writes_declares_whole_mils_to_every_reading() {
    let text = format!("M48\n{MILS_DECLARATION}T1C0.0315\n%\nT1\nX1000Y-5\nX1234567Y0\nM30\n");
    let f = parse_as(&text, None).unwrap();
    assert_eq!(f, parse(&text).unwrap());
    assert!(diagnose(&text).is_empty());
    assert_eq!((f.holes[0].x_mil, f.holes[0].y_mil), (1000, -5));
    assert_eq!(f.holes[1].x_mil, 1_234_567);
    assert!(f.units_declared && !f.metric);
    // and so does a `FILE_FORMAT` comment written with a blank
    let two_four = "M48\nINCH,TZ\n; FILE_FORMAT=2:4\n%\nT1\nX012340Y1\n";
    let f = parse_as(two_four, None).unwrap();
    assert_eq!((f.holes[0].x_mil, f.holes[0].y_mil), (1234, 0));
    // a zero means the same under every format
    assert!(parse_as("M48\nMETRIC\n%\nT1\nX0Y0\n", None).is_ok());
}

#[test]
fn the_names_of_the_formats_go_and_come_back_and_the_help_is_in_eight_languages() {
    for f in NumberFormat::PRESETS {
        assert_eq!(NumberFormat::from_name(&f.name()), Some(f));
        assert_eq!(
            NumberFormat::from_setting(&NumberFormat::setting(Some(f))),
            Ok(Some(f))
        );
    }
    assert_eq!(
        NumberFormat::from_setting(&NumberFormat::setting(None)),
        Ok(None)
    );
    assert_eq!(NumberFormat::from_setting("  "), Ok(None));
    for wrong in [
        "2.4",
        "inch-2.4",
        "inch-0.4-tz",
        "inch-2.7-tz",
        "mm-3.3-tz",
        "inch-2.4-tz-x",
    ] {
        assert!(NumberFormat::from_setting(wrong).is_err(), "{wrong}");
    }
    let languages = ["en", "es", "de", "fr", "it", "ja", "zh", "pt"];
    for (i, a) in languages.iter().enumerate() {
        for f in NumberFormat::PRESETS {
            assert!(how_to_choose(a).contains(&f.name()), "{a}");
        }
        for b in &languages[i + 1..] {
            assert_ne!(how_to_choose(a), how_to_choose(b));
        }
    }
}
