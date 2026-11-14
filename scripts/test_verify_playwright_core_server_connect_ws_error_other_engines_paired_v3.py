"""Mutation controls for the independent four-row Core ledger verifier."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest
import verify_playwright_core_server_connect_ws_error_other_engines_paired_v3 as verifier


def _line(value: dict[str, Any], *, spaced: bool = False) -> bytes:
    separators = None if spaced else (",", ":")
    return (json.dumps(value, sort_keys=True, separators=separators) + "\n").encode()


def _source(index: int, target: tuple[str, str, str] | None) -> dict[str, Any]:
    if target is None:
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
    spec_id, project, kind = target
    file = "library/browsertype-connect.spec.ts"
    return {
        "specId": spec_id,
        "projectId": project,
        "projectName": project,
        "sourceClassification": "implementation-internal",
        "classificationReasons": verifier.CLASSIFICATION_REASONS,
        "sourceFile": f"tests/{file}",
        "file": file,
        "line": 122,
        "column": 5,
        "expectedStatus": "passed",
        "role": "official-entrypoint",
        "suite": "library",
        "titlePath": [file, kind, "should print ws error"],
    }


def _fixture(
    lane: str,
) -> tuple[
    list[bytes],
    list[bytes],
    list[bytes],
    dict[str, str],
    dict[int, tuple[str, str, str]],
]:
    targets = {
        index: value for index, value in enumerate(verifier.TARGETS.values(), start=1)
    }
    results = {
        spec_id: f"/synthetic/{lane}/{project}.json"
        for spec_id, project, _ in targets.values()
    }
    inventory: list[bytes] = []
    predecessor: list[bytes] = []
    successor: list[bytes] = []
    for index in range(6):
        target = targets.get(index)
        source = _source(index, target)
        prior = {
            **source,
            "attempts": 0 if index else 1,
            "outcome": "not-run" if index else "expected",
        }
        if index == 0:
            prior["evidenceScope"] = "legacy-scope"
        inventory.append(_line(source))
        predecessor.append(_line(prior, spaced=index in {0, 5}))
        if target is None:
            successor.append(predecessor[-1])
        else:
            expected = {
                **prior,
                "attempts": 1,
                "outcome": "expected",
                "candidateRuntimeRequired": lane == "candidate",
                "evidenceScope": verifier.SCOPE,
                "resultFile": results[target[0]],
            }
            successor.append(_line(expected))
    return inventory, predecessor, successor, results, targets


def _check(
    lane: str,
    fixture: tuple[
        list[bytes],
        list[bytes],
        list[bytes],
        dict[str, str],
        dict[int, tuple[str, str, str]],
    ],
    *,
    prior_outcomes: dict[str, int] | None = None,
) -> tuple[dict[str, int], dict[str, int]]:
    inventory, predecessor, successor, results, targets = fixture
    return verifier.verify_successor_lines(
        inventory,
        predecessor,
        successor,
        lane=lane,
        result_by_spec=results,
        prior_outcomes=prior_outcomes or {"expected": 1, "not-run": 5},
        prior_scopes={"legacy-scope": 1},
        expected_count=6,
        targets=targets,
    )


@pytest.mark.parametrize("lane", ["candidate", "oracle"])
def test_accepts_only_four_exact_target_deltas_and_preserves_other_bytes(
    lane: str,
) -> None:
    # Arrange
    fixture = _fixture(lane)
    _, predecessor, successor, _, _ = fixture

    # Act
    outcomes, scopes = _check(lane, fixture)

    # Assert
    assert [
        index
        for index, (old, new) in enumerate(zip(predecessor, successor))
        if old != new
    ] == [
        1,
        2,
        3,
        4,
    ]
    assert successor[0] == predecessor[0] and successor[5] == predecessor[5]
    assert outcomes == {"expected": 5, "not-run": 1}
    assert scopes == {"legacy-scope": 1, verifier.SCOPE: 4}


@pytest.mark.parametrize(
    "field,value",
    [
        ("attempts", 2),
        ("outcome", "skipped"),
        ("candidateRuntimeRequired", False),
        ("sourceClassification", "public-api-candidate"),
        ("classificationReasons", []),
        ("resultFile", "/synthetic/wrong.json"),
        ("evidenceScope", "public-api-parity"),
        ("publicApiParityEvidence", True),
    ],
)
def test_rejects_any_target_delta_beyond_the_exact_diagnostic_fields(
    field: str, value: object
) -> None:
    # Arrange
    fixture = _fixture("candidate")
    successor = fixture[2]
    record = json.loads(successor[1])
    record[field] = value
    successor[1] = _line(record)

    # Act / Assert
    with pytest.raises(ValueError, match="target row"):
        _check("candidate", fixture)


@pytest.mark.parametrize("mutation", ["spacing", "outcome", "scope", "spec-id"])
def test_rejects_non_target_changes_even_when_semantics_or_counts_appear_same(
    mutation: str,
) -> None:
    # Arrange
    fixture = _fixture("candidate")
    successor = fixture[2]
    record = json.loads(successor[0])
    if mutation == "spacing":
        successor[0] = _line(record, spaced=False)
    elif mutation == "outcome":
        record["outcome"] = "not-run"
        successor[0] = _line(record)
    elif mutation == "scope":
        record["evidenceScope"] = verifier.SCOPE
        successor[0] = _line(record)
    else:
        record["specId"] = fixture[4][1][0]
        successor[0] = _line(record)

    # Act / Assert
    with pytest.raises(ValueError, match="non-target row"):
        _check("candidate", fixture)


@pytest.mark.parametrize(
    "mutation", ["outcome", "classification", "extra-field", "wrong-id"]
)
def test_rejects_changed_predecessor_target_identity(mutation: str) -> None:
    # Arrange
    fixture = _fixture("oracle")
    prior = fixture[1]
    record = json.loads(prior[1])
    if mutation == "outcome":
        record["outcome"] = "expected"
    elif mutation == "classification":
        record["sourceClassification"] = "public-api-candidate"
    elif mutation == "extra-field":
        record["publicApiParityEvidence"] = True
    else:
        record["specId"] = "other-target"
    prior[1] = _line(record)

    # Act / Assert
    with pytest.raises(ValueError):
        _check("oracle", fixture)


@pytest.mark.parametrize("mutation", ["missing", "extra", "moved", "unterminated"])
def test_rejects_row_count_position_and_termination_drift(mutation: str) -> None:
    # Arrange
    fixture = _fixture("candidate")
    successor = fixture[2]
    if mutation == "missing":
        successor.pop()
    elif mutation == "extra":
        successor.append(successor[0])
    elif mutation == "moved":
        successor[1], successor[2] = successor[2], successor[1]
    else:
        successor[1] = successor[1].rstrip(b"\n")

    # Act / Assert
    with pytest.raises(ValueError):
        _check("candidate", fixture)


def test_rejects_predecessor_summary_outcome_drift() -> None:
    # Arrange
    fixture = _fixture("candidate")

    # Act / Assert
    with pytest.raises(ValueError, match="predecessor summary"):
        _check("candidate", fixture, prior_outcomes={"expected": 2, "not-run": 4})


def test_rejects_duplicate_or_missing_strict_target_result() -> None:
    # Arrange
    fixture = _fixture("candidate")
    fixture[3].pop(fixture[4][1][0])

    # Act / Assert
    with pytest.raises(ValueError, match="strict result IDs"):
        _check("candidate", fixture)


def _strict_fixture() -> dict[str, Any]:
    runs: list[dict[str, Any]] = []
    for project, selection in verifier.capture.PROJECTS.items():
        runs.append(
            {
                "project": project,
                "suite": selection["suite"],
                "result": f"/synthetic/{project}.json",
                "resultSha256": "a" * 64,
                "selected": [
                    {
                        "specId": spec_id,
                        "project": project,
                        "kind": kind,
                        "line": 122,
                        "title": "should print ws error",
                        "status": "expected",
                        "errors": [],
                        "attempts": [
                            {"retry": 0, "status": "passed", "workerIndex": 0}
                        ],
                    }
                    for spec_id, kind in selection["specs"].items()
                ],
            }
        )
    return {
        "inputs": {"scripts/synthetic.py": "a" * 64},
        "runs": runs,
        "candidateArtifactSha256": "b" * 64,
        "candidateSnapshotManifestSha256": "c" * 64,
        "candidateSourceSha256": "d" * 64,
    }


def test_selected_results_rejects_nonpassing_or_retried_official_attempt() -> None:
    # Arrange
    strict = _strict_fixture()
    assert len(verifier.selected_results(strict)) == 4
    strict["runs"][0]["selected"][0]["attempts"][0]["retry"] = 1

    # Act / Assert
    with pytest.raises(ValueError, match="single clean official pass"):
        verifier.selected_results(strict)


def _receipt_header(strict: dict[str, Any]) -> dict[str, Any]:
    return {
        "schema": 1,
        "scope": verifier.SCOPE,
        "promotionKind": "implementation-internal-diagnostic",
        "fixtureAdapted": True,
        "publicApiParityEvidence": False,
        "browserExecutionEvidence": False,
        "sourceClassificationPreserved": "implementation-internal",
        "diagnosticOnly": True,
        "candidateRuntimeAttested": True,
        "selectedOfficialTestRequiresBrowser": False,
        "selectedOfficialTestStartsRemoteServer": False,
        "upstreamAssertionsModified": False,
        "captureSidecarPromotedFlagAtCapture": False,
        "promoterSourceSha256": "a" * 64,
        "strictVerifierSha256": "b" * 64,
        "pairedVerifierSha256": "c" * 64,
        "harnessInputs": strict["inputs"],
        "priorBundle": str(verifier.PREDECESSOR),
        "priorBundleReceiptSha256": verifier.PREDECESSOR_RECEIPT_SHA256,
        "inventory": str(verifier.INVENTORY),
        "inventorySha256": verifier.INVENTORY_SHA256,
        "recordsPerLane": verifier.COUNT,
        "promotedRowsPerLane": 4,
        "targetZeroBasedIndexes": sorted(verifier.TARGETS),
        "targetSpecIds": [
            verifier.TARGETS[index][0] for index in sorted(verifier.TARGETS)
        ],
        "lanes": {"candidate": {}, "oracle": {}},
    }


@pytest.mark.parametrize(
    "field,value",
    [
        ("fixtureAdapted", False),
        ("publicApiParityEvidence", True),
        ("browserExecutionEvidence", True),
        ("sourceClassificationPreserved", "public-api-candidate"),
        ("diagnosticOnly", False),
        ("promotedRowsPerLane", 5),
        ("targetZeroBasedIndexes", [31_234, 31_235, 31_236]),
        ("pairedVerifierSha256", "d" * 64),
    ],
)
def test_receipt_rejects_scope_inflation_or_identity_drift(
    field: str, value: object
) -> None:
    # Arrange
    strict = _strict_fixture()
    receipt = _receipt_header(strict)
    receipt[field] = value

    # Act / Assert
    with pytest.raises(ValueError, match="receipt identity"):
        verifier.verify_receipt_header(
            receipt,
            {"candidate": strict, "oracle": strict},
            promoter_sha="a" * 64,
            paired_verifier_sha="c" * 64,
            strict_verifier_sha="b" * 64,
        )


def test_receipt_rejects_extra_fields_and_lane_set_drift() -> None:
    # Arrange
    strict = _strict_fixture()
    receipt = _receipt_header(strict)
    receipt["publicApiBehaviorAttested"] = True

    # Act / Assert
    with pytest.raises(ValueError, match="field set"):
        verifier.verify_receipt_header(
            receipt,
            {"candidate": strict, "oracle": strict},
            promoter_sha="a" * 64,
            paired_verifier_sha="c" * 64,
            strict_verifier_sha="b" * 64,
        )


def test_sealed_evidence_rejects_mutation_and_symlink(tmp_path: Path) -> None:
    # Arrange
    evidence = tmp_path / "evidence.json"
    evidence.write_bytes(b"original\n")
    expected = verifier.sha_file(evidence)

    # Act / Assert
    verifier.sealed(evidence, expected, "synthetic evidence")
    evidence.write_bytes(b"changed\n")
    with pytest.raises(ValueError, match="SHA-256 drifted"):
        verifier.sealed(evidence, expected, "synthetic evidence")
    evidence.unlink()
    evidence.symlink_to(tmp_path / "other")
    with pytest.raises(ValueError, match="missing or linked"):
        verifier.sealed(evidence, expected, "synthetic evidence")


def test_output_directory_rejects_extra_file_before_reading_any_evidence(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    monkeypatch.setattr(verifier, "OUTPUT", tmp_path)
    for name in (
        "candidate.jsonl",
        "candidate.summary.json",
        "oracle.jsonl",
        "oracle.summary.json",
        "promotion-receipt.json",
        "extra.json",
    ):
        (tmp_path / name).write_bytes(b"")

    # Act / Assert
    with pytest.raises(ValueError, match="missing or extra"):
        verifier.verify_existing()
