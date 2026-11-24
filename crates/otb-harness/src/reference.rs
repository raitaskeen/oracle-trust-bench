//! Reference checks: run each pilot task's C reference through the oracles
//! that apply to it and compare with the spec-literal implementation and
//! the ACVP vectors. A failure here means the reference, the adapter, the
//! driver or the spec-literal code is wrong, and must be investigated
//! (GAP-REF-001); expected values are never edited to match.

use std::collections::BTreeMap;

use otb_core::{FailureClass, OracleKind, Outcome};
use otb_oracles::speclit::{mldsa, mlkem};
use otb_vectors::mlkem::{EncapDecapFile, Function};

use crate::rng::SplitMix64;
use crate::{Harness, HarnessError, Library, Tally};

/// One oracle's result on one task's reference.
#[derive(Clone, Debug)]
pub struct RefResult {
    /// Task ID.
    pub task: &'static str,
    /// Oracle.
    pub oracle: OracleKind,
    /// Outcome.
    pub outcome: Outcome,
    /// Cases run.
    pub cases: u64,
    /// Case counts by group, for reporting (e.g. ACVP test groups).
    pub groups: BTreeMap<String, u64>,
}

fn le16(b: &[u8]) -> u16 {
    u16::from_le_bytes([b[0], b[1]])
}

