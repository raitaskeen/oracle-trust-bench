//! Stable identifiers. See `docs/adr/ADR-002-canonical-json-and-stable-ids.md`.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

/// Errors from parsing or computing an identifier.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IdError {
    /// The task ID does not match `<FAMILY>-T<NN>-<slug>`.
    #[error("invalid task id {0:?}: expected <MLKEM|MLDSA>-T<two digits>-<lowercase slug>")]
    TaskId(String),
    /// Candidate source is not valid UTF-8.
    #[error("candidate source is not valid UTF-8")]
    NotUtf8,
    /// A candidate ID string is not 64 lowercase hex digits.
    #[error("invalid candidate id {0:?}: expected 64 lowercase hex digits")]
    CandidateId(String),
    /// A toolchain ID is empty or contains whitespace.
    #[error("invalid toolchain id {0:?}")]
    ToolchainId(String),
}

/// Identifies a task, for example `MLKEM-T01-ntt`.
///
/// Format: a family (`MLKEM` or `MLDSA`), `-T`, exactly two digits, `-`, and a
/// slug of lowercase ASCII letters and digits in `-`-separated words.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(String);

impl TaskId {
    /// The ID as a string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn is_valid(s: &str) -> bool {
        let Some(rest) = s
            .strip_prefix("MLKEM-T")
            .or_else(|| s.strip_prefix("MLDSA-T"))
        else {
            return false;
        };
        let bytes = rest.as_bytes();
        if bytes.len() < 4 || !bytes[0].is_ascii_digit() || !bytes[1].is_ascii_digit() {
            return false;
        }
        if bytes[2] != b'-' {
            return false;
        }
        let slug = &rest[3..];
        !slug.is_empty()
            && slug.split('-').all(|w| {
                !w.is_empty()
                    && w.bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
            })
    }
}

impl FromStr for TaskId {
    type Err = IdError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if Self::is_valid(s) {
            Ok(Self(s.to_owned()))
        } else {
            Err(IdError::TaskId(s.to_owned()))
        }
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Identifies one candidate implementation: the SHA-256 of its normalized
/// source text.
///
/// Normalization is deliberately minimal, so that two programs that differ in
/// any meaningful way never share an ID:
///
/// 1. the source must be valid UTF-8;
/// 2. every CRLF pair becomes LF;
/// 3. trailing LF characters are removed and exactly one LF is appended.
///
/// Nothing else (whitespace, comments, BOM) is changed.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CandidateId([u8; 32]);

impl CandidateId {
    /// Applies the normalization rules above.
    pub fn normalize(source: &[u8]) -> Result<String, IdError> {
        let text = std::str::from_utf8(source).map_err(|_| IdError::NotUtf8)?;
        let mut out = text.replace("\r\n", "\n");
        let trimmed_len = out.trim_end_matches('\n').len();
        out.truncate(trimmed_len);
        out.push('\n');
        Ok(out)
    }

    /// Computes the ID of a candidate from its raw source bytes.
    pub fn from_source(source: &[u8]) -> Result<Self, IdError> {
        let normalized = Self::normalize(source)?;
        Ok(Self(Sha256::digest(normalized.as_bytes()).into()))
    }

    /// The 32 digest bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for CandidateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for CandidateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CandidateId({self})")
    }
}

impl FromStr for CandidateId {
    type Err = IdError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bad = || IdError::CandidateId(s.to_owned());
        if s.len() != 64 || !s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
            return Err(bad());
        }
        let mut out = [0u8; 32];
        for (i, byte) in out.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).map_err(|_| bad())?;
        }
        Ok(Self(out))
    }
}

/// Identifies the exact toolchain that produced a verdict: for example the
/// container image digest plus the compiler version digest. Opaque to this
/// crate; must be non-empty and contain no whitespace.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ToolchainId(String);

impl ToolchainId {
    /// The ID as a string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for ToolchainId {
    type Err = IdError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() || s.chars().any(char::is_whitespace) {
            Err(IdError::ToolchainId(s.to_owned()))
        } else {
            Ok(Self(s.to_owned()))
        }
    }
}

macro_rules! string_serde {
    ($t:ty) => {
        impl Serialize for $t {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.collect_str(self)
            }
        }
        impl<'de> Deserialize<'de> for $t {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let s = String::deserialize(d)?;
                s.parse().map_err(serde::de::Error::custom)
            }
        }
    };
}

impl fmt::Display for ToolchainId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

string_serde!(TaskId);
string_serde!(CandidateId);
string_serde!(ToolchainId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_ids_valid() {
        for s in [
            "MLKEM-T01-ntt",
            "MLKEM-T07-compress-d",
            "MLKEM-T24-decaps-implicit-rejection",
            "MLDSA-T41-power2round",
            "MLDSA-T47-make-hint",
        ] {
            assert_eq!(s.parse::<TaskId>().unwrap().as_str(), s);
        }
    }

    #[test]
    fn task_ids_invalid() {
        for s in [
            "",
            "MLKEM-T1-ntt",
            "MLKEM-T001-ntt",
            "MLKEM-T01",
            "MLKEM-T01-",
            "MLKEM-T01-NTT",
            "MLKEM-T01-ntt-",
            "MLKEM-T01--ntt",
            "SLHDSA-T01-x",
            "mlkem-T01-ntt",
            "MLKEM-T01_ntt",
        ] {
            assert!(s.parse::<TaskId>().is_err(), "{s:?} should be rejected");
        }
    }

    // The empty source normalizes to "\n". Expected values computed with
    // GNU coreutils: `printf '\n' | sha256sum` and
    // `printf 'int x;\n' | sha256sum`.
    #[test]
    fn candidate_id_known_vectors() {
        let empty = CandidateId::from_source(b"").unwrap();
        assert_eq!(
            empty.to_string(),
            "01ba4719c80b6fe911b091a7c05124b64eeece964e09c058ef8f9805daca546b"
        );
        let one_line = CandidateId::from_source(b"int x;\r\n").unwrap();
        assert_eq!(one_line.to_string(), INT_X_SHA256);
    }

    const INT_X_SHA256: &str = "7c725f30854a46033dd94f728ac6b08caf10845993cd3ed48e40079cdb0a76a6";

    #[test]
    fn candidate_id_normalization() {
        let a = CandidateId::from_source(b"int x;\r\nint y;\r\n\r\n").unwrap();
        let b = CandidateId::from_source(b"int x;\nint y;").unwrap();
        assert_eq!(a, b);
        // Whitespace inside lines is significant.
        let c = CandidateId::from_source(b"int  x;\nint y;").unwrap();
        assert_ne!(a, c);
        // A lone CR is not normalized.
        let d = CandidateId::from_source(b"int x;\rint y;").unwrap();
        assert_ne!(a, d);
    }

    #[test]
    fn candidate_id_rejects_non_utf8() {
        assert_eq!(
            CandidateId::from_source(&[0xff, 0xfe]),
            Err(IdError::NotUtf8)
        );
    }

    #[test]
    fn candidate_id_round_trips() {
        let id = CandidateId::from_source(b"void f(void) {}\n").unwrap();
        let s = id.to_string();
        assert_eq!(s.parse::<CandidateId>().unwrap(), id);
        assert!("ABC".parse::<CandidateId>().is_err());
        assert!(s.to_uppercase().parse::<CandidateId>().is_err());
    }
}
