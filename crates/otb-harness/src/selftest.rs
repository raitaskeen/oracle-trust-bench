//! `otb selftest`: run every seeded fault in `c/seeded-faults/` through the
//! oracles and assert the documented outcome of each one.
//!
//! A seeded fault is a hand-written faulty implementation of one pilot task.
//! Its header comment says what it breaks and, in machine-readable lines,
//! what every applicable oracle must conclude:
//!
//! ```text
//!  * otb-fault: 5
//!  * otb-task: MLKEM-T07-compress-d
//!  * otb-lib: none
//!  * otb-expect: reference-intermediate fail wrong-output
//! ```
//!
//! `otb-lib` is `none`, `mlkem` or `mldsa`. A fault with no library is
//! admitted and built exactly like a candidate. `otb-expect` is
//! `<oracle> pass` or `<oracle> fail [<failure-class>]`. Every applicable
//! oracle that has code must have an expectation, so a new oracle cannot
//! silently skip the faults; expectations for applicable oracles without
//! code yet are kept and reported as pending. The header is the only place
//! the expectations live.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use otb_core::spec::TaskSpec;
use otb_core::{FailureClass, OracleKind, Outcome};

use crate::oracles::{self, OracleRun};
use crate::{Harness, HarnessError, ImplKind, Implementation, Library};

/// What an oracle must conclude about a fault.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expectation {
    /// True if the oracle must fail the fault.
    pub fail: bool,
    /// If given, the failure class the oracle must report.
    pub class: Option<FailureClass>,
}

impl Expectation {
    /// True if `outcome` matches.
    #[must_use]
    pub fn holds(&self, outcome: &Outcome) -> bool {
        match outcome {
            Outcome::Pass => !self.fail,
            Outcome::Fail { evidence } => {
                self.fail && self.class.is_none_or(|c| c == evidence.class)
            }
            _ => false,
        }
    }

    fn render(&self) -> String {
        match (self.fail, self.class) {
            (false, _) => "pass".into(),
            (true, None) => "fail".into(),
            (true, Some(c)) => format!("fail ({})", class_name(c)),
        }
    }
}

