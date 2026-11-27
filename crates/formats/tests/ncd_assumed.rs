//! A drill file that does not say how its numbers are written is read as before, and the supposition is told.

use eh3d_bungard_formats::ncd::{diagnose, parse, parse_declared};

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
