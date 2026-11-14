"""Synthetic positive and negative controls for post-fix dummy-engine evidence."""

from __future__ import annotations

import copy
import hashlib
import json
import shutil
import subprocess
import zipfile
from pathlib import Path

import pytest
from run_playwright_python_dummy_engine_strict_v2 import _candidate_wheel_snapshot
from verify_playwright_python_dummy_engine_strict_v2 import (
    ASSET,
    POLICY,
    ROOT,
    TEST_NAME,
    StrictEvidenceError,
    _policy,
    _receipts,
    _report,
    _tooling,
    _wheel_matches_install,
    file_sha256,
)


def _unready_policy() -> dict:
    policy = json.loads(POLICY.read_text())
    policy["captureReady"] = False
    policy["captureSourceSha256"] = {}
    policy["candidateWheelSha256"] = None
    policy["candidatePackageTreeSha256"] = None
    policy["candidateNativeSha256"] = None
    policy["candidateRuntimeLibraries"] = None
    return policy


def _ready_policy() -> dict:
    policy = copy.deepcopy(_unready_policy())
    policy["captureReady"] = True
    policy["captureSourceSha256"] = {
        relative: file_sha256(ROOT / relative) for relative in policy["captureSourcePaths"]
    }
    policy["candidateWheelSha256"] = "1" * 64
    policy["candidatePackageTreeSha256"] = "2" * 64
    policy["candidateNativeSha256"] = "3" * 64
    policy["candidateRuntimeLibraries"] = {"libfake.dylib": "4" * 64}
    return policy


def test_unready_policy_cannot_authorize_capture() -> None:
    # Arrange
    policy = _unready_policy()

    # Act
    with pytest.raises(StrictEvidenceError, match="not ready"):
        _policy(policy)

    # Assert
    assert policy["captureReady"] is False
    assert policy["candidateWheelSha256"] is None


def test_ready_policy_pins_exact_new_guard_and_runner_sources() -> None:
    # Arrange
    policy = _ready_policy()

    # Act
    guard = _tooling(_policy(policy), {"toolingSources": policy["captureSourceSha256"]}, ROOT)

    # Assert
    assert guard.name == "dummy_engine_guard_strict_v2"


def test_changed_capture_source_is_rejected() -> None:
    # Arrange
    policy = _ready_policy()
    path = "packages/pwright-python/tests/official/dummy_engine_guard_strict_v2/node"
    policy["captureSourceSha256"][path] = "0" * 64

    # Act
    with pytest.raises(StrictEvidenceError, match="SHA-256 differs"):
        _tooling(_policy(policy), {"toolingSources": policy["captureSourceSha256"]}, ROOT)

    # Assert
    assert policy["captureSourceSha256"][path] != file_sha256(ROOT / path)


def test_policy_refuses_relabelled_test_id() -> None:
    # Arrange
    policy = _ready_policy()
    policy["tests"]["async"]["firefox"] = "tests/async/test_launcher.py::test_other[firefox]"

    # Act
    with pytest.raises(StrictEvidenceError, match="four exact official cases"):
        _policy(policy)

    # Assert
    assert TEST_NAME not in policy["tests"]["async"]["firefox"]


def _junit(tmp_path: Path, *, api: str, browser: str, failure: str = "") -> Path:
    report = tmp_path / "one.xml"
    report.write_text(
        f'<testsuites><testsuite tests="1" errors="0" skipped="0" '
        f'failures="{1 if failure else 0}">'
        f'<testcase classname="tests.{api}.test_launcher" name="{TEST_NAME}[{browser}]">'
        f"{failure}</testcase></testsuite></testsuites>"
    )
    return report


@pytest.mark.parametrize("mode", ["candidate", "oracle"])
@pytest.mark.parametrize("browser", ["firefox", "webkit"])
def test_both_lanes_require_one_passing_unchanged_official_case(
    tmp_path: Path, mode: str, browser: str
) -> None:
    # Arrange
    report = _junit(tmp_path, api="async", browser=browser)
    payload = {
        "mode": mode,
        "api": "async",
        "browser": browser,
        "result": str(report),
        "resultSha256": file_sha256(report),
        "exitCode": 0,
    }

    # Act
    _report(payload)

    # Assert
    assert report.is_file()


