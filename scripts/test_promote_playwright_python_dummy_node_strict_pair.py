"""AAA contract checks for fail-closed paired strict dummy-launch promotion."""

from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path

import promote_playwright_python_dummy_node_strict_pair as pair
import pytest


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


@pytest.fixture
def sample(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    # Arrange: synthetic reports stand in for four independently verified sidecars.
    reports: dict[Path, dict] = {}
    paths: dict[tuple[str, str], Path] = {}
    for mode in pair.MODES:
        for api in pair.APIS:
            result = tmp_path / f"{mode}-{api}.xml"
            result.write_text(
                '<testsuites><testsuite tests="1" failures="0" errors="0" skipped="0">'
                f'<testcase classname="tests.{api}.test_launcher" '
                f'name="{pair.TEST_NAME}[chromium]" time="0.25" />'
                "</testsuite></testsuites>"
            )
            sidecar = result.with_suffix(".provenance.json")
            sidecar.write_text("{}\n")
            paths[(mode, api)] = sidecar
            reports[sidecar] = {
                "schema": 2,
                "kind": pair.KIND,
                "evidenceScope": pair.SCOPE,
                "mode": mode,
                "api": api,
                "testId": pair._target(api),
                "manifest": str(pair.POLICY),
                "manifestSha256": pair.POLICY_SHA256,
                "upstream": "/pinned/playwright-python",
                "commit": pair.COMMIT,
                "toolingRoot": str(pair.ROOT),
                "toolingSources": {"capture": "a" * 64},
                "pythonExecutable": f"/pinned/{mode}/bin/python",
                "pythonVersion": "3.12.14",
                "wheelPath": "/pinned/abi119.whl",
                "wheelSha256": "b" * 64,
                "playwrightPackage": f"/pinned/{mode}/site/playwright",
                "playwrightPackageSha256": "c" * 64,
                "driverNode": f"/pinned/{mode}/site/playwright/driver/node",
                "driverNodeSha256": "d" * 64,
                "systemNode": "/pinned/node",
                "systemNodeSha256": "e" * 64,
                "pwrightPackage": f"/pinned/{mode}/site/pwright"
                if mode == "candidate"
                else None,
                "pwrightPackageSha256": "f" * 64 if mode == "candidate" else None,
                "nativePath": f"/pinned/{mode}/site/pwright/_native.abi3.so"
                if mode == "candidate"
                else None,
                "nativeSha256": "1" * 64 if mode == "candidate" else None,
                "runtimeLibraries": {"libtest.dylib": "2" * 64}
                if mode == "candidate"
                else {},
                "fixtureBind": "100.64.0.1",
                "fixtureHost": "test.tail8f3b66.ts.net",
                "launchMode": "owned",
                "endpoint": None,
                "collectOnly": False,
                "bootstrapOnly": False,
                "exitCode": 0,
                "result": str(result),
                "resultSha256": _sha(result),
                "publicReceipt": str(result.with_suffix(".public-launch.jsonl")),
                "processReceipt": str(result.with_suffix(".dummy-process.jsonl")),
                "runNonce": f"{mode}-{api}",
                "verifierSha256": "3" * 64,
            }
    calls: list[Path] = []

    def verified(path: Path) -> dict:
        calls.append(path)
        return copy.deepcopy(reports[path])

    monkeypatch.setattr(pair, "verify_provenance", verified)
    return paths, reports, calls, tmp_path / "paired-successors"


def test_four_passes_publish_exact_two_changed_rows_per_lane(sample) -> None:
    # Arrange
    paths, _, calls, destination = sample

    # Act
    receipt = pair.promote(paths, destination)

    # Assert
    assert len(calls) == 8
    assert destination.is_dir()
    assert receipt["kind"] == pair.RECEIPT_KIND
    assert len(receipt["reports"]) == 4
    assert len(list(destination.iterdir())) == 5
    for mode in pair.MODES:
        name = f"playwright-python-{mode}-dummy-node-abi119-strict-paired-v1.jsonl"
        base_lines = pair.BASES[mode][0].read_bytes().splitlines(keepends=True)
        successor = destination / name
        new_lines = successor.read_bytes().splitlines(keepends=True)
        assert len(base_lines) == len(new_lines) == pair.RECORDS
        changed = [
            index
            for index, (old, new) in enumerate(zip(base_lines, new_lines))
            if old != new
        ]
        assert changed == [2410, 5626]
        for index in changed:
            row = json.loads(new_lines[index])
            assert row["id"] in {pair._target(api) for api in pair.APIS}
            assert row["outcome"] == "passed"
            assert row["attempts"] == 1
            assert row["durationSeconds"] == 0.25
        summary = json.loads(successor.with_suffix(".summary.json").read_text())
        assert summary["ledgerSha256"] == _sha(successor)
        assert summary["records"] == pair.RECORDS
        assert summary["outcomes"]["passed"] == (1527 if mode == "candidate" else 1294)
        assert summary["outcomes"]["not-run"] == (4883 if mode == "candidate" else 5217)
    assert json.loads((destination / "promotion-receipt.json").read_text()) == receipt
    assert pair.verify_bundle(paths, destination) == receipt


def test_read_only_bundle_verifier_rejects_unrelated_row_change(sample) -> None:
    # Arrange
    paths, _, _, destination = sample
    pair.promote(paths, destination)
    successor = (
        destination
        / "playwright-python-candidate-dummy-node-abi119-strict-paired-v1.jsonl"
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


def test_read_only_bundle_verifier_rejects_extra_file(sample) -> None:
    # Arrange
    paths, _, _, destination = sample
    pair.promote(paths, destination)
    (destination / "partial.jsonl").write_text("partial")

    # Act
    with pytest.raises(pair.PromotionError, match="missing or extra"):
        pair.verify_bundle(paths, destination)

    # Assert
    assert (destination / "partial.jsonl").read_text() == "partial"


@pytest.mark.parametrize("missing", [("candidate", "async"), ("oracle", "sync")])
def test_missing_one_of_four_sidecars_prevents_any_output(sample, missing) -> None:
    # Arrange
    paths, _, _, destination = sample
    paths.pop(missing)

    # Act
    with pytest.raises(pair.PromotionError, match="all four"):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()


@pytest.mark.parametrize(
    "field,value,reason",
    [
        ("mode", "oracle", "strict executed"),
        ("testId", "tests/sync/test_launcher.py::other[chromium]", "strict executed"),
        ("manifestSha256", "0" * 64, "strict executed"),
        ("wheelSha256", "0" * 64, "identity differs"),
        ("nativeSha256", "0" * 64, "nativeSha256 differs"),
        ("exitCode", 1, "strict executed"),
        ("collectOnly", True, "strict executed"),
    ],
)
def test_mismatched_provenance_prevents_any_output(
    sample, field, value, reason
) -> None:
    # Arrange
    paths, reports, _, destination = sample
    reports[paths[("candidate", "sync")]][field] = value

    # Act
    with pytest.raises(pair.PromotionError, match=reason):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()


def test_verifier_rejection_prevents_any_output(
    sample, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    paths, _, calls, destination = sample
    original = pair.verify_provenance

    def reject(path: Path) -> dict:
        if path == paths[("oracle", "sync")]:
            raise pair.PromotionError("strict verifier rejected the fourth report")
        return original(path)

    monkeypatch.setattr(pair, "verify_provenance", reject)

    # Act
    with pytest.raises(pair.PromotionError):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()
    assert calls == [
        paths[key]
        for key in (("candidate", "async"), ("candidate", "sync"), ("oracle", "async"))
    ]


def test_reused_run_nonce_prevents_any_output(sample) -> None:
    # Arrange
    paths, reports, _, destination = sample
    reports[paths[("oracle", "sync")]]["runNonce"] = reports[
        paths[("oracle", "async")]
    ]["runNonce"]

    # Act
    with pytest.raises(pair.PromotionError, match="runNonce was reused"):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()


def test_symlinked_sidecar_prevents_any_output(sample) -> None:
    # Arrange
    paths, _, _, destination = sample
    sidecar = paths[("oracle", "sync")]
    sidecar.unlink()
    actual = sidecar.with_suffix(".other.json")
    actual.write_text("{}\n")
    sidecar.symlink_to(actual)

    # Act
    with pytest.raises(pair.PromotionError, match="symlink"):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()


@pytest.mark.parametrize("failure", ["missing", "symlink", "failed", "nan"])
def test_bad_junit_prevents_any_output(sample, failure: str) -> None:
    # Arrange
    paths, reports, _, destination = sample
    report = Path(reports[paths[("oracle", "sync")]]["result"])
    if failure == "missing":
        report.unlink()
    elif failure == "symlink":
        report.unlink()
        target = report.with_suffix(".elsewhere.xml")
        target.write_text("<testsuites />")
        report.symlink_to(target)
    elif failure == "failed":
        report.write_text(
            f'<testsuites><testcase classname="tests.sync.test_launcher" '
            f'name="{pair.TEST_NAME}[chromium]"><failure/></testcase></testsuites>'
        )
        reports[paths[("oracle", "sync")]]["resultSha256"] = _sha(report)
    else:
        report.write_text(
            f'<testsuites><testcase classname="tests.sync.test_launcher" '
            f'name="{pair.TEST_NAME}[chromium]" time="nan"/></testsuites>'
        )
        reports[paths[("oracle", "sync")]]["resultSha256"] = _sha(report)

    # Act
    with pytest.raises(pair.PromotionError):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()


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


def test_evidence_mutation_during_staging_prevents_publication(
    sample, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    paths, reports, calls, destination = sample
    original_fsync = pair._fsync_directory
    report = Path(reports[paths[("oracle", "sync")]]["result"])

    def mutate_after_staging(stage: Path) -> None:
        original_fsync(stage)
        report.write_bytes(report.read_bytes().replace(b'time="0.25"', b'time="0.50"'))

    monkeypatch.setattr(pair, "_fsync_directory", mutate_after_staging)

    # Act
    with pytest.raises(pair.PromotionError, match="strict JUnit changed"):
        pair.promote(paths, destination)

    # Assert
    assert len(calls) == 8
    assert not destination.exists()
    assert not list(destination.parent.glob(f".{destination.name}.staging-*"))


def test_racing_target_is_not_replaced_by_paired_publish(
    sample, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    paths, _, _, destination = sample
    original_publish = pair._publish_exclusive

    def race(stage: Path, target: Path) -> None:
        target.mkdir()
        (target / "sentinel").write_text("another writer")
        original_publish(stage, target)

    monkeypatch.setattr(pair, "_publish_exclusive", race)

    # Act
    with pytest.raises(
        pair.PromotionError, match="atomic paired successor publication failed"
    ):
        pair.promote(paths, destination)

    # Assert
    assert list(destination.iterdir()) == [destination / "sentinel"]
    assert (destination / "sentinel").read_text() == "another writer"
    assert not list(destination.parent.glob(f".{destination.name}.staging-*"))


def test_pinned_base_drift_prevents_any_output(
    sample, monkeypatch: pytest.MonkeyPatch
) -> None:
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
