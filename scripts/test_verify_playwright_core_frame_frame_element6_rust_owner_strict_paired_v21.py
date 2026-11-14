"""Read-only replay and mutation controls for unstable frameElement v21."""

from __future__ import annotations

import json
from pathlib import Path

import promote_playwright_core_frame_frame_element6_rust_owner_strict_paired_v21 as promoter
import pytest
import verify_playwright_core_frame_frame_element6_rust_owner_strict_paired_v21 as verifier


@pytest.fixture(scope="module")
def pair() -> tuple[dict[str, object], dict[str, bytes]]:
    # Arrange
    checked = verifier.verify_inputs()

    # Act
    files = promoter.prepare_pair(checked)
    receipt = verifier.verify_payloads(files, checked)

    # Assert
    assert receipt["unstableSpecIds"] == [verifier.FRAMESET_ID]
    return checked, files


def _mutate_row(
    files: dict[str, bytes], lane: str, index: int, key: str, value: object
) -> dict[str, bytes]:
    changed = dict(files)
    lines = files[f"{lane}.jsonl"].splitlines(keepends=True)
    row = json.loads(lines[index])
    row[key] = value
    lines[index] = verifier.canonical_line(row)
    changed[f"{lane}.jsonl"] = b"".join(lines)
    return changed


def _mutate_receipt(
    files: dict[str, bytes], key: str, value: object
) -> dict[str, bytes]:
    changed = dict(files)
    receipt = json.loads(files["promotion-receipt.json"])
    receipt[key] = value
    changed["promotion-receipt.json"] = (json.dumps(receipt) + "\n").encode()
    return changed


def _target_ids() -> set[str]:
    return {
        row["specId"]
        for raw in verifier.frame_source.INVENTORY.read_text().splitlines()
        if (
            (row := json.loads(raw)).get("suite") == "library"
            and row.get("projectId") == verifier.frame_source.PROJECT
            and row.get("file") == verifier.frame_source.FILE
        )
    }


def _report(run: int) -> dict[str, object]:
    return json.loads(verifier.report_path(run).read_bytes())


def _spec(report: dict[str, object], spec_id: str) -> dict[str, object]:
    return next(spec for spec in report["suites"][0]["specs"] if spec["id"] == spec_id)


def test_six_candidate_rows_and_zero_oracle_bytes_change(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    checked, files = pair

    # Act
    receipt = verifier.verify_payloads(files, checked)

    # Assert
    before = checked["oldLines"]["candidate"]
    after = files["candidate.jsonl"].splitlines(keepends=True)
    assert [
        index
        for index, (left, right) in enumerate(zip(before, after, strict=True))
        if left != right
    ] == sorted(checked["selected"])
    assert files["oracle.jsonl"] == checked["oldFiles"]["oracle.jsonl"]
    assert files["oracle.summary.json"] == checked["oldFiles"]["oracle.summary.json"]
    assert receipt["changedRowsPerLane"] == {"candidate": 6, "oracle": 0}
    assert receipt["unchangedRawRowsPerLane"] == {"candidate": 48_471, "oracle": 48_477}
    assert receipt["ledgerAttemptsPerPromotedSpec"] == 5
    assert receipt["candidateOnlyReruns"] == 4
    assert receipt["publicApiParityEvidence"] is False


def test_first_failure_and_three_green_runs_are_explicit(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    _, files = pair

    # Act
    receipt = json.loads(files["promotion-receipt.json"])

    # Assert
    assert [
        (run["expectedRows"], run["unexpectedRows"]) for run in receipt["runEvidence"]
    ] == [(5, 1), (6, 0), (6, 0), (6, 0)]
    failure = receipt["observedIntermittentNavigationFailure"]
    assert failure["specId"] == verifier.FRAMESET_ID
    assert failure["error"] == verifier.FIRST_FAILURE
    assert failure["officialResultSha256"] == verifier.REPORT_SHA256[1]
    assert failure["provenanceSha256"] == verifier.PROVENANCE_SHA256[1]
    assert failure["beforeFrameElement"] is True
    assert failure["rootCauseEstablished"] is False
    assert receipt["unstableSpecIds"] == [verifier.FRAMESET_ID]
    assert receipt["latestPromotedRun"] == 4


@pytest.mark.parametrize("lane", verifier.LANES)
def test_off_target_raw_row_mutation_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]], lane: str
) -> None:
    # Arrange
    checked, files = pair
    changed = _mutate_row(files, lane, 0, "attempts", 99)

    # Act / Assert
    with pytest.raises(ValueError, match="off-target raw row"):
        verifier.verify_payloads(changed, checked)


