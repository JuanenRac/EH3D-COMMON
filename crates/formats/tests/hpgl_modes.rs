//! The modes of a plot that change where the next commands go: relative coordinates, the scaling and the commands the reader does not follow.

use eh3d_bungard_formats::hpgl::{diagnose, parse, parse_strict};

/// The codes of what the check of a plot finds.
fn codes(text: &str) -> Vec<String> {
    diagnose(text).iter().map(|e| e.code.id()).collect()
}

#[test]
fn the_relative_mode_holds_for_the_pen_commands_until_an_absolute_one() {
    let p = parse("IN;SP1;PU100,100;PR;PD50,0;PD0,50;PU10,10;PD5,0;PA;PD0,0;");
    assert_eq!(p.paths.len(), 2);
    assert_eq!(p.paths[0].points, vec![(100, 100), (150, 100), (150, 150)]);
    assert_eq!(p.paths[1].points, vec![(160, 160), (165, 160), (0, 0)]);
    // `PA` with its own coordinates is absolute again, and so is what follows it
    let back = parse("PU100,100;PD;PR10,0;PA20,20;PD30,30;");
    assert_eq!(
        back.paths[0].points,
        vec![(100, 100), (110, 100), (20, 20), (30, 30)]
    );
}

#[test]
fn a_reset_of_the_plotter_puts_the_coordinates_back_to_absolute() {
    for reset in ["IN", "DF"] {
        let p = parse(&format!("PU100,100;PR1,1;{reset};PU;PD5,5;"));
        assert_eq!(
            p.paths.last().unwrap().points,
            vec![(101, 101), (5, 5)],
            "{reset}"
        );
    }
}

#[test]
fn a_command_that_changes_the_geometry_and_is_not_followed_is_told() {
    for command in [
        "RO90",
        "IW0,0,10,10",
        "IR0,0,50,50",
        "LBtext",
        "CP1,1",
        "EA10,10",
        "ER10,10",
        "RA10,10",
        "RR10,10",
        "EW10,0,90",
        "WG10,0,90",
        "PM0",
        "EP",
        "FP",
        "AT1,1,2,2",
        "RT1,1,2,2",
        "CT1",
        "BZ1,1,2,2,3,3",
        "BR1,1,2,2,3,3",
        "PE<=",
        "UC1,1",
        "SM*",
        "RP",
    ] {
        let text = format!("IN;SP1;PU0,0;{command};PD10,10;");
        assert_eq!(codes(&text), ["EH3D-FORMATS-012"], "{command}");
        assert!(parse_strict(&text, 72).is_err(), "{command}");
        assert_eq!(diagnose(&text)[0].detail, command, "{command}");
    }
    // the forms that put a thing back to its usual value, and what does not move the tool, say nothing
    for command in ["RO", "RO0", "IW", "IR", "CT", "CT0", "SM", "LT2", "VS10"] {
        let text = format!("IN;SP1;PU0,0;{command};PD10,10;");
        assert!(codes(&text).is_empty(), "{command}");
    }
}

#[test]
fn a_scaling_that_cannot_be_applied_as_written_is_told() {
    for text in [
        "SC0,100,0;",
        "SC0,100;",
        "IP0,0,4000;",
        "IP100,100;",
        "IP0,0,4000,4000;SC0,0,0,100;",
        "IP0,0,4000,4000;SC0,100,0,100,1;",
        // user units with no plotter points to lay them on: the numbers are taken as plotter units
        "SC0,100,0,100;PU0,0;PD50,50;PD60,60;",
    ] {
        assert_eq!(codes(text), ["EH3D-FORMATS-013"], "{text}");
    }
    for text in [
        "IP0,0,4000,4000;SC0,100,0,100;PU0,0;PD50,50;",
        "SC0,100,0,100;IP0,0,4000,4000;PU0,0;PD50,50;",
        "IP0,0,4000,4000;SC0,100,0,100,0;PD1,1;",
        "SC;IP;PD1,1;",
        "SC0,100,0,100;SC;PD1,1;",
        "SC0,100,0,100;IN;PD1,1;",
    ] {
        assert!(codes(text).is_empty(), "{text}");
    }
}
