/* Little-endian record I/O for the task drivers. */
#ifndef OTB_IO_H
#define OTB_IO_H

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

/* Reads exactly n bytes. Returns 1 on success, 0 on clean EOF before the
 * first byte, and exits with status 3 on a short read. */
static inline int otb_read(void *buf, size_t n) {
  size_t got = fread(buf, 1, n, stdin);
  if (got == n) return 1;
  if (got == 0 && feof(stdin)) return 0;
  fprintf(stderr, "otb driver: short read (%zu of %zu bytes)\n", got, n);
  exit(3);
}

static inline void otb_write(const void *buf, size_t n) {
  if (fwrite(buf, 1, n, stdout) != n) {
    fprintf(stderr, "otb driver: write failed\n");
    exit(4);
  }
}

static inline uint32_t otb_le32(const uint8_t b[4]) {
  return (uint32_t)b[0] | (uint32_t)b[1] << 8 | (uint32_t)b[2] << 16 |
         (uint32_t)b[3] << 24;
}

static inline void otb_put32(uint8_t b[4], uint32_t v) {
  b[0] = (uint8_t)v;
  b[1] = (uint8_t)(v >> 8);
  b[2] = (uint8_t)(v >> 16);
  b[3] = (uint8_t)(v >> 24);
}

static inline uint16_t otb_le16(const uint8_t b[2]) {
  return (uint16_t)(b[0] | b[1] << 8);
}

static inline void otb_put16(uint8_t b[2], uint16_t v) {
  b[0] = (uint8_t)v;
  b[1] = (uint8_t)(v >> 8);
}

#endif