@pytest.mark.parametrize(
    ("key", "value"),
    [
        ("attempts", 2),
        ("outcome", "unexpected"),
        ("error", "hidden failure"),
        ("resultFile", "wrong-run"),
        ("evidenceScope", "wrong-scope"),
        ("publicApiParityEvidence", True),
    ],
)
def test_promoted_row_mutation_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]], key: str, value: object
) -> None:
    # Arrange
    checked, files = pair
    index = min(checked["selected"])
    changed = _mutate_row(files, "candidate", index, key, value)

    # Act / Assert
    with pytest.raises(ValueError, match="candidate frame target row"):
        verifier.verify_payloads(changed, checked)


def test_oracle_summary_byte_mutation_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    checked, files = pair
    changed = dict(files)
    changed["oracle.summary.json"] += b"\n"

    # Act / Assert
    with pytest.raises(ValueError, match="Oracle summary bytes changed"):
        verifier.verify_payloads(changed, checked)


def test_candidate_summary_spoof_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    checked, files = pair
    changed = dict(files)
    summary = json.loads(files["candidate.summary.json"])
    summary["outcomes"]["expected"] += 1
    changed["candidate.summary.json"] = (json.dumps(summary) + "\n").encode()

    # Act / Assert
    with pytest.raises(ValueError, match="candidate summary differs"):
        verifier.verify_payloads(changed, checked)


@pytest.mark.parametrize(
    ("key", "value"),
    [
        ("fullCorpusExecution", True),
        ("publicApiParityEvidence", True),
        ("unstableSpecIds", []),
        ("observedIntermittentNavigationFailure", {}),
        ("runEvidence", []),
        ("latestPromotedRun", 3),
        ("ledgerAttemptsPerPromotedSpec", 2),
        ("oracleJsonlAndSummaryByteIdenticalToV20", False),
        ("candidateExpectedRows", 5),
        ("changedRowsPerLane", {"candidate": 5, "oracle": 0}),
    ],
)
def test_receipt_instability_or_scope_mutation_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]], key: str, value: object
) -> None:
    # Arrange
    checked, files = pair
    changed = _mutate_receipt(files, key, value)

    # Act / Assert
    with pytest.raises(ValueError, match="receipt differs"):
        verifier.verify_payloads(changed, checked)


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("beforeFrameElement", False),
        ("rootCauseEstablished", True),
        ("officialResultSha256", "0" * 64),
        ("provenanceSha256", "0" * 64),
    ],
)
def test_receipt_first_failure_detail_mutation_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]], field: str, value: object
) -> None:
    # Arrange
    checked, files = pair
    receipt = json.loads(files["promotion-receipt.json"])
    receipt["observedIntermittentNavigationFailure"][field] = value
    changed = dict(files)
    changed["promotion-receipt.json"] = (json.dumps(receipt) + "\n").encode()

    # Act / Assert
    with pytest.raises(ValueError, match="receipt differs"):
        verifier.verify_payloads(changed, checked)


def test_missing_bundle_member_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    checked, files = pair
    changed = dict(files)
    changed.pop("oracle.summary.json")

    # Act / Assert
    with pytest.raises(ValueError, match="bundle files differ"):
        verifier.verify_payloads(changed, checked)


