/*
 * Reference adapter: implements the ML-KEM pilot task interfaces (tasks.h)
 * on top of mlkem-native v2.0.0 (third_party/mlkem-native), built as one
 * compilation unit with the portable C backend only (no native backends,
 * so the NTT uses the standard bit-reversed order).
 *
 * The adapter only converts between the task interface and mlkem-native's
 * internal API (argument order, coefficient normalization, length checks).
 * It computes nothing itself.
 */
#define MLK_CONFIG_PARAMETER_SET 768
#define MLK_CONFIG_NAMESPACE_PREFIX otbk
#include "mlkem_native.c"

#include <stdlib.h>

#include "tasks.h"

#define OTB_Q 3329

/* Canonical representative in [0, q). */
static int16_t otb_mod_q(int32_t a) {
  int32_t r = a % OTB_Q;
  return (int16_t)(r < 0 ? r + OTB_Q : r);
}

/* MLKEM-T01: mlkem-native's forward NTT (output bounded by 8q in absolute
 * value, not reduced), then reduced to [0, q) as the task requires. */
void ntt(int16_t f[256]) {
  mlk_poly p;
  for (int i = 0; i < 256; i++) p.coeffs[i] = f[i];
  otbk_poly_ntt(&p);
  for (int i = 0; i < 256; i++) f[i] = otb_mod_q(p.coeffs[i]);
}

/* MLKEM-T07: mlkem-native's scalar compression for each supported d. */
uint16_t compress_d(uint16_t x, unsigned d) {
  int16_t u = (int16_t)x;
  switch (d) {
    case 1: return mlk_scalar_compress_d1(u);
    case 4: return mlk_scalar_compress_d4(u);
    case 5: return mlk_scalar_compress_d5(u);
    case 10: return mlk_scalar_compress_d10(u);
    case 11: return mlk_scalar_compress_d11(u);
    default: return 0xFFFF; /* outside the task domain */
  }
}

/* MLKEM-T24: length checks (FIPS 203 section 7.3 checks 1 and 2), then
 * mlkem-native's decapsulation, which runs the dk hash check (check 3) on
 * every call and implements implicit rejection. */
int mlkem768_decaps(uint8_t ss[32], const uint8_t *ct, size_t ct_len,
                    const uint8_t *dk, size_t dk_len) {
  if (ct_len != 1088 || dk_len != 2400) return -1;
  return otbk_dec(ss, ct, dk) == 0 ? 0 : -1;
}

/* The harness never generates keys or randomness: every input is given.
 * Any call into the library that asks for randomness is a harness bug. */
int randombytes(uint8_t *out, size_t outlen) {
  (void)out;
  (void)outlen;
  abort();
}
