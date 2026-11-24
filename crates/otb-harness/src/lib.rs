//! Builds task programs inside the sandbox and runs the applicable oracles.
//!
//! A task program is `c/harness/drivers.c` (selected with `-DOTB_TASK_<n>`)
//! linked with one [`Implementation`] of the task's C function: a reference
//! adapter, a seeded fault or a candidate. The program is compiled and run
//! in the sandbox; only the files the build needs are staged into its
//! read-only input directory. Expected values never enter the sandbox:
//! outputs come back over stdout and are compared here.
//!
//! The only panics are `expect` calls on conversions of values whose range
//! the code has just fixed (for example `x < q` into `u16`); they document
//! invariants, so per-function `# Panics` sections are not required here.
#![allow(clippy::missing_panics_doc)]

pub mod admission;
pub mod ct;
pub mod mutants;
pub mod oracles;
pub mod reference;
pub mod rng;
pub mod selftest;
pub mod suite;

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
    /// A harness-owned program (not an implementation under test) failed to
    /// build or run.
    #[error("{what}: {exit:?}\n{stderr}")]
    Program {
        /// What was being run.
        what: String,
        /// How it ended.
        exit: Exit,
        /// Its stderr.
        stderr: String,
    },
    /// A harness-owned program's output had the wrong size.
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
    /// Reading task specs or seeded-fault metadata failed.
    #[error("{0}")]
    Input(String),
}

/// Which pinned library a reference adapter or seeded fault builds on.
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

    /// The library a task's reference builds on.
    #[must_use]
    pub fn for_task(task: &str) -> Self {
        if task.starts_with("MLDSA-") {
            Self::MlDsa
        } else {
            Self::MlKem
        }
    }
}

/// What an implementation is, which decides what it may see when built.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImplKind {
    /// A reference adapter over a pinned library. Trusted.
    Reference(Library),
    /// A hand-written seeded fault. Trusted; may build on a library. A fault
    /// with no library is admitted and staged exactly like a candidate.
    SeededFault(Option<Library>),
    /// Untrusted code under test. Must pass admission; never sees a library.
    Candidate,
}

impl ImplKind {
    /// The library staged for this implementation, if any.
    #[must_use]
    pub fn library(&self) -> Option<Library> {
        match self {
            Self::Reference(l) => Some(*l),
            Self::SeededFault(l) => *l,
            Self::Candidate => None,
        }
    }

    /// True if the source must pass [`admission::check`] before it is built.
    #[must_use]
    pub fn needs_admission(&self) -> bool {
        matches!(self, Self::Candidate | Self::SeededFault(None))
    }
}

/// One implementation of one task's C function.
#[derive(Clone, Debug)]
pub struct Implementation {
    /// Short label for reports, e.g. `reference` or `fault 5`.
    pub label: String,
    /// What it is.
    pub kind: ImplKind,
    /// The C source file (absolute, or relative to the repository root).
    pub source: PathBuf,
}

