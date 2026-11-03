# Exhaustive simultaneous binary/decimal/hexadecimal palindromic prime search

This Rust project generates every binary/hexadecimal palindrome in the unsigned 128-bit range using an exact bit-constraint system, checks decimal palindromicity without strings, and stores every three-base palindrome with a classification. It has deterministic chunking, atomic durable result files, recovery by replaying uncommitted chunks, and a detached supervisor script.

## Build and validate

```sh
cargo build --release
cargo test --release
cargo bench --bench throughput
target/release/multi-pal-prime small /tmp/mpal-smoke 4
target/release/multi-pal-prime verify /tmp/mpal-smoke
```

## Full run and monitoring

```sh
scripts/start.sh /workspace/data 16
scripts/status.sh /workspace/data
tail -f /workspace/data/search.log
scripts/primes.sh /workspace/data
target/release/multi-pal-prime coverage /workspace/data
target/release/multi-pal-prime report /workspace/data
scripts/stop.sh /workspace/data
```

Restarting `scripts/start.sh` uses the same deterministic partition IDs and skips durable committed partitions. `coverage` checks every expected partition from 2 through 128 bits. `verify` independently checks each stored number's base-2, base-10, and base-16 palindromicity. See [the proof](docs/mathematics.md) and [operations](docs/operations.md).

Prime results use exact trial division for the small integer records. Composite classifications cite a small factor or a Miller–Rabin compositeness witness, which is a proof of compositeness. A large candidate that passes the preliminary witness set is marked `unresolved`; passing those tests is not claimed as a primality proof. A final exhaustive claim requires zero unresolved records and successful coverage verification.

## Wider BigUint ranges

`cargo build --release` also builds `BigSpace` for lengths through 256 bits. Run `multi-pal-prime space256` to see the exact workload and `multi-pal-prime big-range BITS START END OUTPUT.jsonl` to process a deterministic half-open assignment range. The full 256-bit candidate workload is about `4.92e19` assignments and is not practical on this machine; passing Miller–Rabin candidates are explicitly unresolved pending a rigorous certificate. See [the wide-search notes](docs/wide-search.md).

For the restartable BigUint search and its incremental `final-report.json`, use `scripts/start-big.sh`; status and safe-stop commands are documented in [docs/wide-search.md](docs/wide-search.md). It compares the 2–128-bit results with `data/` before proceeding above 128 bits.

### Resume after a reboot

The detached supervisor keeps the search running after the terminal or Codex session ends, but it does not start automatically after the machine reboots. Once the machine is back up, start it from the saved BigUint checkpoint:

```sh
cd /workspace
nice -n 10 scripts/start-big.sh /workspace/data-bigint /workspace/data 16
scripts/status-big.sh /workspace/data-bigint
```

The checkpoint and completed partition files in `/workspace/data-bigint` are needed to resume without losing progress. Keep that directory intact. The start command resumes from its committed cursor and retries any uncommitted batch work.
