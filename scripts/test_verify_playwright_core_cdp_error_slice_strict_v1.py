"""AAA negative controls for the pinned browser-free Core CDP error shard."""

from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path

import pytest
import run_playwright_core_cdp_error_slice_strict_v1 as capture
import verify_playwright_core_cdp_error_slice_strict_v1 as verifier

HOST = "fixture.tail8f3b66.ts.net"


def _synthetic(tmp_path: Path, monkeypatch: pytest.MonkeyPatch, mode: str = "oracle") -> Path:
    # Arrange
    result = tmp_path / f"{mode}.json"
    specs = []
    for spec_id, (line, title) in capture.CASES.items():
        specs.append(
            {
                "id": spec_id,
                "file": capture.REPORT_FILE,
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
                                "workerIndex": 0,
                                "status": "passed",
                                "errors": [],
                            }
                        ],
                    }
                ],
            }
        )
    report = {
        "errors": [],
        "suites": [{"specs": specs, "suites": []}],
        "stats": {"expected": 3, "unexpected": 0, "skipped": 0, "flaky": 0},
    }
    result.write_text(json.dumps(report))
    provenance = {
        "mode": mode,
        "suite": capture.SUITE,
        "project": "chromium-library",
        "workers": 1,
        "filters": [f"{capture.SOURCE}:{line}" for line in capture.SELECTED_LINES],
        "fixtureHost": HOST,
        "resultSha256": capture.sha256(result),
        "exitCode": 0,
        "candidate": {"profile": capture.PROFILE} if mode == "candidate" else None,
        "candidateProfile": capture.PROFILE if mode == "candidate" else None,
        "candidateArtifactSha256": capture.ARTIFACT_SHA256 if mode == "candidate" else None,
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
                "adaptedSha256": capture.adapted_source_sha256(HOST),
                "fixtureHost": HOST,
                "replacements": {"websocket": 1, "serverPrefixCases": 0},
            }
        )
        + "\n"
    )
    log = result.with_suffix(".log")
    log.write_text("[pwright-candidate] selected\n" if mode == "candidate" else "oracle selected\n")
    inputs = {
        str(path.relative_to(capture.ROOT)): capture.sha256(path)
        for path in (
            capture.MANIFEST,
            capture.PRELOAD,
            capture.COMMON_PRELOAD,
            capture.ROOT / "scripts/run_playwright_core_cdp_error_slice_strict_v1.py",
        )
    }
    strict = {
        "schema": 2,
        "scope": capture.SCOPE,
        "mode": mode,
        "upstreamCommit": capture.COMMIT,
        "upstreamSourceSha256": capture.SOURCE_SHA256,
        "serverFixtureSha256": capture.SERVER_FIXTURE_SHA256,
        "commonPreloadSha256": capture.COMMON_PRELOAD_SHA256,
        "inputs": inputs,
        "candidateProfile": capture.PROFILE if mode == "candidate" else None,
        "candidateArtifactSha256": capture.ARTIFACT_SHA256 if mode == "candidate" else None,
        "result": str(result),
        "resultSha256": capture.sha256(result),
        "provenanceSha256": capture.sha256(provenance_path),
        "guardReceiptSha256": capture.sha256(receipt),
        "logSha256": capture.sha256(log),
        "exitCode": 0,
        "selected": capture.selected_tests(report),
    }
    sidecar = result.with_suffix(".strict.json")
    sidecar.write_text(json.dumps(strict))

    def fake_generic(raw: object, **_kwargs: object) -> dict:
        assert raw == json.loads(provenance_path.read_text())
        assert isinstance(raw, dict)
        return raw

    monkeypatch.setattr(verifier, "verify_provenance", fake_generic)
    return sidecar


@pytest.mark.parametrize("mode", ["oracle", "candidate"])
def test_accepts_three_exact_official_cases(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, mode: str
) -> None:
    # Arrange
    sidecar = _synthetic(tmp_path, monkeypatch, mode)

    # Act
    strict = verifier.verify_strict(sidecar)

    # Assert
    assert [item["line"] for item in strict["selected"]] == [399, 411, 572]


