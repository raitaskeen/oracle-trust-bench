/*
 * Mutation base for MLKEM-T07-compress-d (ADR-006): FIPS 203 Eq. 4.7 in
 * the exact integer form floor((2^d x + (q - 1) / 2) / q) mod 2^d. It must
 * pass every functional oracle of the task. It divides, so it is not
 * constant time; mutants are used to judge test suites, not timing.
 */
#include <stdint.h>

#include "tasks.h"

/* otb-mutate-begin */
uint16_t compress_d(uint16_t x, unsigned d) {
  uint32_t a = (uint32_t)x << d;
  return (uint16_t)(((a + 1664u) / 3329u) & ((1u << d) - 1u));
}
/* otb-mutate-end */
