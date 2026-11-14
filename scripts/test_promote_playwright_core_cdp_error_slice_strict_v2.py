"""Synthetic fail-closed controls for the three-row CDP error pair promoter."""

from __future__ import annotations

import json
from pathlib import Path

import promote_playwright_core_cdp_error_slice_strict_v2 as promoter
import pytest
import run_playwright_core_cdp_error_slice_strict_v1 as source
import run_playwright_core_cdp_error_slice_strict_v2 as capture


def jsonl(rows: list[dict]) -> bytes:
    return b"".join(
        (json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n").encode() for row in rows
    )


def inventory_row(index: int) -> dict:
    if index in promoter.TARGETS:
        line, spec_id, title = promoter.TARGETS[index]
        file = source.SOURCE.removeprefix("tests/")
        classification = "implementation-internal"
        reasons = promoter.CLASSIFICATION_REASONS
    else:
        line, spec_id, title = 20 + index, f"unrelated-{index}", f"unrelated {index}"
        file = f"library/unrelated-{index}.spec.ts"
        classification = "public-api-candidate"
        reasons = []
    return {
        "specId": spec_id,
        "projectId": "chromium-library",
        "projectName": "chromium-library",
        "sourceClassification": classification,
        "classificationReasons": reasons,
        "sourceFile": f"tests/{file}",
        "file": file,
        "line": line,
        "column": 5,
        "expectedStatus": "passed",
        "role": "official-entrypoint",
        "suite": "library",
        "titlePath": [file, title],
    }


def report(lane: str) -> dict:
    passed = lane == "oracle"
    specs = []
    for worker_index, (line, spec_id, title) in enumerate(promoter.TARGETS.values()):
        specs.append(
            {
                "id": spec_id,
                "file": source.REPORT_FILE,
                "line": line,
                "column": 5,
                "title": title,
                "ok": passed,
                "tests": [
                    {
                        "projectId": "chromium-library",
                        "expectedStatus": "passed",
                        "status": "expected" if passed else "unexpected",
                        "results": [
                            {
                                "retry": 0,
                                "workerIndex": worker_index,
                                "status": "passed" if passed else "failed",
                                "errors": []
                                if passed
                                else [{"message": f"genuine failure at line {line}"}],
                            }
                        ],
                    }
                ],
            }
        )
    return {
        "errors": [],
        "suites": [{"specs": specs, "suites": []}],
        "stats": {
            "expected": 3 if passed else 0,
            "unexpected": 0 if passed else 3,
            "skipped": 0,
            "flaky": 0,
        },
    }


def synthetic_pair(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> dict[str, Path]:
    outcomes = tmp_path / "outcomes"
    baseline = outcomes / "baseline"
    results = tmp_path / "results"
    baseline.mkdir(parents=True)
    results.mkdir()
    inventory = tmp_path / "inventory.jsonl"
    monkeypatch.setattr(
        promoter,
        "TARGETS",
        {
            index: (line, spec_id, title)
            for index, (spec_id, (line, title)) in enumerate(source.CASES.items(), start=1)
        },
    )
    rows = [inventory_row(index) for index in range(5)]
    inventory.write_bytes(jsonl(rows))
    monkeypatch.setattr(promoter, "OUTCOMES", outcomes)
    monkeypatch.setattr(promoter, "BASE", baseline)
    monkeypatch.setattr(promoter, "OUTPUT", outcomes / "successor")
    monkeypatch.setattr(promoter, "INVENTORY", inventory)
    monkeypatch.setattr(promoter, "INVENTORY_SHA256", promoter.sha256(inventory))
    monkeypatch.setattr(promoter, "COUNT", 5)
    prior = {
        "inventory": str(inventory),
        "inventorySha256": promoter.sha256(inventory),
        "recordsPerLane": 5,
        "lanes": {},
    }
    paths: dict[str, Path] = {}
    base_hashes: dict[str, str] = {}
    lanes: dict[str, dict] = {}
    for lane in ("candidate", "oracle"):
        ledger = baseline / f"{lane}.jsonl"
        ledger.write_bytes(jsonl([{**row, "attempts": 0, "outcome": "not-run"} for row in rows]))
        summary = baseline / f"{lane}.summary.json"
        summary.write_text(
            json.dumps(
                {
                    "schema": 1,
                    "inventory": str(inventory),
                    "records": 5,
                    "outcomes": {"not-run": 5},
                    "explicitEvidenceScopes": {},
                    "ledgerSha256": promoter.sha256(ledger),
                    "ledger": str(ledger),
                }
            )
        )
        prior["lanes"][lane] = {
            "ledger": str(ledger),
            "ledgerSha256": promoter.sha256(ledger),
            "summarySha256": promoter.sha256(summary),
        }
        base_hashes[lane] = promoter.sha256(ledger)
        result = results / f"{lane}-v1.json"
        raw = report(lane)
        result.write_text(json.dumps(raw))
        strict_path = result.with_suffix(".strict.json")
        strict_path.write_text(
            json.dumps(
                {
                    "mode": lane,
                    "scope": capture.SCOPE,
                    "upstreamCommit": source.COMMIT,
                    "upstreamSourceSha256": source.SOURCE_SHA256,
                    "serverFixtureSha256": source.SERVER_FIXTURE_SHA256,
                    "commonPreloadSha256": source.COMMON_PRELOAD_SHA256,
                    "inputs": promoter.HARNESS_PINS,
                    "exitCode": 1 if lane == "candidate" else 0,
                    "candidateProfile": source.PROFILE if lane == "candidate" else None,
                    "candidateArtifactSha256": source.ARTIFACT_SHA256
                    if lane == "candidate"
                    else None,
                    "result": str(result),
                    "resultSha256": promoter.sha256(result),
                    "selected": capture.selected_tests(raw),
                }
            )
        )
        lanes[lane] = {
            "strict": strict_path,
            "strictSha256": promoter.sha256(strict_path),
            "outcome": "unexpected" if lane == "candidate" else "expected",
            "exitCode": 1 if lane == "candidate" else 0,
        }
        paths[f"{lane}_base"] = ledger
        paths[f"{lane}_result"] = result
        paths[f"{lane}_strict"] = strict_path
    prior_path = baseline / "promotion-receipt.json"
    prior_path.write_text(json.dumps(prior))
    monkeypatch.setattr(promoter, "BASE_RECEIPT_SHA256", promoter.sha256(prior_path))
    monkeypatch.setattr(promoter, "BASE_SHA256", base_hashes)
    monkeypatch.setattr(promoter, "LANES", lanes)
    monkeypatch.setattr(promoter, "verify_strict", lambda path: json.loads(path.read_text()))
    return paths


def test_only_three_target_rows_change_and_failures_stay_failures(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    paths = synthetic_pair(tmp_path, monkeypatch)

    # Act
    files = promoter.prepare_pair()

    # Assert
    for lane in ("candidate", "oracle"):
        old = paths[f"{lane}_base"].read_bytes().splitlines(keepends=True)
        new = files[f"{lane}.jsonl"].splitlines(keepends=True)
        assert len(old) == len(new) == 5
        assert [index for index in range(5) if old[index] != new[index]] == [1, 2, 3]
        for index in (1, 2, 3):
            before, after = json.loads(old[index]), json.loads(new[index])
            assert after["specId"] == before["specId"]
            assert after["sourceClassification"] == "implementation-internal"
            assert after["classificationReasons"] == before["classificationReasons"]
            assert after["outcome"] == ("unexpected" if lane == "candidate" else "expected")
            assert after["attempts"] == 1
            assert ("error" in after) == (lane == "candidate")
    receipt = json.loads(files["promotion-receipt.json"])
    assert receipt["targetZeroBasedIndexes"] == [1, 2, 3]
    assert receipt["publicApiParityEvidence"] is False
    assert receipt["strictVerifierSha256"] == promoter.VERIFIER_SHA256


def test_atomic_pair_verifies_and_rejects_second_publication(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    synthetic_pair(tmp_path, monkeypatch)
    files = promoter.prepare_pair()

    # Act
    promoter.publish(files)

    # Assert
    assert promoter.verify_existing()["recordsPerLane"] == 5
    with pytest.raises(ValueError, match="already exists"):
        promoter.publish(files)


def test_published_byte_tamper_is_rejected(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    synthetic_pair(tmp_path, monkeypatch)
    promoter.publish(promoter.prepare_pair())
    candidate = promoter.OUTPUT / "candidate.jsonl"
    candidate.write_bytes(candidate.read_bytes() + b"\n")

    # Act / Assert
    with pytest.raises(ValueError, match="immutable output drifted"):
        promoter.verify_existing()


def test_sidecar_sha_drift_is_rejected(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    paths = synthetic_pair(tmp_path, monkeypatch)
    paths["candidate_strict"].write_text(paths["candidate_strict"].read_text() + " ")

    # Act / Assert
    with pytest.raises(ValueError, match="SHA-256 drift"):
        promoter.prepare_pair()


def test_unpinned_sidecar_is_rejected(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    synthetic_pair(tmp_path, monkeypatch)
    promoter.LANES["oracle"]["strictSha256"] = "PENDING"

    # Act / Assert
    with pytest.raises(ValueError, match="unfilled or invalid SHA-256 pin"):
        promoter.prepare_pair()


def test_verifier_source_drift_is_rejected(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    synthetic_pair(tmp_path, monkeypatch)
    monkeypatch.setattr(promoter, "VERIFIER_SHA256", "0" * 64)

    # Act / Assert
    with pytest.raises(ValueError, match="SHA-256 drift"):
        promoter.prepare_pair()


def test_runner_source_drift_is_rejected(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    synthetic_pair(tmp_path, monkeypatch)
    monkeypatch.setitem(
        promoter.HARNESS_PINS,
        "scripts/run_playwright_core_cdp_error_slice_strict_v2.py",
        "0" * 64,
    )

    # Act / Assert
    with pytest.raises(ValueError, match="SHA-256 drift"):
        promoter.prepare_pair()


def test_candidate_pass_cannot_be_mislabeled_as_authentic_failure(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    paths = synthetic_pair(tmp_path, monkeypatch)
    result = paths["candidate_result"]
    raw = json.loads(result.read_text())
    raw["suites"][0]["specs"][0]["ok"] = True
    test = raw["suites"][0]["specs"][0]["tests"][0]
    test["status"] = "expected"
    test["results"][0]["status"] = "passed"
    test["results"][0]["errors"] = []
    raw["stats"].update({"expected": 1, "unexpected": 2})
    result.write_text(json.dumps(raw))
    strict_path = paths["candidate_strict"]
    strict = json.loads(strict_path.read_text())
    strict["resultSha256"] = promoter.sha256(result)
    strict["selected"] = capture.selected_tests(raw)
    strict_path.write_text(json.dumps(strict))
    promoter.LANES["candidate"]["strictSha256"] = promoter.sha256(strict_path)

    # Act / Assert
    with pytest.raises(ValueError, match="official CDP assertion outcome drifted"):
        promoter.prepare_pair()


def test_target_classification_change_is_rejected(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    paths = synthetic_pair(tmp_path, monkeypatch)
    baseline = paths["oracle_base"]
    lines = baseline.read_bytes().splitlines(keepends=True)
    changed = json.loads(lines[1])
    changed["sourceClassification"] = "public-api-candidate"
    lines[1] = jsonl([changed])
    baseline.write_bytes(b"".join(lines))

    # Act / Assert
    with pytest.raises(ValueError, match="inventory identity/classification drift"):
        promoter.prepare_lane("oracle")


def test_prior_receipt_drift_is_rejected(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    synthetic_pair(tmp_path, monkeypatch)
    receipt = promoter.BASE / "promotion-receipt.json"
    receipt.write_text(receipt.read_text() + " ")

    # Act / Assert
    with pytest.raises(ValueError, match="SHA-256 drift"):
        promoter.prepare_pair()
