//! Runs a model-written test suite (ADR-006) and decides whether it counts.
//!
//! A suite is one C file using the interface in
//! `c/harness/test-abi/otb_test.h`. It is admitted like a candidate (plus
//! `otb_test.h`; see `admission::check_test_file`), then built and run in
//! the sandbox:
//!
//! 1. against the task's mutation base (the verified reference) and every
//!    mutant in the manifest that a functional oracle kills, in one job;
//! 2. against the candidate, in a second job that stages no library.
//!
//! A suite is **valid** if it compiles, passes on the reference with at
//! least [`MIN_ASSERTIONS`] assertions executed, and kills at least one
//! mutant (the application's rules, plus "passes on the reference").
//! A candidate **passes its own tests** if a valid suite passes on it.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use otb_core::OracleKind;
use otb_sandbox::{Exit, Limits};

use crate::mutants::{self, Classified, Manifest};
use crate::oracles;
use crate::{Harness, HarnessError, ImplKind, Implementation, Library, admission};

/// Fewest assertions a suite must execute on the reference.
pub const MIN_ASSERTIONS: u64 = 5;

/// Time limit for one run of a suite against one implementation.
pub const RUN_LIMIT_SECS: u64 = 10;

/// The functional oracles, which decide whether a mutant is killable.
pub const FUNCTIONAL: [OracleKind; 4] = [
    OracleKind::ReferenceIntermediate,
    OracleKind::Acvp,
    OracleKind::MalformedInput,
    OracleKind::ImplicitRejection,
];

/// How one run of a suite against one implementation ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunResult {
    /// Exit status 0 and no failed assertion.
    Passed {
        /// Assertions executed.
        assertions: u64,
    },
    /// At least one assertion failed, or the suite exited nonzero.
    Failed {
        /// Assertions executed.
        assertions: u64,
        /// Assertions that failed.
        failures: u64,
        /// Exit status.
        exit: i32,
    },
    /// The process ended without the harness's result line (signal, abort,
    /// `_exit`).
    Crashed {
        /// Shell exit status (128 + signal for a signal).
        exit: i32,
    },
    /// Exceeded [`RUN_LIMIT_SECS`].
    TimedOut,
    /// The suite and the implementation did not build together.
    BuildFailed,
    /// The implementation (a candidate) broke the admission policy.
    Rejected {
        /// Policy name.
        policy: String,
    },
}

impl RunResult {
    /// True only for `Passed`.
    #[must_use]
    pub fn passed(&self) -> bool {
        matches!(self, Self::Passed { .. })
    }

    /// Short text for reports.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Passed { assertions } => format!("passes ({assertions} assertions)"),
            Self::Failed {
                assertions,
                failures,
                exit,
            } => format!("fails ({failures} of {assertions} assertions failed, exit {exit})"),
            Self::Crashed { exit } => format!("crashes (exit {exit}, no result)"),
            Self::TimedOut => format!("times out (over {RUN_LIMIT_SECS} s)"),
            Self::BuildFailed => "does not build".into(),
            Self::Rejected { policy } => format!("rejected by admission ({policy})"),
        }
    }
}

/// Parses one `OTB-RUN` line written by the suite script.
fn parse_run(rest: &str) -> Option<(String, RunResult)> {
    let mut words = rest.split_whitespace();
    let name = words.next()?.to_owned();
    let status = words.next()?;
    if status == "build-failed" {
        return Some((name, RunResult::BuildFailed));
    }
    let rc: i32 = status.strip_prefix("rc=")?.parse().ok()?;
    let mut counts = BTreeMap::new();
    if words.next() == Some("OTB-TEST-RESULT") {
        for w in words {
            if let Some((k, v)) = w.split_once('=') {
                counts.insert(k.to_owned(), v.parse::<u64>().ok()?);
            }
        }
    }
    let result = match (rc, counts.get("assertions"), counts.get("failures")) {
        (137, _, _) => RunResult::TimedOut,
        (0, Some(&a), Some(&0)) => RunResult::Passed { assertions: a },
        (exit, Some(&assertions), Some(&failures)) => RunResult::Failed {
            assertions,
            failures,
            exit,
        },
        (exit, _, _) => RunResult::Crashed { exit },
    };
    Some((name, result))
}

