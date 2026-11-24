/*
 * Mutation base for MLDSA-T47-make-hint (ADR-006): FIPS 204 Algorithm 39,
 * MakeHint(z, r) = [HighBits(r) != HighBits(r + z)], with HighBits from
 * Decompose (Algorithm 36). Task domain: z and r in [0, q). It must pass
 * every functional oracle of the task. It divides, so it is not constant
 * time; mutants are used to judge test suites, not timing.
 */
#include <stdint.h>

#include "tasks.h"

/* otb-mutate-begin */
static int32_t high_bits(int32_t r, int32_t gamma2) {
  int32_t a = 2 * gamma2;
  int32_t r0 = r % a;
  if (r0 > gamma2) r0 -= a;
  if (r - r0 == 8380417 - 1) return 0;
  return (r - r0) / a;
}

int make_hint(int32_t z, int32_t r, int32_t gamma2) {
  int32_t s = r + z;
  if (s >= 8380417) s -= 8380417;
  return high_bits(r, gamma2) != high_bits(s, gamma2);
}
/* otb-mutate-end */
