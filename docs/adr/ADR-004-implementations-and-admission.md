# ADR-004: One path for every implementation, and candidate admission

Status: accepted (M4, 24 Nov 2026)

## Context

Three kinds of C code implement the pilot tasks: the reference adapters
(trusted, built on mlkem-native and mldsa-native), the hand-written seeded
faults (trusted, used to prove the oracles work) and candidates (untrusted,
written by models in the main study). If each kind had its own code path,
an oracle could behave differently on candidates than on the references
and faults that validate it.

Candidates also run in the same process as some oracles. From M5a the
Valgrind oracle marks secrets as undefined inside the candidate's process;
a candidate could undo that marking with Valgrind client requests (which
need inline assembly), run code before `main` (constructors, `ifunc`), or
include files the harness did not mean to give it.

## Decision

**One path.** Every implementation is an `Implementation { kind, source }`
with `kind` one of `Reference(lib)`, `SeededFault(lib or none)` and
`Candidate`. Every oracle takes an `Implementation`; `otb ref run`,
`otb selftest` and `otb check` call the same oracle functions.

What a build can see depends only on the kind:

| Kind | Admission check | Library staged | Staged files |
|---|---|---|---|
| Reference | no | yes | driver, `io.h`, `tasks.h`, the adapter as `impl.c`, the library |
| Seeded fault with a library | no | yes | same as a reference |
| Seeded fault without a library | **yes** | no | driver, `io.h`, `tasks.h`, `impl.c` |
| Candidate | **yes** | no | driver, `io.h`, `tasks.h`, `impl.c` |

A seeded fault without a library is therefore built exactly like a
candidate, so the faults also exercise the candidate path.

**Failures of the code under test are verdicts, not harness errors.** The
build script prints a marker between compiling and running. No marker
means `compile-error`; a signal or nonzero exit means `crash`; the
wall-clock limit (`TASK_WALL_LIMIT`, 120 s per build-and-run) means
`timeout`; output of the wrong size means `wrong-output`. A harness error
is reserved for failures unrelated to the code under test.

**Admission.** Before a candidate is compiled, `otb-harness::admission`
checks its source and, on the first violation in source order, returns a
stable policy name that becomes a `Rejected { policy }` verdict for every
applicable oracle. The rules:

1. at most 64 KiB; printable ASCII, tab, CR and LF only;
2. no trigraphs (`-std=c11` enables them);
3. after line splicing, with comments and literals skipped:
   - no `%:` digraph, no `##`, no stray backslash (each can spell a
     forbidden directive or identifier);
   - directives only from `define`, `elif`, `else`, `endif`, `error`, `if`,
     `ifdef`, `ifndef`, `include`, `undef`;
   - `#include` only of `limits.h`, `stdbool.h`, `stddef.h`, `stdint.h`,
     `stdlib.h`, `string.h` and `tasks.h`, spelled literally (`stdio.h` is
     excluded because the driver owns stdout);
   - no `asm`, `__asm`, `__asm__`, `_Pragma`, `__pragma`, `constructor`,
     `destructor`, `ifunc` (and their `__x__` forms), and no identifier
     containing `valgrind` in any case.

Candidate code runs only under the Docker backend; `otb check` refuses the
weaker fallback.

## Consequences

- An oracle cannot treat candidates differently from the faults that
  validate it.
- The admission check is lexical (GAP-ADM-001): it does not parse C or run
  the preprocessor. Its tests include the evasions it is meant to stop
  (line splicing, trigraphs, digraphs, token pasting, universal character
  names). The sandbox (ADR-003) remains the security boundary.
- How rejected candidates count in H1 must be fixed in the
  pre-registration (M7).
- Policy names are part of the verdict format and must stay stable.

## Evidence

- `crates/otb-harness/src/admission.rs`: one test per rule, plus evasion
  tests.
- `crates/otb-harness/src/selftest.rs`: the standalone seeded faults pass
  admission.
- `otb check` on a candidate including `stdio.h` returns
  `Rejected (header:stdio.h)`; on a candidate that never returns,
  `fail (timeout)` (recorded in `docs/notes/status-report.md`).
