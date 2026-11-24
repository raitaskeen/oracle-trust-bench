# 00 — Understanding, verification and build plan

Status: written before any code. No model has been run. No results yet.
Date: 24 Nov 2026. Drafted with AI assistance (Claude); to be reviewed by the author.

This note covers six things:

1. The study as I understand it.
2. What to reuse from the two platform reports, and what to leave out.
3. Facts checked against primary sources.
4. A critique of the build prompt, with changes I propose.
5. The build plan (M0–M8), risks and open questions.
6. The lessons carried over from earlier work.

---

## 1. The study in my own words

**Question.** Suppose a language model writes post-quantum cryptographic C code and then, in a fresh context, writes the tests for that code. If those tests pass, should we trust the code? The study measures how often they should not. That case is "false assurance": a component passes its own model-written tests but fails an independent, non-LLM check that applies to its task.

**Arms** (from `application_final.md`):

| Arm | Status | What happens | Size |
|---|---|---|---|
| 1 Generation | core | 3 model tiers write 60 C components (36 ML-KEM, 24 ML-DSA) under 3 prompt conditions (plain; security-focused; spec plus checklist), 5 samples per cell | 2,700 implementations |
| 2 Oracle trust | core | Each model writes a test suite for each of its own implementations in a fresh context. A suite counts only if it has at least 5 assertions and kills at least 1 mutant of the reference. Every implementation then goes through the oracles that apply to its task. Cross-tier audits are added, and no verdict depends on an LLM | 2,700 suites + 2,450 audits |
| 3 Migration | secondary | 40 small crypto-adjacent open-source C routines. Models write characterization tests, then migrate each routine to Rust. Differential fuzzing (fixed budget) and seeded faults follow; drift is classified, with a second coder on 20% and Cohen's kappa | 1,200 calls |

**Hypotheses** (quoted in substance; the README will quote them exactly):

- **H1, false assurance.** Among implementations that pass their own model-written suite, the share failing at least one applicable oracle is above 5%. H1 is supported if the lower bound of the task-clustered 95% CI is above 5%. The expected rate is about 15%.
- **H2, prompt robustness.** Security-focused prompts change the rate of constant-time and implicit-rejection violations by less than 5 pp compared with plain prompts. The test is TOST at ±5 pp.
- **H3, silent drift.** For C-to-Rust migrations, the pass rate under model-written characterization tests is at least 10 pp higher than the pass rate under differential fuzzing.

**Task-by-oracle matrix.** This is pre-registered and static: an oracle either applies to a task or is N/A with a written reason. The oracles are:

- reference intermediate values (mlkem-native / mldsa-native);
- ACVP vectors (top-level functions only);
- malformed-input tests and implicit-rejection tests (encaps and decaps only);
- constant-time checks (secret-dependent routines only): Valgrind at -O2 and -O3, a static division check, and dudect on a pre-registered subsample.

False assurance is computed over applicable oracles only.

**Analysis.** Wilson 95% intervals; mixed-effects logistic regression (task random effect; tier and prompt fixed effects); TOST for H2; Holm correction across H1–H3. A simulation-based power analysis with an assumed ICC is filed on OSF before the main run. Null results are reported.

**Budget.** The request is $955.98 at Standard list prices fetched 24 Nov 2026, with no discounts: 9,050 calls, 42.45M input and 42.45M output tokens. I recomputed the B-full table by hand:

- Subtotal: 869.075.
- Pilot plus reserve (10%): 86.9075.
- Total: 955.9825, which rounds to **955.98**.
- By arm: Arm 1 304.515, Arm 2 457.50, Arm 3 107.06.
- By model: astra 651.20, sol 211.10, luna 6.775.

Every figure matches the application. **Rounding order matters.** Rounding line items first gives a subtotal of 869.08, and 10% of that rounds to 86.91, which makes the total 955.99. `otb budget` must therefore use exact decimal (integer micro-dollar) arithmetic and round only for display. `budget.py` is not among the inputs (open question Q2).

**Timeline.**

