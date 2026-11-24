/*
 * Seeded fault 8: Compress_d with a 16-bit intermediate that overflows,
 * and a weak test suite that passes it.
 *
 * Synthetic example written by hand, not a model output. The test file
 * suites/f08-tests.c is also written by hand, in the style of a quick
 * model-written suite: it checks d = 1 and d = 4 only.
 *
 * What it breaks: FIPS 203 Eq. 4.7. x << d is stored in a uint16_t, which
 * is exact for d <= 4 (3328 * 16 < 65536) but overflows for d = 5 when
 * x >= 2048, and for most x when d = 10 or d = 11.
 *
 * What it shows: false assurance in miniature. The suite is valid by the
 * study's rules (at least 5 assertions, passes on the reference, kills at
 * least one mutant) and passes on this fault, yet the independent oracle
 * fails it.
 *
 * Expected outcomes, written before the first run (M5b):
 * reference-intermediate fails; ct-valgrind passes (no secret-dependent
 * branch); ct-static-division fails (it divides by q, which Cortex-M0+
 * keeps as a division); the suite is valid and passes on the fault.
 *
 * No library: admitted and built exactly like a candidate.
 *
 * otb-fault: 8
 * otb-task: MLKEM-T07-compress-d
 * otb-lib: none
 * otb-suite: c/seeded-faults/suites/f08-tests.c
 * otb-expect-suite: valid pass
 * otb-expect: reference-intermediate fail wrong-output
 * otb-expect: ct-valgrind pass
 * otb-expect: ct-static-division fail variable-time-instruction
 */
#include <stdint.h>

#include "tasks.h"

uint16_t compress_d(uint16_t x, unsigned d) {
  uint16_t a = (uint16_t)(x << d); /* FAULT: should be 32 bits wide */
  return (uint16_t)((((uint32_t)a + 1664u) / 3329u) & ((1u << d) - 1u));
}
