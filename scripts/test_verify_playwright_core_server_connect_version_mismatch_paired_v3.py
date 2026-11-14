"""Mutation controls for the independent one-row Core v3 successor verifier."""

from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path
from typing import Any

import pytest
import verify_playwright_core_server_connect_version_mismatch_paired_v3 as verifier


def encoded(value: dict[str, Any], *, spaced: bool = False) -> bytes:
    separators = None if spaced else (",", ":")
    return (json.dumps(value, sort_keys=True, separators=separators) + "\n").encode()


def source(index: int) -> dict[str, Any]:
    if index == 1:
        return copy.deepcopy(verifier.SOURCE)
    return {
        "specId": f"other-{index}",
        "sourceClassification": "public-api-candidate",
        "sourceFile": f"tests/library/other-{index}.spec.ts",
        "suite": "library",
        "projectId": "chromium-library",
        "projectName": "chromium-library",
    }


def fixture(
    lane: str,
) -> tuple[list[bytes], list[bytes], list[bytes]]:
    inventory: list[bytes] = []
    predecessor: list[bytes] = []
    successor: list[bytes] = []
    for index in range(3):
        item = source(index)
        prior = {
            **item,
            "attempts": 1 if index == 0 else 0,
            "outcome": "expected" if index == 0 else "not-run",
        }
        if index == 0:
            prior["evidenceScope"] = "earlier-evidence"
        inventory.append(encoded(item))
        predecessor.append(encoded(prior, spaced=index == 2))
        if index != 1:
            successor.append(predecessor[-1])
        else:
            successor.append(
                encoded(
                    {
                        **prior,
                        "attempts": 1,
                        "candidateRuntimeRequired": lane == "candidate",
                        "evidenceScope": verifier.SCOPE,
                        "outcome": "expected",
                        "resultFile": f"/synthetic/{lane}.json",
                    }
                )
            )
    return inventory, predecessor, successor


def check(
    lane: str,
    rows: tuple[list[bytes], list[bytes], list[bytes]],
) -> tuple[dict[str, int], dict[str, int], dict[str, str]]:
    return verifier.verify_successor_lines(
        *rows,
        lane=lane,
        result_file=f"/synthetic/{lane}.json",
        prior_outcomes={"expected": 1, "not-run": 2},
        prior_scopes={"earlier-evidence": 1},
        expected_count=3,
        target_index=1,
    )


@pytest.mark.parametrize("lane", verifier.LANES)
def test_only_exact_one_row_is_promoted(lane: str) -> None:
    # Arrange
    rows = fixture(lane)

    # Act
    outcomes, scopes, row_hashes = check(lane, rows)

    # Assert
    assert outcomes == {"expected": 2, "not-run": 1}
    assert scopes == {"earlier-evidence": 1, verifier.SCOPE: 1}
    assert rows[1][0] == rows[2][0] and rows[1][2] == rows[2][2]
    assert row_hashes == {
        "oldRowSha256": hashlib.sha256(rows[1][1]).hexdigest(),
        "newRowSha256": hashlib.sha256(rows[2][1]).hexdigest(),
    }


@pytest.mark.parametrize(
    "field,value",
    [
        ("attempts", 2),
        ("outcome", "skipped"),
        ("candidateRuntimeRequired", False),
        ("evidenceScope", "public-api-parity"),
        ("resultFile", "/synthetic/unrelated.json"),
        ("sourceClassification", "public-api-candidate"),
        ("classificationReasons", []),
        ("publicApiParityEvidence", True),
        ("browserExecutionEvidence", True),
    ],
)
def test_rejects_target_tampering(field: str, value: object) -> None:
    # Arrange
    rows = fixture("candidate")
    changed = json.loads(rows[2][1])
    changed[field] = value
    rows[2][1] = encoded(changed)

    # Act / Assert
    with pytest.raises(ValueError, match="target row"):
        check("candidate", rows)


@pytest.mark.parametrize("mutation", ["spacing", "outcome", "spec-id"])
def test_rejects_non_target_byte_or_identity_changes(mutation: str) -> None:
    # Arrange
    rows = fixture("candidate")
    item = json.loads(rows[2][0])
    if mutation == "spacing":
        rows[2][0] = encoded(item, spaced=True)
    else:
        item["outcome" if mutation == "outcome" else "specId"] = (
            "skipped" if mutation == "outcome" else verifier.TARGET_ID
        )
        rows[2][0] = encoded(item)

    # Act / Assert
    with pytest.raises(ValueError, match="non-target row"):
        check("candidate", rows)


