/*
 * ACVP driver for the ML-DSA-65 reference (mldsa-native v2.0.0, portable C
 * backend). Used only to validate the reference build: no pilot task is a
 * top-level ML-DSA function.
 *
 * Each input record starts with one operation byte:
 *   'K'  seed[32]                          -> int32 ret, pk[1952], sk[4032]
 *   'S'  iface, rnd[32], sk[4032], ctx, msg -> int32 ret, sig[3309]
 *   'V'  iface, pk[1952], sig, ctx, msg     -> int32 ret
 * where iface is 0 (internal), 1 (internal, external mu) or 2 (external,
 * pure), and ctx, msg and sig are a uint32 length followed by the bytes.
 *
 * The external pure interface is ML-DSA.Sign / ML-DSA.Verify (FIPS 204
 * Algorithms 2 and 3): the internal functions applied to
 * 0 || len(ctx) || ctx || M, passed here as mldsa-native's prefix.
 */
#define MLD_CONFIG_PARAMETER_SET 65
#define MLD_CONFIG_NAMESPACE_PREFIX otbd
#include "mldsa_native.c"

#include <stdlib.h>
#include <string.h>

#include "io.h"

#define PK 1952
#define SK 4032
#define SIG 3309
#define MAXLEN 65536u

int randombytes(uint8_t *out, size_t outlen) {
  (void)out;
  (void)outlen;
  abort();
}

static uint32_t read_var(uint8_t *buf) {
  uint8_t len[4];
  if (!otb_read(len, 4)) exit(5);
  uint32_t n = otb_le32(len);
  if (n > MAXLEN || (n && !otb_read(buf, n))) exit(5);
  return n;
}

int main(void) {
  static uint8_t ctx[MAXLEN], msg[MAXLEN], sigin[MAXLEN], pre[2 + 255];
  uint8_t op, out4[4];
  while (otb_read(&op, 1)) {
    if (op == 'K') {
      uint8_t seed[32], pk[PK], sk[SK];
      if (!otb_read(seed, 32)) return 5;
      int ret = otbd_keypair_internal(pk, sk, seed);
      otb_put32(out4, (uint32_t)ret);
      otb_write(out4, 4);
      otb_write(pk, PK);
      otb_write(sk, SK);
      continue;
    }
    uint8_t iface;
    if (!otb_read(&iface, 1)) return 5;
    if (op == 'S') {
      uint8_t rnd[32], sk[SK], sig[SIG];
      if (!otb_read(rnd, 32) || !otb_read(sk, SK)) return 5;
      uint32_t ctxlen = read_var(ctx), mlen = read_var(msg);
      size_t prelen = 0;
      if (iface == 2) {
        if (ctxlen > 255) return 6;
        pre[0] = 0;
        pre[1] = (uint8_t)ctxlen;
        memcpy(pre + 2, ctx, ctxlen);
        prelen = 2 + ctxlen;
      }
      memset(sig, 0, SIG);
      int ret = otbd_signature_internal(sig, msg, mlen, prelen ? pre : NULL,
                                        prelen, rnd, sk, iface == 1);
      otb_put32(out4, (uint32_t)ret);
      otb_write(out4, 4);
      otb_write(sig, SIG);
    } else if (op == 'V') {
      uint8_t pk[PK];
      if (!otb_read(pk, PK)) return 5;
      uint32_t siglen = read_var(sigin), ctxlen = read_var(ctx),
               mlen = read_var(msg);
      size_t prelen = 0;
      if (iface == 2) {
        if (ctxlen > 255) return 6;
        pre[0] = 0;
        pre[1] = (uint8_t)ctxlen;
        memcpy(pre + 2, ctx, ctxlen);
        prelen = 2 + ctxlen;
      }
      int ret = siglen == SIG
                    ? otbd_verify_internal(sigin, msg, mlen,
                                           prelen ? pre : NULL, prelen, pk,
                                           iface == 1)
                    : -1;
      otb_put32(out4, (uint32_t)ret);
      otb_write(out4, 4);
    } else {
      return 7;
    }
  }
  return 0;
}
