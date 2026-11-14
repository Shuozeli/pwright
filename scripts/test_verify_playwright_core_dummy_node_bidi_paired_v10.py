"""Mutation controls for the independent Core BiDi dummy-node v10 replay."""

from __future__ import annotations

import json
from copy import deepcopy
from typing import Any

import pytest
import verify_playwright_core_dummy_node_bidi_paired_v10 as verifier


def encoded(record: dict[str, Any]) -> bytes:
    return (json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n").encode()


@pytest.fixture
def sample() -> tuple[
    dict[int, tuple[str, str, str]],
    dict[tuple[str, str], dict[str, Any]],
    list[dict[str, Any]],
    dict[str, list[dict[str, Any]]],
    dict[str, str],
]:
    targets = dict(enumerate(verifier.TARGETS.values()))
    evidence: dict[tuple[str, str], dict[str, Any]] = {}
    source = []
    old = {lane: [] for lane in verifier.LANES}
    previous_results = {}
    for index, (project, engine, spec_id) in targets.items():
        record = verifier.target_source(project, spec_id)
        source.append(record)
        previous_results[project] = f"/official/v2/{project}.json"
        for lane in verifier.LANES:
            old[lane].append(
                {
                    **record,
                    "attempts": 1,
                    "evidenceScope": verifier.V9_SCOPE,
                    "outcome": "unexpected" if lane == "candidate" else "expected",
                    "publicApiParityEvidence": False,
                    "resultFile": previous_results[project],
                }
            )
            evidence[(lane, project)] = {
                "mode": lane,
                "project": project,
                "engine": engine,
                "index": index,
                "specId": spec_id,
                "testStatus": "expected",
                "result": f"/official/v3/{lane}/{project}.json",
            }
    unrelated = {"specId": "unrelated-case", "suite": "other"}
    source.append(unrelated)
    for lane in verifier.LANES:
        old[lane].append({**unrelated, "attempts": 0, "outcome": "not-run"})
    return targets, evidence, source, old, previous_results


def successor(
    targets: dict[int, tuple[str, str, str]],
    evidence: dict[tuple[str, str], dict[str, Any]],
    old: list[dict[str, Any]],
    lane: str,
) -> list[dict[str, Any]]:
    rows = deepcopy(old)
    for index, (project, _, _) in targets.items():
        rows[index].update(
            {
                "attempts": 2,
                "evidenceScope": verifier.SCOPE,
                "outcome": "expected",
                "resultFile": evidence[(lane, project)]["result"],
            }
        )
    return rows


def replay(
    sample: tuple[
        dict[int, tuple[str, str, str]],
        dict[tuple[str, str], dict[str, Any]],
        list[dict[str, Any]],
        dict[str, list[dict[str, Any]]],
        dict[str, str],
    ],
    *,
    lane: str = "candidate",
    targets: dict[int, tuple[str, str, str]] | None = None,
    evidence: dict[tuple[str, str], dict[str, Any]] | None = None,
    source: list[dict[str, Any]] | None = None,
    old: list[dict[str, Any]] | None = None,
    new: list[dict[str, Any]] | None = None,
    new_raw: list[bytes] | None = None,
    previous_results: dict[str, str] | None = None,
    prior_outcomes: dict[str, int] | None = None,
    prior_scopes: dict[str, int] | None = None,
) -> tuple[dict[str, int], dict[str, int], list[dict[str, Any]]]:
    base_targets, base_evidence, base_source, base_old, base_results = sample
    targets = base_targets if targets is None else targets
    evidence = base_evidence if evidence is None else evidence
    source = base_source if source is None else source
    old = base_old[lane] if old is None else old
    new = successor(targets, evidence, old, lane) if new is None else new
    return verifier.verify_successor_lines(
        [encoded(record) for record in source],
        [encoded(record) for record in old],
        [encoded(record) for record in new] if new_raw is None else new_raw,
        lane=lane,
        evidence=evidence,
        previous_results=base_results if previous_results is None else previous_results,
        prior_outcomes=(
            {"unexpected" if lane == "candidate" else "expected": 6, "not-run": 1}
            if prior_outcomes is None
            else prior_outcomes
        ),
        prior_scopes={verifier.V9_SCOPE: 6} if prior_scopes is None else prior_scopes,
        expected_count=len(source),
        targets=targets,
    )


@pytest.mark.parametrize("lane", ["candidate", "oracle"])
def test_six_rerun_rows_and_one_unrelated_row_are_recounted(sample, lane: str) -> None:
    # Arrange
    expected_old = "unexpected" if lane == "candidate" else "expected"

    # Act
    outcomes, scopes, changed = replay(sample, lane=lane)

    # Assert
    assert outcomes == {"expected": 6, "not-run": 1}
    assert scopes == {verifier.SCOPE: 6}
    assert [row["index"] for row in changed] == list(range(6))
    assert all(row["oldOutcome"] == expected_old for row in changed)
    assert all(row["newOutcome"] == "expected" for row in changed)


@pytest.mark.parametrize(
    ("field", "replacement"),
    [
        ("attempts", 1),
        ("attempts", 3),
        ("outcome", "unexpected"),
        ("evidenceScope", "wrong-scope"),
        ("resultFile", "/official/wrong.json"),
        ("publicApiParityEvidence", True),
        ("sourceClassification", "public-api-proven"),
        ("browserExecutionEvidence", True),
        ("bidiProtocolParityEvidence", True),
    ],
)
def test_target_delta_mutation_is_rejected(
    sample, field: str, replacement: object
) -> None:
    # Arrange
    changed = successor(sample[0], sample[1], sample[3]["candidate"], "candidate")
    changed[0][field] = replacement

    # Act / Assert
    with pytest.raises(ValueError, match="exact allowed delta"):
        replay(sample, new=changed)


@pytest.mark.parametrize(
    ("field", "replacement"),
    [
        ("attempts", 0),
        ("outcome", "expected"),
        ("evidenceScope", "wrong-scope"),
        ("resultFile", "/official/wrong.json"),
        ("publicApiParityEvidence", True),
    ],
)
def test_predecessor_target_mutation_is_rejected(
    sample, field: str, replacement: object
) -> None:
    # Arrange
    old = deepcopy(sample[3]["candidate"])
    old[0][field] = replacement

    # Act / Assert
    with pytest.raises(ValueError, match="target predecessor/sidecar identity"):
        replay(sample, old=old)


@pytest.mark.parametrize(
    ("field", "replacement"),
    [
        ("sourceClassification", "implementation-internal"),
        ("classificationReasons", ["private-client-access"]),
        ("projectId", "wrong-project"),
        ("sourceFile", "wrong.ts"),
        ("line", 75),
        ("suite", "library"),
    ],
)
def test_inventory_identity_mutation_is_rejected(
    sample, field: str, replacement: object
) -> None:
    # Arrange
    source = deepcopy(sample[2])
    old = deepcopy(sample[3]["candidate"])
    source[0][field] = replacement
    old[0][field] = replacement

    # Act / Assert
    with pytest.raises(ValueError, match="target predecessor/sidecar identity"):
        replay(sample, source=source, old=old)


def test_missing_strict_case_is_rejected(sample) -> None:
    # Arrange
    evidence = deepcopy(sample[1])
    evidence.pop(("candidate", sample[0][0][0]))
    valid_successor = successor(
        sample[0], sample[1], sample[3]["candidate"], "candidate"
    )

    # Act / Assert
    with pytest.raises(ValueError, match="incomplete"):
        replay(sample, evidence=evidence, new=valid_successor)


@pytest.mark.parametrize("lane", ["candidate", "oracle"])
@pytest.mark.parametrize("status", ["unexpected", "skipped"])
def test_non_green_v3_status_is_rejected(sample, lane: str, status: str) -> None:
    # Arrange
    evidence = deepcopy(sample[1])
    evidence[(lane, sample[0][0][0])]["testStatus"] = status

    # Act / Assert
    with pytest.raises(ValueError, match="target predecessor/sidecar identity"):
        replay(sample, lane=lane, evidence=evidence)


def test_target_at_wrong_index_is_rejected(sample) -> None:
    # Arrange
    targets = dict(sample[0])
    targets[0], targets[1] = targets[1], targets[0]

    # Act / Assert
    with pytest.raises(ValueError, match="target predecessor/sidecar identity"):
        replay(sample, targets=targets)


def test_non_target_raw_byte_change_is_rejected(sample) -> None:
    # Arrange
    rows = successor(sample[0], sample[1], sample[3]["candidate"], "candidate")
    raw = [encoded(record) for record in rows]
    raw[-1] = (
        b'{"suite":"other","specId":"unrelated-case","attempts":0,"outcome":"not-run"}\n'
    )

    # Act / Assert
    with pytest.raises(ValueError, match="non-target row"):
        replay(sample, new_raw=raw)


def test_missing_row_is_rejected(sample) -> None:
    # Arrange
    rows = successor(sample[0], sample[1], sample[3]["candidate"], "candidate")

    # Act / Assert
    with pytest.raises(ValueError, match="row count differs"):
        replay(sample, new_raw=[encoded(record) for record in rows[:-1]])


@pytest.mark.parametrize(
    ("counts", "scopes"),
    [
        ({"unexpected": 5, "expected": 1, "not-run": 1}, {"v9": 6}),
        ({"unexpected": 6, "not-run": 1}, {"v9": 5}),
    ],
)
def test_predecessor_summary_arithmetic_is_rejected(sample, counts, scopes) -> None:
    # Arrange
    scopes = {verifier.V9_SCOPE: scopes["v9"]}

    # Act / Assert
    with pytest.raises(ValueError, match="outcome/scope arithmetic"):
        replay(sample, prior_outcomes=counts, prior_scopes=scopes)


def test_missing_previous_result_reference_is_rejected(sample) -> None:
    # Arrange
    previous = dict(sample[4])
    previous.pop(sample[0][0][0])

    # Act / Assert
    with pytest.raises(ValueError, match="incomplete"):
        replay(sample, previous_results=previous)


def test_receipt_header_preserves_no_parity_credit(sample, monkeypatch) -> None:
    # Arrange
    monkeypatch.setattr(verifier, "PROMOTER", __import__("pathlib").Path(__file__))
    evidence = sample[1]

    # Act
    header = verifier.expected_header(evidence)

    # Assert
    assert header["sourceClassificationPreserved"] == "public-api-candidate"
    assert header["publicApiParityEvidence"] is False
    assert header["browserExecutionEvidence"] is False
    assert header["bidiProtocolParityEvidence"] is False
    assert header["candidateGreenRows"] == 6
    assert header["candidateRedRows"] == 0
    assert header["unchangedRawRowsPerLane"] == verifier.COUNT - 6


def test_receipt_header_rejects_non_green_v3_status(sample) -> None:
    # Arrange
    evidence = deepcopy(sample[1])
    evidence[("candidate", sample[0][0][0])]["testStatus"] = "unexpected"

    # Act / Assert
    with pytest.raises(ValueError, match="not all expected"):
        verifier.expected_header(evidence)
