import importlib.util
import json
from pathlib import Path
import tempfile
import time
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "big-coordinator.py"
SPEC = importlib.util.spec_from_file_location("big_coordinator", SCRIPT)
coordinator = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(coordinator)


class CoordinatorValidationTests(unittest.TestCase):
    def validate(self, row, bits, start, end, require_index, derived_index="0"):
        result = type("Completed", (), {"returncode": 0, "stdout": derived_index + "\n"})()
        with patch.object(coordinator.subprocess, "run", return_value=result):
            return coordinator.valid_record(
                json.dumps(row), bits, start, end, require_index, Path("mock-binary")
            )

    def record(self, value, classification, evidence):
        text = str(value)
        binary = format(value, "b")
        hexadecimal = format(value, "x")
        return {
            "value": text,
            "binary": binary,
            "hex": hexadecimal,
            "binary_bits": len(binary),
            "hex_digits": len(hexadecimal),
            "decimal_digits": len(text),
            "classification": classification,
            "proof_evidence": evidence,
        }

    def test_duration_is_human_readable(self):
        self.assertEqual(coordinator.human_duration(93784), "1d 2h 3m 4s")
        self.assertEqual(coordinator.human_duration(0), "0s")

    def test_large_human_report_numbers_use_commas(self):
        self.assertEqual(coordinator.human_number("49191317547072006819"), "49,191,317,547,072,006,819")

    def test_human_percentages_drop_unneeded_trailing_zeros(self):
        self.assertEqual(coordinator.human_percentage("100.000000000"), "100")
        self.assertEqual(coordinator.human_percentage("25.500000000"), "25.5")
        self.assertEqual(coordinator.human_percentage("0.000000093134"), "0.000000093134")

    def test_binary_hex_exclusion_count_uses_covered_bit_lengths(self):
        self.assertEqual(coordinator.integers_excluded_by_binary_hex_constraints(3, {2: 1, 3: 2}), 3)
        self.assertEqual(coordinator.integers_excluded_by_binary_hex_constraints(1, {}), 0)
        with self.assertRaises(ValueError):
            coordinator.integers_excluded_by_binary_hex_constraints(2, {2: 3})

    def test_terminal_bit_257_state_saves_report_through_bit_256(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            instance = object.__new__(coordinator.Coordinator)
            instance.root = root
            instance.state_path = root / "checkpoint.json"
            instance.counts = {bits: 1 for bits in range(2, 257)}
            instance.total = sum(instance.counts.values())
            instance.state = {
                "status": "complete", "phase": "complete", "bits": 257,
                "next_start": "0", "completed_assignments": str(instance.total),
                "partitions": 255, "result_records": 0, "prime_records": 0,
                "composite_records": 0, "unresolved_records": 0,
                "unresolved_candidates": [], "active_seconds": 1,
                "started_epoch": time.time() - 2,
                "started_utc": "2026-11-03T00:00:00Z",
            }

            instance.save("search_complete")

            markdown = (root / "final-report.md").read_text()
            report = json.loads((root / "final-report.json").read_text())
            self.assertEqual(json.loads(instance.state_path.read_text())["status"], "complete")
            self.assertEqual(report["status"], "complete")
            self.assertIn("through bit length 256", markdown)

    def test_comparison_phases_remain_internal_to_checkpoint(self):
        for phase in ("common_2_128", "compare_2_128"):
            with self.subTest(phase=phase), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                instance = object.__new__(coordinator.Coordinator)
                instance.root = root
                instance.state_path = root / "checkpoint.json"
                instance.counts = {bits: 1 for bits in range(2, 257)}
                instance.total = sum(instance.counts.values())
                instance.state = {
                    "status": "running", "phase": phase, "bits": 128,
                    "next_start": "1", "completed_assignments": "1",
                    "partitions": 1, "result_records": 0, "prime_records": 0,
                    "composite_records": 0, "unresolved_records": 0,
                    "unresolved_candidates": [], "active_seconds": 1,
                    "started_epoch": time.time() - 2,
                    "started_utc": "2026-11-03T00:00:00Z",
                }

                instance.save("comparison_phase_smoke")

                checkpoint = json.loads(instance.state_path.read_text())
                report = json.loads((root / "final-report.json").read_text())
                markdown = (root / "final-report.md").read_text()
                self.assertEqual(checkpoint["phase"], phase)
                self.assertNotIn("phase", report)
                self.assertNotIn(phase, markdown)

    def test_python_u128_comparison_limits_bigint_files_to_128_bits(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            big, u128 = root / "big", root / "u128"
            big.mkdir()
            u128.mkdir()
            row = {"value": "5", "binary": "101", "hex": "5", "classification": "prime"}
            footer = {"status": "completed", "bits": 3, "assignment_start": "0",
                      "assignment_end_exclusive": "1", "assignment_space": "2",
                      "generated": "1", "decimal_pass": 1, "unresolved": 0,
                      "seconds": 0.1, "completion_utc": "2026-11-03T00:00:00Z"}
            (big / "big-b003-a0-e1.jsonl").write_text(json.dumps(row) + "\n" + json.dumps(footer) + "\n")
            other = {"value": "9", "binary": "1001", "hex": "9", "classification": "composite"}
            (big / "big-b129-a0-e1.jsonl").write_text(json.dumps(other) + "\n" + json.dumps(footer) + "\n")
            (u128 / "part-b003-a0-e1.jsonl").write_text(json.dumps(row) + "\n" + json.dumps(footer) + "\n")
            output = root / "comparison.json"
            result = coordinator.compare_u128(big, u128, output)
            self.assertEqual(result["status"], "passed")
            self.assertEqual(result["values"], ["5"])
            self.assertEqual(json.loads(output.read_text())["status"], "passed")
            mismatched = dict(row, binary="111")
            (u128 / "part-b003-a0-e1.jsonl").write_text(json.dumps(mismatched) + "\n" + json.dumps(footer) + "\n")
            self.assertEqual(coordinator.compare_u128(big, u128, output)["status"], "failed")
            with open(big / "big-b003-a0-e1.jsonl", "a", encoding="utf-8") as stream:
                stream.write("{malformed json}\n")
            malformed = coordinator.compare_u128(big, u128, output)
            self.assertEqual(malformed["status"], "failed")
            self.assertIn("malformed JSON", malformed["error"])
            self.assertEqual(json.loads(output.read_text())["status"], "failed")
            (big / "big-b003-a0-e1.jsonl").write_text(
                json.dumps({"classification": "prime"}) + "\n" + json.dumps(footer) + "\n"
            )
            incomplete = coordinator.compare_u128(big, u128, output)
            self.assertEqual(incomplete["status"], "failed")
            self.assertIn("missing value", incomplete["error"])

    def test_miller_rabin_witness_checks_base_range(self):
        self.assertTrue(coordinator.is_mr_witness(9, 2))
        self.assertFalse(coordinator.is_mr_witness(9, 9))

    def test_valid_prime_record(self):
        row = self.record(5, "prime", "deterministic_trial_division")
        self.assertEqual(self.validate(row, 3, 0, 1, False)[0], "prime")

    def test_legacy_row_derives_and_checks_assignment_range(self):
        legacy = self.record(9, "composite", "factor=3")
        self.assertEqual(self.validate(legacy, 4, 0, 1, False)[0], "composite")
        self.assertIsNone(self.validate(legacy, 4, 1, 2, False))

    def test_saved_assignment_index_must_match_derived_index(self):
        row = self.record(9, "composite", "factor=3")
        row["assignment_index"] = "1"
        self.assertIsNone(self.validate(row, 4, 0, 2, False))
        row["assignment_index"] = "0"
        self.assertEqual(self.validate(row, 4, 0, 1, True)[0], "composite")

    def test_rejects_improper_factor_equal_to_value(self):
        row = self.record(9, "composite", "factor=9")
        self.assertIsNone(self.validate(row, 4, 0, 1, False))

    def test_rejects_miller_rabin_base_equal_to_value(self):
        row = self.record(9, "composite", "miller_rabin_base=9")
        self.assertIsNone(self.validate(row, 4, 0, 1, False))

    def test_accepts_proper_factor_and_witness(self):
        factor = self.record(9, "composite", "factor=3")
        witness = self.record(9, "composite", "miller_rabin_base=2")
        for row in (factor, witness):
            self.assertEqual(self.validate(row, 4, 0, 1, False)[0], "composite")

    def test_rejects_bad_palindrome_representation(self):
        row = self.record(9, "composite", "factor=3")
        row["binary"] = "1000"
        self.assertIsNone(self.validate(row, 4, 0, 1, False))

    def test_stop_checkpoint_keeps_committed_cursor_and_clears_batch(self):
        with tempfile.TemporaryDirectory() as directory:
            instance = object.__new__(coordinator.Coordinator)
            instance.root = Path(directory)
            (instance.root / "STOP").touch()
            instance.state = {
                "status": "running", "next_start": "4000000000",
                "active_worker_count": 16, "active_batch_start": "4000000000",
                "active_batch_end": "5600000000", "active_batch_assignments": "1600000000",
                "active_batch_started_utc": "2026-11-03T10:00:00Z",
                "active_batch_elapsed_seconds": 10, "interrupted_batch_seconds": 2,
            }
            saved_events = []
            instance.save = saved_events.append
            instance.save_stopped_batch(time.monotonic() - 5)
            self.assertEqual(instance.state["status"], "stopped")
            self.assertEqual(instance.state["next_start"], "4000000000")
            self.assertEqual(instance.state["active_worker_count"], 0)
            self.assertNotIn("active_batch_start", instance.state)
            self.assertGreaterEqual(instance.state["interrupted_batch_seconds"], 7)
            self.assertEqual(saved_events, ["stop_requested_during_batch"])

    def test_unresolved_candidates_are_persisted_and_listed_in_markdown(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            partition = root / "big-b004-a0-e1.jsonl"
            result_row = {
                "value": "9", "binary": "1001", "hex": "9",
                "binary_bits": 4, "hex_digits": 1, "decimal_digits": 1,
                "classification": "unresolved",
                "method": "probable-prime-requires-proof",
                "proof_evidence": "certificate pending", "assignment_index": "0",
            }
            footer = {"status": "completed", "format_version": 2, "bits": 4,
                      "assignment_start": "0", "assignment_end_exclusive": "1",
                      "assignment_space": "2", "generated": "1", "decimal_pass": 1}
            partition.write_text(json.dumps(result_row) + "\n" + json.dumps(footer) + "\n")
            validator = object.__new__(coordinator.Coordinator)
            validator.counts = {4: 2}
            validator.binary = Path("mock-binary")
            completed = type("Completed", (), {"returncode": 0, "stdout": "0\n"})()
            with patch.object(coordinator.subprocess, "run", return_value=completed):
                stats = validator.file_stats(partition, 4, 0, 1)
            candidate = stats["unresolved_candidates"][0]
            self.assertEqual(candidate["partition"], partition.name)
            instance = object.__new__(coordinator.Coordinator)
            instance.root = root
            instance.state_path = root / "checkpoint.json"
            instance.counts = {bits: 1 for bits in range(2, 15)}
            instance.total = sum(instance.counts.values())
            instance.state = {
                "status": "running", "phase": "above_128", "bits": 14,
                "next_start": "1", "completed_assignments": "1", "partitions": 1,
                "result_records": 1, "prime_records": 0, "composite_records": 0,
                "unresolved_records": stats["unresolved"], "unresolved_candidates": [candidate],
                "active_seconds": 1, "started_epoch": time.time()-2,
                "started_utc": "2026-11-03T00:00:00Z",
            }
            instance.save("unresolved_smoke")
            markdown = (root / "final-report.md").read_text()
            report = json.loads((root / "final-report.json").read_text())
            checkpoint = json.loads((root / "checkpoint.json").read_text())
            self.assertIn("## Unresolved candidates", markdown)
            self.assertIn("`9`", markdown)
            self.assertIn("`1001`", markdown)
            self.assertIn("certificate pending", markdown)
            self.assertNotIn("candidate index", markdown.lower())
            self.assertNotIn("candidate slot", markdown.lower())
            self.assertIn("Candidates processed", markdown)
            self.assertIn("Next candidate position", markdown)
            self.assertIn("This report tracks a restartable search", markdown)
            self.assertLess(markdown.index("This report tracks a restartable search"), markdown.index("- **Status:**"))
            self.assertNotIn("**Phase:**", markdown)
            self.assertNotIn("cross-check", markdown.lower())
            self.assertEqual(report["unresolved_candidates"], [candidate])
            self.assertEqual(checkpoint["unresolved_candidates"], [candidate])
            self.assertNotIn("through_128_assignments", report)
            self.assertNotIn("percentage_of_2_128_workload", report)
            self.assertNotIn("decimal-palindrome check", markdown)
            self.assertEqual(report["integers_excluded_by_binary_hex_palindrome_constraints_through_current_bit"], (1 << 14) - 2 - 13)
            self.assertIn("Integers ruled out by binary/hex palindrome constraints through bit length 14", markdown)

    def test_legacy_unresolved_recovery_ignores_files_beyond_checkpoint_cursor(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            names = [
                "big-b003-a0-e2.jsonl",    # Earlier bit length: committed.
                "big-b004-a0-e1.jsonl",    # Current bit length: before cursor.
                "big-b004-a1-e2.jsonl",    # Current bit length: beyond cursor.
                "big-b005-a0-e1.jsonl",    # Future bit length.
            ]
            for name in names:
                (root / name).touch()
            recovered_rows = {
                names[0]: [{"value": "3"}],
                names[1]: [{"value": "9"}],
                names[2]: [{"value": "99"}],
                names[3]: [{"value": "999"}],
            }
            instance = object.__new__(coordinator.Coordinator)
            instance.root = root
            instance.state = {"bits": 4, "next_start": "1", "unresolved_records": 2}
            instance.file_stats = lambda path, _bits, _start, _end: {
                "unresolved_candidates": recovered_rows[path.name]
            }
            result = instance.recover_unresolved_candidates()
            self.assertEqual(result, [{"value": "3"}, {"value": "9"}])


if __name__ == "__main__":
    unittest.main()
