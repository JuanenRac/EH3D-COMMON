//! A cable to a simulated controller that misbehaves on purpose, so that the dialects of the controllers are tried on what a real serial line
//! does and not only on clean lines: an answer that arrives in pieces, noise between the lines, a flipped bit in what the program sends, a
//! controller that falls silent for a while. A [`Device`] is the controller (it takes bytes and gives bytes); a [`WireLink`] puts a device
//! behind the line transport that the controllers of the suite read, and spoils the bytes by a schedule that a fixed seed makes the same on
//! every computer. No time passes in it: a read that finds nothing says so at once. Nothing here is a machine.

use std::collections::VecDeque;
use std::io;
use std::time::Duration;

use crate::grbl::LineTransport;
use crate::serial::LineBuffer;

/// The generator of the schedule (SplitMix64): the same seed gives the same cable on every computer.
#[derive(Debug, Clone)]
pub struct Seed(u64);

impl Seed {
    pub fn new(seed: u64) -> Seed {
        Seed(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number from 0 to `n` (not included); 0 when `n` is 0.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }

    /// True `per_mille` times in a thousand.
    pub fn chance(&mut self, per_mille: u32) -> bool {
        per_mille > 0 && (self.next_u64() % 1000) < u64::from(per_mille)
    }
}

/// What the cable does to the bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Impair {
    /// The seed of the schedule.
    pub seed: u64,
    /// The most bytes of an answer that come in one piece (an answer is cut at random into pieces of 1 to this many; 0 is no cutting).
    pub chunk_max: usize,
    /// Per thousand pieces, the chance that a line of noise comes in front of the piece (made of bytes that are not a line end).
    pub noise_per_mille: u32,
    /// Per thousand lines the program writes, the chance that one bit of one byte of the line is flipped on its way to the device.
    pub flip_per_mille: u32,
    /// The device is deaf to the answers (it hears the lines, and its answers are lost) from the line `mute_from` (from 1) for `mute_lines`.
    pub mute_from: Option<usize>,
    pub mute_lines: usize,
}

impl Default for Impair {
    fn default() -> Self {
        Impair::clean()
    }
}

impl Impair {
    /// A good cable: nothing is cut, nothing is added, nothing is lost.
    pub fn clean() -> Impair {
        Impair {
            seed: 1,
            chunk_max: 0,
            noise_per_mille: 0,
            flip_per_mille: 0,
            mute_from: None,
            mute_lines: 0,
        }
    }
}

/// A controller behind a cable: it takes the bytes the program writes and gives the bytes it answers.
pub trait Device {
    /// The answer to the bytes (it may be nothing, or more than one line).
    fn input(&mut self, bytes: &[u8]) -> Vec<u8>;
}

/// The transport of the controllers over a [`Device`].
#[derive(Debug)]
pub struct WireLink<D: Device> {
    device: D,
    impair: Impair,
    seed: Seed,
    pending: VecDeque<u8>,
    lines: LineBuffer,
    /// The last piece that came in ended a line (noise is put in only there: it is between the lines, never inside one).
    at_boundary: bool,
    /// How many lines the program has written (a line ends with a line feed).
    pub written_lines: usize,
    /// Everything the program wrote, as the device heard it (after the flipped bits).
    pub heard: Vec<u8>,
    /// How many pieces of noise were put in, and how many bits were flipped.
    pub noise_lines: usize,
    pub flips: usize,
}

impl<D: Device> WireLink<D> {
    pub fn new(device: D, impair: Impair) -> WireLink<D> {
        WireLink {
            device,
            seed: Seed::new(impair.seed),
            impair,
            pending: VecDeque::new(),
            lines: LineBuffer::default(),
            at_boundary: true,
            written_lines: 0,
            heard: Vec::new(),
            noise_lines: 0,
            flips: 0,
        }
    }

    pub fn device(&self) -> &D {
        &self.device
    }

    pub fn device_mut(&mut self) -> &mut D {
        &mut self.device
    }

    /// Bytes the device sends without being asked (its greeting after a restart, a message of its own).
    pub fn push_unsolicited(&mut self, bytes: &[u8]) {
        self.pending.extend(bytes.iter().copied());
    }

    /// What the device heard, as text (the control bytes show as their characters).
    pub fn heard_text(&self) -> String {
        self.heard.iter().map(|&b| b as char).collect()
    }

    fn muted(&self) -> bool {
        match self.impair.mute_from {
            Some(from) => {
                self.written_lines >= from
                    && self.written_lines < from.saturating_add(self.impair.mute_lines)
            }
            None => false,
        }
    }

    /// A piece of noise: a few bytes that are not a line end, so that the line they make is not an answer of anybody.
    fn noise(&mut self) -> Vec<u8> {
        let n = 1 + self.seed.below(6);
        let mut v: Vec<u8> = (0..n)
            .map(|_| {
                let b = (self.seed.next_u64() & 0xFF) as u8;
                if b == b'\n' || b == b'\r' {
                    b'~'
                } else {
                    b
                }
            })
            .collect();
        v.push(b'\n');
        v
    }
}

