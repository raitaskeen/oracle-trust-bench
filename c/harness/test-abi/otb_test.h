/*
 * The test-file interface for model-written test suites (ADR-006).
 *
 * A test file is one C file that includes "otb_test.h" and "tasks.h",
 * defines main, and checks the task function with OTB_ASSERT. Assertions
 * are counted while the tests run, by harness code (otb_test.c) that is
 * linked in; the counts are printed when the process exits, so a suite
 * that returns early or calls exit still reports what it executed.
 *
 *   #include "otb_test.h"
 *   #include "tasks.h"
 *
 *   int main(void) {
 *     OTB_ASSERT(compress_d(0, 1) == 0);
 *     OTB_ASSERT(compress_d(1665, 1) == 1);
 *     return 0;
 *   }
 *
 * A failed assertion is recorded and the suite continues. The suite
 * passes only if it exits with status 0 and no assertion failed.
 */
#ifndef OTB_TEST_H
#define OTB_TEST_H

void otb_assert_record(int ok, const char *expr, int line);

#define OTB_ASSERT(cond) otb_assert_record((cond) ? 1 : 0, #cond, __LINE__)

#endif
