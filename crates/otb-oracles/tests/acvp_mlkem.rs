//! The spec-literal ML-KEM agrees with every official ACVP ML-KEM sample
//! vector, for all three parameter sets. This is what lets it serve as an
//! independent reference (GAP-REF-001).
//!
//! Vectors: `third_party/acvp/`, pinned in `NOTICE.md`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use otb_oracles::speclit::mlkem_kem::{
    ML_KEM_512, ML_KEM_768, ML_KEM_1024, Params, decaps_input_check, decaps_internal, dk_check,
    ek_check, encaps_internal, keygen_internal,
};
use otb_vectors::mlkem::{EncapDecapFile, Function, KeyGenFile};
use otb_vectors::read_json;

fn acvp(dir: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../third_party/acvp")
        .join(dir)
        .join("internalProjection.json")
}

fn params(name: &str) -> Params {
    match name {
        "ML-KEM-512" => ML_KEM_512,
        "ML-KEM-768" => ML_KEM_768,
        "ML-KEM-1024" => ML_KEM_1024,
        other => panic!("unknown parameter set {other}"),
    }
}

#[test]
fn keygen_matches_every_acvp_case() {
    let file: KeyGenFile = read_json(&acvp("ML-KEM-keyGen-FIPS203")).unwrap();
    assert_eq!(
        (file.algorithm.as_str(), file.mode.as_str()),
        ("ML-KEM", "keyGen")
    );
    let mut counts = BTreeMap::new();
    for g in &file.test_groups {
        let p = params(&g.parameter_set);
        for t in &g.tests {
            let (ek, dk) = keygen_internal(p, &t.d.array().unwrap(), &t.z.array().unwrap());
            assert_eq!(ek, t.ek.0, "{} tcId {}", g.parameter_set, t.tc_id);
            assert_eq!(dk, t.dk.0, "{} tcId {}", g.parameter_set, t.tc_id);
            *counts.entry(g.parameter_set.clone()).or_insert(0) += 1;
        }
    }
    assert_eq!(counts.values().sum::<u32>(), 75, "{counts:?}");
}

#[test]
fn encap_decap_matches_every_acvp_case() {
    let file: EncapDecapFile = read_json(&acvp("ML-KEM-encapDecap-FIPS203")).unwrap();
    assert_eq!(
        (file.algorithm.as_str(), file.mode.as_str()),
        ("ML-KEM", "encapDecap")
    );
    let mut counts: BTreeMap<(String, Function, String), u32> = BTreeMap::new();
    for g in &file.test_groups {
        let p = params(&g.parameter_set);
        for t in &g.tests {
            let id = format!("{} {:?} tcId {}", g.parameter_set, g.function, t.tc_id);
            match g.function {
                Function::Encapsulation => {
                    let ek = &t.ek.as_ref().unwrap().0;
                    let (k, c) = encaps_internal(p, ek, &t.m.as_ref().unwrap().array().unwrap());
                    assert_eq!(c, t.c.as_ref().unwrap().0, "{id}");
                    assert_eq!(k.to_vec(), t.k.as_ref().unwrap().0, "{id}");
                }
                Function::Decapsulation => {
                    let dk = &t.dk.as_ref().unwrap().0;
                    let c = &t.c.as_ref().unwrap().0;
                    assert!(decaps_input_check(p, dk, c), "{id}: input check");
                    assert_eq!(
                        decaps_internal(p, dk, c).to_vec(),
                        t.k.as_ref().unwrap().0,
                        "{id}"
                    );
                }
                Function::DecapsulationKeyCheck => {
                    let dk = &t.dk.as_ref().unwrap().0;
                    assert_eq!(dk_check(p, dk), t.test_passed.unwrap(), "{id}");
                }
                Function::EncapsulationKeyCheck => {
                    let ek = &t.ek.as_ref().unwrap().0;
                    assert_eq!(ek_check(p, ek), t.test_passed.unwrap(), "{id}");
                }
            }
            let reason = t.reason.clone().unwrap_or_else(|| "-".into());
            *counts
                .entry((g.parameter_set.clone(), g.function, reason))
                .or_insert(0) += 1;
        }
    }
    // Every group of the file was exercised; print the counts for the record.
    for ((ps, f, r), n) in &counts {
        println!("{ps:12} {f:?} [{r}]: {n}");
    }
    assert_eq!(counts.values().sum::<u32>(), 165, "{counts:?}");
}
