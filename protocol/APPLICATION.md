<!-- Public copy of the application text submitted on 24 Nov 2026. Changes from the submitted text: the faculty advisor's email address is removed. Nothing else is changed. -->

# OpenAI Researcher Access Program: final application text (v2, 24 Nov 2026)

Applicant: Taskeen Haider · Virtual University of Pakistan · BS Software Engineering (undergraduate)
Title: The Oracle-Trust Gap: Verifying LLM-Assisted Modernization of Cryptographic Code
Research areas to tick: Misuse potential, Alignment. Optional: Human-AI interaction, framed as over-reliance on model-written tests. If the live form shows Robustness, tick it. Never tick Interpretability.
ORCID: 0009-0009-2783-4092 · Faculty advisor: Dr. Muhammad Anwaar Saeed, BS Cyber Security program, VU, confirmed in writing

---

## 1. Research Question

### RQ-600 (≈600 chars)
When OpenAI models write post-quantum cryptographic code in C and also write its tests, do passing tests justify trust? I will measure how often an ML-KEM (FIPS 203) or ML-DSA (FIPS 204) component passes its own model-written tests but fails an independent, non-LLM check that applies to it: reference intermediate values, NIST ACVP vectors, malformed-input and implicit-rejection tests, or constant-time checks. I will measure the same gap for model migration of small C routines to Rust.

### RQ-1500 (≈1,500 chars)
When OpenAI models write post-quantum cryptographic code in C and also write the tests for it, do passing tests justify trust? I will measure this for components of ML-KEM (FIPS 203) and ML-DSA (FIPS 204), and for model-assisted migration of small crypto-adjacent C routines to Rust.

H1 (false assurance): among implementations that pass their own model-written test suite, the share that fails at least one applicable independent check exceeds 5%. The independent checks are reference intermediate values, ACVP vectors, malformed-input and implicit-rejection tests, and constant-time checks. H1 is supported if the lower bound of the task-clustered 95% confidence interval is above 5%. I expect a rate near 15%.

H2 (prompt robustness): compared with plain prompts, security-focused prompts change the rate of constant-time and implicit-rejection violations by less than 5 percentage points. I will test this with a TOST equivalence test at ±5 pp.

H3 (silent drift): for C-to-Rust migrations, the pass rate under model-written characterization tests exceeds the pass rate under differential fuzzing against the C original by at least 10 percentage points.

### RQ-full
RQ-1500, followed by this paragraph:

A study of LLM agents that built ML-KEM and ML-DSA hardware found that passing module tests overstated verification strength (arXiv:2611.25339). I ask whether the same gap holds for software. In software, the failures that matter most are secret-dependent timing, missing input validation and wrong implicit-rejection behavior, and a known-answer test cannot see any of them. I also ask whether stronger models or security prompting close the gap.

---

## 2. Research Description and Methodology

### M-600
An evaluation study, not a software project. Three OpenAI model tiers implement 60 ML-KEM/ML-DSA components in C under three prompt conditions, five samples each (2,700 implementations), then write tests for their own code in a fresh context. I compare self-test results with per-task non-LLM oracles: reference intermediate values, ACVP vectors, malformed-input and implicit-rejection tests, Valgrind constant-time checks. A secondary arm measures drift in model C-to-Rust migration. To be pre-registered on OSF; all data public.

### M-1500
This is an evaluation study. It produces a public dataset and a paper, not software for sale.

**Arm 1 (core):** 60 C tasks (36 ML-KEM, 24 ML-DSA), from the NTT and encoding up to encapsulation, decapsulation with implicit rejection, and ML-DSA rounding and hints. Three prompt conditions, three OpenAI model tiers and five samples per cell give 2,700 implementations. Each specification is validated by running the reference implementation through the harness.

**Arm 2 (core):** in a fresh context, each model writes tests for each of its own implementations. A suite counts only if it has a minimum number of assertions and kills at least one mutant of the reference. Every implementation then goes through the independent checks that apply to its task, set in a pre-registered task-by-oracle matrix:
- reference intermediate values from mlkem-native and mldsa-native;
- ACVP vectors for top-level functions;
- malformed-input and implicit-rejection tests for encapsulation and decapsulation;
- Valgrind constant-time checks on non-sanitized -O2 and -O3 builds, plus a static check for division on secret data (the KyberSlash class).

