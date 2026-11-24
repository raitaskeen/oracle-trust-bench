+++
id = "MLDSA-T47-make-hint"
title = "ML-DSA MakeHint"
level = "component"
parameter_set = "ML-DSA-65"
input_domain = "z and r in [0, q) with q = 8380417; gamma2 = (q - 1) / 32"

[fips]
document = "FIPS 204"
algorithms = [
    { number = 39, name = "MakeHint" },
    { number = 37, name = "HighBits" },
    { number = 36, name = "Decompose" },
]
sections = ["7.4"]

[c]
function = "make_hint"
signature = "int make_hint(int32_t z, int32_t r, int32_t gamma2);"
returns = "1 if HighBits(r) != HighBits(r + z), else 0."

[secrets]
inputs = ["z and r: during signing they are derived from the secret vectors (FIPS 204 Algorithm 7)"]
public_outputs = ["the return value: accepted hints are published in the signature (GAP-CT-004)"]

[oracles.reference-intermediate]
applies = true
method = "Sampled and boundary inputs: 1,000,000 seeded random (z, r) pairs, plus every r within 2 of a HighBits boundary paired with z in {0, 1, q - 1, gamma2, q - gamma2}. Expected value: FIPS 204 Algorithm 39 computed as HighBits(r) != HighBits(r + z), with HighBits (Algorithm 37) taken from mldsa-native's decompose, and from the spec-literal implementation. mldsa-native's make_hint(a0, a1) is not compared directly."
[oracles.acvp]
applies = false
reason = "Component function. ACVP tests top-level functions only (GAP-ACVP-001)."
[oracles.malformed-input]
applies = false
reason = "Malformed-input tests apply to ML-KEM encapsulation and decapsulation only."
[oracles.implicit-rejection]
applies = false
reason = "Implicit rejection is a property of ML-KEM decapsulation only."
[oracles.ct-valgrind]
applies = true
method = "z and r marked undefined before the call; the return value declassified after it. Built with the pinned compiler at -O2 and at -O3, no sanitizers. Timing variation in the signing loop is expected and excluded by the matrix (GAP-CT-004)."
[oracles.ct-static-division]
applies = true
method = "Primary: emitted machine code at -O2 and -O3 for the pinned targets. Secondary diagnostic: LLVM IR at -O0 (ADR-005)."
[oracles.ct-dudect]
applies = false
reason = "Not in the draft dudect subsample (MLKEM-T07, MLKEM-T24). The subsample is fixed in the pre-registration."
[oracles.differential]
applies = false
reason = "Differential fuzzing belongs to Arm 3 (C-to-Rust migration), not Arm 1 tasks."
+++

# T47 ML-DSA MakeHint

MakeHint(z, r) = [[HighBits(r) != HighBits(r + z)]] for ML-DSA-65
(FIPS 204 Algorithm 39). gamma2 is passed for clarity and must equal
(q - 1) / 32.
