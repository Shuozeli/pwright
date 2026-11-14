"""Synthetic negative controls for the pinned Core default-UA pair promoter."""

from __future__ import annotations

import json
from pathlib import Path

import promote_playwright_core_cdp_default_user_agent_strict_v2 as promoter
import pytest
from run_playwright_core_cdp_default_user_agent_strict_v2 import (
    COMMIT,
    PROFILE,
    SOURCE,
    SOURCE_SHA256,
    SPEC_ID,
    TITLE,
    selected_tests,
)
from run_playwright_core_cdp_default_user_agent_strict_v2 import SCOPE as STRICT_SCOPE


def jsonl(rows: list[dict]) -> bytes:
    return b"".join(
        (json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n").encode()
        for row in rows
    )


def inventory_row(index: int) -> dict:
    title = TITLE if index == 1 else f"unrelated {index}"
    file = (
        SOURCE.removeprefix("tests/")
        if index == 1
        else f"library/other-{index}.spec.ts"
    )
    return {
        "specId": SPEC_ID if index == 1 else f"other-{index}",
        "projectId": "chromium-library",
        "projectName": "chromium-library",
        "sourceClassification": "implementation-internal"
        if index == 1
        else "public-api-candidate",
        "classificationReasons": promoter.CLASSIFICATION_REASONS if index == 1 else [],
        "sourceFile": SOURCE if index == 1 else f"tests/{file}",
        "file": file,
        "line": 299 if index == 1 else 10 + index,
        "column": 5,
        "expectedStatus": "passed",
        "role": "official-entrypoint",
        "suite": "library",
        "titlePath": [file, title],
    }


def report() -> dict:
    return {
        "errors": [],
        "suites": [
            {
                "specs": [
                    {
                        "id": SPEC_ID,
                        "file": SOURCE.removeprefix("tests/"),
                        "line": 299,
                        "column": 5,
                        "title": TITLE,
                        "ok": True,
                        "tests": [
                            {
                                "projectId": "chromium-library",
                                "expectedStatus": "passed",
                                "status": "expected",
                                "results": [
                                    {
                                        "retry": 0,
                                        "workerIndex": 0,
                                        "status": "passed",
                                        "errors": [],
                                    }
                                ],
                            }
                        ],
                    }
                ],
                "suites": [],
            }
        ],
        "stats": {"expected": 1, "unexpected": 0, "skipped": 0, "flaky": 0},
    }


def synthetic_pair(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> dict[str, Path]:
    outcomes = tmp_path / "outcomes"
    base = outcomes / "baseline"
    results = tmp_path / "results"
    base.mkdir(parents=True)
    results.mkdir()
    inventory = tmp_path / "inventory.jsonl"
    inventory_rows = [inventory_row(index) for index in range(4)]
    inventory.write_bytes(jsonl(inventory_rows))
    monkeypatch.setattr(promoter, "OUTCOMES", outcomes)
    monkeypatch.setattr(promoter, "BASE", base)
    monkeypatch.setattr(promoter, "RESULTS", results)
    monkeypatch.setattr(promoter, "OUTPUT", outcomes / "successor")
    monkeypatch.setattr(promoter, "INVENTORY", inventory)
    monkeypatch.setattr(promoter, "INVENTORY_SHA256", promoter.sha256(inventory))
    monkeypatch.setattr(promoter, "COUNT", 4)
    monkeypatch.setattr(promoter, "TARGET_INDEX", 1)
    lanes: dict[str, dict] = {}
    paths: dict[str, Path] = {}
    prior_receipt = {
        "inventory": str(inventory),
        "inventorySha256": promoter.sha256(inventory),
        "recordsPerLane": 4,
        "lanes": {},
    }
    for lane in ("candidate", "oracle"):
        baseline = [dict(row, outcome="not-run", attempts=0) for row in inventory_rows]
        target = baseline[1]
        target.update(
            {
                "outcome": "unexpected" if lane == "candidate" else "expected",
                "attempts": 1,
                "evidenceScope": promoter.OLD_SCOPE,
                "candidateRuntimeRequired": lane == "candidate",
                "resultFile": f"old-{lane}.json",
            }
        )
        if lane == "candidate":
            target["error"] = "Expected: Playwright/1.58.0; Received: undefined"
        base_path = base / f"{lane}.jsonl"
        base_path.write_bytes(jsonl(baseline))
        counts = {"not-run": 3, target["outcome"]: 1}
        summary = {
            "schema": 1,
            "inventory": str(inventory),
            "records": 4,
            "outcomes": counts,
            "explicitEvidenceScopes": {promoter.OLD_SCOPE: 1},
            "ledgerSha256": promoter.sha256(base_path),
            "ledger": str(base_path),
        }
        summary_path = base / f"{lane}.summary.json"
        summary_path.write_text(json.dumps(summary))
        prior_receipt["lanes"][lane] = {
            "ledger": str(base_path),
            "ledgerSha256": promoter.sha256(base_path),
            "summarySha256": promoter.sha256(summary_path),
        }
        result = results / f"{lane}.json"
        raw_report = report()
        result.write_text(json.dumps(raw_report))
        strict_path = results / f"{lane}.strict.json"
        strict = {
            "mode": lane,
            "exitCode": 0,
            "scope": STRICT_SCOPE,
            "upstreamCommit": COMMIT,
            "upstreamSourceSha256": SOURCE_SHA256,
            "inputs": promoter.HARNESS_PINS,
            "candidateProfile": PROFILE if lane == "candidate" else None,
            "candidateArtifactSha256": promoter.ARTIFACT_SHA256
            if lane == "candidate"
            else None,
            "result": str(result),
            "resultSha256": promoter.sha256(result),
            "selected": selected_tests(raw_report),
        }
        strict_path.write_text(json.dumps(strict))
        lanes[lane] = {
            "baseSha256": promoter.sha256(base_path),
            "strict": strict_path,
            "strictSha256": promoter.sha256(strict_path),
            "oldOutcome": target["outcome"],
        }
        paths[f"{lane}_strict"] = strict_path
        paths[f"{lane}_base"] = base_path
        paths[f"{lane}_result"] = result
    receipt = base / "promotion-receipt.json"
    receipt.write_text(json.dumps(prior_receipt))
    monkeypatch.setattr(promoter, "BASE_RECEIPT_SHA256", promoter.sha256(receipt))
    monkeypatch.setattr(promoter, "LANES", lanes)

    def fake_verify(path: Path) -> dict:
        return json.loads(path.read_text())

    monkeypatch.setattr(promoter, "verify_strict", fake_verify)
    return paths


def test_synthetic_pair_refreshes_only_one_row_per_lane(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    paths = synthetic_pair(tmp_path, monkeypatch)

    # Act
    files = promoter.prepare_pair()

    # Assert
    for lane in ("candidate", "oracle"):
        old = paths[f"{lane}_base"].read_bytes().splitlines(keepends=True)
        new = files[f"{lane}.jsonl"].splitlines(keepends=True)
        assert len(old) == len(new) == 4
        assert [
            index
            for index, pair in enumerate(zip(old, new, strict=True))
            if pair[0] != pair[1]
        ] == [1]
        before, after = json.loads(old[1]), json.loads(new[1])
        assert after["sourceClassification"] == before["sourceClassification"]
        assert after["classificationReasons"] == before["classificationReasons"]
        assert after["specId"] == before["specId"]
        assert after["outcome"] == "expected"
        assert after["evidenceScope"] == promoter.SCOPE
        assert "error" not in after
    receipt = json.loads(files["promotion-receipt.json"])
    assert receipt["targetZeroBasedIndex"] == 1
    assert receipt["strictVerifierSha256"] == promoter.VERIFIER_SHA256
    assert receipt["publicApiParityEvidence"] is False
    assert receipt["lanes"]["oracle"]["oldOutcome"] == "expected"


def test_atomic_published_pair_verifies_byte_for_byte(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    synthetic_pair(tmp_path, monkeypatch)
    files = promoter.prepare_pair()

    # Act
    promoter.publish(files)
    receipt = promoter.verify_existing()

    # Assert
    assert receipt["targetSpecId"] == SPEC_ID
    assert {path.name for path in promoter.OUTPUT.iterdir()} == set(files)


def test_published_pair_rejects_byte_tamper(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    synthetic_pair(tmp_path, monkeypatch)
    promoter.publish(promoter.prepare_pair())
    candidate = promoter.OUTPUT / "candidate.jsonl"
    candidate.write_bytes(candidate.read_bytes() + b"\n")

    # Act / Assert
    with pytest.raises(ValueError, match="immutable output drifted"):
        promoter.verify_existing()


def test_strict_sidecar_hash_drift_is_rejected_before_verification(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    paths = synthetic_pair(tmp_path, monkeypatch)
    paths["candidate_strict"].write_text(paths["candidate_strict"].read_text() + " ")

    # Act / Assert
    with pytest.raises(ValueError, match="SHA-256 drift"):
        promoter.prepare_pair()


def test_unpinned_strict_sidecar_is_rejected(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    synthetic_pair(tmp_path, monkeypatch)
    promoter.LANES["candidate"]["strictSha256"] = ""

    # Act / Assert
    with pytest.raises(ValueError, match="unfilled or invalid SHA-256 pin"):
        promoter.prepare_pair()


def test_verifier_source_drift_is_rejected(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    synthetic_pair(tmp_path, monkeypatch)
    monkeypatch.setattr(promoter, "VERIFIER_SHA256", "0" * 64)

    # Act / Assert
    with pytest.raises(ValueError, match="SHA-256 drift"):
        promoter.prepare_pair()


def test_candidate_failure_cannot_be_relabelled_as_pass(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    paths = synthetic_pair(tmp_path, monkeypatch)
    result = paths["candidate_result"]
    raw = json.loads(result.read_text())
    spec = raw["suites"][0]["specs"][0]
    spec["ok"] = False
    test = spec["tests"][0]
    test["status"] = "unexpected"
    test["results"][0].update({"status": "failed", "errors": [{"message": "wrong UA"}]})
    raw["stats"].update({"expected": 0, "unexpected": 1})
    result.write_text(json.dumps(raw))
    strict_path = paths["candidate_strict"]
    strict = json.loads(strict_path.read_text())
    strict["resultSha256"] = promoter.sha256(result)
    strict_path.write_text(json.dumps(strict))
    promoter.LANES["candidate"]["strictSha256"] = promoter.sha256(strict_path)

    # Act / Assert
    with pytest.raises(ValueError, match="official User-Agent assertion did not pass"):
        promoter.prepare_pair()


def test_baseline_id_reorder_is_rejected(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    paths = synthetic_pair(tmp_path, monkeypatch)
    baseline = paths["candidate_base"]
    lines = baseline.read_bytes().splitlines(keepends=True)
    lines[0], lines[2] = lines[2], lines[0]
    baseline.write_bytes(b"".join(lines))

    # Act / Assert
    with pytest.raises(ValueError, match="inventory identity/classification drift"):
        promoter.prepare_lane("candidate")


def test_artifact_sha_mismatch_is_rejected(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    synthetic_pair(tmp_path, monkeypatch)
    monkeypatch.setattr(promoter, "ARTIFACT_SHA256", "0" * 64)

    # Act / Assert
    with pytest.raises(ValueError, match="selected opt-in artifact"):
        promoter.prepare_pair()
