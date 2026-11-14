"""Mutation controls for the independently built fixed-wheel diagnostic."""

from __future__ import annotations

import json
import zipfile
from pathlib import Path

import pytest
import verify_playwright_python_connect_remote_kill_synthetic_v1 as prior
import verify_playwright_python_connect_remote_kill_synthetic_v2 as verify

FINAL = verify.RESULT_PARENT / "python-connect-remote-kill-synthetic-v2-terminal-fix-oct20"


def test_new_wheel_and_selected_source_are_pinned() -> None:
    # Arrange
    wheel = verify.BUILD_WHEEL

    # Act
    ledgers = verify.verify_sources()
    with zipfile.ZipFile(wheel) as archive:
        api_source = archive.read("pwright/async_api.py")

    # Assert
    assert prior.sha256(wheel) == verify.WHEEL_SHA256
    assert prior.sha256(verify.ROOT / "packages/pwright-python/python/pwright/async_api.py") == verify.SOURCE_HASHES["packages/pwright-python/python/pwright/async_api.py"]
    assert prior.hashlib.sha256(api_source).hexdigest() == verify.SOURCE_HASHES["packages/pwright-python/python/pwright/async_api.py"]
    assert set(ledgers) == {"oracle", "candidate"}


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("fixtureAdapted", False),
        ("testSourceAdapted", True),
        ("publicApiParityEvidence", True),
        ("unadaptedOfficialPass", True),
        ("promotableToOfficialLedger", True),
        ("promotedToPythonLedger", True),
        ("candidateWheelSha256", prior.WHEEL_SHA256),
        ("candidateRepeats", 1),
    ],
)
def test_manifest_mutation_cannot_claim_false_evidence(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, field: str, value: object
) -> None:
    # Arrange
    manifest = json.loads(verify.MANIFEST.read_text())
    manifest[field] = value
    changed = tmp_path / "manifest.json"
    changed.write_text(json.dumps(manifest))
    monkeypatch.setattr(verify, "MANIFEST", changed)

    # Act / Assert
    with pytest.raises(ValueError, match="fixed-wheel manifest changed"):
        verify.verify_sources()


def test_source_hash_mutation_is_rejected(monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    changed = dict(verify.SOURCE_HASHES)
    changed["packages/pwright-python/python/pwright/async_api.py"] = "0" * 64
    monkeypatch.setattr(verify, "SOURCE_HASHES", changed)

    # Act / Assert
    with pytest.raises(ValueError, match="fixed-wheel source changed"):
        verify.verify_sources()


def test_stale_red_wheel_cannot_be_relabelled_as_fixed() -> None:
    # Arrange
    stale = verify.PREDECESSOR / verify.WHEEL_NAME

    # Act / Assert
    with pytest.raises(ValueError, match="fixed candidate wheel changed"):
        verify.verify_wheel_install(stale, verify.PREDECESSOR / "venv/lib/python3.12/site-packages")


def tooling_evidence(tmp_path: Path, wheel: Path) -> tuple[Path, Path]:
    install_log = tmp_path / "install.log"
    freeze = tmp_path / "tooling-freeze.txt"
    install_log.write_text(" + playwright==1.58.0\n + shuozeli-pwright==0.1.0\n")
    freeze.write_text("\n".join(
        [f"{name}=={version}" for name, version in prior.TOOLING.items() if name != "python"]
        + [f"shuozeli-pwright @ file://{wheel}"]
    ) + "\n")
    return install_log, freeze


def test_complete_tooling_evidence_is_accepted(tmp_path: Path) -> None:
    # Arrange
    wheel = tmp_path / verify.WHEEL_NAME
    tooling_evidence(tmp_path, wheel)

    # Act
    hashes = verify.verify_tooling(tmp_path, wheel)

    # Assert
    assert set(hashes) == {"install.log", "tooling-freeze.txt"}


@pytest.mark.parametrize(
    ("tamper", "message"),
    [
        ("playwright-version", "tooling version changed: playwright"),
        ("wheel-origin", "installed candidate wheel source changed"),
        ("install-log", "tooling installation log changed"),
    ],
)
def test_tooling_mutations_are_rejected(tmp_path: Path, tamper: str, message: str) -> None:
    # Arrange
    wheel = tmp_path / verify.WHEEL_NAME
    install_log, freeze = tooling_evidence(tmp_path, wheel)
    if tamper == "playwright-version":
        freeze.write_text(freeze.read_text().replace("playwright==1.58.0", "playwright==1.57.0"))
    elif tamper == "wheel-origin":
        freeze.write_text(freeze.read_text().replace(str(wheel), "/wrong/wheel.whl"))
    elif tamper == "install-log":
        install_log.write_text("not a verified install\n")

    # Act / Assert
    with pytest.raises(ValueError, match=message):
        verify.verify_tooling(tmp_path, wheel)


def test_saved_fixed_wheel_attestation_reverifies_when_present() -> None:
    # Arrange
    if not (FINAL / "attestation.json").is_file():
        pytest.skip("fixed-wheel capture has not been run yet")

    # Act
    verified = verify.verify_directory(FINAL)

    # Assert
    assert verified == prior.read_json(FINAL / "attestation.json")
    assert verified["fixtureAdapted"] is True
    assert verified["promotedToPythonLedger"] is False


@pytest.mark.parametrize(
    ("tamper", "message"),
    [
        ("cdp", "inherited browser/CDP setting"),
        ("xml-hash", "raw evidence hash changed"),
        ("origin", "provenance identity changed"),
    ],
)
def test_saved_provenance_mutations_are_rejected_when_present(
    monkeypatch: pytest.MonkeyPatch, tamper: str, message: str
) -> None:
    # Arrange
    if not (FINAL / "attestation.json").is_file():
        pytest.skip("fixed-wheel capture has not been run yet")
    original = prior.read_json

    def changed_json(path: Path) -> dict[str, object]:
        value = original(path)
        if path == FINAL / "candidate-1.provenance.json":
            if tamper == "cdp":
                value["environment"]["PWRIGHT_CDP_ENDPOINT"] = "http://127.0.0.1:9222"
            elif tamper == "xml-hash":
                value["fileSha256"]["xml"] = "0" * 64
            elif tamper == "origin":
                value["sourceWheelPath"] = "/wrong/wheel.whl"
        return value

    monkeypatch.setattr(prior, "read_json", changed_json)

    # Act / Assert
    with pytest.raises(ValueError, match=message):
        verify.verify_directory(FINAL)
