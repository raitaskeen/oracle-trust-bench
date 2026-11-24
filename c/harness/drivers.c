/*
 * Task drivers. Build with -DOTB_TASK_<NAME> to select one, plus exactly one
 * implementation of that task's function. See README.md for record formats.
 *
 * With -DOTB_CT_VALGRIND the driver marks the task's secret inputs as
 * undefined for Valgrind memcheck before each call, and marks the outputs
 * defined after it, so memcheck reports any branch or memory address inside
 * the implementation that depends on a secret (ADR-005). Which inputs are
 * secret comes from each task spec's [secrets] section. Marking is done
 * here, never in the implementation, and candidates cannot use Valgrind
 * client requests (ADR-004).
 */
#include <string.h>

#include "io.h"
#include "tasks.h"

#if defined(OTB_CT_VALGRIND)
#include <valgrind/memcheck.h>
#define OTB_SECRET(p, n) VALGRIND_MAKE_MEM_UNDEFINED((p), (n))
#define OTB_PUBLIC(p, n) VALGRIND_MAKE_MEM_DEFINED((p), (n))
#else
#define OTB_SECRET(p, n) ((void)(p), (void)(n))
#define OTB_PUBLIC(p, n) ((void)(p), (void)(n))
#endif

#if defined(OTB_TASK_T01)
/* Secret: f (every coefficient). */
int main(void) {
  uint8_t buf[512];
  int16_t f[256];
  while (otb_read(buf, sizeof buf)) {
    for (int i = 0; i < 256; i++) f[i] = (int16_t)otb_le16(buf + 2 * i);
    OTB_SECRET(f, sizeof f);
    ntt(f);
    OTB_PUBLIC(f, sizeof f);
    for (int i = 0; i < 256; i++) otb_put16(buf + 2 * i, (uint16_t)f[i]);
    otb_write(buf, sizeof buf);
  }
  return 0;
}

#elif defined(OTB_TASK_T07)
/* Secret: x. Public: d. */
int main(void) {
  uint8_t in[3], out[2];
  while (otb_read(in, sizeof in)) {
    uint16_t x = otb_le16(in);
    OTB_SECRET(&x, sizeof x);
    uint16_t y = compress_d(x, (unsigned)in[2]);
    OTB_PUBLIC(&y, sizeof y);
    otb_put16(out, y);
    otb_write(out, sizeof out);
  }
  return 0;
}

#elif defined(OTB_TASK_T24)
/* Secret: dk_PKE and z. Public: ct, and the ek and H(ek) parts of dk
 * (FIPS 203 Algorithm 19: dk = dk_PKE || ek || H(ek) || z, with ek the
 * public encapsulation key). Marked only for a key of the valid length. */
#define OTB_MAX_LEN 65536u
#define OTB_DK_LEN 2400u
#define OTB_DK_PKE_LEN 1152u
int main(void) {
  static uint8_t ct[OTB_MAX_LEN], dk[OTB_MAX_LEN];
  uint8_t len[4], out[4 + 32];
  while (otb_read(len, 4)) {
    uint32_t ct_len = otb_le32(len);
    if (ct_len > OTB_MAX_LEN || (ct_len && !otb_read(ct, ct_len))) return 5;
    if (!otb_read(len, 4)) return 5;
    uint32_t dk_len = otb_le32(len);
    if (dk_len > OTB_MAX_LEN || (dk_len && !otb_read(dk, dk_len))) return 5;
    uint8_t ss[32];
    memset(ss, 0xAA, sizeof ss);
    if (dk_len == OTB_DK_LEN) {
      OTB_SECRET(dk, OTB_DK_PKE_LEN);
      OTB_SECRET(dk + OTB_DK_LEN - 32, 32);
    }
    int ret = mlkem768_decaps(ss, ct, ct_len, dk, dk_len);
    OTB_PUBLIC(&ret, sizeof ret);
    OTB_PUBLIC(ss, sizeof ss);
    OTB_PUBLIC(dk, dk_len);
    otb_put32(out, (uint32_t)ret);
    memcpy(out + 4, ss, 32);
    otb_write(out, sizeof out);
  }
  return 0;
}

#elif defined(OTB_TASK_T41)
/* Secret: r. Outputs declassified after the call (r1 is public; r0 leaves
 * the function and is not checked further here). */
int main(void) {
  uint8_t in[4], out[8];
  while (otb_read(in, sizeof in)) {
    int32_t r = (int32_t)otb_le32(in), r1 = 0, r0 = 0;
    OTB_SECRET(&r, sizeof r);
    power2round(r, &r1, &r0);
    OTB_PUBLIC(&r1, sizeof r1);
    OTB_PUBLIC(&r0, sizeof r0);
    otb_put32(out, (uint32_t)r1);
    otb_put32(out + 4, (uint32_t)r0);
    otb_write(out, sizeof out);
  }
  return 0;
}

#elif defined(OTB_TASK_T47)
/* Secret: z and r. Public: gamma2 and the return value (GAP-CT-004). */
int main(void) {
  uint8_t in[8], out[4];
  while (otb_read(in, sizeof in)) {
    int32_t z = (int32_t)otb_le32(in), r = (int32_t)otb_le32(in + 4);
    OTB_SECRET(&z, sizeof z);
    OTB_SECRET(&r, sizeof r);
    int h = make_hint(z, r, (8380417 - 1) / 32);
    OTB_PUBLIC(&h, sizeof h);
    otb_put32(out, (uint32_t)h);
    otb_write(out, sizeof out);
  }
  return 0;
}

#else
#error "define one of OTB_TASK_T01, T07, T24, T41, T47"
#endif
