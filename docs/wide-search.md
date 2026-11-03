# BigUint search through 256 bits

`BigSpace` represents simultaneous binary and hexadecimal palindromes with a `BigUint` value and assignment index. It uses the same bit-equality and leading-zero constraints as `Space`. For a bit length `b`, assignments cover `[0, 2^free_bits)` exactly once. `space256` prints each exact workload; the total through 256 bits is `49,191,317,547,072,006,819` assignments.

## Persistent coordinator

Start the BigUint search with:

```sh
scripts/start-big.sh /workspace/data-bigint /workspace/data 16
```

The coordinator processes every bit length from 2 through 128 first. It compares all discovered triple-palindrome values, binary/hexadecimal representations, and classifications against the completed `u128` result set. It will not start bit length 129 unless that comparison passes. The comparison is saved as `data-bigint/comparison-2-128.json`.

After the comparison, it continues through bit length 256. Both `data-bigint/final-report.json` and the human-readable `data-bigint/final-report.md` are atomically replaced after each completed batch with status, phase, current bit length and assignment, completed/total candidate assignments, overall and 2–128-bit progress, result counts, elapsed and active time, throughput, and update time. Individual committed result partitions are retained as `big-bNNN-aSTART-eEND.jsonl`; they contain result rows and an exact range footer. Atomic `manifest-bNNN-aSTART-eEND.json` files list each completed batch and its exact ranges. `checkpoint.json` is the restart cursor and cumulative report state. Completed ranges are validated and skipped when restarted. If a process fails, the supervisor restarts the coordinator from its last durable batch checkpoint.

Check status and the latest report with:

```sh
scripts/status-big.sh /workspace/data-bigint
cat /workspace/data-bigint/comparison-2-128.json
```

Stop safely and later resume with:

```sh
scripts/stop-big.sh /workspace/data-bigint
scripts/start-big.sh /workspace/data-bigint /workspace/data 16
```

The scripts use `nohup` and `setsid`; the supervisor restarts a failed coordinator while the host remains up. The result and checkpoint directory is separate from the `u128` data. It should have enough durable storage for the run. A machine reboot requires starting the script again unless the host has an external boot-time service configured.

The 256-bit workload is not expected to complete on practical timescales. At 256 bits there are `2^63` simultaneous binary/hex palindrome assignments for that length alone. The incremental report is therefore the operational record of ongoing work; it must not be interpreted as a completed search until it reports `status=complete`.

## Primality status

The BigUint scanner checks decimal palindromicity only after generating a binary/hex palindrome. It records all triple-palindrome survivors with the classification and evidence available to its classifier. Miller–Rabin witnesses and explicit factors prove compositeness. Candidates that pass the configured witnesses are marked `unresolved`; no probable-prime result is described as a proven prime. A rigorous 256-bit prime-certificate backend is not configured, so the report's unresolved count must be considered before claiming a complete prime classification.

## Direct range command

One deterministic half-open range can be processed independently:

```sh
target/release/multi-pal-prime big-range 256 0 1000000 /workspace/data-bigint/example.jsonl
```

Adjacent ranges cover a bit length when each next start equals the previous end and the final end is `2^free_bits`. The persistent coordinator handles this partitioning, checkpointing, reporting, and the 128-bit comparison gate.
