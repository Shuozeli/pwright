"""Read-only replay and mutation controls for the strict Core hierarchy v22 pair."""

from __future__ import annotations

import json
from pathlib import Path

import promote_playwright_core_frame_hierarchy13_rust_owner_strict_paired_v22 as promoter
import pytest
import verify_playwright_core_frame_hierarchy13_rust_owner_strict_paired_v22 as verifier


@pytest.fixture(scope="module")
def pair() -> tuple[dict[str, object], dict[str, bytes]]:
    # Arrange
    checked = verifier.verify_inputs()

    # Act
    files = promoter.prepare_pair(checked)
    receipt = verifier.verify_payloads(files, checked)

    # Assert
    assert receipt["candidateUnexpectedRows"] == 5
    assert receipt["oracleExpectedRows"] == 13
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


def _spec(report: dict[str, object], spec_id: str) -> dict[str, object]:
    return next(spec for spec in report["suites"][0]["specs"] if spec["id"] == spec_id)


def test_thirteen_rows_per_lane_and_unchanged_raw_rows(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    checked, files = pair

    # Act
    receipt = verifier.verify_payloads(files, checked)

    # Assert
    targets = sorted(checked["selected"])
    assert len(targets) == 13
    for lane in verifier.LANES:
        before = checked["oldLines"][lane]
        after = files[f"{lane}.jsonl"].splitlines(keepends=True)
        assert [
            index
            for index, (old, new) in enumerate(zip(before, after, strict=True))
            if old != new
        ] == targets
    assert receipt["changedRowsPerLane"] == {"candidate": 13, "oracle": 13}
    assert receipt["unchangedRawRowsPerLane"] == {"candidate": 48_464, "oracle": 48_464}
    assert receipt["publicApiParityEvidence"] is False
    assert receipt["fullCorpusExecution"] is False


def test_five_candidate_red_errors_and_inherited_instability_remain_visible(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    checked, files = pair

    # Act
    receipt = json.loads(files["promotion-receipt.json"])
    candidate_lines = files["candidate.jsonl"].splitlines(keepends=True)
    reds = {
        json.loads(candidate_lines[index])["specId"]: json.loads(candidate_lines[index])
        for index in checked["selected"]
        if json.loads(candidate_lines[index])["outcome"] == "unexpected"
    }

    # Assert
    assert set(reds) == verifier.EXPECTED_RED_IDS
    assert all(
        row["attempts"] == 1
        and row["error"] == checked["reports"]["candidate"][spec_id]["error"]
        for spec_id, row in reds.items()
    )
    assert receipt["redComparison"]["candidateOnlySpecIds"] == sorted(
        verifier.EXPECTED_RED_IDS
    )
    assert receipt["redComparison"]["candidateOnlyCount"] == 5
    assert receipt["redComparison"]["bothRedCount"] == 0
    assert receipt["redComparison"]["oracleOnlyCount"] == 0
    assert receipt["inheritedUnstableSpecIds"] == [verifier.predecessor.FRAMESET_ID]
    assert receipt["publicApiParityEvidence"] is False


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


@pytest.mark.parametrize("lane", verifier.LANES)
@pytest.mark.parametrize(
    ("key", "value"),
    [
        ("attempts", 2),
        ("outcome", "not-run"),
        ("resultFile", "wrong-report"),
        ("evidenceScope", "wrong-scope"),
        ("publicApiParityEvidence", True),
    ],
)
def test_target_row_mutation_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]], lane: str, key: str, value: object
) -> None:
    # Arrange
    checked, files = pair
    changed = _mutate_row(files, lane, min(checked["selected"]), key, value)

    # Act / Assert
    with pytest.raises(ValueError, match="target row"):
        verifier.verify_payloads(changed, checked)


def test_candidate_red_error_mutation_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]],
) -> None:
    # Arrange
    checked, files = pair
    red_index = next(
        index
        for index, row in checked["replacementRows"]["candidate"].items()
        if row["outcome"] == "unexpected"
    )
    changed = _mutate_row(files, "candidate", red_index, "error", "hidden failure")

    # Act / Assert
    with pytest.raises(ValueError, match="candidate target row"):
        verifier.verify_payloads(changed, checked)


@pytest.mark.parametrize("lane", verifier.LANES)
def test_summary_mutation_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]], lane: str
) -> None:
    # Arrange
    checked, files = pair
    changed = dict(files)
    summary = json.loads(files[f"{lane}.summary.json"])
    summary["outcomes"]["expected"] += 1
    changed[f"{lane}.summary.json"] = (json.dumps(summary) + "\n").encode()

    # Act / Assert
    with pytest.raises(ValueError, match="summary differs"):
        verifier.verify_payloads(changed, checked)


