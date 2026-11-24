# Testing strategy

The harness decides whether other code can be trusted, so the harness
itself is held to a higher standard than the code it checks. This page
says what is tested, how, and where.

## Levels

| Level | What it proves | Where | Runs |
|---|---|---|---|
| Unit tests | Each function does what its doc says | next to the code (`#[cfg(test)]`) | every CI run |
| Property tests (`proptest`) | Parsers never panic; round trips hold; stats behave on random input | next to the code | every CI run |
| Standard-property tests | Spec-literal code satisfies properties the standards state, exhaustively where the domain is small | `crates/otb-oracles/src/speclit/` | every CI run |
| Vector tests | Spec-literal code matches every vendored ACVP case | `crates/otb-oracles/tests/` | every CI run |
| Reference run (`otb ref run`) | Every applicable oracle passes on the C references | CLI, in the sandbox | every CI run (from M3) |
| Seeded-fault self-test (`otb selftest`) | Every oracle catches the faults it should, and misses only the documented blind spots | CLI, in the sandbox; expectations in each `c/seeded-faults/*.c` header | every CI run (from M4); every expectation must hold |
| Valgrind probe (`ct_probes.rs`) | memcheck still reports secret-dependent branches and addresses, and still misses division and conditional moves (the documented blind spots, GAP-CT-001) | `crates/otb-harness/tests/ct_probes.rs`, in the sandbox | every CI run (sandbox job) |
| Admission tests | Each admission rule rejects what it names, and the known evasions (line splicing, trigraphs, digraphs, token pasting, universal character names) are caught | `crates/otb-harness/src/admission.rs` | every CI run |
| Determinism | Verdict files are byte-identical across reruns and at 1, 4 and 16 jobs | CI script | every CI run (from M6) |
| Golden tests (`insta`) | Report output does not change by accident | `otb-cli` | every CI run (from M6) |
| Fuzzing (`cargo-fuzz`) | The ACVP and spec parsers survive hostile input | `fuzz/` | nightly (from M6) |
| dudect | Timing leaks on the pre-registered subsample | nightly workflow | nightly only; never gates CI (GAP-CT-003) |

## Rules

1. **Every reference value names its source**: a standard (section,
   equation or algorithm), an ACVP file, a paper with its table, or a
   command whose output was recorded. No value is typed from memory.
2. **Never weaken a test to make it pass.** A disagreement between an oracle
   and a reference is investigated and recorded in `KNOWN_GAPS.md`.
3. **Oracles are tested both ways.** Each oracle must pass the references
   (no false positives) and fail the seeded faults it targets (no false
   negatives). Documented blind spots are asserted too, so a silent change
   in an oracle's behaviour is caught.
4. **No network and no secrets in tests.** CI runs the test suite inside a
   network namespace with no interfaces.
5. **No timing assertions in default CI.** Shared runners are noisy.
6. **Untrusted C runs only in the sandbox**, including in tests.
7. **Tests are deterministic.** Random inputs come from fixed seeds that
   are printed on failure; property tests can be replayed.

## Running the checks locally

On Linux (WSL2 Ubuntu 24.04 for the author):

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo nextest run --workspace --locked
cargo deny --locked check licenses bans sources
cargo audit --deny warnings
cargo run --locked -q -p otb-cli -- tasks validate
# with Docker and OTB_SANDBOX_IMAGE set (docs/notes/environment.md):
cargo nextest run --workspace --locked --run-ignored all
cargo run --locked -q --release -p otb-cli -- ref run
cargo run --locked -q --release -p otb-cli -- selftest
```

## Current numbers

Produced on 24 Nov 2026 at the commit that closed M5a:

- `cargo nextest run --workspace --locked`: 89 tests run, 89 passed,
  6 skipped (the sandbox isolation tests and the Valgrind probe, which need
  Docker and `OTB_SANDBOX_IMAGE`).
- The same with `--run-ignored all` and `OTB_SANDBOX_IMAGE` set to the
  local image: 95 tests run, 95 passed.
- `otb ref run` (Docker): every implemented oracle passes on every
  reference (18 oracle runs, including both constant-time oracles on all
  five tasks); 92 s wall-clock.
- `otb selftest` (Docker): 25 expectations hold, 0 do not, 0 pending;
  48 s wall-clock. Output in `c/seeded-faults/README.md`.

Earlier records: at the M4 close (`a405a06`), 80 run and 80 passed (85
with Docker), selftest 10/10; at the M3 close (`2d9364b`), 63 run and 63 passed (68
with Docker); at `c88d45f`, 55 run and 55 passed, also with the network
removed (`unshare -r -n`).