@pytest.mark.parametrize("status", ["failure", "skip", "wrong-exit"])
def test_failed_skipped_or_nonzero_worker_is_rejected(tmp_path: Path, status: str) -> None:
    # Arrange
    failure = '<failure message="UnsupportedError" />' if status == "failure" else ""
    report = _junit(tmp_path, api="sync", browser="firefox", failure=failure)
    if status == "skip":
        report.write_text(report.read_text().replace("</testcase>", "<skipped /></testcase>"))
    payload = {
        "mode": "candidate",
        "api": "sync",
        "browser": "firefox",
        "result": str(report),
        "resultSha256": file_sha256(report),
        "exitCode": 1 if status == "wrong-exit" else 0,
    }

    # Act
    with pytest.raises(StrictEvidenceError, match="did not pass"):
        _report(payload)

    # Assert
    assert payload["browser"] == "firefox"


def _receipt_sample(tmp_path: Path, mode: str, browser: str) -> tuple[dict, Path]:
    policy = _ready_policy()
    result = tmp_path / "one.xml"
    asset = tmp_path / ASSET
    asset.parent.mkdir(parents=True)
    asset.write_text("#!/usr/bin/env node\n\nprocess.exit(1);\n")
    public = result.with_suffix(".public-launch.jsonl")
    process = result.with_suffix(".dummy-process.jsonl")
    node = tmp_path / "node"
    node.write_text("pinned node")
    common = {
        "api": "async",
        "mode": mode,
        "browser": browser,
        "runNonce": "0123456789abcdef0123456789abcdef",
        "executablePath": str(asset),
        "assetSha256": policy["upstreamSources"][ASSET],
    }
    process.write_text(
        json.dumps(
            {
                "event": "dummy-node-exited",
                **common,
                "exitCode": 1,
                "childPid": 31337,
                "node": str(node),
                "nodeSha256": policy["systemNodeSha256"],
                "argv": [str(node), str(asset), "--headless"],
            }
        )
        + "\n"
    )
    public.write_text(
        json.dumps({"event": "public-launch-invoked", **common})
        + "\n"
        + json.dumps(
            {
                "event": "public-launch-failed",
                **common,
                "errorClass": policy["expectedErrorClass"][browser],
                "errorMessage": "Browser process exited with code 1",
                "processReceiptSha256": file_sha256(process),
            }
        )
        + "\n"
    )
    payload = {
        "mode": mode,
        "api": "async",
        "browser": browser,
        "runNonce": common["runNonce"],
        "result": str(result),
        "publicReceipt": str(public),
        "publicReceiptSha256": file_sha256(public),
        "processReceipt": str(process),
        "processReceiptSha256": file_sha256(process),
        "systemNode": str(node),
    }
    return payload, asset


@pytest.mark.parametrize("mode", ["candidate", "oracle"])
@pytest.mark.parametrize("browser", ["firefox", "webkit"])
def test_both_lanes_require_exit_one_process_receipt(
    tmp_path: Path, mode: str, browser: str
) -> None:
    # Arrange
    payload, asset = _receipt_sample(tmp_path, mode, browser)

    # Act
    _receipts(payload, _ready_policy(), asset)

    # Assert
    assert Path(payload["processReceipt"]).stat().st_size > 0


@pytest.mark.parametrize("mode", ["candidate", "oracle"])
def test_missing_process_receipt_rejected_in_both_lanes(tmp_path: Path, mode: str) -> None:
    # Arrange
    payload, asset = _receipt_sample(tmp_path, mode, "firefox")
    process = Path(payload["processReceipt"])
    process.write_text("")
    payload["processReceiptSha256"] = file_sha256(process)

    # Act
    with pytest.raises(StrictEvidenceError, match="process receipt"):
        _receipts(payload, _ready_policy(), asset)

    # Assert
    assert process.stat().st_size == 0


