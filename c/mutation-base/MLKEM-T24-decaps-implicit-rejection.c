/*
 * Mutation base for MLKEM-T24-decaps-implicit-rejection (ADR-006): the
 * reference adapter's decapsulation (FIPS 203 section 7.3 length checks,
 * then mlkem-native's decapsulation, which runs the dk hash check and
 * implicit rejection). The library itself is not mutated: mutants change
 * the input checks and the return-value mapping in the otb-mutate region.
 * It must pass every functional oracle of the task.
 */
#define MLK_CONFIG_PARAMETER_SET 768
#define MLK_CONFIG_NAMESPACE_PREFIX otbk
#include "mlkem_native.c"

#include <stdlib.h>

#include "tasks.h"

/* otb-mutate-begin */
int mlkem768_decaps(uint8_t ss[32], const uint8_t *ct, size_t ct_len,
                    const uint8_t *dk, size_t dk_len) {
  if (ct_len != 1088 || dk_len != 2400) return -1;
  return otbk_dec(ss, ct, dk) == 0 ? 0 : -1;
}
/* otb-mutate-end */

int randombytes(uint8_t *out, size_t outlen) {
  (void)out;
  (void)outlen;
  abort();
}
