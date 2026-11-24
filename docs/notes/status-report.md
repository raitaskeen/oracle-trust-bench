# Status report and guide (after M4)

First written after M3 (commit `2d9364b`); updated 24 Nov 2026 at the
commit that closed M4. Every number below names the command that
produced it; judgements are marked as judgements.

No model has been run; no results yet. This repository builds the
verification side of "The Oracle-Trust Gap" (`protocol/APPLICATION.md`).

## 1. What exists today

| Part | What it does | Evidence |
|---|---|---|
| Task specs (`tasks/`) | Five pilot tasks: MLKEM-T01 (NTT), T07 (Compress_d), T24 (decapsulation with implicit rejection), MLDSA-T41 (Power2Round), T47 (MakeHint). Each names its FIPS algorithm, interface, oracles and pass rule. FIX-1 to FIX-4 applied. | `otb tasks validate` (5/5); `docs/notes/fips-verification.md` |
| Oracle matrix | Which oracle applies to which task, generated from the specs | `otb matrix`; `protocol/ORACLE_MATRIX.md` |
| Spec-literal Rust reference | FIPS 203/204 functions written to follow the standard line by line: NTT, Compress, K-PKE, full ML-KEM, Power2Round, Decompose, MakeHint, UseHint | Standard-property tests; matches all 240 ML-KEM ACVP cases (`crates/otb-oracles/tests/acvp_mlkem.rs`) |
| ACVP vectors | NIST example vector sets for ML-KEM and ML-DSA, vendored with SHA-256 sums; strict parsers | `third_party/acvp/SHA256SUMS`; `otb-vectors` |
| C references | mlkem-native and mldsa-native v2.0.0 as pinned submodules, portable C backend | `NOTICE.md` |
| Sandbox | Docker (no network, read-only root, dropped capabilities, uid 10001, memory/pids/CPU/wall-clock limits) and a fallback labelled weaker | 9/9 isolation tests pass on both backends (`docs/notes/environment.md`) |
| Candidate image | Debian 13.8 pinned by digest; GCC 14.2.0, Clang 19.1.7, Valgrind 3.24.0 at exact package versions | `docs/notes/environment.md` |
| Reference run | Compiles the C references in the sandbox and runs every implemented oracle on them | `otb ref run`, below |
| Malformed-input oracle (M4) | T24: wrong ciphertext and key lengths (FIPS 203 section 7.3 checks 1 and 2), the per-call dk hash check (task contract), and valid-input controls; 44 cases | `crates/otb-harness/src/oracles.rs` |
| Implicit-rejection oracle (M4) | T24: 4 keys x 8 modified ciphertexts must return 0 and J(z ‖ c), computed independently in Rust; 36 cases | same |
| Admission check (M4) | Rejects candidate code that could tamper with in-process oracles (inline asm, pragmas, constructors, token pasting, trigraphs, disallowed headers...) before it is compiled | ADR-004; GAP-ADM-001 |
| Seeded faults 2-7 (M4) | Six hand-written faulty implementations; `otb selftest` checks each against the outcomes its header documents | `c/seeded-faults/README.md` |
| `otb check` (M4) | Judges one candidate C file: admission, then every applicable implemented oracle; canonical verdict JSON with `--json` | ADR-004 |
| Budget | Port of `analysis/budget.py`; both print `TOTAL 955.98` | `otb budget`; CI step "Budget" |
| CI | fmt, clippy (pedantic, `-D warnings`), cargo-deny, cargo-audit, spec validation, budget check, tests with the network removed; sandbox job with Docker, `otb ref run` and `otb selftest` | `.github/workflows/ci.yml`. On GitHub the jobs have not started yet: GitHub refuses Actions minutes on the private repository because of an account billing problem; no test has failed |
| Rules and records | AGENTS.md, CONTRIBUTING.md (review order), TESTING.md, PLAN.md, four ADRs, KNOWN_GAPS.md (13 entries; `grep -c '^## GAP-' KNOWN_GAPS.md`), AI_ASSISTANCE.md | the files themselves |

Size (`git ls-files` piped to `wc -l`, at the M4 commit): 6644 lines of
Rust in six crates, 685 lines of C (harness and seeded faults), 26
Markdown files.

### `otb ref run`, Docker backend

