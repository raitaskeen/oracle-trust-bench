# T47 ML-DSA MakeHint (FIPS 204, Algorithm 39)
Signature: `int make_hint(int32_t z, int32_t r, int32_t gamma2);` for ML-DSA-65 (gamma2 = (q-1)/32).
Oracles: intermediate values vs FIPS 204 Algorithm 39 computed as HighBits(r) != HighBits(r+z) (Algorithm 37 through mldsa-native's mld_decompose), and vs the spec-literal Rust implementation, over sampled and boundary inputs. mldsa-native's mld_make_hint(a0, a1) is not compared directly. Constant-time: applies during signing. Timing variation in the rejection loop is expected and excluded by the matrix.
