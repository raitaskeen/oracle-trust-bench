# Constant-time measurements (M5a)

Measured on 24 Nov 2026 inside the pinned image
(`sha256:d07dce05bd48c4d3b894505ff5f9bde94e06f058574e5ca33046fa64aad77ed9`;
GCC 14.2.0, Clang 19.1.7, Valgrind 3.24.0; `docs/notes/environment.md`).
These results decide the design in ADR-005. Each one names its command.

## 1. Where division by a constant survives

Source (two functions):

```c
uint16_t compress_d(uint16_t x, unsigned d) {
  uint32_t a = (uint32_t)x << d;
  return (uint16_t)(((a + 1664u) / 3329u) & ((1u << d) - 1u));
}
uint32_t vdiv(uint32_t a, uint32_t b) { return a / b; }
```

Command, per target, at -O2 and -O3:
`clang --target=<target> -O2 -nostdlibinc -S -o - div.c`, and
`gcc -O2 -S -o - div.c`.

| Target | `compress_d` (divide by the constant q) | `vdiv` (variable divisor) |
|---|---|---|
| x86-64 (GCC and Clang) | multiplication (`imulq $989558401`) | `divl` |
| AArch64 | multiplication (`umull`) | `udiv` |
| Cortex-M4 (Armv7E-M) | multiplication (`umull`) | `udiv` |
| Cortex-M0+ (Armv6-M) | `bl __aeabi_uidiv` | `bl __aeabi_uidiv` |

The same at -O3. `clang -O0 -S -emit-llvm` shows `udiv i32 %11, 3329` for
`compress_d`.

**Consequence.** At -O2 and -O3 the KyberSlash pattern (a secret divided
by q) is visible in emitted code only on a target without a 32 x 32 ->
64-bit multiply. Without Cortex-M0+ in the target set, the emitted-code
check could not see it.

## 2. What Valgrind memcheck reports

`c/ct-probes/valgrind_probe.c`, built with `gcc -std=c11 -O2 -g` and run as
`valgrind -q ./p <mode>`. Inputs are marked undefined with
`VALGRIND_MAKE_MEM_UNDEFINED`. Asserted on every CI run by
`crates/otb-harness/tests/ct_probes.rs`.

| Mode | Code | Emitted (GCC -O2) | memcheck |
|---|---|---|---|
| d | `a / b`, both secret | `divl` | nothing reported |
| s | `r < 0 ? r + 3329 : r`, r secret | `cmovs` | nothing reported |
| b | `while (r >= 100) ...`, r secret | conditional jumps | "Conditional jump or move depends on uninitialised value(s)" |
| i | `t[i & 7]`, i secret | indexed load | "Use of uninitialised value of size 8" |

The same at -O3.

**Consequence.** `ct-valgrind` sees secret-dependent branches and memory
addresses. It does not see division (GAP-CT-001), and it does not see a
conditional move: memcheck propagates undefinedness through the move
without reporting it. A conditional move is constant time on current
processors, so this is not a gap for the timing question, but it means a
source-level `if` can pass or fail depending on what the compiler emits.

## 3. Whole-file versus reachable code, on the references

`otb ref run` with the first version of `ct-static-division`, which counted
every division in the implementation file. The reference files include the
whole mlkem-native library (one compilation unit):

| Reference | Whole-file rule | Finding |
|---|---|---|
| MLKEM-T01, T07, T24 | fail on Cortex-M0+ (-O2 and -O3) | `bl __aeabi_uidiv` in `otbk_shake256x4` |
| MLDSA-T41, T47 | pass | - |

The division is `nblocks = outlen / SHAKE256_RATE` in
`third_party/mlkem-native/mlkem/src/fips202/fips202x4.c`: a public length
divided by the constant 136. In the Cortex-M0+ assembly
(`clang --target=arm-none-eabi -mcpu=cortex-m0plus -mthumb -mfloat-abi=soft -O2`),
`otbk_shake256x4` is never called: Clang inlined it into its only caller,
`otbk_poly_getnoise_eta1_4x`, where `outlen` is a constant and the
division folds away. The remaining copy is unused.

With the rule restricted to code reachable from the task's C function
(ADR-005), all five references pass on all ten configurations, and the
report lists the unreached division as "not counted".

## 4. A division the source does not contain

Seeded fault 7 (`c/seeded-faults/f07-make-hint-comparison.c`) has no `/`
or `%`. Its reduction loop `while (rp >= Q) rp -= Q;` appears in Clang's
-O2 IR for Cortex-M0+ as `urem i32 %4, 8380417` and `udiv i32 %14, 8380417`,
and in the emitted code as `bl __aeabi_uidivmod` and `bl __aeabi_uidiv`
on the secret `rp`. The other targets turn these into multiplications.

**Consequence.** Source-level checks miss divisions the compiler creates;
the emitted-code level finds them. The fault's header records that this
was not predicted before the first run.
