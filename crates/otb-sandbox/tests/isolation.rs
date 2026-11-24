//! Isolation tests that start real sandboxes. They need Linux, a C compiler
//! (on the host for the fallback, in the image for Docker) and, for Docker,
//! the environment variable `OTB_SANDBOX_IMAGE` naming the built image.
//!
//! They are `#[ignore]`d so that a plain `cargo test` on a machine without
//! Docker does not fail; CI runs them explicitly with `--run-ignored all`
//! after building the image. A missing `OTB_SANDBOX_IMAGE` is a test
//! failure, not a skip.

use std::path::PathBuf;
use std::time::Duration;

use otb_sandbox::{Backend, Exit, Job, Limits, run};

fn c_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../c/sandbox-tests")
        .canonicalize()
        .expect("c/sandbox-tests exists")
}

fn docker() -> Backend {
    let image = std::env::var("OTB_SANDBOX_IMAGE")
        .expect("set OTB_SANDBOX_IMAGE to the built image ID (see docs/TESTING.md)");
    Backend::Docker { image }
}

fn sh(script: &str) -> Job {
    Job {
        input_dir: c_dir(),
        argv: vec!["sh".into(), "-c".into(), script.into()],
        stdin: Vec::new(),
        limits: Limits {
            wall: Duration::from_secs(60),
            ..Limits::default()
        },
    }
}

const CONNECT: &str = r#"cc -O2 -o "$OTB_WORK/connect" "$OTB_IN/connect.c" && "$OTB_WORK/connect""#;

fn assert_connect_fails(backend: &Backend) {
    let out = run(backend, &sh(CONNECT)).expect("sandbox runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.success() && stdout.contains("CONNECT_FAILED"),
        "{}: expected connect() to fail; exit {:?}\nstdout: {stdout}\nstderr: {stderr}",
        backend.name(),
        out.exit
    );
}

#[test]
#[ignore = "needs Docker and OTB_SANDBOX_IMAGE"]
fn docker_blocks_connect() {
    assert_connect_fails(&docker());
}

#[test]
#[ignore = "needs Linux user namespaces and a host C compiler"]
fn unshare_fallback_blocks_connect() {
    assert_connect_fails(&Backend::UnshareWeaker);
}

#[test]
#[ignore = "needs Docker and OTB_SANDBOX_IMAGE"]
fn docker_root_filesystem_and_inputs_are_read_only() {
    let out = run(
        &docker(),
        &sh(r#"touch /etc/otb 2>/dev/null && echo ROOT_WRITABLE; touch "$OTB_IN/otb" 2>/dev/null && echo IN_WRITABLE; touch "$OTB_WORK/ok" && echo WORK_OK"#),
    )
    .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.contains("ROOT_WRITABLE"), "{stdout}");
    assert!(!stdout.contains("IN_WRITABLE"), "{stdout}");
    assert!(stdout.contains("WORK_OK"), "{stdout}");
}

#[test]
#[ignore = "needs Docker and OTB_SANDBOX_IMAGE"]
fn docker_kills_on_wall_clock_limit() {
    let mut job = sh("sleep 30");
    job.limits.wall = Duration::from_secs(3);
    let out = run(&docker(), &job).unwrap();
    assert_eq!(out.exit, Exit::TimedOut);
}

#[test]
#[ignore = "needs Linux user namespaces"]
fn unshare_kills_on_wall_clock_limit() {
    let mut job = sh("sleep 30");
    job.limits.wall = Duration::from_secs(1);
    let out = run(&Backend::UnshareWeaker, &job).unwrap();
    assert_eq!(out.exit, Exit::TimedOut);
}