impl Implementation {
    /// The reference adapter for `task`.
    #[must_use]
    pub fn reference(task: &str) -> Self {
        let lib = Library::for_task(task);
        Self {
            label: "reference".into(),
            kind: ImplKind::Reference(lib),
            source: PathBuf::from(lib.adapter()),
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

/// What running an implementation produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProgramRun {
    /// The program built, exited normally and wrote exactly the expected
    /// number of bytes.
    Output(Vec<u8>),
    /// The implementation failed before producing comparable output. This
    /// is evidence about the implementation, never a harness error.
    Failed {
        /// Compile error, crash, timeout or wrong output size.
        class: FailureClass,
        /// Deterministic description (no paths, PIDs or tool logs).
        detail: String,
    },
}

/// Printed by the build script between compiling and running, so that a
/// missing marker means the build failed.
const BUILD_OK: &str = "OTB-BUILD-OK";

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

fn new_stage() -> Result<Stage, HarnessError> {
    let dir = std::env::temp_dir().join(format!(
        "otb-stage-{}-{}",
        std::process::id(),
        STAGE.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir)?;
    Ok(Stage(dir))
}

/// The files of `c/harness` every task program needs. Reference adapters
/// and whole-program tools are not staged for implementations.
const DRIVER_FILES: [&str; 3] = ["drivers.c", "io.h", "tasks.h"];

/// Wall-clock limit for one build-and-run of a task program, including
/// compilation. The largest input set (T41, 8,380,417 records) takes a few
/// seconds with the reference (`docs/notes/environment.md`); the limit
/// leaves room for slow but correct candidates. A candidate that exceeds it
/// fails with class `timeout`.
pub const TASK_WALL_LIMIT: Duration = Duration::from_secs(120);

/// The output limit and wall-clock limit for a task program.
fn task_limits(expected_len: usize) -> Limits {
    Limits {
        wall: TASK_WALL_LIMIT,
        max_output_bytes: expected_len + 1,
        ..Limits::default()
    }
}

impl Harness {
    fn path(&self, p: &Path) -> PathBuf {
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.repo.join(p)
        }
    }

    /// Builds the driver for `task_flag` (e.g. `T07`) with `imp`, runs it on
    /// `input`, and classifies the result. Admission is the caller's job
    /// (see [`oracles::check`]); this only builds and runs.
    pub fn run_impl(
        &self,
        task_flag: &str,
        imp: &Implementation,
        input: Vec<u8>,
        expected_len: usize,
    ) -> Result<ProgramRun, HarnessError> {
        let out = self.run_impl_script(
            imp,
            |lib_include| {
                format!(
                    "cc -std=c11 -O2 -I\"$OTB_IN/harness\"{lib_include} -DOTB_TASK_{task_flag} \
                     \"$OTB_IN/harness/drivers.c\" \"$OTB_IN/impl/impl.c\" -o \"$OTB_WORK/prog\" \
                     && echo {BUILD_OK} >&2 && exec \"$OTB_WORK/prog\""
                )
            },
            input,
            task_limits(expected_len),
        )?;
        Ok(classify(&out, expected_len))
    }

    /// Stages `imp` (as `$OTB_IN/impl/impl.c`), the driver files, the
    /// cross-target headers (`$OTB_IN/harness/ct-include`) and, if the kind
    /// allows it, the library (`$OTB_IN/lib`); then runs the shell script
    /// that `script` builds from the library include flag (empty when no
    /// library is staged). Used by every oracle that builds an
    /// implementation (ADR-004).
    pub fn run_impl_script(
        &self,
        imp: &Implementation,
        script: impl FnOnce(&str) -> String,
        input: Vec<u8>,
        limits: Limits,
    ) -> Result<otb_sandbox::Outcome, HarnessError> {
        let stage = new_stage()?;
        let harness_dir = stage.0.join("harness");
        std::fs::create_dir_all(&harness_dir)?;
        for f in DRIVER_FILES {
            std::fs::copy(self.repo.join("c/harness").join(f), harness_dir.join(f))?;
        }
        copy_dir(
            &self.repo.join("c/harness/ct-include"),
            &harness_dir.join("ct-include"),
        )?;
        std::fs::create_dir_all(stage.0.join("impl"))?;
        std::fs::copy(self.path(&imp.source), stage.0.join("impl/impl.c"))?;
        let lib_include = match imp.kind.library() {
            Some(lib) => {
                copy_dir(&self.repo.join(lib.source_dir()), &stage.0.join("lib"))?;
                " -I\"$OTB_IN/lib\""
            }
            None => "",
        };
        let job = Job {
            input_dir: stage.0.clone(),
            argv: vec!["sh".into(), "-c".into(), script(lib_include)],
            stdin: input,
            limits,
        };
        Ok(otb_sandbox::run(&self.backend, &job)?)
    }

    /// Stages `files` (path inside the stage, contents), `tasks.h`, the
    /// test-file interface (`$OTB_IN/harness/test-abi`) and, if given, a
    /// library (`$OTB_IN/lib`), then runs `script`. Used by the test-suite
    /// runner, which builds one suite against many implementations in one
    /// sandbox job (ADR-006). Untrusted files are staged only as inputs; the
    /// script decides what is compiled.
    pub fn run_files_script(
        &self,
        files: &[(String, Vec<u8>)],
        lib: Option<Library>,
        script: &str,
        limits: Limits,
    ) -> Result<otb_sandbox::Outcome, HarnessError> {
        let stage = new_stage()?;
        let harness_dir = stage.0.join("harness");
        std::fs::create_dir_all(&harness_dir)?;
        std::fs::copy(
            self.repo.join("c/harness/tasks.h"),
            harness_dir.join("tasks.h"),
        )?;
        copy_dir(
            &self.repo.join("c/harness/test-abi"),
            &harness_dir.join("test-abi"),
        )?;
        if let Some(lib) = lib {
            copy_dir(&self.repo.join(lib.source_dir()), &stage.0.join("lib"))?;
        }
        for (rel, bytes) in files {
            let dest = stage.0.join(rel);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(dest, bytes)?;
        }
        let job = Job {
            input_dir: stage.0.clone(),
            argv: vec!["sh".into(), "-c".into(), script.to_owned()],
            stdin: Vec::new(),
            limits,
        };
        Ok(otb_sandbox::run(&self.backend, &job)?)
    }

    /// Builds `sources` (file names in `c/harness`) with `flags` against
    /// `lib`, runs the program on `input`, and returns its stdout, which must
    /// be exactly `expected_len` bytes. For harness-owned whole programs
    /// only (e.g. the ML-DSA ACVP tool); any failure is a harness error.
    pub fn run_program(
        &self,
        label: &str,
        lib: Library,
        sources: &[&str],
        flags: &str,
        input: Vec<u8>,
        expected_len: usize,
    ) -> Result<Vec<u8>, HarnessError> {
        let stage = new_stage()?;
        copy_dir(&self.repo.join("c/harness"), &stage.0.join("harness"))?;
        copy_dir(&self.repo.join(lib.source_dir()), &stage.0.join("lib"))?;
        let files: Vec<String> = sources
            .iter()
            .map(|s| format!("\"$OTB_IN/harness/{s}\""))
            .collect();
        let script = format!(
            "cc -std=c11 -O2 -I\"$OTB_IN/harness\" -I\"$OTB_IN/lib\" {flags} {} \
             -o \"$OTB_WORK/prog\" && exec \"$OTB_WORK/prog\"",
            files.join(" ")
        );
        let job = Job {
            input_dir: stage.0.clone(),
            argv: vec!["sh".into(), "-c".into(), script],
            stdin: input,
            limits: task_limits(expected_len),
        };
        let what = format!("{label} ({})", self.backend.name());
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

/// Turns a sandbox result into a [`ProgramRun`].
fn classify(out: &otb_sandbox::Outcome, expected_len: usize) -> ProgramRun {
    let built = String::from_utf8_lossy(&out.stderr).contains(BUILD_OK);
    let failed = |class, detail: String| ProgramRun::Failed { class, detail };
    match out.exit {
        Exit::TimedOut => failed(FailureClass::Timeout, "wall-clock limit exceeded".into()),
        _ if !built => failed(
            FailureClass::CompileError,
            "did not compile with the pinned toolchain".into(),
        ),
        Exit::Signal(s) => failed(
            FailureClass::Crash,
            format!(
                "killed by signal {}",
                s.map_or_else(|| "?".to_owned(), |n| n.to_string())
            ),
        ),
        Exit::Code(c) if c != 0 => failed(FailureClass::Crash, format!("exited with code {c}")),
        Exit::Code(_) if out.stdout.len() != expected_len => failed(
            FailureClass::WrongOutput,
            format!(
                "wrote {} output bytes, expected {expected_len}",
                out.stdout.len()
            ),
        ),
        Exit::Code(_) => ProgramRun::Output(out.stdout.clone()),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn out(exit: Exit, stdout: &[u8], stderr: &str) -> otb_sandbox::Outcome {
        otb_sandbox::Outcome {
            exit,
            stdout: stdout.to_vec(),
            stderr: stderr.as_bytes().to_vec(),
            truncated: false,
        }
    }

    fn class_of(r: &ProgramRun) -> Option<FailureClass> {
        match r {
            ProgramRun::Output(_) => None,
            ProgramRun::Failed { class, .. } => Some(*class),
        }
    }

    #[test]
    fn classification() {
        let ok = format!("{BUILD_OK}\n");
        assert_eq!(
            classify(&out(Exit::Code(0), b"ab", &ok), 2),
            ProgramRun::Output(b"ab".to_vec())
        );
        let cases = [
            (
                out(Exit::Code(1), b"", "error: x"),
                FailureClass::CompileError,
            ),
            (out(Exit::TimedOut, b"", ""), FailureClass::Timeout),
            (out(Exit::TimedOut, b"", &ok), FailureClass::Timeout),
            (out(Exit::Signal(Some(11)), b"", &ok), FailureClass::Crash),
            (out(Exit::Code(3), b"ab", &ok), FailureClass::Crash),
            (out(Exit::Code(0), b"abc", &ok), FailureClass::WrongOutput),
            (out(Exit::Code(0), b"a", &ok), FailureClass::WrongOutput),
        ];
        for (o, want) in cases {
            assert_eq!(class_of(&classify(&o, 2)), Some(want), "{o:?}");
        }
    }

    #[test]
    fn candidates_never_see_a_library() {
        assert_eq!(ImplKind::Candidate.library(), None);
        assert!(ImplKind::Candidate.needs_admission());
        assert!(ImplKind::SeededFault(None).needs_admission());
        assert!(!ImplKind::SeededFault(Some(Library::MlKem)).needs_admission());
        assert!(!ImplKind::Reference(Library::MlDsa).needs_admission());
    }
}
