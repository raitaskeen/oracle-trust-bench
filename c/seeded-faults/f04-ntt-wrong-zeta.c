/*
 * Seeded fault 4: NTT with one wrong twiddle factor.
 *
 * Synthetic example written by hand, not a model output.
 *
 * What it breaks: FIPS 203 Algorithm 9. The table of zeta^BitRev7(i) mod q
 * is computed correctly, then entry 37 is increased by one, so one block of
 * one butterfly layer uses a wrong zeta.
 *
 * Which oracles must catch it: reference-intermediate. (ACVP does not
 * apply to the NTT component task; GAP-ACVP-001.)
 *
 * No library: admitted and built exactly like a candidate.
 *
 * otb-fault: 4
 * otb-task: MLKEM-T01-ntt
 * otb-lib: none
 * otb-expect: reference-intermediate fail wrong-output
 */
#include <stdint.h>

#include "tasks.h"

#define Q 3329

static int32_t mod_q(int64_t a) {
  int32_t r = (int32_t)(a % Q);
  return r < 0 ? r + Q : r;
}

static unsigned bitrev7(unsigned i) {
  unsigned r = 0;
  for (unsigned b = 0; b < 7; b++) {
    if ((i >> b) & 1u) r |= 1u << (6 - b);
  }
  return r;
}

void ntt(int16_t f[256]) {
  int32_t zetas[128];
  for (unsigned i = 0; i < 128; i++) {
    int64_t z = 1;
    for (unsigned e = 0; e < bitrev7(i); e++) z = z * 17 % Q;
    zetas[i] = (int32_t)z;
  }
  zetas[37] = (zetas[37] + 1) % Q; /* FAULT */

  int32_t a[256];
  for (int j = 0; j < 256; j++) a[j] = mod_q(f[j]);
  unsigned i = 1;
  for (unsigned len = 128; len >= 2; len /= 2) {
    for (unsigned start = 0; start < 256; start += 2 * len) {
      int64_t zeta = zetas[i++];
      for (unsigned j = start; j < start + len; j++) {
        int32_t t = mod_q(zeta * a[j + len]);
        a[j + len] = mod_q((int64_t)a[j] - t);
        a[j] = mod_q((int64_t)a[j] + t);
      }
    }
  }
  for (int j = 0; j < 256; j++) f[j] = (int16_t)a[j];
}