def test_rejects_selected_outcome_forged_as_failure(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    sidecar = _synthetic(tmp_path, monkeypatch)
    strict = json.loads(sidecar.read_text())
    strict["selected"][0]["status"] = "unexpected"
    sidecar.write_text(json.dumps(strict))

    # Act
    with pytest.raises(AssertionError):
        verifier.verify_strict(sidecar)

    # Assert
    assert strict["selected"][0]["line"] == 399


def test_rejects_extra_official_case_even_when_rehashed(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    sidecar = _synthetic(tmp_path, monkeypatch)
    strict = json.loads(sidecar.read_text())
    result = Path(strict["result"])
    report = json.loads(result.read_text())
    report["suites"][0]["specs"].append(report["suites"][0]["specs"][0])
    result.write_text(json.dumps(report))
    strict["resultSha256"] = capture.sha256(result)
    provenance = result.with_suffix(".provenance.json")
    provenance_data = json.loads(provenance.read_text())
    provenance_data["resultSha256"] = strict["resultSha256"]
    provenance.write_text(json.dumps(provenance_data))
    strict["provenanceSha256"] = capture.sha256(provenance)
    sidecar.write_text(json.dumps(strict))

    # Act
    with pytest.raises(AssertionError):
        verifier.verify_strict(sidecar)

    # Assert
    assert len(report["suites"][0]["specs"]) == 4


def test_rejects_guard_receipt_host_tamper_even_when_rehashed(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    sidecar = _synthetic(tmp_path, monkeypatch)
    strict = json.loads(sidecar.read_text())
    receipt = Path(strict["result"]).with_suffix(".guard-receipt.jsonl")
    record = json.loads(receipt.read_text())
    record["fixtureHost"] = "other.tail8f3b66.ts.net"
    receipt.write_text(json.dumps(record) + "\n")
    strict["guardReceiptSha256"] = capture.sha256(receipt)
    sidecar.write_text(json.dumps(strict))

    # Act
    with pytest.raises(AssertionError):
        verifier.verify_strict(sidecar)

    # Assert
    assert record["fixtureHost"] != HOST


def test_rejects_claim_that_server_prefix_assertions_were_rewritten(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    sidecar = _synthetic(tmp_path, monkeypatch)
    strict = json.loads(sidecar.read_text())
    receipt = Path(strict["result"]).with_suffix(".guard-receipt.jsonl")
    record = json.loads(receipt.read_text())
    record["replacements"]["serverPrefixCases"] = 2
    receipt.write_text(json.dumps(record) + "\n")
    strict["guardReceiptSha256"] = capture.sha256(receipt)
    sidecar.write_text(json.dumps(strict))

    # Act
    with pytest.raises(AssertionError):
        verifier.verify_strict(sidecar)

    # Assert
    assert record["replacements"]["serverPrefixCases"] == 2


def test_preload_rewrites_only_same_port_ws_host(tmp_path: Path) -> None:
    # Arrange
    receipt = tmp_path / "preload-receipt.jsonl"
    receipt.write_text("")
    environment = {
        **os.environ,
        "NODE_OPTIONS": f"--require={capture.PRELOAD}",
        "PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM": str(capture.UPSTREAM),
        "PWRIGHT_OFFICIAL_SERVER_HOST": HOST,
        "PWRIGHT_CORE_CDP_ERROR_SLICE_GUARD_RECEIPT": str(receipt),
    }
    expression = (
        "const fs=require('node:fs');const crypto=require('node:crypto');"
        "const source=fs.readFileSync(process.argv[1],'utf8');"
        "process.stdout.write(JSON.stringify({"
        "sha:crypto.createHash('sha256').update(source).digest('hex'),"
        "prefix:source.includes('Unexpected status 404 when connecting to "
        "${server.PREFIX}/json/version/'),"
        "ws:source.includes('ws://fixture.tail8f3b66.ts.net:${server.PORT}/ws')"
        "}));"
    )

    # Act
    completed = subprocess.run(
        ["node", "-e", expression, str(capture.UPSTREAM / capture.SOURCE)],
        check=True,
        capture_output=True,
        text=True,
        env=environment,
    )
    proof = json.loads(completed.stdout)

    # Assert
    assert proof == {"sha": capture.adapted_source_sha256(HOST), "prefix": True, "ws": True}
    record = json.loads(receipt.read_text())
    assert record["replacements"] == {"websocket": 1, "serverPrefixCases": 0}


def test_preload_rejects_upstream_source_drift(tmp_path: Path) -> None:
    # Arrange
    shadow = tmp_path / "upstream"
    source = shadow / capture.SOURCE
    source.parent.mkdir(parents=True)
    source.write_text((capture.UPSTREAM / capture.SOURCE).read_text() + "\n")
    receipt = tmp_path / "preload-receipt.jsonl"
    receipt.write_text("")
    environment = {
        **os.environ,
        "NODE_OPTIONS": f"--require={capture.PRELOAD}",
        "PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM": str(shadow),
        "PWRIGHT_OFFICIAL_SERVER_HOST": HOST,
        "PWRIGHT_CORE_CDP_ERROR_SLICE_GUARD_RECEIPT": str(receipt),
    }

    # Act
    completed = subprocess.run(
        ["node", "-e", "require('node:fs').readFileSync(process.argv[1], 'utf8')", str(source)],
        check=False,
        capture_output=True,
        text=True,
        env=environment,
    )

    # Assert
    assert completed.returncode != 0
    assert "source drifted" in completed.stderr
    assert receipt.read_text() == ""
