"""AAA checks for the pinned, guarded official dummy-launch capture runner."""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest
from run_playwright_python_dummy_node_strict import (
    POLICY,
    ROOT,
    _clean_ambient,
    _fresh_result,
    _probe_sandbox,
    _sandbox_profile,
    _selected_python,
)
from verify_playwright_python_dummy_node_strict import (
    _policy,
    _tooling,
    file_sha256,
)


def test_capture_policy_binds_exact_runner_and_guard_sources() -> None:
    # Arrange
    policy = _policy(json.loads(POLICY.read_text()))

    # Act
    guard = _tooling(policy, {"toolingSources": policy["captureSourceSha256"]}, ROOT)

    # Assert
    assert guard.name == "dummy_guard_strict"
    assert len(policy["captureSourceSha256"]) == 3
    assert policy["captureReady"] is True


@pytest.mark.parametrize(
    "override",
    [
        {"PWRIGHT_CDP_ENDPOINT": "http://127.0.0.1:9222"},
        {"PLAYWRIGHT_BROWSERS_PATH": "/tmp/alternate"},
        {"NODE_OPTIONS": "--require /tmp/alternate.js"},
        {"DYLD_INSERT_LIBRARIES": "/tmp/alternate.dylib"},
        {"PYTEST_ADDOPTS": "-k other"},
    ],
)
def test_inherited_browser_or_loader_override_is_rejected(
    override: dict[str, str],
) -> None:
    # Arrange
    environment = {"PATH": "/usr/bin:/bin", **override}

    # Act
    with pytest.raises(ValueError, match="forbids inherited"):
        _clean_ambient(environment)

    # Assert
    assert override.keys().isdisjoint({"PATH"})


def test_output_names_are_fresh_and_sibling_only(tmp_path: Path) -> None:
    # Arrange
    requested = tmp_path / "run.xml"

    # Act
    result, provenance, public, process = _fresh_result(requested)

    # Assert
    assert result == requested
    assert provenance == tmp_path / "run.provenance.json"
    assert public == tmp_path / "run.public-launch.jsonl"
    assert process == tmp_path / "run.dummy-process.jsonl"
    assert public.read_bytes() == process.read_bytes() == b""
    with pytest.raises(ValueError, match="already exists"):
        _fresh_result(requested)


def test_symlinked_result_is_rejected_without_following_it(tmp_path: Path) -> None:
    # Arrange
    target = tmp_path / "target.xml"
    target.write_text("do not overwrite")
    link = tmp_path / "run.xml"
    link.symlink_to(target)

    # Act
    with pytest.raises(ValueError, match="already exists"):
        _fresh_result(link)

    # Assert
    assert target.read_text() == "do not overwrite"


def test_unpinned_python_is_rejected_before_execution(tmp_path: Path) -> None:
    # Arrange
    runtime = tmp_path / "runtime"
    runtime.write_bytes(b"not an interpreter")
    runtime.chmod(0o755)
    python = tmp_path / "venv/bin/python"
    python.parent.mkdir(parents=True)
    python.symlink_to(runtime)

    # Act
    with pytest.raises(ValueError, match="unpinned Python runtime"):
        _selected_python(python, "0" * 64)

    # Assert
    assert file_sha256(runtime) != "0" * 64


def test_sandbox_profile_rejects_duplicate_allowed_executable() -> None:
    # Arrange
    python = str(Path(sys.executable).resolve())

    # Act
    with pytest.raises(ValueError, match="allowlist"):
        _sandbox_profile([python, python])

    # Assert
    assert Path(python).is_absolute()


@pytest.mark.skipif(sys.platform != "darwin", reason="requires macOS sandbox")
def test_sandbox_probe_denies_exec_and_network() -> None:
    # Arrange
    python = Path(sys.executable).absolute()
    profile = _sandbox_profile(
        list(dict.fromkeys([str(python), str(python.resolve())]))
    )

    # Act
    observed = _probe_sandbox(profile, python)

    # Assert
    assert observed == {"execDenied": True, "networkDenied": True}