fn suite_script(lib_include: &str) -> String {
    format!(
        "cd \"$OTB_WORK\"\n\
         if ! cc -std=c11 -O2 -c -I\"$OTB_IN/harness\" -I\"$OTB_IN/harness/test-abi\" \
           \"$OTB_IN/suite/tests.c\" -o tests.o; then echo OTB-SUITE-NOCOMPILE; exit 0; fi\n\
         cc -std=c11 -O2 -c -I\"$OTB_IN/harness/test-abi\" \
           \"$OTB_IN/harness/test-abi/otb_test.c\" -o otb_test.o || exit 3\n\
         for f in \"$OTB_IN\"/impls/*.c; do\n\
           n=$(basename \"$f\" .c)\n\
           if cc -std=c11 -O2 -I\"$OTB_IN/harness\"{lib_include} \"$f\" tests.o otb_test.o \
              -o \"t_$n\" 2>/dev/null; then\n\
             timeout -s KILL {RUN_LIMIT_SECS} \"./t_$n\" > \"out_$n\" 2>/dev/null; rc=$?\n\
             echo \"OTB-RUN $n rc=$rc $(grep '^OTB-TEST-RESULT' \"out_$n\" | tail -n 1)\"\n\
             rm -f \"out_$n\" \"t_$n\"\n\
           else echo \"OTB-RUN $n build-failed\"; fi\n\
         done\n"
    )
}

/// Runs the suite script on `impls` (name, source) and returns each run's
/// result, or `None` if the suite itself does not compile.
fn run_batch(
    h: &Harness,
    tests: &[u8],
    impls: Vec<(String, Vec<u8>)>,
    lib: Option<Library>,
) -> Result<Option<BTreeMap<String, RunResult>>, HarnessError> {
    let n = impls.len();
    let mut files = vec![("suite/tests.c".to_owned(), tests.to_vec())];
    files.extend(
        impls
            .into_iter()
            .map(|(name, src)| (format!("impls/{name}.c"), src)),
    );
    let lib_include = if lib.is_some() {
        " -I\"$OTB_IN/lib\""
    } else {
        ""
    };
    let limits = Limits {
        // Compilation plus at most RUN_LIMIT_SECS per implementation.
        wall: Duration::from_secs(60 + (RUN_LIMIT_SECS + 10) * u64::try_from(n).expect("small")),
        ..Limits::default()
    };
    let out = h.run_files_script(&files, lib, &suite_script(lib_include), limits)?;
    let text = String::from_utf8_lossy(&out.stdout);
    if out.exit != Exit::Code(0) || out.truncated {
        return Err(HarnessError::Program {
            what: "test-suite run".into(),
            exit: out.exit,
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        });
    }
    if text.lines().any(|l| l == "OTB-SUITE-NOCOMPILE") {
        return Ok(None);
    }
    let results: BTreeMap<String, RunResult> = text
        .lines()
        .filter_map(|l| l.strip_prefix("OTB-RUN "))
        .filter_map(parse_run)
        .collect();
    if results.len() != n {
        return Err(HarnessError::Input(format!(
            "test-suite run: expected {n} results, got {}",
            results.len()
        )));
    }
    Ok(Some(results))
}

/// Reads a task's base file and manifest, and binds them.
///
/// # Errors
///
/// A missing or stale manifest.
pub fn load_manifest(repo: &Path, task: &str) -> Result<(Manifest, String), HarnessError> {
    let base = std::fs::read_to_string(repo.join(Manifest::base_path(task)))?;
    let text = std::fs::read_to_string(repo.join(Manifest::path(task))).map_err(|e| {
        HarnessError::Input(format!(
            "{}: {e}; run `otb mutants classify`",
            Manifest::path(task)
        ))
    })?;
    let mut m: Manifest = serde_json::from_str(&text)
        .map_err(|e| HarnessError::Input(format!("{}: {e}", Manifest::path(task))))?;
    m.bind(&base).map_err(HarnessError::Input)?;
    Ok((m, base))
}

/// Everything known about one suite.
#[derive(Clone, Debug)]
pub struct SuiteReport {
    /// Task.
    pub task: String,
    /// Set if the test file broke the admission policy (nothing was run).
    pub rejected: Option<String>,
    /// False if the test file did not compile.
    pub compiles: bool,
    /// The run against the reference (mutation base).
    pub reference: Option<RunResult>,
    /// Killable mutants killed by the suite (IDs).
    pub killed: Vec<String>,
    /// Killable mutants the suite let survive (IDs).
    pub survived: Vec<String>,
    /// The run against the candidate, if one was given.
    pub candidate: Option<RunResult>,
}

