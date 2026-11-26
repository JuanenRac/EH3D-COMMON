//! Data files cut or changed, and the processes of every printer. Nothing here talks to a printer.

use eh3d_printers::catalog::{parse_printer_file, Catalog, PRINTER_FILES};
use eh3d_printers::start::{Level, Processes};

#[test]
fn a_data_file_cut_or_changed_is_an_error_and_never_a_panic() {
    let mut state = 0x5EED_0004u64;
    let mut below = |n: usize| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % n as u64) as usize
    };
    for (name, text) in PRINTER_FILES {
        let end = text.trim_end().len();
        for (cut, _) in text[..end].char_indices() {
            assert!(
                parse_printer_file(name, &text[..cut]).is_err(),
                "{name} {cut}"
            );
        }
        for _ in 0..500 {
            let mut chars: Vec<char> = text.chars().collect();
            for _ in 0..1 + below(4) {
                let at = below(chars.len());
                chars[at] = ['0', '9', '-', '"', '{', '}', ',', 'e', '#', '\u{2028}'][below(10)];
            }
            let changed: String = chars.into_iter().collect();
            let _ = Catalog::from_texts(&[(name, changed.as_str())]);
        }
    }
}

#[test]
fn every_nozzle_of_every_printer_gives_processes_that_make_sense() {
    let catalog = Catalog::embedded().unwrap();
    let processes = Processes::embedded().unwrap();
    for printer in catalog.printers() {
        for &nozzle in &printer.nozzles_mm {
            for level in Level::ALL {
                let p = processes.for_nozzle(level, nozzle).unwrap();
                assert!(p.layer_mm > 0.0 && p.layer_mm < nozzle, "{}", printer.id);
                assert!(p.first_layer_mm > 0.0 && p.first_layer_mm < nozzle);
                assert!(p.line_width_mm > 0.0 && p.walls > 0);
            }
        }
    }
    for nozzle in [f64::NAN, f64::INFINITY, -0.4, 0.0, 50.0] {
        assert!(processes.for_nozzle(Level::Standard, nozzle).is_err());
    }
}