def test_wrong_nonce_process_receipt_is_rejected(tmp_path: Path) -> None:
    # Arrange
    payload, asset = _receipt_sample(tmp_path, "candidate", "webkit")
    process = Path(payload["processReceipt"])
    row = json.loads(process.read_text())
    row["runNonce"] = "f" * 32
    process.write_text(json.dumps(row) + "\n")
    payload["processReceiptSha256"] = file_sha256(process)

    # Act
    with pytest.raises(StrictEvidenceError, match="process receipt"):
        _receipts(payload, _ready_policy(), asset)

    # Assert
    assert row["runNonce"] != payload["runNonce"]


def test_non_one_child_exit_is_rejected(tmp_path: Path) -> None:
    # Arrange
    payload, asset = _receipt_sample(tmp_path, "candidate", "firefox")
    process = Path(payload["processReceipt"])
    row = json.loads(process.read_text())
    row["exitCode"] = 0
    process.write_text(json.dumps(row) + "\n")
    payload["processReceiptSha256"] = file_sha256(process)

    # Act
    with pytest.raises(StrictEvidenceError, match="exit-one child"):
        _receipts(payload, _ready_policy(), asset)

    # Assert
    assert row["exitCode"] == 0


def test_candidate_unsupported_error_is_rejected_after_real_process(
    tmp_path: Path,
) -> None:
    # Arrange
    payload, asset = _receipt_sample(tmp_path, "candidate", "webkit")
    public = Path(payload["publicReceipt"])
    rows = [json.loads(line) for line in public.read_text().splitlines()]
    rows[1]["errorClass"] = "UnsupportedError"
    rows[1]["errorMessage"] = "PWRIGHT_UNSUPPORTED"
    public.write_text("".join(json.dumps(row) + "\n" for row in rows))
    payload["publicReceiptSha256"] = file_sha256(public)

    # Act
    with pytest.raises(StrictEvidenceError, match="public launch failure"):
        _receipts(payload, _ready_policy(), asset)

    # Assert
    assert rows[1]["errorClass"] == "UnsupportedError"


def test_candidate_wheel_snapshot_never_replaces_prior_evidence(tmp_path: Path) -> None:
    # Arrange
    source = tmp_path / "release.whl"
    source.write_bytes(b"pinned release wheel bytes")
    expected = hashlib.sha256(source.read_bytes()).hexdigest()
    result = tmp_path / "candidate.xml"

    # Act
    snapshot = _candidate_wheel_snapshot(result, source, expected)

    # Assert
    assert snapshot.read_bytes() == source.read_bytes()
    with pytest.raises(ValueError, match="already exists"):
        _candidate_wheel_snapshot(result, source, expected)


def test_candidate_wheel_snapshot_rejects_source_digest_drift(tmp_path: Path) -> None:
    # Arrange
    source = tmp_path / "release.whl"
    source.write_bytes(b"changed wheel")

    # Act
    with pytest.raises(StrictEvidenceError, match="SHA-256 differs"):
        _candidate_wheel_snapshot(tmp_path / "candidate.xml", source, "0" * 64)

    # Assert
    assert not (tmp_path / "candidate.candidate-wheel.whl").exists()


def test_wheel_install_must_match_each_pinned_candidate_file(tmp_path: Path) -> None:
    # Arrange
    site = tmp_path / "site-packages"
    wheel = tmp_path / "candidate.whl"
    entries = {
        "pwright/__init__.py": b"source = 'wheel'\n",
        "pwright/_native.abi3.so": b"native",
        "shuozeli_pwright.dylibs/libfake.dylib": b"dylib",
    }
    with zipfile.ZipFile(wheel, "w") as archive:
        for name, content in entries.items():
            archive.writestr(name, content)
    for name, content in entries.items():
        path = site / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)

    # Act
    _wheel_matches_install(wheel, site)

    # Assert
    changed = site / "pwright/__init__.py"
    changed.write_text("source = 'other'\n")
    with pytest.raises(StrictEvidenceError, match="differs from wheel"):
        _wheel_matches_install(wheel, site)