| When | Work | Spend |
|---|---|---|
| Before credits | This harness, specs, OSF draft | $0 |
| Month 1 | Freeze tasks and prompts; pilot on 5 tasks; file the pre-registration | $43.45 |
| Months 2–3 | Arm 1 | $304.52 |
| Months 4–5 | Arm 2 | $457.50 |
| Months 6–7 | Arm 3 | $107.06 |
| Month 8 | Analysis and preprint | $43.45 |
| Month 9 | v1.0 release | — |
| Months 10–12 | Buffer | — |

**This repository now** is the verification side only:

- task specs;
- the independent oracles;
- the sandboxed harness;
- documentation;
- 5 pilot tasks.

It has no model calls, no API keys, no results and no product.

---

## 2. What to reuse from the two reports, and what to leave out

The FINAL report (Edition 2) replaces the MASTER report. The MASTER report adds nothing research-relevant that FINAL lacks.

**Reuse (engineering and research knowledge only):**

- FIPS names and sizes. Re-verified in §3 from the FIPS texts, not from the report table.
- KyberSlash and CVE-2024-37880 as constant-time lessons (§3).
- Related work: "Nothing Breaks" (2609.07849) and "Can Coding Agents Migrate to Post-Quantum Cryptography?" (2512.12989), verified in §3.
- Determinism: byte-identical output at any worker count (report rule E-5).
- The fact-diff practice. Here it becomes a verdict diff between runs.
- A known-gaps ledger that tests keep in sync with the code (rule E-12).
- The hostile-input threat model. Here the hostile input is the candidate C code, not a repository.
- CI hardening: actions pinned by SHA, read-only permissions, no secrets, an offline test step, and no timing assertions on shared runners (E-8, E-13, §26.3).
- "Refuse rather than guess", and "every uncertainty is a visible state" (principles 4 and 6). Here these become `Error{reason}` and `NotApplicable`, never a silent pass.
- Quality tools: clippy -D warnings, nextest, proptest, insta, cargo-deny/audit, cargo-fuzz.
- "Rule of two" for abstractions (O-1); one-page ADRs (O-4).

**Leave out entirely:** market, competitors, funding, pricing, packaging, brand names, go-to-market, CISO trust ladder, revenue, success odds, legal-entity advice, the platform architecture (extractors, CBOM, remediation classes, knowledge packs, MCP server), and all deadline and policy material (IR 8547, EO 14412 and so on). None of it is needed for the study.

**Do not commit the two reports.** They hold business content and internal history from another project, and this repo will be public. `inputs/` will be in `.gitignore`. See Q6 for `application_final.md`, which contains the advisor's email address.

---

## 3. Facts checked against primary sources

How each fact was checked:

- **FIPS 203 and FIPS 204:** read from the NIST PDFs (nvlpubs.nist.gov), converted with `pdftotext -layout`.
  - FIPS 203 PDF sha256: `fe1f12f32a7e44ec9fdebbf400cda843a40b506dee676725234dc6f7923b6cac`
  - FIPS 204 PDF sha256: `57239b9f84c03227eda3ca0991204dc7764c79af9ce2e6824eda774918d46b6b`
- **ACVP, mlkem-native, mldsa-native:** read from their GitHub pages and the NIST ACVP draft specs.
- **arXiv papers:** read from the abstract pages.

FIPS 205 is not used by any pilot task, so I did not check it.

Result codes:

- **OK:** matches the source.
- **FIX:** the source differs from the input.
- **PART:** partly verified.
- **NO:** could not verify, so I will not rely on it.

### 3.1 FIPS 203 (ML-KEM)

