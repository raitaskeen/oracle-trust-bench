//! The result of running one oracle on one candidate.

use serde::{Deserialize, Serialize};

use crate::{CandidateId, OracleKind, TaskId, ToolchainId};

/// One oracle's result for one candidate of one task.
///
/// A verdict holds no timestamp, host name or path, so that the same inputs
/// and toolchain always serialize to the same bytes (ADR-002). Timing goes in
/// the run log, which is not part of any hash.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Verdict {
    /// The task.
    pub task: TaskId,
    /// The candidate implementation.
    pub candidate: CandidateId,
    /// The oracle that produced this verdict.
    pub oracle: OracleKind,
    /// What happened.
    pub outcome: Outcome,
    /// The toolchain that compiled and ran the candidate.
    pub toolchain: ToolchainId,
}

/// What an oracle concluded.
///
/// Only `Pass` and `Fail` are evidence about the candidate. `NotApplicable`
/// comes from the static oracle matrix. `Rejected` means the candidate broke
/// the admission policy before it was compiled. `HarnessError` means the
/// harness itself failed; such verdicts are rerun and never counted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Outcome {
    /// Every applicable case passed.
    Pass,
    /// At least one case failed. Candidate-caused crashes and timeouts are
    /// failures, recorded in the evidence class.
    Fail {
        /// What failed, in deterministic form.
        evidence: Evidence,
    },
    /// The oracle does not apply to this task (decided by the matrix).
    NotApplicable,
    /// The candidate broke the admission policy and was not compiled.
    Rejected {
        /// The policy rule that was broken, e.g. `inline-asm`.
        policy: String,
    },
    /// The harness failed for a reason unrelated to the candidate.
    HarnessError {
        /// A deterministic description of the harness failure.
        reason: String,
    },
}

impl Outcome {
    /// True only for `Pass`.
    #[must_use]
    pub fn is_pass(&self) -> bool {
        matches!(self, Self::Pass)
    }

    /// True only for `Fail`.
    #[must_use]
    pub fn is_fail(&self) -> bool {
        matches!(self, Self::Fail { .. })
    }
}

/// Deterministic evidence for a failure. It never contains raw tool logs,
/// process IDs, addresses or paths; those go to unhashed artifacts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    /// The kind of failure.
    pub class: FailureClass,
    /// Number of cases the oracle ran.
    pub cases_total: u64,
    /// Number of cases that failed.
    pub cases_failed: u64,
    /// Stable ID of the first failing case in canonical case order, if the
    /// oracle has cases.
    pub first_failing_case: Option<String>,
    /// A short, deterministic, human-readable description.
    pub detail: String,
}

/// The kind of failure an oracle observed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FailureClass {
    /// An output differs from the expected value.
    WrongOutput,
    /// A return code differs from the expected value.
    WrongReturnCode,
    /// The task contract is broken where FIPS itself allows the behaviour
    /// (for example, no per-call dk hash check in T24).
    TaskContract,
    /// The candidate crashed (signal, abort, sanitizer report).
    Crash,
    /// The candidate exceeded its time limit.
    Timeout,
    /// The candidate failed to compile with the pinned toolchain.
    CompileError,
    /// A branch depends on secret data.
    SecretDependentBranch,
    /// A memory address depends on secret data.
    SecretDependentAddress,
    /// A variable-time instruction (such as division) has a secret operand.
    VariableTimeInstruction,
    /// A statistical timing test exceeded its threshold.
    TimingLeak,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::to_canonical_string;

    fn sample(outcome: Outcome) -> Verdict {
        Verdict {
            task: "MLKEM-T07-compress-d".parse().unwrap(),
            candidate: CandidateId::from_source(b"int x;\n").unwrap(),
            oracle: OracleKind::CtStaticDivision,
            outcome,
            toolchain: "test-toolchain".parse().unwrap(),
        }
    }

    #[test]
    fn canonical_form_is_stable() {
        let v = sample(Outcome::Fail {
            evidence: Evidence {
                class: FailureClass::VariableTimeInstruction,
                cases_total: 1,
                cases_failed: 1,
                first_failing_case: Some("O2/x86_64".into()),
                detail: "div on secret operand".into(),
            },
        });
        let s = to_canonical_string(&v).unwrap();
        assert_eq!(
            s,
            concat!(
                r#"{"candidate":"7c725f30854a46033dd94f728ac6b08caf10845993cd3ed48e40079cdb0a76a6","#,
                r#""oracle":"ct-static-division","#,
                r#""outcome":{"evidence":{"cases_failed":1,"cases_total":1,"class":"variable-time-instruction","#,
                r#""detail":"div on secret operand","first_failing_case":"O2/x86_64"},"kind":"fail"},"#,
                r#""task":"MLKEM-T07-compress-d","toolchain":"test-toolchain"}"#
            )
        );
        let back: Verdict = serde_json::from_str(&s).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn unit_outcomes_serialize_with_kind() {
        let s = to_canonical_string(&sample(Outcome::NotApplicable)).unwrap();
        assert!(s.contains(r#""outcome":{"kind":"not-applicable"}"#), "{s}");
    }

    #[test]
    fn unknown_fields_rejected() {
        let s = to_canonical_string(&sample(Outcome::Pass)).unwrap();
        let tampered = s.replacen('{', r#"{"timestamp":"2026-11-24","#, 1);
        assert!(serde_json::from_str::<Verdict>(&tampered).is_err());
    }
}
