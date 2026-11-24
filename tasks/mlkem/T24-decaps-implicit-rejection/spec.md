+++
id = "MLKEM-T24-decaps-implicit-rejection"
title = "ML-KEM-768.Decaps with input checking and implicit rejection"
level = "top-level"
parameter_set = "ML-KEM-768"
input_domain = "ct of any length (valid length 1088), dk of any length (valid length 2400)"

[fips]
document = "FIPS 203"
algorithms = [
    { number = 21, name = "ML-KEM.Decaps" },
    { number = 18, name = "ML-KEM.Decaps_internal" },
]
sections = ["3.3", "6.3", "7.3"]
equations = ["7.2"]

[c]
function = "mlkem768_decaps"
signature = "int mlkem768_decaps(uint8_t ss[32], const uint8_t *ct, size_t ct_len, const uint8_t *dk, size_t dk_len);"
returns = "0 for every ciphertext of the correct length, whether decapsulation succeeds or implicitly rejects, and ss is always written. Nonzero only when an input check fails."

[secrets]
inputs = ["dk_PKE and z: the secret parts of dk = dk_PKE || ek || H(ek) || z (FIPS 203 Algorithm 19); ek and H(ek) are public"]
public_outputs = []

[oracles.reference-intermediate]
applies = false
reason = "Top-level function. Its outputs are checked by acvp, malformed-input and implicit-rejection."
[oracles.acvp]
applies = true
method = "ML-KEM-encapDecap-FIPS203 vectors at a pinned ACVP-Server commit: every ML-KEM-768 decapsulation test case, and every decapsulationKeyCheck test case. Counts reported by test group."
[oracles.malformed-input]
applies = true
method = "Wrong ct_len (FIPS 203 section 7.3 check 1, required on every call) and wrong dk_len (check 2) must return nonzero. Task contract, stricter than FIPS 203: the dk hash check (check 3, Eq. 7.2) must run on every call; a failure here is a task-contract violation, not a FIPS violation (GAP-SPEC-001)."
[oracles.implicit-rejection]
applies = true
method = "A modified ct must give ss = J(z || c), computed independently, and the same return value as the valid path (FIPS 203 section 6.3: the implicit-reject flag must not be returned in any form)."
[oracles.ct-valgrind]
applies = true
method = "dk_PKE and z marked undefined before the call (ek and H(ek) are public). Built with the pinned compiler at -O2 and at -O3, no sanitizers. Covers the ciphertext comparison and the selection of the shared secret."
[oracles.ct-static-division]
applies = true
method = "Primary: emitted machine code at -O2 and -O3 for the pinned targets. Secondary diagnostic: LLVM IR at -O0 (ADR-005)."
[oracles.ct-dudect]
applies = true
method = "Draft subsample member. Two input classes (valid ct versus modified ct), fixed measurement budget, threshold |t| > 10. Nightly only, reported separately (GAP-CT-003)."
[oracles.differential]
applies = false
reason = "Differential fuzzing belongs to Arm 3 (C-to-Rust migration), not Arm 1 tasks."
+++

# T24 ML-KEM-768.Decaps with input checking and implicit rejection

Core computation: FIPS 203 Algorithm 18. Input checks: FIPS 203 section 7.3
and Algorithm 21. See `docs/notes/fips-verification.md`, FIX-1 to FIX-3.
