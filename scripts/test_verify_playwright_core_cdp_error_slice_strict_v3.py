"""AAA negative controls for the post-fix paired Core CDP error shard."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pytest
import run_playwright_core_cdp_error_slice_strict_v1 as base
import run_playwright_core_cdp_error_slice_strict_v2 as worker_aware
import run_playwright_core_cdp_error_slice_strict_v3 as capture
import verify_playwright_core_cdp_error_slice_strict_v3 as verifier
from official_candidate_package_snapshot_v1 import create_snapshot, sha256
from run_playwright_official_conformance import _candidate_source_sha256

HOST = "fixture.tail8f3b66.ts.net"
TEST_ARTIFACT = b"fake post-fix native artifact"
TEST_SHA256 = hashlib.sha256(TEST_ARTIFACT).hexdigest()


def _report(mode: str) -> dict:
    passed = mode == "oracle"
    specs = []
    for index, (spec_id, (line, title)) in enumerate(base.CASES.items()):
        specs.append(
            {
                "id": spec_id,
                "file": base.REPORT_FILE,
                "line": line,
                "column": 5,
                "title": title,
                "ok": passed,
                "tests": [
                    {
                        "projectId": "chromium-library",
                        "expectedStatus": "passed",
                        "status": "expected" if passed else "unexpected",
                        "results": [
                            {
                                "retry": 0,
                                "workerIndex": 0 if passed else index,
                                "status": "passed" if passed else "failed",
                                "errors": []
                                if passed
                                else [{"message": f"official failure {line}"}],
                            }
                        ],
                    }
                ],
            }
        )
    return {
        "errors": [],
        "suites": [{"specs": specs, "suites": []}],
        "stats": {
            "expected": 3 if passed else 0,
            "unexpected": 0 if passed else 3,
            "skipped": 0,
            "flaky": 0,
        },
    }


def _write_json(path: Path, value: dict) -> None:
    path.write_text(json.dumps(value))


def _lane(tmp_path: Path, mode: str) -> Path:
    result = tmp_path / f"{mode}-v1.json"
    report = _report(mode)
    _write_json(result, report)
    provenance = {
        "mode": mode,
        "suite": capture.SUITE,
        "project": "chromium-library",
        "workers": 1,
        "filters": [f"{base.SOURCE}:{line}" for line in capture.SELECTED_LINES],
        "fixtureHost": HOST,
        "resultSha256": base.sha256(result),
        "exitCode": 0 if mode == "oracle" else 1,
        "candidate": str(capture.SNAPSHOT_PACKAGE) if mode == "candidate" else None,
        "candidateProfile": capture.PROFILE if mode == "candidate" else None,
        "candidateArtifactSha256": TEST_SHA256 if mode == "candidate" else None,
        "candidateSourceSha256": (
            json.loads(capture.SNAPSHOT_MANIFEST.read_text())["candidateSourceSha256"]
            if mode == "candidate"
            else None
        ),
    }
    provenance_path = result.with_suffix(".provenance.json")
    _write_json(provenance_path, provenance)
    worker_indices = ("0",) if mode == "oracle" else ("0", "1", "2")
    receipt_path = result.with_suffix(".guard-receipt.jsonl")
    receipt_path.write_text(
        "".join(
            json.dumps(
                {
                    "pid": 1000 + index,
                    "workerIndex": worker,
                    "sourceSha256": base.SOURCE_SHA256,
                    "adaptedSha256": base.adapted_source_sha256(HOST),
                    "fixtureHost": HOST,
                    "replacements": {"websocket": 1, "serverPrefixCases": 0},
                }
            )
            + "\n"
            for index, worker in enumerate((None, *worker_indices))
        )
    )
    log_path = result.with_suffix(".log")
    log_path.write_text(
        "[pwright-candidate] selected" if mode == "candidate" else "oracle selected"
    )
    selected = worker_aware.selected_tests(report)
    strict = {
        "schema": 4,
        "scope": capture.SCOPE,
        "mode": mode,
        "upstreamCommit": base.COMMIT,
        "upstreamSourceSha256": base.SOURCE_SHA256,
        "serverFixtureSha256": base.SERVER_FIXTURE_SHA256,
        "commonPreloadSha256": base.COMMON_PRELOAD_SHA256,
        "inputs": {
            str(path.relative_to(base.ROOT)): base.sha256(path) for path in capture.INPUT_FILES
        },
        "candidateProfile": capture.PROFILE if mode == "candidate" else None,
        "candidateArtifactSha256": TEST_SHA256,
        "pairedOracleStrictSha256": None,
        "candidateSnapshot": str(capture.SNAPSHOT_PACKAGE) if mode == "candidate" else None,
        "candidateSnapshotManifestSha256": (
            sha256(capture.SNAPSHOT_MANIFEST) if mode == "candidate" else None
        ),
        "candidateSourceSha256": provenance["candidateSourceSha256"],
        "result": str(result),
        "resultSha256": base.sha256(result),
        "provenanceSha256": base.sha256(provenance_path),
        "guardReceiptSha256": base.sha256(receipt_path),
        "logSha256": base.sha256(log_path),
        "exitCode": provenance["exitCode"],
        "selected": selected,
    }
    if mode == "candidate":
        strict["pairedOracleStrictSha256"] = base.sha256(tmp_path / "oracle-v1.strict.json")
    sidecar = result.with_suffix(".strict.json")
    _write_json(sidecar, strict)
    return sidecar


def _pair(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> tuple[Path, Path]:
    package = tmp_path / "live-package"
    (package / "src").mkdir(parents=True)
    (package / "src/index.cjs").write_text("module.exports = {};\n")
    (package / "package.json").write_text('{"name":"fixture"}\n')
    artifact = package / "native" / capture.PROFILE / "pwright.node"
    artifact.parent.mkdir(parents=True)
    artifact.write_bytes(TEST_ARTIFACT)
    monkeypatch.setattr(capture, "RESULT_PARENT", tmp_path)
    monkeypatch.setattr(capture, "ORACLE_SIDECAR", tmp_path / "oracle-v1.strict.json")
    monkeypatch.setattr(capture, "SOURCE_PACKAGE", package)
    monkeypatch.setattr(capture, "SNAPSHOT_PACKAGE", tmp_path / "candidate-package-snapshot-v1")
    monkeypatch.setattr(
        capture,
        "SNAPSHOT_MANIFEST",
        tmp_path / "candidate-package-snapshot-v1.files.json",
    )
    monkeypatch.setattr(capture, "ARTIFACT_SHA256", TEST_SHA256)
    create_snapshot(
        package,
        capture.SNAPSHOT_PACKAGE,
        capture.SNAPSHOT_MANIFEST,
        profile=capture.PROFILE,
        artifact_sha256=TEST_SHA256,
    )

    def fake_provenance(raw: object, **kwargs: object) -> dict:
        assert isinstance(raw, dict)
        assert kwargs["candidate_override"] == (
            capture.SNAPSHOT_PACKAGE if raw["mode"] == "candidate" else None
        )
        return raw

    monkeypatch.setattr(verifier, "verify_provenance", fake_provenance)
    oracle = _lane(tmp_path, "oracle")
    candidate = _lane(tmp_path, "candidate")
    return oracle, candidate


def test_refuses_capture_without_final_artifact_sha(monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    monkeypatch.setattr(capture, "ARTIFACT_SHA256", "")

    # Act
    with pytest.raises(ValueError, match="awaits a pinned final candidate artifact"):
        capture.check_artifact()

    # Assert
    assert capture.ARTIFACT_SHA256 == ""


def test_refuses_artifact_digest_drift(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    artifact = tmp_path / "pwright.node"
    artifact.write_bytes(b"test artifact")
    monkeypatch.setattr(capture, "ARTIFACT_SHA256", TEST_SHA256)

    # Act
    with pytest.raises(ValueError, match="SHA256 changed"):
        capture.check_artifact(artifact)

    # Assert
    assert artifact.is_file()


def test_oracle_reaches_official_runner_without_candidate_artifact(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    monkeypatch.setattr(capture, "RESULT_PARENT", tmp_path)
    monkeypatch.setattr(capture, "ARTIFACT_SHA256", TEST_SHA256)
    monkeypatch.setattr(capture, "_check_environment", lambda: None)
    monkeypatch.setattr(capture, "check_manifest", lambda: None)

    def forbidden_artifact_check() -> None:
        raise AssertionError("oracle consulted candidate artifact")

    def reached_official_runner(*_args: object, **_kwargs: object) -> None:
        raise RuntimeError("official runner reached")

    monkeypatch.setattr(capture, "check_artifact", forbidden_artifact_check)
    monkeypatch.setattr(capture.subprocess, "run", reached_official_runner)

    # Act
    with pytest.raises(RuntimeError, match="official runner reached"):
        capture.run("oracle", tmp_path / "oracle-v1.json")

    # Assert
    assert not (tmp_path / "oracle-v1.json").exists()
    assert (tmp_path / "oracle-v1.guard-receipt.jsonl").is_file()


def test_snapshot_source_remains_verifiable_after_live_source_changes(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    _, candidate = _pair(tmp_path, monkeypatch)
    (capture.SOURCE_PACKAGE / "src/index.cjs").write_text("module.exports = {changed:true};\n")

    # Act
    verified = verifier.verify_strict(candidate)

    # Assert
    assert verified["candidateSourceSha256"] != _candidate_source_sha256(capture.SOURCE_PACKAGE)


def test_accepts_exact_source_bound_oracle_candidate_pair(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    oracle, candidate = _pair(tmp_path, monkeypatch)

    # Act
    verified = verifier.verify_strict(candidate)

    # Assert
    assert verified["pairedOracleStrictSha256"] == base.sha256(oracle)
    assert [item["status"] for item in verified["selected"]] == ["unexpected"] * 3


def test_rejects_candidate_paired_to_wrong_oracle_hash(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    _, candidate = _pair(tmp_path, monkeypatch)
    strict = json.loads(candidate.read_text())
    strict["pairedOracleStrictSha256"] = "0" * 64
    _write_json(candidate, strict)

    # Act
    with pytest.raises(AssertionError):
        verifier.verify_strict(candidate)

    # Assert
    assert strict["pairedOracleStrictSha256"] != base.sha256(capture.ORACLE_SIDECAR)


def test_rejects_candidate_worker_receipt_tamper_even_when_rehashed(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    _, candidate = _pair(tmp_path, monkeypatch)
    strict = json.loads(candidate.read_text())
    receipt = Path(strict["result"]).with_suffix(".guard-receipt.jsonl")
    records = [json.loads(line) for line in receipt.read_text().splitlines()]
    records[-1]["fixtureHost"] = "other.tail8f3b66.ts.net"
    receipt.write_text("".join(json.dumps(record) + "\n" for record in records))
    strict["guardReceiptSha256"] = base.sha256(receipt)
    _write_json(candidate, strict)

    # Act
    with pytest.raises(AssertionError):
        verifier.verify_strict(candidate)

    # Assert
    assert records[-1]["workerIndex"] == "2"


def test_rejects_snapshot_source_tamper_even_when_manifest_rehashed(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    _, candidate = _pair(tmp_path, monkeypatch)
    strict = json.loads(candidate.read_text())
    source = capture.SNAPSHOT_PACKAGE / "src/index.cjs"
    source.write_text("module.exports = {tampered:true};\n")
    strict["candidateSnapshotManifestSha256"] = sha256(capture.SNAPSHOT_MANIFEST)
    _write_json(candidate, strict)

    # Act
    with pytest.raises(ValueError, match="candidate snapshot file changed"):
        verifier.verify_strict(candidate)

    # Assert
    assert source.is_file()


def test_rejects_extra_snapshot_source_file(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    _, candidate = _pair(tmp_path, monkeypatch)
    extra = capture.SNAPSHOT_PACKAGE / "src/injected.cjs"
    extra.write_text("module.exports = true;\n")

    # Act
    with pytest.raises(ValueError, match="missing or extra files"):
        verifier.verify_strict(candidate)

    # Assert
    assert extra.is_file()


def test_rejects_snapshot_symlink(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    _, candidate = _pair(tmp_path, monkeypatch)
    link = capture.SNAPSHOT_PACKAGE / "src/alias.cjs"
    link.symlink_to("index.cjs")

    # Act
    with pytest.raises(ValueError, match="contains a symlink"):
        verifier.verify_strict(candidate)

    # Assert
    assert link.is_symlink()


def test_rejects_oracle_failure_even_when_report_and_hashes_agree(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    oracle, _ = _pair(tmp_path, monkeypatch)
    strict = json.loads(oracle.read_text())
    result = Path(strict["result"])
    report = _report("candidate")
    _write_json(result, report)
    strict["resultSha256"] = base.sha256(result)
    strict["selected"] = worker_aware.selected_tests(report)
    strict["exitCode"] = 1
    provenance = result.with_suffix(".provenance.json")
    provenance_data = json.loads(provenance.read_text())
    provenance_data["resultSha256"] = strict["resultSha256"]
    provenance_data["exitCode"] = 1
    _write_json(provenance, provenance_data)
    strict["provenanceSha256"] = base.sha256(provenance)
    _write_json(oracle, strict)

    # Act
    with pytest.raises(AssertionError):
        verifier.verify_strict(oracle)

    # Assert
    assert strict["selected"][0]["status"] == "unexpected"
