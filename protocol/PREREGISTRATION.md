# Pre-registration (DRAFT)

**Status: draft. Not filed. Nothing here is final until the author files
the pre-registration on OSF before any model is run.** No model has been
run; no results yet.

This file collects, one decision at a time, what the pre-registration will
fix. The study design, hypotheses H1 to H3 and the analysis plan are those
of `protocol/APPLICATION.md`; the full text is written at M7. Each entry
names the decision record it comes from.

## Decided so far

### D1. Constant-time outcome (ADR-005; author's decision Q3, 24 Nov 2026)

For H1 and H2, a candidate fails "constant-time checks" if either of the
following holds.

- `ct-valgrind`: Valgrind memcheck reports a secret-dependent branch or
  memory address in a build at -O2 or at -O3 (GCC 14.2.0, x86-64, no
  sanitizers), with secrets marked as listed in the task spec.
- `ct-static-division`: the emitted machine code at -O2 or -O3, for any of
  the configurations in ADR-005, contains a division instruction or a
  division helper call reachable from the task function.

A violation at either optimisation level counts. Source and IR analysis is
a secondary diagnostic and is never used alone. dudect results are
reported separately and do not enter H1.

**Open, for the author to confirm before filing:**

1. the target list (GCC x86-64; Clang x86-64, AArch64, Cortex-M4,
   Cortex-M0+), in particular whether Cortex-M0+ stays: it is the only
   target where the KyberSlash pattern survives at -O2 and -O3
   (`docs/notes/ct-measurements.md`, measurement 1);
2. the reachability rule (only divisions reachable from the task function
   count);
3. how a division on public values in reachable code counts (today it
   counts; GAP-CT-002);
4. how candidates rejected by admission count in H1 (ADR-004,
   GAP-ADM-001).

### D2. Self-test validity and the H1 denominator (ADR-006)

A model-written test suite uses the interface in
`c/harness/test-abi/otb_test.h` and counts as a test suite only if it:

1. passes the admission policy for test files;
2. compiles and **passes on the task's verified reference** (the mutation
   base in `c/mutation-base/`);
3. executes **at least 5 assertions** on the reference;
4. **kills at least one mutant** from the committed mutant set
   (`c/mutation-base/<task>.mutants.json`; only mutants that a functional
   oracle kills count).

Rules 3 and 4 are from the application; rule 2 is added because a suite
that fails on a verified reference is not testing the task. An
implementation "passes its own model-written test suite" (H1's
denominator) if its suite is valid and passes on it. The mutation score
(killed / counted mutants) is reported for every suite.

**Open, for the author to confirm before filing:**

1. the added rule 2;
2. the mutation operators and the committed mutant sets (121 mutants, 103
   counted; GAP-MUT-001), cited by the manifests' base hashes;
3. how implementations with an invalid suite are reported (today: outside
   the H1 denominator, counted separately).

## To be written at M7

Hypotheses and outcome definitions as in the application; task sample and
oracle matrix (`protocol/ORACLE_MATRIX.md`); model tiers, prompts and
sampling; self-test validity rules (M5b); exclusions; statistical analysis
(task-clustered intervals, TOST at +-5 pp, Holm correction); power
(`otb power`, M6); deviations policy.