@pytest.mark.parametrize("mutation", ["missing", "extra", "unterminated"])
def test_rejects_missing_extra_or_unterminated_rows(mutation: str) -> None:
    # Arrange
    rows = fixture("candidate")
    if mutation == "missing":
        rows[2].pop()
    elif mutation == "extra":
        rows[2].append(rows[2][-1])
    else:
        rows[2][0] = rows[2][0].rstrip(b"\n")

    # Act / Assert
    with pytest.raises(ValueError, match="row length|total rows|unterminated"):
        check("candidate", rows)


def test_rejects_inventory_source_drift_at_target() -> None:
    # Arrange
    rows = fixture("candidate")
    item = json.loads(rows[0][1])
    item["line"] = 1045
    rows[0][1] = encoded(item)

    # Act / Assert
    with pytest.raises(ValueError, match="predecessor source or outcome"):
        check("candidate", rows)


def test_rejects_predecessor_outcome_summary_drift() -> None:
    # Arrange
    rows = fixture("candidate")

    # Act / Assert
    with pytest.raises(ValueError, match="predecessor counts"):
        verifier.verify_successor_lines(
            *rows,
            lane="candidate",
            result_file="/synthetic/candidate.json",
            prior_outcomes={"expected": 2, "not-run": 1},
            prior_scopes={"earlier-evidence": 1},
            expected_count=3,
            target_index=1,
        )


def clean_report() -> dict[str, Any]:
    return {
        "errors": [],
        "stats": {"expected": 1, "unexpected": 0, "skipped": 0, "flaky": 0},
        "suites": [
            {
                "specs": [
                    {
                        "id": verifier.TARGET_ID,
                        "title": verifier.SOURCE["titlePath"][1],
                        "line": 1046,
                        "column": 1,
                        "ok": True,
                        "tests": [
                            {
                                "projectId": "chromium-library",
                                "expectedStatus": "passed",
                                "status": "expected",
                                "results": [
                                    {"retry": 0, "status": "passed", "errors": []}
                                ],
                            }
                        ],
                    }
                ],
                "suites": [],
            }
        ],
    }


def test_accepts_one_clean_original_official_assertion() -> None:
    # Arrange
    report = clean_report()

    # Act
    verifier.selected_official_report(report)

    # Assert
    assert report["stats"]["expected"] == 1


@pytest.mark.parametrize(
    "mutation",
    ["global-error", "retry", "test-error", "wrong-spec", "extra-spec", "wrong-stats"],
)
def test_rejects_weakened_or_ambiguous_official_report(mutation: str) -> None:
    # Arrange
    report = clean_report()
    spec = report["suites"][0]["specs"][0]
    attempt = spec["tests"][0]["results"][0]
    if mutation == "global-error":
        report["errors"] = ["failed setup"]
    elif mutation == "retry":
        attempt["retry"] = 1
    elif mutation == "test-error":
        attempt["errors"] = ["unexpected diagnostic"]
    elif mutation == "wrong-spec":
        spec["id"] = "other"
    elif mutation == "extra-spec":
        report["suites"][0]["specs"].append(copy.deepcopy(spec))
    else:
        report["stats"]["unexpected"] = 1

    # Act / Assert
    with pytest.raises(ValueError, match="official"):
        verifier.selected_official_report(report)


def envelope(lane: str) -> dict[str, Any]:
    return {
        "mode": lane,
        "harnessInputs": [
            {"path": str(verifier.ROOT / "scripts/synthetic.py"), "sha256": "a" * 64}
        ],
        "rawProvenance": f"/synthetic/{lane}.provenance.json",
        "rawProvenanceSha256": "b" * 64,
        "ownerReceipt": "/synthetic/candidate.owner.jsonl"
        if lane == "candidate"
        else None,
        "ownerReceiptSha256": "c" * 64 if lane == "candidate" else None,
        "result": f"/synthetic/{lane}.json",
        "resultSha256": "d" * 64,
        "snapshot": {"addonSha256": "e" * 64},
    }


