# Known gaps

This file lists the known limits of the harness and its oracles. Each gap has
a stable ID. Code and documents cite gaps by ID, and a test
(`crates/otb-core/tests/known_gaps.rs`) fails if any cited ID is missing here.

Entries are never deleted. A closed gap stays, with its status set to
`closed` and the commit that closed it.

## GAP-CT-001 Valgrind cannot see variable-latency instructions

- Status: open
- Affects: `ct-valgrind`
- Memcheck reports branches and memory addresses that depend on undefined
  (secret-marked) data. It does not report instructions whose latency depends
  on their operands, such as integer division. The KyberSlash pattern can
  therefore pass `ct-valgrind`. The `ct-static-division` oracle covers this
  case.
- Measured in the pinned image with Valgrind 3.24.0 (M5a): a `divl` with
  secret operands and a `cmovs` on a secret condition are not reported; a
  secret-dependent branch and a secret-derived index are
  (`docs/notes/ct-measurements.md`). `crates/otb-harness/tests/ct_probes.rs`
  asserts this on every CI run, and seeded fault 1 asserts that the
  KyberSlash pattern passes `ct-valgrind`. Conditional moves are constant
  time on current processors, so the second point matters for
  interpretation, not for timing: whether a source-level `if` passes
  depends on what the compiler emits.

## GAP-CT-002 The static division check is conservative and target-specific

- Status: open
- Affects: `ct-static-division`
- The primary level checks emitted machine code for the pinned compilers
  and targets only (GCC 14.2 x86-64; Clang 19.1.7 x86-64, AArch64,
  Cortex-M4, Cortex-M0+; -O2 and -O3; ADR-005). Other compilers, flags or
  targets can add or remove division instructions; measurement 1 in
  `docs/notes/ct-measurements.md` shows the KyberSlash pattern survives
  only on Cortex-M0+.
- Operands are not traced. Every division reachable from the task function
  counts, including divisions of public values such as lengths, so the
  check can over-report for a candidate that divides public data in
  reachable code. On the five references it reports nothing
  (`docs/oracles/false-positives.md`).
- Reachability follows direct references by name and functions whose
  address appears in data. A function reached only through an address the
  compiler computes in another way would be missed; none of the references
  or seeded faults does this.
- The secondary level (LLVM IR at -O0) is a diagnostic only and never
  decides an outcome.

## GAP-CT-003 dudect results are statistical and noisy

- Status: open
- Affects: `ct-dudect`
- Timing tests depend on the machine, its load and the measurement budget.
  dudect runs only on a pre-registered subsample, never in default CI, and
  its results are reported separately from the other oracles.

## GAP-CT-004 ML-DSA signing time varies by design

- Status: open
- Affects: `ct-valgrind`, `ct-dudect` on ML-DSA tasks
- The signing loop rejects and retries, so its running time legitimately
  depends on data. The oracle matrix declassifies values that FIPS 204 lets
  an implementation reveal. A wrong declassification rule could hide a real
  leak or report a false one.

## GAP-CT-005 Cross targets are compiled against declaration-only headers

- Status: open
- Affects: `ct-static-division` on the AArch64, Cortex-M4 and Cortex-M0+
  configurations
- The image has a C library only for x86-64. For the other targets the
  implementation is compiled to assembly against Clang's own freestanding
  headers plus `c/harness/ct-include/` (declarations of the `string.h` and
  `stdlib.h` functions the admission policy allows). A candidate that needs
  a declaration those headers lack does not compile for that target; the
  oracle then reports the configuration as "not compiled" and decides on
  the others, so coverage for that candidate is smaller. The run record
  shows which configurations compiled.

## GAP-ACVP-001 ACVP covers top-level functions only

- Status: open
- Affects: `acvp`
- ACVP vectors test ML-KEM keyGen/encapsulation/decapsulation and ML-DSA
  keyGen/sigGen/sigVer. Component tasks (NTT, compression, rounding, hints)
  rely on the `reference-intermediate` oracle.

## GAP-REF-001 Reference trust rests on agreement, not on proof

