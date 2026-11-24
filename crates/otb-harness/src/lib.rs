//! Builds task programs inside the sandbox and runs the applicable oracles.
//!
//! A task program is `c/harness/drivers.c` (selected with `-DOTB_TASK_<n>`)
//! linked with one implementation of the task's C function. The program is
//! compiled and run in the sandbox; only the files the build needs are
//! staged into its read-only input directory. Expected values never enter
//! the sandbox: outputs come back over stdout and are compared here.
//!
//! The only panics are `expect` calls on conversions of values whose range
//! the code has just fixed (for example `x < q` into `u16`); they document
//! invariants, so per-function `# Panics` sections are not required here.
#![allow(clippy::missing_panics_doc)]

pub mod reference;
pub mod rng;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use otb_core::{Evidence, FailureClass, Outcome};
use otb_sandbox::{Backend, Exit, Job, Limits};

/// Errors that stop the harness from producing a verdict. These are harness
/// failures, never evidence about the code under test.
#[derive(Debug, thiserror::Error)]
pub enum HarnessError {
    /// Staging the build inputs failed.
    #[error("staging: {0}")]
    Stage(#[from] std::io::Error),
    /// The sandbox could not run the job.
    #[error("sandbox: {0}")]
    Sandbox(#[from] otb_sandbox::SandboxError),
    /// A reference program failed to build or run.
    #[error("{what}: {exit:?}\n{stderr}")]
    Program {
        /// What was being run.
        what: String,
        /// How it ended.
        exit: Exit,
        /// Its stderr.
        stderr: String,
    },
    /// The program's output had the wrong size.
    #[error("{what}: expected {expected} output bytes, got {got}")]
    OutputSize {
        /// What was being run.
        what: String,
        /// Expected byte count.
        expected: usize,
        /// Actual byte count.
        got: usize,
    },
    /// Reading the ACVP vectors failed.
    #[error("vectors: {0}")]
    Vectors(#[from] otb_vectors::VectorError),
}

/// Which pinned library a reference adapter builds on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Library {
    /// mlkem-native (`third_party/mlkem-native/mlkem`).
    MlKem,
    /// mldsa-native (`third_party/mldsa-native/mldsa`).
    MlDsa,
}

impl Library {
    fn source_dir(self) -> &'static str {
        match self {
            Self::MlKem => "third_party/mlkem-native/mlkem",
            Self::MlDsa => "third_party/mldsa-native/mldsa",
        }
    }

    /// The reference adapter source, relative to the repository root.
    #[must_use]
    pub fn adapter(self) -> &'static str {
        match self {
            Self::MlKem => "c/harness/ref_mlkem.c",
            Self::MlDsa => "c/harness/ref_mldsa.c",
        }
    }
}

/// Where the harness runs things.
#[derive(Clone, Debug)]
pub struct Harness {
    /// Repository root (holds `c/` and `third_party/`).
    pub repo: PathBuf,
    /// Sandbox backend.
    pub backend: Backend,
}

static STAGE: AtomicU64 = AtomicU64::new(0);

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    let mut entries: Vec<_> = std::fs::read_dir(from)?.flatten().collect();
    entries.sort_by_key(std::fs::DirEntry::path);
    for e in entries {
        let p = e.path();
        let dest = to.join(e.file_name());
        if p.is_dir() {
            copy_dir(&p, &dest)?;
        } else if p.is_file() {
            std::fs::copy(&p, &dest)?;
        }
    }
    Ok(())
}

/// A staged build directory, deleted on drop.
struct Stage(PathBuf);

impl Drop for Stage {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl Harness {
    fn stage(&self, lib: Library) -> Result<Stage, HarnessError> {
        let dir = std::env::temp_dir().join(format!(
            "otb-stage-{}-{}",
            std::process::id(),
            STAGE.fetch_add(1, Ordering::Relaxed)
        ));
        copy_dir(&self.repo.join("c/harness"), &dir.join("harness"))?;
        copy_dir(&self.repo.join(lib.source_dir()), &dir.join("lib"))?;
        Ok(Stage(dir))
    }

    /// Builds the driver for `task_flag` (e.g. `T07`) against a reference
    /// adapter, runs it on `input`, and returns its stdout, which must be
    /// exactly `expected_len` bytes.
    pub fn run_reference(
        &self,
        task_flag: &str,
        lib: Library,
        input: Vec<u8>,
        expected_len: usize,
    ) -> Result<Vec<u8>, HarnessError> {
        let stage = self.stage(lib)?;
        let adapter = Path::new(lib.adapter())
            .file_name()
            .and_then(|s| s.to_str())
            .expect("adapter has a file name");
        let script = format!(
            "cc -std=c11 -O2 -I\"$OTB_IN/harness\" -I\"$OTB_IN/lib\" -DOTB_TASK_{task_flag} \
             \"$OTB_IN/harness/drivers.c\" \"$OTB_IN/harness/{adapter}\" -o \"$OTB_WORK/prog\" \
             && exec \"$OTB_WORK/prog\""
        );
        let job = Job {
            input_dir: stage.0.clone(),
            argv: vec!["sh".into(), "-c".into(), script],
            stdin: input,
            limits: Limits {
                wall: Duration::from_secs(600),
                max_output_bytes: expected_len + 1,
                ..Limits::default()
            },
        };
        let what = format!("{task_flag} reference ({})", self.backend.name());
        let out = otb_sandbox::run(&self.backend, &job)?;
        if !out.success() {
            return Err(HarnessError::Program {
                what,
                exit: out.exit,
                stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            });
        }
        if out.stdout.len() != expected_len {
            return Err(HarnessError::OutputSize {
                what,
                expected: expected_len,
                got: out.stdout.len(),
            });
        }
        Ok(out.stdout)
    }
}

/// Counts from one oracle run, turned into an [`Outcome`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    /// Cases run.
    pub total: u64,
    /// Cases that failed.
    pub failed: u64,
    /// ID of the first failing case, in case order.
    pub first_failure: Option<String>,
    /// Class of the first failure.
    pub class: Option<FailureClass>,
}

impl Tally {
    /// Records one case.
    pub fn check(&mut self, ok: bool, case: impl FnOnce() -> String, class: FailureClass) {
        self.total += 1;
        if !ok {
            self.failed += 1;
            if self.first_failure.is_none() {
                self.first_failure = Some(case());
                self.class = Some(class);
            }
        }
    }

    /// The outcome: `Pass` if no case failed.
    #[must_use]
    pub fn outcome(&self, detail: &str) -> Outcome {
        if self.failed == 0 {
            Outcome::Pass
        } else {
            Outcome::Fail {
                evidence: Evidence {
                    class: self.class.unwrap_or(FailureClass::WrongOutput),
                    cases_total: self.total,
                    cases_failed: self.failed,
                    first_failing_case: self.first_failure.clone(),
                    detail: detail.to_owned(),
                },
            }
        }
    }
}