| Fact | Result |
|---|---|
| NTT = Algorithm 9, NTT⁻¹ = Alg 10. Forward NTT: 7 layers (len 128→2), zeta = BitRev7(i), i from 1, no final scaling | OK |
| Compress_d = Eq. (4.7), Decompress_d = Eq. (4.8), defined for d < 12; "Floating-point computations shall not be used" | OK |
| Decaps_internal = Alg 18, KeyGen_internal = 16, Encaps_internal = 17, ML-KEM.Decaps = Alg 21, Encaps = 20, KeyGen = 19 | OK |
| Input checking belongs to ML-KEM.Encaps and ML-KEM.Decaps (§3.3: they "require input checking"; §7.3). Alg 18 takes plain inputs, while Alg 21 takes "Checked input". (Correction: an earlier draft of this note cited a §5 sentence about K-PKE here; that sentence is not about Alg 18.) | OK. This leads to **FIX-1** in §4 |
| §7.2 encaps key check: (1) type check, length 384k+32; (2) modulus check, ByteEncode12(ByteDecode12(ek[0:384k])) = ek[0:384k] (Eq. 7.1) | OK |
| §7.3 decaps check: (1) ciphertext type check, 32(du·k+dv); (2) dk type check, 768k+96; (3) hash check, H(dk[384k:768k+32]) = dk[768k+32:768k+64] (Eq. 7.2) | OK |
| §7.3: dk checking "need not be performed by the decapsulating party, nor with every execution"; "Ciphertext checking shall be performed with every execution" | OK. This leads to **FIX-2** |
| Alg 18 text: the implicit-reject flag "shall be destroyed"; "returning the value of the flag as an output in any form is not permitted" | OK. This leads to **FIX-3** |
| Implicit rejection: K̄ = J(z‖c) at Alg 18 line 7, z = dk[768k+64:768k+96] | OK. The J definition will be re-read at M4 |
| ML-KEM-768: k=3, η1=2, η2=2, du=10, dv=4; ek 1184, dk 2400, ct 1088, ss 32 | OK |
| §1 rule: Encaps/Decaps "require input checking" | OK |

### 3.2 FIPS 204 (ML-DSA)

| Fact | Result |
|---|---|
| Power2Round = Alg 35, Decompose = 36, HighBits = 37, LowBits = 38, MakeHint = 39, UseHint = 40, NTT = 41 | OK |
| Power2Round: r⁺ = r mod⁺ q; r0 = r⁺ mod± 2^d; return ((r⁺−r0)/2^d, r0). mod± range is −α/2 < m′ ≤ α/2, so r0 ∈ (−2^12, 2^12] | OK |
| MakeHint(z, r): r1 = HighBits(r), v1 = HighBits(r+z), return [r1 ≠ v1] | OK |
| q = 8380417, d = 13, γ2 = (q−1)/32 for ML-DSA-65 | OK |
| ML-DSA-65: pk 1952, sk 4032, signature 3309 bytes | OK |
| Deterministic signing leaks information about the message (not the key) through timing | OK. This informs the CT declassification rules |

### 3.3 Reference code and vectors

| Fact | Result |
|---|---|
| mlkem-native: Apache-2.0 OR ISC OR MIT; latest tag v2.0.0 (7 Aug 2026, d1b2fe7) | PART. Tag and SPDX seen; full SHA to be pinned at M3 |
| mldsa-native: same triple licence; v2.0.0 (6 Aug 2026, 834a90d) | PART. Same as above |
| mlkem-native supports single-CU builds and `MLK_CONFIG_INTERNAL_API_QUALIFIER`, `MLK_CONFIG_NAMESPACE_PREFIX` | OK |
| mlkem-native has `mlk_scalar_compress_d{1,4,5,10,11}` (static inline) and `mlk_poly_ntt`; forward NTT output bound is \|x\| < 8q | OK. Outputs must be normalised mod q |
| mlkem-native native backends may use a custom NTT order (`mlk_poly_permute_bitrev_to_custom`, `MLK_USE_NATIVE_NTT_CUSTOM_ORDER`) | OK. The reference build must use the portable C backend only |
| mldsa-native `mld_power2round(int32_t *a0, int32_t *a1, int32_t a)`, a0 ∈ (−2^(D−1), 2^(D−1)] | OK |
| mldsa-native `mld_make_hint(a0, a1)` is "not a line-for-line implementation of FIPS 204's MakeHint(z, r)" | OK. This leads to **FIX-4** |
| mlkem-native has `MLK_CONFIG_CT_TESTING_ENABLED`, using Valgrind `MAKE_MEM_UNDEFINED/DEFINED` | OK. Equivalent mldsa-native switch not found, so it will be checked at M5 |
| Both libraries are tested against all ACVP vectors (`test/acvp`) and Wycheproof | OK |
| ACVP folders `ML-KEM-encapDecap-FIPS203` and `ML-DSA-sigGen-FIPS204`, each with prompt / expectedResults / internalProjection / registration / validation JSON | OK |
| ACVP folders `ML-KEM-keyGen-FIPS203`, `ML-DSA-keyGen-FIPS204`, `ML-DSA-sigVer-FIPS204` | NO, not yet seen. To check at M3 before use |
| ACVP ML-KEM functions: encapsulation, decapsulation, encapsulationKeyCheck, decapsulationKeyCheck. Decaps VAL vectors include "randomly modified" ciphertexts on the implicit-rejection path; the dk check includes "modified H" | OK. Official vectors cover part of the implicit-rejection and hash-check oracles |
| ACVP seed-format field name (`d`/`z` vs `seed`) is inconsistent inside the spec | OK, the inconsistency is confirmed. The parser must be tested on real files |
| Latest commit touching `gen-val/json-files`: a7f283cdc87d2d6dd93c1bac59e5622c5f9f8324 (31 Jul 2026) | PART. This is not the branch head; pin at M3 |
| ACVP-Server licence: NIST public-software notice, no SPDX identifier | OK. NOTICE.md will quote the notice |
| dudect: MIT. Its README says both "t values larger than 5" (very likely leakage) and "If it surpasses 10" (most likely a leak) | OK. Our threshold of \|t\| > 10 is a stated choice, not "the" convention |

