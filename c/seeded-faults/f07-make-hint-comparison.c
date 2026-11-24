/*
 * Seeded fault 7: MakeHint with a flipped comparison in Decompose.
 *
 * Synthetic example written by hand, not a model output.
 *
 * What it breaks: FIPS 204 Algorithm 36 (Decompose), used through HighBits
 * by Algorithm 39 (MakeHint). r0 = r+ mod+- 2 gamma2 must lie in
 * (-gamma2, gamma2]. This version tests r0 >= gamma2 instead of
 * r0 > gamma2, so r0 = gamma2 is moved to -gamma2 and HighBits is one too
 * high exactly at the odd multiples of gamma2. Only inputs next to a
 * HighBits boundary expose it.
 *
 * Which oracles must catch it: reference-intermediate (the boundary cases).
 *
 * No library: admitted and built exactly like a candidate.
 *
 * Constant time: ct-valgrind fails, because the reduction loops run a
 * secret-dependent number of times. ct-static-division also fails. The
 * prediction written before the first constant-time run (M5a) said it
 * would pass, since the source has no / or %. The first run showed that
 * Clang turns the repeated-subtraction loop `while (rp >= Q) rp -= Q;` in
 * high_bits into urem and udiv by q (visible in its -O2 LLVM IR); on
 * Cortex-M0+ those become calls to __aeabi_uidivmod and __aeabi_uidiv on
 * the secret rp, while the other targets turn them into multiplications.
 * The compiler introduced a variable-time division the source does not
 * contain. This is why the emitted code, not the source, is the primary
 * level (ADR-005).
 *
 * otb-fault: 7
 * otb-task: MLDSA-T47-make-hint
 * otb-lib: none
 * otb-expect: reference-intermediate fail wrong-output
 * otb-expect: ct-valgrind fail secret-dependent-branch
 * otb-expect: ct-static-division fail variable-time-instruction
 */
#include <stdint.h>

#include "tasks.h"

#define Q 8380417

static int32_t high_bits(int32_t r, int32_t gamma2) {
  int32_t rp = r;
  while (rp >= Q) rp -= Q;
  while (rp < 0) rp += Q;
  const int32_t a = 2 * gamma2;
  int32_t r0 = rp, r1 = 0;
  while (r0 >= a) {
    r0 -= a;
    r1++;
  }
  if (r0 >= gamma2) { /* FAULT: should be r0 > gamma2 */
    r0 -= a;
    r1++;
  }
  if (rp - r0 == Q - 1) return 0;
  return r1;
}

int make_hint(int32_t z, int32_t r, int32_t gamma2) {
  return high_bits(r, gamma2) != high_bits(r + z, gamma2);
}
