"""Source-only replay and mutation controls for the strict Core Page.fill v13 pair."""

from __future__ import annotations

import json
from pathlib import Path
from unittest.mock import patch

import pytest
import verify_playwright_core_page_fill45_strict_paired_v13 as verifier


@pytest.fixture(scope="module")
def pair() -> tuple[dict[str, object], dict[str, bytes]]:
    # Arrange
    inputs = verifier.verify_inputs()
    candidate_lines = list(inputs["oldLines"]["candidate"])
    red_index = inputs["redIndex"]
    candidate_lines[red_index] = verifier.canonical_line(
        verifier.expected_red_row(json.loads(candidate_lines[red_index]))
    )
    ledgers = {
        "candidate": b"".join(candidate_lines),
        "oracle": b"".join(inputs["oldLines"]["oracle"]),
    }
    files = {f"{lane}.jsonl": ledgers[lane] for lane in verifier.LANES}
    for lane in verifier.LANES:
        summary = verifier.expected_summary(
            lane, inputs["oldSummaries"][lane], ledgers[lane]
        )
        files[f"{lane}.summary.json"] = (json.dumps(summary, indent=2) + "\n").encode()
    receipt = verifier.expected_receipt(files, inputs)
    files["promotion-receipt.json"] = (
        json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    ).encode()

    # Act
    checked = verifier.verify_payloads(files, inputs)

    # Assert
    assert checked == receipt
    return inputs, files


def _mutate_receipt(
    files: dict[str, bytes], key: str, value: object
) -> dict[str, bytes]:
    changed = dict(files)
    receipt = json.loads(files["promotion-receipt.json"])
    receipt[key] = value
    changed["promotion-receipt.json"] = (json.dumps(receipt) + "\n").encode()
    return changed


def _mutate_target(
    files: dict[str, bytes], key: str, value: object
) -> dict[str, bytes]:
    changed = dict(files)
    lines = files["candidate.jsonl"].splitlines(keepends=True)
    row = json.loads(lines[verifier.RED_INDEX])
    row[key] = value
    lines[verifier.RED_INDEX] = verifier.canonical_line(row)
    changed["candidate.jsonl"] = b"".join(lines)
    return changed


def _report(lane: str) -> dict[str, object]:
    return json.loads(verifier.report_path(lane).read_bytes())


def _target_ids(inputs: dict[str, object]) -> set[str]:
    return {row["specId"] for row in inputs["targets"].values()}


def test_exact_green_successor_changes_only_historical_candidate_red(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    inputs, files = pair
    old_candidate = inputs["oldLines"]["candidate"]
    new_candidate = files["candidate.jsonl"].splitlines(keepends=True)

    # Act
    receipt = verifier.verify_payloads(files, inputs)

    # Assert
    assert len(inputs["targets"]) == 45
    assert inputs["redIndex"] == 39_564
    assert sum(a != b for a, b in zip(old_candidate, new_candidate, strict=True)) == 1
    assert all(
        before == after
        for index, (before, after) in enumerate(
            zip(old_candidate, new_candidate, strict=True)
        )
        if index != inputs["redIndex"]
    )
    assert files["oracle.jsonl"] == b"".join(inputs["oldLines"]["oracle"])
    red = json.loads(new_candidate[inputs["redIndex"]])
    assert (red["specId"], red["line"], red["outcome"], red["attempts"]) == (
        verifier.RED_SPEC_ID,
        388,
        "expected",
        2,
    )
    assert "error" not in red
    assert red["resultFile"] == str(verifier.report_path("candidate"))
    assert receipt["changedRowsPerLane"] == {"candidate": 1, "oracle": 0}
    assert receipt["unchangedRawRowsPerLane"] == {"candidate": 48_476, "oracle": 48_477}
    assert receipt["candidateExpectedRows"] == receipt["oracleExpectedRows"] == 45
    assert receipt["candidateUnexpectedRows"] == 0
    assert receipt["completeFileExecution"] is True
    assert receipt["fullCorpusExecution"] is False
    assert receipt["publicApiParityEvidence"] is False


def test_candidate_non_target_raw_mutation_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    inputs, files = pair
    changed = dict(files)
    lines = files["candidate.jsonl"].splitlines(keepends=True)
    row = json.loads(lines[0])
    row["attempts"] = 123
    lines[0] = verifier.canonical_line(row)
    changed["candidate.jsonl"] = b"".join(lines)

    # Act / Assert
    with pytest.raises(ValueError, match="non-target raw row"):
        verifier.verify_payloads(changed, inputs)


def test_oracle_raw_mutation_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    inputs, files = pair
    changed = dict(files)
    lines = files["oracle.jsonl"].splitlines(keepends=True)
    row = json.loads(lines[verifier.RED_INDEX])
    row["attempts"] = 2
    lines[verifier.RED_INDEX] = verifier.canonical_line(row)
    changed["oracle.jsonl"] = b"".join(lines)

    # Act / Assert
    with pytest.raises(ValueError, match="non-target raw row"):
        verifier.verify_payloads(changed, inputs)


@pytest.mark.parametrize(
    ("key", "value"),
    [
        ("attempts", 1),
        ("outcome", "unexpected"),
        ("error", verifier.RED_ERROR),
        ("line", 389),
        ("publicApiParityEvidence", True),
        ("browserExecutionEvidence", False),
    ],
)
def test_historical_red_row_mutation_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]],
    key: str,
    value: object,
) -> None:
    # Arrange
    inputs, files = pair
    changed = _mutate_target(files, key, value)

    # Act / Assert
    with pytest.raises(ValueError, match="historical Page.fill RED row"):
        verifier.verify_payloads(changed, inputs)


