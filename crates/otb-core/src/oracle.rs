//! The closed set of independent, non-LLM checks.

use serde::{Deserialize, Serialize};

/// One kind of independent check. The set is closed: adding a kind is a
/// protocol change and needs an update to `protocol/ORACLE_MATRIX.md`.
///
/// Whether a kind applies to a task is decided statically by the task spec
/// and the oracle matrix, never at run time (ADR-004).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OracleKind {
    /// Outputs compared with the instrumented reference implementations and
    /// the spec-literal Rust implementation.
    ReferenceIntermediate,
    /// NIST ACVP vectors (top-level functions only).
    Acvp,
    /// Wrong lengths, non-canonical values and the FIPS 203 section 7 checks.
    MalformedInput,
    /// Modified ciphertexts must yield `J(z || c)` with the valid-path return value.
    ImplicitRejection,
    /// Valgrind memcheck with secrets marked undefined.
    CtValgrind,
    /// Division on secret data, at emitted-code level (primary) and
    /// source/IR level (diagnostic). See ADR-005.
    CtStaticDivision,
    /// dudect timing test on a pre-registered subsample. Never in default CI.
    CtDudect,
    /// Differential fuzzing against a C original (Arm 3; not in the pilot).
    Differential,
}

impl OracleKind {
    /// Every kind, in canonical order.
    pub const ALL: [Self; 8] = [
        Self::ReferenceIntermediate,
        Self::Acvp,
        Self::MalformedInput,
        Self::ImplicitRejection,
        Self::CtValgrind,
        Self::CtStaticDivision,
        Self::CtDudect,
        Self::Differential,
    ];

    /// The stable name used in files and on the command line.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::ReferenceIntermediate => "reference-intermediate",
            Self::Acvp => "acvp",
            Self::MalformedInput => "malformed-input",
            Self::ImplicitRejection => "implicit-rejection",
            Self::CtValgrind => "ct-valgrind",
            Self::CtStaticDivision => "ct-static-division",
            Self::CtDudect => "ct-dudect",
            Self::Differential => "differential",
        }
    }

    /// Parses a stable name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.name() == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_match_serde_and_round_trip() {
        for k in OracleKind::ALL {
            let json = serde_json::to_string(&k).unwrap();
            assert_eq!(json, format!("\"{}\"", k.name()));
            assert_eq!(OracleKind::from_name(k.name()), Some(k));
        }
        assert_eq!(OracleKind::from_name("nope"), None);
    }

    #[test]
    fn all_is_sorted_and_unique() {
        let mut v = OracleKind::ALL.to_vec();
        v.sort();
        v.dedup();
        assert_eq!(v, OracleKind::ALL.to_vec());
    }
}
