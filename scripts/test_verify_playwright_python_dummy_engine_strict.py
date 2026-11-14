"""AAA negative checks for the versioned non-Chromium strict evidence gate."""

from __future__ import annotations

import copy
import json
from pathlib import Path

import pytest

from playwright_python_official_provenance import file_sha256
from summarize_playwright_python_official_results import _verified_generic_result
from verify_playwright_python_dummy_engine_strict import (
    ASSET,
    KIND,
    POLICY,
    ROOT,
    SCOPE,
    TEST_NAME,
    StrictEvidenceError,
    _policy,
    _receipts,
    _report,
    _tooling,
)


def _write(path: Path, content: str) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content)
    return path


def _policy_data() -> dict:
    return json.loads(POLICY.read_text())


def test_exact_new_policy_accepts_pinned_tooling() -> None:
    # Arrange
    policy = _policy_data()

    # Act
    guard = _tooling(_policy(policy), {"toolingSources": policy["captureSourceSha256"]}, ROOT)

    # Assert
    assert guard.name == "dummy_engine_guard_strict"


def test_unready_policy_cannot_verify_a_run() -> None:
    # Arrange
    policy = _policy_data()
    policy["captureReady"] = False

    # Act
    with pytest.raises(StrictEvidenceError, match="not ready"):
        _policy(policy)

    # Assert
    assert policy["captureSourceSha256"]


def test_changed_capture_source_hash_is_rejected() -> None:
    # Arrange
    policy = copy.deepcopy(_policy_data())
    path = "packages/pwright-python/tests/official/dummy_engine_guard_strict/node"
    policy["captureSourceSha256"][path] = "0" * 64

    # Act
    with pytest.raises(StrictEvidenceError, match="SHA-256 differs"):
        _tooling(_policy(policy), {"toolingSources": policy["captureSourceSha256"]}, ROOT)

    # Assert
    assert policy["captureSourceSha256"][path] != _policy_data()["captureSourceSha256"][path]


@pytest.mark.parametrize(
    "mode,outcome",
    [("candidate", "passed"), ("oracle", "failed")],
)
def test_wrong_official_outcome_is_rejected(tmp_path: Path, mode: str, outcome: str) -> None:
    # Arrange
    failure = (
        '<failure message="UnsupportedError: launch is only available for Chromium" />'
        if outcome == "failed"
        else ""
    )
    report = _write(
        tmp_path / "one.xml",
        f'<testsuites><testsuite tests="1" errors="0" skipped="0" '
        f'failures="{1 if failure else 0}">'
        f'<testcase classname="tests.async.test_launcher" '
        f'name="{TEST_NAME}[firefox]">{failure}</testcase></testsuite></testsuites>',
    )
    payload = {
        "mode": mode,
        "api": "async",
        "browser": "firefox",
        "result": str(report),
        "resultSha256": file_sha256(report),
        "exitCode": 1 if failure else 0,
    }

    # Act
    with pytest.raises(StrictEvidenceError, match="did not (fail|pass)"):
        _report(payload, _policy_data())

    # Assert
    assert payload["exitCode"] == (0 if outcome == "passed" else 1)


def _receipt_sample(tmp_path: Path, mode: str) -> tuple[dict, Path]:
    policy = _policy_data()
    result = tmp_path / "one.xml"
    asset = tmp_path / ASSET
    asset.parent.mkdir(parents=True)
    asset.write_text("#!/usr/bin/env node\n\nprocess.exit(1);\n")
    process = result.with_suffix(".dummy-process.jsonl")
    public = result.with_suffix(".public-launch.jsonl")
    nonce = "0123456789abcdef0123456789abcdef"
    common = {
        "api": "async",
        "mode": mode,
        "browser": "firefox",
        "runNonce": nonce,
        "executablePath": str(asset),
        "assetSha256": policy["upstreamSources"][ASSET],
    }
    node = tmp_path / "node"
    _write(node, "pinned node")
    row = {
        "event": "dummy-node-exited",
        **common,
        "exitCode": 1,
        "childPid": 31337,
        "node": str(node),
        "nodeSha256": policy["systemNodeSha256"],
        "argv": [str(node), str(asset)],
    }
    process.write_text(json.dumps(row) + "\n" if mode == "oracle" else "")
    failed = {
        "event": "public-launch-failed",
        **common,
        "errorClass": (
            policy["oracleErrorClass"]["firefox"]
            if mode == "oracle"
            else policy["candidateErrorClass"]
        ),
        "errorMessage": (
            "Browser process exited with code 1"
            if mode == "oracle"
            else policy["candidateErrorMessage"]
        ),
        "processReceiptSha256": file_sha256(process) if mode == "oracle" else None,
    }
    public.write_text(
        json.dumps({"event": "public-launch-invoked", **common}) + "\n" + json.dumps(failed) + "\n"
    )
    payload = {
        "mode": mode,
        "api": "async",
        "browser": "firefox",
        "runNonce": nonce,
        "result": str(result),
        "publicReceipt": str(public),
        "publicReceiptSha256": file_sha256(public),
        "processReceipt": str(process),
        "processReceiptSha256": file_sha256(process),
        "systemNode": str(node),
    }
    return payload, asset