Cross-tier model audits test whether review catches what self-tests miss.

**Arm 3 (secondary):** 40 open-source C routines, five samples per tier. Models write characterization tests, then migrate each routine to Rust. Differential fuzzing and seeded faults then measure drift.

**Analysis:** mixed-effects regression with a simulation-based power analysis, filed on OSF before the main run.

### M-full

**Motivation.**
- Developers are starting to use language models to implement and migrate cryptographic code. A green test suite is often the only evidence that the code is correct.
- Model-written crypto code is often insecure. In a 7-model Python benchmark, explicit secure prompting reduced the pooled vulnerability rate only from 74.2% (neutral prompts) to 71.7% ([CIPHER, arXiv:2602.01438](https://arxiv.org/abs/2602.01438)).
- Model-written regression tests can encode faults as expected behavior. In one study, 8–17% of generated tests were fault-enforcing, against 2.4–4.8% fault-revealing ([arXiv:2610.11835](https://arxiv.org/abs/2610.11835)).
- The closest work studied LLM agents that build ML-KEM and ML-DSA FPGA hardware. It found that passing module tests overstated verification strength ([arXiv:2611.25339](https://arxiv.org/abs/2611.25339)).
- I found no study that measures this gap for software implementations across model tiers and prompt conditions. In software, secret-dependent timing and implicit-rejection behavior are invisible to known-answer tests.

This is an evaluation study. It produces a public dataset and a paper, not software for sale.

**Arm 1: generation (core).**
- **Tasks:** 60 specifications, 36 for ML-KEM and 24 for ML-DSA. They cover the NTT, compression and encoding, noise and rejection sampling, K-PKE, encapsulation with input validation, decapsulation with implicit rejection, and ML-DSA rounding, hint and norm-check routines.
- **Task format:** each task is a C function with a fixed harness interface, written against a self-contained summary of the public FIPS text.
- **Spec validation:** each specification is validated by running the reference implementation through the harness.
- **Design:** three prompt conditions (plain; security-focused; specification plus an explicit checklist), three OpenAI model tiers (gpt-6-luna, gpt-6.2-sol, gpt-6.2-astra) and five samples per cell give 2,700 implementations.
- **Pinning:** snapshot IDs and reasoning-effort settings are pinned and recorded.
- **Contamination:** to estimate memorization, I record verbatim overlap with public reference code. I also include renamed and re-parameterized variants of 10 tasks.

**Arm 2: oracle trust (core).**
- **Self-tests:** in a fresh context, each model writes a unit test suite for each of its own implementations. A suite counts as a real test suite only if it has at least five assertions and kills at least one mutant. The mutants are generated from the verified reference, and the suite is scored by how many it kills.
- **Independent checks:** every implementation goes through the non-LLM oracles that apply to its task, set in a pre-registered task-by-oracle matrix:
  - intermediate values from instrumented mlkem-native and mldsa-native;
  - NIST ACVP vectors, for top-level functions only;
  - malformed-input tests (wrong-length keys and ciphertexts, non-canonical coefficients) and implicit-rejection tests, for encapsulation and decapsulation only;
  - constant-time checks, for secret-dependent routines only. These use Valgrind memcheck with secrets marked undefined, on non-sanitized builds at -O2 and -O3 from one pinned compiler. A static check flags division or modulo on secret operands. dudect runs on a pre-registered subsample with a fixed measurement budget and threshold.
- **Oracle validation:** I report each oracle's false-positive rate on known-good reference code.
- **False assurance:** an implementation counts as false assurance if it passes its self-tests and fails any applicable oracle.
- **Model audits:** cross-tier audits test whether review catches what self-tests miss. gpt-6.2-sol audits all luna and astra implementations, and gpt-6.2-astra audits a stratified 650 of sol's. No model audits its own code, and no verdict depends on an LLM judgment.

**Arm 3: legacy migration (secondary).**
- **Routines:** 40 small crypto-adjacent C routines from permissively licensed open-source projects: constant-time comparison, padding checks, DER integer parsing, and endianness and serialization helpers.
- **Design:** each model tier writes characterization tests for each routine and then migrates it to Rust, five samples per routine.
- **Pass definition:** a migration passes differential fuzzing if libFuzzer/cargo-fuzz finds no output or return-code divergence from the C original within a fixed, pre-registered execution budget.
- **Seeded faults:** these count fault-enforcing tests.
- **Drift classification:** drift is classified as integer width and signedness, endianness, error-code semantics, or reliance on undefined behavior. A second coder labels 20% of cases, and I report Cohen's kappa.

**Analysis.**
- Wilson 95% intervals for rates.
- Mixed-effects logistic regression, with task as a random effect and model tier and prompt condition as fixed effects.
- TOST for H2, and Holm correction across H1–H3.
- Because the samples are nested in tasks, a simulation-based power analysis with an assumed intraclass correlation goes into the OSF pre-registration. Tier-by-prompt interactions are exploratory.
- Null and negative results will be reported.

**Scope control.** ML-KEM within Arms 1–2 is the core and runs first. If time runs short, ML-DSA tasks and Arm 3 are reduced, not dropped silently. Any reduction is recorded against the pre-registration.

**Preliminary work.**
- I maintain an open-source (MIT) static-analysis engine for JavaScript and TypeScript, with 2,507 passing tests.
- Its CI runs two runtime oracles. They execute real test suites and fail a change if any static claim contradicts observed behavior.
- Arm 2 applies the same principle: claims are checked against independent execution, not against the system that made them.

**Safety.**
- Only synthetic keys are used.
- I test no deployed system and write no code intended to defeat security controls.
- Implementations that fail verification are released only in a clearly labeled, non-installable research dataset.
- Flaws found in third-party libraries go to their maintainers through coordinated disclosure.
- Any safety or security issue in the OpenAI API is reported immediately through OpenAI's Coordinated Vulnerability Disclosure Program.

---

## 3. Requested Credit and Justification

### B-600
I request $955.98, computed by a script from Standard list prices fetched on 24 Nov 2026, with no cache or batch discounts. That covers about 42.5M input and 42.5M output tokens over 9,050 calls. The comparison across three model tiers is the central question, and gpt-6.2-astra alone costs about $651. I am an undergraduate with no institutional research funding, and I can spend under $20 a month on API use. That would cover at most one mid-tier model over a year and none of the flagship tier.

### B-1500
I request $955.98. A script published with the study computes the estimate from Standard list prices fetched on 24 Nov 2026. It applies no cached-input, Batch or background discounts, so the estimate is conservative. Output counts include reasoning tokens.

- Arm 1, implementations (2,700 calls): $304.52
- Arm 2, self-written tests (2,700 calls): $272.70
- Arm 2, cross-tier model audits (2,450 calls): $184.80
- Arm 3, characterization tests and migrations (1,200 calls): $107.06
- Pilot and prompt development (5%): $43.45
- Rerun reserve (5%): $43.45

By model, before pilot and reserve: gpt-6.2-astra $651.20, gpt-6.2-sol $211.10, gpt-6-luna $6.78. The comparison across three tiers is the central question, so the flagship tier cannot be dropped.

I am an undergraduate at the Virtual University of Pakistan with no institutional research budget, compute allocation or API funding. I can personally spend under $20 a month on API use, which would cover at most one mid-tier model over a year and none of the flagship tier. Without credits, H1 and H2 could not be tested across tiers.

All credits will be spent within 12 months, mostly in months 1–7. None will go to fine-tuning or to any other work.

### B-full
B-1500, followed by the token table:

| Stage | Model | Calls | Input (M) | Output (M) | USD |
|---|---|---:|---:|---:|---:|
| Arm 1 implementations | gpt-6-luna | 900 | 3.15 | 5.40 | 3.02 |
| Arm 1 implementations | gpt-6.2-sol | 900 | 3.15 | 5.40 | 60.30 |
| Arm 1 implementations | gpt-6.2-astra | 900 | 3.15 | 5.40 | 241.20 |
| Arm 2 self-tests | gpt-6-luna | 900 | 4.50 | 4.50 | 2.70 |
| Arm 2 self-tests | gpt-6.2-sol | 900 | 4.50 | 4.50 | 54.00 |
| Arm 2 self-tests | gpt-6.2-astra | 900 | 4.50 | 4.50 | 216.00 |
| Arm 2 audit of luna+astra code | gpt-6.2-sol | 1,800 | 10.80 | 5.40 | 75.60 |
| Arm 2 audit of sol code (650) | gpt-6.2-astra | 650 | 3.90 | 1.95 | 109.20 |
| Arm 3 tests + migration | gpt-6-luna | 400 | 1.60 | 1.80 | 1.06 |
| Arm 3 tests + migration | gpt-6.2-sol | 400 | 1.60 | 1.80 | 21.20 |
| Arm 3 tests + migration | gpt-6.2-astra | 400 | 1.60 | 1.80 | 84.80 |
| Subtotal | | 9,050 | 42.45 | 42.45 | 869.08 |
| Pilot (5%) + reserve (5%) | | | | | 86.90 |
| **Total** | | | | | **955.98** |

Prices are USD per 1M tokens. gpt-6.2-astra costs $8 in / $40 out, gpt-6.2-sol $2 / $10, and gpt-6-luna $0.10 / $0.50, all from [OpenAI Models](https://developers.openai.com/api/docs/models).

---

## 4. Dissemination Plan

### D-600
Everything will be public at github.com/raitaskeen/oracle-trust-bench: task specs, prompts, the containerized harness, raw generations with model and date, analysis code and the OSF pre-registration. Code is MIT and data is CC BY 4.0, subject to upstream licenses. Failing implementations go in a labeled, non-installable dataset. I will post an arXiv preprint (cs.CR, cross-listed to cs.SE and cs.AI), submit it to a security or AI-safety workshop, and report null results. Publication follows OpenAI's sharing and publication policy.

### D-1500
**What I will publish.** All artifacts will be public at github.com/raitaskeen/oracle-trust-bench:
- task specifications and the task-by-oracle matrix;
- prompts;
- the containerized verification harness and the wrappers for the independent oracles;
- raw model generations, with model snapshot and date;
- analysis code and the OSF pre-registration.

**Licenses.** Code is released under MIT and data under CC BY 4.0, subject to the licenses of upstream test vectors and source files.

**Failing code.** Implementations that fail verification go into a separate, clearly labeled dataset with a warning README. They are never packaged as installable libraries.

**Papers.** I will post an arXiv preprint (cs.CR, cross-listed to cs.SE and cs.AI) and submit it to a software-security or AI-safety workshop. Null and negative results will be reported.

**OpenAI's policy.** Publication follows OpenAI's sharing and publication policy. I will review generations before sharing them, attribute content to myself and state clearly how AI was used.

**Disclosure.** Any safety or security issue in the API goes immediately to OpenAI's Coordinated Vulnerability Disclosure Program. Flaws in third-party libraries go to their maintainers through coordinated disclosure.

---

## 5. Timeline

### T-600
- **Before credits (no API spend):** harness on mlkem-native, mldsa-native and ACVP vectors; task specs; OSF draft.
- **Month 1:** freeze tasks and prompts, pilot on 5 tasks, file the pre-registration ($43.45).
- **Months 2–3:** Arm 1 ($304.52).
- **Months 4–5:** Arm 2 ($457.50).
- **Months 6–7:** Arm 3 ($107.06).
- **Month 8:** analysis, reruns from the reserve ($43.45), arXiv preprint.
- **Month 9:** v1.0 release and workshop submission.
- **Months 10–12:** buffer.

### T-full
| When | Milestone | Deliverable | API spend (USD) |
|---|---|---|---|
| Before credits | Harness on mlkem-native, mldsa-native and ACVP vectors; task specs validated against the reference; OSF draft | Public repo with protocol and pilot tasks | 0 |
| Month 1 | Freeze tasks, prompts and the task-by-oracle matrix; pilot on 5 tasks; calibrate token counts | OSF pre-registration filed | 43.45 |
| Months 2–3 | Arm 1: generate and verify 2,700 implementations (ML-KEM first) | Verified Arm 1 dataset | 304.52 |
| Months 4–5 | Arm 2: self-written tests, mutation scores, cross-tier audits | H1 and H2 results | 457.50 |
| Months 6–7 | Arm 3: characterization tests and C-to-Rust migration | Drift taxonomy, H3 results | 107.06 |
| Month 8 | Analysis and writing; reruns from the reserve | arXiv preprint | 43.45 |
| Month 9 | Release v1.0; disclosures; workshop submission | Public dataset and harness | 0 |
| Months 10–12 | Buffer; replication on newer model versions only if credits remain | Updated results | 0 |
| **Total** | | | **955.98** |

---

## 6. Disclosure (conflict of interest / additional information)

### X-600
I am one of three co-founders of LegacyExodus, an early-stage company in Pakistan founded in August 2026. It builds static analysis for legacy JavaScript and TypeScript, and its engine is open source under MIT. It plans a characterization-test offering, so I treat Arm 3 as a direct conflict:
- no credits go to company work;
- Arm 3 uses only open-source C and Rust;
- the company gets no early or exclusive access to results;
- my faculty advisor, who has no tie to the company, reviews the Arm 3 analysis;
- all data and code will be public whatever the outcome.

### X-full
I am one of three co-founders of LegacyExodus, an early-stage company founded in Pakistan in August 2026. It builds static analysis for legacy JavaScript and TypeScript codebases. Its engine is open source under MIT and it has no revenue yet. The engine contains no cryptographic or LLM code. The company's published plans include a characterization-test offering, which is the technique Arm 3 studies, so I treat this as a direct conflict of interest. To keep the study independent:
1. No credits will be used for company work.
2. Arm 3 targets only C and Rust code from public open-source projects; the company works on JavaScript and TypeScript.
3. The company receives no early or exclusive access, and results will not be used in company material before public release.
4. My faculty advisor, Dr. Muhammad Anwaar Saeed, who has no tie to the company, reviews the Arm 3 analysis.
5. All tasks, prompts, harness code, raw generations and results will be published under MIT and CC BY 4.0 whatever the outcome.
6. The study is pre-registered, so its analysis cannot be tuned afterward.

---

## Sources (opened 24 Nov 2026)
- [OpenAI Researcher Access Program form page](https://openai.com/form/researcher-access-program): $1,000, reviews in Mar/Jun/Sep/Dec. The [FAQ](https://help.openai.com/en/articles/10139500-researcher-access-program-faq) says $10,000 and Apr/Aug/Dec. This conflict is flagged; December is in both.
- [Sharing and publication policy](https://openai.com/policies/sharing-publication-policy)
- [OpenAI Models](https://developers.openai.com/api/docs/models) and [Pricing](https://developers.openai.com/api/docs/pricing): luna is $0.10/$0.50 on the Models page and $0.05/$0.25 in an unlabeled pricing table. I used the higher figures.
- [arXiv:2611.25339](https://arxiv.org/abs/2611.25339), [arXiv:2602.01438](https://arxiv.org/html/2602.01438), [arXiv:2610.11835](https://arxiv.org/abs/2610.11835), [arXiv:2610.32548](https://arxiv.org/abs/2610.32548), [arXiv:2606.07341](https://arxiv.org/abs/2606.07341), [arXiv:2604.27001](https://arxiv.org/abs/2604.27001), [arXiv:2606.19474](https://arxiv.org/abs/2606.19474), [arXiv:2608.13784](https://arxiv.org/abs/2608.13784), [arXiv:2605.21384](https://arxiv.org/abs/2605.21384), [arXiv:2511.21654](https://arxiv.org/abs/2511.21654), [arXiv:2510.07604](https://arxiv.org/abs/2510.07604), [arXiv:2610.13811](https://arxiv.org/abs/2610.13811)
- [mlkem-native](https://github.com/pq-code-package/mlkem-native), [mldsa-native](https://github.com/pq-code-package/mldsa-native), [Crucible](https://symbolic.software/blog/2026-03-23-crucible/)
