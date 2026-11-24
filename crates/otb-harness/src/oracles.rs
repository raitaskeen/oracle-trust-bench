//! The oracles implemented so far, for any implementation of a pilot task.
//!
//! Every oracle builds the task program with the implementation under test,
//! feeds it inputs chosen here, and compares its outputs with values
//! computed here by the spec-literal Rust code or read from the ACVP files.
//! The same functions check the references (`otb ref run`), the seeded
//! faults (`otb selftest`) and candidates (`otb check`).
//!
//! Which oracles apply to a task is decided by the task spec (the oracle
//! matrix), never here. [`implemented`] lists the ones that have code;
//! applicable oracles without code are reported as pending, never as a
//! verdict.

use std::collections::BTreeMap;

use otb_core::spec::TaskSpec;
use otb_core::{FailureClass, OracleKind, Outcome};
use otb_oracles::speclit::mlkem_kem::{self, ML_KEM_768};
use otb_oracles::speclit::{mldsa, mlkem};
use otb_vectors::mlkem::{EncapDecapFile, Function};

use crate::rng::SplitMix64;
use crate::{Harness, HarnessError, Implementation, ProgramRun, Tally, admission};

/// One oracle's result on one implementation of one task.
#[derive(Clone, Debug)]
pub struct OracleRun {
    /// Task ID (or a label for whole-library checks).
    pub task: String,
    /// Oracle.
    pub oracle: OracleKind,
    /// Outcome.
    pub outcome: Outcome,
    /// Cases run.
    pub cases: u64,
    /// Case counts by group, for reporting (e.g. ACVP test groups).
    pub groups: BTreeMap<String, u64>,
}

/// The oracles that have code for `task`, in report order.
#[must_use]
pub fn implemented(task: &str) -> &'static [OracleKind] {
    match task {
        "MLKEM-T01-ntt"
        | "MLKEM-T07-compress-d"
        | "MLDSA-T41-power2round"
        | "MLDSA-T47-make-hint" => &[OracleKind::ReferenceIntermediate],
        "MLKEM-T24-decaps-implicit-rejection" => &[
            OracleKind::Acvp,
            OracleKind::MalformedInput,
            OracleKind::ImplicitRejection,
        ],
        _ => &[],
    }
}

/// The driver selector for a task ID: `MLKEM-T07-compress-d` gives `T07`.
#[must_use]
pub fn task_flag(task: &str) -> &str {
    task.split('-').nth(1).unwrap_or("")
}

/// Runs one implemented oracle.
///
/// # Errors
///
/// A harness failure (staging, sandbox, vectors), or an oracle with no
/// code for this task.
pub fn run_oracle(
    h: &Harness,
    task: &str,
    oracle: OracleKind,
    imp: &Implementation,
) -> Result<OracleRun, HarnessError> {
    match (task, oracle) {
        ("MLKEM-T01-ntt", OracleKind::ReferenceIntermediate) => t01(h, imp),
        ("MLKEM-T07-compress-d", OracleKind::ReferenceIntermediate) => t07(h, imp),
        ("MLDSA-T41-power2round", OracleKind::ReferenceIntermediate) => t41(h, imp),
        ("MLDSA-T47-make-hint", OracleKind::ReferenceIntermediate) => t47(h, imp),
        ("MLKEM-T24-decaps-implicit-rejection", OracleKind::Acvp) => {
            run_t24(h, imp, OracleKind::Acvp, &t24_acvp_cases(h)?, ACVP_DETAIL)
        }
        ("MLKEM-T24-decaps-implicit-rejection", OracleKind::MalformedInput) => run_t24(
            h,
            imp,
            OracleKind::MalformedInput,
            &t24_malformed_cases(),
            "input check missing or valid input refused (FIPS 203 section 7.3)",
        ),
        ("MLKEM-T24-decaps-implicit-rejection", OracleKind::ImplicitRejection) => run_t24(
            h,
            imp,
            OracleKind::ImplicitRejection,
            &t24_implicit_rejection_cases(),
            "modified ciphertext not answered with J(z || c) and return value 0 (FIPS 203 section 6.3)",
        ),
        _ => Err(HarnessError::Input(format!(
            "no {} oracle for {task}",
            oracle.name()
        ))),
    }
}

