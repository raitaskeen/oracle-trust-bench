/*
 * Valgrind memcheck probe: what the ct-valgrind oracle can and cannot see
 * (GAP-CT-001, docs/notes/ct-measurements.md). Trusted harness code, run by
 * crates/otb-harness/tests/ct_probes.rs inside the sandbox.
 *
 * Each mode marks its inputs undefined, calls one small function and marks
 * the result defined before printing it:
 *   d  division with secret operands          -> memcheck reports nothing
 *   s  select on a secret (conditional move)  -> memcheck reports nothing
 *   b  loop bound on a secret (branch)        -> "Conditional jump ..."
 *   i  table index from a secret (address)    -> "Use of uninitialised value"
 */
#include <stdint.h>
#include <stdio.h>
#include <valgrind/memcheck.h>

__attribute__((noinline)) uint32_t vdiv(uint32_t a, uint32_t b) { return a / b; }
__attribute__((noinline)) int32_t sel(int32_t r) { return r < 0 ? r + 3329 : r; }
__attribute__((noinline)) int32_t br(int32_t r) {
  int32_t n = 0;
  while (r >= 100) {
    r -= 100;
    n++;
  }
  return n;
}
__attribute__((noinline)) int32_t idx(const int32_t *t, uint32_t i) { return t[i & 7]; }

int main(int argc, char **argv) {
  static const int32_t t[8] = {1, 2, 3, 4, 5, 6, 7, 8};
  uint32_t a = 1000u + (uint32_t)argc, b = 7u + (uint32_t)argc;
  int32_t r = -5 + argc;
  if (argc < 2) return 2;
  VALGRIND_MAKE_MEM_UNDEFINED(&a, sizeof a);
  VALGRIND_MAKE_MEM_UNDEFINED(&b, sizeof b);
  VALGRIND_MAKE_MEM_UNDEFINED(&r, sizeof r);
  int64_t out = 0;
  switch (argv[1][0]) {
    case 'd': out = vdiv(a, b); break;
    case 's': out = sel(r); break;
    case 'b': out = br((int32_t)a); break;
    case 'i': out = idx(t, a); break;
    default: return 2;
  }
  VALGRIND_MAKE_MEM_DEFINED(&out, sizeof out);
  printf("%lld\n", (long long)out);
  return 0;
}