impl<D: Device> LineTransport for WireLink<D> {
    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
        let mut sent = bytes.to_vec();
        let ends = sent.iter().filter(|&&b| b == b'\n').count();
        if ends > 0 && self.seed.chance(self.impair.flip_per_mille) {
            // one bit of one byte of the line (not its end) goes wrong on the way
            let at = self.seed.below(sent.len());
            if sent[at] != b'\n' {
                sent[at] ^= 1 << self.seed.below(7);
                self.flips += 1;
            }
        }
        self.written_lines += ends;
        self.heard.extend_from_slice(&sent);
        let answer = self.device.input(&sent);
        // the lines written this time count first: a mute that begins at line N loses the answer to line N
        if !self.muted() {
            self.pending.extend(answer);
        }
        Ok(())
    }

    fn read_line(&mut self, _timeout: Duration) -> io::Result<Option<String>> {
        loop {
            if let Some(line) = self.lines.pop() {
                return Ok(Some(line));
            }
            if self.pending.is_empty() {
                return Ok(None);
            }
            if self.at_boundary && self.seed.chance(self.impair.noise_per_mille) {
                let n = self.noise();
                self.noise_lines += 1;
                self.lines.push(&n);
            }
            let take = if self.impair.chunk_max == 0 {
                self.pending.len()
            } else {
                (1 + self.seed.below(self.impair.chunk_max)).min(self.pending.len())
            };
            let piece: Vec<u8> = self.pending.drain(..take).collect();
            self.at_boundary = piece.last() == Some(&b'\n');
            self.lines.push(&piece);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Answers every line with `ok` and says what it was given when asked with a question mark.
    struct Echo;

    impl Device for Echo {
        fn input(&mut self, bytes: &[u8]) -> Vec<u8> {
            let mut out = Vec::new();
            for line in bytes.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
                out.extend_from_slice(b"ok:");
                out.extend_from_slice(line);
                out.push(b'\n');
            }
            out
        }
    }

    fn drain(link: &mut WireLink<Echo>) -> Vec<String> {
        let mut all = Vec::new();
        while let Some(l) = link.read_line(Duration::ZERO).unwrap() {
            all.push(l);
        }
        all
    }

    #[test]
    fn the_same_seed_gives_the_same_numbers_and_a_different_one_does_not() {
        let a: Vec<u64> = {
            let mut s = Seed::new(7);
            (0..5).map(|_| s.next_u64()).collect()
        };
        let b: Vec<u64> = {
            let mut s = Seed::new(7);
            (0..5).map(|_| s.next_u64()).collect()
        };
        let c: Vec<u64> = {
            let mut s = Seed::new(8);
            (0..5).map(|_| s.next_u64()).collect()
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(Seed::new(1).below(0), 0);
        assert!(!Seed::new(1).chance(0));
        assert!(Seed::new(1).chance(1000));
    }

    #[test]
    fn an_answer_cut_into_pieces_comes_back_whole() {
        for chunk_max in [0, 1, 2, 3, 7, 64] {
            let mut link = WireLink::new(
                Echo,
                Impair {
                    chunk_max,
                    seed: 11,
                    ..Impair::clean()
                },
            );
            link.write_all(b"G0 X1\nG1 Y2 F100\n").unwrap();
            assert_eq!(
                drain(&mut link),
                vec!["ok:G0 X1", "ok:G1 Y2 F100"],
                "chunk_max {chunk_max}"
            );
        }
    }

    #[test]
    fn noise_comes_as_lines_of_its_own_and_never_inside_an_answer() {
        let mut link = WireLink::new(
            Echo,
            Impair {
                seed: 5,
                chunk_max: 3,
                noise_per_mille: 700,
                ..Impair::clean()
            },
        );
        link.write_all(b"G0 X1\n").unwrap();
        let got = drain(&mut link);
        assert!(got.iter().any(|l| l == "ok:G0 X1" || l.ends_with("G0 X1")));
        assert!(link.noise_lines > 0, "the noise was never put in");
    }

    #[test]
    fn a_mute_loses_the_answers_of_the_lines_it_covers_and_then_ends() {
        let mut link = WireLink::new(
            Echo,
            Impair {
                mute_from: Some(2),
                mute_lines: 2,
                ..Impair::clean()
            },
        );
        let mut heard = Vec::new();
        for l in ["a\n", "b\n", "c\n", "d\n"] {
            link.write_all(l.as_bytes()).unwrap();
            heard.extend(drain(&mut link));
        }
        assert_eq!(heard, vec!["ok:a", "ok:d"]);
        // the device did hear every line
        assert_eq!(link.heard_text(), "a\nb\nc\nd\n");
        assert_eq!(link.written_lines, 4);
    }

    #[test]
    fn a_flipped_bit_changes_one_byte_of_one_line_and_the_schedule_repeats_with_the_seed() {
        let run = |seed: u64| {
            let mut link = WireLink::new(
                Echo,
                Impair {
                    seed,
                    flip_per_mille: 1000,
                    ..Impair::clean()
                },
            );
            link.write_all(b"G1 X10 F100\n").unwrap();
            link.heard_text()
        };
        let a = run(3);
        assert_eq!(a, run(3));
        assert_ne!(a, "G1 X10 F100\n");
        assert_eq!(a.len(), "G1 X10 F100\n".len());
        let diff = a
            .bytes()
            .zip("G1 X10 F100\n".bytes())
            .filter(|(x, y)| x != y)
            .count();
        assert_eq!(diff, 1);
    }
}