def test_wheel_install_rejects_unrecorded_installed_file(tmp_path: Path) -> None:
    # Arrange
    site = tmp_path / "site-packages"
    wheel = tmp_path / "candidate.whl"
    with zipfile.ZipFile(wheel, "w") as archive:
        archive.writestr("pwright/_native.abi3.so", b"native")
    native = site / "pwright/_native.abi3.so"
    native.parent.mkdir(parents=True)
    native.write_bytes(b"native")
    adjacent = site / "shuozeli_pwright.dylibs/libextra.dylib"
    adjacent.parent.mkdir()
    adjacent.write_bytes(b"unexpected")

    # Act
    with pytest.raises(StrictEvidenceError, match="differs from wheel"):
        _wheel_matches_install(wheel, site)

    # Assert
    assert adjacent.is_file()


def test_wheel_install_rejects_unsafe_member_path(tmp_path: Path) -> None:
    # Arrange
    site = tmp_path / "site-packages"
    wheel = tmp_path / "candidate.whl"
    with zipfile.ZipFile(wheel, "w") as archive:
        archive.writestr("../pwright/_native.abi3.so", b"native")

    # Act
    with pytest.raises(StrictEvidenceError, match="unsafe or linked"):
        _wheel_matches_install(wheel, site)

    # Assert
    assert wheel.is_file()


def test_wheel_install_rejects_unpinned_sourceless_bytecode(tmp_path: Path) -> None:
    # Arrange
    site = tmp_path / "site-packages"
    wheel = tmp_path / "candidate.whl"
    with zipfile.ZipFile(wheel, "w") as archive:
        archive.writestr("pwright/_native.abi3.so", b"native")
    native = site / "pwright/_native.abi3.so"
    native.parent.mkdir(parents=True)
    native.write_bytes(b"native")
    (site / "shuozeli_pwright.dylibs").mkdir()
    bytecode = site / "pwright/extra.pyc"
    bytecode.write_bytes(b"unrecorded executable code")

    # Act
    with pytest.raises(StrictEvidenceError, match="unpinned bytecode"):
        _wheel_matches_install(wheel, site)

    # Assert
    assert bytecode.is_file()


@pytest.mark.parametrize("mode", ["candidate", "oracle"])
def test_path_wrapper_accepts_both_lanes_and_records_exit_one(tmp_path: Path, mode: str) -> None:
    # Arrange
    command = shutil.which("node", path="/opt/homebrew/bin:/usr/bin:/bin")
    if command is None:
        pytest.skip("system Node unavailable")
    node = Path(command).resolve()
    upstream = tmp_path / "upstream"
    asset = upstream / ASSET
    asset.parent.mkdir(parents=True)
    asset.write_text("#!/usr/bin/env node\n\nprocess.exit(1);\n")
    asset.chmod(0o755)
    receipt = tmp_path / "dummy-process.jsonl"
    receipt.write_text("")
    env = {
        "PWRIGHT_PYTHON_DUMMY_UPSTREAM": str(upstream),
        "PWRIGHT_PYTHON_DUMMY_PROCESS_RECEIPT": str(receipt),
        "PWRIGHT_PYTHON_DUMMY_REAL_NODE": str(node),
        "PWRIGHT_PYTHON_DUMMY_NONCE": "0123456789abcdef0123456789abcdef",
        "PWRIGHT_PYTHON_DUMMY_API": "async",
        "PWRIGHT_OFFICIAL_MODE": mode,
        "PWRIGHT_PYTHON_DUMMY_BROWSER": "firefox",
    }

    # Act
    completed = subprocess.run(
        [
            str(ROOT / "packages/pwright-python/tests/official/dummy_engine_guard_strict_v2/node"),
            str(asset),
            "--headless",
        ],
        env=env,
        capture_output=True,
        text=True,
        timeout=10,
        check=False,
    )

    # Assert
    assert completed.returncode == 1, completed.stderr
    row = json.loads(receipt.read_text())
    assert row["mode"] == mode
    assert row["exitCode"] == 1
    assert row["argv"][:2] == [str(node), str(asset)]
    assert row["runNonce"] == env["PWRIGHT_PYTHON_DUMMY_NONCE"]