@pytest.mark.parametrize(
    ("key", "value"),
    [
        ("publicApiParityEvidence", True),
        ("completeFileExecution", False),
        ("fullCorpusExecution", True),
        ("chromeVersionObserved", "154.0.0.0"),
        ("launchMode", "local-launch"),
        ("fixtureAdapted", False),
        ("changedRowsPerLane", {"candidate": 45, "oracle": 0}),
        ("targetZeroBasedIndexes", []),
    ],
)
def test_receipt_scope_mutation_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]],
    key: str,
    value: object,
) -> None:
    # Arrange
    inputs, files = pair
    changed = _mutate_receipt(files, key, value)

    # Act / Assert
    with pytest.raises(ValueError, match="receipt differs"):
        verifier.verify_payloads(changed, inputs)


def test_summary_outcome_spoof_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    inputs, files = pair
    changed = dict(files)
    summary = json.loads(files["candidate.summary.json"])
    summary["outcomes"]["expected"] += 1
    changed["candidate.summary.json"] = (json.dumps(summary) + "\n").encode()

    # Act / Assert
    with pytest.raises(ValueError, match="candidate summary differs"):
        verifier.verify_payloads(changed, inputs)


def test_missing_bundle_member_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    inputs, files = pair
    changed = dict(files)
    changed.pop("oracle.summary.json")

    # Act / Assert
    with pytest.raises(ValueError, match="bundle files"):
        verifier.verify_payloads(changed, inputs)


def test_report_requires_executed_pass_for_every_invocation(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    inputs, _ = pair
    report = _report("candidate")
    report["suites"][0]["specs"][0]["tests"][0]["results"] = []

    # Act / Assert
    with pytest.raises(ValueError, match="unexecuted or failing invocation"):
        verifier.report_rows(
            "candidate", json.dumps(report).encode(), _target_ids(inputs)
        )


def test_report_rejects_duplicate_id(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    inputs, _ = pair
    report = _report("candidate")
    specs = report["suites"][0]["specs"]
    specs[1]["id"] = specs[0]["id"]

    # Act / Assert
    with pytest.raises(ValueError, match="wrong, missing, or duplicate invocation"):
        verifier.report_rows(
            "candidate", json.dumps(report).encode(), _target_ids(inputs)
        )


def test_report_rejects_skipped_attempt(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    inputs, _ = pair
    report = _report("oracle")
    report["suites"][0]["specs"][0]["tests"][0]["results"][0]["status"] = "skipped"

    # Act / Assert
    with pytest.raises(ValueError, match="unexecuted or failing invocation"):
        verifier.report_rows("oracle", json.dumps(report).encode(), _target_ids(inputs))


def test_report_rejects_attempt_error_array(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    inputs, _ = pair
    report = _report("candidate")
    report["suites"][0]["specs"][0]["tests"][0]["results"][0]["errors"] = [
        {"message": "hidden failure"}
    ]

    # Act / Assert
    with pytest.raises(ValueError, match="unexecuted or failing invocation"):
        verifier.report_rows(
            "candidate", json.dumps(report).encode(), _target_ids(inputs)
        )


def test_candidate_provenance_rejects_nonzero_exit_code() -> None:
    # Arrange
    provenance = json.loads(verifier.provenance_path("candidate").read_bytes())
    provenance["exitCode"] = 1

    # Act / Assert
    with pytest.raises(ValueError, match="provenance identity"):
        verifier.verify_provenance("candidate", json.dumps(provenance).encode())


def test_oracle_provenance_rejects_candidate_runtime() -> None:
    # Arrange
    provenance = json.loads(verifier.provenance_path("oracle").read_bytes())
    provenance["candidateArtifactSha256"] = verifier.CANDIDATE_ARTIFACT_SHA256

    # Act / Assert
    with pytest.raises(ValueError, match="contains a candidate runtime"):
        verifier.verify_provenance("oracle", json.dumps(provenance).encode())


def test_candidate_provenance_rejects_tooling_source_drift() -> None:
    # Arrange
    provenance = json.loads(verifier.provenance_path("candidate").read_bytes())
    provenance["toolingSources"][0]["sha256"] = "0" * 64

    # Act / Assert
    with pytest.raises(ValueError, match="tooling source pins differ"):
        verifier.verify_provenance("candidate", json.dumps(provenance).encode())


def test_frozen_package_manifest_hash_drift_is_rejected(tmp_path: Path) -> None:
    # Arrange
    manifest = tmp_path / "candidate-manifest.json"
    manifest.write_bytes(b"changed")

    # Act / Assert
    with (
        patch.object(verifier, "PACKAGE_MANIFEST", manifest),
        pytest.raises(ValueError, match="SHA-256 changed"),
    ):
        verifier.verify_package()


def test_historical_v12_receipt_hash_drift_is_rejected() -> None:
    # Arrange
    changed_pin = {"promotion-receipt.json": "0" * 64}

    # Act / Assert
    with (
        patch.dict(verifier.V12_SHA256, changed_pin),
        pytest.raises(ValueError, match="v12 promotion-receipt.json SHA-256 changed"),
    ):
        verifier.verify_inputs()
