"""Mutation controls for the pinned Firefox/WebKit official dummy-node lanes."""

from __future__ import annotations

import json
import shutil
from pathlib import Path
from typing import Any

import pytest
import verify_playwright_core_dummy_node_other_engines_strict_v1 as verifier

RESULTS = (
    Path(__file__).resolve().parents[1]
    / ".work/official-conformance/results/core-dummy-node-other-engines-strict-v1"
)
CASES = (
    ("oracle", "firefox"),
    ("oracle", "webkit"),
    ("candidate", "firefox"),
    ("candidate", "webkit"),
)
ARTIFACT_KEYS = ("result", "launchReceipt", "profileReceipt", "processReceipt", "log")


def copy_evidence(tmp_path: Path) -> dict[tuple[str, str], Path]:
    """Clone the sealed evidence, changing only absolute file references."""
    paths: dict[tuple[str, str], Path] = {}
    for mode, engine in CASES:
        source = RESULTS / f"oct20-{mode}-{engine}-v2.strict.json"
        sidecar = json.loads(source.read_text())
        for key in ARTIFACT_KEYS:
            original = Path(sidecar[key])
            copied = tmp_path / original.name
            shutil.copy2(original, copied)
            sidecar[key] = str(copied)
        path = tmp_path / source.name
        path.write_text(json.dumps(sidecar, sort_keys=True, indent=2) + "\n")
        paths[(mode, engine)] = path
    return paths


def load(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text())
    assert isinstance(value, dict)
    return value


def save(path: Path, value: dict[str, Any]) -> None:
    path.write_text(json.dumps(value, sort_keys=True, indent=2) + "\n")


def mutate_artifact(path: Path, key: str, mutate: Any) -> None:
    sidecar = load(path)
    artifact = Path(sidecar[key])
    value = load(artifact)
    mutate(value)
    save(artifact, value)
    sidecar[f"{key}Sha256"] = verifier.digest(artifact)
    save(path, sidecar)


def mutate_receipt(path: Path, key: str, mutate: Any) -> None:
    sidecar = load(path)
    artifact = Path(sidecar[key])
    rows = [json.loads(line) for line in artifact.read_text().splitlines() if line]
    mutate(rows)
    artifact.write_text("".join(json.dumps(row, sort_keys=True) + "\n" for row in rows))
    sidecar[f"{key}Sha256"] = verifier.digest(artifact)
    save(path, sidecar)


def test_sealed_four_lane_evidence_passes(tmp_path: Path) -> None:
    paths = copy_evidence(tmp_path)
    assert set(verifier.verify_all(list(paths.values()))) == {
        (engine, mode) for mode, engine in CASES
    }


@pytest.mark.parametrize(
    ("mode", "engine", "key", "replacement"),
    [
        ("candidate", "firefox", "specId", "other"),
        ("candidate", "webkit", "candidateNativeRuntimeInvoked", True),
        ("candidate", "firefox", "publicApiParityEvidence", True),
        ("oracle", "webkit", "candidatePublicMethodAttested", True),
        ("oracle", "firefox", "testStatus", "skipped"),
        ("candidate", "webkit", "sandboxNetworkOutbound", "allowed"),
        ("oracle", "webkit", "tailscaleBind", "192.168.1.9"),
        ("oracle", "firefox", "tailscaleHost", "other-tailnet.ts.net"),
        ("candidate", "firefox", "manifestSha256", "0" * 64),
    ],
)
def test_sidecar_mutation_rejected(
    tmp_path: Path, mode: str, engine: str, key: str, replacement: Any
) -> None:
    paths = copy_evidence(tmp_path)
    path = paths[(mode, engine)]
    evidence = load(path)
    evidence[key] = replacement
    save(path, evidence)
    with pytest.raises(ValueError):
        verifier.verify_all(list(paths.values()))