def receipt_header() -> dict[str, Any]:
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
        "selectedOfficialTestStartsRemoteServer": True,
        "upstreamAssertionsModified": False,
        "captureSidecarPromotedFlagAtCapture": False,
        "promoterSourceSha256": "f" * 64,
        "pairedVerifierSha256": "1" * 64,
        "harnessInputs": {"scripts/synthetic.py": "a" * 64},
        "priorBundle": str(verifier.PREDECESSOR),
        "priorBundleReceiptSha256": verifier.PREDECESSOR_RECEIPT_SHA256,
        "inventory": str(verifier.INVENTORY),
        "inventorySha256": verifier.INVENTORY_SHA256,
        "recordsPerLane": verifier.COUNT,
        "promotedRowsPerLane": 1,
        "targetZeroBasedIndexes": [verifier.TARGET_INDEX],
        "targetSpecIds": [verifier.TARGET_ID],
        "lanes": {"candidate": {}, "oracle": {}},
    }


def check_receipt_header(receipt: dict[str, Any]) -> None:
    verifier.verify_receipt_header(
        receipt,
        {lane: envelope(lane) for lane in verifier.LANES},
        promoter_sha256="f" * 64,
        verifier_sha256="1" * 64,
    )


def test_accepts_exact_internal_diagnostic_receipt_header() -> None:
    # Arrange
    receipt = receipt_header()

    # Act
    check_receipt_header(receipt)

    # Assert
    assert receipt["publicApiParityEvidence"] is False


@pytest.mark.parametrize(
    "field,value",
    [
        ("fixtureAdapted", False),
        ("publicApiParityEvidence", True),
        ("browserExecutionEvidence", True),
        ("sourceClassificationPreserved", "public-api-candidate"),
        ("diagnosticOnly", False),
        ("selectedOfficialTestStartsRemoteServer", False),
        ("upstreamAssertionsModified", True),
        ("promotedRowsPerLane", 2),
        ("targetZeroBasedIndexes", [31_533]),
        ("pairedVerifierSha256", "2" * 64),
        ("harnessInputs", {"scripts/synthetic.py": "0" * 64}),
    ],
)
def test_rejects_receipt_parity_inflation_or_identity_drift(
    field: str, value: object
) -> None:
    # Arrange
    receipt = receipt_header()
    receipt[field] = value

    # Act / Assert
    with pytest.raises(ValueError, match="receipt identity"):
        check_receipt_header(receipt)


def test_rejects_receipt_extra_field_and_missing_lane() -> None:
    # Arrange
    receipt = receipt_header()
    receipt["publicPass"] = True

    # Act / Assert
    with pytest.raises(ValueError, match="field set"):
        check_receipt_header(receipt)
    receipt.pop("publicPass")
    receipt["lanes"].pop("oracle")
    with pytest.raises(ValueError, match="lane set"):
        check_receipt_header(receipt)


def lane_receipt(lane: str, evidence: dict[str, Any]) -> dict[str, Any]:
    return {
        "base": str(verifier.PREDECESSOR / f"{lane}.jsonl"),
        "baseSha256": verifier.PREDECESSOR_LEDGER_SHA256[lane],
        "baseSummarySha256": verifier.PREDECESSOR_SUMMARY_SHA256[lane],
        "strictEnvelope": str(
            verifier.EVIDENCE / f"{lane}.runserver-url-owner-v3-attestation.json"
        ),
        "strictEnvelopeSha256": verifier.ENVELOPE_SHA256[lane],
        "rawProvenance": evidence["rawProvenance"],
        "rawProvenanceSha256": evidence["rawProvenanceSha256"],
        "ownerReceipt": evidence["ownerReceipt"],
        "ownerReceiptSha256": evidence["ownerReceiptSha256"],
        "officialResult": evidence["result"],
        "officialResultSha256": evidence["resultSha256"],
        "candidateArtifactSha256": (
            evidence["snapshot"]["addonSha256"] if lane == "candidate" else None
        ),
        "ledger": str(verifier.SUCCESSOR / f"{lane}.jsonl"),
        "ledgerSha256": "3" * 64,
        "summarySha256": "4" * 64,
        "oldOutcome": "not-run",
        "newOutcome": "expected",
        "promotedRows": 1,
        "oldRowSha256": "5" * 64,
        "newRowSha256": "6" * 64,
        "outcomes": {"expected": 2, "not-run": 1},
    }


def check_lane_receipt(
    lane: str, item: dict[str, Any], evidence: dict[str, Any]
) -> None:
    verifier.verify_lane_receipt(
        lane,
        item,
        evidence,
        ledger_sha256="3" * 64,
        summary_sha256="4" * 64,
        row_hashes={"oldRowSha256": "5" * 64, "newRowSha256": "6" * 64},
        outcomes={"expected": 2, "not-run": 1},
    )


