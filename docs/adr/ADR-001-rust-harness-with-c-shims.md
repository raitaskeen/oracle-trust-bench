# ADR-001: Rust harness with C shims

Status: accepted (24 Nov 2026)

## Context

The harness compiles untrusted C written by language models, runs it, and
decides whether it is correct and constant-time. Its verdicts are the
study's data. It must be correct, deterministic, fast enough to check
2,700 implementations with exhaustive domains of up to 8,380,417 inputs,
and easy for a reviewer to audit.

## Decision

- **The harness is Rust** (edition 2024, toolchain pinned in
  `rust-toolchain.toml`), with `unsafe` code forbidden in every crate.
  Rust gives memory safety in the component that parses untrusted vector
  files and tool output, and it compiles to a single static binary.
- **Candidate C never enters the Rust process.** It is compiled and run
  inside the sandbox (ADR-003) with a small C driver per task (the "shim")
  that reads inputs and writes outputs over a pipe. Comparison happens in
  Rust, outside the candidate's process.
- **Bulk data uses fixed-width binary frames; control uses JSON lines.**
  Exhaustive checks send millions of fixed-size records, which JSON would
  slow down and bloat.
- **Few, focused crates**: `otb-core` (types, specs, canonical JSON),
  `otb-vectors` (ACVP parsers), `otb-oracles` (checks and the spec-literal
  reference), `otb-sandbox`, `otb-harness`, `otb-stats`, and the `otb` CLI.
  A crate is added only when its milestone needs it.
- **Performance by design, not by tuning**: parallelism over candidates and
  oracles (planned with `rayon` at M6, with results sorted by a stable key
  so output is identical at any job count); exact integer arithmetic; no
  allocation in inner comparison loops. Optimizations are added only after a
  measurement shows the need, and the measurement goes in the PR.
- **Dependencies are few and mainstream** (`serde`, `serde_json`, `toml`,
  `clap`, RustCrypto `sha2`/`sha3`/`shake`, `thiserror`, `proptest`),
  pinned in `Cargo.lock` and checked by `cargo deny` and `cargo audit`.

## Consequences

- The C shims are the only C written by the project; they are small and
  reviewed as carefully as the oracles.
- Contributors need Rust and Docker; the C toolchain lives in the image.
- Windows hosts work for the pure-Rust parts; anything that runs C needs
  Linux (WSL2 on the author's machine).