/// Everything `check` found for one implementation of one task.
#[derive(Debug)]
pub struct CheckReport {
    /// One entry per applicable, implemented oracle, in report order.
    pub runs: Vec<Result<OracleRun, HarnessError>>,
    /// Applicable oracles that have no code yet (no verdict is produced).
    pub pending: Vec<OracleKind>,
    /// Oracles the matrix marks not applicable.
    pub not_applicable: Vec<OracleKind>,
}

/// Runs every applicable, implemented oracle on `imp`. Candidates (and
/// seeded faults without a library) go through admission first; a rejected
/// source is not compiled and gets a `Rejected` outcome from every
/// applicable oracle.
///
/// # Errors
///
/// Reading the implementation's source failed.
pub fn check(
    h: &Harness,
    spec: &TaskSpec,
    imp: &Implementation,
) -> Result<CheckReport, HarnessError> {
    let task = spec.id.as_str();
    let applicable: Vec<OracleKind> = OracleKind::ALL
        .into_iter()
        .filter(|k| spec.oracles.get(k).is_some_and(|a| a.applies))
        .collect();
    let not_applicable = OracleKind::ALL
        .into_iter()
        .filter(|k| !applicable.contains(k))
        .collect();
    let have = implemented(task);
    let pending = applicable
        .iter()
        .copied()
        .filter(|k| !have.contains(k))
        .collect();
    let to_run: Vec<OracleKind> = have
        .iter()
        .copied()
        .filter(|k| applicable.contains(k))
        .collect();

    if imp.kind.needs_admission() {
        let source = std::fs::read(h.path(&imp.source))?;
        if let Err(r) = admission::check(&source) {
            let runs = to_run
                .into_iter()
                .map(|oracle| {
                    Ok(OracleRun {
                        task: task.to_owned(),
                        oracle,
                        outcome: Outcome::Rejected {
                            policy: r.policy.clone(),
                        },
                        cases: 0,
                        groups: BTreeMap::new(),
                    })
                })
                .collect();
            return Ok(CheckReport {
                runs,
                pending,
                not_applicable,
            });
        }
    }
    let runs = to_run
        .into_iter()
        .map(|oracle| run_oracle(h, task, oracle, imp))
        .collect();
    Ok(CheckReport {
        runs,
        pending,
        not_applicable,
    })
}

fn le16(b: &[u8]) -> u16 {
    u16::from_le_bytes([b[0], b[1]])
}

