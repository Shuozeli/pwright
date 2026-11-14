"""Mutation controls for the independent mixed-outcome Core v9 verifier."""

from __future__ import annotations

import json
from copy import deepcopy
from typing import Any

import pytest
import verify_playwright_core_dummy_node_bidi_paired_v9 as verifier


def encoded(record: dict[str, Any]) -> bytes:
    return (json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n").encode()


@pytest.fixture
def sample() -> tuple[
    dict[int, tuple[str, str, str]],
    dict[tuple[str, str], dict[str, Any]],
    list[dict[str, Any]],
    list[dict[str, Any]],
]:
    targets = dict(enumerate(verifier.TARGETS.values()))
    evidence = {}
    source = []
    old = []
    for index, (project, engine, spec_id) in targets.items():
        record = verifier.target_source(project, spec_id)
        source.append(record)
        old.append({**record, "attempts": 0, "outcome": "not-run"})
        for lane in verifier.LANES:
            evidence[(lane, project)] = {
                "mode": lane,
                "project": project,
                "engine": engine,
                "index": index,
                "specId": spec_id,
                "testStatus": "unexpected" if lane == "candidate" and index % 2 else "expected",
                "result": f"/official/{lane}/{project}.json",
            }
    unrelated = {"specId": "unrelated-case", "suite": "other"}
    source.append(unrelated)
    old.append({**unrelated, "attempts": 0, "outcome": "not-run"})
    return targets, evidence, source, old


def successor(
    targets: dict[int, tuple[str, str, str]],
    evidence: dict[tuple[str, str], dict[str, Any]],
    old: list[dict[str, Any]],
    lane: str,
) -> list[dict[str, Any]]:
    rows = deepcopy(old)
    for index, (project, _, _) in targets.items():
        sidecar = evidence[(lane, project)]
        rows[index].update(
            {
                "attempts": 1,
                "evidenceScope": verifier.SCOPE,
                "outcome": sidecar["testStatus"],
                "publicApiParityEvidence": False,
                "resultFile": sidecar["result"],
            }
        )
    return rows


def replay(
    sample: tuple[
        dict[int, tuple[str, str, str]],
        dict[tuple[str, str], dict[str, Any]],
        list[dict[str, Any]],
        list[dict[str, Any]],
    ],
    *,
    lane: str = "candidate",
    targets: dict[int, tuple[str, str, str]] | None = None,
    evidence: dict[tuple[str, str], dict[str, Any]] | None = None,
    source: list[dict[str, Any]] | None = None,
    old: list[dict[str, Any]] | None = None,
    new: list[dict[str, Any]] | None = None,
    new_raw: list[bytes] | None = None,
    prior_outcomes: dict[str, int] | None = None,
) -> tuple[dict[str, int], dict[str, int], list[dict[str, Any]]]:
    base_targets, base_evidence, base_source, base_old = sample
    targets = base_targets if targets is None else targets
    evidence = base_evidence if evidence is None else evidence
    source = base_source if source is None else source
    old = base_old if old is None else old
    new = successor(targets, evidence, old, lane) if new is None else new
    return verifier.verify_successor_lines(
        [encoded(record) for record in source],
        [encoded(record) for record in old],
        [encoded(record) for record in new] if new_raw is None else new_raw,
        lane=lane,
        evidence=evidence,
        prior_outcomes={"not-run": len(old)} if prior_outcomes is None else prior_outcomes,
        prior_scopes={},
        expected_count=len(source),
        targets=targets,
    )


def test_mixed_candidate_outcomes_are_recorded_without_parity_credit(sample) -> None:
    # Arrange
    expected_indexes = list(range(6))

    # Act
    outcomes, scopes, changed = replay(sample)

    # Assert
    assert outcomes == {"expected": 3, "not-run": 1, "unexpected": 3}
    assert scopes == {verifier.SCOPE: 6}
    assert [row["index"] for row in changed] == expected_indexes
    assert [row["outcome"] for row in changed] == [
        "expected",
        "unexpected",
        "expected",
        "unexpected",
        "expected",
        "unexpected",
    ]


def test_oracle_six_green_rows_are_recorded(sample) -> None:
    # Arrange
    lane = "oracle"

    # Act
    outcomes, scopes, changed = replay(sample, lane=lane)

    # Assert
    assert outcomes == {"expected": 6, "not-run": 1}
    assert scopes == {verifier.SCOPE: 6}
    assert all(row["outcome"] == "expected" for row in changed)


@pytest.mark.parametrize("status", ["expected", "unexpected"])
def test_candidate_outcome_is_derived_from_sidecar(sample, status: str) -> None:
    # Arrange
    targets, original, _, _ = sample
    evidence = deepcopy(original)
    for project, _, _ in targets.values():
        evidence[("candidate", project)]["testStatus"] = status

    # Act
    outcomes, _, changed = replay(sample, evidence=evidence)

    # Assert
    assert outcomes[status] == 6
    assert all(row["outcome"] == status for row in changed)


@pytest.mark.parametrize(
    ("field", "replacement"),
    [
        ("sourceClassification", "implementation-internal"),
        ("classificationReasons", ["private-client-access"]),
        ("projectId", "chromium-library"),
        ("projectName", "chromium-library"),
        ("sourceFile", "wrong.ts"),
        ("line", 75),
        ("column", 5),
        ("expectedStatus", "failed"),
        ("titlePath", ["wrong", "wrong"]),
        ("suite", "library"),
    ],
)
def test_inventory_identity_mutation_is_rejected(sample, field: str, replacement: object) -> None:
    # Arrange
    source = deepcopy(sample[2])
    old = deepcopy(sample[3])
    source[0][field] = replacement
    old[0][field] = replacement
    new = successor(sample[0], sample[1], old, "candidate")

    # Act / Assert
    with pytest.raises(ValueError, match="target identity/classification"):
        replay(sample, source=source, old=old, new=new)


@pytest.mark.parametrize(
    ("field", "replacement"),
    [
        ("attempts", 2),
        ("outcome", "unexpected"),
        ("evidenceScope", "wrong-scope"),
        ("publicApiParityEvidence", True),
        ("resultFile", "/official/wrong.json"),
        ("sourceClassification", "public-api-proven"),
    ],
)
def test_target_delta_mutation_is_rejected(sample, field: str, replacement: object) -> None:
    # Arrange
    changed = successor(sample[0], sample[1], sample[3], "candidate")
    changed[0][field] = replacement

    # Act / Assert
    with pytest.raises(ValueError, match="exact allowed delta"):
        replay(sample, new=changed)


def test_missing_strict_case_is_rejected(sample) -> None:
    # Arrange
    evidence = deepcopy(sample[1])
    evidence.pop(("candidate", sample[0][0][0]))

    # Act / Assert
    with pytest.raises(ValueError, match="incomplete"):
        verifier.verify_successor_lines(
            [],
            [],
            [],
            lane="candidate",
            evidence=evidence,
            prior_outcomes={},
            prior_scopes={},
            expected_count=0,
            targets=sample[0],
        )


def test_oracle_red_is_rejected(sample) -> None:
    # Arrange
    evidence = deepcopy(sample[1])
    project = sample[0][0][0]
    evidence[("oracle", project)]["testStatus"] = "unexpected"

    # Act / Assert
    with pytest.raises(ValueError, match="ineligible"):
        replay(sample, lane="oracle", evidence=evidence)


def test_skipped_candidate_is_rejected(sample) -> None:
    # Arrange
    evidence = deepcopy(sample[1])
    evidence[("candidate", sample[0][0][0])]["testStatus"] = "skipped"

    # Act / Assert
    with pytest.raises(ValueError, match="ineligible"):
        replay(sample, evidence=evidence)


def test_target_at_wrong_index_is_rejected(sample) -> None:
    # Arrange
    targets = dict(sample[0])
    targets[0], targets[1] = targets[1], targets[0]

    # Act / Assert
    with pytest.raises(ValueError, match="target identity/classification"):
        replay(sample, targets=targets, new=successor(sample[0], sample[1], sample[3], "candidate"))


def test_non_target_raw_byte_change_is_rejected(sample) -> None:
    # Arrange
    rows = successor(sample[0], sample[1], sample[3], "candidate")
    raw = [encoded(record) for record in rows]
    raw[-1] = b'{"suite":"other","specId":"unrelated-case","attempts":0,"outcome":"not-run"}\n'

    # Act / Assert
    with pytest.raises(ValueError, match="non-target row"):
        replay(sample, new_raw=raw)


def test_missing_row_is_rejected(sample) -> None:
    # Arrange
    rows = successor(sample[0], sample[1], sample[3], "candidate")

    # Act / Assert
    with pytest.raises(ValueError, match="row count differs"):
        replay(sample, new_raw=[encoded(record) for record in rows[:-1]])


def test_predecessor_target_must_be_unrun(sample) -> None:
    # Arrange
    old = deepcopy(sample[3])
    old[0]["outcome"] = "expected"

    # Act / Assert
    with pytest.raises(ValueError, match="target identity/classification"):
        replay(sample, old=old)


def test_predecessor_counts_are_recounted(sample) -> None:
    # Arrange
    wrong = {"not-run": 6, "expected": 1}

    # Act / Assert
    with pytest.raises(ValueError, match="outcome/scope arithmetic"):
        replay(sample, prior_outcomes=wrong)


def test_receipt_header_keeps_source_label_separate_from_parity(sample) -> None:
    # Arrange
    evidence = sample[1]

    # Act
    header = verifier.expected_header(evidence)

    # Assert
    assert header["sourceClassificationPreserved"] == "public-api-candidate"
    assert header["publicApiParityEvidence"] is False
    assert header["browserExecutionEvidence"] is False
    assert header["bidiProtocolParityEvidence"] is False
    assert header["candidateGreenRows"] == 3
    assert header["candidateRedRows"] == 3


def test_receipt_header_rejects_unknown_candidate_status(sample) -> None:
    # Arrange
    evidence = deepcopy(sample[1])
    evidence[("candidate", sample[0][0][0])]["testStatus"] = "skipped"

    # Act / Assert
    with pytest.raises(ValueError, match="candidate statuses"):
        verifier.expected_header(evidence)
