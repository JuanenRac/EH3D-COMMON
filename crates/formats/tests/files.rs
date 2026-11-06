//! The sample files of the machine programs (in `tests/data`, written for the tests, not copies of anybody's files): they are read, and the configuration and the tool tables are
//! written back keeping what the programs do not edit.

use std::path::Path;

use eh3d_bungard_formats::{config, drill_tools, hpgl, ncd, route_tools};

fn read(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join(name);
    config::bytes_to_text(&std::fs::read(path).unwrap())
}

#[test]
fn an_unchanged_configuration_is_rewritten_without_a_difference_in_its_values() {
    for (name, route) in [("DRILLPRO.DCF", false), ("ROUTEPRO.RCF", true)] {
        let text = read(name);
        let cfg = config::parse(&text).unwrap();
        let settings = if route {
            cfg.route_settings().unwrap()
        } else {
            cfg.settings().unwrap()
        };
        let out = config::rewrite(
            &text,
            &settings,
            &cfg.directory,
            &cfg.default_extension,
            route,
        );
        let back = config::parse(&out).unwrap();
        let back = if route {
            back.route_settings().unwrap()
        } else {
            back.settings().unwrap()
        };
        assert_eq!(back, settings, "{name}");
        // the comments are untouched: the same number of lines
        assert_eq!(out.lines().count(), text.lines().count(), "{name}");
    }
}

#[test]
fn edited_values_come_back_through_the_file() {
    let text = read("DRILLPRO.DCF");
    let cfg = config::parse(&text).unwrap();
    let mut s = cfg.settings().unwrap();
    s.scale = (1.5, 0.75);
    s.offset_mm = (3.25, 4.0);
    s.speed_xy = (3500, 900, 800);
    s.correction = (0.9991, 1.0002, 0.9);
    s.sorting = config::SortMode::XFirst;
    s.unit = config::CoordUnit::PlotterStep;
    let out = config::rewrite(&text, &s, r".\NEWDIR", "*.DRL", false);
    let back = config::parse(&out).unwrap();
    assert_eq!(back.directory, r".\NEWDIR");
    assert_eq!(back.default_extension, "*.DRL");
    assert_eq!(back.settings().unwrap(), s);
}

#[test]
fn the_tool_tables_keep_their_comment_lines_when_they_are_written_back() {
    let text = read("DRILLPRO.DTD");
    let table = drill_tools::parse(&text);
    assert!(!table.is_empty());
    let out = drill_tools::rewrite(&text, &table);
    assert_eq!(drill_tools::parse(&out), table);
    assert_eq!(
        out.lines().take(19).collect::<Vec<_>>(),
        text.lines().take(19).collect::<Vec<_>>()
    );
    let text = read("ROUTEPRO.RTD");
    let table = route_tools::parse(&text);
    assert!(!table.is_empty());
    assert_eq!(
        route_tools::parse(&route_tools::rewrite(&text, &table)),
        table
    );
}

#[test]
fn the_sample_drill_files_and_plots_are_read() {
    for name in ["100THOU.NCD", "14TOOLS.NCD"] {
        let file = ncd::parse(&read(name)).unwrap();
        assert!(!file.holes.is_empty(), "{name}");
        assert!(!file.tools.is_empty(), "{name}");
    }
    for name in ["FRAME.PLT", "FDEMO1.PLT"] {
        let plot = hpgl::parse(&read(name));
        assert!(!plot.paths.is_empty(), "{name}");
        assert!(plot.bounds_mm().is_some(), "{name}");
    }
}

#[test]
fn the_sample_plots_have_nothing_the_reader_drops() {
    for name in ["FRAME.PLT", "FDEMO1.PLT"] {
        let text = read(name);
        assert!(hpgl::diagnose(&text).is_empty(), "{name}");
        assert_eq!(
            hpgl::parse_strict(&text, hpgl::SEGMENTS_PER_CIRCLE).unwrap(),
            hpgl::parse(&text),
            "{name}"
        );
    }
}
