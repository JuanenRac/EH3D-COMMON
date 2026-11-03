//! Object identities: UUID version 7 (RFC 9562) in canonical lowercase text, shared by the
//! formats of the suite so that every program names its objects the same way.
//!
//! An id is created when a person or an import creates an object, by an [`IdGenerator`]. The
//! generator reads a [`Clock`] and a [`RandomSource`] that are injected, so tests give it a fixed
//! clock and a seeded source and get the same ids every time. Writers take [`ObjectId`] values
//! and never own a generator, so writing a file never reads the clock.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use eh3d_diagnostic::{Category, Diagnostic, Recoverability, Severity};

/// Failures of the object ids.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IdError {
    /// The text is not a canonical lowercase UUID version 7.
    #[error("not a canonical lowercase UUID v7: {reason}")]
    InvalidText {
        /// What is wrong with the text.
        reason: &'static str,
    },
    /// The clock gave a time that does not fit the 48-bit field of a UUID v7.
    #[error("the clock value {0} ms does not fit in 48 bits")]
    ClockOutOfRange(u64),
}

impl IdError {
    /// The stable code of this error: `EH3D-ID-001` or `EH3D-ID-002`.
    pub fn code(&self) -> &'static str {
        match self {
            IdError::InvalidText { .. } => "EH3D-ID-001",
            IdError::ClockOutOfRange(_) => "EH3D-ID-002",
        }
    }

    /// The common diagnostic of this error.
    pub fn diagnostic(&self) -> Diagnostic {
        Diagnostic::new(0, Severity::Error, self.to_string())
            .with_code(self.code())
            .with_category(Category::Format)
            .with_recoverability(Recoverability::Unrecoverable)
    }
}

/// Milliseconds fit in 48 bits.
const MAX_UNIX_MS: u64 = (1 << 48) - 1;
/// The 12-bit counter of the same millisecond.
const COUNTER_MAX: u16 = 0x0fff;
/// Mask of the 62 random bits.
const RAND_B_MASK: u64 = (1 << 62) - 1;

/// The identity of a design object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId([u8; 16]);

impl ObjectId {
    /// Builds an id from its three fields: the Unix time in milliseconds (48 bits), the 12-bit
    /// counter (`rand_a`) and 62 random bits (`rand_b`). Version and variant bits are set.
    pub fn from_parts(unix_ms: u64, counter: u16, rand_b: u64) -> Result<ObjectId, IdError> {
        if unix_ms > MAX_UNIX_MS {
            return Err(IdError::ClockOutOfRange(unix_ms));
        }
        let mut b = [0u8; 16];
        b[..6].copy_from_slice(&unix_ms.to_be_bytes()[2..]);
        let counter = counter & COUNTER_MAX;
        b[6] = 0x70 | (counter >> 8) as u8;
        b[7] = (counter & 0xff) as u8;
        let r = (rand_b & RAND_B_MASK) | (0b10 << 62);
        b[8..].copy_from_slice(&r.to_be_bytes());
        Ok(ObjectId(b))
    }

    /// The 16 bytes of the id.
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Reads the 16 bytes back, checking version 7 and the RFC variant.
    pub fn from_bytes(bytes: [u8; 16]) -> Result<ObjectId, IdError> {
        if bytes[6] >> 4 != 7 {
            return Err(IdError::InvalidText {
                reason: "the version is not 7",
            });
        }
        if bytes[8] >> 6 != 0b10 {
            return Err(IdError::InvalidText {
                reason: "the variant is not the RFC one",
            });
        }
        Ok(ObjectId(bytes))
    }

    /// The time stored in the id, in Unix milliseconds.
    pub fn unix_ms(&self) -> u64 {
        let mut b = [0u8; 8];
        b[2..].copy_from_slice(&self.0[..6]);
        u64::from_be_bytes(b)
    }

