//! Canonical JSON as in RFC 8785 (JSON Canonicalization Scheme), used by every new JSON entry of
//! the containers (not by the `.eh3dproj` 1.0.0 preset, whose bytes are fixed).
//!
//! * object keys sorted by UTF-16 code units, no insignificant whitespace;
//! * numbers in the ECMAScript shortest round-trip form (`1e+30`, `0.002`, never `2E-3`);
//! * strings with the minimal escapes (`\"`, `\\`, `\b`, `\t`, `\n`, `\f`, `\r`, other control
//!   characters as lowercase `\u00xx`; everything else literally, in UTF-8).
//!
//! Local rules on top: UTF-8 without a byte order mark, one final line feed, optional fields
//! written only when set (serde `skip_serializing_if` on the type), and integers that no double
//! equals refused (a JSON number is a double).

use serde::Serialize;
use serde_json::{Map, Number, Value};

use thiserror::Error;

/// Failures of the canonical JSON writer.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CanonicalError {
    /// An integer that no double equals (JSON numbers are doubles in RFC 8785).
    #[error("the integer {0} is not exactly representable as a JSON number")]
    IntegerNotRepresentable(String),
    /// The value could not be turned into JSON.
    #[error("serialisation failed: {0}")]
    Serialise(String),
    /// NaN or infinity cannot be written.
    #[error("a number is not finite")]
    NotFinite,
}

/// The canonical text of a value, without the final line feed.
pub fn canonical_string(value: &Value) -> Result<String, CanonicalError> {
    let mut out = String::new();
    write_value(value, &mut out)?;
    Ok(out)
}

/// The canonical bytes of a file: the canonical text plus one line feed.
pub fn canonical_file_bytes(value: &Value) -> Result<Vec<u8>, CanonicalError> {
    let mut text = canonical_string(value)?;
    text.push('\n');
    Ok(text.into_bytes())
}

/// Serialises any `Serialize` value to canonical file bytes.
pub fn to_canonical_file_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, CanonicalError> {
    let v = serde_json::to_value(value).map_err(|e| CanonicalError::Serialise(e.to_string()))?;
    canonical_file_bytes(&v)
}

fn write_value(value: &Value, out: &mut String) -> Result<(), CanonicalError> {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(n) => out.push_str(&number_string(n)?),
        Value::String(s) => write_string(s, out),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(item, out)?;
            }
            out.push(']');
        }
        Value::Object(map) => write_object(map, out)?,
    }
    Ok(())
}

fn write_object(map: &Map<String, Value>, out: &mut String) -> Result<(), CanonicalError> {
    let mut entries: Vec<(&String, &Value)> = map.iter().collect();
    entries.sort_by(|a, b| a.0.encode_utf16().cmp(b.0.encode_utf16()));
    out.push('{');
    for (i, (key, value)) in entries.into_iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_string(key, out);
        out.push(':');
        write_value(value, out)?;
    }
    out.push('}');
    Ok(())
}