fn le32(b: &[u8]) -> i32 {
    i32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

/// Builds and runs `imp`, then either compares its records with `compare`
/// or, if the program failed, fails every case with the program's class.
fn finish(
    task: &str,
    oracle: OracleKind,
    run: ProgramRun,
    cases: usize,
    detail: &str,
    compare: impl FnOnce(&[u8]) -> (Tally, BTreeMap<String, u64>),
) -> OracleRun {
    let (outcome, groups) = match run {
        ProgramRun::Output(out) => {
            let (t, groups) = compare(&out);
            (t.outcome(detail), groups)
        }
        ProgramRun::Failed { class, detail } => {
            let n = u64::try_from(cases).expect("fits");
            let t = Tally {
                total: n,
                failed: n,
                first_failure: None,
                class: Some(class),
            };
            (t.outcome(&detail), BTreeMap::new())
        }
    };
    OracleRun {
        task: task.to_owned(),
        oracle,
        outcome,
        cases: u64::try_from(cases).expect("fits"),
        groups,
    }
}

/// MLKEM-T01-ntt: unit vectors, extremes and seeded random inputs against
/// the spec-literal NTT (FIPS 203 Algorithm 9).
fn t01(h: &Harness, imp: &Implementation) -> Result<OracleRun, HarnessError> {
    const TASK: &str = "MLKEM-T01-ntt";
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
    let run = h.run_impl("T01", imp, bytes, inputs.len() * 512)?;
    Ok(finish(
        TASK,
        OracleKind::ReferenceIntermediate,
        run,
        inputs.len(),
        "NTT output differs from FIPS 203 Algorithm 9",
        |out| {
            let mut t = Tally::default();
            for (n, (f, rec)) in inputs.iter().zip(out.chunks(512)).enumerate() {
                let want = mlkem::ntt(f);
                let ok = (0..256).all(|i| le16(&rec[2 * i..]) == want[i]);
                t.check(ok, || format!("input {n}"), FailureClass::WrongOutput);
            }
            (t, BTreeMap::new())
        },
    ))
}

/// MLKEM-T07-compress-d: every x in [0, q) for each d, against Eq. (4.7).
fn t07(h: &Harness, imp: &Implementation) -> Result<OracleRun, HarnessError> {
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
    let run = h.run_impl("T07", imp, bytes, cases.len() * 2)?;
    Ok(finish(
        "MLKEM-T07-compress-d",
        OracleKind::ReferenceIntermediate,
        run,
        cases.len(),
        "Compress_d differs from FIPS 203 Eq. 4.7",
        |out| {
            let mut t = Tally::default();
            for (&(x, d), rec) in cases.iter().zip(out.chunks(2)) {
                let ok = u32::from(le16(rec)) == mlkem::compress(d, x);
                t.check(ok, || format!("d={d} x={x}"), FailureClass::WrongOutput);
            }
            (t, BTreeMap::new())
        },
    ))
}

/// MLDSA-T41-power2round: every r in [0, q), against Algorithm 35.
fn t41(h: &Harness, imp: &Implementation) -> Result<OracleRun, HarnessError> {
    let q = i32::try_from(mldsa::Q).expect("q fits");
    let bytes: Vec<u8> = (0..q).flat_map(i32::to_le_bytes).collect();
    let n = usize::try_from(q).expect("q fits");
    let run = h.run_impl("T41", imp, bytes, n * 8)?;
    Ok(finish(
        "MLDSA-T41-power2round",
        OracleKind::ReferenceIntermediate,
        run,
        n,
        "Power2Round differs from FIPS 204 Algorithm 35",
        |out| {
            let mut t = Tally::default();
            for (r, rec) in (0..q).zip(out.chunks(8)) {
                let (r1, r0) = mldsa::power2round(i64::from(r));
                let ok = i64::from(le32(rec)) == r1 && i64::from(le32(&rec[4..])) == r0;
                t.check(ok, || format!("r={r}"), FailureClass::WrongOutput);
            }
            (t, BTreeMap::new())
        },
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
fn t47(h: &Harness, imp: &Implementation) -> Result<OracleRun, HarnessError> {
    let cases = t47_inputs();
    let mut bytes = Vec::with_capacity(cases.len() * 8);
    for &(z, r) in &cases {
        bytes.extend_from_slice(&i32::try_from(z).expect("z < q").to_le_bytes());
        bytes.extend_from_slice(&i32::try_from(r).expect("r < q").to_le_bytes());
    }
    let run = h.run_impl("T47", imp, bytes, cases.len() * 4)?;
    Ok(finish(
        "MLDSA-T47-make-hint",
        OracleKind::ReferenceIntermediate,
        run,
        cases.len(),
        "MakeHint differs from FIPS 204 Algorithm 39",
        |out| {
            let mut t = Tally::default();
            for (&(z, r), rec) in cases.iter().zip(out.chunks(4)) {
                let want = i32::from(mldsa::make_hint(z, r, mldsa::GAMMA2_65));
                t.check(
                    le32(rec) == want,
                    || format!("z={z} r={r}"),
                    FailureClass::WrongOutput,
                );
            }
            (t, BTreeMap::new())
        },
    ))
}

// ---- MLKEM-T24: one record format, three oracles ----

/// What a T24 call must do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expect {
    /// Return 0 and, if given, write exactly this shared secret.
    Accept(Option<[u8; 32]>),
    /// Return nonzero. The class is the failure class when it does not.
    Reject(FailureClass),
}

/// One T24 test case.
#[derive(Clone, Debug)]
pub struct T24Case {
    /// Stable case ID.
    pub id: String,
    /// Report group.
    pub group: String,
    /// Ciphertext (any length).
    pub ct: Vec<u8>,
    /// Decapsulation key (any length).
    pub dk: Vec<u8>,
    /// What must happen.
    pub expect: Expect,
}

const T24: &str = "MLKEM-T24-decaps-implicit-rejection";
const ACVP_DETAIL: &str = "ACVP ML-KEM-768 decapsulation case failed";

fn run_t24(
    h: &Harness,
    imp: &Implementation,
    oracle: OracleKind,
    cases: &[T24Case],
    detail: &str,
) -> Result<OracleRun, HarnessError> {
    let mut bytes = Vec::new();
    for c in cases {
        for part in [&c.ct, &c.dk] {
            bytes.extend_from_slice(&u32::try_from(part.len()).expect("small").to_le_bytes());
            bytes.extend_from_slice(part);
        }
    }
    let run = h.run_impl("T24", imp, bytes, cases.len() * 36)?;
    Ok(finish(T24, oracle, run, cases.len(), detail, |out| {
        let mut t = Tally::default();
        let mut groups = BTreeMap::new();
        for (c, rec) in cases.iter().zip(out.chunks(36)) {
            let ret = le32(rec);
            let (ok, class) = match &c.expect {
                Expect::Accept(ss) => {
                    if ret != 0 {
                        (false, FailureClass::WrongReturnCode)
                    } else {
                        let ss_ok = ss.as_ref().is_none_or(|k| k[..] == rec[4..36]);
                        (ss_ok, FailureClass::WrongOutput)
                    }
                }
                Expect::Reject(class) => (ret != 0, *class),
            };
            t.check(ok, || c.id.clone(), class);
            *groups.entry(c.group.clone()).or_insert(0) += 1;
        }
        (t, groups)
    }))
}

/// ACVP: every ML-KEM-768 decapsulation case (shared secret and return
/// value 0) and every decapsulation-key-check case (0 for a valid key,
/// nonzero for a key failing the section 7.3 hash check; the ciphertext is
/// a zero ciphertext of the correct length). A missing hash check is a
/// task-contract failure (GAP-SPEC-001).
fn t24_acvp_cases(h: &Harness) -> Result<Vec<T24Case>, HarnessError> {
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
                Function::Decapsulation => {
                    let k =
                        tc.k.as_ref()
                            .map(|x| <[u8; 32]>::try_from(x.0.as_slice()))
                            .transpose()
                            .map_err(|_| HarnessError::Input("ACVP k is not 32 bytes".into()))?;
                    cases.push(T24Case {
                        id: format!("decapsulation tcId {}", tc.tc_id),
                        group: format!("decapsulation: {reason}"),
                        ct: tc.c.clone().map_or_else(Vec::new, |x| x.0),
                        dk,
                        expect: Expect::Accept(k),
                    });
                }
                Function::DecapsulationKeyCheck => cases.push(T24Case {
                    id: format!("decapsulationKeyCheck tcId {}", tc.tc_id),
                    group: format!("decapsulationKeyCheck: {reason}"),
                    ct: vec![0u8; ML_KEM_768.ct_len()],
                    dk,
                    expect: if tc.test_passed == Some(true) {
                        Expect::Accept(None)
                    } else {
                        Expect::Reject(FailureClass::TaskContract)
                    },
                }),
                Function::Encapsulation | Function::EncapsulationKeyCheck => {}
            }
        }
    }
    Ok(cases)
}

