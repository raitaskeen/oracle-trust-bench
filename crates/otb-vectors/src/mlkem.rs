//! ACVP ML-KEM vector files: `ML-KEM-keyGen-FIPS203` and
//! `ML-KEM-encapDecap-FIPS203` (ACVP ML-KEM specification, sections 6-9).

use serde::Deserialize;

use crate::hex::Hex;

/// `ML-KEM-keyGen-FIPS203/internalProjection.json`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KeyGenFile {
    /// Vector set ID.
    pub vs_id: u64,
    /// Algorithm.
    pub algorithm: String,
    /// Mode.
    pub mode: String,
    /// Revision.
    pub revision: String,
    /// Sample flag.
    pub is_sample: bool,
    /// Test groups.
    pub test_groups: Vec<KeyGenGroup>,
}

/// One keyGen test group.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KeyGenGroup {
    /// Group ID.
    pub tg_id: u64,
    /// Test type, `AFT`.
    pub test_type: String,
    /// Parameter set, e.g. `ML-KEM-768`.
    pub parameter_set: String,
    /// Test cases.
    pub tests: Vec<KeyGenCase>,
}

/// One keyGen case: seeds in, keys out.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KeyGenCase {
    /// Case ID.
    pub tc_id: u64,
    /// ACVP bookkeeping flag.
    pub deferred: bool,
    /// Seed z.
    pub z: Hex,
    /// Seed d.
    pub d: Hex,
    /// Expected encapsulation key.
    pub ek: Hex,
    /// Expected decapsulation key.
    pub dk: Hex,
}

/// `ML-KEM-encapDecap-FIPS203/internalProjection.json`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncapDecapFile {
    /// Vector set ID.
    pub vs_id: u64,
    /// Algorithm.
    pub algorithm: String,
    /// Mode.
    pub mode: String,
    /// Revision.
    pub revision: String,
    /// Sample flag.
    pub is_sample: bool,
    /// Test groups.
    pub test_groups: Vec<EncapDecapGroup>,
}

/// The function a group tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Function {
    /// Encaps_internal with given randomness m.
    Encapsulation,
    /// Decaps_internal, including modified ciphertexts.
    Decapsulation,
    /// The section 7.2 encapsulation key check.
    EncapsulationKeyCheck,
    /// The section 7.3 decapsulation key check.
    DecapsulationKeyCheck,
}

/// One encapDecap test group.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncapDecapGroup {
    /// Group ID.
    pub tg_id: u64,
    /// Test type, `AFT` or `VAL`.
    pub test_type: String,
    /// Parameter set.
    pub parameter_set: String,
    /// Function under test.
    pub function: Function,
    /// Test cases.
    pub tests: Vec<EncapDecapCase>,
}

/// One encapDecap case. Which fields are present depends on the function.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncapDecapCase {
    /// Case ID.
    pub tc_id: u64,
    /// ACVP bookkeeping flag.
    pub deferred: bool,
    /// Encapsulation key.
    pub ek: Option<Hex>,
    /// Decapsulation key.
    pub dk: Option<Hex>,
    /// Ciphertext.
    pub c: Option<Hex>,
    /// Shared secret.
    pub k: Option<Hex>,
    /// Encapsulation randomness.
    pub m: Option<Hex>,
    /// Why the case exists, e.g. `modified ciphertext` or `modified H`.
    pub reason: Option<String>,
    /// Expected key-check result.
    pub test_passed: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_fields() {
        let text = r#"{"tcId":1,"deferred":false,"z":"00","d":"00","ek":"00","dk":"00","extra":1}"#;
        assert!(serde_json::from_str::<KeyGenCase>(text).is_err());
    }

    #[test]
    fn parses_a_key_check_case() {
        let text = r#"{"tcId":7,"testPassed":false,"deferred":false,"ek":"AB","dk":"CD","reason":"modified H"}"#;
        let c: EncapDecapCase = serde_json::from_str(text).unwrap();
        assert_eq!(c.test_passed, Some(false));
        assert_eq!(c.reason.as_deref(), Some("modified H"));
        assert_eq!(c.ek.unwrap().0, vec![0xab]);
    }
}