### 3.4 Implementation attacks and related work

| Fact | Result |
|---|---|
| KyberSlash: secret-dependent division by KYBER_Q. KyberSlash1 is in `poly_tomsg`; KyberSlash2 is in `poly_compress`/`polyvec_compress`. Whether a `div` is emitted depends on compiler and flags: the authors show `gcc -Os` on a Raspberry Pi 2 calling a branching `__divsi3`. TCHES 2025(2). Fix: multiply by a precomputed reciprocal | PART, from the KyberSlash FAQ, the pqc-forum and ePrint 2024/1049 as summarised by search. kyberslash.cr.yp.to did not load. Reading the paper is a task for M5 |
| CVE-2024-37880: Kyber reference `poly_frommsg` in `ref/poly.c` before commit 9b8d306; Clang can emit a secret-dependent branch (CWE-203) | PART. Consistent across OSV, Debian and NVD mirrors; the NVD page did not render. Reading the fix commit is a task for M5 |
| 2609.07849 "Nothing Breaks" (Y. Han, 7 Sep 2026): removing hybrid KEX passes validation and tests; agents downgraded in 40/40 episodes from engineering prose vs 0/40 on a neutral document | OK |
| 2512.12989 "Can Coding Agents Migrate to Post-Quantum Cryptography?" (A. Alquwayfili, v3 20 Sep 2026): 160 attempts, 12 patches pass local checks but fail external requirements; checker access does not raise completion | OK. The report's "four trials with frontier models passed all 40 checks" was not seen, so I will not use it |
| 2611.25339 (Park et al., 19 Nov 2026): accelerator passes 85 of 285 ACVP cases, the rest interface-inapplicable; simulation gates kill 76% of 399 mutants | OK |
| 2602.01438 CIPHER: pooled neutral 74.2% vs secure 71.7% (Table 4, v2) | OK |
| 2610.11835: 8–17% fault-enforcing vs 2.4–4.8% fault-revealing | OK |

Facts I will **not** use until verified: the extra 2512.12989 detail above, the three unseen ACVP folder names, and the exact compiler versions in CVE-2024-37880.

---

## 4. Critique of the build prompt, and proposed changes

Primary standards win. Items marked **FIX** are conflicts with FIPS 203 or FIPS 204; the rest improve soundness, scope or honesty.

### 4.1 Conflicts with the standards

