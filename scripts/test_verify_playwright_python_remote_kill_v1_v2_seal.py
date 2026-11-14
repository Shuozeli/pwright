"""Mutation controls for the versioned historical remote-kill input seal."""

from __future__ import annotations

import shutil
from pathlib import Path

import pytest

import seal_playwright_python_remote_kill_v1_v2 as create
import verify_playwright_python_remote_kill_v1_v2_seal as verify


def copied_seal(tmp_path: Path) -> Path:
    destination = tmp_path / "seal"
    shutil.copytree(verify.SEAL, destination)
    return destination


def test_existing_versioned_seal_cannot_be_overwritten() -> None:
    # Arrange
    original_hash = verify.sha256(verify.SEAL / "seal.json")

    # Act / Assert
    with pytest.raises(ValueError, match="refusing to overwrite seal"):
        create.seal()
    assert verify.sha256(verify.SEAL / "seal.json") == original_hash


def test_sealed_verifiers_replay_original_red_and_green_evidence() -> None:
    # Arrange
    seal = verify.SEAL

    # Act
    outcomes = verify.replay(seal)

    # Assert
    assert outcomes["v1"]["lanes"]["oracle"]["passed"] == 4
    assert outcomes["v1"]["lanes"]["candidate"]["passed"] == 2
    assert outcomes["v2"]["lanes"]["oracle"]["passed"] == 4
    assert [
        outcomes["v2"]["lanes"][f"candidate-{index}"]["passed"] for index in range(1, 6)
    ] == [4] * 5
    assert all(
        outcome["promotedToPythonLedger"] is False for outcome in outcomes.values()
    )


def test_replay_never_reads_mutable_candidate_source(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    live_source = verify.ROOT / "packages/pwright-python/python/pwright/async_api.py"
    original = Path.is_file

    def reject_live_source(path: Path) -> bool:
        if path == live_source:
            raise AssertionError("historical replay consulted mutable source")
        return original(path)

    monkeypatch.setattr(Path, "is_file", reject_live_source)

    # Act
    outcomes = verify.replay()

    # Assert
    assert outcomes["v2"]["lanes"]["candidate-5"]["passed"] == 4


def test_modified_manifest_is_rejected_before_replay(tmp_path: Path) -> None:
    # Arrange
    seal = tmp_path / "seal"
    seal.mkdir()
    (seal / "seal.json").write_bytes((verify.SEAL / "seal.json").read_bytes() + b" ")

    # Act / Assert
    with pytest.raises(ValueError, match="seal manifest SHA-256 changed"):
        verify.verify_seal(seal)


@pytest.mark.parametrize(
    "relative",
    [
        "repo/packages/pwright-python/python/pwright/async_api.py",
        f"build-wheel/{verify.BUILD_WHEEL.name}",
    ],
)
def test_modified_source_or_wheel_is_rejected(tmp_path: Path, relative: str) -> None:
    # Arrange
    seal = copied_seal(tmp_path)
    selected = seal / "inputs" / relative
    selected.write_bytes(selected.read_bytes() + b"tamper")

    # Act / Assert
    with pytest.raises(ValueError, match="sealed bytes changed"):
        verify.verify_seal(seal)


def test_unlisted_input_is_rejected(tmp_path: Path) -> None:
    # Arrange
    seal = copied_seal(tmp_path)
    (seal / "inputs/repo/unlisted.txt").write_text("extra\n")

    # Act / Assert
    with pytest.raises(ValueError, match="missing or unexpected input files"):
        verify.verify_seal(seal)


def test_verifier_edit_after_seal_check_is_rejected_before_execution(
    tmp_path: Path,
) -> None:
    # Arrange
    attestation = verify.verify_seal()
    seal = copied_seal(tmp_path)
    verifier = (
        seal
        / "inputs/repo/scripts/verify_playwright_python_connect_remote_kill_synthetic_v1.py"
    )
    verifier.write_bytes(
        verifier.read_bytes() + b"\nraise RuntimeError('must not execute')\n"
    )

    # Act / Assert
    with pytest.raises(ValueError, match="sealed verifier changed before execution"):
        verify.load_sealed_verifiers(seal, attestation)


def test_symlinked_input_root_is_rejected(tmp_path: Path) -> None:
    # Arrange
    seal = tmp_path / "seal"
    seal.mkdir()
    shutil.copyfile(verify.SEAL / "seal.json", seal / "seal.json")
    (seal / "inputs").symlink_to(verify.SEAL / "inputs", target_is_directory=True)

    # Act / Assert
    with pytest.raises(ValueError, match="inputs directory missing or symlinked"):
        verify.verify_seal(seal)


def test_upstream_git_object_mismatch_is_rejected(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    attestations = verify.verify_seal()
    monkeypatch.setattr(
        verify, "_git_object_bytes", lambda *_args: b"wrong upstream blob"
    )

    # Act / Assert
    with pytest.raises(ValueError, match="not the pinned Git object"):
        verify.verify_upstream_objects(verify.SEAL, attestations["v1"])
