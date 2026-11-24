/*
 * Reference adapter: implements the ML-DSA pilot task interfaces (tasks.h)
 * on top of mldsa-native v2.0.0 (third_party/mldsa-native), built as one
 * compilation unit with the portable C backend only.
 */
#define MLD_CONFIG_PARAMETER_SET 65
#define MLD_CONFIG_NAMESPACE_PREFIX otbd
#include "mldsa_native.c"

#include <stdlib.h>

#include "tasks.h"

#define OTB_DSA_Q 8380417

/* MLDSA-T41: mldsa-native's power2round takes (a0, a1, a); the task takes
 * (r, r1, r0). Input domain [0, q). */
void power2round(int32_t r, int32_t *r1, int32_t *r0) {
  mld_power2round(r0, r1, r);
}

/* HighBits (FIPS 204 Algorithm 37) via mldsa-native's decompose, for
 * gamma2 = (q - 1) / 32 (ML-DSA-65). */
static int32_t otb_high_bits(int32_t r) {
  int32_t a0, a1;
  mld_decompose(&a0, &a1, r);
  return a1;
}

/* MLDSA-T47: FIPS 204 Algorithm 39, MakeHint(z, r) =
 * [HighBits(r) != HighBits(r + z)], with r + z reduced to [0, q).
 * mldsa-native's own make_hint takes decomposed inputs and is not used. */
int make_hint(int32_t z, int32_t r, int32_t gamma2) {
  (void)gamma2;
  int64_t s = ((int64_t)r + z) % OTB_DSA_Q;
  if (s < 0) s += OTB_DSA_Q;
  return otb_high_bits(r) != otb_high_bits((int32_t)s);
}

/* The harness never generates keys or randomness: every input is given.
 * Any call into the library that asks for randomness is a harness bug. */
int randombytes(uint8_t *out, size_t outlen) {
  (void)out;
  (void)outlen;
  abort();
}
