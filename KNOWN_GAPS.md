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
  case. To be confirmed against the Valgrind documentation at M5.

## GAP-CT-002 The static division check is conservative and target-specific

- Status: open
- Affects: `ct-static-division`
- The primary level checks emitted machine code for one pinned compiler, at
  -O2 and -O3, on the pinned targets only. Other compilers, flags or targets
  can add or remove division instructions. The secondary source/IR level
  flags any division in the task function, including divisions on public
  data, so it over-reports. It is a diagnostic only (ADR-005).

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