impl SuiteReport {
    /// Why the suite does not count; empty if it is valid.
    #[must_use]
    pub fn invalid_reasons(&self) -> Vec<String> {
        let mut r = Vec::new();
        if let Some(p) = &self.rejected {
            r.push(format!("rejected by admission ({p})"));
            return r;
        }
        if !self.compiles {
            r.push("does not compile".into());
            return r;
        }
        match &self.reference {
            Some(RunResult::Passed { assertions }) if *assertions < MIN_ASSERTIONS => {
                r.push(format!(
                    "{assertions} assertions on the reference, fewer than {MIN_ASSERTIONS}"
                ));
            }
            Some(RunResult::Passed { .. }) => {}
            Some(other) => r.push(format!("on the reference it {}", other.describe())),
            None => r.push("not run on the reference".into()),
        }
        if self.killed.is_empty() {
            r.push("kills no mutant".into());
        }
        r
    }

    /// True if the suite counts as a test suite.
    #[must_use]
    pub fn valid(&self) -> bool {
        self.invalid_reasons().is_empty()
    }

    /// Mutation score: killed / killable.
    #[must_use]
    pub fn score(&self) -> (usize, usize) {
        (self.killed.len(), self.killed.len() + self.survived.len())
    }
}

/// Runs the suite in `tests` for `task` against the reference, the
/// killable mutants and, if given, `candidate`.
///
/// # Errors
///
/// A harness failure or a missing manifest.
pub fn run_suite(
    h: &Harness,
    task: &str,
    tests: &Path,
    candidate: Option<&Implementation>,
) -> Result<SuiteReport, HarnessError> {
    let tests = std::fs::read(tests)?;
    let mut report = SuiteReport {
        task: task.to_owned(),
        rejected: None,
        compiles: true,
        reference: None,
        killed: Vec::new(),
        survived: Vec::new(),
        candidate: None,
    };
    if let Err(r) = admission::check_test_file(&tests) {
        report.rejected = Some(r.policy);
        return Ok(report);
    }
    let (manifest, base) = load_manifest(&h.repo, task)?;
    let lib = (task == "MLKEM-T24-decaps-implicit-rejection").then_some(Library::MlKem);
    let mut impls = vec![("reference".to_owned(), base.clone().into_bytes())];
    impls.extend(
        manifest
            .killable()
            .map(|m| (m.id.clone(), m.apply(&base).into_bytes())),
    );
    let Some(mut results) = run_batch(h, &tests, impls, lib)? else {
        report.compiles = false;
        return Ok(report);
    };
    report.reference = results.remove("reference");
    for m in manifest.killable() {
        let passed = results.get(&m.id).is_some_and(RunResult::passed);
        if passed {
            report.survived.push(m.id.clone());
        } else {
            report.killed.push(m.id.clone());
        }
    }
    if let Some(c) = candidate {
        let src = std::fs::read(h.path(&c.source))?;
        report.candidate = Some(
            if c.kind.needs_admission()
                && let Err(r) = admission::check(&src)
            {
                RunResult::Rejected { policy: r.policy }
            } else {
                match run_batch(
                    h,
                    &tests,
                    vec![("candidate".to_owned(), src)],
                    c.kind.library(),
                )? {
                    Some(mut r) => r.remove("candidate").unwrap_or(RunResult::BuildFailed),
                    None => RunResult::BuildFailed,
                }
            },
        );
    }
    Ok(report)
}

