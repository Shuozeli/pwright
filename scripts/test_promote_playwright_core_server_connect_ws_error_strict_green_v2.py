"""Negative controls for the immutable Core Server-connect GREEN successor."""

from __future__ import annotations

import json
from pathlib import Path

import promote_playwright_core_server_connect_ws_error_strict_green_v2 as promoter
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
        old = {**record, "attempts": 0, "outcome": "not-run"}
        if index in promoter.TARGETS:
            old.update(
                {
                    "attempts": 1,
                    "outcome": "unexpected" if lane == "candidate" else "expected",
                    "resultFile": "/synthetic/red.json",
                    "evidenceScope": promoter.previous.SCOPE,
                    "candidateRuntimeRequired": lane == "candidate",
                }
            )
            if lane == "candidate":
                old["error"] = (
                    "browserType.connect is not supported by the current Pwright native profile"
                )
        baseline.append(_line(old, spaced=index not in promoter.TARGETS))
    cases = {
        spec_id: {
            "specId": spec_id,
            "kind": kind,
            "line": 122,
            "title": "should print ws error",
            "status": "expected",
            "errors": [],
            "attempts": [{"retry": 0, "status": "passed", "workerIndex": 0}],
        }
        for spec_id, kind in promoter.TARGETS.values()
    }
    return inventory, baseline, cases


def _transform(
    inventory: list[bytes], baseline: list[bytes], cases: dict, lane: str
) -> tuple[bytes, dict[str, int], dict[str, int]]:
    return promoter.transform_lines(
        inventory,
        baseline,
        lane=lane,
        cases=cases,
        prior_result_file="/synthetic/red.json",
        result_file="/synthetic/green.json",
        count=5,
        prior_outcomes={
            "not-run": 3,
            "unexpected" if lane == "candidate" else "expected": 2,
        },
        prior_scopes={promoter.previous.SCOPE: 2},
    )


@pytest.mark.parametrize("lane", ["candidate", "oracle"])
def test_changes_exactly_two_rows_and_keeps_all_others_as_bytes(
    monkeypatch: pytest.MonkeyPatch, lane: str
) -> None:
    # Arrange
    inventory, baseline, cases = _fixture(monkeypatch, lane)

    # Act
    successor, outcomes, scopes = _transform(inventory, baseline, cases, lane)

    # Assert
    lines = successor.splitlines(keepends=True)
    assert len(lines) == len(baseline) == 5
    assert [i for i, (old, new) in enumerate(zip(baseline, lines)) if old != new] == [
        1,
        2,
    ]
    assert all(lines[index] == baseline[index] for index in (0, 3, 4))
    targets = [json.loads(lines[index]) for index in (1, 2)]
    assert all(
        target["outcome"] == "expected" and target["attempts"] == 1
        for target in targets
    )
    assert all(
        target["sourceClassification"] == "implementation-internal"
        for target in targets
    )
    assert all(
        target["classificationReasons"] == promoter.CLASSIFICATION_REASONS
        for target in targets
    )
    assert all(target["evidenceScope"] == promoter.SCOPE for target in targets)
    assert all(target["resultFile"] == "/synthetic/green.json" for target in targets)
    assert all("error" not in target for target in targets)
    assert all(
        target["candidateRuntimeRequired"] == (lane == "candidate")
        for target in targets
    )
    assert outcomes == {"not-run": 3, "expected": 2}
    assert scopes == {promoter.SCOPE: 2}


@pytest.mark.parametrize(
    "drift",
    [
        "red-outcome",
        "classification",
        "position",
        "count",
        "prior-result",
        "scope",
        "runtime-flag",
    ],
)
def test_rejects_red_predecessor_or_target_drift(
    monkeypatch: pytest.MonkeyPatch, drift: str
) -> None:
    # Arrange
    inventory, baseline, cases = _fixture(monkeypatch, "candidate")
    if drift == "count":
        baseline.pop()
    else:
        record = json.loads(baseline[1])
        if drift == "red-outcome":
            record["outcome"] = "expected"
        elif drift == "classification":
            record["sourceClassification"] = "public-api-candidate"
        elif drift == "position":
            record["specId"] = "different-spec-id"
        elif drift == "prior-result":
            record["resultFile"] = "/synthetic/unrelated.json"
        elif drift == "scope":
            record["evidenceScope"] = "unrelated-scope"
        else:
            record["candidateRuntimeRequired"] = False
        baseline[1] = _line(record)

    # Act / Assert
    with pytest.raises(ValueError):
        _transform(inventory, baseline, cases, "candidate")


@pytest.mark.parametrize("drift", ["failed", "error", "retry", "missing-case"])
def test_rejects_incomplete_green_assertion(
    monkeypatch: pytest.MonkeyPatch, drift: str
) -> None:
    # Arrange
    inventory, baseline, cases = _fixture(monkeypatch, "candidate")
    first = ORIGINAL_TARGETS[31_232][0]
    if drift == "failed":
        cases[first]["attempts"][0]["status"] = "failed"
    elif drift == "error":
        cases[first]["errors"] = ["official failure"]
    elif drift == "retry":
        cases[first]["attempts"][0]["retry"] = 1
    else:
        del cases[first]

    # Act / Assert
    with pytest.raises(ValueError):
        _transform(inventory, baseline, cases, "candidate")


def test_sealed_evidence_rejects_mutation_and_symlink(tmp_path: Path) -> None:
    # Arrange
    evidence = tmp_path / "evidence.json"
    evidence.write_bytes(b"evidence\n")
    expected = promoter.sha256(evidence)

    # Act / Assert
    promoter.sealed(evidence, expected)
    evidence.write_bytes(b"drifted\n")
    with pytest.raises(ValueError, match="SHA-256 drift"):
        promoter.sealed(evidence, expected)
    evidence.unlink()
    evidence.symlink_to(tmp_path / "other")
    with pytest.raises(ValueError, match="missing or linked evidence"):
        promoter.sealed(evidence, expected)


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
