"""Pure mutation controls for the independent six-row HTTP-401 successor verifier."""

from __future__ import annotations

import json
from copy import deepcopy

import pytest
import verify_playwright_core_server_connect_http_401_bidi_chromium_paired_v7 as verifier


def encoded(record: dict) -> bytes:
    return (json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n").encode()


@pytest.fixture
def six_rows():
    targets = {index: value for index, value in enumerate(verifier.TARGETS.values())}
    source = [verifier.target_source(*value) for value in targets.values()]
    old = [{**record, "attempts": 0, "outcome": "not-run"} for record in source]
    results = {record["specId"]: f"/official/{record['specId']}.json" for record in source}
    new = [
        {
            **record,
            "attempts": 1,
            "candidateRuntimeRequired": True,
            "evidenceScope": verifier.SCOPE,
            "outcome": "expected",
            "resultFile": results[record["specId"]],
        }
        for record in old
    ]
    return targets, source, old, new, results


def check_rows(
    six_rows,
    *,
    source=None,
    old=None,
    new=None,
    targets=None,
    lane="candidate",
    results=None,
    prior_outcomes=None,
    prior_scopes=None,
):
    base_targets, base_source, base_old, base_new, base_results = six_rows
    source = base_source if source is None else source
    old = base_old if old is None else old
    new = base_new if new is None else new
    return verifier.verify_successor_lines(
        [encoded(item) for item in source],
        [encoded(item) for item in old],
        [encoded(item) for item in new],
        lane=lane,
        results_by_id=base_results if results is None else results,
        prior_outcomes={"not-run": len(old)} if prior_outcomes is None else prior_outcomes,
        prior_scopes={} if prior_scopes is None else prior_scopes,
        expected_count=len(source),
        targets=base_targets if targets is None else targets,
    )


def test_exact_six_candidate_transitions_accepted(six_rows) -> None:
    # Arrange
    expected_indexes = list(range(6))

    # Act
    outcomes, scopes, changed = check_rows(six_rows)

    # Assert
    assert outcomes == {"expected": 6}
    assert scopes == {verifier.SCOPE: 6}
    assert [item["index"] for item in changed] == expected_indexes


def test_exact_six_oracle_transitions_accepted(six_rows) -> None:
    # Arrange
    oracle = deepcopy(six_rows[3])
    for row in oracle:
        row["candidateRuntimeRequired"] = False

    # Act
    outcomes, scopes, changed = check_rows(six_rows, new=oracle, lane="oracle")

    # Assert
    assert outcomes == {"expected": 6}
    assert scopes == {verifier.SCOPE: 6}
    assert len(changed) == 6


@pytest.mark.parametrize(
    ("index", "field", "replacement"),
    [
        (0, "specId", "wrong-id"),
        (0, "sourceClassification", "public-api"),
        (0, "classificationReasons", ["private-client-access"]),
        (0, "projectId", "chromium-library"),
        (0, "projectName", "chromium-library"),
        (0, "sourceFile", "wrong-source.ts"),
        (0, "expectedStatus", "failed"),
        (0, "line", 130),
        (0, "column", 1),
        (0, "titlePath", ["wrong", "should print HTTP error"]),
        (5, "titlePath", ["library/browsertype-connect.spec.ts", "wrong"]),
    ],
)
def test_target_source_identity_mutations_rejected(
    six_rows, index: int, field: str, replacement: object
) -> None:
    # Arrange
    source, old, new = deepcopy(six_rows[1:4])
    for record in (source[index], old[index], new[index]):
        record[field] = replacement

    # Act / Assert
    with pytest.raises(ValueError, match="target identity/classification"):
        check_rows(six_rows, source=source, old=old, new=new)


@pytest.mark.parametrize(
    ("field", "replacement"),
    [
        ("attempts", 2),
        ("outcome", "unexpected"),
        ("candidateRuntimeRequired", False),
        ("evidenceScope", "wrong-scope"),
        ("resultFile", "/official/wrong.json"),
        ("sourceClassification", "public-api"),
    ],
)
def test_successor_delta_mutations_rejected(six_rows, field: str, replacement: object) -> None:
    # Arrange
    changed = deepcopy(six_rows[3])
    changed[0][field] = replacement

    # Act / Assert
    with pytest.raises(ValueError, match="exact allowed delta"):
        check_rows(six_rows, new=changed)


def test_wrong_target_position_rejected(six_rows) -> None:
    # Arrange
    targets = dict(six_rows[0])
    targets[0], targets[1] = targets[1], targets[0]

    # Act / Assert
    with pytest.raises(ValueError, match="target identity/classification"):
        check_rows(six_rows, targets=targets)


def test_predecessor_target_must_be_not_run_zero_attempts(six_rows) -> None:
    # Arrange
    old = deepcopy(six_rows[2])
    old[0]["outcome"] = "unexpected"

    # Act / Assert
    with pytest.raises(ValueError, match="target identity/classification"):
        check_rows(six_rows, old=old)


def test_non_target_raw_byte_change_rejected(six_rows) -> None:
    # Arrange
    targets, source, old, new, results = six_rows
    extra = {"specId": "unrelated-case", "suite": "other"}
    source_raw = [encoded(item) for item in [*source, extra]]
    old_raw = [encoded(item) for item in old] + [
        encoded({**extra, "attempts": 0, "outcome": "not-run"})
    ]
    changed_raw = [encoded(item) for item in new] + [
        b'{"suite":"other","specId":"unrelated-case","attempts":0,"outcome":"not-run"}\n'
    ]

    # Act / Assert
    with pytest.raises(ValueError, match="non-target row"):
        verifier.verify_successor_lines(
            source_raw,
            old_raw,
            changed_raw,
            lane="candidate",
            results_by_id=results,
            prior_outcomes={"not-run": 7},
            prior_scopes={},
            expected_count=7,
            targets=targets,
        )


def test_missing_target_row_rejected(six_rows) -> None:
    # Arrange
    targets, source, old, new, results = six_rows

    # Act / Assert
    with pytest.raises(ValueError, match="row count differs"):
        verifier.verify_successor_lines(
            [encoded(item) for item in source],
            [encoded(item) for item in old],
            [encoded(item) for item in new[:-1]],
            lane="candidate",
            results_by_id=results,
            prior_outcomes={"not-run": 6},
            prior_scopes={},
            expected_count=6,
            targets=targets,
        )


def test_predecessor_count_mutation_rejected(six_rows) -> None:
    # Arrange
    wrong_counts = {"not-run": 5, "expected": 1}

    # Act / Assert
    with pytest.raises(ValueError, match="predecessor counts"):
        check_rows(six_rows, prior_outcomes=wrong_counts)


def test_wrong_result_mapping_rejected(six_rows) -> None:
    # Arrange
    results = dict(six_rows[4])
    first = next(iter(results))
    results[first] = "/official/other.json"

    # Act / Assert
    with pytest.raises(ValueError, match="exact allowed delta"):
        check_rows(six_rows, results=results)


@pytest.fixture
def receipt_header():
    strict = {
        "candidate": {"inputs": {"source": "sealed"}},
        "oracle": {"inputs": {"source": "sealed"}},
    }
    receipt = {**verifier.expected_header(strict), "lanes": {"candidate": {}, "oracle": {}}}
    return strict, receipt


def test_exact_diagnostic_receipt_header_accepted(receipt_header) -> None:
    # Arrange
    strict, receipt = receipt_header

    # Act
    verifier.verify_receipt_header(receipt, strict)

    # Assert
    assert receipt["sourceClassificationPreserved"] == "implementation-internal"
    assert receipt["promotedRowsPerLane"] == 6
    assert receipt["receiptAttribution"] == "worker-count-only"
    assert receipt["publicApiParityEvidence"] is False


@pytest.mark.parametrize(
    ("field", "replacement"),
    [
        ("publicApiParityEvidence", True),
        ("browserExecutionEvidence", True),
        ("bidiProtocolParityEvidence", True),
        ("currentLivePackageEvidence", True),
        ("fixtureAdapted", False),
        ("fixtureAdaptation", "unadapted"),
        ("receiptAttribution", "per-case"),
        ("diagnosticOnly", False),
        ("frozenCandidateSnapshotAttested", False),
        ("bidiProjectNamesOnly", False),
        ("selectedOfficialTestRequiresBrowser", True),
        ("selectedOfficialTestStartsRemoteServer", True),
        ("upstreamAssertionsModified", True),
        ("captureSidecarPromotedFlagAtCapture", True),
        ("sourceClassificationPreserved", "public-api"),
        ("promotedRowsPerLane", 7),
        ("targetZeroBasedIndexes", [5336, 5337, 5338, 5339, 5340, 5342]),
        ("promoterSourceSha256", "0" * 64),
        ("pairedVerifierSha256", "0" * 64),
        ("promotionMutationTestSha256", "0" * 64),
    ],
)
def test_receipt_header_mutations_rejected(receipt_header, field: str, replacement: object) -> None:
    # Arrange
    strict, receipt = deepcopy(receipt_header)
    receipt[field] = replacement

    # Act / Assert
    with pytest.raises(ValueError, match="receipt header"):
        verifier.verify_receipt_header(receipt, strict)


def test_receipt_extra_field_rejected(receipt_header) -> None:
    # Arrange
    strict, receipt = deepcopy(receipt_header)
    receipt["parityEvidence"] = True

    # Act / Assert
    with pytest.raises(ValueError, match="receipt field set"):
        verifier.verify_receipt_header(receipt, strict)


def test_receipt_missing_oracle_lane_rejected(receipt_header) -> None:
    # Arrange
    strict, receipt = deepcopy(receipt_header)
    del receipt["lanes"]["oracle"]

    # Act / Assert
    with pytest.raises(ValueError, match="receipt header"):
        verifier.verify_receipt_header(receipt, strict)
