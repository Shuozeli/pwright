"""Fail on any drift from the explicitly recorded Python strict-Pyright debt.

This gate analyzes the same full Python source/test/script target set as the
original CI command. A passing baseline is not a zero-error type check.
"""

from __future__ import annotations

import hashlib
import importlib.metadata
import json
import subprocess
import sys
from collections import Counter
from collections.abc import Mapping
from pathlib import Path
from typing import cast

ROOT = Path(__file__).resolve().parents[1]
PACKAGE = ROOT / "packages/pwright-python"
BASELINE = ROOT / "scripts/playwright-python-pyright-debt-v1.json"
PYRIGHT_TARGETS = (
    "python",
    "tests",
    "../../scripts/check_playwright_python_official_source.py",
    "../../scripts/playwright_python_official_provenance.py",
    "../../scripts/run_playwright_python_official_conformance.py",
    "../../scripts/select_playwright_python_junit.py",
    "../../scripts/verify_playwright_python_official_provenance.py",
    "../../scripts/audit_playwright_python_official_invocations.py",
    "../../scripts/check_playwright_python_environment_blockers.py",
    "../../scripts/test_check_playwright_python_environment_blockers.py",
    "../../scripts/test_playwright_python_official_provenance.py",
    "../../scripts/test_run_playwright_python_official_conformance.py",
    "../../scripts/test_verify_playwright_python_official_provenance.py",
    "../../scripts/test_audit_playwright_python_official_invocations.py",
)
REQUIRED_VERSIONS = {
    "pyright": "1.1.405",
    "pytest": "8.4.2",
    "pytest-asyncio": "1.2.0",
    "playwright": "1.58.0",
}
PYRIGHT_CONFIG_SHA256 = (
    "20005c9d41430771a63ebdb4c9dd9f4c6736910e31e6901e41eb8555c3d5895e"
)
KNOWN_DIAGNOSTICS_SHA256 = (
    "0e953b32aa96e8164e19fd6634929d173785903409233014c5a753f5bb187a5f"
)
KNOWN_ERROR_COUNT = 66
PINNED_SOURCES = {
    (
        "packages/pwright-python/tests/official/connect_lifecycle_synthetic_v1/"
        "pwright_python_connect_lifecycle_synthetic_v1.py"
    ): "fc4f4c4a612af65a8cb8c8ac5018c5c1f81eabe3914883d5a4778e1b6722dc5d",
    (
        "packages/pwright-python/tests/official/connect_lifecycle_synthetic_v2/"
        "pwright_python_connect_lifecycle_synthetic_v2.py"
    ): "faa7dd0b5bfb71ada724146b868859bf9fd5d5b9027f5a105337c829098d7fe1",
    (
        "packages/pwright-python/tests/official/connect_remote_kill_synthetic_v1/"
        "pwright_python_connect_remote_kill_synthetic_v1.py"
    ): "41e7b4cfc2480988640eb223675bcff2c9dbbc25a3f822256451946ec02cbe99",
    (
        "packages/pwright-python/tests/official/ws_connect_guard_strict_v2/"
        "pwright_official_connect_guard_v2.py"
    ): "f1cc02ce09be1688d2a5882afbc36ee22987871a517cab134a52e55374083d66",
    (
        "packages/pwright-python/tests/official/ws_connect_refused_guard_strict_v1/"
        "pwright_official_connect_refused_guard_v1.py"
    ): "10d6ede9b5a8ecf552bb6caaab6d33474e40d23cc1ca9a00de1a988cfa9d9450",
    (
        "scripts/run_playwright_python_upstream_skip_markers_strict_v1.py"
    ): "61027b05720ffee373ea17b26b6c7d696ea010c2f8dffee0d511c2c36b9bf550",
    (
        "scripts/verify_playwright_python_upstream_skip_markers_strict_v1.py"
    ): "168b3801f88248f11d0d628521392ce6a05615cede547a2e708492a20391feda",
    (
        "packages/pwright-python/tests/official/"
        "upstream-skip-callable-fixtures-strict-v2-manifest.json"
    ): "929225455a1f371ff671e5800455746f7c03d18c40f709d68d6f6a7ec077eb69",
    (
        "packages/pwright-python/tests/official/upstream_skip_guard_strict_v1/"
        "pwright_official_upstream_skip_guard_v1.py"
    ): "9e4b8cbddd267db02bba68b3c139abf2422c55b062deef284f98d103f63ee448",
    (
        "scripts/run_playwright_python_upstream_skip_callable_fixtures_strict_v2.py"
    ): "ccd4032b0d2706008477b3c418e7b208766d3ca893610ecdb4d7516bff0fcdbd",
    (
        "scripts/verify_playwright_python_upstream_skip_callable_fixtures_strict_v2.py"
    ): "95a49c1d391a55b792828ecd9bbae38a41b741c94d34e4209502c69dcfbc4526",
    (
        "scripts/promote_playwright_python_upstream_skip_callable_fixtures_strict_v2.py"
    ): "891543d38877631763806b68a4449d0e7689e7269f3473ebfe27d2ecd7401bd1",
}