- **FIX-1: T24 cites the wrong algorithm.** Its title is Alg 18 (Decaps_internal), but Alg 18 takes unchecked inputs; input checking is assigned to ML-KEM.Decaps (§3.3, §7.3). The malformed-input checks belong to **ML-KEM.Decaps (Alg 21) with §7.3**. Rename T24 to "ML-KEM-768.Decaps (Alg 21, §7.3; core Alg 18)".
- **FIX-2: skipping the dk hash check per call is not by itself non-compliant.** FIPS 203 §7.3 allows the dk check to be done once, or by another party. So seeded fault 3 ("missing the dk hash check") breaks our **task contract**, not FIPS. The T24 spec will require a per-call dk check (stricter than FIPS). The oracle will label that failure `TaskContract`, and the paper must say so. The ciphertext length check, by contrast, is FIPS-mandatory on every call.
- **FIX-3: the return value must not reveal implicit rejection.** The draft signature returns `int`. The contract will be: return 0 for both valid and modified ciphertexts; return nonzero only when an input check fails; always write `ss` on return 0. Any return value that differs between the valid path and the implicit-rejection path fails the oracle, citing the Alg 18 text in §3.1.
- **FIX-4: the T47 reference needs an adapter.** mldsa-native's `make_hint` takes decomposed inputs, so it cannot be compared to `make_hint(z, r)` directly. The reference shim will compute FIPS MakeHint as `HighBits(r) != HighBits(r+z)` using `mld_decompose`. The adapter is checked exhaustively against our spec-literal implementation (§4.2.1).
- **Minor.** For T41, `mld_power2round(a0*, a1*, a)` has a different argument order from the task; the shim maps it. For T07, mlkem-native takes `int16_t u`; the task takes `uint16_t x`; the domain is [0, q) in both.

### 4.2 Soundness of the oracles

1. **mlkem-native and mldsa-native are a single point of trust.** Add a small, FIPS-literal Rust implementation of each pilot function (straight transcription of the pseudocode, no optimisation; SHA-3 from a vetted crate). References are validated only when the spec-literal code, the C reference and ACVP (where it applies) all agree. This catches shim bugs, such as NTT normalisation or argument order, which would otherwise look like candidate failures. It also gives the reference oracle two real backends, so the abstraction meets the rule of two. Honest caveat: I would write the spec-literal code with AI assistance. Its trust comes from exhaustive agreement with mlkem-native/mldsa-native and ACVP, not from authorship.
2. **The static division check at -O2/-O3 on x86-64 will usually miss KyberSlash.** Compilers turn division by the constant q into a multiply. This is the central constant-time claim, so I propose two reported levels:
   - **(a) Source level:** LLVM IR at -O0. Flag `udiv/sdiv/urem/srem` in the task function. This is the KyberSlash *pattern*, conservative and compiler-independent.
   - **(b) Emitted level:** assembly at -O2, -O3 and -Os for x86-64, plus compile-only cross-compilation for `thumbv7m` (Cortex-M3) and aarch64. Flag div instructions and calls to `__aeabi_*div`, `__divsi3`, `__udivsi3`.

   Which level counts toward H1 must be pre-registered (Q3).
3. **Valgrind memcheck does not flag variable-latency instructions** (from my knowledge; to be confirmed against the Valgrind and TIMECOP docs at M5). So seeded fault 1 is expected to **pass** Valgrind. `otb selftest` will assert every documented outcome, blind spots included (`expected = pass, gap = GAP-CT-001`), not just "the expected oracle fails".
4. **A candidate could evade Valgrind.** It runs in the same process as the poisoning calls, so it could undo them with client requests or inline asm. Add a **candidate admission check** in Rust, before compiling:
   - C11 only;
   - an allowlist of headers;
   - no inline asm, `#pragma`, constructor/destructor attributes, or `valgrind` includes;
   - a size limit.

   A rejected candidate gets a distinct `Rejected{policy}` verdict, and its treatment in H1 is pre-registered.
5. **Expected values never enter the candidate's sandbox.** Reference outputs are produced in a separate sandbox run, hashed into `run.json`, and compared in Rust. The candidate's process only ever sees inputs.
6. **Exhaustive domains are large.** Power2Round has 8,380,417 inputs, so JSON-lines would be slow and bulky. Use JSON-lines for control and fixed-width little-endian binary records for bulk vectors, behind one documented frame format.
7. **Declassification is part of the spec.** Each TaskSpec lists its secret inputs and its public outputs (for example, r1 of Power2Round is part of the public key). Without this, Valgrind gives false positives on correct code. Each oracle's false-positive table (M5) is measured on the references.
8. **Seeded randomness.** Sampled cases use ChaCha20 with a seed derived from (task ID, oracle, purpose), recorded in `run.json`.

### 4.3 Missing pieces the study needs

