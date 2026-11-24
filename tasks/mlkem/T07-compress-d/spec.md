# T07 ML-KEM Compress_d (FIPS 203, Eq. 4.7)
Signature: `uint16_t compress_d(uint16_t x, unsigned d);` for d in {1,4,5,10,11}.
Oracles: exhaustive check over x in [0, q) for each d against the reference. Constant-time: applies. The static check flags any `/` or `%` on a secret operand (the KyberSlash class).
