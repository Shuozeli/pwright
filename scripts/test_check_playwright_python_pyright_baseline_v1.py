"""Mutation controls for the exact, full-scope Python Pyright debt gate."""

from __future__ import annotations

import copy
import tempfile
import unittest
from pathlib import Path

import check_playwright_python_pyright_baseline_v1 as gate


class PyrightBaselineGateTests(unittest.TestCase):
    def setUp(self) -> None:
        self.root = gate.ROOT
        self.report: dict[str, object] = {
            "version": "1.1.405",
            "generalDiagnostics": [
                {
                    "file": str(
                        self.root / "packages/pwright-python/tests/official/example.py"
                    ),
                    "severity": "error",
                    "rule": "reportArgumentType",
                    "message": "pinned type mismatch",
                    "range": {
                        "start": {"line": 3, "character": 4},
                        "end": {"line": 3, "character": 12},
                    },
                }
            ],
            "summary": {
                "filesAnalyzed": 91,
                "errorCount": 1,
                "warningCount": 0,
                "informationCount": 0,
            },
        }
        self.baseline: dict[str, object] = {
            "schema": 1,
            "kind": "pwright-python-pyright-debt-v1",
            "pyrightVersion": "1.1.405",
            "typeCheckingMode": "strict",
            "pyrightConfigSha256": gate.PYRIGHT_CONFIG_SHA256,
            "targets": list(gate.PYRIGHT_TARGETS),
            **gate.report_fingerprint(self.report, self.root),
        }

    def assert_rejected(self, report: dict[str, object] | None = None) -> None:
        with self.assertRaises(gate.GateError):
            gate.verify_report(report or self.report, self.baseline, self.root)

    def test_exact_report_matches(self) -> None:
        self.assertEqual(
            gate.verify_report(self.report, self.baseline, self.root),
            gate.report_fingerprint(self.report, self.root),
        )

    def test_command_resolves_dependencies_from_selected_python(self) -> None:
        # Arrange
        python = "/isolated/bin/python"

        # Act
        command = gate.pyright_command(python)

        # Assert
        self.assertEqual(
            command[:6],
            [python, "-m", "pyright", "--pythonpath", python, "--outputjson"],
        )
        self.assertEqual(command[6:], list(gate.PYRIGHT_TARGETS))

    def test_changed_message_is_rejected(self) -> None:
        report = copy.deepcopy(self.report)
        report["generalDiagnostics"][0]["message"] = "changed mismatch"
        self.assert_rejected(report)

    def test_changed_rule_is_rejected(self) -> None:
        report = copy.deepcopy(self.report)
        report["generalDiagnostics"][0]["rule"] = "reportUnknownMemberType"
        self.assert_rejected(report)

    def test_changed_range_is_rejected(self) -> None:
        report = copy.deepcopy(self.report)
        report["generalDiagnostics"][0]["range"]["start"]["line"] = 4
        self.assert_rejected(report)

    def test_changed_path_is_rejected(self) -> None:
        report = copy.deepcopy(self.report)
        report["generalDiagnostics"][0]["file"] = str(self.root / "other.py")
        self.assert_rejected(report)

    def test_added_diagnostic_is_rejected(self) -> None:
        report = copy.deepcopy(self.report)
        report["generalDiagnostics"].append(
            copy.deepcopy(report["generalDiagnostics"][0])
        )
        report["summary"]["errorCount"] = 2
        self.assert_rejected(report)

    def test_removed_diagnostic_is_rejected(self) -> None:
        report = copy.deepcopy(self.report)
        report["generalDiagnostics"].clear()
        report["summary"]["errorCount"] = 0
        self.assert_rejected(report)

    def test_source_scope_change_is_rejected(self) -> None:
        self.baseline["targets"] = ["python"]
        self.assert_rejected()

    def test_analyzed_file_count_change_is_rejected(self) -> None:
        report = copy.deepcopy(self.report)
        report["summary"]["filesAnalyzed"] = 90
        self.assert_rejected(report)

    def test_changed_tool_version_is_rejected(self) -> None:
        report = copy.deepcopy(self.report)
        report["version"] = "1.1.406"
        self.assert_rejected(report)

    def test_outside_repository_diagnostic_is_rejected(self) -> None:
        report = copy.deepcopy(self.report)
        report["generalDiagnostics"][0]["file"] = "/outside.py"
        self.assert_rejected(report)

    def test_summary_count_mismatch_is_rejected(self) -> None:
        report = copy.deepcopy(self.report)
        report["summary"]["errorCount"] = 0
        self.assert_rejected(report)

    def test_checked_in_versioned_baseline_matches_script_pins(self) -> None:
        baseline = gate.json.loads(gate.BASELINE.read_text())
        gate.verify_versioned_baseline(baseline)

    def test_changed_versioned_diagnostic_hash_is_rejected(self) -> None:
        baseline = gate.json.loads(gate.BASELINE.read_text())
        baseline["diagnosticsSha256"] = "0" * 64
        with self.assertRaises(gate.GateError):
            gate.verify_versioned_baseline(baseline)

    def test_changed_versioned_error_count_is_rejected(self) -> None:
        baseline = gate.json.loads(gate.BASELINE.read_text())
        baseline["errorCount"] = 67
        with self.assertRaises(gate.GateError):
            gate.verify_versioned_baseline(baseline)

    def test_pinned_sources_are_byte_checked(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            sources = {}
            for index, relative_path in enumerate(gate.PINNED_SOURCES):
                path = root / relative_path
                path.parent.mkdir(parents=True, exist_ok=True)
                payload = f"source {index}".encode()
                path.write_bytes(payload)
                sources[relative_path] = gate.hashlib.sha256(payload).hexdigest()
            gate.verify_sources({"shaPinnedSources": sources}, root, sources)
            first = root / next(iter(sources))
            first.write_bytes(b"changed")
            with self.assertRaises(gate.GateError):
                gate.verify_sources({"shaPinnedSources": sources}, root, sources)


if __name__ == "__main__":
    unittest.main()
