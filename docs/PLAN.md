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
| M1 | Workspace, core types, canonical JSON, gaps ledger, CI | done | commit `8ba1463` |
| M2 | Spec format, five pilot specs, spec-literal FIPS functions | done | commit `a260d48`; `otb tasks validate` |
| M3 | Vectors, sandbox, C shims, `otb ref run` | done | `800ee3e`, `9003b27`, `2933c29`, `2023fb1`, `2d9364b`; `otb ref run` passes on both backends |
| M4 | Reference-intermediate, malformed-input, implicit-rejection oracles; admission; seeded faults 2-7 | done | the M4 commit; `otb ref run` passes; `otb selftest` 10/10 expectations hold (`c/seeded-faults/README.md`) |
| M5a | Constant-time oracles (Valgrind, static division), fault 1, false-positive table | next | |
| M5b | Runner for model-written test suites, mutants, fault 8 | later | |
| M6 | Determinism at 1/4/16 jobs, `run.json`, stats, `otb power`; selftest in CI as a gate | later | `otb budget` done (`c88d45f`); `otb selftest` exists from M4 |
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

### M4 checklist (done)

- [x] One code path for references, seeded faults and candidates
      (`Implementation`, ADR-004); failures of the code under test become
      verdict classes (compile error, crash, timeout, wrong output), never
      harness errors.
- [x] Candidate admission check (ADR-004, GAP-ADM-001), with evasion tests.
- [x] `malformed-input` for T24: FIPS 203 section 7.3 checks 1 and 2,
      check 3 as task contract (GAP-SPEC-001), and valid-input controls;
      44 cases, each cross-checked against the spec-literal input check.
- [x] `implicit-rejection` for T24: 4 keys x (1 valid + 8 modified
      ciphertexts), K = J(z || c) computed independently; 36 cases.
- [x] ACVP key-check failures classed as task contract.
- [x] Seeded faults 2-7 in `c/seeded-faults/`, expectations in their
      headers; `otb selftest` holds 10/10.
- [x] `otb check --task ID --candidate FILE [--json]`: canonical verdicts
      per oracle; Docker only.
- [x] In-container `timeout` so an orphaned container ends on its own;
      per-program wall-clock limit 120 s.

### M5a next steps

- [ ] ADR-005 (constant-time method: emitted code at -O2 and -O3 primary,
      source/IR secondary) and `protocol/PREREGISTRATION.md` draft entry.
- [ ] `ct-valgrind`: secrets marked undefined via memcheck client requests
      in the driver (never in the candidate), declassification per spec.
- [ ] `ct-static-division`: division instructions and division helper calls
      in emitted code for the pinned targets.
- [ ] Seeded fault 1 (KyberSlash pattern); CT expectations for faults 2-7.
- [ ] False-positive table on the references
      (`docs/oracles/false-positives.md`).

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
