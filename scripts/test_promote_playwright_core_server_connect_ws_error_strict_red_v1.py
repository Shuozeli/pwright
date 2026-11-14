"""Portable synthetic controls for the diagnostic-only Core Server-connect RED pair."""

from __future__ import annotations

import json
from pathlib import Path

import promote_playwright_core_server_connect_ws_error_strict_red_v1 as promoter
import pytest

ORIGINAL_TARGETS = dict(promoter.TARGETS)


def _row(index: int) -> dict:
    if index in (1, 2):
        spec_id, kind = ORIGINAL_TARGETS[31_231 + index]
        file = "library/browsertype-connect.spec.ts"
        return {
            "specId": spec_id,
            "projectId": "chromium-library",
            "projectName": "chromium-library",
            "sourceClassification": "implementation-internal",
            "classificationReasons": promoter.CLASSIFICATION_REASONS,
            "sourceFile": promoter.capture.SOURCE,
            "file": file,
            "line": 122,
            "column": 5,
            "expectedStatus": "passed",
            "role": "official-entrypoint",
            "suite": "library",
            "titlePath": [file, kind, "should print ws error"],
        }
    file = f"library/other-{index}.spec.ts"
    return {
        "specId": f"other-{index}",
        "projectId": "chromium-library",
        "projectName": "chromium-library",
        "sourceClassification": "public-api-candidate",
        "classificationReasons": [],
        "sourceFile": f"tests/{file}",
        "file": file,
        "line": 10 + index,
        "column": 5,
        "expectedStatus": "passed",
        "role": "official-entrypoint",
        "suite": "library",
        "titlePath": [file, f"other {index}"],
    }


def _line(record: dict, *, spaced: bool = False) -> bytes:
    if spaced:
        return (json.dumps(record, indent=2).replace("\n", " ") + "\n").encode()
    return (json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n").encode()


def _fixture(
    monkeypatch: pytest.MonkeyPatch, lane: str
) -> tuple[list[bytes], list[bytes], dict]:
    monkeypatch.setattr(
        promoter, "TARGETS", {1: ORIGINAL_TARGETS[31_232], 2: ORIGINAL_TARGETS[31_233]}
    )
    inventory = [_line(_row(index)) for index in range(5)]
    baseline = []
    for index, line in enumerate(inventory):
        record = json.loads(line)
        baseline.append(
            _line(
                {**record, "attempts": 0, "outcome": "not-run"},
                spaced=index not in promoter.TARGETS,
            )
        )
    cases = {
        spec_id: {
            "specId": spec_id,
            "kind": kind,
            "line": 122,
            "status": "unexpected" if lane == "candidate" else "expected",
            "errors": ["official mismatch"] if lane == "candidate" else [],
        }
        for spec_id, kind in promoter.TARGETS.values()
    }
    return inventory, baseline, cases


@pytest.mark.parametrize("lane", ["candidate", "oracle"])
def test_promotes_exactly_two_rows_without_rewriting_other_lines(
    monkeypatch: pytest.MonkeyPatch, lane: str
) -> None:
    # Arrange
    inventory, baseline, cases = _fixture(monkeypatch, lane)

    # Act
    successor, outcomes, scopes = promoter.transform_lines(
        inventory,
        baseline,
        lane=lane,
        cases=cases,
        result_file="/synthetic/official-result.json",
        count=5,
        prior_outcomes={"not-run": 5},
        prior_scopes={},
    )

    # Assert
    successor_lines = successor.splitlines(keepends=True)
    assert len(successor_lines) == len(baseline) == 5
    assert [
        index
        for index, (old, new) in enumerate(zip(baseline, successor_lines))
        if old != new
    ] == [1, 2]
    assert all(successor_lines[index] == baseline[index] for index in (0, 3, 4))
    target_rows = [json.loads(successor_lines[index]) for index in (1, 2)]
    assert all(
        row["sourceClassification"] == "implementation-internal" for row in target_rows
    )
    assert all(
        row["classificationReasons"] == promoter.CLASSIFICATION_REASONS
        for row in target_rows
    )
    assert all(
        row["candidateRuntimeRequired"] == (lane == "candidate") for row in target_rows
    )
    assert all(
        row["outcome"] == ("unexpected" if lane == "candidate" else "expected")
        for row in target_rows
    )
    assert all(
        row.get("error") == ("official mismatch" if lane == "candidate" else None)
        for row in target_rows
    )
    assert outcomes == {
        "not-run": 3,
        "unexpected" if lane == "candidate" else "expected": 2,
    }
    assert scopes == {promoter.SCOPE: 2}


@pytest.mark.parametrize(
    "drift", ["outcome", "classification", "target-position", "count"]
)
def test_rejects_baseline_or_target_drift(
    monkeypatch: pytest.MonkeyPatch, drift: str
) -> None:
    # Arrange
    inventory, baseline, cases = _fixture(monkeypatch, "candidate")
    if drift == "count":
        baseline.pop()
    else:
        record = json.loads(baseline[1])
        if drift == "outcome":
            record["outcome"] = "expected"
        elif drift == "classification":
            record["sourceClassification"] = "public-api-candidate"
        else:
            record["specId"] = "different-spec-id"
        baseline[1] = _line(record)

    # Act / Assert
    with pytest.raises(ValueError):
        promoter.transform_lines(
            inventory,
            baseline,
            lane="candidate",
            cases=cases,
            result_file="/synthetic/official-result.json",
            count=5,
            prior_outcomes={"not-run": 5},
            prior_scopes={},
        )


def test_publish_is_atomic_no_replace_and_existing_is_verified(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    monkeypatch.setattr(promoter, "OUTCOMES", tmp_path)
    monkeypatch.setattr(promoter, "OUTPUT", tmp_path / "successor")
    files = {
        "candidate.jsonl": b"candidate\n",
        "oracle.jsonl": b"oracle\n",
        "promotion-receipt.json": b'{"scope":"synthetic"}\n',
    }
    monkeypatch.setattr(promoter, "prepare_pair", lambda: files)

    # Act
    promoter.publish(files)

    # Assert
    assert promoter.verify_existing() == {"scope": "synthetic"}
    with pytest.raises(ValueError, match="already exists"):
        promoter.publish(files)
    (promoter.OUTPUT / "candidate.jsonl").write_bytes(b"tampered\n")
    with pytest.raises(ValueError, match="drifted"):
        promoter.verify_existing()
