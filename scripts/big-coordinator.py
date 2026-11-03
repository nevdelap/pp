#!/usr/bin/env python3
"""Restartable coordinator for the BigUint 256-bit search.

The Rust binary enumerates and classifies each assigned candidate range. This
coordinator owns deterministic scheduling, durable checkpoints, report updates,
and validation of every persisted range before it is counted or reused.
"""

from __future__ import annotations

import fcntl
import json
import math
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import time
from datetime import datetime, timezone


STOP_REQUESTED = False


def request_stop(_signum: int, _frame: object) -> None:
    global STOP_REQUESTED
    STOP_REQUESTED = True


def utc() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def human_duration(seconds: float) -> str:
    remaining = max(0, int(seconds))
    parts = []
    for size, suffix in ((86400, "d"), (3600, "h"), (60, "m"), (1, "s")):
        amount, remaining = divmod(remaining, size)
        if amount:
            parts.append(f"{amount}{suffix}")
    return " ".join(parts) or "0s"


def human_long_duration(seconds: int) -> str:
    remaining = max(0, int(seconds))
    parts = []
    for size, label in (
        (365 * 86400, "years"), (30 * 86400, "months"), (86400, "days"),
        (3600, "hours"), (60, "minutes"), (1, "seconds"),
    ):
        amount, remaining = divmod(remaining, size)
        unit = label[:-1] if amount == 1 else label
        parts.append(f"{human_number(amount)} {unit}")
    return ", ".join(parts)


