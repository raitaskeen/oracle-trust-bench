# Seeded faults

Hand-written faulty implementations of the pilot tasks. They are
**synthetic examples written by hand, not model outputs.** They exist to
prove the oracles work: each oracle must fail the faults it targets, pass
the ones it should not see, and show its documented blind spots
(`docs/TESTING.md`, rule 3).

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
| 1 | `f01-compress-division.c` | MLKEM-T07 | correct result, but divides a secret by q (KyberSlash pattern) | ct-static-division (Cortex-M0+); passes ct-valgrind on purpose (GAP-CT-001) |
| 2 | `f02-decaps-error-on-rejection.c` | MLKEM-T24 | returns an error on implicit rejection (FIPS 203 section 6.3) | implicit-rejection, acvp, malformed-input, ct-valgrind |
| 3 | `f03-decaps-no-dk-hash-check.c` | MLKEM-T24 | no per-call dk hash check (task contract, GAP-SPEC-001) | malformed-input, acvp |
| 4 | `f04-ntt-wrong-zeta.c` | MLKEM-T01 | one wrong zeta (FIPS 203 Algorithm 9) | reference-intermediate; also ct-static-division |
| 5 | `f05-compress-rounding.c` | MLKEM-T07 | off-by-one rounding (Eq. 4.7); 5 of 16,645 inputs | reference-intermediate (exhaustive); also ct-static-division |
| 6 | `f06-power2round-sign.c` | MLDSA-T41 | sign error for negative r0 (Algorithm 35) | reference-intermediate (exhaustive) |
| 7 | `f07-make-hint-comparison.c` | MLDSA-T47 | `>=` for `>` in Decompose (Algorithm 36) | reference-intermediate (boundary cases); also ct-valgrind and ct-static-division |
| 8 | (M5b) | | passes a weak hand-written test file but fails an oracle | an oracle, not the test file |

## Current output

`otb selftest` (Docker backend, image in `docs/notes/environment.md`),
24 Nov 2026, at the commit that closed M5a, 48 s wall-clock:

```text
fault task                                 oracle                   expected                       observed
    1 MLKEM-T07-compress-d                 reference-intermediate   pass                           pass (16645 cases) [ok]
    1 MLKEM-T07-compress-d                 ct-valgrind              pass                           pass (2 cases) [ok]
    1 MLKEM-T07-compress-d                 ct-static-division       fail (variable-time-instruction) fail (variable-time-instruction; 2/10 cases; first: clang-armv6m-cortex-m0plus O2: bl __aeabi_uidiv in compress_d) [ok]
    2 MLKEM-T24-decaps-implicit-rejection  acvp                     fail (wrong-return-code)       fail (wrong-return-code; 10/20 cases; first: decapsulation tcId 88) [ok]
    2 MLKEM-T24-decaps-implicit-rejection  malformed-input          fail (wrong-return-code)       fail (wrong-return-code; 2/44 cases; first: key 0: bit flipped in dk_PKE) [ok]
    2 MLKEM-T24-decaps-implicit-rejection  implicit-rejection       fail (wrong-return-code)       fail (wrong-return-code; 32/36 cases; first: key 0: bit 0 of byte 0) [ok]
    2 MLKEM-T24-decaps-implicit-rejection  ct-valgrind              fail (secret-dependent-branch) fail (secret-dependent-branch; 2/2 cases; first: gcc-x86_64 -O2: Conditional jump or move depends on uninitialised value(s) at mlkem768_decaps (impl.c:55)) [ok]
    2 MLKEM-T24-decaps-implicit-rejection  ct-static-division       pass                           pass (10 cases) [ok]
    3 MLKEM-T24-decaps-implicit-rejection  acvp                     fail (task-contract)           fail (task-contract; 5/20 cases; first: decapsulationKeyCheck tcId 128) [ok]
    3 MLKEM-T24-decaps-implicit-rejection  malformed-input          fail (task-contract)           fail (task-contract; 8/44 cases; first: key 0: bit flipped in H(ek) first byte) [ok]
    3 MLKEM-T24-decaps-implicit-rejection  implicit-rejection       pass                           pass (36 cases) [ok]
    3 MLKEM-T24-decaps-implicit-rejection  ct-valgrind              pass                           pass (2 cases) [ok]
    3 MLKEM-T24-decaps-implicit-rejection  ct-static-division       pass                           pass (10 cases) [ok]
    4 MLKEM-T01-ntt                        reference-intermediate   fail (wrong-output)            fail (wrong-output; 1129/1258 cases; first: input 4) [ok]
    4 MLKEM-T01-ntt                        ct-valgrind              pass                           pass (2 cases) [ok]
    4 MLKEM-T01-ntt                        ct-static-division       fail (variable-time-instruction) fail (variable-time-instruction; 8/10 cases; first: clang-x86_64 O2: divl in ntt) [ok]
    5 MLKEM-T07-compress-d                 reference-intermediate   fail (wrong-output)            fail (wrong-output; 5/16645 cases; first: d=1 x=832) [ok]
    5 MLKEM-T07-compress-d                 ct-valgrind              pass                           pass (2 cases) [ok]
    5 MLKEM-T07-compress-d                 ct-static-division       fail (variable-time-instruction) fail (variable-time-instruction; 2/10 cases; first: clang-armv6m-cortex-m0plus O2: bl __aeabi_uidiv in compress_d) [ok]
    6 MLDSA-T41-power2round                reference-intermediate   fail (wrong-output)            fail (wrong-output; 4189185/8380417 cases; first: r=4097) [ok]
    6 MLDSA-T41-power2round                ct-valgrind              pass                           pass (2 cases) [ok]
    6 MLDSA-T41-power2round                ct-static-division       pass                           pass (10 cases) [ok]
    7 MLDSA-T47-make-hint                  reference-intermediate   fail (wrong-output)            fail (wrong-output; 98/1000340 cases; first: z=1 r=261887) [ok]
    7 MLDSA-T47-make-hint                  ct-valgrind              fail (secret-dependent-branch) fail (secret-dependent-branch; 2/2 cases; first: gcc-x86_64 -O2: Conditional jump or move depends on uninitialised value(s) at high_bits (impl.c:44)) [ok]
    7 MLDSA-T47-make-hint                  ct-static-division       fail (variable-time-instruction) fail (variable-time-instruction; 2/10 cases; first: clang-armv6m-cortex-m0plus O2: bl __aeabi_uidivmod in make_hint) [ok]
25 expectations hold, 0 do not, 0 pending
```

Fault 5's first failure, d = 1 and x = 832, is the input predicted in its
header: 2 x 832 = 1664, so 2^d x = 1664 (mod q). Fault 4's divisions are
reported without saying whether their operands are secret: the check does
not trace operands (GAP-CT-002).

## Corrections after a first run

Two expectations were corrected after the run that first exercised them.
Both corrections, and the reasons, are recorded in the fault's header, and
the oracle was right both times:

- **Fault 2, malformed-input** (M4): first expected to pass. Its "dk_PKE
  bit flipped" control is a valid input that decapsulates by implicit
  rejection, so a fault that reports rejection as an error must fail it.
  A unit test (`dk_pke_control_is_an_implicit_rejection`) now asserts this.
- **Fault 7, ct-static-division** (M5a): first expected to pass because the
  source has no `/` or `%`. Clang turns its reduction loop into a division
  by q, which Cortex-M0+ calls as `__aeabi_uidivmod` on a secret
  (`docs/notes/ct-measurements.md`, measurement 4).

The other 23 expectations held as written before their first run.
