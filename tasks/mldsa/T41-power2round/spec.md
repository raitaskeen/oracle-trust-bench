+++
id = "MLDSA-T41-power2round"
title = "ML-DSA Power2Round"
level = "component"
parameter_set = "ML-DSA-65"
input_domain = "r in [0, q) with q = 8380417; d = 13"

[fips]
document = "FIPS 204"
algorithms = [{ number = 35, name = "Power2Round" }]
sections = ["7.4"]

[c]
function = "power2round"
signature = "void power2round(int32_t r, int32_t *r1, int32_t *r0);"

[secrets]
inputs = ["r: Power2Round is applied to t, which is computed from the secret vectors s1 and s2 in key generation"]
public_outputs = ["r1: t1 is part of the public key"]

[oracles.reference-intermediate]
applies = true
method = "Exhaustive: every r in [0, q) (8,380,417 cases), compared with mldsa-native's power2round (arguments mapped to this signature) and with the spec-literal implementation."
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
method = "r marked undefined before the call; r1 declassified after it. Built with the pinned compiler at -O2 and at -O3, no sanitizers."
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

# T41 ML-DSA Power2Round

Decomposes r into (r1, r0) with r = r1 * 2^d + r0 mod q and r0 in
(-2^(d-1), 2^(d-1)], d = 13 (FIPS 204 Algorithm 35; mod± as defined in
FIPS 204).