def test_official_result_status_mutation_rejected_even_when_rehashed(
    tmp_path: Path,
) -> None:
    paths = copy_evidence(tmp_path)
    path = paths[("candidate", "firefox")]

    def change(report: dict[str, Any]) -> None:
        report["suites"][0]["specs"][0]["tests"][0]["status"] = "expected"
        report["stats"]["expected"] = 1
        report["stats"]["unexpected"] = 0

    mutate_artifact(path, "result", change)
    with pytest.raises(ValueError):
        verifier.verify_all(list(paths.values()))


def test_official_failure_reason_mutation_rejected_even_when_rehashed(
    tmp_path: Path,
) -> None:
    paths = copy_evidence(tmp_path)
    path = paths[("candidate", "webkit")]

    def change(report: dict[str, Any]) -> None:
        report["suites"][0]["specs"][0]["tests"][0]["results"][0]["errors"][0][
            "message"
        ] = "fixture failed before assertion"

    mutate_artifact(path, "result", change)
    with pytest.raises(ValueError):
        verifier.verify_all(list(paths.values()))


def test_official_failure_location_mutation_rejected_even_when_rehashed(
    tmp_path: Path,
) -> None:
    paths = copy_evidence(tmp_path)
    path = paths[("candidate", "firefox")]

    def change(report: dict[str, Any]) -> None:
        report["suites"][0]["specs"][0]["tests"][0]["results"][0]["errors"][0][
            "location"
        ]["line"] = 64

    mutate_artifact(path, "result", change)
    with pytest.raises(ValueError):
        verifier.verify_all(list(paths.values()))


def test_launch_receipt_mutation_rejected_even_when_rehashed(tmp_path: Path) -> None:
    paths = copy_evidence(tmp_path)
    path = paths[("candidate", "firefox")]
    mutate_receipt(
        path, "launchReceipt", lambda rows: rows[0].__setitem__("engine", "chromium")
    )
    with pytest.raises(ValueError):
        verifier.verify_all(list(paths.values()))


def test_oracle_process_exit_mutation_rejected_even_when_rehashed(
    tmp_path: Path,
) -> None:
    paths = copy_evidence(tmp_path)
    path = paths[("oracle", "webkit")]
    mutate_receipt(
        path, "processReceipt", lambda rows: rows[0].__setitem__("exitCode", 0)
    )
    with pytest.raises(ValueError):
        verifier.verify_all(list(paths.values()))


def test_oracle_profile_mutation_rejected_even_when_rehashed(tmp_path: Path) -> None:
    paths = copy_evidence(tmp_path)
    path = paths[("oracle", "firefox")]
    mutate_receipt(
        path,
        "profileReceipt",
        lambda rows: rows[0].__setitem__("inode", rows[0]["inode"] + 1),
    )
    with pytest.raises(ValueError):
        verifier.verify_all(list(paths.values()))


def test_duplicate_lane_and_missing_lane_rejected(tmp_path: Path) -> None:
    paths = copy_evidence(tmp_path)
    values = list(paths.values())
    with pytest.raises(ValueError):
        verifier.verify_all([*values[:3], values[0]])
    with pytest.raises(ValueError):
        verifier.verify_all(values[:3])


def test_cross_engine_reused_run_id_rejected_even_with_matching_receipts(
    tmp_path: Path,
) -> None:
    paths = copy_evidence(tmp_path)
    reused = load(paths[("candidate", "firefox")])["runId"]
    path = paths[("candidate", "webkit")]
    evidence = load(path)
    evidence["runId"] = reused
    save(path, evidence)
    mutate_receipt(
        path,
        "launchReceipt",
        lambda rows: [row.__setitem__("runId", reused) for row in rows],
    )
    verifier.verify_one(path)
    with pytest.raises(ValueError):
        verifier.verify_all(list(paths.values()))


def test_result_path_mismatch_rejected(tmp_path: Path) -> None:
    paths = copy_evidence(tmp_path)
    path = paths[("candidate", "webkit")]
    evidence = load(path)
    evidence["result"] = str(tmp_path / "not-the-result.json")
    save(path, evidence)
    with pytest.raises(ValueError):
        verifier.verify_all(list(paths.values()))
