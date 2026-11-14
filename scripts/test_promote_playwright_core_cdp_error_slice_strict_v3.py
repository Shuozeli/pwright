"""Synthetic controls for the unsealed post-fix Core CDP error promoter."""

from __future__ import annotations

import json
from pathlib import Path

import promote_playwright_core_cdp_error_slice_strict_v3 as promoter
import pytest
import run_playwright_core_cdp_error_slice_strict_v1 as source
import run_playwright_core_cdp_error_slice_strict_v2 as worker_aware
import run_playwright_core_cdp_error_slice_strict_v3 as capture


def jsonl(rows: list[dict]) -> bytes:
    return b"".join(
        (json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n").encode() for row in rows
    )


def inventory_row(index: int) -> dict:
    if index in promoter.TARGETS:
        line, spec_id, title = promoter.TARGETS[index]
        file = source.REPORT_FILE
        reasons = promoter.CLASSIFICATION_REASONS
        classification = "implementation-internal"
    else:
        line, spec_id, title = 20 + index, f"unrelated-{index}", f"unrelated {index}"
        file = f"library/unrelated-{index}.spec.ts"
        reasons = []
        classification = "public-api-candidate"
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


def passing_report() -> dict:
    specs = []
    for worker_index, (line, spec_id, title) in enumerate(promoter.TARGETS.values()):
        specs.append(
            {
                "id": spec_id,
                "file": source.REPORT_FILE,
                "line": line,
                "column": 5,
                "title": title,
                "ok": True,
                "tests": [
                    {
                        "projectId": "chromium-library",
                        "expectedStatus": "passed",
                        "status": "expected",
                        "results": [
                            {
                                "retry": 0,
                                "workerIndex": worker_index,
                                "status": "passed",
                                "errors": [],
                            }
                        ],
                    }
                ],
            }
        )
    return {
        "errors": [],
        "suites": [{"specs": specs, "suites": []}],
        "stats": {"expected": 3, "unexpected": 0, "skipped": 0, "flaky": 0},
    }


def synthetic_pair(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> dict[str, Path]:
    outcomes = tmp_path / "outcomes"
    baseline = outcomes / "baseline"
    results = tmp_path / "results"
    baseline.mkdir(parents=True)
    results.mkdir()
    monkeypatch.setattr(
        promoter,
        "TARGETS",
        {
            index: (line, spec_id, title)
            for index, (spec_id, (line, title)) in enumerate(source.CASES.items(), start=1)
        },
    )
    inventory = tmp_path / "inventory.jsonl"
    rows = [inventory_row(index) for index in range(5)]
    inventory.write_bytes(jsonl(rows))
    monkeypatch.setattr(promoter, "OUTCOMES", outcomes)
    monkeypatch.setattr(promoter, "BASE", baseline)
    monkeypatch.setattr(promoter, "OUTPUT", outcomes / "successor")
    monkeypatch.setattr(promoter, "INVENTORY", inventory)
    monkeypatch.setattr(promoter, "INVENTORY_SHA256", promoter.sha256(inventory))
    monkeypatch.setattr(promoter, "COUNT", 5)
    monkeypatch.setattr(capture, "RESULT_PARENT", results)
    monkeypatch.setattr(capture, "SNAPSHOT_PACKAGE", results / "candidate-package-snapshot-v1")
    snapshot_manifest = results / "candidate-package-snapshot-v1.files.json"
    snapshot_manifest.write_text('{"synthetic":true}\n')
    monkeypatch.setattr(capture, "SNAPSHOT_MANIFEST", snapshot_manifest)
    snapshot_manifest_sha = promoter.sha256(snapshot_manifest)
    monkeypatch.setattr(promoter, "SNAPSHOT_MANIFEST_SHA256", snapshot_manifest_sha)
    artifact = tmp_path / "candidate.node"
    artifact.write_bytes(b"synthetic native profile")
    artifact_sha = promoter.sha256(artifact)
    monkeypatch.setattr(promoter, "ARTIFACT_PATH", artifact)
    monkeypatch.setattr(promoter, "ARTIFACT_SHA256", artifact_sha)
    monkeypatch.setattr(capture, "ARTIFACT_SHA256", artifact_sha)
    monkeypatch.setattr(
        promoter,
        "HARNESS_PINS",
        {str(path.relative_to(source.ROOT)): promoter.sha256(path) for path in capture.INPUT_FILES},
    )
    prior = {
        "inventory": str(inventory),
        "inventorySha256": promoter.sha256(inventory),
        "recordsPerLane": 5,
        "promoterSourceSha256": promoter.BASE_PROMOTER_SHA256,
        "targetZeroBasedIndexes": [1, 2, 3],
        "targetSpecIds": [promoter.TARGETS[index][1] for index in (1, 2, 3)],
        "sourceClassificationPreserved": "implementation-internal",
        "publicApiParityEvidence": False,
        "lanes": {},
    }
    paths: dict[str, Path] = {}
    lanes: dict[str, dict] = {}
    base_hashes: dict[str, str] = {}
    for lane in ("candidate", "oracle"):
        old_outcome = "unexpected" if lane == "candidate" else "expected"
        baseline_rows = []
        for index, row in enumerate(rows):
            if index in promoter.TARGETS:
                target = {
                    **row,
                    "attempts": 1,
                    "outcome": old_outcome,
                    "resultFile": str(promoter.OLD_RESULT_PARENT / f"{lane}-v1.json"),
                    "evidenceScope": promoter.OLD_SCOPE,
                    "candidateRuntimeRequired": lane == "candidate",
                }
                if lane == "candidate":
                    target["error"] = f"old authentic failure {index}"
                baseline_rows.append(target)
            else:
                baseline_rows.append({**row, "attempts": 0, "outcome": "not-run"})
        ledger = baseline / f"{lane}.jsonl"
        ledger.write_bytes(jsonl(baseline_rows))
        summary = baseline / f"{lane}.summary.json"
        summary.write_text(
            json.dumps(
                {
                    "schema": 1,
                    "inventory": str(inventory),
                    "records": 5,
                    "outcomes": {"not-run": 2, old_outcome: 3},
                    "explicitEvidenceScopes": {promoter.OLD_SCOPE: 3},
                    "ledgerSha256": promoter.sha256(ledger),
                    "ledger": str(ledger),
                }
            )
        )
        prior["lanes"][lane] = {
            "ledger": str(ledger),
            "ledgerSha256": promoter.sha256(ledger),
            "summarySha256": promoter.sha256(summary),
            "newOutcome": old_outcome,
        }
        base_hashes[lane] = promoter.sha256(ledger)
        paths[f"{lane}_base"] = ledger
        result = results / f"{lane}-v1.json"
        raw = passing_report()
        result.write_text(json.dumps(raw))
        strict_path = result.with_suffix(".strict.json")
        strict = {
            "schema": 4,
            "mode": lane,
            "scope": capture.SCOPE,
            "upstreamCommit": source.COMMIT,
            "upstreamSourceSha256": source.SOURCE_SHA256,
            "serverFixtureSha256": source.SERVER_FIXTURE_SHA256,
            "commonPreloadSha256": source.COMMON_PRELOAD_SHA256,
            "inputs": promoter.HARNESS_PINS,
            "exitCode": 0,
            "candidateProfile": capture.PROFILE if lane == "candidate" else None,
            "candidateArtifactSha256": artifact_sha,
            "pairedOracleStrictSha256": None,
            "candidateSnapshot": str(capture.SNAPSHOT_PACKAGE) if lane == "candidate" else None,
            "candidateSnapshotManifestSha256": (
                snapshot_manifest_sha if lane == "candidate" else None
            ),
            "candidateSourceSha256": "b" * 64 if lane == "candidate" else None,
            "result": str(result),
            "resultSha256": promoter.sha256(result),
            "selected": worker_aware.selected_tests(raw),
        }
        strict_path.write_text(json.dumps(strict))
        lanes[lane] = {
            "strict": strict_path,
            "strictSha256": promoter.sha256(strict_path),
            "oldOutcome": old_outcome,
        }
        paths[f"{lane}_result"] = result
        paths[f"{lane}_strict"] = strict_path
    candidate_strict = paths["candidate_strict"]
    candidate = json.loads(candidate_strict.read_text())
    candidate["pairedOracleStrictSha256"] = lanes["oracle"]["strictSha256"]
    candidate_strict.write_text(json.dumps(candidate))
    lanes["candidate"]["strictSha256"] = promoter.sha256(candidate_strict)
    prior_path = baseline / "promotion-receipt.json"
    prior_path.write_text(json.dumps(prior))
    monkeypatch.setattr(promoter, "BASE_RECEIPT_SHA256", promoter.sha256(prior_path))
    monkeypatch.setattr(promoter, "BASE_SHA256", base_hashes)
    monkeypatch.setattr(promoter, "LANES", lanes)
    monkeypatch.setattr(promoter, "verify_strict", lambda path: json.loads(path.read_text()))
    return paths


def test_only_three_rows_refresh_with_genuine_candidate_passes(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    paths = synthetic_pair(tmp_path, monkeypatch)

    # Act
    files = promoter.prepare_pair()

    # Assert
    for lane in ("candidate", "oracle"):
        before = paths[f"{lane}_base"].read_bytes().splitlines(keepends=True)
        after = files[f"{lane}.jsonl"].splitlines(keepends=True)
        assert len(before) == len(after) == 5
        assert [index for index in range(5) if before[index] != after[index]] == [1, 2, 3]
        for index in (1, 2, 3):
            old, new = json.loads(before[index]), json.loads(after[index])
            assert new["specId"] == old["specId"]
            assert new["sourceClassification"] == "implementation-internal"
            assert new["classificationReasons"] == old["classificationReasons"]
            assert new["outcome"] == "expected"
            assert new["attempts"] == 1
            assert "error" not in new
    receipt = json.loads(files["promotion-receipt.json"])
    assert receipt["publicApiParityEvidence"] is False
    assert receipt["targetZeroBasedIndexes"] == [1, 2, 3]
    assert receipt["lanes"]["candidate"]["oldOutcome"] == "unexpected"
    assert receipt["lanes"]["oracle"]["oldOutcome"] == "expected"


def test_atomic_publication_is_verifiable_and_exclusive(
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
    ledger = promoter.OUTPUT / "oracle.jsonl"
    ledger.write_bytes(ledger.read_bytes() + b"\n")

    # Act / Assert
    with pytest.raises(ValueError, match="immutable output drifted"):
        promoter.verify_existing()


def test_candidate_failure_cannot_be_promoted_as_pass(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    paths = synthetic_pair(tmp_path, monkeypatch)
    result = paths["candidate_result"]
    raw = json.loads(result.read_text())
    spec = raw["suites"][0]["specs"][0]
    spec["ok"] = False
    test = spec["tests"][0]
    test["status"] = "unexpected"
    test["results"][0]["status"] = "failed"
    test["results"][0]["errors"] = [{"message": "still broken"}]
    raw["stats"].update({"expected": 2, "unexpected": 1})
    result.write_text(json.dumps(raw))
    strict_path = paths["candidate_strict"]
    strict = json.loads(strict_path.read_text())
    strict["resultSha256"] = promoter.sha256(result)
    strict["selected"] = worker_aware.selected_tests(raw)
    strict["exitCode"] = 1
    strict_path.write_text(json.dumps(strict))
    promoter.LANES["candidate"]["strictSha256"] = promoter.sha256(strict_path)

    # Act / Assert
    with pytest.raises(ValueError, match="strict official pass provenance drifted"):
        promoter.prepare_pair()


def test_oracle_failure_cannot_refresh_expected_row(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    paths = synthetic_pair(tmp_path, monkeypatch)
    result = paths["oracle_result"]
    raw = json.loads(result.read_text())
    spec = raw["suites"][0]["specs"][1]
    spec["ok"] = False
    test = spec["tests"][0]
    test["status"] = "unexpected"
    test["results"][0]["status"] = "failed"
    test["results"][0]["errors"] = [{"message": "oracle failed"}]
    raw["stats"].update({"expected": 2, "unexpected": 1})
    result.write_text(json.dumps(raw))
    strict_path = paths["oracle_strict"]
    strict = json.loads(strict_path.read_text())
    strict["resultSha256"] = promoter.sha256(result)
    strict["selected"] = worker_aware.selected_tests(raw)
    strict["exitCode"] = 1
    strict_path.write_text(json.dumps(strict))
    promoter.LANES["oracle"]["strictSha256"] = promoter.sha256(strict_path)
    candidate_path = paths["candidate_strict"]
    candidate = json.loads(candidate_path.read_text())
    candidate["pairedOracleStrictSha256"] = promoter.LANES["oracle"]["strictSha256"]
    candidate_path.write_text(json.dumps(candidate))
    promoter.LANES["candidate"]["strictSha256"] = promoter.sha256(candidate_path)

    # Act / Assert
    with pytest.raises(ValueError, match="strict official pass provenance drifted"):
        promoter.prepare_pair()


def test_candidate_must_bind_pinned_oracle_sidecar(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    paths = synthetic_pair(tmp_path, monkeypatch)
    strict_path = paths["candidate_strict"]
    strict = json.loads(strict_path.read_text())
    strict["pairedOracleStrictSha256"] = "0" * 64
    strict_path.write_text(json.dumps(strict))
    promoter.LANES["candidate"]["strictSha256"] = promoter.sha256(strict_path)

    # Act / Assert
    with pytest.raises(ValueError, match="oracle/runtime pairing drifted"):
        promoter.prepare_pair()


def test_sidecar_hash_drift_is_rejected(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    paths = synthetic_pair(tmp_path, monkeypatch)
    paths["candidate_strict"].write_text(paths["candidate_strict"].read_text() + " ")

    # Act / Assert
    with pytest.raises(ValueError, match="SHA-256 drift"):
        promoter.prepare_pair()


def test_snapshot_manifest_hash_drift_is_rejected(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    synthetic_pair(tmp_path, monkeypatch)
    capture.SNAPSHOT_MANIFEST.write_text('{"synthetic":false}\n')

    # Act / Assert
    with pytest.raises(ValueError, match="SHA-256 drift"):
        promoter.prepare_pair()


def test_pending_artifact_pin_blocks_promotion(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    synthetic_pair(tmp_path, monkeypatch)
    monkeypatch.setattr(promoter, "ARTIFACT_SHA256", "PENDING_FINAL_ARTIFACT_SHA256")

    # Act / Assert
    with pytest.raises(ValueError, match="artifact/profile pin drifted"):
        promoter.prepare_pair()


def test_verifier_hash_drift_is_rejected(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    synthetic_pair(tmp_path, monkeypatch)
    monkeypatch.setitem(
        promoter.HARNESS_PINS,
        "scripts/verify_playwright_core_cdp_error_slice_strict_v3.py",
        "0" * 64,
    )

    # Act / Assert
    with pytest.raises(ValueError, match="SHA-256 drift"):
        promoter.prepare_pair()


def test_prior_receipt_drift_is_rejected(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    synthetic_pair(tmp_path, monkeypatch)
    receipt = promoter.BASE / "promotion-receipt.json"
    receipt.write_text(receipt.read_text() + " ")

    # Act / Assert
    with pytest.raises(ValueError, match="SHA-256 drift"):
        promoter.prepare_pair()


def test_classification_change_is_rejected(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    paths = synthetic_pair(tmp_path, monkeypatch)
    ledger = paths["candidate_base"]
    lines = ledger.read_bytes().splitlines(keepends=True)
    changed = json.loads(lines[1])
    changed["sourceClassification"] = "public-api-candidate"
    lines[1] = jsonl([changed])
    ledger.write_bytes(b"".join(lines))

    # Act / Assert
    with pytest.raises(ValueError, match="inventory identity/classification drift"):
        promoter.prepare_lane("candidate")