@pytest.mark.parametrize(
    "mutation",
    [
        "first-failure-hidden",
        "first-failure-moved",
        "first-failure-error",
        "first-failure-no-terminal-error",
        "later-failure",
        "retry",
        "skip",
        "flaky",
        "top-level-error",
        "wrong-project",
        "wrong-id",
    ],
)
def test_four_run_evidence_mutation_is_rejected(mutation: str) -> None:
    # Arrange
    run = 2 if mutation == "later-failure" else 1
    report = _report(run)
    frameset = _spec(report, verifier.FRAMESET_ID)
    test = frameset["tests"][0]
    attempt = test["results"][0]
    if mutation == "first-failure-hidden":
        test["status"] = "expected"
        attempt["status"] = "passed"
        attempt["error"] = None
        attempt["errors"] = []
        report["stats"]["expected"] = 6
        report["stats"]["unexpected"] = 0
    elif mutation == "first-failure-moved":
        test["status"] = "expected"
        attempt["status"] = "passed"
        attempt["error"] = None
        attempt["errors"] = []
        other = next(
            spec
            for spec in report["suites"][0]["specs"]
            if spec["id"] != verifier.FRAMESET_ID
        )
        other_test = other["tests"][0]
        other_test["status"] = "unexpected"
        other_test["results"][0]["status"] = "failed"
        other_test["results"][0]["error"] = {"message": "moved failure"}
        other_test["results"][0]["errors"] = [{"message": "moved failure"}]
    elif mutation == "first-failure-error":
        attempt["error"]["message"] = "a different failure"
    elif mutation == "first-failure-no-terminal-error":
        attempt.pop("error")
    elif mutation == "later-failure":
        test["status"] = "unexpected"
        attempt["status"] = "failed"
        attempt["error"] = {"message": verifier.FIRST_FAILURE}
        attempt["errors"] = [{"message": verifier.FIRST_FAILURE}]
        report["stats"]["expected"] = 5
        report["stats"]["unexpected"] = 1
    elif mutation == "retry":
        attempt["retry"] = 1
    elif mutation == "skip":
        report["stats"]["skipped"] = 1
    elif mutation == "flaky":
        report["stats"]["flaky"] = 1
    elif mutation == "top-level-error":
        report["errors"] = [{"message": "runner failure"}]
    elif mutation == "wrong-project":
        test["projectId"] = "firefox-page"
    else:
        frameset["id"] = "wrong-id"

    # Act / Assert
    with pytest.raises(ValueError):
        verifier.report_rows(run, json.dumps(report).encode(), _target_ids())


@pytest.mark.parametrize(
    ("key", "value"),
    [
        ("resultSha256", "0" * 64),
        ("candidateArtifactSha256", "0" * 64),
        ("candidateSourceSha256", "0" * 64),
        ("candidateProfile", "official-conformance-page-check"),
        ("exitCode", 0),
        ("filters", ["tests/page/page-dialog.spec.ts"]),
        ("command", ["node", "wrong-runner"]),
    ],
)
def test_first_run_provenance_mutation_is_rejected(key: str, value: object) -> None:
    # Arrange
    provenance = json.loads(verifier.provenance_path(1).read_bytes())
    provenance[key] = value

    # Act / Assert
    with pytest.raises(ValueError, match="provenance identity"):
        verifier.verify_run_provenance(1, json.dumps(provenance).encode())


def test_sealed_first_run_digest_rejects_byte_mutation(tmp_path: Path) -> None:
    # Arrange
    altered = tmp_path / "first-run.json"
    altered.write_bytes(verifier.report_path(1).read_bytes() + b"\n")

    # Act / Assert
    with pytest.raises(ValueError, match="SHA-256 changed"):
        verifier.sealed(
            altered, verifier.REPORT_SHA256[1], "candidate v1 official report"
        )


def test_sealed_latest_run_digest_rejects_byte_mutation(tmp_path: Path) -> None:
    # Arrange
    altered = tmp_path / "latest-run.json"
    altered.write_bytes(verifier.report_path(4).read_bytes() + b"\n")

    # Act / Assert
    with pytest.raises(ValueError, match="SHA-256 changed"):
        verifier.sealed(
            altered, verifier.REPORT_SHA256[4], "candidate v4 official report"
        )