1. **The self-test runner is missing.** H1's denominator is "passes its own model-written suite". The harness needs:
   - a fixed test-file ABI: a C file with `main` that uses a harness-supplied `OTB_ASSERT`, so assertions are counted at run time;
   - a runner that executes a suite against the candidate, the reference and a pre-registered **mutant set** per task.

   The validity rules come from the application: at least 5 assertions, kills at least 1 mutant. I add one more: a suite that fails on the verified reference is invalid. This is new work (milestone M5b). Mutants, which validate test suites, are kept apart from seeded faults, which validate oracles.
2. **Outcome taxonomy.** The draft says `Error{reason}` but not whose error it is. Split it two ways:
   - `HarnessError`: rerun it, and never count it.
   - Candidate-caused failures (crash, timeout, wrong output): these are `Fail` with an evidence class.

   Also pre-register how compile failures are handled.
3. **Statistics.**
   - Wilson intervals ignore clustering, so they are descriptive only. The H1 decision uses the task-cluster bootstrap that the application names.
   - With 60 clusters and rates near 5–15%, percentile intervals can under-cover. The power simulation should also report coverage, and BCa should be considered.
   - The power simulation needs an assumed self-test pass rate (the denominator) and an ICC on the logit scale (Q4).
4. **Pilot ACVP scope.** Of the 5 pilot tasks, only T24 is top-level. ACVP for keyGen, sigGen and sigVer serves to validate the reference build, not as a pilot oracle. The matrix will say so.

### 4.4 Architecture and engineering

- **Sandbox moves earlier, from M5 to M3.** M3 already compiles and runs C. All C, references included, runs through the one sandbox path.
- **Merge `otb-spec` into `otb-core`.** Spec loading is plain serde over the core types, so a separate crate fails the rule of two. That gives 7 crates. The budget is a module in `otb-stats`.
- **Canonical JSON without floats.** Verdict and run files contain no floats: rates are integer pairs, money is integer micro-dollars. That makes RFC 8785 canonicalisation small and auditable. A small writer will be tested against the RFC's published examples instead of depending on a little-used crate.
- **CandidateId normalisation stays minimal.** Decode UTF-8, convert CRLF to LF, ensure exactly one trailing newline, then SHA-256. Anything more could merge two different programs into one ID. Specified in ADR-002.
- **Determinism details:**
  - `BTreeMap` only;
  - output sorted by (task, candidate, oracle, case);
  - evidence uses case indices and counts, never raw Valgrind logs (PIDs, addresses). Raw logs are kept as unhashed artifacts.
- **Vectors are vendored, not submoduled.** Vendor only the needed ACVP JSON files, with the source commit and per-file sha256 in `NOTICE.md`, because the ACVP-Server repo is large. mlkem-native, mldsa-native and dudect are submodules pinned by full SHA.
- **Container image:** a Dockerfile on a dated `snapshot.debian.org` base, pinned by digest. The compiler `--version` strings go into the toolchain digest.

### 4.5 On "saleable"

The application says this is "an evaluation study ... not software for sale". The conflict-of-interest section promises the company no early or exclusive access. Commercial framing here would contradict both and weaken the grant. I read "saleable" as **professional and reusable**: clean architecture, tests, documentation and an MIT licence, which anyone (the author included) may later build on in the open. The repo will carry no product language. Please confirm (Q1).

---

## 5. Build plan

Each milestone ends with a commit and a summary to you.