@pytest.mark.parametrize(
    ("key", "value"),
    [
        ("fullCorpusExecution", True),
        ("publicApiParityEvidence", True),
        ("inheritedUnstableSpecIds", []),
        ("candidateUnexpectedRows", 0),
        ("oracleExpectedRows", 12),
        ("changedRowsPerLane", {"candidate": 13, "oracle": 12}),
        ("redComparison", {}),
        ("targetZeroBasedIndexes", []),
        ("upstreamAssertionsModified", True),
    ],
)
def test_receipt_scope_or_red_mutation_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]], key: str, value: object
) -> None:
    # Arrange
    checked, files = pair
    changed = _mutate_receipt(files, key, value)

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
        "missing-spec",
        "wrong-id",
        "wrong-file",
        "wrong-project",
        "retry",
        "skip",
        "flaky",
        "top-level-error",
        "red-hidden",
        "red-moved",
        "red-no-error",
        "green-failed",
        "extra-attempt",
    ],
)
def test_candidate_report_mutation_is_rejected(
    pair: tuple[dict[str, object], dict[str, bytes]], mutation: str
) -> None:
    # Arrange
    checked, _ = pair
    report = json.loads(verifier.report_path("candidate").read_bytes())
    red = _spec(report, min(verifier.EXPECTED_RED_IDS))
    red_test = red["tests"][0]
    red_attempt = red_test["results"][0]
    green = next(
        spec
        for spec in report["suites"][0]["specs"]
        if spec["tests"][0]["status"] == "expected"
    )
    green_test = green["tests"][0]
    if mutation == "missing-spec":
        report["suites"][0]["specs"].remove(red)
    elif mutation == "wrong-id":
        red["id"] = "wrong-id"
    elif mutation == "wrong-file":
        red["file"] = "page/other.spec.ts"
    elif mutation == "wrong-project":
        red_test["projectId"] = "firefox-page"
    elif mutation == "retry":
        red_attempt["retry"] = 1
    elif mutation == "skip":
        report["stats"]["skipped"] = 1
    elif mutation == "flaky":
        report["stats"]["flaky"] = 1
    elif mutation == "top-level-error":
        report["errors"] = [{"message": "runner failure"}]
    elif mutation == "red-hidden":
        red_test["status"] = "expected"
        red_attempt["status"] = "passed"
        red_attempt["error"] = None
        red_attempt["errors"] = []
        report["stats"]["expected"] = 9
        report["stats"]["unexpected"] = 4
    elif mutation == "red-moved":
        red_test["status"] = "expected"
        red_attempt["status"] = "passed"
        red_attempt["error"] = None
        red_attempt["errors"] = []
        green_test["status"] = "unexpected"
        green_test["results"][0]["status"] = "failed"
        green_test["results"][0]["error"] = {"message": "moved failure"}
        green_test["results"][0]["errors"] = [{"message": "moved failure"}]
    elif mutation == "red-no-error":
        red_attempt.pop("error")
    elif mutation == "green-failed":
        green_test["results"][0]["status"] = "failed"
    else:
        red_test["results"].append(dict(red_attempt))

    # Act / Assert
    target_ids = {row["specId"] for row in checked["selected"].values()}
    with pytest.raises(ValueError):
        verifier.report_rows("candidate", json.dumps(report).encode(), target_ids)


@pytest.mark.parametrize(
    ("key", "value"),
    [
        ("resultSha256", "0" * 64),
        ("candidateArtifactSha256", "0" * 64),
        ("candidateSourceSha256", "0" * 64),
        ("candidateProfile", "official-conformance-page-check"),
        ("exitCode", 0),
        ("filters", ["tests/page/other.spec.ts"]),
        ("command", ["node", "wrong-runner"]),
    ],
)
def test_candidate_provenance_mutation_is_rejected(key: str, value: object) -> None:
    # Arrange
    provenance = json.loads(verifier.provenance_path("candidate").read_bytes())
    provenance[key] = value

    # Act / Assert
    with pytest.raises(ValueError, match="provenance identity|frozen runtime"):
        verifier.verify_provenance("candidate", json.dumps(provenance).encode())


def test_sealed_official_report_rejects_byte_mutation(tmp_path: Path) -> None:
    # Arrange
    altered = tmp_path / "candidate.json"
    altered.write_bytes(verifier.report_path("candidate").read_bytes() + b"\n")

    # Act / Assert
    with pytest.raises(ValueError, match="SHA-256 changed"):
        verifier.sealed(
            altered, verifier.REPORT_SHA256["candidate"], "candidate report"
        )
