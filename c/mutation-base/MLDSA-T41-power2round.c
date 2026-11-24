/*
 * Mutation base for MLDSA-T41-power2round (ADR-006): FIPS 204 Algorithm 35
 * for r in [0, q), with r1 = floor((r + 2^12 - 1) / 2^13) and
 * r0 = r - r1 * 2^13, which puts r0 in (-2^12, 2^12]. It must pass every
 * functional oracle of the task.
 */
#include <stdint.h>

#include "tasks.h"

/* otb-mutate-begin */
void power2round(int32_t r, int32_t *r1, int32_t *r0) {
  int32_t hi = (r + 4095) >> 13;
  *r1 = hi;
  *r0 = r - (hi << 13);
}
/* otb-mutate-end */
