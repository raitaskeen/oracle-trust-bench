# AI assistance

The code and documents in this repository were drafted with AI assistance
(Claude, by Anthropic, used through Claude Code). The author, Taskeen Haider,
reviews every change and is responsible for its content.

What this means in practice:

- AI-drafted code passes the same checks as any other code: formatting,
  lints with warnings as errors, tests, licence and advisory checks.
- Facts about standards (FIPS 203, FIPS 204, ACVP) were checked against the
  primary documents. The checks are recorded in
  `docs/notes/fips-verification.md`, with the hashes of the files read.
- The oracles must never depend on an LLM's judgment. The spec-literal
  reference implementations were drafted with AI assistance, so they are
  trusted only when they agree with mlkem-native, mldsa-native and ACVP
  (`KNOWN_GAPS.md`, GAP-REF-001).
- The study itself evaluates OpenAI models. Those models were not used to
  write this repository.
