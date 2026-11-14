"""Positive and mutation controls for the v2 redirect-timeout attestation."""

from __future__ import annotations

import json
import shutil
from pathlib import Path

import pytest
import run_playwright_core_server_connect_redirect_timeout_strict_v2 as capture
import verify_playwright_core_server_connect_redirect_timeout_strict_v2 as verify

RESULT = (
    capture.RESULT_PARENT
    / "core-server-connect-redirect-timeout-strict-v2-fixed-20261012"
)


def cloned_evidence(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    parent = tmp_path / capture.RESULT_PARENT.name
    parent.mkdir()
    clone = parent / RESULT.name
    shutil.copytree(RESULT, clone)
    monkeypatch.setattr(capture, "RESULT_PARENT", parent)
    return clone


def test_exact_v2_attestation_verifies() -> None:
    # Arrange / Act
    attestation = verify.verify_directory(RESULT)

    # Assert
    assert attestation["lanes"]["oracle"]["passed"] == 9
    assert attestation["lanes"]["candidate"]["passed"] == 9
    assert attestation["promotedToCoreLedger"] is False
    assert attestation["predecessorRedAttestationSha256"] == capture.PREDECESSOR_SHA256


def test_untampered_copy_verifies_before_attestation_path_binding(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    clone = cloned_evidence(tmp_path, monkeypatch)

    # Act
    attestation = verify.verify_directory(clone, require_attestation=False)

    # Assert
    assert attestation["lanes"]["candidate"]["passed"] == 9


@pytest.mark.parametrize(
    "mutation",
    [
        "missing-row",
        "wrong-source-line",
        "wrong-result-hash",
        "wrong-artifact-hash",
        "missing-socket-close",
        "wrong-redirect-location",
        "candidate-marker-removed",
    ],
)
def test_mutated_evidence_is_rejected(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, mutation: str
) -> None:
    # Arrange
    clone = cloned_evidence(tmp_path, monkeypatch)
    result = clone / f"candidate-{next(iter(capture.PROJECTS_BY_SUITE))}.json"
    provenance = result.with_suffix(".provenance.json")
    receipt = result.with_suffix(".receipt.jsonl")
    log = result.with_suffix(".log")
    if mutation in {"missing-row", "wrong-source-line"}:
        report = json.loads(result.read_text())
        specs = report["suites"][0]["specs"]
        if mutation == "missing-row":
            specs.pop()
        else:
            specs[0]["line"] = 1055
        result.write_text(json.dumps(report))
    elif mutation in {"wrong-result-hash", "wrong-artifact-hash"}:
        sidecar = json.loads(provenance.read_text())
        key = (
            "resultSha256"
            if mutation == "wrong-result-hash"
            else "candidateArtifactSha256"
        )
        sidecar[key] = "0" * 64
        provenance.write_text(json.dumps(sidecar))
    elif mutation in {"missing-socket-close", "wrong-redirect-location"}:
        records = [json.loads(line) for line in receipt.read_text().splitlines()]
        if mutation == "missing-socket-close":
            records = [row for row in records if row["event"] != "slow-socket-closed"]
        else:
            next(row for row in records if row["event"] == "redirect-response")[
                "location"
            ] = "/wrong"
        receipt.write_text("".join(json.dumps(row) + "\n" for row in records))
    else:
        log.write_text(log.read_text().replace("[pwright-candidate]", "[removed]"))

    # Act / Assert
    with pytest.raises((AssertionError, ValueError, KeyError, TypeError)):
        verify.verify_directory(clone, require_attestation=False)
