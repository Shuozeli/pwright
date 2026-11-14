"""Guard the exact two-row Core CDP URL-query ledger promotion."""

from __future__ import annotations

import json
from pathlib import Path

import pytest
from promote_playwright_core_cdp_url_query_strict_pair import (
    BASE,
    LANES,
    TARGETS,
    check_official_result,
    prepare_lane,
    sealed_file,
    validate_published_files,
    validate_target,
)


def _target_record(lane: str, index: int) -> dict:
    with (BASE / f"{lane}.jsonl").open("rb") as baseline:
        for line_number, line in enumerate(baseline):
            if line_number == index:
                return json.loads(line)
    raise AssertionError("target row not found")


@pytest.mark.parametrize("lane", ["candidate", "oracle"])
def test_prepared_successor_changes_only_two_pinned_lines(lane: str) -> None:
    # Arrange
    baseline = BASE / f"{lane}.jsonl"

    # Act
    successor, summary, _ = prepare_lane(lane)

    # Assert
    changed: list[int] = []
    with baseline.open("rb") as before:
        for index, (old, new) in enumerate(
            zip(before, successor.splitlines(keepends=True), strict=True)
        ):
            if old != new:
                changed.append(index)
                original = json.loads(old)
                promoted = json.loads(new)
                assert original["sourceClassification"] == promoted["sourceClassification"]
                assert original["classificationReasons"] == promoted["classificationReasons"]
                assert promoted["outcome"] == "expected"
    assert changed == sorted(TARGETS)
    scopes = json.loads(summary)["explicitEvidenceScopes"]
    assert scopes["upstream-private-source-only"] == 2610
    assert scopes["official-browser-free-cdp-handshake-headers"] == 2


def test_rejects_source_classification_laundering() -> None:
    # Arrange
    index = 31_781
    record = _target_record("candidate", index)
    record["sourceClassification"] = "public-api-candidate"

    # Act / Assert
    with pytest.raises(ValueError, match="exact unrun implementation-internal"):
        validate_target(record, index)


def test_rejects_fabricated_official_pass() -> None:
    # Arrange
    strict = json.loads(LANES["candidate"]["strict"].read_text())
    strict["selected"][0]["status"] = "unexpected"

    # Act / Assert
    with pytest.raises(ValueError, match="did not both pass"):
        check_official_result(strict, "candidate")


def test_rejects_wrong_official_spec_id() -> None:
    # Arrange
    strict = json.loads(LANES["oracle"]["strict"].read_text())
    strict["selected"][0]["specId"] = "0" * 41

    # Act / Assert
    with pytest.raises(ValueError, match="IDs/locations drifted"):
        check_official_result(strict, "oracle")


def test_rejects_tampered_base_hash(tmp_path: Path) -> None:
    # Arrange
    fake_base = tmp_path / "fake.jsonl"
    fake_base.write_bytes(b"{}\n")

    # Act / Assert
    with pytest.raises(ValueError, match="SHA-256 drift"):
        sealed_file(fake_base, "0" * 64)


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

    # Act / Assert
    with pytest.raises(ValueError, match="drifted"):
        validate_published_files(directory, {"candidate.jsonl": b"candidate\n"})
