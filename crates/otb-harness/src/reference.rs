//! Reference checks: run each pilot task's C reference through every
//! implemented oracle (see [`crate::oracles`]), plus the ML-DSA-65 ACVP
//! check of the reference library itself. A failure here means the
//! reference, the adapter, the driver or the spec-literal code is wrong,
//! and must be investigated (GAP-REF-001); expected values are never edited
//! to match.

use std::collections::BTreeMap;

use otb_core::{FailureClass, OracleKind};

use crate::oracles::{self, OracleRun};
use crate::{Harness, HarnessError, Implementation, Library, Tally};

/// The pilot tasks, in report order.
pub const TASKS: [&str; 5] = [
    "MLDSA-T41-power2round",
    "MLDSA-T47-make-hint",
    "MLKEM-T01-ntt",
    "MLKEM-T07-compress-d",
    "MLKEM-T24-decaps-implicit-rejection",
];

fn le32(b: &[u8]) -> i32 {
    i32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

/// What one ML-DSA ACVP record should produce.
enum DsaExpect {
    Key { pk: Vec<u8>, sk: Vec<u8> },
    Sig(Vec<u8>),
    Verify(bool),
}

fn put_var(buf: &mut Vec<u8>, bytes: &[u8]) {
    buf.extend_from_slice(&u32::try_from(bytes.len()).expect("small").to_le_bytes());
    buf.extend_from_slice(bytes);
}

/// ML-DSA-65 reference build against the ACVP keyGen, sigGen and sigVer
/// vectors for the internal, external-mu and external pure interfaces.
/// Pre-hash groups are counted as not covered (GAP-ACVP-003).
///
/// One linear pass: encode every record, run once, decode in the same
/// order. Kept in one function so the record format is read in one place.
#[allow(clippy::too_many_lines)]
pub fn mldsa65_acvp(h: &Harness) -> Result<OracleRun, HarnessError> {
    use otb_vectors::mldsa::{KeyGenFile, SigFile};
    let dir = h.repo.join("third_party/acvp");
    let kg: KeyGenFile =
        otb_vectors::read_json(&dir.join("ML-DSA-keyGen-FIPS204/internalProjection.json"))?;
    let sg: SigFile =
        otb_vectors::read_json(&dir.join("ML-DSA-sigGen-FIPS204/internalProjection.json"))?;
    let sv: SigFile =
        otb_vectors::read_json(&dir.join("ML-DSA-sigVer-FIPS204/internalProjection.json"))?;

    let mut input = Vec::new();
    let mut expect: Vec<(String, String, DsaExpect)> = Vec::new();
    let mut groups: BTreeMap<String, u64> = BTreeMap::new();

    for g in kg
        .test_groups
        .iter()
        .filter(|g| g.parameter_set == "ML-DSA-65")
    {
        for tc in &g.tests {
            input.push(b'K');
            input.extend_from_slice(&tc.seed.0);
            expect.push((
                format!("keyGen tcId {}", tc.tc_id),
                "keyGen".into(),
                DsaExpect::Key {
                    pk: tc.pk.0.clone(),
                    sk: tc.sk.0.clone(),
                },
            ));
        }
    }
    let iface = |interface: &str, pre_hash: &str, ext_mu: bool| -> Option<u8> {
        match (interface, pre_hash, ext_mu) {
            ("internal", _, false) => Some(0),
            ("internal", _, true) => Some(1),
            ("external", "pure", false) => Some(2),
            _ => None,
        }
    };
    let empty = Vec::new();
    for (mode, file) in [("sigGen", &sg), ("sigVer", &sv)] {
        for g in file
            .test_groups
            .iter()
            .filter(|g| g.parameter_set == "ML-DSA-65")
        {
            let label = format!(
                "{mode}: {} {}{}",
                g.signature_interface,
                g.pre_hash,
                if g.external_mu { " externalMu" } else { "" }
            );
            let Some(code) = iface(&g.signature_interface, &g.pre_hash, g.external_mu) else {
                *groups
                    .entry(format!("{label} (not covered, GAP-ACVP-003)"))
                    .or_insert(0) += u64::try_from(g.tests.len()).expect("small");
                continue;
            };
            for tc in &g.tests {
                let msg = if g.external_mu { &tc.mu } else { &tc.message };
                let msg = msg.as_ref().map_or(&empty, |x| &x.0);
                let ctx = tc.context.as_ref().map_or(&empty, |x| &x.0);
                let id = format!("{mode} tgId {} tcId {}", g.tg_id, tc.tc_id);
                if mode == "sigGen" {
                    input.push(b'S');
                    input.push(code);
                    let rnd = tc.rnd.as_ref().map_or(vec![0u8; 32], |x| x.0.clone());
                    input.extend_from_slice(&rnd);
                    input.extend_from_slice(&tc.sk.as_ref().expect("sigGen has sk").0);
                    put_var(&mut input, ctx);
                    put_var(&mut input, msg);
                    let sig = tc
                        .signature
                        .as_ref()
                        .expect("sigGen has signature")
                        .0
                        .clone();
                    expect.push((id, label.clone(), DsaExpect::Sig(sig)));
                } else {
                    input.push(b'V');
                    input.push(code);
                    input.extend_from_slice(&tc.pk.as_ref().expect("sigVer has pk").0);
                    put_var(
                        &mut input,
                        &tc.signature.as_ref().expect("sigVer has signature").0,
                    );
                    put_var(&mut input, ctx);
                    put_var(&mut input, msg);
                    let pass = tc.test_passed.expect("sigVer has testPassed");
                    expect.push((id, label.clone(), DsaExpect::Verify(pass)));
                }
            }
        }
    }
    let out_len: usize = expect
        .iter()
        .map(|(_, _, e)| match e {
            DsaExpect::Key { .. } => 4 + 1952 + 4032,
            DsaExpect::Sig(_) => 4 + 3309,
            DsaExpect::Verify(_) => 4,
        })
        .sum();
    let out = h.run_program(
        "ML-DSA-65 ACVP",
        Library::MlDsa,
        &["acvp_mldsa.c"],
        "",
        input,
        out_len,
    )?;
    let mut t = Tally::default();
    let mut pos = 0;
    for (id, label, e) in &expect {
        let ret = le32(&out[pos..]);
        pos += 4;
        let ok = match e {
            DsaExpect::Key { pk, sk } => {
                let ok = ret == 0
                    && out[pos..pos + 1952] == pk[..]
                    && out[pos + 1952..pos + 5984] == sk[..];
                pos += 5984;
                ok
            }
            DsaExpect::Sig(sig) => {
                let ok = ret == 0 && out[pos..pos + 3309] == sig[..];
                pos += 3309;
                ok
            }
            DsaExpect::Verify(pass) => (ret == 0) == *pass,
        };
        t.check(ok, || id.clone(), FailureClass::WrongOutput);
        *groups.entry(label.clone()).or_insert(0) += 1;
    }
    Ok(OracleRun {
        task: "ML-DSA-65 reference build".into(),
        oracle: OracleKind::Acvp,
        outcome: t.outcome("ACVP ML-DSA-65 case failed"),
        cases: t.total,
        groups,
    })
}

/// Every implemented oracle on every task's reference, in task order,
/// followed by the ML-DSA-65 reference-build check.
#[must_use]
pub fn run_all(h: &Harness) -> Vec<Result<OracleRun, HarnessError>> {
    let mut out = Vec::new();
    for task in TASKS {
        let imp = Implementation::reference(task);
        for &oracle in oracles::implemented(task) {
            out.push(oracles::run_oracle(h, task, oracle, &imp));
        }
    }
    out.push(mldsa65_acvp(h));
    out
}