@pytest.mark.parametrize("mode", ["candidate", "oracle"])
def test_receipt_contract_accepts_exact_lane_shape(tmp_path: Path, mode: str) -> None:
    # Arrange
    payload, asset = _receipt_sample(tmp_path, mode)

    # Act
    _receipts(payload, _policy_data(), asset)

    # Assert
    assert Path(payload["publicReceipt"]).is_file()


def test_candidate_receipt_rejects_even_a_real_dummy_spawn(tmp_path: Path) -> None:
    # Arrange
    payload, asset = _receipt_sample(tmp_path, "candidate")
    process = Path(payload["processReceipt"])
    process.write_text('{"event":"dummy-node-exited"}\n')
    payload["processReceiptSha256"] = file_sha256(process)

    # Act
    with pytest.raises(StrictEvidenceError, match="before spawning"):
        _receipts(payload, _policy_data(), asset)

    # Assert
    assert process.stat().st_size > 0


def test_oracle_receipt_rejects_missing_process(tmp_path: Path) -> None:
    # Arrange
    payload, asset = _receipt_sample(tmp_path, "oracle")
    process = Path(payload["processReceipt"])
    process.write_text("")
    payload["processReceiptSha256"] = file_sha256(process)

    # Act
    with pytest.raises(StrictEvidenceError, match="process receipt"):
        _receipts(payload, _policy_data(), asset)

    # Assert
    assert process.stat().st_size == 0


def test_webkit_oracle_requires_target_closed_error(tmp_path: Path) -> None:
    # Arrange
    payload, asset = _receipt_sample(tmp_path, "oracle")
    payload["browser"] = "webkit"
    public = Path(payload["publicReceipt"])
    process = Path(payload["processReceipt"])
    process_row = json.loads(process.read_text())
    process_row["browser"] = "webkit"
    process.write_text(json.dumps(process_row) + "\n")
    payload["processReceiptSha256"] = file_sha256(process)
    rows = [json.loads(line) for line in public.read_text().splitlines()]
    for row in rows:
        row["browser"] = "webkit"
    rows[1]["processReceiptSha256"] = payload["processReceiptSha256"]
    public.write_text("".join(json.dumps(row) + "\n" for row in rows))
    payload["publicReceiptSha256"] = file_sha256(public)

    # Act
    with pytest.raises(StrictEvidenceError, match="oracle lacks pinned"):
        _receipts(payload, _policy_data(), asset)

    # Assert
    assert rows[1]["errorClass"] == "Error"


def test_generic_ledger_gate_refuses_new_engine_sidecar(tmp_path: Path) -> None:
    # Arrange
    report = _write(tmp_path / "one.xml", "<testsuites />\n")
    sidecar = report.with_suffix(".provenance.json")
    _write(
        sidecar,
        json.dumps(
            {
                "schema": 2,
                "kind": KIND,
                "evidenceScope": SCOPE,
                "mode": "candidate",
                "result": str(report),
                "resultSha256": file_sha256(report),
                "collectOnly": False,
                "bootstrapOnly": False,
            }
        ),
    )

    # Act
    with pytest.raises(SystemExit, match="provenance verification failed"):
        _verified_generic_result(report, "candidate")

    # Assert
    assert sidecar.is_file()
