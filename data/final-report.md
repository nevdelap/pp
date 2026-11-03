# Exhaustive 128-bit multi-base palindromic prime search

## Conclusion

The complete unsigned 128-bit search finished. Every binary/hexadecimal palindrome in the canonical integer range was enumerated, decimal palindromicity was checked, and every three-base palindrome was classified. The independent coverage verifier found **11,023 of 11,023 partitions complete**, with **11,453,508,259 of 11,453,508,259 assignments covered**, zero gaps, and zero invalid partition footers. The independent result verifier found zero invalid palindrome records, zero unresolved classifications, and verified all prime records.

The complete set of qualifying primes is **3, 5, 7**.

| Decimal | Binary | Hex | Decimal digits | Binary bits | Hex digits | Proof |
|---:|---:|---:|---:|---:|---:|---|
| 3 | `11` | `3` | 1 | 2 | 1 | Trial division through `floor(sqrt(3))` |
| 5 | `101` | `5` | 1 | 3 | 1 | Trial division through `floor(sqrt(5))` |
| 7 | `111` | `7` | 1 | 3 | 1 | Trial division through `floor(sqrt(7))` |

The only composite simultaneously palindromic in all three bases was **9** (`1001₂`, `9₁₆`, `9₁₀`), proven composite by the factor 3.

## Coverage and method

The generator represents a value as bit variables. Binary symmetry and hexadecimal digit symmetry become equalities between bit positions; the partially filled leading hexadecimal digit adds forced-zero constraints. A disjoint-set structure forms the equivalence classes, fixes the most significant bit to one, and enumerates each assignment to the remaining classes once. This directly enumerates the complete simultaneous base-2/base-16 palindrome space for every bit length 2 through 128. The derivation and proof are in `docs/mathematics.md`.

No length-based prime pruning was used in the run, so the composite three-base palindrome audit was preserved. Decimal-palindrome checking ran only after the generated candidate passed the hexadecimal check. Every decimal-palindrome survivor was conclusively classified. Even-length binary, hexadecimal, and decimal palindrome divisibility facts are implemented as proven compositeness classifications.

## Runtime and machine

- UTC start: `2026-11-03T08:05:14.110620977Z`
- UTC completion: `2026-11-03T08:20:12.168851448Z`
- Wall and active run duration: **898.038993 seconds**
- Average throughput: **12,753,909.7 assignments/second**
- Peak sampled throughput: **12,938,843.3 assignments/second**
- Hardware: AMD Ryzen 7 5700U, x86_64, 8 physical cores / 16 logical CPUs, AVX2, 22 GiB RAM
- Build: Rust 1.99.1, release mode, `opt-level=3`, thin LTO, one codegen unit, `target-cpu=native`
- Peak observed RSS: **4,096 KiB**
- Average sampled CPU utilization: **90.2% per logical CPU**; peak sampled process utilization was 1,467% (about 14.7 logical CPUs)

Stage totals accumulated across workers were 2,165.278 seconds for hexadecimal checks, 8,792.181 seconds for decimal checks, and 0.000001484 seconds for primality classification. The sum of per-partition worker durations was 14,175.392 seconds; it exceeds wall time because partitions ran in parallel. Per-bit-length counts and worker durations are in `final-report.json`.

The process had zero restarts, interruptions, failed partitions, or checkpoint recoveries. The last persisted cumulative CPU-time sample was **12,363.450 CPU seconds at 08:19:16 UTC**, 56 seconds before completion, so it is not an exact end-of-run CPU total. Checkpoint-write overhead was not separately instrumented and is recorded as `null`; the reported wall duration includes it. These two telemetry limits do not affect coverage or primality correctness.

## Verification artifacts

- `final-report.json`: machine-readable summary, coverage, stage totals, CPU and memory samples, hardware/compiler, and throughput by bit length.
- `primes.txt`: human-readable proven prime list.
- `part-*.jsonl`: durable per-partition results and coverage footers.
- `partitions.jsonl`: completed partition manifest.
- `telemetry.jsonl`, `system_metrics.jsonl`, `supervisor.jsonl`: timing and process telemetry.
- `docs/mathematics.md`: necessary-and-sufficient generator constraints and proof.

Verification commands used:

```sh
/tmp/target/release/multi-pal-prime coverage /workspace/data
/tmp/target/release/multi-pal-prime verify /workspace/data
/tmp/target/release/multi-pal-prime report /workspace/data
```
