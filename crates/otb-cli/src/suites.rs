//! `otb suite` and `otb mutants` (ADR-006).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use otb_harness::mutants::Manifest;
use otb_harness::reference::TASKS;
use otb_harness::suite::{self, MIN_ASSERTIONS};
use otb_harness::{Harness, ImplKind, Implementation};

type Res = Result<ExitCode, Box<dyn std::error::Error>>;

/// `otb suite --task --tests [--candidate]`.
pub fn suite(h: &Harness, task: &str, tests: &Path, candidate: Option<PathBuf>) -> Res {
    if candidate.is_some() && matches!(h.backend, otb_sandbox::Backend::UnshareWeaker) {
        return Err("candidate code runs only under the Docker backend (ADR-003)".into());
    }
    if !TASKS.contains(&task) {
        return Err(format!("unknown task {task:?}").into());
    }
    let imp = candidate
        .map(std::path::absolute)
        .transpose()?
        .map(|source| Implementation {
            label: "candidate".into(),
            kind: ImplKind::Candidate,
            source,
        });
    let r = suite::run_suite(h, task, &std::path::absolute(tests)?, imp.as_ref())?;
    let (killed, total) = r.score();
    println!("task:       {task}");
    println!("suite:      {}", tests.display());
    match (&r.rejected, r.compiles) {
        (Some(p), _) => println!("admission:  rejected ({p})"),
        (None, false) => println!("compiles:   no"),
        (None, true) => {
            println!(
                "reference:  {}",
                r.reference
                    .as_ref()
                    .map_or_else(|| "not run".into(), suite::RunResult::describe)
            );
            println!("mutants:    {killed} of {total} killed");
            if !r.survived.is_empty() {
                println!("survived:   {}", r.survived.join(", "));
            }
        }
    }
    let reasons = r.invalid_reasons();
    if reasons.is_empty() {
        println!(
            "validity:   valid (at least {MIN_ASSERTIONS} assertions, passes on the reference, kills a mutant)"
        );
    } else {
        println!("validity:   INVALID: {}", reasons.join("; "));
    }
    if let Some(c) = &r.candidate {
        println!("candidate:  {}", c.describe());
        if reasons.is_empty() {
            println!(
                "self-test:  {}",
                if c.passed() {
                    "the candidate passes its own valid test suite"
                } else {
                    "the candidate fails its own test suite"
                }
            );
        }
    }
    Ok(if reasons.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// `otb mutants classify` (write manifests) and `otb mutants verify`
/// (compare with the committed ones).
pub fn classify(h: &Harness, task: Option<&str>, verify: bool) -> Res {
    let tasks: Vec<&str> = match task {
        Some(t) if TASKS.contains(&t) => vec![t],
        Some(t) => return Err(format!("unknown task {t:?}").into()),
        None => TASKS.to_vec(),
    };
    let mut differ = false;
    for t in tasks {
        let fresh = suite::classify(h, t, |line| eprintln!("  {line}"))?;
        let killable = fresh.mutants.iter().filter(|m| m.killed_by_oracle).count();
        let path = h.repo.join(Manifest::path(t));
        if verify {
            let (committed, _) = suite::load_manifest(&h.repo, t)?;
            let same = committed.mutants.len() == fresh.mutants.len()
                && committed.mutants.iter().zip(&fresh.mutants).all(|(a, b)| {
                    a.killed_by_oracle == b.killed_by_oracle && a.evidence == b.evidence
                });
            differ |= !same;
            println!(
                "{t}: {} mutants, {killable} killed by a functional oracle: {}",
                fresh.mutants.len(),
                if same {
                    "matches the manifest"
                } else {
                    "DIFFERS from the manifest"
                }
            );
        } else {
            let mut text = serde_json::to_string_pretty(&fresh)?;
            text.push('\n');
            std::fs::write(&path, text)?;
            println!(
                "{t}: {} mutants, {killable} killed by a functional oracle; wrote {}",
                fresh.mutants.len(),
                Manifest::path(t)
            );
        }
    }
    Ok(if differ {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}
