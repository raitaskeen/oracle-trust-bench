# T41 ML-DSA Power2Round (FIPS 204, Algorithm 35)
Signature: `void power2round(int32_t r, int32_t *r1, int32_t *r0);` d = 13, q = 8380417.
Oracles: exhaustive check over r in [0, q) against mldsa-native. Constant-time: applies (t is secret during keygen).
