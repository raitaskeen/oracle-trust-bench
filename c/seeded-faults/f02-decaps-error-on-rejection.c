/*
 * Seeded fault 2: decapsulation reports implicit rejection as an error.
 *
 * Synthetic example written by hand, not a model output.
 *
 * What it breaks: FIPS 203 section 6.3 says the implicit-rejection flag must
 * not be returned in any form. This version decapsulates correctly, then
 * recomputes J(z || c) and, if the library returned it (the ciphertext was
 * rejected), returns -2 instead of 0. The shared secret is still the
 * correct one, so only the return value betrays the rejection.
 *
 * Which oracles must catch it: implicit-rejection (every modified
 * ciphertext gives a nonzero return value) and acvp (the ACVP
 * decapsulation cases with a modified ciphertext, and the key-check cases,
 * which use a zero ciphertext that is itself rejected), and also
 * malformed-input: its length and hash checks come from the library and
 * pass, but its "dk_PKE bit flipped" control is a valid input that
 * decapsulates by implicit rejection, and this fault returns -2 there.
 * (The first version of this header predicted a pass for malformed-input;
 * the first selftest run showed the control is an implicit rejection,
 * which a unit test now asserts.)
 *
 * otb-fault: 2
 * otb-task: MLKEM-T24-decaps-implicit-rejection
 * otb-lib: mlkem
 * otb-expect: acvp fail wrong-return-code
 * otb-expect: malformed-input fail wrong-return-code
 * otb-expect: implicit-rejection fail wrong-return-code
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
  if (otbk_dec(ss, ct, dk) != 0) return -1;
  uint8_t zc[32 + 1088], k_bar[32];
  memcpy(zc, dk + 2400 - 32, 32);
  memcpy(zc + 32, ct, 1088);
  otbk_shake256(k_bar, 32, zc, sizeof zc);
  if (memcmp(k_bar, ss, 32) == 0) return -2; /* FAULT */
  return 0;
}

int randombytes(uint8_t *out, size_t outlen) {
  (void)out;
  (void)outlen;
  abort();
}
