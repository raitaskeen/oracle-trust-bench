//! Runs untrusted commands (compilers, C references, shims and candidates)
//! with no network access and with resource limits. See ADR-003.
//!
//! Two backends:
//!
//! - [`Backend::Docker`] (default): a container from the pinned image with
//!   `--network none`, a read-only root filesystem, tmpfs scratch, pids,
//!   memory and CPU limits, all capabilities dropped, `no-new-privileges`,
//!   and an unprivileged user. Inputs are mounted read-only at `/in`.
//! - [`Backend::UnshareWeaker`], the fallback, uses `unshare` (new user and
//!   network namespace) plus `prlimit`. **Weaker**: it isolates the network and limits
//!   resources, but the command can read and write the host filesystem with
//!   the caller's permissions. Its name says so wherever it is printed.
//!
//! Either way the command sees two environment variables: `OTB_IN`, the
//! input directory, and `OTB_WORK`, an empty scratch directory. Untrusted
//! code is never loaded into the calling process.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// Where and how a job runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Backend {
    /// A Docker container from `image` (an image ID or digest reference).
    Docker {
        /// The image, ideally `sha256:...` so the exact image is used.
        image: String,
    },
    /// The weaker fallback: `unshare` + `prlimit` on the host.
    UnshareWeaker,
}

impl Backend {
    /// A stable name for logs and run records. The fallback's name states
    /// that it is weaker.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Docker { .. } => "docker",
            Self::UnshareWeaker => "unshare-prlimit-WEAKER-no-filesystem-isolation",
        }
    }
}

/// Resource limits for one job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Wall-clock limit; the job is killed when it is exceeded.
    pub wall: Duration,
    /// Memory limit in bytes (Docker: memory and swap; fallback: address space).
    pub memory_bytes: u64,
    /// Maximum number of processes or threads.
    pub pids: u32,
    /// CPU quota, in CPUs (Docker only), as a decimal string such as `"1"`.
    pub cpus: String,
    /// Size of the scratch tmpfs in MiB (Docker only).
    pub scratch_mib: u32,
    /// Maximum bytes kept from each of stdout and stderr; the rest is dropped
    /// and the outcome is marked truncated.
    pub max_output_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            wall: Duration::from_secs(60),
            memory_bytes: 2 << 30,
            pids: 64,
            cpus: "1".to_owned(),
            scratch_mib: 256,
            max_output_bytes: 64 << 20,
        }
    }
}

/// One command to run.
#[derive(Clone, Debug)]
pub struct Job {
    /// Host directory made available as `$OTB_IN` (read-only under Docker).
    pub input_dir: PathBuf,
    /// The command and its arguments.
    pub argv: Vec<String>,
    /// Bytes written to the command's stdin.
    pub stdin: Vec<u8>,
    /// Resource limits.
    pub limits: Limits,
}

/// How the command ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    /// Exited normally with this code.
    Code(i32),
    /// Killed by a signal (number if known), including resource-limit kills.
    Signal(Option<i32>),
    /// Exceeded the wall-clock limit and was killed.
    TimedOut,
}

/// The result of a job.
#[derive(Clone, Debug)]
pub struct Outcome {
    /// How it ended.
    pub exit: Exit,
    /// Captured stdout (at most `max_output_bytes`).
    pub stdout: Vec<u8>,
    /// Captured stderr (at most `max_output_bytes`).
    pub stderr: Vec<u8>,
    /// True if output was cut at `max_output_bytes`.
    pub truncated: bool,
}

impl Outcome {
    /// True if the command exited with code 0.
    #[must_use]
    pub fn success(&self) -> bool {
        self.exit == Exit::Code(0)
    }
}

