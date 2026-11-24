//! Canonical JSON following RFC 8785 (JSON Canonicalization Scheme), restricted
//! to documents without floating-point numbers.
//!
//! Every hashed file in this repository is written through this module, so the
//! same data always gives the same bytes. Floats are rejected rather than
//! formatted: rates are stored as integer counts and money as integer
//! micro-dollars, which keeps the number rules of RFC 8785 section 3.2.2.3
//! out of scope (see ADR-002).
//!
//! Implemented rules:
//! - no whitespace between tokens (section 3.2.1);
//! - literals `null`, `true`, `false` (3.2.2.1);
//! - strings: U+0000..U+001F as `\b \t \n \f \r` or lowercase `\u00hh`,
//!   `"` and `\` escaped, everything else written as-is (3.2.2.2);
//! - integers in plain decimal; any non-integer is an error (3.2.2.3, narrowed);
//! - object members sorted by their names as UTF-16 code units, recursively;
//!   array order kept (3.2.3);
//! - UTF-8 output (3.2.4).

use std::cmp::Ordering;
use std::fmt::Write as _;

use serde::Serialize;
use serde_json::Value;

/// Errors from canonicalization.
#[derive(Debug, thiserror::Error)]
pub enum CanonError {
    /// The document contains a number that is not an integer.
    #[error("non-integer number {0} is not allowed in canonical documents")]
    NonInteger(String),
    /// The value could not be converted to JSON, or the input text is not JSON.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

/// Serializes `value` as canonical JSON text.
pub fn to_canonical_string<T: Serialize + ?Sized>(value: &T) -> Result<String, CanonError> {
    let v = serde_json::to_value(value)?;
    let mut out = String::new();
    write_value(&v, &mut out)?;
    Ok(out)
}

/// Parses JSON text and re-serializes it canonically.
pub fn canonicalize_str(json: &str) -> Result<String, CanonError> {
    let v: Value = serde_json::from_str(json)?;
    let mut out = String::new();
    write_value(&v, &mut out)?;
    Ok(out)
}

fn write_value(v: &Value, out: &mut String) -> Result<(), CanonError> {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                write!(out, "{i}").expect("writing to a String cannot fail");
            } else if let Some(u) = n.as_u64() {
                write!(out, "{u}").expect("writing to a String cannot fail");
            } else {
                return Err(CanonError::NonInteger(n.to_string()));
            }
        }
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
        Value::Object(map) => {
            let mut members: Vec<(&String, &Value)> = map.iter().collect();
            members.sort_by(|a, b| cmp_utf16(a.0, b.0));
            out.push('{');
            for (i, (k, val)) in members.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(k, out);
                out.push(':');
                write_value(val, out)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

fn write_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{0C}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            c if u32::from(c) < 0x20 => {
                write!(out, "\\u{:04x}", u32::from(c)).expect("writing to a String cannot fail");
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Compares two strings as sequences of UTF-16 code units (RFC 8785 section 3.2.3).
fn cmp_utf16(a: &str, b: &str) -> Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// RFC 8785 section 3.2.3: the sorting example. The expected order of the
    /// values is quoted from the RFC.
    #[test]
    fn rfc8785_sorting_example() {
        // Keys are written with Rust escapes: U+20AC, CR, U+FB33, "1",
        // U+1F600, U+0080, U+00F6 (the RFC's seven names, in its input order).
        let input = serde_json::json!({
            "\u{20ac}": "Euro Sign",
            "\r": "Carriage Return",
            "\u{fb33}": "Hebrew Letter Dalet With Dagesh",
            "1": "One",
            "\u{1f600}": "Emoji: Grinning Face",
            "\u{80}": "Control",
            "\u{f6}": "Latin Small Letter O With Diaeresis"
        });
        let out = to_canonical_string(&input).unwrap();
        let v: serde_json::Map<String, Value> = serde_json::from_str(&out).unwrap();
        // serde_json's Map does not keep order, so read the order from the text.
        let order: Vec<&str> = [
            "Carriage Return",
            "One",
            "Control",
            "Latin Small Letter O With Diaeresis",
            "Euro Sign",
            "Emoji: Grinning Face",
            "Hebrew Letter Dalet With Dagesh",
        ]
        .to_vec();
        let positions: Vec<usize> = order.iter().map(|s| out.find(s).unwrap()).collect();
        assert!(positions.windows(2).all(|w| w[0] < w[1]), "{out}");
        assert_eq!(v.len(), 7);
    }

    /// RFC 8785 section 3.2.2 example, with the "numbers" member removed
    /// because this module rejects non-integers by design. The expected
    /// "literals" and "string" output is the RFC's canonical output.
    #[test]
    fn rfc8785_string_and_literals_example() {
        let input = r#"{"string": "€$\u000F\u000aA'B\"\\\\\"\/", "literals": [null, true, false]}"#;
        let expected = "{\"literals\":[null,true,false],\"string\":\"\u{20ac}$\\u000f\\nA'B\\\"\\\\\\\\\\\"/\"}";
        assert_eq!(canonicalize_str(input).unwrap(), expected);
    }

    #[test]
    fn rfc8785_numbers_are_rejected() {
        let input =
            r#"{"numbers": [333333333.33333329, 1E30, 4.50, 2e-3, 0.000000000000000000000000001]}"#;
        assert!(matches!(
            canonicalize_str(input),
            Err(CanonError::NonInteger(_))
        ));
    }

    #[test]
    fn integers_and_nesting() {
        let out =
            canonicalize_str(r#"{"b": [3, -1, {"z": 0, "a": 18446744073709551615}], "a": {}}"#)
                .unwrap();
        assert_eq!(
            out,
            r#"{"a":{},"b":[3,-1,{"a":18446744073709551615,"z":0}]}"#
        );
    }

    #[test]
    fn utf16_order_differs_from_utf8_order() {
        // U+FB33 (one UTF-16 unit, 0xFB33) sorts after U+1F600 (surrogates
        // starting 0xD83D) in UTF-16 order, but before it in UTF-8 byte order.
        assert_eq!(cmp_utf16("\u{fb33}", "\u{1f600}"), Ordering::Greater);
        assert!("\u{fb33}".as_bytes() < "\u{1f600}".as_bytes());
    }

    fn arb_json() -> impl Strategy<Value = Value> {
        let leaf = prop_oneof![
            Just(Value::Null),
            any::<bool>().prop_map(Value::Bool),
            any::<i64>().prop_map(Value::from),
            any::<u64>().prop_map(Value::from),
            any::<String>().prop_map(Value::String),
        ];
        leaf.prop_recursive(4, 64, 8, |inner| {
            prop_oneof![
                prop::collection::vec(inner.clone(), 0..8).prop_map(Value::Array),
                prop::collection::btree_map(any::<String>(), inner, 0..8)
                    .prop_map(|m| Value::Object(m.into_iter().collect())),
            ]
        })
    }

    proptest! {
        #[test]
        fn canonical_is_idempotent_and_lossless(v in arb_json()) {
            let once = to_canonical_string(&v).unwrap();
            let parsed: Value = serde_json::from_str(&once).unwrap();
            prop_assert_eq!(&parsed, &v);
            let twice = canonicalize_str(&once).unwrap();
            prop_assert_eq!(once, twice);
        }
    }
}
