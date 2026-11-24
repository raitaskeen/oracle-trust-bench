/*
 * Seeded fault 6: Power2Round with a sign error for negative r0.
 *
 * Synthetic example written by hand, not a model output.
 *
 * What it breaks: FIPS 204 Algorithm 35. r0 = r+ mod+- 2^13 must lie in
 * (-2^12, 2^12]; when the low 13 bits exceed 2^12 the representative is
 * low - 2^13, which is negative. This version uses 2^13 - low instead (the
 * right magnitude with the wrong sign) and derives r1 from it. About half
 * of all inputs are wrong.
 *
 * Which oracles must catch it: reference-intermediate (exhaustive over
 * [0, q)).
 *
 * No library: admitted and built exactly like a candidate.
 *
 * Constant time (predicted before the first constant-time run, M5a): both
 * pass. There is no division, and the conditional assignment is expected
 * to compile to a conditional move, which memcheck does not report
 * (GAP-CT-001).
 *
 * otb-fault: 6
 * otb-task: MLDSA-T41-power2round
 * otb-lib: none
 * otb-expect: reference-intermediate fail wrong-output
 * otb-expect: ct-valgrind pass
 * otb-expect: ct-static-division pass
 */
#include <stdint.h>

#include "tasks.h"

void power2round(int32_t r, int32_t *r1, int32_t *r0) {
  /* The task domain is r in [0, q), so r mod+ q = r. */
  int32_t low = r & 0x1FFF;
  if (low > 4096) low = 8192 - low; /* FAULT: should be low - 8192 */
  *r0 = low;
  *r1 = (r - low) >> 13;
}
