# ADR-006: Model-written test suites, mutants and validity

Status: accepted for the pilot (M5b, 24 Nov 2026). **Draft for the
pre-registration**: the interface, the operators and the validity rules
below must be confirmed by the author before filing
(`protocol/PREREGISTRATION.md`, entry D2).

## Context

H1's denominator is "implementations that pass their own model-written
test suite". The application sets two rules for a suite to count: at least
five assertions, and it kills at least one mutant, with mutants "generated
from the verified reference". The study needs:

1. a fixed way for a model to write tests the harness can run and count;
2. a fixed, documented mutant set per task;
3. a runner that decides validity and runs the suite on the candidate.

## Decision

### Test-file interface

A suite is one C file that includes `otb_test.h` and `tasks.h`, defines
`main`, and checks the task function with `OTB_ASSERT(condition)`
(`c/harness/test-abi/`). Assertions are counted at run time by harness code
linked into the test program (`otb_test.c`), which prints
`OTB-TEST-RESULT assertions=<n> failures=<m>` at exit, so early returns and
`exit` calls still report. A failed assertion is recorded and the suite
continues. A run **passes** if the process exits with status 0, the
result line is present and no assertion failed. Each run has a 10 s limit.

Suites are admitted like candidates (ADR-004) with two differences:
`otb_test.h` is allowed, and `#define`/`#undef` of any name starting with
`OTB_` or `otb_` is rejected (policy `harness-name`).

### Mutation bases and mutants

Each task has a mutation base, `c/mutation-base/<task>.c`: a standalone
reference in C (T01, T07, T41, T47) or the reference decapsulation wrapper
around mlkem-native (T24). A base must pass every functional oracle of its
task; `otb mutants classify` refuses to run otherwise.

Mutants are first-order changes to one token in the base's marked region:
relational operator swaps, `+`/`-`, `<<`/`>>`, `&`/`|`/`^`, `&&`/`||`,
compound assignments, and integer literals +1 and -1
(`crates/otb-harness/src/mutants.rs`). Generation is deterministic.

Every mutant is run through the task's functional oracles once; the
result is committed as `c/mutation-base/<task>.mutants.json` (base hash,
every mutant, whether an oracle kills it and how). Only mutants an oracle
kills count when a suite is scored; the rest are treated as equivalent
(GAP-MUT-001). `otb mutants verify` re-runs the classification and fails if
it differs from the committed manifests.

### Validity and the self-test result

`otb suite --task <id> --tests <file> [--candidate <file>]`:

- the suite is run on the base and on every counted mutant in one sandbox
  job, and on the candidate in a second job with no library staged;
- the suite is **valid** if it is admitted, compiles, passes on the base
  with at least 5 assertions executed, and kills at least one counted
  mutant (fails, crashes, times out or does not build against it);
- the mutation score is killed / counted mutants;
- a candidate **passes its own tests** if a valid suite passes on it.

The rule "passes on the reference" is added to the application's two
rules: a suite that fails on a verified reference is testing something
other than the task.

## Evidence

- `otb selftest`: seeded fault 8 has a weak hand-written suite
  (`c/seeded-faults/suites/f08-tests.c`) that is valid by these rules and
  passes on the fault, while `reference-intermediate` fails it: false
  assurance in miniature.
- Unit tests for the generator, the manifest binding, the run-line parser
  and the validity rules; the committed manifests are checked against the
  generator on every test run.

## Consequences

- The mutant set, and so the validity rule, is fixed by committed files
  that the pre-registration can cite by hash.
- T24 mutants exercise the input checks and return codes, not the
  decapsulation arithmetic (GAP-MUT-001).
- Assertion counts are trustworthy for suites written in good faith
  (GAP-SUITE-001).
