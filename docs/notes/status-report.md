# Status report and guide (after M3)

Written 24 Nov 2026 at commit `2472daa`. Every number below names the
command that produced it; judgements are marked as judgements.

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
| Reference run | Compiles the C references in the sandbox and checks them against the spec-literal Rust and ACVP | `otb ref run`, below |
| Budget | Port of `analysis/budget.py`; both print `TOTAL 955.98` | `otb budget`; CI step "Budget" |
| CI | fmt, clippy (pedantic, `-D warnings`), cargo-deny, cargo-audit, spec validation, budget check, tests with the network removed, sandbox job with Docker | `.github/workflows/ci.yml` (not yet run on GitHub: no remote) |
| Rules and records | AGENTS.md, CONTRIBUTING.md (review order), TESTING.md, PLAN.md, three ADRs, KNOWN_GAPS.md (12 entries; `grep -c '^## GAP-' KNOWN_GAPS.md`), AI_ASSISTANCE.md | the files themselves |

Size (`git ls-files` piped to `wc -l`): 4721 lines of Rust in six crates,
407 lines of C harness code, 23 Markdown files, 13 commits.

### `otb ref run`, Docker backend

| Check | Oracle | Result | Cases |
|---|---|---|---|
| MLDSA-T41 Power2Round | reference-intermediate | pass | 8,380,417 (every input in [0, q)) |
| MLDSA-T47 MakeHint | reference-intermediate | pass | 1,000,340 (340 boundary cases) |
| MLKEM-T01 NTT | reference-intermediate | pass | 1,258 |
| MLKEM-T07 Compress_d | reference-intermediate | pass | 16,645 |
| MLKEM-T24 decapsulation | acvp | pass | 20 (10 with a modified ciphertext or H) |
| ML-DSA-65 reference build | acvp | pass | 160 (45 pre-hash cases not covered, GAP-ACVP-003) |

Wall-clock: 16 s. Tests: `cargo nextest run --workspace --locked
--run-ignored all` with the image set, 68 run, 68 passed.

## 2. What it can do now, and what it cannot

It **can**:

- validate task specs against the FIPS algorithm tables and render the
  oracle matrix;
- compile any C file inside a locked-down container and run it on
  framed inputs;
- tell whether a C implementation of the five component tasks agrees with
  three independent sources (spec-literal Rust, the C reference, ACVP);
- prove the sandbox blocks the network and kills runaway jobs.

It **cannot yet**:

- judge a model-written candidate end to end: candidate admission and the
  verdict record come in M4;
- check malformed-input handling or implicit rejection beyond ACVP (M4);
- check constant-time behaviour: Valgrind and emitted-code division
  checks (M5a, ADR-005);
- run model-written test suites or mutants (M5b);
- run the self-test, the determinism check at 1/4/16 jobs, the statistics
  or the power grid (M6);
- serve as a public benchmark: no README, PREREGISTRATION.md, THREAT_MODEL
  or mdBook yet (M7), and no independent review (M8).

## 3. Rating (judgement)

Rated as a research artifact, which is what the application promises;
it is not software for sale.

| Aspect | Now | Reason |
|---|---|---|
| Engineering foundation | 8/10 | Pinned toolchain, image and actions; strict lints; sandbox tested; three-way reference agreement; every gap recorded. Loses points because CI has never run on GitHub and the image is not yet fixed by digest (GAP-ENV-002). |
| Correctness evidence | 7/10 | Exhaustive or near-exhaustive checks where the domain allows; ACVP agreement. ACVP sets are small example sets (GAP-ACVP-002), and the reference is trusted by agreement, not proof (GAP-REF-001). |
| Completeness against the plan | 3/10 | M0-M3 of M0-M8 done. The oracles that carry the study's main claim (malformed input, implicit rejection, constant time) are not built. |
| Documentation | 6/10 | Internal docs are thorough; the public-facing README and pre-registration do not exist yet. |
| **Overall as a finished benchmark today** | **4/10** | A strong base, not yet usable to judge a candidate. |
| Expected at M8 if the plan holds | 8/10 | Assuming M4-M7 land as planned and the independent review finds nothing structural. |

## 4. Remaining work and time (estimates)

Estimates are in working sessions like this one and assume no blocking
environment problems. M3 took longer than estimated because of the
Debian mirror: the image build took 6775 s (about 1 h 53 min), of which
apt reported "Fetched 152 MB in 1h 34min 48s (26.7 kB/s)".

| Milestone | Work | Estimate |
|---|---|---|
| M4 | Candidate admission and verdict record; malformed-input and implicit-rejection oracles; seeded faults 2-7 | 1-2 sessions |
| M5a | ADR-005; Valgrind secret-marking oracle; emitted-code division check at -O2 and -O3; fault 1; false-positive table | 2 sessions |
| M5b | Runner for model-written test suites; mutants; fault 8 | 1-2 sessions |
| M6 | `otb selftest`; determinism at 1/4/16 jobs; statistics; `otb power` grid | 1-2 sessions |
| M7 | README, PREREGISTRATION.md (draft), THREAT_MODEL, mdBook | 1 session |
| M8 | Independent review by someone other than the author and Claude | depends on the reviewer |

## 5. Publishing on GitHub

Nothing has been pushed. The repository has no remote; it exists only
in WSL at `~/projects/oracle-trust-bench`. Claude does not push, create
remotes or publish; you do.

### When

1. **Now, as a private repository**: safe, and it gives you an off-machine
   backup and a first real CI run. Recommended.
2. **Public**: after M7, when the README states "No model has been run;
   no results yet" and PREREGISTRATION.md exists. It must be public, with
   the pre-registration timestamped, **before** any model is run, because
   that is what makes the study pre-registered.

### Before the first push

1. Find your numeric GitHub ID (open `https://api.github.com/users/raitaskeen`
   in a browser and read `"id"`), and tell Claude. Every commit is currently
   authored as `raitaskeen@users.noreply.github.com`, without the ID; Claude
   will rewrite all unpushed commits to `ID+raitaskeen@users.noreply.github.com`.
2. Check that nothing private is tracked: `git status --short` should be
   empty and `git ls-files inputs` should print nothing.

### Commands (in the WSL terminal)

Create an empty **private** repository named `oracle-trust-bench` on
github.com (no README, licence or .gitignore, so histories do not clash),
then:

```sh
cd ~/projects/oracle-trust-bench
git remote add origin https://github.com/raitaskeen/oracle-trust-bench.git
git push -u origin main
```

GitHub asks for a username and a personal access token (not your
password) on the first push; create the token under Settings, Developer
settings, Personal access tokens, with access to this one repository.
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
```

`otb ref run --backend unshare` uses the weaker fallback (no filesystem
isolation, GAP-SBX-001); it is for harness development only.

Where to read next: `docs/PLAN.md` (status), `KNOWN_GAPS.md` (limits),
`docs/adr/` (design decisions), `AGENTS.md` (rules for AI agents).
