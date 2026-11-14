"""AAA checks for exact eight-report, fail-closed atomic Python promotion."""

from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path

import promote_playwright_python_dummy_engine_strict_pair as pair
import pytest


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


@pytest.fixture
def sample(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    # Arrange: synthetic records exercise the promoter; the real verifier is
    # separately exercised against all eight pinned sidecars before publication.
    paths: dict[tuple[str, str, str], Path] = {}
    reports: dict[Path, dict] = {}
    for mode in pair.MODES:
        for api in pair.APIS:
            for browser in pair.BROWSERS:
                result = tmp_path / f"{mode}-{api}-{browser}.xml"
                failure = (
                    '<failure message="_native.UnsupportedError: '
                    'launch is only available for Chromium" />'
                    if mode == "candidate"
                    else ""
                )
                result.write_text(
                    '<testsuites><testsuite tests="1" errors="0" skipped="0" '
                    f'failures="{1 if failure else 0}">'
                    f'<testcase classname="tests.{api}.test_launcher" '
                    f'name="{pair.TEST_NAME}[{browser}]" time="0.25">'
                    f"{failure}</testcase></testsuite></testsuites>"
                )
                sidecar = result.with_suffix(".provenance.json")
                sidecar.write_text("{}\n")
                paths[(mode, api, browser)] = sidecar
                reports[sidecar] = {
                    "schema": 2,
                    "kind": pair.KIND,
                    "evidenceScope": pair.SCOPE,
                    "mode": mode,
                    "api": api,
                    "browser": browser,
                    "testId": pair._target(api, browser),
                    "manifest": str(pair.POLICY),
                    "manifestSha256": pair.POLICY_SHA256,
                    "upstream": "/pinned/playwright-python",
                    "commit": pair.COMMIT,
                    "toolingRoot": str(pair.ROOT),
                    "toolingSources": {"capture": "a" * 64},
                    "pythonExecutable": f"/pinned/{mode}/bin/python",
                    "pythonVersion": "3.12.14",
                    "wheelPath": "/pinned/abi121.whl",
                    "wheelSha256": "b" * 64,
                    "playwrightPackage": f"/pinned/{mode}/site/playwright",
                    "playwrightPackageSha256": "c" * 64,
                    "driverNode": f"/pinned/{mode}/site/playwright/driver/node",
                    "driverNodeSha256": "d" * 64,
                    "systemNode": "/pinned/node",
                    "systemNodeSha256": "e" * 64,
                    "pwrightPackage": (
                        f"/pinned/{mode}/site/pwright" if mode == "candidate" else None
                    ),
                    "pwrightPackageSha256": "f" * 64 if mode == "candidate" else None,
                    "nativePath": (
                        f"/pinned/{mode}/site/pwright/_native.abi3.so"
                        if mode == "candidate"
                        else None
                    ),
                    "nativeSha256": "1" * 64 if mode == "candidate" else None,
                    "runtimeLibraries": {"libtest.dylib": "2" * 64} if mode == "candidate" else {},
                    "fixtureBind": "100.64.0.1",
                    "fixtureHost": "test.tail8f3b66.ts.net",
                    "launchMode": "owned",
                    "endpoint": None,
                    "collectOnly": False,
                    "bootstrapOnly": False,
                    "exitCode": 1 if mode == "candidate" else 0,
                    "result": str(result),
                    "resultSha256": _sha(result),
                    "publicReceipt": str(result.with_suffix(".public-launch.jsonl")),
                    "processReceipt": str(result.with_suffix(".dummy-process.jsonl")),
                    "runNonce": f"{mode}-{api}-{browser}",
                    "verifierSha256": "3" * 64,
                }
    calls: list[Path] = []

    def verified(path: Path) -> dict:
        calls.append(path)
        return copy.deepcopy(reports[path])

    monkeypatch.setattr(pair, "SIDECARS", dict(paths))
    monkeypatch.setattr(pair, "verify_provenance", verified)
    return paths, reports, calls, tmp_path / "paired-successors"


def test_eight_results_publish_four_exact_changes_per_lane(sample) -> None:
    # Arrange
    paths, _, calls, destination = sample

    # Act
    receipt = pair.promote(paths, destination)

    # Assert
    assert len(calls) == 16
    assert destination.is_dir()
    assert receipt["kind"] == pair.RECEIPT_KIND
    assert len(receipt["reports"]) == 8
    assert len(list(destination.iterdir())) == 5
    for mode in pair.MODES:
        name = f"playwright-python-{mode}-dummy-engine-launch-strict-paired-v1.jsonl"
        before = pair.BASES[mode][0].read_bytes().splitlines(keepends=True)
        successor = destination / name
        after = successor.read_bytes().splitlines(keepends=True)
        assert len(before) == len(after) == pair.RECORDS
        changed = [index for index, (old, new) in enumerate(zip(before, after)) if old != new]
        assert changed == list(pair.TARGET_INDEXES)
        for index in changed:
            row = json.loads(after[index])
            assert row["outcome"] == ("failed" if mode == "candidate" else "passed")
            assert row["attempts"] == 1
            assert row["durationSeconds"] == 0.25
            assert row["sourceClassification"] == "public-api-source"
            assert ("error" in row) == (mode == "candidate")
        summary = json.loads(successor.with_suffix(".summary.json").read_text())
        assert summary["ledgerSha256"] == _sha(successor)
        assert summary["records"] == pair.RECORDS
    assert json.loads((destination / "promotion-receipt.json").read_text()) == receipt
    assert pair.verify_bundle(paths, destination) == receipt


def test_read_only_verifier_rejects_unrelated_ledger_row_tamper(sample) -> None:
    # Arrange
    paths, _, _, destination = sample
    pair.promote(paths, destination)
    successor = (
        destination / "playwright-python-candidate-dummy-engine-launch-strict-paired-v1.jsonl"
    )
    lines = successor.read_bytes().splitlines(keepends=True)
    first = json.loads(lines[0])
    first["outcome"] = "failed"
    lines[0] = (json.dumps(first, sort_keys=True, separators=(",", ":")) + "\n").encode()
    successor.write_bytes(b"".join(lines))

    # Act
    with pytest.raises(pair.PromotionError, match="changed from exact eight-report merge"):
        pair.verify_bundle(paths, destination)

    # Assert
    assert successor.is_file()


def test_read_only_verifier_rejects_receipt_tamper(sample) -> None:
    # Arrange
    paths, _, _, destination = sample
    pair.promote(paths, destination)
    receipt = destination / "promotion-receipt.json"
    receipt.write_text(receipt.read_text().replace(pair.RECEIPT_KIND, "forged-kind"))

    # Act
    with pytest.raises(pair.PromotionError, match="changed from exact eight-report merge"):
        pair.verify_bundle(paths, destination)

    # Assert
    assert "forged-kind" in receipt.read_text()


def test_read_only_verifier_rejects_extra_file(sample) -> None:
    # Arrange
    paths, _, _, destination = sample
    pair.promote(paths, destination)
    (destination / "partial.jsonl").write_text("partial")

    # Act
    with pytest.raises(pair.PromotionError, match="missing or extra"):
        pair.verify_bundle(paths, destination)

    # Assert
    assert (destination / "partial.jsonl").read_text() == "partial"


def test_missing_one_sidecar_prevents_output(sample) -> None:
    # Arrange
    paths, _, _, destination = sample
    paths.pop(("oracle", "sync", "webkit"))

    # Act
    with pytest.raises(pair.PromotionError, match="exact eight"):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()


@pytest.mark.parametrize(
    "field,value,reason",
    [
        ("mode", "oracle", "exact executed"),
        ("testId", "tests/sync/test_launcher.py::other[webkit]", "exact executed"),
        ("manifestSha256", "0" * 64, "exact executed"),
        ("wheelSha256", "0" * 64, "identity differs"),
        ("nativeSha256", "0" * 64, "nativeSha256 differs"),
        ("exitCode", 0, "exact executed"),
        ("collectOnly", True, "exact executed"),
    ],
)
def test_mismatched_provenance_prevents_output(sample, field, value, reason) -> None:
    # Arrange
    paths, reports, _, destination = sample
    reports[paths[("candidate", "sync", "webkit")]][field] = value

    # Act
    with pytest.raises(pair.PromotionError, match=reason):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()


def test_candidate_false_green_junit_prevents_output(sample) -> None:
    # Arrange
    paths, reports, _, destination = sample
    payload = reports[paths[("candidate", "sync", "webkit")]]
    result = Path(payload["result"])
    result.write_text(
        result.read_text().replace(
            '<failure message="_native.UnsupportedError: launch is only available for Chromium" />',
            "",
        )
    )
    payload["resultSha256"] = _sha(result)

    # Act
    with pytest.raises(pair.PromotionError, match="authentic unsupported failure"):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()


def test_reused_nonce_prevents_output(sample) -> None:
    # Arrange
    paths, reports, _, destination = sample
    reports[paths[("oracle", "sync", "webkit")]]["runNonce"] = reports[
        paths[("candidate", "async", "firefox")]
    ]["runNonce"]

    # Act
    with pytest.raises(pair.PromotionError, match="runNonce was reused"):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()


def test_symlinked_sidecar_prevents_output(sample) -> None:
    # Arrange
    paths, _, _, destination = sample
    sidecar = paths[("oracle", "sync", "webkit")]
    sidecar.unlink()
    target = sidecar.with_suffix(".other.json")
    target.write_text("{}\n")
    sidecar.symlink_to(target)

    # Act
    with pytest.raises(pair.PromotionError, match="linked"):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()


def test_pinned_base_drift_prevents_output(sample, monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    paths, _, _, destination = sample
    changed = dict(pair.BASES)
    original, _ = changed["oracle"]
    changed["oracle"] = original, "0" * 64
    monkeypatch.setattr(pair, "BASES", changed)

    # Act
    with pytest.raises(pair.PromotionError, match="pinned base SHA-256 differs"):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()


def test_evidence_mutation_during_staging_prevents_publication(
    sample, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    paths, reports, calls, destination = sample
    original_fsync = pair._fsync_directory
    result = Path(reports[paths[("oracle", "sync", "webkit")]]["result"])

    def mutate_after_staging(stage: Path) -> None:
        original_fsync(stage)
        result.write_bytes(result.read_bytes().replace(b'time="0.25"', b'time="0.50"'))

    monkeypatch.setattr(pair, "_fsync_directory", mutate_after_staging)

    # Act
    with pytest.raises(pair.PromotionError, match="strict JUnit changed"):
        pair.promote(paths, destination)

    # Assert
    assert len(calls) == 16
    assert not destination.exists()
    assert not list(destination.parent.glob(f".{destination.name}.staging-*"))


def test_atomic_publish_failure_leaves_no_partial_target(
    sample, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    paths, _, _, destination = sample

    def fail(_stage: Path, _target: Path) -> None:
        raise pair.PromotionError("exclusive publish failed")

    monkeypatch.setattr(pair, "_publish_exclusive", fail)

    # Act
    with pytest.raises(pair.PromotionError, match="exclusive publish failed"):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()
    assert not list(destination.parent.glob(f".{destination.name}.staging-*"))


@pytest.mark.parametrize("state", ["file", "directory", "symlink", "partial"])
def test_existing_or_partial_target_is_never_overwritten(sample, state: str) -> None:
    # Arrange
    paths, _, calls, destination = sample
    if state == "file":
        destination.write_text("keep me")
    elif state == "directory":
        destination.mkdir()
    elif state == "symlink":
        other = destination.with_name("other")
        other.mkdir()
        destination.symlink_to(other, target_is_directory=True)
    else:
        destination.mkdir()
        (destination / "candidate.jsonl").write_text("partial")

    # Act
    with pytest.raises(pair.PromotionError, match="already exists"):
        pair.promote(paths, destination)

    # Assert
    assert not calls
    if state == "file":
        assert destination.read_text() == "keep me"
    elif state == "partial":
        assert (destination / "candidate.jsonl").read_text() == "partial"


def test_racing_target_is_not_replaced(sample, monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    paths, _, _, destination = sample
    original_publish = pair._publish_exclusive

    def race(stage: Path, target: Path) -> None:
        target.mkdir()
        (target / "sentinel").write_text("another writer")
        original_publish(stage, target)

    monkeypatch.setattr(pair, "_publish_exclusive", race)

    # Act
    with pytest.raises(pair.PromotionError, match="atomic paired publication failed"):
        pair.promote(paths, destination)

    # Assert
    assert list(destination.iterdir()) == [destination / "sentinel"]
    assert not list(destination.parent.glob(f".{destination.name}.staging-*"))
