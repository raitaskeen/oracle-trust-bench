/*
 * Mutation base for MLKEM-T01-ntt (ADR-006): a standalone transcription of
 * FIPS 203 Algorithm 9 that must pass every functional oracle of the task
 * (checked by `otb mutants classify` and `otb mutants verify`). It is the
 * reference a model-written test suite must pass, and every mutant is one
 * operator or constant change inside the otb-mutate region.
 */
#include <stdint.h>

#include "tasks.h"

/* otb-mutate-begin */
static int32_t mod_q(int32_t a) {
  int32_t r = a % 3329;
  return r < 0 ? r + 3329 : r;
}

static unsigned bitrev7(unsigned i) {
  unsigned r = 0;
  for (unsigned b = 0; b < 7; b++) r |= ((i >> b) & 1u) << (6 - b);
  return r;
}

void ntt(int16_t f[256]) {
  int32_t a[256];
  for (int j = 0; j < 256; j++) a[j] = f[j];
  unsigned k = 1;
  for (unsigned len = 128; len >= 2; len /= 2) {
    for (unsigned start = 0; start < 256; start += 2 * len) {
      int32_t zeta = 1;
      for (unsigned e = 0; e < bitrev7(k); e++) zeta = mod_q(zeta * 17);
      k++;
      for (unsigned j = start; j < start + len; j++) {
        int32_t t = mod_q(zeta * a[j + len]);
        a[j + len] = mod_q(a[j] - t);
        a[j] = mod_q(a[j] + t);
      }
    }
  }
  for (int j = 0; j < 256; j++) f[j] = (int16_t)a[j];
}
/* otb-mutate-end */
