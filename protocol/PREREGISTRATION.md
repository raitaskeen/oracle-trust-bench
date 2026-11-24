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

## To be written at M7

Hypotheses and outcome definitions as in the application; task sample and
oracle matrix (`protocol/ORACLE_MATRIX.md`); model tiers, prompts and
sampling; self-test validity rules (M5b); exclusions; statistical analysis
(task-clustered intervals, TOST at +-5 pp, Holm correction); power
(`otb power`, M6); deviations policy.
