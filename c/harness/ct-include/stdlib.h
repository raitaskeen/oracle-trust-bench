/*
 * Minimal <stdlib.h> for compile-only cross-target builds of the
 * ct-static-division oracle (ADR-005). Declarations only; see string.h in
 * this directory and GAP-CT-005.
 */
#ifndef OTB_CT_STDLIB_H
#define OTB_CT_STDLIB_H

#include <stddef.h>

#define EXIT_SUCCESS 0
#define EXIT_FAILURE 1

typedef struct { int quot, rem; } div_t;
typedef struct { long quot, rem; } ldiv_t;
typedef struct { long long quot, rem; } lldiv_t;

void *malloc(size_t n);
void *calloc(size_t count, size_t n);
void *realloc(void *p, size_t n);
void free(void *p);
_Noreturn void abort(void);
_Noreturn void exit(int status);
int abs(int x);
long labs(long x);
long long llabs(long long x);
div_t div(int a, int b);
ldiv_t ldiv(long a, long b);
lldiv_t lldiv(long long a, long long b);
void qsort(void *base, size_t n, size_t size, int (*cmp)(const void *, const void *));
void *bsearch(const void *key, const void *base, size_t n, size_t size,
              int (*cmp)(const void *, const void *));

#endif
