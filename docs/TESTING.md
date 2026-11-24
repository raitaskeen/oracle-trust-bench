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
| Seeded-fault self-test (`otb selftest`) | Every oracle catches the faults it should, and misses only the documented blind spots | CLI, in the sandbox | every CI run (from M6); 100% required |
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
```

## Current numbers

Produced by `cargo nextest run --workspace --locked` on 24 Nov 2026, at
commit `d248392`: 55 tests run, 55 passed. The same 55 pass with the network
removed (`unshare -r -n`).
