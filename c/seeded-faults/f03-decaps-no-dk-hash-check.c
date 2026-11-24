/*
 * Seeded fault 3: decapsulation without the decapsulation-key hash check.
 *
 * Synthetic example written by hand, not a model output.
 *
 * What it breaks: the T24 task contract, which requires FIPS 203 section
 * 7.3 check 3 (the H(ek) stored in dk equals the hash of the ek stored in
 * dk, Eq. 7.2) on every call. FIPS 203 itself allows the check to be done
 * once or by another party, so this is a task-contract failure, not a FIPS
 * violation (GAP-SPEC-001). The length checks are intact.
 *
 * How: before calling the library (which does run the check), it
 * overwrites the stored hash in a copy of dk with H(ek), so the check can
 * never fail. The effect is exactly that of skipping it.
 *
 * Which oracles must catch it: malformed-input (keys with a modified hash
 * or ek are accepted) and acvp (the decapsulationKeyCheck cases with a
 * modified H). implicit-rejection passes.
 *
 * otb-fault: 3
 * otb-task: MLKEM-T24-decaps-implicit-rejection
 * otb-lib: mlkem
 * otb-expect: acvp fail task-contract
 * otb-expect: malformed-input fail task-contract
 * otb-expect: implicit-rejection pass
 */
#define MLK_CONFIG_PARAMETER_SET 768
#define MLK_CONFIG_NAMESPACE_PREFIX otbk
#include "mlkem_native.c"

#include <stdlib.h>
#include <string.h>

#include "tasks.h"

int mlkem768_decaps(uint8_t ss[32], const uint8_t *ct, size_t ct_len,
                    const uint8_t *dk, size_t dk_len) {
  if (ct_len != 1088 || dk_len != 2400) return -1;
  uint8_t dk2[2400];
  memcpy(dk2, dk, sizeof dk2);
  /* FAULT. dk = dk_PKE (1152) || ek (1184) || H(ek) (32) || z (32). */
  otbk_sha3_256(dk2 + 1152 + 1184, dk2 + 1152, 1184);
  return otbk_dec(ss, ct, dk2) == 0 ? 0 : -1;
}

int randombytes(uint8_t *out, size_t outlen) {
  (void)out;
  (void)outlen;
  abort();
}
