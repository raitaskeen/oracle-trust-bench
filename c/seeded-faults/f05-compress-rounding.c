/*
 * Seeded fault 5: Compress_d with off-by-one rounding.
 *
 * Synthetic example written by hand, not a model output.
 *
 * What it breaks: FIPS 203 Eq. 4.7, Compress_d(x) = round(2^d / q * x)
 * mod 2^d. For odd q the exact integer form is
 * floor((2^d x + (q - 1) / 2) / q); this version adds (q + 1) / 2 = 1665
 * instead of 1664. The result differs only when 2^d x = 1664 (mod q), which
 * holds for exactly one x in [0, q) for each d: 5 of the 16,645 inputs.
 * Random sampling is unlikely to find them; the exhaustive check must.
 *
 * Which oracles must catch it: reference-intermediate (exhaustive over
 * [0, q) for each d). It also divides a secret-derived value by q, the
 * KyberSlash pattern, which ct-static-division is expected to report (M5a).
 *
 * No library: admitted and built exactly like a candidate.
 *
 * otb-fault: 5
 * otb-task: MLKEM-T07-compress-d
 * otb-lib: none
 * otb-expect: reference-intermediate fail wrong-output
 */
#include <stdint.h>

#include "tasks.h"

uint16_t compress_d(uint16_t x, unsigned d) {
  uint32_t a = (uint32_t)x << d;
  return (uint16_t)(((a + 1665u) / 3329u) & ((1u << d) - 1u)); /* FAULT: 1664 */
}
