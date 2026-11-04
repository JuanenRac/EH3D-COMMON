//! The numbers of the errors and the alarms of GRBL 1.1 with the text its documentation gives them, so that a refusal of the controller can be shown with its words and not
//! only with its number.

/// The text of `error:N`, from the table of GRBL 1.1; `None` for a number it does not use.
pub fn grbl_error_text(code: u32) -> Option<&'static str> {
    Some(match code {
        1 => "G-code words consist of a letter and a value. Letter was not found.",
        2 => "Numeric value format is not valid or missing an expected value.",
        3 => "Grbl '$' system command was not recognized or supported.",
        4 => "Negative value received for an expected positive value.",
        5 => "Homing cycle is not enabled via settings.",
        6 => "Minimum step pulse time must be greater than 3 microseconds.",
        7 => "EEPROM read failed. Reset and restored to default values.",
        8 => "Grbl '$' command cannot be used unless Grbl is IDLE.",
        9 => "G-code locked out during alarm or jog state.",
        10 => "Soft limits cannot be enabled without homing also enabled.",
        11 => "Max characters per line exceeded. Line was not processed and executed.",
        12 => "Grbl '$' setting value causes the step rate to exceed the maximum supported.",
        13 => "Safety door detected as opened and door state initiated.",
        14 => "Build info or startup line exceeded EEPROM line length limit.",
        15 => "Jog target exceeds machine travel. Command ignored.",
        16 => "Jog command with no '=' or contains prohibited g-code.",
        17 => "Laser mode requires PWM output.",
        20 => "Unsupported or invalid g-code command found in block.",
        21 => "More than one g-code command from same modal group found in block.",
        22 => "Feed rate has not yet been set or is undefined.",
        23 => "G-code command in block requires an integer value.",
        24 => "Two G-code commands that both require the use of the XYZ axis words were detected in the block.",
        25 => "A G-code word was repeated in the block.",
        26 => "A G-code command implicitly or explicitly requires XYZ axis words in the block, but none were detected.",
        27 => "N line number value is not within the valid range of 1 - 9,999,999.",
        28 => "A G-code command was sent, but is missing some required P or L value words in the line.",
        29 => "Grbl supports six work coordinate systems G54-G59. G59.1, G59.2, and G59.3 are not supported.",
        30 => "The G53 G-code command requires either a G0 seek or G1 feed motion mode to be active.",
        31 => "There are unused axis words in the block and G80 motion mode cancel is active.",
        32 => "A G2 or G3 arc was commanded but there are no XYZ axis words in the selected plane to trace the arc.",
        33 => "The motion command has an invalid target.",
        34 => "A G2 or G3 arc, traced with the radius definition, had a mathematical error when computing the arc geometry.",
        35 => "A G2 or G3 arc, traced with the offset definition, is missing the IJK offset word in the selected plane to trace the arc.",
        36 => "There are unused, leftover G-code words that aren't used by any command in the block.",
        37 => "The G43.1 dynamic tool length offset command cannot apply an offset to an axis other than its configured axis.",
        38 => "Tool number greater than max supported value.",
        _ => return None,
    })
}

/// The text of `ALARM:N`, from the table of GRBL 1.1; `None` for a number it does not use.
pub fn grbl_alarm_text(code: u32) -> Option<&'static str> {
    Some(match code {
        1 => "Hard limit triggered. Machine position is likely lost due to sudden and immediate halt. Re-homing is highly recommended.",
        2 => "G-code motion target exceeds machine travel. Machine position safely retained. Alarm may be unlocked.",
        3 => "Reset while in motion. Grbl cannot guarantee position. Lost steps are likely. Re-homing is highly recommended.",
        4 => "Probe fail. The probe is not in the expected initial state before starting probe cycle.",
        5 => "Probe fail. Probe did not contact the workpiece within the programmed travel for G38.2 and G38.4.",
        6 => "Homing fail. Reset during active homing cycle.",
        7 => "Homing fail. Safety door was opened during active homing cycle.",
        8 => "Homing fail. Cycle failed to clear limit switch when pulling off. Try increasing pull-off setting or check wiring.",
        9 => "Homing fail. Could not find limit switch within search distance.",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_numbers_of_the_table_have_their_text_and_the_others_do_not() {
        for code in (1..=17).chain(20..=38) {
            assert!(grbl_error_text(code).is_some(), "error {code}");
        }
        for code in [0, 18, 19, 39, 1000] {
            assert!(grbl_error_text(code).is_none(), "error {code}");
        }
        for code in 1..=9 {
            assert!(grbl_alarm_text(code).is_some(), "alarm {code}");
        }
        assert!(grbl_alarm_text(0).is_none() && grbl_alarm_text(10).is_none());
        assert!(grbl_error_text(20).unwrap().contains("Unsupported"));
    }

    #[test]
    fn a_refusal_and_an_alarm_give_their_number_and_their_official_text() {
        use crate::Error;
        let refused = Error::Refused("error 20".into());
        assert_eq!(refused.code(), Some(20));
        assert!(refused.official_text().unwrap().contains("Unsupported"));
        let alarm = Error::Alarm("alarm 3".into());
        assert_eq!(alarm.code(), Some(3));
        assert!(alarm
            .official_text()
            .unwrap()
            .contains("Reset while in motion"));
        // the same number is not the same text for an error and for an alarm
        assert_ne!(
            Error::Refused("error 3".into()).official_text(),
            Error::Alarm("alarm 3".into()).official_text()
        );
        assert_eq!(Error::Refused("error 99".into()).official_text(), None);
        assert_eq!(Error::Refused("no number".into()).code(), None);
        assert_eq!(Error::Timeout.code(), None);
        assert_eq!(Error::Link("x1".into()).official_text(), None);
    }
}
