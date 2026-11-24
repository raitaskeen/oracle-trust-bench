+++
id = "MLKEM-T01-ntt"
title = "ML-KEM NTT"
level = "component"
parameter_set = "ML-KEM-768"
input_domain = "256 coefficients, each in [0, q) with q = 3329"

[fips]
document = "FIPS 203"
algorithms = [{ number = 9, name = "NTT" }]
sections = ["4.3"]

[c]
function = "ntt"
signature = "void ntt(int16_t f[256]);"

[secrets]
inputs = ["f: the NTT is applied to secret vectors in K-PKE.KeyGen and to secret-dependent values in decapsulation"]

[oracles.reference-intermediate]
applies = true
method = "All 256 unit vectors (the NTT is linear, so these fix it completely), the all-zero and all-(q-1) inputs, and 1,000 seeded random inputs. Outputs are reduced mod q and compared with mlkem-native (portable C backend) and with the spec-literal implementation."
[oracles.acvp]
applies = false
reason = "Component function. ACVP tests top-level functions only (GAP-ACVP-001)."
[oracles.malformed-input]
applies = false
reason = "Component function with a fixed-size array input. Malformed-input tests apply to ML-KEM encapsulation and decapsulation only."
[oracles.implicit-rejection]
applies = false
reason = "Implicit rejection is a property of ML-KEM decapsulation only."
[oracles.ct-valgrind]
applies = true
method = "f marked undefined before the call. Built with the pinned compiler at -O2 and at -O3, no sanitizers."
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

# T01 ML-KEM NTT

In place. Coefficients in Z_q, q = 3329, zeta = 17. The output is the NTT
representation with coefficients in [0, q).

Notes for the prompt summary (to be written and frozen in Month 1): zetas in
bit-reversed order (BitRev7), 7 layers, no final scaling (FIPS 203
Algorithm 9).
