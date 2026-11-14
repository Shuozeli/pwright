"""AAA checks for fail-closed paired official CDP WebSocket promotion."""

from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path

import promote_playwright_python_ws_transport_strict_pair as pair
import pytest


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


@pytest.fixture
def sample(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    # Arrange: replace only the expensive independent verifier, not the base ledgers.
    paths: dict[tuple[str, str], Path] = {}
    reports: dict[Path, dict] = {}
    for mode in pair.MODES:
        for case in pair.CASES:
            result = tmp_path / f"{mode}-{case}.xml"
            result.write_text(
                '<testsuites><testsuite tests="1" failures="0" errors="0" skipped="0">'
                '<testcase classname="tests.async.test_browsertype_connect_cdp" '
                f'name="{pair._target(case).split("::", 1)[1]}" time="0.25" />'
                "</testsuite></testsuites>"
            )
            sidecar = result.with_suffix(".provenance.json")
            sidecar.write_text("{}\n")
            paths[(mode, case)] = sidecar
            reports[sidecar] = {
                "schema": 1,
                "kind": pair.KIND,
                "mode": mode,
                "testId": pair._target(case),
                "manifest": str(pair.POLICY),
                "manifestSha256": pair.POLICY_SHA256,
                "upstream": "/pinned/playwright-python",
                "commit": pair.COMMIT,
                "toolingRoot": str(pair.ROOT),
                "toolingSources": {"guard": "a" * 64},
                "pythonExecutable": f"/pinned/{mode}/bin/python",
                "pythonRuntimeSha256": "1" * 64,
                "pythonVersion": "3.12.14",
                "wheelPath": "/pinned/new-wheel.whl",
                "wheelSha256": "b" * 64,
                "playwrightPackage": f"/pinned/{mode}/site/playwright",
                "playwrightPackageSha256": "c" * 64,
                "driverNode": f"/pinned/{mode}/site/playwright/driver/node",
                "driverNodeSha256": "d" * 64,
                "pwrightPackage": f"/pinned/{mode}/site/pwright"
                if mode == "candidate"
                else None,
                "pwrightPackageSha256": "e" * 64 if mode == "candidate" else None,
                "nativePath": f"/pinned/{mode}/site/pwright/_native.abi3.so"
                if mode == "candidate"
                else None,
                "nativeSha256": "f" * 64 if mode == "candidate" else None,
                "runtimeLibraries": {"libtest.dylib": "0" * 64}
                if mode == "candidate"
                else {},
                "fixtureBind": "100.78.97.63",
                "fixtureHost": "test.tail8f3b66.ts.net",
                "fixtureTransportAdaptation": pair._ADAPTATION,
                "verifierSha256": "2" * 64,
                "outcome": "passed",
                "exitCode": 0,
                "result": str(result),
                "resultSha256": _sha(result),
                "callReceipt": str(result.with_suffix(".call.jsonl")),
                "runNonce": f"{mode}-{case}",
            }
    calls: list[Path] = []

    def verified(path: Path) -> dict:
        calls.append(path)
        return copy.deepcopy(reports[path])

    monkeypatch.setattr(pair, "verify_provenance", verified)
    return paths, reports, calls, tmp_path / "new-paired-successors"


def test_prepare_is_read_only_and_changes_only_two_exact_rows(sample) -> None:
    # Arrange
    paths, _, calls, destination = sample

    # Act
    content, receipt = pair.prepare(paths, destination)

    # Assert
    assert not destination.exists()
    assert len(calls) == 4
    assert receipt["kind"] == pair.RECEIPT_KIND
    assert len(receipt["reports"]) == 4
    assert len(content) == 5
    for mode in pair.MODES:
        name = f"playwright-python-{mode}-cdp-ws-transport-strict-paired-v1.jsonl"
        original = pair.BASES[mode][0].read_bytes().splitlines(keepends=True)
        updated = content[name].splitlines(keepends=True)
        assert len(original) == len(updated) == pair.RECORDS
        assert (
            tuple(
                index
                for index, (old, new) in enumerate(zip(original, updated))
                if old != new
            )
            == pair.TARGET_INDEXES
        )
        for index in pair.TARGET_INDEXES:
            row = json.loads(updated[index])
            assert row["id"] in pair.TEST_IDS
            assert row["outcome"] == "passed"
            assert row["attempts"] == 1
            assert row["durationSeconds"] == 0.25
        summary = json.loads(content[Path(name).with_suffix(".summary.json").name])
        assert summary["ledgerSha256"] == hashlib.sha256(content[name]).hexdigest()
        assert summary["records"] == 6526
        assert summary["outcomes"]["passed"] == (1529 if mode == "candidate" else 1296)
        assert summary["outcomes"]["not-run"] == (4881 if mode == "candidate" else 5215)


def test_four_passes_publish_one_immutable_pair(sample) -> None:
    # Arrange
    paths, _, calls, destination = sample

    # Act
    receipt = pair.promote(paths, destination)

    # Assert
    assert len(calls) == 8
    assert destination.is_dir()
    assert len(list(destination.iterdir())) == 5
    assert json.loads((destination / "promotion-receipt.json").read_text()) == receipt
    assert pair.verify_bundle(paths, destination) == receipt


@pytest.mark.parametrize("missing", [("candidate", "header"), ("oracle", "close")])
def test_missing_sidecar_never_publishes(sample, missing) -> None:
    # Arrange
    paths, _, _, destination = sample
    paths.pop(missing)

    # Act
    with pytest.raises(pair.PromotionError, match="all four"):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()


@pytest.mark.parametrize(
    "key,value,reason",
    [
        ("outcome", "failed", "verified passing"),
        ("exitCode", 1, "verified passing"),
        (
            "testId",
            "tests/async/test_browsertype_connect_cdp.py::other[chromium]",
            "verified passing",
        ),
        ("manifestSha256", "0" * 64, "verified passing"),
        ("commit", "0" * 40, "verified passing"),
        ("fixtureTransportAdaptation", "none", "verified passing"),
        ("wheelSha256", "0" * 64, "identity differs"),
        ("nativeSha256", "0" * 64, "nativeSha256 differs"),
    ],
)
def test_failing_or_mismatched_candidate_evidence_never_publishes(
    sample, key, value, reason
) -> None:
    # Arrange
    paths, reports, _, destination = sample
    reports[paths[("candidate", "close")]][key] = value

    # Act
    with pytest.raises(pair.PromotionError, match=reason):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()


def test_failed_oracle_evidence_never_publishes(sample) -> None:
    # Arrange
    paths, reports, _, destination = sample
    reports[paths[("oracle", "header")]]["outcome"] = "failed"

    # Act
    with pytest.raises(pair.PromotionError, match="verified passing"):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()


def test_independent_verifier_rejection_never_publishes(
    sample, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    paths, _, _, destination = sample

    def reject(_path: Path) -> dict:
        raise pair.StrictWebSocketEvidenceError("source pin no longer verifies")

    monkeypatch.setattr(pair, "verify_provenance", reject)

    # Act
    with pytest.raises(pair.StrictWebSocketEvidenceError, match="source pin"):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()


def test_sidecar_mutation_during_verification_never_publishes(
    sample, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    paths, reports, _, destination = sample
    changed_path = paths[("candidate", "header")]

    def verified_then_changed(path: Path) -> dict:
        report = copy.deepcopy(reports[path])
        if path == changed_path:
            path.write_text('{"changed":true}\n')
        return report

    monkeypatch.setattr(pair, "verify_provenance", verified_then_changed)

    # Act
    with pytest.raises(pair.PromotionError, match="changed during verification"):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()


def test_real_junit_failure_cannot_be_laundered_as_a_pass(sample) -> None:
    # Arrange
    paths, reports, _, destination = sample
    payload = reports[paths[("candidate", "header")]]
    result = Path(payload["result"])
    result.write_text(
        '<testsuites><testsuite tests="1" failures="1" errors="0" skipped="0">'
        '<testcase classname="tests.async.test_browsertype_connect_cdp" '
        f'name="{pair._target("header").split("::", 1)[1]}" time="0.25">'
        '<failure message="assertion failed" /></testcase></testsuite></testsuites>'
    )
    payload["resultSha256"] = _sha(result)

    # Act
    with pytest.raises(pair.PromotionError, match="exact passing official case"):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()


def test_reused_nonce_or_result_never_publishes(sample) -> None:
    # Arrange
    paths, reports, _, destination = sample
    reports[paths[("candidate", "close")]]["runNonce"] = reports[
        paths[("candidate", "header")]
    ]["runNonce"]

    # Act
    with pytest.raises(pair.PromotionError, match="runNonce was reused"):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()


def test_stale_base_digest_never_publishes(
    sample, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    paths, _, _, destination = sample
    stale = dict(pair.BASES)
    stale["candidate"] = (stale["candidate"][0], "0" * 64)
    monkeypatch.setattr(pair, "BASES", stale)

    # Act
    with pytest.raises(pair.PromotionError, match="base SHA-256 differs"):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()


def test_existing_destination_is_not_replaced(sample) -> None:
    # Arrange
    paths, _, _, destination = sample
    destination.mkdir()
    marker = destination / "user.txt"
    marker.write_text("leave me alone")

    # Act
    with pytest.raises(pair.PromotionError, match="already exists"):
        pair.promote(paths, destination)

    # Assert
    assert marker.read_text() == "leave me alone"


def test_recheck_during_staging_rejects_changed_evidence(
    sample, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    paths, reports, _, destination = sample
    original = pair.prepare
    checks = 0

    def changing_prepare(sidecars, output_dir):
        nonlocal checks
        checks += 1
        if checks == 2:
            reports[paths[("oracle", "close")]]["wheelSha256"] = "0" * 64
        return original(sidecars, output_dir)

    monkeypatch.setattr(pair, "prepare", changing_prepare)

    # Act
    with pytest.raises(pair.PromotionError, match="identity differs"):
        pair.promote(paths, destination)

    # Assert
    assert checks == 2
    assert not destination.exists()
    assert not list(destination.parent.glob(f".{destination.name}.staging-*"))


def test_atomic_publish_failure_leaves_no_partial_bundle(
    sample, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    paths, _, _, destination = sample

    def fail_rename(staging: Path, output: Path) -> None:
        raise pair.PromotionError("simulated no-replace rename failure")

    monkeypatch.setattr(pair, "_publish_exclusive", fail_rename)

    # Act
    with pytest.raises(pair.PromotionError, match="simulated"):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()
    assert not list(destination.parent.glob(f".{destination.name}.staging-*"))


def test_read_only_bundle_verifier_rejects_other_row_change(sample) -> None:
    # Arrange
    paths, _, _, destination = sample
    pair.promote(paths, destination)
    successor = (
        destination
        / "playwright-python-candidate-cdp-ws-transport-strict-paired-v1.jsonl"
    )
    lines = successor.read_bytes().splitlines(keepends=True)
    first = json.loads(lines[0])
    first["outcome"] = "failed"
    lines[0] = (
        json.dumps(first, sort_keys=True, separators=(",", ":")) + "\n"
    ).encode()
    successor.write_bytes(b"".join(lines))

    # Act
    with pytest.raises(
        pair.PromotionError, match="changed from exact four-report merge"
    ):
        pair.verify_bundle(paths, destination)

    # Assert
    assert successor.is_file()