fn write_string(s: &str, out: &mut String) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{0c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn number_string(n: &Number) -> Result<String, CanonicalError> {
    if let Some(i) = n.as_i64() {
        return integer_string(i128::from(i));
    }
    if let Some(u) = n.as_u64() {
        return integer_string(i128::from(u));
    }
    match n.as_f64() {
        Some(f) => es_number_string(f),
        None => Err(CanonicalError::NotFinite),
    }
}

/// An integer is written as the double it equals; one that no double equals is refused.
fn integer_string(n: i128) -> Result<String, CanonicalError> {
    let f = n as f64;
    if f as i128 != n {
        return Err(CanonicalError::IntegerNotRepresentable(n.to_string()));
    }
    es_number_string(f)
}

/// `Number::toString` of ECMAScript for a finite double: the shortest digits that round-trip,
/// laid out as plain decimal for exponents from `1e-6` up to below `1e21` and as `d.ddde+x`
/// otherwise. Negative zero is written `0`.
pub fn es_number_string(value: f64) -> Result<String, CanonicalError> {
    if !value.is_finite() {
        return Err(CanonicalError::NotFinite);
    }
    if value == 0.0 {
        return Ok("0".to_string());
    }
    let sign = if value < 0.0 { "-" } else { "" };
    // Rust prints the shortest round-trip digits in this form: `d.ddde<exp>`.
    let sci = format!("{:e}", value.abs());
    let (mantissa, exp) = sci
        .split_once('e')
        .ok_or_else(|| CanonicalError::Serialise(sci.clone()))?;
    let exp: i32 = exp
        .parse()
        .map_err(|_| CanonicalError::Serialise(sci.clone()))?;
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let k = digits.len() as i32;
    let n = exp + 1;
    let body = if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let e = n - 1;
        let e_text = format!("{}{}", if e < 0 { "-" } else { "+" }, e.abs());
        if k == 1 {
            format!("{digits}e{e_text}")
        } else {
            format!("{}.{}e{e_text}", &digits[..1], &digits[1..])
        }
    };
    Ok(format!("{sign}{body}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SplitMix64: a seeded generator so the generated documents are the same in every run.
    struct DeterministicRandom(u64);
    impl DeterministicRandom {
        fn new(seed: u64) -> Self {
            Self(seed)
        }
        fn next_u64(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        }
    }
    use serde_json::json;

    fn canon(text: &str) -> String {
        canonical_string(&serde_json::from_str::<Value>(text).unwrap()).unwrap()
    }

    /// The JSON escape `\uXXXX` built at run time.
    fn esc(hex: &str) -> String {
        let mut s = String::from("\\");
        s.push('u');
        s.push_str(hex);
        s
    }

    #[test]
    fn rfc_8785_section_3_2_4_example() {
        // Input of the RFC with its escapes: euro sign, "$", U+000F, line feed, "A", "'", "B",
        // a quote and a backslash given as escapes, then an escaped backslash, quote and slash.
        let string_body = format!(
            "{}${}{}A'{}{}{}\\\\\\\"\\/",
            esc("20ac"),
            esc("000F"),
            esc("000a"),
            esc("0042"),
            esc("0022"),
            esc("005c")
        );
        let input = format!(
            "{{\"numbers\": [333333333.33333329, 1E30, 4.50, 2e-3, 0.000000000000000000000000001], \"string\": \"{string_body}\", \"literals\": [null, true, false]}}"
        );
        let expected = format!(
            "{{\"literals\":[null,true,false],\"numbers\":[333333333.3333333,1e+30,4.5,0.002,1e-27],\"string\":\"\u{20ac}${}\\nA'B\\\"\\\\\\\\\\\"/\"}}",
            esc("000f")
        );
        assert_eq!(canon(&input), expected);
    }

    #[test]
    fn rfc_8785_section_3_2_3_key_order_is_by_utf16_code_unit() {
        let input = format!(
            "{{\"{}\": 1, \"{}\": 2, \"{}\": 3, \"1\": 4, \"{}{}\": 5, \"{}\": 6, \"{}\": 7}}",
            esc("20ac"),
            "\\r",
            esc("fb33"),
            esc("d83d"),
            esc("de00"),
            esc("0080"),
            esc("00f6")
        );
        let expected = "{\"\\r\":2,\"1\":4,\"\u{80}\":6,\"\u{f6}\":7,\"\u{20ac}\":1,\"\u{1F600}\":5,\"\u{fb33}\":3}";
        let out = canon(&input);
        assert_eq!(out, expected);
        // The emoji (surrogate pair D83D DE00) sorts before U+FB33 in UTF-16, after it in UTF-8.
        assert!(out.find('\u{1F600}').unwrap() < out.find('\u{fb33}').unwrap());
    }

    #[test]
    fn rfc_8785_appendix_b_number_vectors() {
        let cases: [(u64, &str); 17] = [
            (0x0000_0000_0000_0000, "0"),
            (0x8000_0000_0000_0000, "0"),
            (0x0000_0000_0000_0001, "5e-324"),
            (0x7fef_ffff_ffff_ffff, "1.7976931348623157e+308"),
            (0x4340_0000_0000_0000, "9007199254740992"),
            (0xc340_0000_0000_0000, "-9007199254740992"),
            (0x4430_0000_0000_0000, "295147905179352830000"),
            (0x44b5_2d02_c7e1_4af5, "9.999999999999997e+22"),
            (0x44b5_2d02_c7e1_4af6, "1e+23"),
            (0x44b5_2d02_c7e1_4af7, "1.0000000000000001e+23"),
            (0x444b_1ae4_d6e2_ef50, "1e+21"),
            (0x3eb0_c6f7_a0b5_ed8d, "0.000001"),
            (0x3eb0_c6f7_a0b5_ed8c, "9.999999999999997e-7"),
            (0x41b3_de43_5555_5553, "333333333.3333332"),
            (0x41b3_de43_5555_5554, "333333333.33333325"),
            (0x41b3_de43_5555_5555, "333333333.3333333"),
            (0x41b3_de43_5555_5556, "333333333.3333334"),
        ];
        for (bits, expected) in cases {
            assert_eq!(
                es_number_string(f64::from_bits(bits)).unwrap(),
                expected,
                "{bits:#x}"
            );
        }
    }

    #[test]
    fn number_layout_boundaries() {
        for (v, s) in [
            (1.0, "1"),
            (-1.5, "-1.5"),
            (123456789012345680000.0, "123456789012345680000"),
            (1e21, "1e+21"),
            (1.5e21, "1.5e+21"),
            (0.000001, "0.000001"),
            (0.0000001, "1e-7"),
            (1.25e-7, "1.25e-7"),
            (12.4, "12.4"),
            (100.0, "100"),
            (0.5, "0.5"),
        ] {
            assert_eq!(es_number_string(v).unwrap(), s);
        }
        assert_eq!(es_number_string(f64::NAN), Err(CanonicalError::NotFinite));
        assert_eq!(
            es_number_string(f64::INFINITY),
            Err(CanonicalError::NotFinite)
        );
    }

    #[test]
    fn integers_that_no_double_equals_are_refused() {
        assert_eq!(
            canonical_string(&json!(9007199254740992_u64)).unwrap(),
            "9007199254740992"
        );
        assert!(matches!(
            canonical_string(&json!(9007199254740993_u64)),
            Err(CanonicalError::IntegerNotRepresentable(_))
        ));
        assert!(matches!(
            canonical_string(&json!(-9007199254740993_i64)),
            Err(CanonicalError::IntegerNotRepresentable(_))
        ));
        assert_eq!(canonical_string(&json!(-5)).unwrap(), "-5");
        // 2^60 is a double: written as the double is written, so a re-read gives the same text.
        assert_eq!(
            canonical_string(&json!(1_u64 << 60)).unwrap(),
            "1152921504606847000"
        );
        assert!(canonical_string(&json!((1_u64 << 60) + 1)).is_err());
    }

    #[test]
    fn strings_use_minimal_escapes() {
        let v = json!("a\u{8}\t\n\u{c}\r\u{1}\u{1f}\"\\/\u{7f}\u{e9}\u{1F600}");
        assert_eq!(
            canonical_string(&v).unwrap(),
            "\"a\\b\\t\\n\\f\\r\\u0001\\u001f\\\"\\\\/\u{7f}\u{e9}\u{1F600}\""
        );
    }

    #[test]
    fn file_bytes_end_with_one_line_feed_and_have_no_bom() {
        let bytes = canonical_file_bytes(&json!({"b": 1, "a": [true, null]})).unwrap();
        assert_eq!(bytes, b"{\"a\":[true,null],\"b\":1}\n");
        assert!(!bytes.starts_with(&[0xef, 0xbb, 0xbf]));
    }

    #[test]
    fn nested_objects_are_sorted_everywhere_and_empty_containers_work() {
        assert_eq!(
            canon(r#"{"z":{"b":[],"a":{}},"a":[{"y":1,"x":2}]}"#),
            r#"{"a":[{"x":2,"y":1}],"z":{"a":{},"b":[]}}"#
        );
    }

    #[derive(Serialize)]
    struct Item {
        id_nm: i64,
        name: &'static str,
        #[serde(skip_serializing_if = "Option::is_none")]
        note: Option<&'static str>,
    }

    #[test]
    fn typed_values_serialise_in_key_order_and_skip_unset_fields() {
        let a = to_canonical_file_bytes(&Item {
            id_nm: 5,
            name: "x",
            note: None,
        })
        .unwrap();
        assert_eq!(a, b"{\"id_nm\":5,\"name\":\"x\"}\n");
        let b = to_canonical_file_bytes(&Item {
            id_nm: 5,
            name: "x",
            note: Some("n"),
        })
        .unwrap();
        assert_eq!(b, b"{\"id_nm\":5,\"name\":\"x\",\"note\":\"n\"}\n");
    }

    fn random_value(rng: &mut DeterministicRandom, depth: u32) -> Value {
        let pick = rng.next_u64() % if depth == 0 { 5 } else { 7 };
        match pick {
            0 => Value::Null,
            1 => Value::Bool(rng.next_u64() & 1 == 0),
            2 => json!((rng.next_u64() % 2_000_001) as i64 - 1_000_000),
            3 => {
                let mantissa = (rng.next_u64() % 1_000_000_000) as f64 / 1000.0;
                let exp = (rng.next_u64() % 60) as i32 - 30;
                json!(mantissa * 10f64.powi(exp))
            }
            4 => {
                let alphabet = [
                    'a',
                    'Z',
                    '"',
                    '\\',
                    '\n',
                    '\u{1}',
                    '\u{e9}',
                    '\u{20ac}',
                    '\u{1F600}',
                    ' ',
                ];
                let len = rng.next_u64() % 6;
                Value::String(
                    (0..len)
                        .map(|_| alphabet[(rng.next_u64() % 10) as usize])
                        .collect(),
                )
            }
            5 => Value::Array(
                (0..rng.next_u64() % 4)
                    .map(|_| random_value(rng, depth - 1))
                    .collect(),
            ),
            _ => {
                let mut m = Map::new();
                for _ in 0..rng.next_u64() % 4 {
                    let key = format!(
                        "k{}{}",
                        rng.next_u64() % 5,
                        if rng.next_u64() & 1 == 0 {
                            "\u{20ac}"
                        } else {
                            "\u{1F600}"
                        }
                    );
                    m.insert(key, random_value(rng, depth - 1));
                }
                Value::Object(m)
            }
        }
    }

    #[test]
    fn write_read_write_is_stable_for_generated_documents() {
        let mut rng = DeterministicRandom::new(2024);
        for _ in 0..500 {
            let v = random_value(&mut rng, 4);
            let first = canonical_string(&v).unwrap();
            let reread: Value = serde_json::from_str(&first).unwrap();
            let second = canonical_string(&reread).unwrap();
            assert_eq!(first, second);
            // Insertion order never matters: reverse the top-level keys and compare.
            if let Value::Object(m) = &v {
                let mut reversed = Map::new();
                for (k, x) in m.iter().rev() {
                    reversed.insert(k.clone(), x.clone());
                }
                assert_eq!(canonical_string(&Value::Object(reversed)).unwrap(), first);
            }
        }
    }
}
