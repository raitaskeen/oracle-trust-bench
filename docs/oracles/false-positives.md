# Oracle false positives on the references

An oracle's false-positive rate is measured on the unmodified references,
which are correct by three-way agreement (spec-literal Rust, the pinned C
libraries and ACVP; GAP-REF-001). A failure on a reference is a false
positive unless it reveals a real defect in the reference.

Measured on 24 Nov 2026 with `otb ref run` (Docker backend; image in
`docs/notes/environment.md`), at the commit that closed M5a.

| Oracle | Tasks it applies to | References failing | Rate |
|---|---|---|---|
| reference-intermediate | T01, T07, T41, T47 | 0 | 0/4 |
| acvp | T24 (and the ML-DSA-65 library check) | 0 | 0/1 (0/1) |
| malformed-input | T24 | 0 | 0/1 |
| implicit-rejection | T24 | 0 | 0/1 |
| ct-valgrind | T01, T07, T24, T41, T47 | 0 | 0/5 |
| ct-static-division | T01, T07, T24, T41, T47 | 0 (10 configurations each) | 0/5 |
| ct-dudect | T07, T24 (subsample) | not implemented (nightly only) | - |

## Notes

- **ct-static-division, whole-file rule (superseded).** Counting every
  division in the implementation file gave 3/5 false positives (T01, T07,
  T24): an unused copy of `otbk_shake256x4` in mlkem-native divides a
  public length by 136 on Cortex-M0+. Restricting the rule to code
  reachable from the task function (ADR-005) removed them; the report
  still lists the unreached division as "not counted"
  (`docs/notes/ct-measurements.md`, measurement 3).
- **Reference adapters.** Two adapters were changed in M5a so that they do
  not add non-constant-time code around the library: T01 now reduces with
  mlkem-native's `poly_reduce` instead of `%` and a branch, and T47
  reduces r + z with a sign mask instead of `%` and a branch. Their
  functional results are unchanged (`otb ref run`, reference-intermediate).
- **Small reference set.** Five references are a small sample. A rate of
  0/5 is evidence that the oracles do not misfire on careful constant-time
  code, not an estimate of the false-positive rate on candidates
  (GAP-CT-002 describes how `ct-static-division` can over-report).