/// Errors that stop a job from being run at all.
#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    /// The input directory is missing or not absolute.
    #[error("input directory {0} must exist and be absolute")]
    InputDir(String),
    /// The command list is empty.
    #[error("empty command")]
    EmptyCommand,
    /// The backend program (docker, unshare, prlimit) could not be started.
    #[error("could not start {program}: {source}")]
    Spawn {
        /// The program.
        program: String,
        /// The I/O error.
        source: std::io::Error,
    },
    /// Waiting on or talking to the process failed.
    #[error("i/o: {0}")]
    Io(#[from] std::io::Error),
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_name() -> String {
    format!(
        "otb-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

/// The `docker run` arguments for a job. Exposed for review and testing.
#[must_use]
pub fn docker_args(image: &str, job: &Job, name: &str) -> Vec<String> {
    let l = &job.limits;
    let mut a: Vec<String> = [
        "run",
        "--rm",
        "--interactive",
        "--network=none",
        "--read-only",
        "--cap-drop=ALL",
        "--security-opt=no-new-privileges",
        "--user=10001:10001",
        "--ipc=none",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    a.push(format!("--name={name}"));
    a.push(format!("--pids-limit={}", l.pids));
    a.push(format!("--memory={}", l.memory_bytes));
    a.push(format!("--memory-swap={}", l.memory_bytes));
    a.push(format!("--cpus={}", l.cpus));
    a.push(format!(
        "--tmpfs=/work:rw,exec,nosuid,nodev,size={}m,uid=10001,gid=10001,mode=0700",
        l.scratch_mib
    ));
    a.push("--tmpfs=/tmp:rw,exec,nosuid,nodev,size=64m,mode=1777".to_owned());
    a.push(format!(
        "--mount=type=bind,source={},target=/in,readonly",
        job.input_dir.display()
    ));
    a.push("--env=OTB_IN=/in".to_owned());
    a.push("--env=OTB_WORK=/work".to_owned());
    a.push("--workdir=/work".to_owned());
    a.push(image.to_owned());
    // A second wall-clock limit inside the container, 10 s after the
    // harness's own, so the container ends even if the harness process
    // dies and cannot kill it.
    a.push("timeout".to_owned());
    a.push("--signal=KILL".to_owned());
    a.push(format!("{}s", l.wall.as_secs() + 10));
    a.extend(job.argv.iter().cloned());
    a
}

/// The `unshare` + `prlimit` arguments for a job. Exposed for review and testing.
#[must_use]
pub fn unshare_args(job: &Job, work: &Path) -> Vec<String> {
    let l = &job.limits;
    let mut a: Vec<String> = vec![
        "--user".into(),
        "--map-root-user".into(),
        "--net".into(),
        "--".into(),
        "prlimit".into(),
        format!("--as={}", l.memory_bytes),
        format!("--nproc={}", l.pids),
        format!("--cpu={}", l.wall.as_secs().max(1)),
        "--core=0".into(),
        "--".into(),
        "env".into(),
        format!("OTB_IN={}", job.input_dir.display()),
        format!("OTB_WORK={}", work.display()),
    ];
    a.extend(job.argv.iter().cloned());
    a
}

/// Runs a job on a backend and waits for it, enforcing the wall-clock limit.
pub fn run(backend: &Backend, job: &Job) -> Result<Outcome, SandboxError> {
    if job.argv.is_empty() {
        return Err(SandboxError::EmptyCommand);
    }
    if !job.input_dir.is_absolute() || !job.input_dir.is_dir() {
        return Err(SandboxError::InputDir(job.input_dir.display().to_string()));
    }
    match backend {
        Backend::Docker { image } => {
            let name = unique_name();
            let mut cmd = Command::new("docker");
            cmd.args(docker_args(image, job, &name));
            supervise(cmd, "docker", job, || {
                // Killing the client does not stop the container.
                let _ = Command::new("docker")
                    .args(["kill", &name])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            })
        }
        Backend::UnshareWeaker => {
            let work = std::env::temp_dir().join(unique_name());
            std::fs::create_dir(&work)?;
            let mut cmd = Command::new("unshare");
            cmd.args(unshare_args(job, &work)).current_dir(&work);
            let out = supervise(cmd, "unshare", job, || {});
            let _ = std::fs::remove_dir_all(&work);
            out
        }
    }
}

fn read_capped(
    mut r: impl Read + Send + 'static,
    cap: usize,
) -> thread::JoinHandle<(Vec<u8>, bool)> {
    thread::spawn(move || {
        let mut kept = Vec::new();
        let mut truncated = false;
        let mut buf = vec![0u8; 65536];
        loop {
            match r.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let room = cap.saturating_sub(kept.len());
                    if n > room {
                        truncated = true;
                    }
                    kept.extend_from_slice(&buf[..n.min(room)]);
                }
            }
        }
        (kept, truncated)
    })
}

fn supervise(
    mut cmd: Command,
    program: &str,
    job: &Job,
    on_timeout: impl FnOnce(),
) -> Result<Outcome, SandboxError> {
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Own process group, so a timeout kills every descendant, not only the
    // direct child (otherwise a grandchild keeps the output pipes open).
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut cmd, 0);
    let mut child: Child = cmd.spawn().map_err(|source| SandboxError::Spawn {
        program: program.to_owned(),
        source,
    })?;
    let mut stdin = child.stdin.take().expect("stdin is piped");
    let input = job.stdin.clone();
    let writer = thread::spawn(move || {
        let _ = stdin.write_all(&input);
    });
    let cap = job.limits.max_output_bytes;
    let out = read_capped(child.stdout.take().expect("stdout is piped"), cap);
    let err = read_capped(child.stderr.take().expect("stderr is piped"), cap);

    let start = Instant::now();
    let mut timed_out = false;
    let status = loop {
        if let Some(s) = child.try_wait()? {
            break s;
        }
        if start.elapsed() >= job.limits.wall {
            timed_out = true;
            on_timeout();
            kill_group(child.id());
            let _ = child.kill();
            break child.wait()?;
        }
        thread::sleep(Duration::from_millis(5));
    };
    let _ = writer.join();
    let (stdout, t1) = out.join().unwrap_or_default();
    let (stderr, t2) = err.join().unwrap_or_default();
    let exit = if timed_out {
        Exit::TimedOut
    } else if let Some(code) = status.code() {
        Exit::Code(code)
    } else {
        Exit::Signal(signal_of(status))
    };
    Ok(Outcome {
        exit,
        stdout,
        stderr,
        truncated: t1 || t2,
    })
}

/// Sends SIGKILL to the whole process group led by `pid`.
#[cfg(unix)]
fn kill_group(pid: u32) {
    let _ = Command::new("kill")
        .args(["-KILL", "--", &format!("-{pid}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(not(unix))]
fn kill_group(_pid: u32) {}

#[cfg(unix)]
fn signal_of(status: std::process::ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

#[cfg(not(unix))]
fn signal_of(_status: std::process::ExitStatus) -> Option<i32> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job() -> Job {
        Job {
            input_dir: PathBuf::from("/in-host"),
            argv: vec!["sh".into(), "-c".into(), "true".into()],
            stdin: Vec::new(),
            limits: Limits::default(),
        }
    }

    #[test]
    fn docker_args_contain_every_isolation_flag() {
        let a = docker_args("sha256:abc", &job(), "otb-test");
        for flag in [
            "--network=none",
            "--read-only",
            "--cap-drop=ALL",
            "--security-opt=no-new-privileges",
            "--user=10001:10001",
            "--pids-limit=64",
            "--memory-swap=2147483648",
            "--mount=type=bind,source=/in-host,target=/in,readonly",
        ] {
            assert!(a.iter().any(|x| x == flag), "missing {flag}: {a:?}");
        }
        // The image comes right before the in-container time limit (the
        // harness limit, 60 s here, plus 10 s), then the command.
        let i = a.iter().position(|x| x == "sha256:abc").unwrap();
        assert_eq!(
            &a[i + 1..],
            ["timeout", "--signal=KILL", "70s", "sh", "-c", "true"]
        );
    }

    #[test]
    fn unshare_args_isolate_the_network() {
        let a = unshare_args(&job(), Path::new("/tmp/w"));
        assert_eq!(&a[..3], ["--user", "--map-root-user", "--net"]);
        assert!(a.contains(&"OTB_WORK=/tmp/w".to_owned()));
    }

    #[test]
    fn fallback_name_says_weaker() {
        assert!(Backend::UnshareWeaker.name().contains("WEAKER"));
    }

    #[test]
    fn rejects_relative_input_dir_and_empty_command() {
        let mut j = job();
        j.input_dir = PathBuf::from("relative");
        assert!(matches!(
            run(&Backend::UnshareWeaker, &j),
            Err(SandboxError::InputDir(_))
        ));
        let mut j = job();
        j.argv.clear();
        assert!(matches!(
            run(&Backend::UnshareWeaker, &j),
            Err(SandboxError::EmptyCommand)
        ));
    }
}