/// A key pair and a valid ciphertext from the spec-literal code, seeded.
struct KeyCase {
    dk: Vec<u8>,
    ct: Vec<u8>,
    k: [u8; 32],
}

/// Variable names follow FIPS 203 Algorithms 16 and 17 (d, z, m, ek, dk).
#[allow(clippy::many_single_char_names)]
fn key_cases(label: &str, n: usize) -> Vec<KeyCase> {
    let mut rng = SplitMix64::from_label(label);
    (0..n)
        .map(|_| {
            let (d, z, m) = (rng.bytes32(), rng.bytes32(), rng.bytes32());
            let (ek, dk) = mlkem_kem::keygen_internal(ML_KEM_768, &d, &z);
            let (k, ct) = mlkem_kem::encaps_internal(ML_KEM_768, &ek, &m);
            KeyCase { dk, ct, k }
        })
        .collect()
}

fn resized(v: &[u8], len: usize) -> Vec<u8> {
    let mut out = v.to_vec();
    out.resize(len, 0);
    out
}

fn flipped(v: &[u8], byte: usize, bit: u8) -> Vec<u8> {
    let mut out = v.to_vec();
    out[byte] ^= 1 << bit;
    out
}

/// Malformed-input cases for T24, in report order: FIPS 203 section 7.3
/// check 1 (ciphertext length) and check 2 (key length), required on every
/// call; check 3 (the dk hash, Eq. 7.2), required per call by the task
/// contract only (GAP-SPEC-001); then controls, which are valid inputs that
/// must be accepted, so that an implementation cannot pass by refusing
/// everything.
#[must_use]
pub fn t24_malformed_cases() -> Vec<T24Case> {
    let p = ML_KEM_768;
    let (ct_len, dk_len, k) = (p.ct_len(), p.dk_len(), p.k);
    let h_at = 768 * k + 32; // H(ek) occupies dk[h_at..h_at + 32]
    let ek_at = 384 * k; // ek occupies dk[ek_at..h_at]
    let z_at = h_at + 32;
    let mut cases = Vec::new();
    for (n, kc) in key_cases("MLKEM-T24/malformed-input", 2).iter().enumerate() {
        let mut push = |id: String, group: &str, ct: Vec<u8>, dk: Vec<u8>, expect| {
            cases.push(T24Case {
                id: format!("key {n}: {id}"),
                group: group.to_owned(),
                ct,
                dk,
                expect,
            });
        };
        let fips = Expect::Reject(FailureClass::WrongReturnCode);
        for len in [0, 1, ct_len - 1, ct_len + 1, 768, 1568, 2 * ct_len] {
            push(
                format!("ct_len {len}"),
                "check 1: ciphertext length",
                resized(&kc.ct, len),
                kc.dk.clone(),
                fips.clone(),
            );
        }
        for len in [0, 1, dk_len - 1, dk_len + 1, 1632, 3168, 2 * dk_len] {
            push(
                format!("dk_len {len}"),
                "check 2: decapsulation key length",
                kc.ct.clone(),
                resized(&kc.dk, len),
                fips.clone(),
            );
        }
        push(
            format!("ct_len {} and dk_len {}", ct_len - 1, dk_len - 1),
            "checks 1 and 2: both lengths",
            resized(&kc.ct, ct_len - 1),
            resized(&kc.dk, dk_len - 1),
            fips.clone(),
        );
        let contract = Expect::Reject(FailureClass::TaskContract);
        for (what, byte, bit) in [
            ("H(ek) first byte", h_at, 0),
            ("H(ek) last byte", h_at + 31, 7),
            ("ek first byte", ek_at, 0),
            ("ek last byte (rho)", h_at - 1, 7),
        ] {
            push(
                format!("bit flipped in {what}"),
                "check 3: decapsulation key hash (task contract)",
                kc.ct.clone(),
                flipped(&kc.dk, byte, bit),
                contract.clone(),
            );
        }
        push(
            "valid ciphertext and key".into(),
            "control: valid input",
            kc.ct.clone(),
            kc.dk.clone(),
            Expect::Accept(Some(kc.k)),
        );
        // z and dk_PKE are not covered by the hash: still valid input, so
        // the return value must be 0. With the dk_PKE bit flipped,
        // decryption no longer re-encrypts to c, so Algorithm 18 rejects
        // implicitly and K = J(z || c) (asserted in the tests below).
        push(
            "bit flipped in z".into(),
            "control: key parts the hash does not cover",
            kc.ct.clone(),
            flipped(&kc.dk, z_at, 0),
            Expect::Accept(Some(kc.k)),
        );
        let dk = flipped(&kc.dk, 0, 0);
        let want = mlkem_kem::decaps_internal(p, &dk, &kc.ct);
        push(
            "bit flipped in dk_PKE".into(),
            "control: key parts the hash does not cover",
            kc.ct.clone(),
            dk,
            Expect::Accept(Some(want)),
        );
    }
    cases
}