fn le32(b: &[u8]) -> i32 {
    i32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

fn result(task: &'static str, oracle: OracleKind, t: &Tally, detail: &str) -> RefResult {
    RefResult {
        task,
        oracle,
        outcome: t.outcome(detail),
        cases: t.total,
        groups: BTreeMap::new(),
    }
}

/// MLKEM-T01-ntt: unit vectors, extremes and seeded random inputs against
/// the spec-literal NTT (FIPS 203 Algorithm 9).
pub fn t01(h: &Harness) -> Result<RefResult, HarnessError> {
    let q = u16::try_from(mlkem::Q).expect("q fits");
    let mut inputs: Vec<[u16; 256]> = Vec::new();
    for j in 0..256 {
        let mut f = [0u16; 256];
        f[j] = 1;
        inputs.push(f);
    }
    inputs.push([0; 256]);
    inputs.push([q - 1; 256]);
    let mut rng = SplitMix64::from_label("MLKEM-T01-ntt/reference-intermediate");
    for _ in 0..1000 {
        inputs.push(std::array::from_fn(|_| {
            u16::try_from(rng.below(u64::from(q))).expect("below q")
        }));
    }
    let bytes: Vec<u8> = inputs
        .iter()
        .flatten()
        .flat_map(|c| c.to_le_bytes())
        .collect();
    let out = h.run_reference("T01", Library::MlKem, bytes, inputs.len() * 512)?;
    let mut t = Tally::default();
    for (n, (f, rec)) in inputs.iter().zip(out.chunks(512)).enumerate() {
        let want = mlkem::ntt(f);
        let ok = (0..256).all(|i| le16(&rec[2 * i..]) == want[i]);
        t.check(ok, || format!("input {n}"), FailureClass::WrongOutput);
    }
    Ok(result(
        "MLKEM-T01-ntt",
        OracleKind::ReferenceIntermediate,
        &t,
        "NTT output differs from FIPS 203 Algorithm 9",
    ))
}

/// MLKEM-T07-compress-d: every x in [0, q) for each d, against Eq. (4.7).
pub fn t07(h: &Harness) -> Result<RefResult, HarnessError> {
    let mut cases = Vec::new();
    for d in mlkem::COMPRESS_D {
        for x in 0..mlkem::Q {
            cases.push((x, d));
        }
    }
    let mut bytes = Vec::with_capacity(cases.len() * 3);
    for &(x, d) in &cases {
        bytes.extend_from_slice(&u16::try_from(x).expect("x < q").to_le_bytes());
        bytes.push(u8::try_from(d).expect("d < 12"));
    }
    let out = h.run_reference("T07", Library::MlKem, bytes, cases.len() * 2)?;
    let mut t = Tally::default();
    for (&(x, d), rec) in cases.iter().zip(out.chunks(2)) {
        let ok = u32::from(le16(rec)) == mlkem::compress(d, x);
        t.check(ok, || format!("d={d} x={x}"), FailureClass::WrongOutput);
    }
    Ok(result(
        "MLKEM-T07-compress-d",
        OracleKind::ReferenceIntermediate,
        &t,
        "Compress_d differs from FIPS 203 Eq. 4.7",
    ))
}

/// MLDSA-T41-power2round: every r in [0, q), against Algorithm 35.
pub fn t41(h: &Harness) -> Result<RefResult, HarnessError> {
    let q = i32::try_from(mldsa::Q).expect("q fits");
    let bytes: Vec<u8> = (0..q).flat_map(i32::to_le_bytes).collect();
    let n = usize::try_from(q).expect("q fits");
    let out = h.run_reference("T41", Library::MlDsa, bytes, n * 8)?;
    let mut t = Tally::default();
    for (r, rec) in (0..q).zip(out.chunks(8)) {
        let (r1, r0) = mldsa::power2round(i64::from(r));
        let ok = i64::from(le32(rec)) == r1 && i64::from(le32(&rec[4..])) == r0;
        t.check(ok, || format!("r={r}"), FailureClass::WrongOutput);
    }
    Ok(result(
        "MLDSA-T41-power2round",
        OracleKind::ReferenceIntermediate,
        &t,
        "Power2Round differs from FIPS 204 Algorithm 35",
    ))
}

/// The inputs of the T47 check: every r within 2 of a HighBits boundary
/// (found by scanning all of [0, q)) paired with five fixed z values, then
/// 1,000,000 seeded random (z, r) pairs.
#[must_use]
pub fn t47_inputs() -> Vec<(i64, i64)> {
    let q = mldsa::Q;
    let g = mldsa::GAMMA2_65;
    let mut boundaries = Vec::new();
    for r in 1..q {
        if mldsa::high_bits(r, g) != mldsa::high_bits(r - 1, g) {
            boundaries.push(r);
        }
    }
    // HighBits also changes across the wrap from q - 1 to 0.
    boundaries.push(0);
    let mut cases = Vec::new();
    for &b in &boundaries {
        for r in b - 2..=b + 1 {
            for z in [0, 1, q - 1, g, q - g] {
                cases.push((z, r.rem_euclid(q)));
            }
        }
    }
    let mut rng = SplitMix64::from_label("MLDSA-T47-make-hint/reference-intermediate");
    let qu = u64::try_from(q).expect("q fits");
    for _ in 0..1_000_000 {
        let z = i64::try_from(rng.below(qu)).expect("below q");
        let r = i64::try_from(rng.below(qu)).expect("below q");
        cases.push((z, r));
    }
    cases
}

/// MLDSA-T47-make-hint against Algorithm 39.
pub fn t47(h: &Harness) -> Result<RefResult, HarnessError> {
    let cases = t47_inputs();
    let mut bytes = Vec::with_capacity(cases.len() * 8);
    for &(z, r) in &cases {
        bytes.extend_from_slice(&i32::try_from(z).expect("z < q").to_le_bytes());
        bytes.extend_from_slice(&i32::try_from(r).expect("r < q").to_le_bytes());
    }
    let out = h.run_reference("T47", Library::MlDsa, bytes, cases.len() * 4)?;
    let mut t = Tally::default();
    for (&(z, r), rec) in cases.iter().zip(out.chunks(4)) {
        let want = i32::from(mldsa::make_hint(z, r, mldsa::GAMMA2_65));
        t.check(
            le32(rec) == want,
            || format!("z={z} r={r}"),
            FailureClass::WrongOutput,
        );
    }
    Ok(result(
        "MLDSA-T47-make-hint",
        OracleKind::ReferenceIntermediate,
        &t,
        "MakeHint differs from FIPS 204 Algorithm 39",
    ))
}

/// One T24 test case sent to the driver.
struct Case {
    id: String,
    group: String,
    ct: Vec<u8>,
    dk: Vec<u8>,
    want_ret_zero: bool,
    want_ss: Option<Vec<u8>>,
}

/// MLKEM-T24 ACVP oracle: every ML-KEM-768 decapsulation case (shared secret
/// and return value 0) and every decapsulation-key-check case (return value
/// 0 for a valid key, nonzero for a key failing the section 7.3 hash check;
/// the ciphertext is a zero ciphertext of the correct length).
pub fn t24_acvp(h: &Harness) -> Result<RefResult, HarnessError> {
    let file: EncapDecapFile = otb_vectors::read_json(
        &h.repo
            .join("third_party/acvp/ML-KEM-encapDecap-FIPS203/internalProjection.json"),
    )?;
    let mut cases = Vec::new();
    for g in file
        .test_groups
        .iter()
        .filter(|g| g.parameter_set == "ML-KEM-768")
    {
        for tc in &g.tests {
            let reason = tc.reason.clone().unwrap_or_default();
            let dk = tc.dk.clone().map_or_else(Vec::new, |x| x.0);
            match g.function {
                Function::Decapsulation => cases.push(Case {
                    id: format!("decapsulation tcId {}", tc.tc_id),
                    group: format!("decapsulation: {reason}"),
                    ct: tc.c.clone().map_or_else(Vec::new, |x| x.0),
                    dk,
                    want_ret_zero: true,
                    want_ss: tc.k.clone().map(|x| x.0),
                }),
                Function::DecapsulationKeyCheck => cases.push(Case {
                    id: format!("decapsulationKeyCheck tcId {}", tc.tc_id),
                    group: format!("decapsulationKeyCheck: {reason}"),
                    ct: vec![0u8; 1088],
                    dk,
                    want_ret_zero: tc.test_passed == Some(true),
                    want_ss: None,
                }),
                Function::Encapsulation | Function::EncapsulationKeyCheck => {}
            }
        }
    }
    let mut bytes = Vec::new();
    for c in &cases {
        for part in [&c.ct, &c.dk] {
            bytes.extend_from_slice(&u32::try_from(part.len()).expect("small").to_le_bytes());
            bytes.extend_from_slice(part);
        }
    }
    let out = h.run_reference("T24", Library::MlKem, bytes, cases.len() * 36)?;
    let mut t = Tally::default();
    let mut groups = BTreeMap::new();
    for (c, rec) in cases.iter().zip(out.chunks(36)) {
        let ret = le32(rec);
        let ret_ok = (ret == 0) == c.want_ret_zero;
        let ss_ok = c
            .want_ss
            .as_ref()
            .is_none_or(|k| k.as_slice() == &rec[4..36]);
        let class = if ret_ok {
            FailureClass::WrongOutput
        } else {
            FailureClass::WrongReturnCode
        };
        t.check(ret_ok && ss_ok, || c.id.clone(), class);
        *groups.entry(c.group.clone()).or_insert(0) += 1;
    }
    let mut r = result(
        "MLKEM-T24-decaps-implicit-rejection",
        OracleKind::Acvp,
        &t,
        "ACVP ML-KEM-768 decapsulation case failed",
    );
    r.groups = groups;
    Ok(r)
}

/// Every reference check that exists so far, in task order.
#[must_use]
pub fn run_all(h: &Harness) -> Vec<Result<RefResult, HarnessError>> {
    vec![t41(h), t47(h), t01(h), t07(h), t24_acvp(h)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t47_inputs_cover_all_boundaries() {
        let cases = t47_inputs();
        // ML-DSA-65: HighBits takes 16 values, so there are 16 boundaries
        // counted by the scan (15 inside [0, q) plus the special case at
        // q - 1 - gamma2) plus the wrap at 0.
        let boundary_cases = cases.len() - 1_000_000;
        assert_eq!(boundary_cases % 20, 0);
        assert!(boundary_cases >= 16 * 20, "{boundary_cases}");
        assert!(
            cases
                .iter()
                .all(|&(z, r)| (0..mldsa::Q).contains(&z) && (0..mldsa::Q).contains(&r))
        );
    }
}
