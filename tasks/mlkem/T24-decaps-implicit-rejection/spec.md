# T24 ML-KEM.Decaps_internal with implicit rejection (FIPS 203, Algorithm 18)
Signature: `int mlkem768_decaps(uint8_t ss[32], const uint8_t *ct, size_t ct_len, const uint8_t *dk, size_t dk_len);`
Oracles:
- ACVP decapsulation vectors.
- Malformed input: wrong ct_len or dk_len; dk hash-check failure (FIPS 203 §7.3).
- Implicit rejection: a modified ct must return J(z || c), not an error.
- Constant-time: the comparison and the selection of the shared secret.
