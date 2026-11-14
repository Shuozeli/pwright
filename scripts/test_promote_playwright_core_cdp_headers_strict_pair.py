"""Guard the exact two-row Core CDP-header ledger promotion."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from promote_playwright_core_cdp_headers_strict_pair import (
    LANES,
    TARGETS,
    prepare_lane,
    selected_report,
    validate_published_files,
    validate_target,
)


def _target_record(lane: str, index: int) -> dict:
    with LANES[lane]["base"].open("rb") as baseline:
        for line_number, line in enumerate(baseline):
            if line_number == index:
                return json.loads(line)
    raise AssertionError("target row not found")


def _default_test(report: dict) -> dict:
    def visit(suite: dict) -> dict | None:
        for spec in suite.get("specs", []):
            if spec.get("id") == TARGETS[31_783][0]:
                return spec["tests"][0]
        for child in suite.get("suites", []):
            if result := visit(child):
                return result
        return None

    for suite in report["suites"]:
        if result := visit(suite):
            return result
    raise AssertionError("default User-Agent case not found")


@pytest.mark.parametrize("lane", ["candidate", "oracle"])
def test_prepared_successor_changes_only_two_pinned_lines(lane: str) -> None:
    # Arrange
    baseline = LANES[lane]["base"]

    # Act
    successor, summary, _ = prepare_lane(lane)

    # Assert
    changed: list[int] = []
    with baseline.open("rb") as before:
        for index, (old, new) in enumerate(zip(before, successor.splitlines(keepends=True), strict=True)):
            if old != new:
                changed.append(index)
                original = json.loads(old)
                promoted = json.loads(new)
                assert original["sourceClassification"] == promoted["sourceClassification"]
                assert original["classificationReasons"] == promoted["classificationReasons"]
                assert promoted["outcome"] == LANES[lane]["expected"][index]
    assert changed == sorted(TARGETS)
    assert json.loads(summary)["explicitEvidenceScopes"]["upstream-private-source-only"] == 2610


def test_rejects_source_classification_laundering() -> None:
    # Arrange
    index = 31_780
    record = _target_record("candidate", index)
    record["sourceClassification"] = "public-api-candidate"

    # Act / Assert
    with pytest.raises(ValueError, match="exact unrun implementation-internal"):
        validate_target(record, index)


def test_rejects_fabricated_candidate_default_pass() -> None:
    # Arrange
    strict = json.loads(LANES["candidate"]["strict"].read_text())
    report = json.loads(Path(strict["result"]).read_text())
    _default_test(report)["status"] = "expected"

    # Act / Assert
    with pytest.raises(ValueError, match="outcome drifted"):
        selected_report(report, "candidate")


def test_rejects_unrelated_candidate_failure() -> None:
    # Arrange
    strict = json.loads(LANES["candidate"]["strict"].read_text())
    report = json.loads(Path(strict["result"]).read_text())
    _default_test(report)["results"][0]["error"]["location"]["line"] = 100

    # Act / Assert
    with pytest.raises(ValueError, match="not the pinned default User-Agent assertion"):
        selected_report(report, "candidate")


def test_rejects_tampered_immutable_output(tmp_path: Path) -> None:
    # Arrange
    expected = {"candidate.jsonl": b"candidate\n", "oracle.jsonl": b"oracle\n"}
    for name, content in expected.items():
        (tmp_path / name).write_bytes(content)
    (tmp_path / "oracle.jsonl").write_bytes(b"fabricated pass\n")

    # Act / Assert
    with pytest.raises(ValueError, match="drifted"):
        validate_published_files(tmp_path, expected)


def test_rejects_symlinked_immutable_output(tmp_path: Path) -> None:
    # Arrange
    external = tmp_path / "external"
    external.write_bytes(b"candidate\n")
    directory = tmp_path / "bundle"
    directory.mkdir()
    (directory / "candidate.jsonl").symlink_to(external)
    expected = {"candidate.jsonl": b"candidate\n"}

    # Act / Assert
    with pytest.raises(ValueError, match="drifted"):
        validate_published_files(directory, expected)