    /// The 12-bit counter stored in the id.
    pub fn counter(&self) -> u16 {
        (u16::from(self.0[6] & 0x0f) << 8) | u16::from(self.0[7])
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut out = String::with_capacity(36);
        for (i, byte) in self.0.iter().enumerate() {
            if matches!(i, 4 | 6 | 8 | 10) {
                out.push('-');
            }
            out.push(HEX[usize::from(byte >> 4)] as char);
            out.push(HEX[usize::from(byte & 0x0f)] as char);
        }
        f.write_str(&out)
    }
}

impl FromStr for ObjectId {
    type Err = IdError;

    /// Only the canonical form is accepted: 36 characters, lowercase hexadecimal, hyphens at
    /// positions 8, 13, 18 and 23, version 7 and the RFC variant.
    fn from_str(text: &str) -> Result<ObjectId, IdError> {
        let bad = |reason| IdError::InvalidText { reason };
        let t = text.as_bytes();
        if t.len() != 36 {
            return Err(bad("the length is not 36"));
        }
        let mut bytes = [0u8; 16];
        let mut n = 0;
        let mut i = 0;
        while i < 36 {
            if matches!(i, 8 | 13 | 18 | 23) {
                if t[i] != b'-' {
                    return Err(bad("a hyphen is missing"));
                }
                i += 1;
                continue;
            }
            let hi = hex(t[i]).ok_or(bad("not lowercase hexadecimal"))?;
            let lo = hex(t[i + 1]).ok_or(bad("not lowercase hexadecimal"))?;
            bytes[n] = (hi << 4) | lo;
            n += 1;
            i += 2;
        }
        ObjectId::from_bytes(bytes)
    }
}

fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    }
}

impl Serialize for ObjectId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for ObjectId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// A source of the current time, in Unix milliseconds.
pub trait Clock {
    /// The current time.
    fn now_unix_ms(&mut self) -> u64;
}

/// A source of random 64-bit values.
pub trait RandomSource {
    /// The next value.
    fn next_u64(&mut self) -> u64;
}

/// The wall clock. Used by the application, never by a writer or a test.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_unix_ms(&mut self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
    }
}

/// A clock for tests: it returns `start`, then adds `step` on every call (a step of zero keeps
/// the time still, which exercises the counter).
#[derive(Debug, Clone, Copy)]
pub struct StepClock {
    next: u64,
    step: u64,
}

impl StepClock {
    /// A clock that starts at `start` and moves `step` milliseconds per reading.
    pub fn new(start: u64, step: u64) -> StepClock {
        StepClock { next: start, step }
    }
}

impl Clock for StepClock {
    fn now_unix_ms(&mut self) -> u64 {
        let now = self.next;
        self.next = self.next.saturating_add(self.step);
        now
    }
}

/// A seeded generator (SplitMix64) for reproducible tests. It is not secret and not for keys.
#[derive(Debug, Clone, Copy)]
pub struct DeterministicRandom {
    state: u64,
}

impl DeterministicRandom {
    /// The same seed always gives the same sequence.
    pub fn new(seed: u64) -> DeterministicRandom {
        DeterministicRandom { state: seed }
    }
}

impl RandomSource for DeterministicRandom {
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}

/// Randomness for the application: every call hashes a counter with a hasher that the standard
/// library seeds from the operating system. Enough to make ids unique; not for secrets.
#[derive(Debug, Default)]
pub struct SystemRandom {
    counter: u64,
}

impl RandomSource for SystemRandom {
    fn next_u64(&mut self) -> u64 {
        use std::hash::{BuildHasher, Hasher};
        self.counter = self.counter.wrapping_add(1);
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u64(self.counter);
        h.finish()
    }
}

/// Produces UUID v7 ids that grow with time. Within one millisecond (or when the clock moves
/// back) the 12-bit counter grows instead, so ids from one generator are strictly increasing.
#[derive(Debug)]
pub struct IdGenerator<C: Clock, R: RandomSource> {
    clock: C,
    random: R,
    last_ms: u64,
    counter: u16,
    started: bool,
}

