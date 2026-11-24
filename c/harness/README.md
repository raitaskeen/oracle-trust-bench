# C harness: drivers and reference adapters

All C here runs only inside the sandbox (ADR-001, ADR-003).

- `tasks.h` declares the C interface of every pilot task, exactly as in the
  task specs. A candidate implements one of these functions.
- `drivers.c` holds one driver per task, selected at compile time with
  `-DOTB_TASK_T01` (or `T07`, `T24`, `T41`, `T47`). A driver reads fixed-width binary
  records from stdin, calls the task function once per record, and writes
  fixed-width binary records to stdout. It holds no expected values and
  makes no decisions; the Rust oracles compare the outputs.
- `ref_mlkem.c` and `ref_mldsa.c` implement the task interfaces on top of
  mlkem-native and mldsa-native (pinned submodules), built as single
  compilation units with the portable C backend only.

A program is built as `drivers.c` + one implementation (a reference
adapter, a seeded fault or a candidate).

## Record formats

All integers are little-endian. One input record produces one output record.

| Task | Input record | Output record |
|---|---|---|
| MLKEM-T01-ntt | 256 x int16 (512 bytes) | 256 x int16 (512 bytes) |
| MLKEM-T07-compress-d | uint16 x, uint8 d (3 bytes) | uint16 result (2 bytes) |
| MLKEM-T24-decaps-implicit-rejection | uint32 ct_len, ct bytes, uint32 dk_len, dk bytes | int32 return value, 32 bytes ss |
| MLDSA-T41-power2round | int32 r (4 bytes) | int32 r1, int32 r0 (8 bytes) |
| MLDSA-T47-make-hint | int32 z, int32 r (8 bytes) | int32 return value (4 bytes) |

For T24, ss is filled with 0xAA before the call, so a call that does not
write ss is visible.
