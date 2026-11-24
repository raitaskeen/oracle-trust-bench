# T01 ML-KEM NTT (FIPS 203, Algorithm 9)
Signature: `void ntt(int16_t f[256]);` in-place, coefficients in Z_q, q = 3329, zeta = 17.
Input: 256 coefficients in [0, q). Output: NTT representation, coefficients in [0, q).
Oracles: intermediate values vs mlkem-native (normalized mod q). Constant-time: applies (secret input in keygen/decaps).
Notes for the spec summary: bit-reversed zeta order; 7 layers; no final scaling.
