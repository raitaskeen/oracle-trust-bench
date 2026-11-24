//! ACVP ML-DSA vector files: `ML-DSA-keyGen-FIPS204`,
//! `ML-DSA-sigGen-FIPS204` and `ML-DSA-sigVer-FIPS204`.

use serde::Deserialize;

use crate::hex::Hex;

/// `ML-DSA-keyGen-FIPS204/internalProjection.json`.
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

/// One keyGen group.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KeyGenGroup {
    /// Group ID.
    pub tg_id: u64,
    /// Test type.
    pub test_type: String,
    /// Parameter set, e.g. `ML-DSA-65`.
    pub parameter_set: String,
    /// Cases.
    pub tests: Vec<KeyGenCase>,
}

/// One keyGen case.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KeyGenCase {
    /// Case ID.
    pub tc_id: u64,
    /// ACVP bookkeeping flag.
    pub deferred: bool,
    /// Seed xi.
    pub seed: Hex,
    /// Expected public key.
    pub pk: Hex,
    /// Expected private key.
    pub sk: Hex,
}

/// `ML-DSA-sigGen-FIPS204` and `ML-DSA-sigVer-FIPS204` files.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SigFile {
    /// Vector set ID.
    pub vs_id: u64,
    /// Algorithm.
    pub algorithm: String,
    /// Mode, `sigGen` or `sigVer`.
    pub mode: String,
    /// Revision.
    pub revision: String,
    /// Sample flag.
    pub is_sample: bool,
    /// Test groups.
    pub test_groups: Vec<SigGroup>,
}

/// One sigGen or sigVer group.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SigGroup {
    /// Group ID.
    pub tg_id: u64,
    /// Test type.
    pub test_type: String,
    /// Parameter set.
    pub parameter_set: String,
    /// sigGen only: true if `rnd` is all zeros (deterministic signing).
    pub deterministic: Option<bool>,
    /// `internal` or `external`.
    pub signature_interface: String,
    /// `pure`, `preHash` or `none`.
    pub pre_hash: String,
    /// True if the message input is the 64-byte mu.
    pub external_mu: bool,
    /// sigGen only: corner-case label.
    pub corner_case: Option<String>,
    /// Cases.
    pub tests: Vec<SigCase>,
}

/// One sigGen or sigVer case.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SigCase {
    /// Case ID.
    pub tc_id: u64,
    /// ACVP bookkeeping flag.
    pub deferred: bool,
    /// Message (absent when mu is given).
    pub message: Option<Hex>,
    /// External mu.
    pub mu: Option<Hex>,
    /// Context string (external interface).
    pub context: Option<Hex>,
    /// Hash algorithm for pre-hash groups.
    pub hash_alg: Option<String>,
    /// Public key.
    pub pk: Option<Hex>,
    /// Private key.
    pub sk: Option<Hex>,
    /// Signing randomness (hedged groups).
    pub rnd: Option<Hex>,
    /// Expected (sigGen) or given (sigVer) signature.
    pub signature: Option<Hex>,
    /// sigVer only: expected verification result.
    pub test_passed: Option<bool>,
    /// sigVer only: why the case exists.
    pub reason: Option<String>,
}
