"""Fail-closed checks for the inert Python official dummy-launch diagnostic."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest
from run_playwright_python_dummy_node_diagnostic import (
    ASSET,
    GUARD_DIR,
    NODE_WRAPPER,
    OFFICIAL,
    ROOT,
    _probe_sandbox,
    _python_paths,
    _receipt,
    _sandbox_profile,
)

UPSTREAM = Path(
    os.environ.get(
        "PWRIGHT_PYTHON_DUMMY_TEST_UPSTREAM",
        "/Users/cyuan/projects/thirdparty/playwright-python",
    )
)
SYSTEM_NODE = Path(
    shutil.which("node", path="/opt/homebrew/bin:/usr/bin:/bin") or "/nonexistent"
).resolve()


def test_oracle_python_path_uses_installed_package_not_upstream_source() -> None:
    # Arrange
    upstream = Path("/pinned/upstream")

    # Act
    oracle = _python_paths("oracle", upstream)
    candidate = _python_paths("candidate", upstream)

    # Assert
    assert oracle == [OFFICIAL / "runtime", GUARD_DIR]
    assert upstream not in oracle
    assert candidate == [
        OFFICIAL / "shim",
        ROOT / "packages/pwright-python/python",
        upstream,
        OFFICIAL / "runtime",
        GUARD_DIR,
    ]


@pytest.mark.skipif(sys.platform != "darwin", reason="macOS sandbox diagnostic")
def test_sandbox_denies_unlisted_process_and_outbound_network() -> None:
    # Arrange
    python = Path(sys.executable).absolute()
    profile = _sandbox_profile([python, python.resolve()])

    # Act
    result = _probe_sandbox(profile, python)

    # Assert
    assert result is None


@pytest.mark.skipif(
    sys.platform != "darwin"
    or not (UPSTREAM / ASSET).is_file()
    or not SYSTEM_NODE.is_file(),
    reason="requires macOS, pinned upstream dummy asset and Node",
)
def test_node_wrapper_receipts_only_after_pinned_dummy_exits(tmp_path: Path) -> None:
    # Arrange
    asset = UPSTREAM / ASSET
    receipt = tmp_path / "process.jsonl"
    receipt.touch()
    profile = _sandbox_profile(
        [
            asset,
            NODE_WRAPPER,
            Path("/usr/bin/env"),
            Path("/usr/bin/python3"),
            Path("/Library/Developer/CommandLineTools/usr/bin/python3"),
            Path(
                "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/bin/python3.9"
            ),
            Path(
                "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/Resources/Python.app/Contents/MacOS/Python"
            ),
            SYSTEM_NODE,
        ]
    )
    env = os.environ.copy()
    env.update(
        {
            "PATH": f"{GUARD_DIR}:/opt/homebrew/bin:/usr/bin:/bin",
            "PWRIGHT_PYTHON_DUMMY_UPSTREAM": str(UPSTREAM),
            "PWRIGHT_PYTHON_DUMMY_PROCESS_RECEIPT": str(receipt),
            "PWRIGHT_PYTHON_DUMMY_REAL_NODE": str(SYSTEM_NODE),
            "PWRIGHT_PYTHON_DUMMY_API": "async",
            "PWRIGHT_OFFICIAL_MODE": "candidate",
        }
    )

    # Act
    completed = subprocess.run(
        ["/usr/bin/sandbox-exec", "-p", profile, str(asset)],
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )

    # Assert
    assert completed.returncode == 1, completed.stderr
    assert receipt.read_text(), completed.stderr
    _receipt(
        receipt,
        [
            {
                "event": "dummy-node-exited",
                "api": "async",
                "mode": "candidate",
                "executablePath": str(asset),
                "assetSha256": "9928a78e7de42f29dfc61783a2f31673b387d7d6ea27cda5bc4472e4338c1f7f",
                "exitCode": 1,
            }
        ],
    )


def test_receipt_rejects_missing_or_extra_event(tmp_path: Path) -> None:
    # Arrange
    receipt = tmp_path / "process.jsonl"
    receipt.write_text(json.dumps({"event": "extra"}) + "\n")

    # Act
    with pytest.raises(ValueError, match="receipt differs") as error:
        _receipt(receipt, [{"event": "expected"}])

    # Assert
    assert "extra" in str(error.value)