| Check | Oracle | Result | Cases |
|---|---|---|---|
| MLDSA-T41 Power2Round | reference-intermediate | pass | 8,380,417 (every input in [0, q)) |
| MLDSA-T47 MakeHint | reference-intermediate | pass | 1,000,340 (340 boundary cases) |
| MLKEM-T01 NTT | reference-intermediate | pass | 1,258 |
| MLKEM-T07 Compress_d | reference-intermediate | pass | 16,645 |
| MLKEM-T24 decapsulation | acvp | pass | 20 (10 with a modified ciphertext or H) |
| MLKEM-T24 decapsulation | malformed-input | pass | 44 |
| MLKEM-T24 decapsulation | implicit-rejection | pass | 36 |
| ML-DSA-65 reference build | acvp | pass | 160 (45 pre-hash cases not covered, GAP-ACVP-003) |

Wall-clock: 8 s. Tests: `cargo nextest run --workspace --locked
--run-ignored all` with the image set, 85 run, 85 passed.

### `otb selftest`, Docker backend

10 expectations hold, 0 do not (full table in `c/seeded-faults/README.md`).
Highlights: fault 5 (off-by-one rounding) is wrong on only 5 of 16,645
inputs and is found by the exhaustive check; fault 7 (one flipped
comparison) is wrong on 98 of 1,000,340 inputs, found by the boundary
cases; fault 3 (no per-call key hash check) is reported as a
`task-contract` failure, not a FIPS violation.

### `otb check` on three hand-written candidates (MLKEM-T07)

| Candidate | Result |
|---|---|
| includes `stdio.h` | `Rejected (header:stdio.h)`, not compiled |
| correct `compress_d` written from the formula | pass, 16,645 cases |
| never returns | `fail (timeout)` after 126 s (limit 120 s); no container left running |

## 2. What it can do now, and what it cannot

It **can**:

- validate task specs against the FIPS algorithm tables and render the
  oracle matrix;
- take any C file that claims to implement a pilot task, refuse it if it
  breaks the admission policy, and otherwise compile and run it in a
  locked-down container;
- judge it against three independent sources (spec-literal Rust, the C
  reference, ACVP), exhaustively where the input domain allows;
- for ML-KEM decapsulation, check the FIPS 203 input checks and implicit
  rejection, not only the happy path;
- prove its own oracles work: six seeded faults, each caught as documented;
- prove the sandbox blocks the network and kills runaway jobs.

It **cannot yet**:

- check constant-time behaviour: Valgrind and emitted-code division
  checks (M5a, ADR-005); these are applicable to every pilot task and are
  listed as pending, without a verdict;
- run model-written test suites or mutants (M5b);
- write a `run.json`, check determinism at 1/4/16 jobs, or run the
  statistics or the power grid (M6);
- serve as a public benchmark: no README, PREREGISTRATION.md, THREAT_MODEL
  or mdBook yet (M7), and no independent review (M8).

## 2a. Who can use this, and for what

Stated in terms of the study; no product claims.

- **Researchers** get a reusable, pre-registered way to measure whether
  tests written for cryptographic code (by models or people) actually
  detect faults, with oracles that do not depend on any model.
- **Developers of post-quantum code** get a harness that checks a C
  implementation of an ML-KEM or ML-DSA component against FIPS 203/204,
  the NIST ACVP vectors and the pinned references, including the error
  paths (input checks, implicit rejection) that happy-path tests miss.
- **Reviewers and auditors** get a list of the fault patterns that slip
  past weak tests (the seeded faults) and the exact oracle that catches
  each, plus a ledger of what the oracles cannot see (`KNOWN_GAPS.md`).

## 3. Rating (judgement)

Rated as a research artifact, which is what the application promises;
it is not software for sale.

