//! Garbage in, no panic out: the readers of the drill file, the plot, the drawing and the Gerber outline are given thousands of texts made from a fixed seed (pieces of real
//! commands glued together at random, cut at random and with random bytes) and must always answer with a result, never panic or loop. The seed is fixed, so a failure is the same
//! on every computer and the text that caused it is printed.

use eh3d_bungard_formats::limits::{LimitVerdict, ReaderLimits};
use eh3d_bungard_formats::{dxf, gerber, hpgl, ncd};

/// A small generator with a fixed seed (a linear congruential generator): the same texts on every run.
struct Rng(u64);

impl Rng {
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

const PIECES: &[&str] = &[
    "%",
    "M48",
    "METRIC",
    "INCH",
    "T1",
    "T01C0.8",
    "X1000Y2000",
    "X-5Y+3",
    "X",
    "Y9999999999999",
    "M30",
    "M71",
    "M72",
    ";",
    "(",
    ")",
    "IN;",
    "SP1;",
    "PU0,0;",
    "PD100,100,200,50;",
    "PA",
    "CI50;",
    "AA0,0,90;",
    "AR0,0,-90;",
    "PD",
    ",",
    "-",
    ".",
    "9",
    "0\nSECTION\n",
    "2\nENTITIES\n",
    "0\nLINE\n",
    "10\n1.5\n",
    "20\n2.5\n",
    "11\n3\n",
    "21\n4\n",
    "0\nENDSEC\n",
    "0\nEOF\n",
    "%FSLAX24Y24*%",
    "%MOMM*%",
    "G01*",
    "G02*",
    "G03*",
    "G75*",
    "D10*",
    "D01*",
    "D02*",
    "D03*",
    "X10000Y20000D01*",
    "M02*",
    "\n",
    "\r\n",
    " ",
    "\t",
    "\u{0}",
    "é",
    "日本",
];

fn text(rng: &mut Rng) -> String {
    let n = 1 + rng.below(40);
    let mut s = String::new();
    for _ in 0..n {
        s.push_str(PIECES[rng.below(PIECES.len())]);
        if rng.below(4) == 0 {
            s.push('\n');
        }
    }
    // cut it at a random place, as a damaged file would be
    if rng.below(3) == 0 && !s.is_empty() {
        let mut cut = rng.below(s.len());
        while !s.is_char_boundary(cut) {
            cut -= 1;
        }
        s.truncate(cut);
    }
    s
}

#[test]
fn no_reader_panics_on_ten_thousand_made_up_texts() {
    let mut rng = Rng(0x5EED_CAFE);
    for _ in 0..10_000 {
        let t = text(&mut rng);
        let _ = ncd::parse(&t);
        let _ = hpgl::parse_with(&t, 1 + rng.below(90) as u32);
        let _ = hpgl::diagnose(&t);
        let _ = hpgl::parse_strict(&t, 36);
        let _ = gerber::outline(&t, 36);
        let _ = gerber::outline_checked(&t, 36);
        let _ = dxf::outline(&t, 36);
        let _ = dxf::outline_checked(&t, 36);
        let _ = gerber::looks_like_outline("x.gbr", &t);
    }
}

#[test]
fn the_limited_readers_answer_the_same_for_the_same_text_and_never_panic() {
    let mut rng = Rng(0xBAD_5EED);
    let limits = ReaderLimits::from_config(
        "max_bytes = 120\nmax_lines = 8\nwarn_bytes = 60\nwarn_lines = 4\n",
    )
    .expect("a valid file of limits");
    let mut refused = 0;
    let mut warned = 0;
    for _ in 0..5_000 {
        let t = text(&mut rng);
        let a = ncd::parse_limited(&t, &limits).map(|(_, v)| v);
        let b = ncd::parse_limited(&t, &limits).map(|(_, v)| v);
        assert_eq!(a.is_ok(), b.is_ok());
        // a text past a hard limit is refused with the code of the limits, whatever else it holds
        let over = t.len() > 120 || t.bytes().filter(|&c| c == b'\n').count() + 1 > 8;
        match &a {
            Err(e) if over => {
                refused += 1;
                assert_eq!(e.code.id(), "EH3D-FORMATS-011", "{t:?}");
            }
            Ok(LimitVerdict::Warn(_)) => warned += 1,
            _ => {}
        }
        assert_eq!(hpgl::parse_limited(&t, 36, &limits).is_err(), over, "{t:?}");
    }
    assert!(
        refused > 10 && warned > 10,
        "{refused} {warned}: the made up texts must reach both"
    );
}
