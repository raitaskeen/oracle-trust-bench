# ADR-005: Constant-time method

Status: accepted for the pilot (M5a, 24 Nov 2026). **Draft for the
pre-registration**: the target set and the reachability rule below must be
confirmed by the author before the pre-registration is filed
(`protocol/PREREGISTRATION.md`).

## Context

The application promises "Valgrind constant-time checks on non-sanitized
-O2 and -O3 builds, plus a static check for division on secret data (the
KyberSlash class)", with Valgrind builds "from one pinned compiler". The
author decided (Q3, 24 Nov 2026) that the primary signal is emitted machine
code at -O2 and -O3, a violation at either level counts, and that
source/IR analysis is a secondary diagnostic, never used alone.

Measurements in the pinned image (`docs/notes/ct-measurements.md`) showed:

1. division by the constant q becomes a multiplication at -O2 and -O3 on
   x86-64, AArch64 and Cortex-M4, and stays a division only on Cortex-M0+;
2. memcheck reports secret-dependent branches and addresses, but not
   division and not conditional moves;
3. a whole-file rule flags an unused copy of a library function in three
   of the five references;
4. compilers can create divisions the source does not contain.

## Decision

### `ct-valgrind`

- Build: driver + implementation with GCC 14.2.0 (the pinned compiler of
  every run in the study), `-std=c11 -g`, at -O2 and at -O3, no
  sanitizers, with `-DOTB_CT_VALGRIND`.
- Secrets: the driver (never the implementation) marks the inputs listed in
  the task spec's `[secrets]` section undefined before each call and marks
  the outputs defined after it. For T24 only dk_PKE and z are marked; ek and
  H(ek) are public (FIPS 203 Algorithm 19).
- Run: `valgrind -q --tool=memcheck` on a few fixed inputs per task
  (memcheck flags a dependency whatever the secret's value).
- Verdict: fail if either build reports "Conditional jump or move depends
  on uninitialised value(s)" (class `secret-dependent-branch`) or "Use of
  uninitialised value" (class `secret-dependent-address`); other memcheck
  errors and abnormal exits are `crash`.
- Known blind spots, asserted in CI (`ct_probes.rs`): division and
  conditional moves are not reported (GAP-CT-001).

### `ct-static-division`

- Level: emitted assembly of the implementation file only (not the driver).
- Configurations (10 builds): GCC 14.2.0 for x86-64, and Clang 19.1.7 for
  x86-64, AArch64, Cortex-M4 (Armv7E-M) and Cortex-M0+ (Armv6-M), each at
  -O2 and -O3. Cortex-M0+ is included because it is the target where
  division by a constant survives (measurement 1). Targets without a C
  library in the image are compiled against declaration-only headers
  (GAP-CT-005).
- What counts as a division: any division instruction (`div`, `idiv`,
  `udiv`, `sdiv`, `fdiv` and their vector and suffixed forms) or any
  reference to a division or modulo helper (`__aeabi_uidiv`,
  `__aeabi_ldivmod`, `__udivdi3`, `div`, ...; the full list is
  `DIVISION_HELPERS` in `crates/otb-harness/src/ct.rs`).
- Scope: only code reachable from the task's C function: that function,
  every function whose address appears in data, and transitively every
  function a reachable function names (calls, tail calls, address taken).
  Divisions elsewhere in the file are listed in the report as "not
  counted". Rationale: measurement 3.
- Operands are not traced: every reachable division counts, whether its
  operands are secret or public (GAP-CT-002). In the pilot tasks every
  input is secret, so this over-reports only when a candidate divides
  public values (for example a length) in reachable code.
- Verdict: fail (class `variable-time-instruction`) if any configuration
  has a reachable division; a violation at -O2 or -O3 on any target
  counts. If the GCC x86-64 -O2 build does not compile, the outcome is
  `compile-error`.

### Secondary diagnostic

The implementation file is also compiled with `clang -O0 -S -emit-llvm`,
and `udiv`, `sdiv`, `urem`, `srem`, `fdiv` and `frem` reachable from the
task function are counted. The count appears in the report, labelled
"diagnostic, not part of the verdict", and never decides an outcome.

### dudect

Not implemented in M5a. It runs only on the pre-registered subsample,
nightly, and is reported separately (GAP-CT-003).

## Evidence

- References: every reference passes both oracles on every configuration
  (`otb ref run`; `docs/oracles/false-positives.md`).
- Seeded faults: `otb selftest`; faults 1, 4, 5 and 7 fail
  `ct-static-division`, faults 2 and 7 fail `ct-valgrind`, and fault 1
  passes `ct-valgrind`, which asserts the division blind spot.

## Consequences

- The KyberSlash pattern is caught at the emitted level, but only through
  the Cortex-M0+ configurations. Removing that target would remove the
  primary level's ability to see it.
- The reachability rule depends on the compiler's inlining; an unused
  function with a division does not count, a used one does.
- The target list and the reachability rule are part of what the
  pre-registration must fix.
