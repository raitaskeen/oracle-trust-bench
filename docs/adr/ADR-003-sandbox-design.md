# ADR-003: Sandbox design

Status: accepted (M3, 24 Nov 2026)

## Context

The harness compiles and runs C written by language models. That code is
untrusted: it may be wrong, slow, crash, fork, allocate without limit or
try to reach the network. It must never run with the author's privileges
or see the network, the author's files or the expected values.

## Decision

All C (references, drivers, seeded faults and candidates) runs through
`otb-sandbox`, which has two backends.

**Docker (default; required for candidate code).** One container per job,
from the image built by `docker/Dockerfile` (Debian 13.8 pinned by digest;
compilers and tools pinned to exact Debian package versions, so a build
fails rather than installing a different compiler), with:

- `--network=none`;
- `--read-only` root filesystem; tmpfs scratch at `/work` and `/tmp`;
- inputs bind-mounted read-only at `/in`; only the files the build needs
  are staged there (never the repository, never `inputs/`, never expected
  values);
- `--cap-drop=ALL`, `--security-opt=no-new-privileges`, user 10001,
  `--ipc=none`;
- `--pids-limit`, `--memory` (swap equal to memory), `--cpus`;
- a wall-clock limit enforced by the harness, which then kills the
  container.

**unshare + prlimit (fallback, weaker).** New user and network namespaces,
with address-space, process and CPU-time limits. It blocks the network but
does **not** isolate the filesystem (GAP-SBX-001). Its backend name,
`unshare-prlimit-WEAKER-no-filesystem-isolation`, appears in every run
record. It exists so the harness itself can be developed and tested where
Docker is unavailable.

Every job runs in its own process group, so a timeout kills every
descendant, not just the direct child.

Commands see `OTB_IN` and `OTB_WORK`, so the same command line works on
both backends.

## Evidence

`crates/otb-sandbox/tests/isolation.rs`: `connect()` to a public address
fails under both backends; the wall-clock limit kills under both; under
Docker the root filesystem and the input mount are read-only. CI runs these
tests on every push (job `sandbox`).

## Limits

- Docker isolation depends on the host kernel and Docker; a container
  escape is outside this threat model (`docs/THREAT_MODEL.md`, M7).
- Valgrind and timing measurements also run inside the container, so the
  limits must leave room for them; limits are per job and recorded.