| Aspect | Now | Reason |
|---|---|---|
| Engineering foundation | 8/10 | Pinned toolchain, image and actions; strict lints; sandbox tested; one code path for every implementation; every gap recorded. Loses points because CI has never run on GitHub and the image is not yet fixed by digest (GAP-ENV-002). |
| Correctness evidence | 8/10 | Exhaustive or near-exhaustive checks where the domain allows; ACVP agreement; every generated case cross-checked against the spec-literal code; oracles proven on seeded faults. ACVP sets are small example sets (GAP-ACVP-002), and the reference is trusted by agreement, not proof (GAP-REF-001). |
| Completeness against the plan | 5/10 | M0-M4 of M0-M8 done. Constant-time oracles, the test-suite runner, determinism, statistics and public docs remain. |
| Documentation | 6/10 | Internal docs are thorough; the public-facing README and pre-registration do not exist yet (M7). |
| **Overall as a finished benchmark today** | **6/10** | Judges candidates end to end for correctness, input checks and implicit rejection; not yet for constant time. |
| Expected at M8 if the plan holds | 8/10 | Assuming M5-M7 land as planned and the independent review finds nothing structural. |

(After M3 the overall judgement was 4/10.)

## 4. Remaining work and time (estimates)

Estimates are in working sessions like this one and assume no blocking
environment problems. M3 took longer than estimated because of the
Debian mirror: the image build took 6775 s (about 1 h 53 min), of which
apt reported "Fetched 152 MB in 1h 34min 48s (26.7 kB/s)".

| Milestone | Work | Estimate |
|---|---|---|
| M4 | Done in one session | - |
| M5a | ADR-005; Valgrind secret-marking oracle; emitted-code division check at -O2 and -O3; fault 1; false-positive table | 2 sessions |
| M5b | Runner for model-written test suites; mutants; fault 8 | 1-2 sessions |
| M6 | `run.json`; determinism at 1/4/16 jobs; statistics; `otb power` grid | 1-2 sessions |
| M7 | README, PREREGISTRATION.md (draft), THREAT_MODEL, mdBook | 1 session |
| M8 | Independent review by someone other than the author and Claude | depends on the reviewer |

## 5. Publishing on GitHub

**Done, private.** At the author's request in chat (24 Nov 2026) the
repository was created as a private repository at
`https://github.com/raitaskeen/oracle-trust-bench` and `main` was pushed.
Before the push:

- every commit was rewritten to the author
  `Taskeen Haider <137790592+raitaskeen@users.noreply.github.com>` (ID from
  `https://api.github.com/users/raitaskeen`); the file trees are unchanged
  and commit hashes cited in the docs were updated;
- tracked files were checked: nothing under `inputs/`, and the only email
  addresses are GitHub noreply addresses.

**Public:** after M7, when the README states "No model has been run; no
results yet" and PREREGISTRATION.md exists. It must be public, with the
pre-registration timestamped, **before** any model is run, because that
is what makes the study pre-registered. Visibility changes only on the
author's request.

To push later work yourself from WSL (no `gh` there), let Git borrow the
Windows GitHub CLI login for one command:

```sh
git -c credential.helper='!"/mnt/c/Program Files/GitHub CLI/gh.exe" auth git-credential' push
```

The submodules point to their public upstream repositories, so they need
nothing extra.

After the push, open the Actions tab and check that both jobs (`rust`,
`sandbox`) pass. If one fails, paste the log to Claude.

## 6. Guide: running it yourself

Inside WSL, from `~/projects/oracle-trust-bench`:

```sh
git submodule update --init                      # once, fetches the C references
cargo nextest run --workspace --locked           # unit, property and vector tests
cargo run -q -p otb-cli -- tasks validate        # check the five specs
cargo run -q -p otb-cli -- matrix                # print the oracle matrix
cargo run -q -p otb-cli -- budget                # must end with TOTAL 955.98
docker build -t otb-candidate:dev docker/        # the sandbox image (slow first time)
export OTB_SANDBOX_IMAGE=$(docker image inspect otb-candidate:dev --format '{{.Id}}')
cargo nextest run -p otb-sandbox --run-ignored all   # sandbox isolation tests
cargo run -q --release -p otb-cli -- ref run     # reference checks in the sandbox
cargo run -q --release -p otb-cli -- selftest    # seeded faults: every expectation must hold
cargo run -q --release -p otb-cli -- check --task MLKEM-T07-compress-d --candidate my.c
```

`otb check` prints one line per oracle; add `--json` for canonical verdict
records. It runs candidates under Docker only.

`otb ref run --backend unshare` uses the weaker fallback (no filesystem
isolation, GAP-SBX-001); it is for harness development only.

Where to read next: `docs/PLAN.md` (status), `KNOWN_GAPS.md` (limits),
`docs/adr/` (design decisions), `AGENTS.md` (rules for AI agents).