/// Classifies every mutant of `task` with the functional oracles: first
/// checks that the base itself passes them all, then runs them on each
/// mutant. The result is the manifest to commit.
///
/// # Errors
///
/// A harness failure, or a base that fails a functional oracle.
pub fn classify(
    h: &Harness,
    task: &str,
    mut progress: impl FnMut(&str),
) -> Result<Manifest, HarnessError> {
    let base_rel = Manifest::base_path(task);
    let base = std::fs::read_to_string(h.repo.join(&base_rel))?;
    let all = mutants::generate(&base).map_err(|e| HarnessError::Input(e.to_string()))?;
    let lib = (task == "MLKEM-T24-decaps-implicit-rejection").then_some(Library::MlKem);
    let oracles: Vec<OracleKind> = oracles::implemented(task)
        .iter()
        .copied()
        .filter(|k| FUNCTIONAL.contains(k))
        .collect();
    let run_all = |path: &Path| -> Result<Option<String>, HarnessError> {
        let imp = Implementation {
            label: "mutant".into(),
            kind: ImplKind::SeededFault(lib),
            source: path.to_path_buf(),
        };
        for &k in &oracles {
            let r = oracles::run_oracle(h, task, k, &imp)?;
            if let otb_core::Outcome::Fail { evidence } = r.outcome {
                return Ok(Some(format!(
                    "{} {}",
                    k.name(),
                    crate::selftest::class_name(evidence.class)
                )));
            }
        }
        Ok(None)
    };
    if let Some(e) = run_all(&h.repo.join(&base_rel))? {
        return Err(HarnessError::Input(format!(
            "{task}: the mutation base fails a functional oracle ({e})"
        )));
    }
    let tmp = std::env::temp_dir().join(format!("otb-mutant-{}.c", std::process::id()));
    let mut out = Vec::new();
    for m in all {
        std::fs::write(&tmp, m.apply(&base))?;
        let evidence = run_all(&tmp)?;
        progress(&format!(
            "{task} {} line {} '{}' -> '{}': {}",
            m.id,
            m.line,
            m.from,
            m.to,
            evidence
                .as_deref()
                .unwrap_or("survives every functional oracle")
        ));
        out.push(Classified {
            mutant: m,
            killed_by_oracle: evidence.is_some(),
            evidence,
        });
    }
    let _ = std::fs::remove_file(&tmp);
    Ok(Manifest {
        task: task.to_owned(),
        base: base_rel,
        base_sha256: mutants::base_digest(&base),
        mutants: out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_run_lines() {
        let cases = [
            (
                "reference rc=0 OTB-TEST-RESULT assertions=7 failures=0",
                RunResult::Passed { assertions: 7 },
            ),
            (
                "M003 rc=0 OTB-TEST-RESULT assertions=7 failures=2",
                RunResult::Failed {
                    assertions: 7,
                    failures: 2,
                    exit: 0,
                },
            ),
            (
                "M004 rc=1 OTB-TEST-RESULT assertions=7 failures=0",
                RunResult::Failed {
                    assertions: 7,
                    failures: 0,
                    exit: 1,
                },
            ),
            ("M005 rc=139 ", RunResult::Crashed { exit: 139 }),
            ("M006 rc=137 ", RunResult::TimedOut),
            ("M007 build-failed", RunResult::BuildFailed),
        ];
        for (line, want) in cases {
            let name = line.split_whitespace().next().unwrap().to_owned();
            assert_eq!(parse_run(line), Some((name, want)), "{line}");
        }
        assert_eq!(parse_run("x"), None);
    }

    fn report(reference: RunResult, killed: usize) -> SuiteReport {
        SuiteReport {
            task: "T".into(),
            rejected: None,
            compiles: true,
            reference: Some(reference),
            killed: (0..killed).map(|i| format!("M{i:03}")).collect(),
            survived: vec!["M900".into()],
            candidate: None,
        }
    }

    #[test]
    fn validity_rules() {
        assert!(report(RunResult::Passed { assertions: 5 }, 1).valid());
        let few = report(RunResult::Passed { assertions: 4 }, 3);
        assert_eq!(
            few.invalid_reasons(),
            ["4 assertions on the reference, fewer than 5"]
        );
        let weak = report(RunResult::Passed { assertions: 9 }, 0);
        assert_eq!(weak.invalid_reasons(), ["kills no mutant"]);
        let wrong = report(
            RunResult::Failed {
                assertions: 9,
                failures: 1,
                exit: 0,
            },
            2,
        );
        assert!(!wrong.valid());
        assert_eq!(wrong.score(), (2, 3));
        let mut rejected = report(RunResult::Passed { assertions: 9 }, 2);
        rejected.rejected = Some("header:stdio.h".into());
        assert_eq!(rejected.invalid_reasons().len(), 1);
    }

    /// Every committed manifest matches its base file and the generator,
    /// and every task has at least one mutant an oracle kills.
    #[test]
    fn committed_manifests_match_the_generator() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for task in crate::reference::TASKS {
            let (m, _) = load_manifest(&repo, task).unwrap();
            assert_eq!(m.task, task);
            assert!(m.killable().count() >= 1, "{task}");
        }
    }

    #[test]
    fn script_compiles_the_suite_once_and_limits_each_run() {
        let s = suite_script(" -I\"$OTB_IN/lib\"");
        assert_eq!(s.matches("suite/tests.c").count(), 1);
        assert!(s.contains("timeout -s KILL 10"));
        assert!(s.contains("-I\"$OTB_IN/lib\""));
    }
}
