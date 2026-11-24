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
| M4 | Reference-intermediate, malformed-input, implicit-rejection oracles; admission; seeded faults 2-7 | done | `a405a06`; `otb ref run` passes; `otb selftest` 10/10 expectations held at M4 |
| M5a | Constant-time oracles (Valgrind, static division), fault 1, false-positive table | done | `a2973fd`; ADR-005; `otb ref run` passes every oracle; `otb selftest` 25/25 (`c/seeded-faults/README.md`); `docs/oracles/false-positives.md` |
| M5b | Runner for model-written test suites, mutants, fault 8 | done | ADR-006; 121 mutants (103 counted); `otb selftest` 29/29 including fault 8's suite |
| M6 | Determinism at 1/4/16 jobs, `run.json`, stats, `otb power` | next | `otb budget` done (`c88d45f`); `otb selftest` exists from M4 |
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

### M5a checklist (done)

- [x] Measurements first (`docs/notes/ct-measurements.md`): where division
      by q survives (only Cortex-M0+ at -O2/-O3), what memcheck reports
      (branches and addresses; not division, not conditional moves).
- [x] ADR-005 and the draft entry D1 in `protocol/PREREGISTRATION.md`, with
      four points left for the author to confirm before filing.
- [x] `ct-valgrind`: the driver marks the spec's secrets undefined (T24:
      dk_PKE and z only; the spec now says so), GCC 14.2 at -O2 and -O3.
- [x] `ct-static-division`: 10 emitted-code configurations, division
      instructions and helpers, counted only when reachable from the task
      function; LLVM IR -O0 count as a diagnostic.
- [x] Reference adapters for T01 and T47 made constant time (they used %
      and a branch around the library).
- [x] Seeded fault 1; CT expectations for faults 2-7 written before the
      first run; 24 of 25 held, fault 7's corrected with the reason.
- [x] False-positive table: 0/5 for both CT oracles on the references.
- [x] `ct_probes.rs` asserts the Valgrind blind spots in CI.

### M5b checklist (done)

- [x] Test-file interface (`c/harness/test-abi/`): a C file with `main`
      and `OTB_ASSERT`; assertions counted at run time by linked harness
      code; test-file admission (no redefining `OTB_`/`otb_` names).
- [x] Mutation bases for the five tasks (`c/mutation-base/`), each passing
      every functional oracle; a deterministic first-order mutant
      generator; committed manifests (121 mutants, 103 killed by a
      functional oracle; the 18 survivors checked, GAP-MUT-001).
- [x] `otb suite`: validity (admitted, compiles, passes on the reference
      with at least 5 assertions, kills a counted mutant), mutation score,
      and the result on the candidate.
- [x] `otb mutants classify` and `otb mutants verify`.
- [x] Seeded fault 8 and its weak suite; `otb selftest` 29/29.
- [x] ADR-006; draft pre-registration entry D2.

Changed on the way: the mutant set is generated from the bases by fixed
operators, not written by hand as first planned, so it is reproducible and
can be cited by hash.

### M6 next steps

- [ ] `run.json`: hashes of every input, the image, the toolchain; one
      record per run.
- [ ] Determinism: verdict files byte-identical at 1, 4 and 16 jobs (needs
      a parallel runner).
- [ ] Statistics (`otb-stats`): Wilson intervals, task-clustered
      bootstrap, TOST at +-5 pp, Holm; checked against published values.
- [ ] `otb power` over the sensitivity grid (self-test pass rate, ICC,
      true false-assurance rate); KNOWN_GAPS entry if power < 0.8.

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