def utc_after_seconds(now: datetime, seconds: int) -> str:
    """Format a UTC date after an arbitrary duration, including years > 9999."""
    now = now.astimezone(timezone.utc).replace(microsecond=0)
    seconds_of_day = now.hour * 3600 + now.minute * 60 + now.second + max(0, seconds)
    days_to_add, seconds_of_day = divmod(seconds_of_day, 86400)
    ordinal = now.date().toordinal() + days_to_add

    elapsed_days = ordinal - 1
    cycles, day_of_cycle = divmod(elapsed_days, 146097)
    year = cycles * 400 + 1
    while True:
        leap = year % 4 == 0 and (year % 100 != 0 or year % 400 == 0)
        days_in_year = 366 if leap else 365
        if day_of_cycle < days_in_year:
            break
        day_of_cycle -= days_in_year
        year += 1

    month_lengths = [31, 29 if leap else 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    month = 1
    for month_length in month_lengths:
        if day_of_cycle < month_length:
            break
        day_of_cycle -= month_length
        month += 1
    day = day_of_cycle + 1
    hour, remainder = divmod(seconds_of_day, 3600)
    minute, second = divmod(remainder, 60)
    year_text = f"{year:04d}" if year <= 9999 else human_number(year)
    return f"{year_text}-{month:02d}-{day:02d} {hour:02d}:{minute:02d}:{second:02d} UTC"


def human_number(value: int | str) -> str:
    return f"{int(value):,}"


def human_percentage(value: str | float) -> str:
    return f"{float(value):.12f}".rstrip("0").rstrip(".") or "0"


def integers_excluded_by_binary_hex_constraints(bits: int, counts: dict[int, int]) -> int:
    if bits < 2:
        return 0
    bits = min(bits, max(counts, default=1))
    if bits < 2:
        return 0
    raw_integers = (1 << bits) - 2  # Search domain is 2 <= n < 2**bits.
    generated_candidates = sum(counts[length] for length in range(2, bits + 1))
    if generated_candidates > raw_integers:
        raise ValueError("candidate count exceeds the covered integer range")
    return raw_integers - generated_candidates


def atomic_write(path: Path, data: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_name(path.name + f".tmp.{os.getpid()}")
    with open(tmp, "w", encoding="utf-8") as stream:
        stream.write(data)
        stream.flush()
        os.fsync(stream.fileno())
    os.replace(tmp, path)
    dir_fd = os.open(path.parent, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    try:
        os.fsync(dir_fd)
    finally:
        os.close(dir_fd)


def atomic_json(path: Path, value: object) -> None:
    atomic_write(path, json.dumps(value, separators=(",", ":"), sort_keys=True) + "\n")


def read_comparison_records(root: Path, pattern: str, max_bits: int | None = None) -> tuple[dict, int]:
    records: dict[str, dict] = {}
    count = 0
    for path in root.glob(pattern):
        match = re.fullmatch(r"big-b(\d{3})-.*\.jsonl", path.name)
        if max_bits is not None and (match is None or int(match.group(1)) > max_bits):
            continue
        with open(path, encoding="utf-8") as stream:
            for line_number, line in enumerate(stream, start=1):
                try:
                    record = json.loads(line)
                except json.JSONDecodeError as exc:
                    raise ValueError(f"malformed JSON at {path}:{line_number}") from exc
                if not isinstance(record, dict):
                    raise ValueError(f"expected JSON object at {path}:{line_number}")
                if "value" not in record:
                    if not valid_completion_footer(record):
                        raise ValueError(f"result JSON object missing value at {path}:{line_number}")
                    continue
                value = record["value"]
                if not isinstance(value, str) or not value.isascii() or not value.isdecimal():
                    raise ValueError(f"invalid result value in {path}")
                if value in records:
                    raise ValueError(f"duplicate value {value} in {root}")
                binary, hexadecimal = record.get("binary"), record.get("hex")
                if not isinstance(binary, str) or not isinstance(hexadecimal, str):
                    raise ValueError(f"missing representation for {value} in {path}")
                records[value] = {"binary": binary, "hex": hexadecimal,
                                  "classification": record.get("classification")}
                count += 1
    return records, count


def valid_completion_footer(record: dict) -> bool:
    required = ("bits", "assignment_start", "assignment_end_exclusive", "assignment_space",
                "generated", "decimal_pass", "unresolved", "seconds", "completion_utc")
    if record.get("status") != "completed" or any(key not in record for key in required):
        return False
    try:
        bits = int(record["bits"])
        start = int(record["assignment_start"])
        end = int(record["assignment_end_exclusive"])
        space = int(record["assignment_space"])
        generated = int(record["generated"])
        decimal_pass = int(record["decimal_pass"])
        unresolved = int(record["unresolved"])
        seconds = float(record["seconds"])
    except (ValueError, TypeError, OverflowError):
        return False
    return (2 <= bits <= 256 and start >= 0 and end > start and space >= end
            and generated == end - start and 0 <= decimal_pass <= generated and unresolved >= 0
            and unresolved <= decimal_pass and seconds >= 0
            and isinstance(record["completion_utc"], str) and bool(record["completion_utc"]))


def compare_u128(big_root: Path, u128_root: Path, report_path: Path) -> dict:
    try:
        big, big_count = read_comparison_records(big_root, "big-b???-*.jsonl", max_bits=128)
        fixed, fixed_count = read_comparison_records(u128_root, "part-b*-*.jsonl")
    except (OSError, ValueError) as exc:
        result = {"status": "failed", "timestamp_utc": utc(), "bigint_records": 0,
                  "u128_records": 0, "missing_from_bigint": [], "extra_in_bigint": [],
                  "representation_mismatches": [], "classification_mismatches": [],
                  "values": [], "error": str(exc)}
        atomic_json(report_path, result)
        return result
    missing = sorted((value for value in fixed if value not in big), key=lambda value: (len(value), value))
    extra = sorted((value for value in big if value not in fixed), key=lambda value: (len(value), value))
    representations = sorted((value for value in big if value in fixed and
                              (big[value]["binary"] != fixed[value]["binary"] or
                               big[value]["hex"] != fixed[value]["hex"])), key=lambda value: (len(value), value))
    classifications = sorted((value for value in big if value in fixed and
                              big[value]["classification"] != fixed[value]["classification"]),
                             key=lambda value: (len(value), value))
    passed = not (missing or extra or representations or classifications or big_count != fixed_count)
    values = sorted(big, key=lambda value: (len(value), value))
    result = {"status": "passed" if passed else "failed", "timestamp_utc": utc(),
              "bigint_records": big_count, "u128_records": fixed_count,
              "missing_from_bigint": missing, "extra_in_bigint": extra,
              "representation_mismatches": representations,
              "classification_mismatches": classifications, "values": values}
    atomic_json(report_path, result)
    return result


def palindrome(text: str) -> bool:
    return text == text[::-1]


def is_mr_witness(n: int, base: int) -> bool:
    if n < 3 or n % 2 == 0 or not 2 <= base <= n - 2:
        return False
    d, s = n - 1, 0
    while d % 2 == 0:
        d //= 2
        s += 1
    x = pow(base, d, n)
    if x in (1, n - 1):
        return False
    for _ in range(s - 1):
        x = x * x % n
        if x == n - 1:
            return False
    return True


def valid_record(line: str, bits: int, start: int, end: int, require_index: bool,
                 binary: Path) -> tuple[str, dict] | None:
    try:
        record = json.loads(line)
        if not isinstance(record, dict):
            return None
        decimal = record.get("value")
        if not isinstance(decimal, str) or not decimal.isascii() or not decimal.isdecimal() or decimal.startswith("0"):
            return None
        n = int(decimal)
        if n < 1:
            return None
        bits_repr, hex_repr = format(n, "b"), format(n, "x")
        if (len(bits_repr) != bits or record.get("binary") != bits_repr or record.get("hex") != hex_repr
                or record.get("binary_bits") != bits or record.get("hex_digits") != len(hex_repr)
                or record.get("decimal_digits") != len(decimal)
                or not palindrome(bits_repr) or not palindrome(hex_repr) or not palindrome(decimal)):
            return None
        idx_value = record.get("assignment_index")
        if require_index and idx_value is None:
            return None
        # Legacy format-version-1 rows have no saved assignment index. Derive
        # it from the canonical value so old files get the same range check.
        check = subprocess.run([str(binary), "big-index", str(bits), decimal],
                               check=False, capture_output=True, text=True)
        if check.returncode:
            return None
        derived_index = int(check.stdout.strip())
        if not start <= derived_index < end:
            return None
        if idx_value is not None and int(idx_value) != derived_index:
            return None

        classification = record.get("classification")
        evidence = record.get("proof_evidence")
        if classification == "prime":
            if n > 251:
                return None
            if any(n % divisor == 0 for divisor in range(2, int(n**0.5) + 1)):
                return None
        elif classification == "composite":
            if not isinstance(evidence, str):
                return None
            if evidence.startswith("factor="):
                factor = int(evidence.removeprefix("factor="))
                if not 1 < factor < n or n % factor:
                    return None
            elif evidence.startswith("miller_rabin_base="):
                base = int(evidence.removeprefix("miller_rabin_base="))
                if not 2 <= base <= n - 2 or not is_mr_witness(n, base):
                    return None
            else:
                return None
        elif classification != "unresolved" or evidence is None:
            return None
        return classification, record
    except (ValueError, TypeError, KeyError, OverflowError, json.JSONDecodeError):
        return None


class Coordinator:
    def __init__(self, binary: Path, root: Path, u128_root: Path, threads: int,
                 test_max_bits: int | None):
        self.binary = binary.resolve()
        self.root = root.resolve()
        self.u128_root = u128_root.resolve()
        self.threads = threads
        self.test_max_bits = test_max_bits
        self.root.mkdir(parents=True, exist_ok=True)
        self.lock_file = open(self.root / "coordinator.lock", "a+")
        try:
            fcntl.flock(self.lock_file, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as exc:
            raise RuntimeError("another BigInt coordinator holds the lock") from exc

        space = subprocess.run([str(self.binary), "space256"], check=True, capture_output=True, text=True)
        self.counts: dict[int, int] = {}
        for line in space.stdout.splitlines():
            if line.startswith("bits="):
                fields = dict(item.split("=", 1) for item in line.split() if "=" in item)
                self.counts[int(fields["bits"])] = int(fields["assignments"])
        if any(bit not in self.counts for bit in range(2, 257)):
            raise RuntimeError("space256 omitted one or more bit lengths")
        self.total = sum(self.counts.values())
        self.state_path = self.root / "checkpoint.json"
        if self.state_path.exists():
            self.state = json.loads(self.state_path.read_text(encoding="utf-8"))
        else:
            self.state = {
                "status": "running", "phase": "common_2_128", "bits": 2,
                "next_start": "0", "completed_assignments": "0", "partitions": 0,
                "result_records": 0, "prime_records": 0, "composite_records": 0,
                "unresolved_records": 0, "unresolved_candidates": [], "active_seconds": 0,
                "started_epoch": time.time(), "started_utc": utc(),
            }
        self.state.setdefault("started_epoch", time.time())
        self.state.setdefault("started_utc", utc())
        if "unresolved_candidates" not in self.state:
            self.state["unresolved_candidates"] = (
                self.recover_unresolved_candidates()
                if int(self.state.get("unresolved_records", 0)) else []
            )
        self.state.pop("active_batch_elapsed_seconds", None)
        self.state.pop("interrupted_batch_seconds", None)
        for field in ("active_batch_start", "active_batch_end", "active_batch_assignments",
                      "active_worker_count", "active_batch_started_utc"):
            self.state.pop(field, None)
        self.state["status"] = "running"

    def save(self, event: str) -> None:
        state = self.state
        state["updated_utc"] = utc()
        state["total_assignments"] = str(self.total)
        state["event"] = event
        done = int(state["completed_assignments"])
        state["percentage_complete"] = f"{100 * done / self.total:.12f}"
        state["elapsed_wall_seconds"] = f"{time.time() - float(state['started_epoch']):.3f}"
        state["active_seconds"] = f"{float(state.get('active_seconds', 0)) + float(state.pop('batch_active_seconds', 0)):.3f}"
        elapsed_wall_seconds = float(state["elapsed_wall_seconds"])
        remaining_assignments = max(0, self.total - done)
        eta_seconds = (
            math.ceil(remaining_assignments * elapsed_wall_seconds / done)
            if done > 0 and elapsed_wall_seconds > 0 else None
        )
        estimated_completion_utc = (
            utc_after_seconds(datetime.now(timezone.utc), eta_seconds)
            if eta_seconds is not None else None
        )
        palindrome_bits = min(int(state["bits"]), max(self.counts))
        palindrome_excluded = integers_excluded_by_binary_hex_constraints(palindrome_bits, self.counts)
        atomic_json(self.state_path, state)
        active_seconds = float(state["active_seconds"])
        report = {
            "status": state["status"], "current_bit_length": state["bits"],
            "next_assignment": state["next_start"], "completed_assignments": state["completed_assignments"],
            "total_assignments": str(self.total),
            "percentage_complete": float(state["percentage_complete"]),
            "completed_partitions": state["partitions"],
            "triple_palindrome_records": state["result_records"], "prime_classifications": state["prime_records"],
            "composite_classifications": state["composite_records"], "unresolved_classifications": state["unresolved_records"],
            "unresolved_candidates": state.get("unresolved_candidates", []),
            "integers_excluded_by_binary_hex_palindrome_constraints_through_current_bit": palindrome_excluded,
            "active_seconds": active_seconds, "started_utc": state["started_utc"],
            "elapsed_wall_seconds": float(state["elapsed_wall_seconds"]),
            "estimated_remaining_seconds": eta_seconds,
            "estimated_completion_utc": estimated_completion_utc,
            "average_assignments_per_active_second": done / active_seconds if active_seconds else 0,
            "active_batch_start": state.get("active_batch_start"),
            "active_batch_end_exclusive": state.get("active_batch_end"),
            "active_batch_assignments": state.get("active_batch_assignments"),
            "active_workers": state.get("active_worker_count", 0),
            "active_batch_elapsed_seconds": float(state.get("active_batch_elapsed_seconds", 0)),
            "updated_utc": state["updated_utc"], "last_event": event,
            "note": "Incremental BigUint status. Primality records marked unresolved are not proven primes.",
        }
        atomic_json(self.root / "final-report.json", report)
        batch = (f"- **Active batch candidates:** {human_number(state['active_batch_start'])}–{human_number(state['active_batch_end'])} "
                 f"({human_number(state['active_batch_assignments'])} scheduled)\n- **Active workers:** {human_number(state['active_worker_count'])}\n"
                 f"- **Active batch elapsed:** {human_duration(float(state.get('active_batch_elapsed_seconds', 0)))}\n"
                 if state.get("active_batch_start") is not None else "- **Active batch:** none\n")
        human = (
            "# BigUint exhaustive search progress\n\n"
            "This report tracks a restartable search for integers from 2 through 2^256−1 that are palindromic in binary, decimal, and hexadecimal and are prime. The search enumerates binary/hexadecimal palindrome candidates, then checks decimal palindromicity and primality. Progress is measured against that constrained candidate workload.\n\n"
            f"- **Status:** {state['status']}\n"
            f"- **Current bit length:** {state['bits']}\n"
            f"- **Next candidate position:** {human_number(state['next_start'])}\n"
            f"- **Candidates processed:** {human_number(state['completed_assignments'])} / {human_number(self.total)}\n"
            f"- **Candidates processed per active search second:** {done / active_seconds if active_seconds else 0:,.3f}\n"
            f"- **Integers ruled out by binary/hex palindrome constraints through bit length {palindrome_bits}:** {human_number(palindrome_excluded)}\n"
            f"- **Overall workload complete:** {human_percentage(state['percentage_complete'])}%\n"
            f"- **Completed partitions:** {human_number(state['partitions'])}\n{batch}"
            f"- **Triple-palindrome records:** {human_number(state['result_records'])}\n"
            f"- **Prime classifications:** {human_number(state['prime_records'])}\n"
            f"- **Composite classifications:** {human_number(state['composite_records'])}\n"
            f"- **Unresolved classifications:** {human_number(state['unresolved_records'])}\n"
            f"- **Active worker time:** {human_duration(active_seconds)}\n"
            f"- **Elapsed wall time:** {human_duration(float(state['elapsed_wall_seconds']))}\n"
            f"- **Estimated remaining time:** {human_long_duration(eta_seconds) if eta_seconds is not None else 'not available'}\n"
            f"- **Estimated completion date:** {estimated_completion_utc or 'not available'}\n"
            "Each candidate position maps to one integer that is palindromic in binary and hexadecimal. "
            "Decimal palindromicity and then primality are checked afterward.\n"
        )
        if state.get("unresolved_candidates"):
            human += "\n## Unresolved candidates\n"
            for candidate in state["unresolved_candidates"]:
                human += (
                    f"\n- **Decimal:** `{candidate['value']}`\n"
                    f"  - **Binary:** `{candidate['binary']}`\n"
                    f"  - **Hexadecimal:** `{candidate['hex']}`\n"
                    f"  - **Bit length / decimal digits:** {candidate['binary_bits']} / {candidate['decimal_digits']}\n"
                    f"  - **Verification method:** {candidate['method']}\n"
                    f"  - **Evidence:** {candidate['proof_evidence']}\n"
                    f"  - **Partition:** `{candidate['partition']}`\n"
                )
        human += (f"- **Started UTC:** {state['started_utc']}\n- **Updated UTC:** {state['updated_utc']}\n\n"
                  "Prime classifications in this BigUint run are not, by themselves, rigorous primality proofs. "
                  "The 256-bit scan is not complete until status is `complete`; unresolved candidates require separate proof or compositeness verification.\n")
        atomic_write(self.root / "final-report.md", human)

    def partition_path(self, bits: int, start: int, end: int) -> Path:
        return self.root / f"big-b{bits:03d}-a{start}-e{end}.jsonl"

    def file_stats(self, path: Path, bits: int, start: int, end: int) -> dict | None:
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
            if not lines:
                return None
            footer = json.loads(lines[-1])
            if (footer.get("status") != "completed" or int(footer.get("bits", -1)) != bits
                    or int(footer.get("assignment_start", -1)) != start
                    or int(footer.get("assignment_end_exclusive", -1)) != end
                    or int(footer.get("assignment_space", -1)) != self.counts[bits]
                    or int(footer.get("generated", -1)) != end - start
                    or len(lines) - 1 != int(footer.get("decimal_pass", -1))):
                return None
            version = int(footer.get("format_version", 1))
            totals = {"generated": end - start, "records": len(lines) - 1,
                      "primes": 0, "composites": 0, "unresolved": 0,
                      "unresolved_candidates": []}
            seen: set[str] = set()
            for line in lines[:-1]:
                validated = valid_record(line, bits, start, end, version >= 2, self.binary)
                if validated is None:
                    return None
                classification, record = validated
                if record["value"] in seen:
                    return None
                seen.add(record["value"])
                totals[{"prime": "primes", "composite": "composites", "unresolved": "unresolved"}[classification]] += 1
                if classification == "unresolved":
                    totals["unresolved_candidates"].append({
                        "value": record["value"], "binary": record["binary"], "hex": record["hex"],
                        "binary_bits": record["binary_bits"], "decimal_digits": record["decimal_digits"],
                        "method": record.get("method", "unspecified"),
                        "proof_evidence": record["proof_evidence"], "partition": path.name,
                    })
            return totals
        except (OSError, ValueError, TypeError, KeyError):
            return None

    def recover_unresolved_candidates(self) -> list[dict]:
        recovered = []
        pattern = re.compile(r"big-b(\d{3})-a(\d+)-e(\d+)\.jsonl")
        checkpoint_bits = int(self.state["bits"])
        checkpoint_cursor = int(self.state["next_start"])
        for path in self.root.glob("big-b*.jsonl"):
            match = pattern.fullmatch(path.name)
            if not match:
                continue
            bits, start, end = map(int, match.groups())
            # Files beyond the durable cursor can be complete outputs from a
            # batch that crashed before its manifest/checkpoint commit.
            if bits > checkpoint_bits or (bits == checkpoint_bits and end > checkpoint_cursor):
                continue
            stats = self.file_stats(path, bits, start, end)
            if stats is None:
                raise RuntimeError(f"cannot recover unresolved report data from invalid partition {path}")
            recovered.extend(stats["unresolved_candidates"])
        if len(recovered) != int(self.state.get("unresolved_records", 0)):
            raise RuntimeError("recovered unresolved candidate list does not match the checkpoint count")
        return recovered

    def save_stopped_batch(self) -> None:
        self.state["status"] = "stopped" if (self.root / "STOP").exists() else "interrupted"
        self.state["active_worker_count"] = 0
        for field in ("active_batch_start", "active_batch_end", "active_batch_assignments",
                      "active_batch_started_utc", "active_batch_elapsed_seconds"):
            self.state.pop(field, None)
        self.save("stop_requested_during_batch")

    def run_batch(self, bits: int, start: int, end: int, chunk: int) -> None:
        tasks = []
        cursor = start
        batch_started = time.monotonic()
        while cursor < end:
            stop = min(cursor + chunk, end)
            path = self.partition_path(bits, cursor, stop)
            stats = self.file_stats(path, bits, cursor, stop)
            process = None
            if stats is None:
                path.unlink(missing_ok=True)
                process = subprocess.Popen([str(self.binary), "big-range", str(bits), str(cursor), str(stop), str(path)])
            tasks.append((cursor, stop, path, stats, process))
            cursor = stop
        self.state.update({"active_batch_start": str(start), "active_batch_end": str(cursor),
                           "active_batch_assignments": str(cursor-start),
                           "active_worker_count": sum(task[4] is not None for task in tasks),
                           "active_batch_started_utc": utc(), "active_batch_elapsed_seconds": 0})
        self.save("batch_started")
        last_snapshot = time.monotonic()
        failed = False
        pending = [task[4] for task in tasks if task[4] is not None]
        while pending:
            for proc in list(pending):
                status = proc.poll()
                if status is not None:
                    pending.remove(proc)
                    failed |= status != 0
            if STOP_REQUESTED:
                for proc in pending:
                    try:
                        proc.wait(timeout=0.1)
                    except subprocess.TimeoutExpired:
                        proc.terminate()
                for proc in pending:
                    try:
                        proc.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        proc.kill()
                        proc.wait()
                pending.clear()
                self.save_stopped_batch()
                return
            now = time.monotonic()
            if now - last_snapshot >= 30:
                self.state["active_batch_elapsed_seconds"] = f"{now-batch_started:.3f}"
                self.state["active_worker_count"] = len(pending)
                self.save("batch_in_progress")
                last_snapshot = now
            if pending:
                time.sleep(0.25)
        elapsed = time.monotonic() - batch_started
        if STOP_REQUESTED:
            self.save_stopped_batch()
            return
        if failed:
            raise RuntimeError("BigInt partition batch failed; supervisor will retry from checkpoint")
        batch_assignments = 0
        manifest_ranges = []
        for s, e, path, _old_stats, _process in tasks:
            stats = self.file_stats(path, bits, s, e)
            if stats is None:
                raise RuntimeError(f"invalid committed partition {path}")
            batch_assignments += stats["generated"]
            self.state["result_records"] += stats["records"]
            self.state["prime_records"] += stats["primes"]
            self.state["composite_records"] += stats["composites"]
            self.state["unresolved_records"] += stats["unresolved"]
            known_unresolved = {item["value"] for item in self.state.get("unresolved_candidates", [])}
            for candidate in stats["unresolved_candidates"]:
                if candidate["value"] in known_unresolved:
                    raise RuntimeError(f"duplicate unresolved candidate {candidate['value']}")
                known_unresolved.add(candidate["value"])
                self.state.setdefault("unresolved_candidates", []).append(candidate)
            self.state["partitions"] += 1
            manifest_ranges.append({"assignment_start": str(s), "assignment_end_exclusive": str(e),
                                    "generated": str(stats["generated"]), "decimal_pass": stats["records"],
                                    "status": "completed", "file": str(path)})
        manifest = {"bits": bits, "batch_start": str(start), "batch_end_exclusive": str(cursor),
                    "status": "completed", "completion_utc": utc(), "ranges": manifest_ranges}
        atomic_json(self.root / f"manifest-b{bits:03d}-a{start}-e{cursor}.json", manifest)
        self.state["completed_assignments"] = str(int(self.state["completed_assignments"]) + batch_assignments)
        self.state["bits"], self.state["next_start"] = bits, str(cursor)
        self.state["batch_active_seconds"] = elapsed
        for field in ("active_batch_start", "active_batch_end", "active_batch_assignments", "active_worker_count", "active_batch_started_utc", "active_batch_elapsed_seconds"):
            self.state.pop(field, None)
        self.save("partition_batch_completed")

    def run(self) -> None:
        self.save("coordinator_started")
        while self.state["phase"] != "complete":
            if STOP_REQUESTED:
                self.state["status"] = "stopped" if (self.root / "STOP").exists() else "interrupted"
                self.state["active_worker_count"] = 0
                self.save("stop_requested")
                return
            if self.state["phase"] == "compare_2_128":
                comparison = compare_u128(self.root, self.u128_root, self.root / "comparison-2-128.json")
                if comparison["status"] != "passed":
                    raise RuntimeError("128-bit result comparison failed")
                self.state.update(phase="above_128", bits=129, next_start="0")
                self.save("u128_bigint_comparison_passed")
                continue
            bits = int(self.state["bits"])
            if self.test_max_bits is not None and bits > self.test_max_bits:
                self.state.update(status="test_stopped", phase="test_stopped")
                self.save("test_bit_limit_reached")
                return
            if bits > 256:
                self.state.update(phase="complete", status="complete")
                self.save("search_complete")
                return
            count, cursor = self.counts[bits], int(self.state["next_start"])
            if cursor >= count:
                if bits == 128 and self.state["phase"] == "common_2_128":
                    self.state["phase"] = "compare_2_128"
                    self.save("common_range_complete")
                    continue
                self.state.update(bits=bits + 1, next_start="0")
                self.save("bit_length_complete")
                continue
            chunk = 1_048_576 if bits <= 128 else 100_000_000
            batch_end = min(cursor + chunk * self.threads, count)
            self.run_batch(bits, cursor, batch_end, chunk)
            if self.state["status"] in ("stopped", "interrupted"):
                return


def main(argv: list[str]) -> int:
    if len(argv) not in (4, 5):
        print("usage: big-coordinator.py BIN BIG_ROOT U128_ROOT THREADS [--max-bits=N]", file=sys.stderr)
        return 2
    binary, root, u128_root = map(Path, argv[:3])
    try:
        threads = int(argv[3])
        if threads < 1:
            raise ValueError("threads must be positive")
        max_bits = None
        if len(argv) == 5:
            if not argv[4].startswith("--max-bits="):
                raise ValueError("invalid test bit limit")
            max_bits = int(argv[4].split("=", 1)[1])
            if not 2 <= max_bits <= 256:
                raise ValueError("invalid test bit limit")
        signal.signal(signal.SIGTERM, request_stop)
        signal.signal(signal.SIGINT, request_stop)
        Coordinator(binary, root, u128_root, threads, max_bits).run()
        return 0
    except Exception as exc:
        print(f"big coordinator error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
