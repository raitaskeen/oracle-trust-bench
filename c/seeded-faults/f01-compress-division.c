/*
 * Seeded fault 1: Compress_d with a division on secret data (the
 * KyberSlash pattern).
 *
 * Synthetic example written by hand, not a model output.
 *
 * What it breaks: constant time. The result is correct (FIPS 203 Eq. 4.7,
 * in the exact integer form floor((2^d x + (q - 1) / 2) / q)), but it
 * divides a secret-derived value by q. Whether that division survives into
 * machine code depends on the target: at -O2 and -O3, x86-64, AArch64 and
 * Cortex-M4 turn division by a constant into a multiplication, while
 * Cortex-M0+ (no 32 x 32 -> 64-bit multiply) calls __aeabi_uidiv, whose
 * running time depends on its operands (docs/notes/ct-measurements.md).
 *
 * Which oracles must catch it: ct-static-division (the Cortex-M0+
 * configurations). reference-intermediate passes: the output is correct.
 * ct-valgrind passes: memcheck does not report division, and on x86-64
 * there is no division to report anyway (GAP-CT-001). That blind spot is
 * asserted here on purpose.
 *
 * No library: admitted and built exactly like a candidate.
 *
 * otb-fault: 1
 * otb-task: MLKEM-T07-compress-d
 * otb-lib: none
 * otb-expect: reference-intermediate pass
 * otb-expect: ct-valgrind pass
 * otb-expect: ct-static-division fail variable-time-instruction
 */
#include <stdint.h>

#include "tasks.h"

uint16_t compress_d(uint16_t x, unsigned d) {
  uint32_t a = (uint32_t)x << d;
  return (uint16_t)(((a + 1664u) / 3329u) & ((1u << d) - 1u)); /* FAULT: division */
}
