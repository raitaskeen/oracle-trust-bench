/*
 * Weak test suite for seeded fault 8. Written by hand, in the style of a
 * quick model-written suite: plausible values, d = 1 and d = 4 only.
 * Not a model output.
 *
 * Expected values from FIPS 203 Eq. 4.7,
 * Compress_d(x) = floor((2^d x + 1664) / 3329) mod 2^d:
 *   d = 1: x = 0 -> 0; 1664 -> 4992 / 3329 = 1; 3328 -> 8320 / 3329 = 2,
 *          mod 2 = 0; 832 -> 3328 / 3329 = 0.
 *   d = 4: x = 1665 -> 28304 / 3329 = 8; 208 -> 4992 / 3329 = 1;
 *          3328 -> 54912 / 3329 = 16, mod 16 = 0.
 */
#include "otb_test.h"
#include "tasks.h"

int main(void) {
  OTB_ASSERT(compress_d(0, 1) == 0);
  OTB_ASSERT(compress_d(1664, 1) == 1);
  OTB_ASSERT(compress_d(3328, 1) == 0);
  OTB_ASSERT(compress_d(832, 1) == 0);
  OTB_ASSERT(compress_d(1665, 4) == 8);
  OTB_ASSERT(compress_d(208, 4) == 1);
  OTB_ASSERT(compress_d(3328, 4) == 0);
  return 0;
}
