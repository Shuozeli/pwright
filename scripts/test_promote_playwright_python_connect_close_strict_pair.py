"""AAA tests for honest, atomic official BrowserType.connect promotion."""

from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path

import promote_playwright_python_connect_close_strict_pair as pair
import pytest


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


@pytest.fixture
def sample(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    # Arrange: fake only the independent verifier; retain real pinned ledgers.
    paths: dict[tuple[str, str], Path] = {}
    reports: dict[Path, dict] = {}
    for mode in pair.MODES:
        for engine in pair.ENGINES:
            result = tmp_path / f"{mode}-{engine}.xml"
            if mode == "candidate":
                result.write_text(
                    '<testsuites><testsuite tests="1" failures="1" errors="0" skipped="0">'
                    '<testcase classname="tests.async.test_browsertype_connect" '
                    f'name="{pair._target(engine).split("::", 1)[1]}" time="0.25">'
                    '<failure message="AttributeError: &apos;BrowserType&apos; object has no '
                    'attribute &apos;connect&apos;" /></testcase></testsuite></testsuites>'
                )
            else:
                result.write_text(
                    '<testsuites><testsuite tests="1" failures="0" errors="0" skipped="0">'
                    '<testcase classname="tests.async.test_browsertype_connect" '
                    f'name="{pair._target(engine).split("::", 1)[1]}" time="0.25" />'
                    "</testsuite></testsuites>"
                )
            sidecar = result.with_suffix(".provenance.json")
            sidecar.write_text("{}\n")
            paths[(mode, engine)] = sidecar
            reports[sidecar] = {
                "schema": 1,
                "kind": pair.KIND,
                "mode": mode,
                "testId": pair._target(engine),
                "manifest": str(pair.POLICY),
                "manifestSha256": pair.POLICY_SHA256,
                "upstream": "/pinned/playwright-python",
                "commit": pair.COMMIT,
                "toolingRoot": str(pair.ROOT),
                "toolingSources": {"runner": "a" * 64},
                "pythonExecutable": f"/pinned/{mode}/bin/python",
                "pythonRuntimeSha256": "1" * 64,
                "pythonVersion": "3.12.14",
                "wheelPath": "/pinned/abi121.whl",
                "wheelSha256": pair.WHEEL_SHA256,
                "playwrightPackage": f"/pinned/{mode}/site/playwright",
                "playwrightPackageSha256": "b" * 64,
                "driverNode": f"/pinned/{mode}/site/playwright/driver/node",
                "driverNodeSha256": "c" * 64,
                "pwrightPackage": f"/pinned/{mode}/site/pwright"
                if mode == "candidate"
                else None,
                "pwrightPackageSha256": "d" * 64 if mode == "candidate" else None,
                "nativePath": f"/pinned/{mode}/site/pwright/_native.abi3.so"
                if mode == "candidate"
                else None,
                "nativeSha256": "e" * 64 if mode == "candidate" else None,
                "runtimeLibraries": {"libtest.dylib": "f" * 64}
                if mode == "candidate"
                else {},
                "fixtureBind": "100.78.97.63",
                "fixtureHost": "test.tail8f3b66.ts.net",
                "fixtureTransportAdaptation": pair.ADAPTATION,
                "verifierSha256": "2" * 64,
                "outcome": "failed" if mode == "candidate" else "passed",
                "exitCode": 1 if mode == "candidate" else 0,
                "result": str(result),
                "resultSha256": _sha(result),
                "callReceipt": str(result.with_suffix(".call.jsonl")),
                "runNonce": f"{mode}-{engine}",
            }

    def verified(path: Path) -> dict:
        return copy.deepcopy(reports[path])

    monkeypatch.setattr(pair, "SIDECARS", paths)
    monkeypatch.setattr(pair, "verify_provenance", verified)
    return paths, reports, tmp_path / "new-paired-successors"


def test_prepare_preserves_all_other_rows_and_records_real_failure(sample) -> None:
    # Arrange
    paths, _, destination = sample

    # Act
    content, receipt = pair.prepare(paths, destination)

    # Assert
    assert not destination.exists()
    assert len(content) == 5
    assert len(receipt["reports"]) == 6
    assert [item["outcome"] for item in receipt["reports"]] == [
        "failed",
        "failed",
        "failed",
        "passed",
        "passed",
        "passed",
    ]
    for mode in pair.MODES:
        name = f"playwright-python-{mode}-browser-connect-close-strict-paired-v1.jsonl"
        before = pair.BASES[mode][0].read_bytes().splitlines(keepends=True)
        after = content[name].splitlines(keepends=True)
        assert len(before) == len(after) == pair.RECORDS
        assert tuple(
            i for i, (old, new) in enumerate(zip(before, after)) if old != new
        ) == (pair.TARGET_INDEXES)
        for index in pair.TARGET_INDEXES:
            row = json.loads(after[index])
            assert row["id"] in pair.TEST_IDS
            assert row["attempts"] == 1
            assert row["durationSeconds"] == 0.25
            assert row["outcome"] == ("failed" if mode == "candidate" else "passed")
            assert ("error" in row) == (mode == "candidate")
            if mode == "candidate":
                assert "has no attribute 'connect'" in row["error"]
        summary = json.loads(content[Path(name).with_suffix(".summary.json").name])
        assert summary["ledgerSha256"] == hashlib.sha256(content[name]).hexdigest()
        assert summary["records"] == 6526
        assert summary["outcomes"]["not-run"] == (4878 if mode == "candidate" else 5212)
        assert summary["outcomes"]["failed"] == (105 if mode == "candidate" else 4)
        assert summary["outcomes"]["passed"] == (1529 if mode == "candidate" else 1299)


def test_six_evidences_publish_one_immutable_pair(sample) -> None:
    # Arrange
    paths, _, destination = sample

    # Act
    receipt = pair.promote(paths, destination)

    # Assert
    assert destination.is_dir()
    assert len(list(destination.iterdir())) == 5
    assert json.loads((destination / "promotion-receipt.json").read_text()) == receipt
    assert pair.verify_bundle(paths, destination) == receipt
    with pytest.raises(pair.PromotionError, match="already exists"):
        pair.promote(paths, destination)


@pytest.mark.parametrize(
    "mode,engine,key,value,reason",
    [
        ("candidate", "chromium", "outcome", "passed", "verified official outcome"),
        ("oracle", "firefox", "outcome", "failed", "verified official outcome"),
        ("candidate", "webkit", "exitCode", 0, "verified official outcome"),
        ("oracle", "webkit", "testId", "wrong", "verified official outcome"),
        ("candidate", "firefox", "wheelSha256", "0" * 64, "verified official outcome"),
        ("candidate", "firefox", "nativeSha256", "0" * 64, "nativeSha256 differs"),
    ],
)
def test_mismatched_evidence_never_publishes(
    sample, mode, engine, key, value, reason
) -> None:
    # Arrange
    paths, reports, destination = sample
    reports[paths[(mode, engine)]][key] = value

    # Act
    with pytest.raises(pair.PromotionError, match=reason):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()


def test_six_exact_sidecars_are_required(sample) -> None:
    # Arrange
    paths, _, destination = sample
    incomplete = dict(paths)
    incomplete.pop(("oracle", "webkit"))

    # Act
    with pytest.raises(pair.PromotionError, match="exact six pinned sidecars"):
        pair.promote(incomplete, destination)

    # Assert
    assert not destination.exists()


def test_candidate_junit_failure_cannot_be_laundered_as_pass(sample) -> None:
    # Arrange
    paths, reports, destination = sample
    payload = reports[paths[("candidate", "chromium")]]
    result = Path(payload["result"])
    result.write_text(
        '<testsuites><testsuite tests="1" failures="0" errors="0" skipped="0">'
        '<testcase classname="tests.async.test_browsertype_connect" '
        f'name="{pair._target("chromium").split("::", 1)[1]}" time="0.25" />'
        "</testsuite></testsuites>"
    )
    payload["resultSha256"] = _sha(result)

    # Act
    with pytest.raises(pair.PromotionError, match="authentic failed case"):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()


def test_wrong_candidate_failure_reason_cannot_publish(sample) -> None:
    # Arrange
    paths, reports, destination = sample
    payload = reports[paths[("candidate", "chromium")]]
    result = Path(payload["result"])
    result.write_text(
        result.read_text().replace("has no attribute", "failed for another reason")
    )
    payload["resultSha256"] = _sha(result)

    # Act
    with pytest.raises(pair.PromotionError, match="missing public connect method"):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()


def test_reused_nonce_or_result_never_publishes(sample) -> None:
    # Arrange
    paths, reports, destination = sample
    reports[paths[("candidate", "firefox")]]["runNonce"] = reports[
        paths[("candidate", "chromium")]
    ]["runNonce"]

    # Act
    with pytest.raises(pair.PromotionError, match="runNonce was reused"):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()


def test_stale_base_sha_never_publishes(
    sample, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    paths, _, destination = sample
    bases = dict(pair.BASES)
    bases["candidate"] = (bases["candidate"][0], "0" * 64)
    monkeypatch.setattr(pair, "BASES", bases)

    # Act
    with pytest.raises(pair.PromotionError, match="base SHA-256 differs"):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()


def test_independent_verifier_rejection_never_publishes(
    sample, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    paths, _, destination = sample

    def reject(_path: Path) -> dict:
        raise pair.StrictBrowserConnectEvidenceError("source pin no longer verifies")

    monkeypatch.setattr(pair, "verify_provenance", reject)

    # Act
    with pytest.raises(pair.StrictBrowserConnectEvidenceError, match="source pin"):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()


def test_sidecar_mutation_during_verification_never_publishes(
    sample, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    paths, reports, destination = sample
    changed_path = paths[("candidate", "chromium")]

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


def test_staging_recheck_rejects_changed_evidence(
    sample, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    paths, reports, destination = sample
    original = pair.prepare
    checks = 0

    def changing_prepare(sidecars, output_dir):
        nonlocal checks
        checks += 1
        if checks == 2:
            reports[paths[("oracle", "webkit")]]["wheelSha256"] = "0" * 64
        return original(sidecars, output_dir)

    monkeypatch.setattr(pair, "prepare", changing_prepare)

    # Act
    with pytest.raises(pair.PromotionError, match="verified official outcome"):
        pair.promote(paths, destination)

    # Assert
    assert checks == 2
    assert not destination.exists()
    assert not list(destination.parent.glob(f".{destination.name}.staging-*"))


def test_atomic_publish_failure_leaves_no_partial_bundle(
    sample, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    paths, _, destination = sample

    def fail_rename(_staging: Path, _destination: Path) -> None:
        raise pair.PromotionError("simulated no-replace rename failure")

    monkeypatch.setattr(pair, "_publish_exclusive", fail_rename)

    # Act
    with pytest.raises(pair.PromotionError, match="simulated"):
        pair.promote(paths, destination)

    # Assert
    assert not destination.exists()
    assert not list(destination.parent.glob(f".{destination.name}.staging-*"))


def test_read_only_verifier_rejects_unrelated_row_change(sample) -> None:
    # Arrange
    paths, _, destination = sample
    pair.promote(paths, destination)
    successor = (
        destination
        / "playwright-python-candidate-browser-connect-close-strict-paired-v1.jsonl"
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
        pair.PromotionError, match="changed from exact six-report merge"
    ):
        pair.verify_bundle(paths, destination)

    # Assert
    assert successor.is_file()
