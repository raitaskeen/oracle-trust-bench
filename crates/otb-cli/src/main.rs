//! The `otb` command. Each subcommand is a thin layer over the library crates.

use std::path::PathBuf;
use std::process::ExitCode;

mod budget;

use clap::{Args, Parser, Subcommand};
use otb_core::canonical::to_canonical_string;
use otb_core::spec::{TaskSpec, load_all, render_matrix_markdown};
use otb_core::{CandidateId, OracleKind, Outcome, ToolchainId, Verdict};
use otb_harness::oracles::{self, OracleRun};
use otb_harness::{Harness, ImplKind, Implementation, reference, selftest};

#[derive(Parser)]
#[command(name = "otb", version, about = "oracle-trust-bench harness")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Task specification commands.
    Tasks {
        #[command(subcommand)]
        command: TasksCommand,
    },
    /// Recompute the API credit request (port of analysis/budget.py).
    Budget,
    /// Reference implementation commands.
    Ref {
        #[command(subcommand)]
        command: RefCommand,
    },
    /// Run every applicable, implemented oracle on one candidate C file.
    Check {
        /// Task ID, e.g. MLKEM-T07-compress-d.
        #[arg(long)]
        task: String,
        /// The candidate's C source (one file implementing the task function).
        #[arg(long)]
        candidate: PathBuf,
        /// Print canonical verdict JSON (one per line) instead of a table.
        #[arg(long)]
        json: bool,
        #[command(flatten)]
        sandbox: SandboxArgs,
    },
    /// Run every seeded fault in c/seeded-faults and assert the outcome its
    /// header documents for each oracle.
    Selftest {
        #[command(flatten)]
        sandbox: SandboxArgs,
    },
    /// Print the task-by-oracle applicability matrix.
    Matrix {
        /// Directory holding the task specs.
        #[arg(long, default_value = "tasks")]
        tasks: PathBuf,
        /// Rewrite the generated part of this file instead of printing.
        #[arg(long, value_name = "FILE")]
        write: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum TasksCommand {
    /// Parse every spec and check it against the schema and the FIPS tables.
    Validate {
        /// Directory holding the task specs.
        #[arg(long, default_value = "tasks")]
        tasks: PathBuf,
    },
}

#[derive(Subcommand)]
enum RefCommand {
    /// Run the C references through every implemented oracle and compare
    /// with the spec-literal code and the ACVP vectors.
    Run {
        #[command(flatten)]
        sandbox: SandboxArgs,
    },
}

#[derive(Args)]
struct SandboxArgs {
    /// Sandbox backend: `docker` (default) or `unshare` (weaker; never for
    /// candidates).
    #[arg(long, default_value = "docker")]
    backend: String,
    /// Docker image ID; defaults to $OTB_SANDBOX_IMAGE.
    #[arg(long)]
    image: Option<String>,
}

impl SandboxArgs {
    fn backend(&self) -> Result<otb_sandbox::Backend, String> {
        match self.backend.as_str() {
            "docker" => {
                let image = self
                    .image
                    .clone()
                    .or_else(|| std::env::var("OTB_SANDBOX_IMAGE").ok())
                    .ok_or("docker backend needs --image or OTB_SANDBOX_IMAGE")?;
                Ok(otb_sandbox::Backend::Docker { image })
            }
            "unshare" => Ok(otb_sandbox::Backend::UnshareWeaker),
            other => Err(format!("unknown backend {other:?} (use docker or unshare)")),
        }
    }

    fn harness(&self) -> Result<Harness, Box<dyn std::error::Error>> {
        let repo = std::env::current_dir()?;
        Ok(Harness {
            repo,
            backend: self.backend()?,
        })
    }
}

/// The toolchain ID recorded in verdicts: the backend and, for Docker, the
/// image ID that holds the pinned compilers.
fn toolchain(backend: &otb_sandbox::Backend) -> Result<ToolchainId, otb_core::IdError> {
    match backend {
        otb_sandbox::Backend::Docker { image } => format!("docker:{image}").parse(),
        otb_sandbox::Backend::UnshareWeaker => format!("{}:host-cc", backend.name()).parse(),
    }
}

/// Pending oracles, by milestone, for the report footer.
fn pending_note(pending: &[OracleKind]) -> String {
    let names: Vec<&str> = pending.iter().map(|k| k.name()).collect();
    format!(
        "pending (no code yet, no verdict): {}",
        if names.is_empty() {
            "none".to_owned()
        } else {
            names.join(", ")
        }
    )
}

fn outcome_text(o: &Outcome) -> String {
    match o {
        Outcome::Pass => "pass".into(),
        Outcome::Fail { evidence } => format!(
            "FAIL {} ({}/{} cases; first: {}; {})",
            selftest::class_name(evidence.class),
            evidence.cases_failed,
            evidence.cases_total,
            evidence.first_failing_case.as_deref().unwrap_or("-"),
            evidence.detail
        ),
        Outcome::Rejected { policy } => format!("REJECTED by admission ({policy})"),
        Outcome::NotApplicable => "not applicable".into(),
        Outcome::HarnessError { reason } => format!("HARNESS ERROR {reason}"),
    }
}

fn print_run(r: &OracleRun) {
    println!(
        "{:38} {:24} {:5} {} cases",
        r.task,
        r.oracle.name(),
        match r.outcome {
            Outcome::Pass => "pass",
            Outcome::Rejected { .. } => "REJ",
            _ => "FAIL",
        },
        r.cases
    );
    if !r.outcome.is_pass() {
        println!("{:38}   {}", "", outcome_text(&r.outcome));
    }
    for (group, n) in &r.groups {
        println!("{:38}   {group}: {n}", "");
    }
}

fn ref_run(h: &Harness) -> ExitCode {
    println!("backend: {}", h.backend.name());
    let mut failed = false;
    for r in reference::run_all(h) {
        match r {
            Ok(r) => {
                failed |= !r.outcome.is_pass();
                print_run(&r);
            }
            Err(e) => {
                failed = true;
                println!("HARNESS ERROR: {e}");
            }
        }
    }
    println!("pending (no code yet): ct-dudect (nightly only, reported separately; GAP-CT-003)");
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn find_spec(specs: &[TaskSpec], task: &str) -> Result<TaskSpec, String> {
    specs
        .iter()
        .find(|s| s.id.as_str() == task)
        .cloned()
        .ok_or_else(|| format!("unknown task {task:?}"))
}

fn check(
    h: &Harness,
    task: &str,
    candidate: PathBuf,
    json: bool,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    if matches!(h.backend, otb_sandbox::Backend::UnshareWeaker) {
        return Err(
            "candidate code runs only under the Docker backend (ADR-003, GAP-SBX-001)".into(),
        );
    }
    let specs = load_all(&h.repo.join("tasks"))?;
    let spec = find_spec(&specs, task)?;
    let candidate = std::path::absolute(candidate)?;
    let id = CandidateId::from_source(&std::fs::read(&candidate)?)?;
    let imp = Implementation {
        label: "candidate".into(),
        kind: ImplKind::Candidate,
        source: candidate,
    };
    let report = oracles::check(h, &spec, &imp)?;
    let toolchain = toolchain(&h.backend)?;
    let mut worst = ExitCode::SUCCESS;
    let mut verdicts = Vec::new();
    for r in report.runs {
        match r {
            Ok(run) => {
                if !run.outcome.is_pass() {
                    worst = ExitCode::FAILURE;
                }
                verdicts.push((
                    Verdict {
                        task: spec.id.clone(),
                        candidate: id,
                        oracle: run.oracle,
                        outcome: run.outcome.clone(),
                        toolchain: toolchain.clone(),
                    },
                    run,
                ));
            }
            Err(e) => {
                eprintln!("HARNESS ERROR: {e}");
                return Ok(ExitCode::from(2));
            }
        }
    }
    for k in &report.not_applicable {
        verdicts.push((
            Verdict {
                task: spec.id.clone(),
                candidate: id,
                oracle: *k,
                outcome: Outcome::NotApplicable,
                toolchain: toolchain.clone(),
            },
            OracleRun {
                task: task.to_owned(),
                oracle: *k,
                outcome: Outcome::NotApplicable,
                cases: 0,
                groups: std::collections::BTreeMap::new(),
            },
        ));
    }
    verdicts.sort_by_key(|(v, _)| v.oracle);
    if json {
        for (v, _) in &verdicts {
            println!("{}", to_canonical_string(v)?);
        }
    } else {
        println!("task:      {task}");
        println!("candidate: {id}");
        println!("toolchain: {toolchain}");
        for (v, run) in &verdicts {
            match v.outcome {
                Outcome::NotApplicable => {
                    println!("{:38} {:24} not applicable", "", v.oracle.name());
                }
                _ => print_run(run),
            }
        }
        println!("{}", pending_note(&report.pending));
    }
    Ok(worst)
}

fn run_selftest(h: &Harness) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let specs = load_all(&h.repo.join("tasks"))?;
    println!("backend: {}", h.backend.name());
    println!(
        "{:5} {:36} {:24} {:30} observed",
        "fault", "task", "oracle", "expected"
    );
    let rows = selftest::run(h, &specs)?;
    let (mut held, mut broken, mut pending) = (0, 0, 0);
    for r in &rows {
        let mark = if r.pending() {
            pending += 1;
            "..."
        } else if r.holds() {
            held += 1;
            "ok"
        } else {
            broken += 1;
            "MISMATCH"
        };
        println!(
            "{:5} {:36} {:24} {:30} {} [{mark}]",
            r.fault,
            r.task,
            r.oracle.name(),
            r.expected_text(),
            r.observed_text()
        );
    }
    println!("{held} expectations hold, {broken} do not, {pending} pending");
    Ok(if broken == 0 && held > 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

const MARKER: &str = "<!-- generated by `otb matrix`; do not edit below -->\n";

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode, Box<dyn std::error::Error>> {
    match cli.command {
        Command::Tasks {
            command: TasksCommand::Validate { tasks },
        } => {
            let specs = load_all(&tasks)?;
            let mut bad = 0usize;
            for s in &specs {
                let problems = s.problems();
                if problems.is_empty() {
                    println!("ok    {}", s.id);
                } else {
                    bad += 1;
                    println!("FAIL  {}", s.id);
                    for p in problems {
                        println!("      - {p}");
                    }
                }
            }
            println!(
                "{} specs, {} valid, {} invalid",
                specs.len(),
                specs.len() - bad,
                bad
            );
            Ok(if bad == 0 && !specs.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            })
        }
        Command::Budget => {
            print!("{}", budget::render());
            Ok(ExitCode::SUCCESS)
        }
        Command::Ref {
            command: RefCommand::Run { sandbox },
        } => Ok(ref_run(&sandbox.harness()?)),
        Command::Check {
            task,
            candidate,
            json,
            sandbox,
        } => check(&sandbox.harness()?, &task, candidate, json),
        Command::Selftest { sandbox } => run_selftest(&sandbox.harness()?),
        Command::Matrix { tasks, write } => {
            let generated = render_matrix_markdown(&load_all(&tasks)?);
            match write {
                None => print!("{generated}"),
                Some(path) => {
                    let text = std::fs::read_to_string(&path)?.replace("\r\n", "\n");
                    let (head, _) = text
                        .split_once(MARKER)
                        .ok_or("file has no generated-section marker")?;
                    std::fs::write(&path, format!("{head}{MARKER}{generated}"))?;
                    println!("wrote {}", path.display());
                }
            }
            Ok(ExitCode::SUCCESS)
        }
    }
}