/// Implicit-rejection cases for T24: for four seeded key pairs, the valid
/// ciphertext (control) and eight modified ciphertexts of the correct
/// length, each of which must give return value 0 and K = J(z || c)
/// (FIPS 203 Algorithm 18, lines 8-11; section 6.3).
#[must_use]
pub fn t24_implicit_rejection_cases() -> Vec<T24Case> {
    let p = ML_KEM_768;
    let ct_len = p.ct_len();
    let z_at = p.dk_len() - 32;
    let mut rng = SplitMix64::from_label("MLKEM-T24/implicit-rejection/ciphertexts");
    let mut cases = Vec::new();
    for (n, kc) in key_cases("MLKEM-T24/implicit-rejection", 4)
        .iter()
        .enumerate()
    {
        cases.push(T24Case {
            id: format!("key {n}: valid ciphertext"),
            group: "control: valid ciphertext".into(),
            ct: kc.ct.clone(),
            dk: kc.dk.clone(),
            expect: Expect::Accept(Some(kc.k)),
        });
        let mut random = || -> Vec<u8> {
            (0..ct_len)
                .map(|_| u8::try_from(rng.below(256)).expect("byte"))
                .collect()
        };
        let modified: Vec<(String, &str, Vec<u8>)> = vec![
            (
                "bit 0 of byte 0".into(),
                "single bit flip",
                flipped(&kc.ct, 0, 0),
            ),
            (
                format!("bit 7 of byte {}", ct_len - 1),
                "single bit flip",
                flipped(&kc.ct, ct_len - 1, 7),
            ),
            (
                "bit 3 of byte 500 (u)".into(),
                "single bit flip",
                flipped(&kc.ct, 500, 3),
            ),
            (
                "bit 5 of byte 1000 (v)".into(),
                "single bit flip",
                flipped(&kc.ct, 1000, 5),
            ),
            ("all zero".into(), "constant", vec![0u8; ct_len]),
            ("all 0xff".into(), "constant", vec![0xffu8; ct_len]),
            ("random 0".into(), "random", random()),
            ("random 1".into(), "random", random()),
        ];
        for (what, group, ct) in modified {
            let mut zc = kc.dk[z_at..].to_vec();
            zc.extend_from_slice(&ct);
            let k_bar = mlkem_kem::j(&zc);
            cases.push(T24Case {
                id: format!("key {n}: {what}"),
                group: format!("modified ciphertext: {group}"),
                ct,
                dk: kc.dk.clone(),
                expect: Expect::Accept(Some(k_bar)),
            });
        }
    }
    cases
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_specs() -> Vec<TaskSpec> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tasks");
        otb_core::spec::load_all(&root).unwrap()
    }

    #[test]
    fn implemented_oracles_are_applicable_per_the_matrix() {
        let specs = repo_specs();
        assert_eq!(specs.len(), 5);
        for s in &specs {
            let have = implemented(s.id.as_str());
            assert!(!have.is_empty(), "{}", s.id);
            for k in have {
                assert!(
                    s.oracles.get(k).is_some_and(|a| a.applies),
                    "{} {} implemented but not applicable",
                    s.id,
                    k.name()
                );
            }
        }
    }

    #[test]
    fn task_flags() {
        assert_eq!(task_flag("MLKEM-T07-compress-d"), "T07");
        assert_eq!(task_flag("MLDSA-T41-power2round"), "T41");
    }

    /// The spec-literal checks (FIPS 203 section 7.3) agree with every
    /// malformed-input expectation, so the cases test what they claim.
    #[test]
    fn malformed_cases_agree_with_the_spec_literal_input_check() {
        let cases = t24_malformed_cases();
        assert_eq!(cases.len(), 2 * (7 + 7 + 1 + 4 + 3));
        for c in &cases {
            let valid = mlkem_kem::decaps_input_check(ML_KEM_768, &c.dk, &c.ct);
            match &c.expect {
                Expect::Accept(Some(k)) => {
                    assert!(valid, "{}", c.id);
                    assert_eq!(&mlkem_kem::decaps_internal(ML_KEM_768, &c.dk, &c.ct), k);
                }
                Expect::Accept(None) => unreachable!("{}", c.id),
                Expect::Reject(_) => assert!(!valid, "{}", c.id),
            }
        }
    }

    /// The dk_PKE control is an implicit rejection: its expected shared
    /// secret is J(z || c), not the valid K.
    #[test]
    fn dk_pke_control_is_an_implicit_rejection() {
        let z_at = ML_KEM_768.dk_len() - 32;
        let mut seen = 0;
        for c in t24_malformed_cases()
            .iter()
            .filter(|c| c.id.ends_with("bit flipped in dk_PKE"))
        {
            let mut zc = c.dk[z_at..].to_vec();
            zc.extend_from_slice(&c.ct);
            assert_eq!(
                c.expect,
                Expect::Accept(Some(mlkem_kem::j(&zc))),
                "{}",
                c.id
            );
            seen += 1;
        }
        assert_eq!(seen, 2);
    }

    /// Every modified ciphertext is implicitly rejected by the spec-literal
    /// Decaps_internal, i.e. the expected J(z || c) is what Algorithm 18
    /// returns, and differs from the valid shared secret.
    #[test]
    fn implicit_rejection_cases_agree_with_algorithm_18() {
        let cases = t24_implicit_rejection_cases();
        assert_eq!(cases.len(), 4 * 9);
        let mut valid_k = None;
        for c in &cases {
            assert!(mlkem_kem::decaps_input_check(ML_KEM_768, &c.dk, &c.ct));
            let Expect::Accept(Some(k)) = c.expect else {
                panic!("{}", c.id)
            };
            assert_eq!(
                mlkem_kem::decaps_internal(ML_KEM_768, &c.dk, &c.ct),
                k,
                "{}",
                c.id
            );
            if c.group.starts_with("control") {
                valid_k = Some(k);
            } else {
                assert_ne!(Some(k), valid_k, "{}", c.id);
            }
        }
    }

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