- Status: open
- Affects: `reference-intermediate`
- Expected values come from mlkem-native and mldsa-native, and from a
  spec-literal Rust implementation drafted with AI assistance. A reference
  value is trusted only when these agree (and, where it applies, when ACVP
  agrees too). Agreement between implementations is evidence, not proof.

## GAP-SPEC-001 The per-call dk hash check in T24 is stricter than FIPS 203

- Status: open
- Affects: `malformed-input` on MLKEM-T24
- FIPS 203 section 7.3 allows the decapsulation-key check to be done once, or
  by another party. T24 requires it on every call. A candidate that skips it
  fails with class `task-contract`, which is not a FIPS violation. See
  `docs/notes/fips-verification.md`.

## GAP-ENV-001 No Linux sandbox on the author's Windows host yet

- Status: closed on 24 Nov 2026 (WSL2 Ubuntu 24.04 with Docker Engine; see
  `docs/notes/environment.md`)
- Affects: milestones M3 and later, when run locally
- WSL2 could not start until the machine was rebooted and virtualization
  was enabled. Both are now done.

## GAP-ENV-002 The candidate image is not bit-for-bit reproducible

- Status: open
- Affects: every oracle that compiles or runs C
- `docker/Dockerfile` pins the base image by digest and every compiler and
  tool package by exact version, so rebuilds install the same software.
  The resulting image ID still differs between builds (layer timestamps),
  and a pinned version can disappear from deb.debian.org, which makes the
  build fail. Before the pre-registration is filed, one image must be
  fixed by content digest and archived, and every run record must name it.

## GAP-ACVP-002 The ACVP vectors are published example sets with few cases

- Status: open
- Affects: `acvp`, and validation of the spec-literal reference
- The vendored files are the example vector sets in the ACVP-Server
  repository, not a validation session (`isSample` is true for
  ML-KEM-encapDecap and false for the other four files). For ML-KEM-768 they hold
  25 keyGen, 25 encapsulation, 10 decapsulation (5 with a modified
  ciphertext) and 10 decapsulation-key-check cases. Passing them is
  necessary evidence, not exhaustive evidence. The `malformed-input` and
  `implicit-rejection` oracles add seeded generated cases.

## GAP-ACVP-003 ML-DSA pre-hash ACVP groups are not run

- Status: open
- Affects: the ML-DSA-65 reference-build check in `otb ref run`
- HashML-DSA (FIPS 204 Algorithms 4 and 5) needs the pre-hash computed with
  the hash function each case names (SHA-2, SHA-3 and SHAKE variants).
  mldsa-native does not provide these hashes, so the pre-hash sigGen and
  sigVer groups are counted and reported as "not covered" (45 cases for
  ML-DSA-65). No pilot task uses HashML-DSA. The keyGen, internal,
  external-mu and external pure groups run in full.

## GAP-ADM-001 Candidate admission is a lexical policy, not a proof

- Status: open
- Affects: `otb-harness::admission` (ADR-004)
- The admission check works on tokens after line splicing, with comments
  and literals skipped. It rejects inline assembly, pragmas, constructor,
  destructor and ifunc attributes, token pasting, trigraphs, digraphs,
  universal character names, Valgrind identifiers and headers outside an
  allowlist. It does not parse C or run the preprocessor, so it cannot
  prove that a candidate never reaches an undesired construct by a route
  the rules do not name. Its purpose is to stop a candidate from tampering
  with in-process oracles (M5a); it is not the security boundary, which is
  the sandbox (ADR-003). How rejected candidates count in H1 must be fixed
  in the pre-registration.

## GAP-SBX-001 The unshare fallback does not isolate the filesystem

- Status: open
- Affects: `otb-sandbox` with `--backend unshare`
- The fallback runs the command in new user and network namespaces under
  `prlimit` limits. It blocks network access and bounds memory, processes
  and CPU time, but the command can read and write the host filesystem with
  the caller's permissions. Its backend name says so. Candidate code must
  run under the Docker backend; the fallback is for developing the harness.
