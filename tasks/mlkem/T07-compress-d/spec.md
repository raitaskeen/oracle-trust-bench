+++
id = "MLKEM-T07-compress-d"
title = "ML-KEM Compress_d"
level = "component"
parameter_set = "ML-KEM-768"
input_domain = "x in [0, q) with q = 3329, and d in {1, 4, 5, 10, 11}"

[fips]
document = "FIPS 203"
equations = ["4.7"]
sections = ["4.2.1"]

[c]
function = "compress_d"
signature = "uint16_t compress_d(uint16_t x, unsigned d);"
returns = "Compress_d(x), an integer in [0, 2^d)."

[secrets]
inputs = ["x: compression is applied to the secret-dependent message (d = 1) and, in decapsulation, to the re-encrypted ciphertext (KyberSlash1 and KyberSlash2)"]

[oracles.reference-intermediate]
applies = true
method = "Exhaustive: every x in [0, q) for each d in {1, 4, 5, 10, 11} (16,645 cases), compared with mlkem-native's scalar compress functions and with the spec-literal implementation."
[oracles.acvp]
applies = false
reason = "Component function. ACVP tests top-level functions only (GAP-ACVP-001)."
[oracles.malformed-input]
applies = false
reason = "Component function. Malformed-input tests apply to ML-KEM encapsulation and decapsulation only."
[oracles.implicit-rejection]
applies = false
reason = "Implicit rejection is a property of ML-KEM decapsulation only."
[oracles.ct-valgrind]
applies = true
method = "x marked undefined before the call; d is public. Built with the pinned compiler at -O2 and at -O3, no sanitizers. Division itself is invisible to Valgrind (GAP-CT-001)."
[oracles.ct-static-division]
applies = true
method = "Primary: emitted machine code at -O2 and -O3 for the pinned targets, flagging division instructions and division helper calls. Secondary diagnostic: LLVM IR at -O0, flagging udiv, sdiv, urem and srem (ADR-005)."
[oracles.ct-dudect]
applies = true
method = "Draft subsample member. Two input classes (fixed x versus random x), fixed measurement budget, threshold |t| > 10. Nightly only, reported separately (GAP-CT-003)."
[oracles.differential]
applies = false
reason = "Differential fuzzing belongs to Arm 3 (C-to-Rust migration), not Arm 1 tasks."
+++

# T07 ML-KEM Compress_d

Compress_d(x) = round((2^d / q) * x) mod 2^d, computed with rational
division and rounding, never floating point (FIPS 203 Eq. 4.7).

The static check flags any `/` or `%` on a secret operand: the KyberSlash
class.
