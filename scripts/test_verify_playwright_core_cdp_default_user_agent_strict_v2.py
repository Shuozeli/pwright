"""Synthetic negative controls for the official Core CDP default-UA v2 lane."""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

import pytest
import run_playwright_core_cdp_default_user_agent_strict_v2 as capture
import verify_playwright_core_cdp_default_user_agent_strict_v2 as verifier

HOST = "fixture.tail8f3b66.ts.net"


@pytest.fixture(autouse=True)
def pinned_upstream_checkout(monkeypatch: pytest.MonkeyPatch) -> None:
    """Use the pinned local clone or the CI checkout of the same commit."""
    upstream = capture.UPSTREAM
    if not (upstream / capture.SOURCE).is_file():
        upstream = capture.ROOT / ".work/core455-upstream"
    assert (upstream / capture.SOURCE).is_file()
    monkeypatch.setattr(capture, "UPSTREAM", upstream)
    monkeypatch.setattr(verifier, "UPSTREAM", upstream)


def create_synthetic_oracle(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    # Arrange
    result = tmp_path / "oracle.json"
    report = {
        "errors": [],
        "suites": [
            {
                "specs": [
                    {
                        "id": capture.SPEC_ID,
                        "file": capture.REPORT_FILE,
                        "line": 299,
                        "column": 5,
                        "title": capture.TITLE,
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
    result.write_text(json.dumps(report))
    provenance = {
        "mode": "oracle",
        "suite": capture.SUITE,
        "project": "chromium-library",
        "workers": 1,
        "filters": [f"{capture.SOURCE}:299"],
        "fixtureHost": HOST,
        "resultSha256": capture.sha256(result),
        "exitCode": 0,
        "candidate": None,
    }
    provenance_path = result.with_suffix(".provenance.json")
    provenance_path.write_text(json.dumps(provenance))
    receipt = result.with_suffix(".guard-receipt.jsonl")
    receipt.write_text(
        json.dumps(
            {
                "pid": 12345,
                "workerIndex": "0",
                "sourceSha256": capture.SOURCE_SHA256,
                "adaptedSha256": verifier.adapted_source_sha256(HOST),
                "fixtureHost": HOST,
                "replacements": {"websocket": 1},
            }
        )
        + "\n"
    )
    log = result.with_suffix(".log")
    log.write_text("official oracle test passed\n")
    selected = capture.selected_tests(report)
    inputs = {
        str(path.relative_to(capture.ROOT)): capture.sha256(path)
        for path in (
            capture.MANIFEST,
            capture.PRELOAD,
            capture.ROOT
            / "scripts/run_playwright_core_cdp_default_user_agent_strict_v2.py",
        )
    }
    strict = {
        "schema": 2,
        "scope": capture.SCOPE,
        "mode": "oracle",
        "upstreamCommit": capture.COMMIT,
        "upstreamSourceSha256": capture.SOURCE_SHA256,
        "serverFixtureSha256": capture.SERVER_FIXTURE_SHA256,
        "inputs": inputs,
        "candidateProfile": None,
        "candidateArtifactSha256": None,
        "result": str(result),
        "resultSha256": capture.sha256(result),
        "provenanceSha256": capture.sha256(provenance_path),
        "guardReceiptSha256": capture.sha256(receipt),
        "logSha256": capture.sha256(log),
        "exitCode": 0,
        "selected": selected,
    }
    sidecar = result.with_suffix(".strict.json")
    sidecar.write_text(json.dumps(strict))

    def fake_generic(raw: object, **_kwargs: object) -> dict:
        assert raw == json.loads(provenance_path.read_text())
        return raw  # type: ignore[return-value]

    monkeypatch.setattr(verifier, "verify_provenance", fake_generic)
    return sidecar


def test_accepts_synthetic_oracle_with_exact_official_test(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    sidecar = create_synthetic_oracle(tmp_path, monkeypatch)

    # Act
    strict = verifier.verify_strict(sidecar)

    # Assert
    assert strict["selected"][0]["specId"] == capture.SPEC_ID
    assert strict["selected"][0]["attempts"] == [{"retry": 0, "status": "passed"}]


def test_rejects_fabricated_selected_pass(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    sidecar = create_synthetic_oracle(tmp_path, monkeypatch)
    strict = json.loads(sidecar.read_text())
    strict["selected"][0]["attempts"] = [{"retry": 0, "status": "failed"}]
    sidecar.write_text(json.dumps(strict))

    # Act / Assert
    with pytest.raises(AssertionError):
        verifier.verify_strict(sidecar)


def test_rejects_failure_report_relabelled_as_pass_even_when_rehashed(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    sidecar = create_synthetic_oracle(tmp_path, monkeypatch)
    strict = json.loads(sidecar.read_text())
    result = Path(strict["result"])
    report = json.loads(result.read_text())
    spec = report["suites"][0]["specs"][0]
    spec["ok"] = False
    spec["tests"][0]["status"] = "unexpected"
    spec["tests"][0]["results"][0]["status"] = "failed"
    spec["tests"][0]["results"][0]["errors"] = [{"message": "User-Agent mismatch"}]
    report["stats"].update({"expected": 0, "unexpected": 1})
    result.write_text(json.dumps(report))
    strict["resultSha256"] = capture.sha256(result)
    strict["exitCode"] = 1
    provenance_path = result.with_suffix(".provenance.json")
    provenance = json.loads(provenance_path.read_text())
    provenance["resultSha256"] = strict["resultSha256"]
    provenance["exitCode"] = 1
    provenance_path.write_text(json.dumps(provenance))
    strict["provenanceSha256"] = capture.sha256(provenance_path)
    sidecar.write_text(json.dumps(strict))

    # Act / Assert
    with pytest.raises(AssertionError):
        verifier.verify_strict(sidecar)


def test_rejects_additional_official_test_even_when_result_rehashed(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    sidecar = create_synthetic_oracle(tmp_path, monkeypatch)
    strict = json.loads(sidecar.read_text())
    result = Path(strict["result"])
    report = json.loads(result.read_text())
    report["suites"][0]["specs"].append(report["suites"][0]["specs"][0])
    result.write_text(json.dumps(report))
    strict["resultSha256"] = capture.sha256(result)
    provenance_path = result.with_suffix(".provenance.json")
    provenance = json.loads(provenance_path.read_text())
    provenance["resultSha256"] = strict["resultSha256"]
    provenance_path.write_text(json.dumps(provenance))
    strict["provenanceSha256"] = capture.sha256(provenance_path)
    sidecar.write_text(json.dumps(strict))

    # Act / Assert
    with pytest.raises(AssertionError):
        verifier.verify_strict(sidecar)


def test_rejects_tampered_receipt_even_when_rehashed(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    sidecar = create_synthetic_oracle(tmp_path, monkeypatch)
    strict = json.loads(sidecar.read_text())
    receipt_path = Path(strict["result"]).with_suffix(".guard-receipt.jsonl")
    receipt = json.loads(receipt_path.read_text())
    receipt["adaptedSha256"] = "0" * 64
    receipt_path.write_text(json.dumps(receipt) + "\n")
    strict["guardReceiptSha256"] = capture.sha256(receipt_path)
    sidecar.write_text(json.dumps(strict))

    # Act / Assert
    with pytest.raises(AssertionError):
        verifier.verify_strict(sidecar)


@pytest.mark.parametrize(
    "field,value",
    [
        ("candidateProfile", "default"),
        ("candidateArtifactSha256", "0" * 64),
        ("upstreamSourceSha256", "0" * 64),
        ("logSha256", "0" * 64),
    ],
)
def test_rejects_tampered_attestation(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    field: str,
    value: str,
) -> None:
    # Arrange
    sidecar = create_synthetic_oracle(tmp_path, monkeypatch)
    strict = json.loads(sidecar.read_text())
    strict[field] = value
    sidecar.write_text(json.dumps(strict))

    # Act / Assert
    with pytest.raises(AssertionError):
        verifier.verify_strict(sidecar)


def test_rejects_inherited_runtime_override_before_capture(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setenv("NODE_OPTIONS", "--require=unexpected.cjs")
    monkeypatch.setattr(
        sys, "argv", ["capture", "oracle", "--result", str(tmp_path / "oracle.json")]
    )

    # Act / Assert
    with pytest.raises(SystemExit):
        capture.main()
    assert not (tmp_path / "oracle.json").exists()


def test_candidate_requires_pinned_artifact_digest_before_capture(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    for key in (
        "NODE_OPTIONS",
        "PWTEST_MODE",
        "PWPAGE_IMPL",
        "PWTEST_USE_BIDI_EXPECTATIONS",
        "PWRIGHT_CANDIDATE_CDP_ENDPOINT",
        "PWRIGHT_ORACLE_CDP_ENDPOINT",
        "PWRIGHT_NATIVE_PROFILE",
        "PWRIGHT_NODE_CANDIDATE",
        "PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM",
        "PWRIGHT_OFFICIAL_SERVER_HOST",
        "PWRIGHT_CORE_CDP_DEFAULT_UA_V2_GUARD_RECEIPT",
    ):
        monkeypatch.delenv(key, raising=False)
    monkeypatch.setattr(
        sys,
        "argv",
        ["capture", "candidate", "--result", str(tmp_path / "candidate.json")],
    )

    # Act / Assert
    with pytest.raises(SystemExit):
        capture.main()
    assert not (tmp_path / "candidate.json").exists()


def test_preload_only_changes_target_test_endpoint(tmp_path: Path) -> None:
    # Arrange
    receipt = tmp_path / "receipt.jsonl"
    receipt.write_text("")
    env = os.environ.copy()
    env.update(
        {
            "PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM": str(capture.UPSTREAM),
            "PWRIGHT_OFFICIAL_SERVER_HOST": HOST,
            "PWRIGHT_CORE_CDP_DEFAULT_UA_V2_GUARD_RECEIPT": str(receipt),
            "TEST_WORKER_INDEX": "0",
        }
    )
    code = (
        "const fs=require('node:fs');const crypto=require('node:crypto');"
        "const source=fs.readFileSync(process.argv[1],'utf8');"
        "process.stdout.write(crypto.createHash('sha256').update(source).digest('hex'));"
    )

    # Act
    result = subprocess.run(
        [
            "node",
            f"--require={capture.PRELOAD}",
            "-e",
            code,
            str(capture.UPSTREAM / capture.SOURCE),
        ],
        env=env,
        capture_output=True,
        text=True,
        check=True,
    )

    # Assert
    assert result.stdout == verifier.adapted_source_sha256(HOST)
    records = [json.loads(line) for line in receipt.read_text().splitlines()]
    assert len(records) == 1
    assert records[0]["replacements"] == {"websocket": 1}
    assert records[0]["adaptedSha256"] == result.stdout
