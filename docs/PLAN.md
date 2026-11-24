# Build plan and status

This is the single tracker for building the verification side of the
study. Agents and humans update it in the same commit as the work it
describes. Status words: `done`, `in progress`, `next`, `blocked`, `later`.

Last updated: 24 Nov 2026.

## Scope reminder

The repository builds the **verification side** of the study: task specs,
independent non-LLM oracles, a sandboxed harness and the protocol. It makes
no model calls and holds no results. See `protocol/APPLICATION.md`.

## Order of targets

1. **Harness language: Rust.** Every oracle, the sandbox driver and the
   statistics are Rust (ADR-001).
2. **Code under test: C.** Arm 1 asks models for C implementations of
   ML-KEM and ML-DSA components, built against a fixed C interface per task.
3. **ML-KEM first, then ML-DSA.** The application makes ML-KEM within
   Arms 1-2 the core, run first; ML-DSA and Arm 3 are reduced, not dropped,
   if time runs short.
4. **Arm 3 (C-to-Rust migration) last.** The `differential` oracle stays a
   documented stub until then.

## Milestones

| # | Milestone | Status | Evidence |
|---|---|---|---|
| M0 | Environment: Rust, WSL2 Ubuntu 24.04, Docker, Valgrind | done | `docs/notes/environment.md` |
| M1 | Workspace, core types, canonical JSON, gaps ledger, CI | done | commit `ff78dee` |
| M2 | Spec format, five pilot specs, spec-literal FIPS functions | done | commit `b7e5923`; `otb tasks validate` |
| M3 | Vectors, sandbox, C shims, `otb ref run` | done | `8d8e2fa`, `9c0cac6`, `70bf205`, `86343cd` and the M3-close commit; `otb ref run` passes on both backends |
| M4 | Reference-intermediate, malformed-input, implicit-rejection oracles; seeded faults 2-7 | next | |
| M5a | Constant-time oracles (Valgrind, static division), fault 1, false-positive table | later | |
| M5b | Runner for model-written test suites, mutants, fault 8 | later | |
| M6 | `otb selftest`, determinism at 1/4/16 jobs, stats, `otb power` | later | `otb budget` already done (`d248392`) |
| M7 | Documentation and mdBook | later | |
| M8 | Independent review | later | |

### M3 checklist (done)

- [x] Container image: Dockerfile on `debian:trixie-slim` pinned by digest
      (`sha256:8aa5a7bf5a1955bddbe7d74fcdd0b91ab30e74362a8b73af672b0481b35b44a0`,
      Debian 13.8), with Debian's GCC, Clang and Valgrind pinned to exact
      package versions; the built image is referenced by its ID in each run
      (`docs/notes/environment.md`).
- [x] `otb-sandbox`: Docker backend (`--network none`, `--read-only`, tmpfs
      scratch, pids, memory, CPU and wall-clock limits) and the weaker
      `prlimit` + `unshare` fallback, labelled weaker everywhere.
- [x] Test: a candidate calling `connect()` fails under both backends.
- [x] Submodules: mlkem-native v2.0.0 and mldsa-native v2.0.0, pinned by
      full commit SHA; recorded in `NOTICE.md`.
- [x] C shims (fixed ABI per task, binary framing for bulk vectors) for the
      references, portable C backend only.
- [x] `otb ref run`: references through every applicable oracle; ACVP
      counts by group for ML-KEM-768 and ML-DSA-65.
- [x] ML-DSA ACVP parser and spec-literal agreement where it applies
      (pre-hash groups not run: GAP-ACVP-003).

## Toolchain policy

| Tool | Version now | How it is pinned | When it changes |
|---|---|---|---|
| Rust | 1.100.0 (latest stable on 24 Nov 2026) | `rust-toolchain.toml` | A reviewed PR that reruns the full CI |
| Rust crates | latest compatible at the time of adding | `Cargo.lock` | A reviewed PR; `cargo deny` and `cargo audit` must pass |
| cargo-nextest, cargo-deny, cargo-audit | 0.9.152, 0.22.0, 0.23.0 (latest stable) | CI installs by tool name; local copies recorded here | When CI's installer moves; checked in review |
| GitHub Actions | checkout v7.0.1, install-action v2.90.8 | full commit SHA in `ci.yml` | A reviewed PR |
| C compilers for candidates | Debian 13 packages inside the pinned image | image digest | **Never during the main run.** Fixed before the pre-registration is filed; a change would change verdicts |

The C compiler is the one place where "latest" is deliberately not the
rule. The study measures what code does when compiled by one fixed
toolchain; ADR-005 records which compiler and why.

## How work is tracked

- Each milestone ends with a commit and a summary to the author.
- Open questions and decisions go in `docs/notes/`; architecture decisions
  go in `docs/adr/` (one page each).
- Limits go in `KNOWN_GAPS.md` with a stable ID.
- Testing rules are in `docs/TESTING.md`; review rules in `CONTRIBUTING.md`.
