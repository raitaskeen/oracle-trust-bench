/*
 * Task drivers. Build with -DOTB_TASK_<NAME> to select one, plus exactly one
 * implementation of that task's function. See README.md for record formats.
 */
#include <string.h>

#include "io.h"
#include "tasks.h"

#if defined(OTB_TASK_T01)
int main(void) {
  uint8_t buf[512];
  int16_t f[256];
  while (otb_read(buf, sizeof buf)) {
    for (int i = 0; i < 256; i++) f[i] = (int16_t)otb_le16(buf + 2 * i);
    ntt(f);
    for (int i = 0; i < 256; i++) otb_put16(buf + 2 * i, (uint16_t)f[i]);
    otb_write(buf, sizeof buf);
  }
  return 0;
}

#elif defined(OTB_TASK_T07)
int main(void) {
  uint8_t in[3], out[2];
  while (otb_read(in, sizeof in)) {
    otb_put16(out, compress_d(otb_le16(in), (unsigned)in[2]));
    otb_write(out, sizeof out);
  }
  return 0;
}

#elif defined(OTB_TASK_T24)
#define OTB_MAX_LEN 65536u
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
    int ret = mlkem768_decaps(ss, ct, ct_len, dk, dk_len);
    otb_put32(out, (uint32_t)ret);
    memcpy(out + 4, ss, 32);
    otb_write(out, sizeof out);
  }
  return 0;
}

#elif defined(OTB_TASK_T41)
int main(void) {
  uint8_t in[4], out[8];
  while (otb_read(in, sizeof in)) {
    int32_t r1 = 0, r0 = 0;
    power2round((int32_t)otb_le32(in), &r1, &r0);
    otb_put32(out, (uint32_t)r1);
    otb_put32(out + 4, (uint32_t)r0);
    otb_write(out, sizeof out);
  }
  return 0;
}

#elif defined(OTB_TASK_T47)
int main(void) {
  uint8_t in[8], out[4];
  while (otb_read(in, sizeof in)) {
    int h = make_hint((int32_t)otb_le32(in), (int32_t)otb_le32(in + 4),
                      (8380417 - 1) / 32);
    otb_put32(out, (uint32_t)h);
    otb_write(out, sizeof out);
  }
  return 0;
}

#else
#error "define one of OTB_TASK_T01, T07, T24, T41, T47"
#endif
