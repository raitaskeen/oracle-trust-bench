//! Hex-encoded byte strings as they appear in ACVP files (upper or lower case).

use std::fmt;

use serde::{Deserialize, Deserializer};

/// Bytes decoded from a hex string.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct Hex(pub Vec<u8>);

impl fmt::Debug for Hex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hex({} bytes)", self.0.len())
    }
}

impl Hex {
    /// Decodes a hex string. Returns `None` on odd length or a non-hex digit.
    #[must_use]
    pub fn decode(s: &str) -> Option<Self> {
        let b = s.as_bytes();
        if !b.len().is_multiple_of(2) {
            return None;
        }
        let nibble = |c: u8| match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            b'A'..=b'F' => Some(c - b'A' + 10),
            _ => None,
        };
        b.chunks(2)
            .map(|p| Some((nibble(p[0])? << 4) | nibble(p[1])?))
            .collect::<Option<Vec<u8>>>()
            .map(Self)
    }

    /// The bytes as a fixed-size array, if the length matches.
    #[must_use]
    pub fn array<const N: usize>(&self) -> Option<[u8; N]> {
        self.0.as_slice().try_into().ok()
    }
}

impl<'de> Deserialize<'de> for Hex {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::decode(&s).ok_or_else(|| serde::de::Error::custom("invalid hex string"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn decodes_both_cases() {
        assert_eq!(Hex::decode("00ffAb").unwrap().0, vec![0x00, 0xff, 0xab]);
        assert_eq!(Hex::decode("").unwrap().0, Vec::<u8>::new());
    }

    #[test]
    fn rejects_bad_input() {
        assert!(Hex::decode("abc").is_none());
        assert!(Hex::decode("zz").is_none());
        assert!(Hex::decode("0g").is_none());
    }

    proptest! {
        #[test]
        fn round_trips(bytes in prop::collection::vec(any::<u8>(), 0..64), upper in any::<bool>()) {
            let s: String = bytes.iter().map(|b| if upper { format!("{b:02X}") } else { format!("{b:02x}") }).collect();
            prop_assert_eq!(Hex::decode(&s).unwrap().0, bytes);
        }

        #[test]
        fn never_panics(s in ".*") {
            let _ = Hex::decode(&s);
        }
    }
}
