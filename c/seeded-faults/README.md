# Seeded faults

Hand-written faulty implementations of the pilot tasks. They are
**synthetic examples written by hand, not model outputs.** They exist to
prove the oracles work: each oracle must fail the faults it targets and
pass the ones it should not see (`docs/TESTING.md`, rule 3).

Each file's header comment says what it breaks, why, and which oracles
must catch it, and ends with machine-readable lines:

```text
 * otb-fault: 5
 * otb-task: MLKEM-T07-compress-d
 * otb-lib: none
 * otb-expect: reference-intermediate fail wrong-output
```

The header is the only place the expected outcomes live. `otb selftest`
reads it, runs every applicable oracle that has code, and fails if an
outcome differs, or if an implemented applicable oracle has no
expectation. A fault with `otb-lib: none` is admitted and built exactly
like a candidate (ADR-004).

| # | File | Task | What it breaks | Caught by |
|---|---|---|---|---|
| 1 | (M5a) | MLKEM-T07 | division on secret data (KyberSlash pattern) | ct-static-division |
| 2 | `f02-decaps-error-on-rejection.c` | MLKEM-T24 | returns an error on implicit rejection (FIPS 203 section 6.3) | implicit-rejection, acvp, malformed-input |
| 3 | `f03-decaps-no-dk-hash-check.c` | MLKEM-T24 | no per-call dk hash check (task contract, GAP-SPEC-001) | malformed-input, acvp |
| 4 | `f04-ntt-wrong-zeta.c` | MLKEM-T01 | one wrong zeta (FIPS 203 Algorithm 9) | reference-intermediate |
| 5 | `f05-compress-rounding.c` | MLKEM-T07 | off-by-one rounding (Eq. 4.7); 5 of 16,645 inputs | reference-intermediate (exhaustive) |
| 6 | `f06-power2round-sign.c` | MLDSA-T41 | sign error for negative r0 (Algorithm 35) | reference-intermediate (exhaustive) |
| 7 | `f07-make-hint-comparison.c` | MLDSA-T47 | `>=` for `>` in Decompose (Algorithm 36) | reference-intermediate (boundary cases) |
| 8 | (M5b) | | passes a weak hand-written test file but fails an oracle | an oracle, not the test file |

## Current output

`otb selftest` (Docker backend, image in `docs/notes/environment.md`),
24 Nov 2026, 7 s wall-clock:

```text
fault task                                 oracle                   expected                       observed
    2 MLKEM-T24-decaps-implicit-rejection  acvp                     fail (wrong-return-code)       fail (wrong-return-code; 10/20 cases; first: decapsulation tcId 88) [ok]
    2 MLKEM-T24-decaps-implicit-rejection  malformed-input          fail (wrong-return-code)       fail (wrong-return-code; 2/44 cases; first: key 0: bit flipped in dk_PKE) [ok]
    2 MLKEM-T24-decaps-implicit-rejection  implicit-rejection       fail (wrong-return-code)       fail (wrong-return-code; 32/36 cases; first: key 0: bit 0 of byte 0) [ok]
    3 MLKEM-T24-decaps-implicit-rejection  acvp                     fail (task-contract)           fail (task-contract; 5/20 cases; first: decapsulationKeyCheck tcId 128) [ok]
    3 MLKEM-T24-decaps-implicit-rejection  malformed-input          fail (task-contract)           fail (task-contract; 8/44 cases; first: key 0: bit flipped in H(ek) first byte) [ok]
    3 MLKEM-T24-decaps-implicit-rejection  implicit-rejection       pass                           pass (36 cases) [ok]
    4 MLKEM-T01-ntt                        reference-intermediate   fail (wrong-output)            fail (wrong-output; 1129/1258 cases; first: input 4) [ok]
    5 MLKEM-T07-compress-d                 reference-intermediate   fail (wrong-output)            fail (wrong-output; 5/16645 cases; first: d=1 x=832) [ok]
    6 MLDSA-T41-power2round                reference-intermediate   fail (wrong-output)            fail (wrong-output; 4189185/8380417 cases; first: r=4097) [ok]
    7 MLDSA-T47-make-hint                  reference-intermediate   fail (wrong-output)            fail (wrong-output; 98/1000340 cases; first: z=1 r=261887) [ok]
10 expectations hold, 0 do not, 0 pending
```

Fault 5's first failure, d = 1 and x = 832, is the input predicted in its
header: 2 x 832 = 1664, so 2^d x = 1664 (mod q).

One expectation was corrected after the first run, and the correction is
recorded in the fault's header: fault 2 was first expected to pass
malformed-input, but that oracle's "dk_PKE bit flipped" control is a valid
input that decapsulates by implicit rejection, so a fault that reports
rejection as an error must fail it. A unit test
(`dk_pke_control_is_an_implicit_rejection`) now asserts this property of
the control.
