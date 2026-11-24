# FIPS verification of the pilot task specs

Status: FIX-1 to FIX-4 approved by the author on 24 Nov 2026.
Drafted with AI assistance (Claude); reviewed by the author.

## Sources and method

| Document | Source | sha256 of the PDF read |
|---|---|---|
| FIPS 203 (Aug 2024) | https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.203.pdf | `fe1f12f32a7e44ec9fdebbf400cda843a40b506dee676725234dc6f7923b6cac` |
| FIPS 204 (Aug 2024) | https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.204.pdf | `57239b9f84c03227eda3ca0991204dc7764c79af9ce2e6824eda774918d46b6b` |

Each PDF was converted with `pdftotext -layout` (poppler). Every algorithm, section and equation number below was then found in that text. The PDFs are not committed: they are NIST publications, and anyone can fetch them from the URLs above and check them against the hashes.

To reproduce a check:

```sh
sha256sum NIST.FIPS.203.pdf
pdftotext -layout NIST.FIPS.203.pdf - | grep -n "Algorithm 18"
```

## Fixes to the draft specs (approved)

| # | File | Old text | New text | Justification |
|---|---|---|---|---|
| FIX-1 | `tasks/mlkem/T24-decaps-implicit-rejection/spec.md`, title | `T24 ML-KEM.Decaps_internal with implicit rejection (FIPS 203, Algorithm 18)` | `T24 ML-KEM-768.Decaps with input checking and implicit rejection (FIPS 203, Algorithm 21 and §7.3; core: Algorithm 18)` | FIPS 203 §3.3: "The algorithms ML-KEM.Encaps and ML-KEM.Decaps require input checking." Alg 18 lists its inputs as plain "Input"; Alg 21 lists them as "Checked input". The checks themselves are in §7.3. |
| FIX-2 | same file, malformed-input bullet | `Malformed input: wrong ct_len or dk_len; dk hash-check failure (FIPS 203 §7.3).` | `Malformed input: wrong ct_len (§7.3 check 1, required on every call) or wrong dk_len (§7.3 check 2).` plus a task-contract bullet: the dk hash check (§7.3 check 3, Eq. 7.2) must run on every call, and a failure is a task-contract violation, not a FIPS violation | FIPS 203 §7.3: decapsulation key checking "need not be performed by the decapsulating party, nor with every execution of ML-KEM.Decaps"; "Ciphertext checking shall be performed with every execution of ML-KEM.Decaps." |
| FIX-3 | same file, signature and implicit-rejection bullet | `int mlkem768_decaps(...)`; `a modified ct must return J(z \|\| c), not an error.` | Added: returns 0 for every ciphertext of the correct length, whether decapsulation succeeds or implicitly rejects, and always writes ss; returns nonzero only when an input check fails. A modified ct must give ss = J(z \|\| c) and the same return value as the valid path | FIPS 203 §6.3 (text before Alg 18): the implicit-reject flag "shall be destroyed"; "returning the value of the flag as an output in any form is not permitted." Alg 18 lines 7–11. |
| FIX-4 | `tasks/mldsa/T47-make-hint/spec.md`, oracle line | `Oracles: intermediate values vs mldsa-native over sampled and boundary inputs.` | `Oracles: intermediate values vs FIPS 204 Algorithm 39 computed as HighBits(r) != HighBits(r+z) (Algorithm 37 through mldsa-native's mld_decompose), and vs the spec-literal Rust implementation ...; mldsa-native's mld_make_hint(a0, a1) is not compared directly.` | FIPS 204 Alg 39 (MakeHint(z, r): r1 ← HighBits(r), v1 ← HighBits(r + z), return [[r1 ≠ v1]]); Alg 37 (HighBits); Alg 36 (Decompose). The mldsa-native `rounding.h` header says its `mld_make_hint` is not a line-for-line implementation of MakeHint(z, r). |

## Every FIPS reference in the pilot specs

| Task | Claim in spec | Where in FIPS | Result |
|---|---|---|---|
| T01 | NTT is FIPS 203 Algorithm 9 | FIPS 203 §4.3, Alg 9 | OK |
| T01 | q = 3329, ζ = 17 | FIPS 203 §4.3: "ζ = 17 ... is a primitive 256-th root of unity modulo q"; §8 Table 2: q = 3329 | OK |
| T01 | bit-reversed zeta order, 7 layers, no final scaling | Alg 9 lines 3–13: len runs from 128 down to 2 (7 layers); zeta ← ζ^BitRev7(i) mod q with i starting at 1; no multiplication after the loops | OK |
| T07 | Compress_d is FIPS 203 Eq. 4.7, defined for d < 12 | FIPS 203 §4.2.1, Eq. (4.7); Decompress is Eq. (4.8); floating point "shall not be used" | OK |
| T07 | d ∈ {1, 4, 5, 10, 11} | Table 2: (du, dv) = (10, 4) for ML-KEM-512/768 and (11, 5) for ML-KEM-1024; d = 1 is used for the message (Alg 14 line 20, Decompress1; Alg 15, Compress1) | OK |
| T24 | Decaps core is Alg 18; top level is Alg 21; checks in §7.3 | FIPS 203 §6.3 Alg 18; §7.3 Alg 21 | OK, after FIX-1 |
| T24 | ct, dk lengths for ML-KEM-768 | §8 Table 3: dk 2400 B, ciphertext 1088 B; §7.3 checks 1–2: 32(du·k + dv) and 768k + 96 | OK |
| T24 | hash check | §7.3 Eq. (7.2): H(dk[384k : 768k+32]) compared with dk[768k+32 : 768k+64]; H = SHA3-256 (Eq. 4.4) | OK |
| T24 | implicit rejection K̄ = J(z ‖ c) | Alg 18 lines 4 and 7; J(s) = SHAKE256(s, 8·32) (Eq. 4.4) | OK |
| T41 | Power2Round is FIPS 204 Algorithm 35 | FIPS 204 §7.4, Alg 35 | OK |
| T41 | d = 13, q = 8380417 | FIPS 204 §4 Table 1 | OK |
| T41 | r0 range | Alg 35: r0 ← r⁺ mod± 2^d; mod± gives −α/2 < m′ ≤ α/2, so r0 ∈ (−4096, 4096] | OK |
| T47 | MakeHint is FIPS 204 Algorithm 39 | FIPS 204 §7.4, Alg 39 | OK |
| T47 | γ2 = (q−1)/32 for ML-DSA-65 | FIPS 204 §4 Table 1 | OK |

## Constant-time claims in the specs

The FIPS texts do not define constant-time requirements for individual subroutines. The constant-time rows in the pilot specs ("applies") are therefore study design decisions, not FIPS requirements. Each one will be justified in `protocol/ORACLE_MATRIX.md` by naming the secret input.