/// The kebab-case name of a failure class, as in verdict JSON.
#[must_use]
pub fn class_name(c: FailureClass) -> String {
    serde_json::to_value(c)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn parse_class(s: &str) -> Option<FailureClass> {
    serde_json::from_value(serde_json::Value::String(s.to_owned())).ok()
}

/// One seeded fault, read from its header.
#[derive(Clone, Debug)]
pub struct Fault {
    /// Fault number (1-8, as in the build plan).
    pub number: u32,
    /// The C file, relative to the repository root.
    pub file: PathBuf,
    /// The task it implements.
    pub task: String,
    /// The library it builds on, if any.
    pub lib: Option<Library>,
    /// Expected outcome per oracle.
    pub expect: BTreeMap<OracleKind, Expectation>,
}

impl Fault {
    /// The implementation to check.
    #[must_use]
    pub fn implementation(&self) -> Implementation {
        Implementation {
            label: format!("fault {}", self.number),
            kind: ImplKind::SeededFault(self.lib),
            source: self.file.clone(),
        }
    }
}

/// Parses the `otb-*` header lines of one fault file.
///
/// # Errors
///
/// A missing, repeated or malformed header line.
pub fn parse_fault(file: &Path, text: &str) -> Result<Fault, String> {
    let at = |msg: String| format!("{}: {msg}", file.display());
    let mut number = None;
    let mut task = None;
    let mut lib = None;
    let mut expect = BTreeMap::new();
    for line in text.lines() {
        let Some(rest) = line.trim_start().strip_prefix('*') else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix("otb-") else {
            continue;
        };
        let (key, value) = rest
            .split_once(':')
            .ok_or_else(|| at(format!("malformed header line {line:?}")))?;
        let value = value.trim();
        let twice = || at(format!("otb-{key} given twice"));
        match key {
            "fault" => {
                if number.is_some() {
                    return Err(twice());
                }
                number = Some(
                    value
                        .parse::<u32>()
                        .map_err(|_| at(format!("bad fault number {value:?}")))?,
                );
            }
            "task" => {
                if task.is_some() {
                    return Err(twice());
                }
                task = Some(value.to_owned());
            }
            "lib" => {
                if lib.is_some() {
                    return Err(twice());
                }
                lib = Some(match value {
                    "none" => None,
                    "mlkem" => Some(Library::MlKem),
                    "mldsa" => Some(Library::MlDsa),
                    _ => return Err(at(format!("bad otb-lib {value:?}"))),
                });
            }
            "expect" => {
                let words: Vec<&str> = value.split_whitespace().collect();
                let (oracle, e) = match words.as_slice() {
                    [o, "pass"] => (
                        o,
                        Expectation {
                            fail: false,
                            class: None,
                        },
                    ),
                    [o, "fail"] => (
                        o,
                        Expectation {
                            fail: true,
                            class: None,
                        },
                    ),
                    [o, "fail", c] => (
                        o,
                        Expectation {
                            fail: true,
                            class: Some(
                                parse_class(c)
                                    .ok_or_else(|| at(format!("bad failure class {c:?}")))?,
                            ),
                        },
                    ),
                    _ => return Err(at(format!("bad otb-expect {value:?}"))),
                };
                let kind = OracleKind::from_name(oracle)
                    .ok_or_else(|| at(format!("unknown oracle {oracle:?}")))?;
                if expect.insert(kind, e).is_some() {
                    return Err(at(format!("two expectations for {oracle}")));
                }
            }
            _ => return Err(at(format!("unknown header key otb-{key}"))),
        }
    }
    Ok(Fault {
        number: number.ok_or_else(|| at("missing otb-fault".into()))?,
        task: task.ok_or_else(|| at("missing otb-task".into()))?,
        lib: lib.ok_or_else(|| at("missing otb-lib".into()))?,
        file: file.to_path_buf(),
        expect,
    })
}

/// Reads every `c/seeded-faults/*.c`, sorted by fault number.
///
/// # Errors
///
/// An unreadable or malformed fault file, or two files with one number.
pub fn load_faults(repo: &Path) -> Result<Vec<Fault>, HarnessError> {
    let dir = repo.join("c/seeded-faults");
    let mut faults = Vec::new();
    for e in std::fs::read_dir(&dir)? {
        let path = e?.path();
        if path.extension().is_some_and(|x| x == "c") {
            let text = std::fs::read_to_string(&path)?;
            let rel = path.strip_prefix(repo).unwrap_or(&path).to_path_buf();
            faults.push(parse_fault(&rel, &text).map_err(HarnessError::Input)?);
        }
    }
    faults.sort_by_key(|f| f.number);
    if let Some(w) = faults.windows(2).find(|w| w[0].number == w[1].number) {
        return Err(HarnessError::Input(format!(
            "two seeded faults numbered {}",
            w[0].number
        )));
    }
    Ok(faults)
}

/// One line of the selftest table.
#[derive(Debug)]
pub struct Row {
    /// Fault number.
    pub fault: u32,
    /// Task.
    pub task: String,
    /// Oracle.
    pub oracle: OracleKind,
    /// What the header says must happen.
    pub expected: Expectation,
    /// What happened: an oracle run, or `None` if the oracle has no code
    /// yet (pending).
    pub observed: Option<Result<OracleRun, String>>,
}

impl Row {
    /// True if the oracle ran and the expectation holds.
    #[must_use]
    pub fn holds(&self) -> bool {
        matches!(&self.observed, Some(Ok(r)) if self.expected.holds(&r.outcome))
    }

    /// True if the oracle has no code yet.
    #[must_use]
    pub fn pending(&self) -> bool {
        self.observed.is_none()
    }

    /// The expected outcome, for the table.
    #[must_use]
    pub fn expected_text(&self) -> String {
        self.expected.render()
    }

    /// The observed outcome, for the table.
    #[must_use]
    pub fn observed_text(&self) -> String {
        match &self.observed {
            None => "pending (oracle not implemented yet)".into(),
            Some(Err(e)) => format!("harness error: {e}"),
            Some(Ok(r)) => match &r.outcome {
                Outcome::Pass => format!("pass ({} cases)", r.cases),
                Outcome::Fail { evidence } => format!(
                    "fail ({}; {}/{} cases; first: {})",
                    class_name(evidence.class),
                    evidence.cases_failed,
                    evidence.cases_total,
                    evidence.first_failing_case.as_deref().unwrap_or("-")
                ),
                Outcome::Rejected { policy } => format!("rejected ({policy})"),
                other => format!("{other:?}"),
            },
        }
    }
}

/// Runs every seeded fault.
///
/// # Errors
///
/// A fault file is malformed, names an unknown task, lacks an expectation
/// for an implemented applicable oracle, or has one for an oracle the
/// matrix marks not applicable.
pub fn run(h: &Harness, specs: &[TaskSpec]) -> Result<Vec<Row>, HarnessError> {
    let faults = load_faults(&h.repo)?;
    let mut rows = Vec::new();
    for f in &faults {
        let spec = specs
            .iter()
            .find(|s| s.id.as_str() == f.task)
            .ok_or_else(|| {
                HarnessError::Input(format!("fault {}: unknown task {}", f.number, f.task))
            })?;
        let applies = |k: &OracleKind| spec.oracles.get(k).is_some_and(|a| a.applies);
        if let Some(k) = f.expect.keys().find(|k| !applies(k)) {
            return Err(HarnessError::Input(format!(
                "fault {}: expectation for {}, which does not apply to {}",
                f.number,
                k.name(),
                f.task
            )));
        }
        let have = oracles::implemented(&f.task);
        if let Some(k) = have
            .iter()
            .find(|k| applies(k) && !f.expect.contains_key(k))
        {
            return Err(HarnessError::Input(format!(
                "fault {}: no expectation for {}",
                f.number,
                k.name()
            )));
        }
        let report = oracles::check(h, spec, &f.implementation())?;
        let mut ran: BTreeMap<OracleKind, Result<OracleRun, String>> = BTreeMap::new();
        for r in report.runs {
            let run = r?;
            ran.insert(run.oracle, Ok(run));
        }
        for (oracle, expected) in &f.expect {
            rows.push(Row {
                fault: f.number,
                task: f.task.clone(),
                oracle: *oracle,
                expected: expected.clone(),
                observed: ran.remove(oracle),
            });
        }
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use otb_core::Evidence;

    const HEADER: &str = "/*\n * Off-by-one.\n * otb-fault: 5\n * otb-task: MLKEM-T07-compress-d\n * otb-lib: none\n * otb-expect: reference-intermediate fail wrong-output\n * otb-expect: ct-static-division fail\n */\n";

    #[test]
    fn parses_a_header() {
        let f = parse_fault(Path::new("x.c"), HEADER).unwrap();
        assert_eq!(f.number, 5);
        assert_eq!(f.task, "MLKEM-T07-compress-d");
        assert_eq!(f.lib, None);
        assert_eq!(
            f.expect[&OracleKind::ReferenceIntermediate],
            Expectation {
                fail: true,
                class: Some(FailureClass::WrongOutput)
            }
        );
        assert_eq!(
            f.expect[&OracleKind::CtStaticDivision],
            Expectation {
                fail: true,
                class: None
            }
        );
    }

    #[test]
    fn rejects_bad_headers() {
        for bad in [
            HEADER.replace("otb-fault: 5", "otb-fault: five"),
            HEADER.replace(" * otb-lib: none\n", ""),
            HEADER.replace("otb-lib: none", "otb-lib: openssl"),
            HEADER.replace("wrong-output", "wrong-everything"),
            HEADER.replace("ct-static-division fail", "reference-intermediate pass"),
            HEADER.replace("ct-static-division", "telepathy"),
            format!("{HEADER} * otb-colour: blue\n"),
        ] {
            assert!(parse_fault(Path::new("x.c"), &bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn expectation_matching() {
        let fail = |class| Outcome::Fail {
            evidence: Evidence {
                class,
                cases_total: 1,
                cases_failed: 1,
                first_failing_case: None,
                detail: String::new(),
            },
        };
        let pass = Expectation {
            fail: false,
            class: None,
        };
        let any_fail = Expectation {
            fail: true,
            class: None,
        };
        let contract = Expectation {
            fail: true,
            class: Some(FailureClass::TaskContract),
        };
        assert!(pass.holds(&Outcome::Pass));
        assert!(!pass.holds(&fail(FailureClass::WrongOutput)));
        assert!(any_fail.holds(&fail(FailureClass::Crash)));
        assert!(!any_fail.holds(&Outcome::Pass));
        assert!(contract.holds(&fail(FailureClass::TaskContract)));
        assert!(!contract.holds(&fail(FailureClass::WrongReturnCode)));
        let rejected = Outcome::Rejected {
            policy: "inline-asm".into(),
        };
        assert!(!pass.holds(&rejected) && !any_fail.holds(&rejected));
    }

    #[test]
    fn repository_faults_parse_and_document_every_implemented_oracle() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let faults = load_faults(&repo).unwrap();
        let numbers: Vec<u32> = faults.iter().map(|f| f.number).collect();
        assert_eq!(numbers, [2, 3, 4, 5, 6, 7]);
        let specs = otb_core::spec::load_all(&repo.join("tasks")).unwrap();
        for f in &faults {
            let spec = specs.iter().find(|s| s.id.as_str() == f.task).unwrap();
            for k in oracles::implemented(&f.task) {
                assert!(
                    f.expect.contains_key(k),
                    "fault {} lacks {}",
                    f.number,
                    k.name()
                );
            }
            for k in f.expect.keys() {
                assert!(spec.oracles[k].applies, "fault {} {}", f.number, k.name());
            }
            if f.lib.is_none() {
                let src = std::fs::read(repo.join(&f.file)).unwrap();
                assert_eq!(crate::admission::check(&src), Ok(()), "fault {}", f.number);
            }
        }
    }
}
