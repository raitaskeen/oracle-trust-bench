/*
 * C interfaces of the pilot tasks, exactly as in tasks/<family>/<task>/spec.md.
 * A candidate, a reference adapter or a seeded fault implements one of them.
 */
#ifndef OTB_TASKS_H
#define OTB_TASKS_H

#include <stddef.h>
#include <stdint.h>

/* MLKEM-T01-ntt (FIPS 203 Algorithm 9) */
void ntt(int16_t f[256]);

/* MLKEM-T07-compress-d (FIPS 203 Eq. 4.7) */
uint16_t compress_d(uint16_t x, unsigned d);

/* MLKEM-T24-decaps-implicit-rejection (FIPS 203 Algorithm 21, section 7.3) */
int mlkem768_decaps(uint8_t ss[32], const uint8_t *ct, size_t ct_len,
                    const uint8_t *dk, size_t dk_len);

/* MLDSA-T41-power2round (FIPS 204 Algorithm 35) */
void power2round(int32_t r, int32_t *r1, int32_t *r0);

/* MLDSA-T47-make-hint (FIPS 204 Algorithm 39) */
int make_hint(int32_t z, int32_t r, int32_t gamma2);

#endif
