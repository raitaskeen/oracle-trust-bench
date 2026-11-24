/*
 * Harness side of the test-file interface (otb_test.h, ADR-006). Linked
 * with every test suite; never written by a model and not subject to
 * admission.
 *
 * The counters are private to this file. At exit, one line is written to
 * standard output:
 *
 *   OTB-TEST-RESULT assertions=<n> failures=<m>
 *
 * and each failed assertion is reported on standard error as
 * "OTB-TEST-FAIL line <l>: <expression>" (at most 20 of them).
 */
#include <stdio.h>
#include <stdlib.h>

#include "otb_test.h"

static unsigned long otb_assertions;
static unsigned long otb_failures;

void otb_assert_record(int ok, const char *expr, int line) {
  otb_assertions++;
  if (!ok) {
    otb_failures++;
    if (otb_failures <= 20) {
      fprintf(stderr, "OTB-TEST-FAIL line %d: %s\n", line, expr);
    }
  }
}

static void otb_report(void) {
  printf("OTB-TEST-RESULT assertions=%lu failures=%lu\n", otb_assertions,
         otb_failures);
  fflush(stdout);
}

/* Runs before main, so the report is registered whatever main does. */
__attribute__((constructor)) static void otb_register(void) {
  atexit(otb_report);
}