class GateError(ValueError):
    """An untrusted report, environment, or pinned debt baseline changed."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise GateError(message)


def _mapping(value: object, label: str) -> dict[str, object]:
    require(isinstance(value, dict), f"{label} must be an object")
    assert isinstance(value, dict)
    checked = cast(dict[object, object], value)
    require(
        all(isinstance(key, str) for key in checked), f"{label} keys must be strings"
    )
    return cast(dict[str, object], checked)


def _integer(value: object, label: str) -> int:
    require(type(value) is int and value >= 0, f"{label} must be a nonnegative integer")
    assert isinstance(value, int)
    return value


def _string(value: object, label: str) -> str:
    require(isinstance(value, str), f"{label} must be a string")
    assert isinstance(value, str)
    return value


def _diagnostic_record(value: object, root: Path) -> tuple[object, ...]:
    entry = _mapping(value, "diagnostic")
    path = Path(_string(entry.get("file"), "diagnostic.file"))
    require(path.is_absolute(), "diagnostic.file must be absolute")
    try:
        relative_path = path.resolve().relative_to(root.resolve()).as_posix()
    except ValueError as error:
        raise GateError("diagnostic.file escapes repository") from error
    span = _mapping(entry.get("range"), "diagnostic.range")
    start = _mapping(span.get("start"), "diagnostic.range.start")
    end = _mapping(span.get("end"), "diagnostic.range.end")
    return (
        relative_path,
        _string(entry.get("severity"), "diagnostic.severity"),
        _string(entry.get("rule"), "diagnostic.rule"),
        _integer(start.get("line"), "diagnostic.range.start.line"),
        _integer(start.get("character"), "diagnostic.range.start.character"),
        _integer(end.get("line"), "diagnostic.range.end.line"),
        _integer(end.get("character"), "diagnostic.range.end.character"),
        _string(entry.get("message"), "diagnostic.message"),
    )


def report_fingerprint(payload: object, root: Path) -> dict[str, object]:
    report = _mapping(payload, "Pyright report")
    require(
        report.get("version") == REQUIRED_VERSIONS["pyright"], "Pyright version drift"
    )
    diagnostics = report.get("generalDiagnostics")
    require(isinstance(diagnostics, list), "generalDiagnostics must be an array")
    assert isinstance(diagnostics, list)
    records = sorted(
        _diagnostic_record(entry, root) for entry in cast(list[object], diagnostics)
    )
    counts = Counter(record[1] for record in records)
    summary = _mapping(report.get("summary"), "Pyright summary")
    analyzed = _integer(summary.get("filesAnalyzed"), "summary.filesAnalyzed")
    for severity in ("error", "warning", "information"):
        require(
            _integer(summary.get(f"{severity}Count"), f"summary.{severity}Count")
            == counts[severity],
            f"Pyright {severity} count disagrees with diagnostics",
        )
    require(
        set(counts).issubset({"error", "warning", "information"}), "unknown severity"
    )
    encoded = json.dumps(records, ensure_ascii=False, separators=(",", ":")).encode(
        "utf-8"
    )
    return {
        "filesAnalyzed": analyzed,
        "errorCount": counts["error"],
        "warningCount": counts["warning"],
        "informationCount": counts["information"],
        "diagnosticsSha256": hashlib.sha256(encoded).hexdigest(),
        "diagnosticsByFile": dict(
            sorted(Counter(record[0] for record in records).items())
        ),
        "diagnosticsByRule": dict(
            sorted(Counter(record[2] for record in records).items())
        ),
    }


def verify_report(payload: object, baseline: object, root: Path) -> dict[str, object]:
    expected = _mapping(baseline, "baseline")
    require(expected.get("schema") == 1, "baseline schema drift")
    require(
        expected.get("kind") == "pwright-python-pyright-debt-v1",
        "baseline kind drift",
    )
    require(
        expected.get("pyrightVersion") == REQUIRED_VERSIONS["pyright"],
        "baseline tool drift",
    )
    require(expected.get("typeCheckingMode") == "strict", "baseline strict mode drift")
    require(
        expected.get("pyrightConfigSha256") == PYRIGHT_CONFIG_SHA256,
        "Pyright config pin drift",
    )
    require(
        expected.get("targets") == list(PYRIGHT_TARGETS), "baseline target scope drift"
    )
    observed = report_fingerprint(payload, root)
    for key, value in observed.items():
        require(expected.get(key) == value, f"{key} differs from recorded Pyright debt")
    return observed


def verify_versioned_baseline(baseline: object) -> None:
    expected = _mapping(baseline, "baseline")
    require(
        expected.get("diagnosticsSha256") == KNOWN_DIAGNOSTICS_SHA256,
        "versioned diagnostic pin changed",
    )
    require(
        expected.get("errorCount") == KNOWN_ERROR_COUNT,
        "versioned error count changed",
    )


def verify_sources(
    baseline: object, root: Path, source_pins: Mapping[str, str] = PINNED_SOURCES
) -> None:
    expected = _mapping(baseline, "baseline")
    sources = _mapping(expected.get("shaPinnedSources"), "baseline.shaPinnedSources")
    require(sources == source_pins, "immutable plugin source pins changed")
    for relative_path, digest in sources.items():
        path = root / relative_path
        require(
            path.is_file() and not path.is_symlink(),
            f"missing pinned source: {relative_path}",
        )
        require(
            hashlib.sha256(path.read_bytes()).hexdigest() == digest,
            f"pinned source changed: {relative_path}",
        )


def verify_environment() -> None:
    require(sys.version_info[:2] == (3, 12), "Pyright baseline requires Python 3.12")
    for package, expected in REQUIRED_VERSIONS.items():
        require(
            importlib.metadata.version(package) == expected, f"{package} version drift"
        )
    content = (PACKAGE / "pyproject.toml").read_text()
    marker = "[tool.pyright]\n"
    require(content.count(marker) == 1, "Pyright config section changed")
    section = marker + content.split(marker, 1)[1].split("\n[", 1)[0]
    require(
        hashlib.sha256(section.encode()).hexdigest() == PYRIGHT_CONFIG_SHA256,
        "Pyright strict config or scope changed",
    )


def pyright_command(python: str) -> list[str]:
    # Pyright's target language is 3.9, but dependencies come from the
    # selected 3.12 verification environment, not macOS system Python 3.9.
    return [
        python,
        "-m",
        "pyright",
        "--pythonpath",
        python,
        "--outputjson",
        *PYRIGHT_TARGETS,
    ]


def main() -> int:
    try:
        verify_environment()
        baseline = json.loads(BASELINE.read_text())
        verify_versioned_baseline(baseline)
        verify_sources(baseline, ROOT)
        process = subprocess.run(
            pyright_command(sys.executable),
            cwd=PACKAGE,
            capture_output=True,
            text=True,
            timeout=180,
            check=False,
        )
        require(
            process.returncode in (0, 1), f"Pyright failed: {process.stderr.strip()}"
        )
        observed = verify_report(json.loads(process.stdout), baseline, ROOT)
    except (
        GateError,
        OSError,
        ValueError,
        subprocess.TimeoutExpired,
        importlib.metadata.PackageNotFoundError,
    ) as error:
        print(f"Python Pyright baseline v1 FAILED: {error}", file=sys.stderr)
        return 1
    print(
        "Python strict Pyright analyzed "
        f"{observed['filesAnalyzed']} files; exact baseline of "
        f"{observed['errorCount']} known errors matched. "
        "This is type debt, not a zero-error type check."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