| # | Milestone | Done when |
|---|---|---|
| **M0** | Environment: Linux toolchain decision (Q5); `rust-toolchain.toml`; Dockerfile drafted | `cargo --version` works and the image builds (or CI-only mode is agreed) |
| **M1** | Workspace (7 crates), CI skeleton pinned by SHA, `otb-core` types and outcome taxonomy, canonical JSON plus RFC 8785 examples, `.gitattributes` (LF), LICENSE / LICENSE-DATA / NOTICE, `KNOWN_GAPS.md` plus the gap-ID test | fmt, clippy -D warnings and nextest green; workflow YAML valid |
| **M2** | Spec format (TOML front matter plus Markdown); 5 pilot specs with FIX-1 to FIX-4 applied; spec-literal Rust functions with exhaustive self-checks; `docs/notes/fips-verification.md` | `otb tasks validate` passes; every algorithm, section and equation number is cited with the check recorded |
| **M3** | Sandbox (Docker/Podman plus the weaker Linux fallback), no-network test; submodules; C shims; ACVP parser plus oracle | `otb ref run` passes all applicable ACVP cases for ML-KEM-768 and ML-DSA-65, with counts by group; reference agrees with spec-literal |
| **M4** | Reference-intermediate (exhaustive and sampled), malformed-input, implicit-rejection oracles; admission check; seeded faults 2–7 | References pass; faults 2–7 caught as documented |
| **M5a** | CT oracles: Valgrind at -O2/-O3 with declassification; static division at both levels; dudect harness (nightly only); fault 1; false-positive table | Fault 1 caught at the levels that should catch it; documented blind spots asserted; references clean |
| **M5b** | Self-test runner (test ABI, assertion counting, mutant sets, validity rules); fault 8 (weak "model-style" suite, hand-written, labelled synthetic) | Fault 8 shows false assurance in miniature |
| **M6** | `otb selftest`; determinism at 1/4/16 jobs; stats (Wilson, cluster bootstrap, TOST, Holm) checked against cited values; `otb budget`; `otb power` | 100% of documented outcomes hold; verdicts byte-identical; budget = 955.98 |
| **M7** | Docs (README, PREREGISTRATION draft, ORACLE_MATRIX, THREAT_MODEL, ADR-001 to 005, policies); mdBook | Every number names its command |
| **M8** | Independent review by a fresh agent (cryptographer and grant reviewer) | `docs/notes/review-M8.md` before/after table; fixes merged |

**Stats reference values.** No R or SciPy is installed here. Test values will come from published worked examples with citations, for example Newcombe (1998) for Wilson and Schuirmann (1987) for TOST, or from closed-form derivations shown in the test. I will not invent reference numbers.

### Risks

| Risk | Mitigation |
|---|---|
| No Linux, Docker, compiler or Valgrind on this machine | Q5: WSL2 plus Docker or Podman, or CI-only for M3+ |
| Oracle false positives on correct code (CT declassification) | Measure on references at M5; fix the spec, never the expected values |
| Static division rule misread as proof of constant time | Two levels, documented limits, GAP entries |
| A shim bug looks like a candidate failure | Spec-literal agreement gate (§4.2.1) |
| Scope creep from 8 oracles and 2 libraries | Pilot only: Differential stays a stub; dudect is nightly only |
| Unpinned or unverified ACVP folder names | Verify at M3 before use |
| Overstated claims in docs | M8 review plus the "every number names its command" rule |

### Open questions (I need your answers)

- **Q1.** Do you agree that "saleable" means professional and reusable, with no commercial framing in this repo (§4.5)?
- **Q2.** `budget.py` is not in the zip. Should I send it, or rebuild the budget from the B-full table as a data file (a port "with the same numbers")?
- **Q3.** For H1, which static-division level counts as a constant-time failure: (a) source/IR, (b) emitted code, or both reported with (a) primary?
- **Q4.** For the power simulation: what self-test pass rate should be assumed (for example 50%, 70% or 90%), and a false-assurance rate of 15% under H1?
- **Q5.** This machine has no Rust, C compiler, Docker, WSL or Python. Do you want to:
  - (a) install WSL2 (Ubuntu) and Docker Desktop or Podman yourself;
  - (b) let me install rustup for Windows now and do M1–M2 here, with M3+ in CI or later on Linux; or
  - (c) something else?
- **Q6.** May `application_final.md` be committed publicly? It contains the advisor's email address. I suggest committing a version with the email removed, and keeping both reports out of git.
- **Q7.** Do you accept FIX-1 to FIX-4 (spec changes forced by FIPS 203 and FIPS 204), and the new self-test runner milestone (M5b)?

---

## 6. Lessons carried over (knowledge only; no code reused)

- Determinism checked by bytes at several worker counts.
- Stable IDs from the first commit.
- A known-gaps ledger that tests enforce.
- Measure before believing: no performance or accuracy claim without a command.
- Review must be shown to have happened (the M8 review is recorded).
- Every crate and tool lives inside the checked workspace.
- No timing assertions on shared CI runners: dudect runs nightly only and is reported separately.
- One line-ending policy from the first commit.
- Few documents, kept in one place.