impl IdGenerator<SystemClock, SystemRandom> {
    /// The generator of the running application.
    pub fn system() -> Self {
        IdGenerator::new(SystemClock, SystemRandom::default())
    }
}

impl<C: Clock, R: RandomSource> IdGenerator<C, R> {
    /// A generator over the given clock and random source.
    pub fn new(clock: C, random: R) -> Self {
        IdGenerator {
            clock,
            random,
            last_ms: 0,
            counter: 0,
            started: false,
        }
    }

    /// The next id.
    pub fn generate(&mut self) -> Result<ObjectId, IdError> {
        let now = self.clock.now_unix_ms();
        if !self.started || now > self.last_ms {
            self.started = true;
            self.last_ms = now;
            // Start below the top of the range so the counter has room for the same millisecond.
            self.counter = (self.random.next_u64() & 0x01ff) as u16;
        } else if self.counter < COUNTER_MAX {
            self.counter += 1;
        } else {
            // The counter is exhausted: borrow the next millisecond.
            self.last_ms += 1;
            self.counter = 0;
        }
        let rand_b = self.random.next_u64();
        ObjectId::from_parts(self.last_ms, self.counter, rand_b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gen(start: u64, step: u64, seed: u64) -> IdGenerator<StepClock, DeterministicRandom> {
        IdGenerator::new(StepClock::new(start, step), DeterministicRandom::new(seed))
    }

    #[test]
    fn text_form_is_canonical_lowercase_v7() {
        let id = ObjectId::from_parts(0x0123_4567_89ab, 0x0cde, 0x3fed_cba9_8765_4321).unwrap();
        assert_eq!(id.to_string(), "01234567-89ab-7cde-bfed-cba987654321");
        assert_eq!(id.unix_ms(), 0x0123_4567_89ab);
        assert_eq!(id.counter(), 0x0cde);
        assert_eq!(
            "01234567-89ab-7cde-bfed-cba987654321"
                .parse::<ObjectId>()
                .unwrap(),
            id
        );
    }

    #[test]
    fn parsing_is_strict() {
        let good = "01234567-89ab-7cde-bfed-cba987654321";
        assert!(good.parse::<ObjectId>().is_ok());
        for bad in [
            "01234567-89AB-7cde-bfed-cba987654321",  // uppercase
            "01234567-89ab-4cde-bfed-cba987654321",  // version 4
            "01234567-89ab-7cde-4fed-cba987654321",  // wrong variant
            "0123456789ab-7cde-bfed-cba987654321",   // hyphen missing
            "01234567-89ab-7cde-bfed-cba98765432",   // short
            "01234567-89ab-7cde-bfed-cba9876543211", // long
            "01234567-89ab-7cde-bfed-cba98765432g",  // not hex
            "{01234567-89ab-7cde-bfed-cba987654321}",
            "01234567_89ab_7cde_bfed_cba987654321",
            "",
        ] {
            assert!(bad.parse::<ObjectId>().is_err(), "{bad}");
        }
        // Multibyte characters must not panic on slicing.
        assert!("0123456é-89ab-7cde-bfed-cba987654321"
            .parse::<ObjectId>()
            .is_err());
    }

    #[test]
    fn the_clock_must_fit_48_bits() {
        assert!(ObjectId::from_parts(MAX_UNIX_MS, 0, 0).is_ok());
        assert_eq!(
            ObjectId::from_parts(MAX_UNIX_MS + 1, 0, 0),
            Err(IdError::ClockOutOfRange(MAX_UNIX_MS + 1))
        );
        let mut g = gen(u64::MAX, 0, 1);
        assert!(matches!(g.generate(), Err(IdError::ClockOutOfRange(_))));
    }

    #[test]
    fn the_same_seed_gives_the_same_ids() {
        let mut a = gen(1_700_000_000_000, 1, 42);
        let mut b = gen(1_700_000_000_000, 1, 42);
        for _ in 0..50 {
            assert_eq!(a.generate().unwrap(), b.generate().unwrap());
        }
        let mut c = gen(1_700_000_000_000, 1, 43);
        let mut a2 = gen(1_700_000_000_000, 1, 42);
        assert_ne!(a2.generate().unwrap(), c.generate().unwrap());
    }

    #[test]
    fn a_pinned_sequence_does_not_drift() {
        // Fixes the algorithm: a change of the counter or the random layout breaks this.
        let mut g = gen(1_700_000_000_000, 0, 7);
        let first = g.generate().unwrap();
        let second = g.generate().unwrap();
        assert_eq!(first.unix_ms(), 1_700_000_000_000);
        assert_eq!(second.counter(), first.counter() + 1);
        assert_eq!(first.to_string(), "018bcfe5-6800-71d7-844c-3cd7f43c661c");
    }

    #[test]
    fn ids_grow_even_when_the_clock_stands_still_or_goes_back() {
        let mut g = gen(5_000, 0, 9);
        let mut previous = g.generate().unwrap();
        for _ in 0..10_000 {
            let next = g.generate().unwrap();
            assert!(next > previous, "{next} must be after {previous}");
            previous = next;
        }
        // 10 000 ids in one still millisecond overflow the 12-bit counter and borrow time.
        assert!(previous.unix_ms() > 5_000);

        struct Backwards(Vec<u64>);
        impl Clock for Backwards {
            fn now_unix_ms(&mut self) -> u64 {
                self.0.remove(0)
            }
        }
        let mut g = IdGenerator::new(
            Backwards(vec![9_000, 8_000, 8_000, 9_001]),
            DeterministicRandom::new(3),
        );
        let ids: Vec<_> = (0..4).map(|_| g.generate().unwrap()).collect();
        assert!(ids.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(ids[1].unix_ms(), 9_000, "the stored time never goes back");
        assert_eq!(ids[3].unix_ms(), 9_001);
    }

    #[test]
    fn generated_ids_are_unique_and_round_trip_through_text() {
        let mut g = gen(1_000, 0, 11);
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..5_000 {
            let id = g.generate().unwrap();
            let text = id.to_string();
            assert_eq!(text.parse::<ObjectId>().unwrap(), id);
            assert!(seen.insert(text));
        }
    }

    #[test]
    fn system_generator_makes_distinct_valid_ids() {
        let mut g = IdGenerator::system();
        let a = g.generate().unwrap();
        let b = g.generate().unwrap();
        assert!(a < b);
        assert!(
            a.unix_ms() > 1_600_000_000_000,
            "the system clock is after 2020"
        );
        assert_eq!(a.to_string().parse::<ObjectId>().unwrap(), a);
    }

    #[test]
    fn serde_uses_the_canonical_text() {
        let id = ObjectId::from_parts(1, 2, 3).unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, format!("\"{id}\""));
        assert_eq!(serde_json::from_str::<ObjectId>(&json).unwrap(), id);
        assert!(serde_json::from_str::<ObjectId>("\"nope\"").is_err());
        assert!(serde_json::from_str::<ObjectId>("5").is_err());
    }

    #[test]
    fn deterministic_random_matches_the_reference_splitmix64() {
        // Reference values of SplitMix64 for seed 0.
        let mut r = DeterministicRandom::new(0);
        assert_eq!(r.next_u64(), 0xe220_a839_7b1d_cdaf);
        assert_eq!(r.next_u64(), 0x6e78_9e6a_a1b9_65f4);
    }

    #[test]
    fn errors_have_stable_codes_and_diagnostics() {
        let a = IdError::InvalidText { reason: "x" };
        let b = IdError::ClockOutOfRange(1);
        assert_eq!((a.code(), b.code()), ("EH3D-ID-001", "EH3D-ID-002"));
        assert_eq!(a.diagnostic().code.as_deref(), Some("EH3D-ID-001"));
        assert_eq!(b.diagnostic().severity, Severity::Error);
    }
}
