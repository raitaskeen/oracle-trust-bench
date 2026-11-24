# T24 ML-KEM-768.Decaps with input checking and implicit rejection (FIPS 203, Algorithm 21 and §7.3; core: Algorithm 18)
Signature: `int mlkem768_decaps(uint8_t ss[32], const uint8_t *ct, size_t ct_len, const uint8_t *dk, size_t dk_len);`
Returns 0 for every ciphertext of the correct length, whether decapsulation succeeds or implicitly rejects, and always writes ss. Returns nonzero only when an input check fails.
Oracles:
- ACVP decapsulation vectors.
- Malformed input: wrong ct_len (§7.3 check 1, required on every call) or wrong dk_len (§7.3 check 2).
- Task contract, stricter than FIPS 203: the dk hash check (§7.3 check 3, Eq. 7.2) must run on every call. A failure here is reported as a task-contract violation, not a FIPS violation.
- Implicit rejection: a modified ct must give ss = J(z || c) and the same return value as the valid path.
- Constant-time: the comparison and the selection of the shared secret.