def test_accepts_exact_candidate_and_oracle_lane_receipts() -> None:
    # Arrange
    candidate = envelope("candidate")
    oracle = envelope("oracle")

    # Act
    check_lane_receipt("candidate", lane_receipt("candidate", candidate), candidate)
    check_lane_receipt("oracle", lane_receipt("oracle", oracle), oracle)

    # Assert
    assert candidate["ownerReceipt"] is not None and oracle["ownerReceipt"] is None


@pytest.mark.parametrize(
    "field,value",
    [
        ("rawProvenance", "/synthetic/oracle.provenance.json"),
        ("rawProvenanceSha256", "0" * 64),
        ("strictEnvelope", "/synthetic/oracle.attestation.json"),
        ("officialResult", "/synthetic/oracle.json"),
        ("candidateArtifactSha256", None),
        ("newRowSha256", "0" * 64),
        ("outcomes", {"expected": 3, "not-run": 0}),
    ],
)
def test_rejects_lane_evidence_swap_or_summary_tamper(
    field: str, value: object
) -> None:
    # Arrange
    candidate = envelope("candidate")
    item = lane_receipt("candidate", candidate)
    item[field] = value

    # Act / Assert
    with pytest.raises(ValueError, match="receipt does not match"):
        check_lane_receipt("candidate", item, candidate)


def test_rejects_swapped_lane_envelope_even_if_receipt_rewritten() -> None:
    # Arrange
    oracle = envelope("oracle")
    item = lane_receipt("candidate", oracle)

    # Act / Assert
    with pytest.raises(ValueError, match="evidence lane"):
        check_lane_receipt("candidate", item, oracle)


def test_rejects_changed_predecessor_and_summary_bytes(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    inventory = tmp_path / "inventory.jsonl"
    inventory.write_bytes(b"sealed inventory\n")
    predecessor = tmp_path / "prior"
    predecessor.mkdir()
    ledgers: dict[str, str] = {}
    summaries: dict[str, str] = {}
    receipt_lanes: dict[str, dict[str, Any]] = {}
    for lane in verifier.LANES:
        ledger = predecessor / f"{lane}.jsonl"
        ledger.write_bytes(f"{lane} ledger\n".encode())
        ledgers[lane] = verifier.sha_file(ledger)
        summary_file = predecessor / f"{lane}.summary.json"
        summary_file.write_text(
            json.dumps(
                {
                    "schema": 1,
                    "inventory": str(inventory),
                    "records": verifier.COUNT,
                    "ledger": str(ledger),
                    "ledgerSha256": ledgers[lane],
                    "outcomes": {"not-run": verifier.COUNT},
                    "explicitEvidenceScopes": {},
                }
            ),
            encoding="utf-8",
        )
        summaries[lane] = verifier.sha_file(summary_file)
        receipt_lanes[lane] = {
            "ledger": str(ledger),
            "ledgerSha256": ledgers[lane],
            "summarySha256": summaries[lane],
            "outcomes": {"not-run": verifier.COUNT},
        }
    receipt_file = predecessor / "promotion-receipt.json"
    receipt_file.write_text(
        json.dumps(
            {
                "schema": 1,
                "inventory": str(inventory),
                "inventorySha256": verifier.sha_file(inventory),
                "recordsPerLane": verifier.COUNT,
                "publicApiParityEvidence": False,
                "lanes": receipt_lanes,
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.setattr(verifier, "INVENTORY", inventory)
    monkeypatch.setattr(verifier, "PREDECESSOR", predecessor)
    monkeypatch.setattr(verifier, "INVENTORY_SHA256", verifier.sha_file(inventory))
    monkeypatch.setattr(
        verifier, "PREDECESSOR_RECEIPT_SHA256", verifier.sha_file(receipt_file)
    )
    monkeypatch.setattr(verifier, "PREDECESSOR_LEDGER_SHA256", ledgers)
    monkeypatch.setattr(verifier, "PREDECESSOR_SUMMARY_SHA256", summaries)
    assert len(verifier.verify_predecessor()) == 2

    # Act / Assert
    (predecessor / "candidate.jsonl").write_bytes(b"tampered\n")
    with pytest.raises(ValueError, match="predecessor ledger SHA-256"):
        verifier.verify_predecessor()
    (predecessor / "candidate.jsonl").write_bytes(b"candidate ledger\n")
    (predecessor / "oracle.summary.json").write_text(
        json.dumps({"outcomes": {"expected": verifier.COUNT}}), encoding="utf-8"
    )
    with pytest.raises(ValueError, match="predecessor summary SHA-256"):
        verifier.verify_predecessor()
