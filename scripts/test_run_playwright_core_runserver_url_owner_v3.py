"""Fail-closed checks for the isolated v3 RunServer diagnostic envelope."""

from __future__ import annotations

import json
from hashlib import sha256
from pathlib import Path

import pytest
import run_playwright_core_runserver_url_owner_v3 as v3
from run_playwright_official_conformance import _candidate_source_sha256


def identity(mode: str) -> tuple[dict[str, object], dict[str, object]]:
    harness_inputs = v3.expected_raw_harness_inputs(mode)
    snapshot = {
        "artifactRoot": "/isolated/artifact",
        "addon": "/isolated/artifact/pwright.node",
        "addonSha256": "a" * 64,
        "candidate": str(v3.CANDIDATE),
        "candidateSourceSha256": "b" * 64,
        "candidateArtifact": str(v3.CANDIDATE / "native" / v3.PROFILE / "pwright.node"),
        "candidateArtifactSha256": "a" * 64,
    }
    candidate = mode == "candidate"
    envelope: dict[str, object] = {
        "mode": mode,
        "upstream": "/pinned/upstream",
        "result": "/pinned/result.json",
        "runnerExitCode": 0,
        "resultSha256": "c" * 64,
        "bindIp": "100.78.97.63",
        "magicDnsHost": "macbookpro-yuacx.tail8f3b66.ts.net",
        "snapshot": snapshot if candidate else None,
    }
    raw: dict[str, object] = {
        "mode": mode,
        "suite": v3.SUITE,
        "suiteRole": "browser-independent-runserver-url-only-owner-fixture-diagnostic",
        "manifest": str(v3.MANIFEST),
        "manifestSha256": v3.digest(v3.MANIFEST),
        "upstream": envelope["upstream"],
        "result": envelope["result"],
        "resultSha256": envelope["resultSha256"],
        "exitCode": envelope["runnerExitCode"],
        "fixtureBind": envelope["bindIp"],
        "fixtureHost": envelope["magicDnsHost"],
        "candidateProfile": v3.PROFILE if candidate else None,
        "candidate": snapshot["candidate"] if candidate else None,
        "candidateSourceSha256": snapshot["candidateSourceSha256"]
        if candidate
        else None,
        "candidateArtifact": snapshot["candidateArtifact"] if candidate else None,
        "candidateArtifactSha256": snapshot["candidateArtifactSha256"]
        if candidate
        else None,
        "harnessInputs": harness_inputs,
    }
    return raw, envelope


@pytest.mark.parametrize("mode", ["candidate", "oracle"])
def test_raw_identity_accepts_exact_isolated_selection(mode: str) -> None:
    # Arrange
    raw, envelope = identity(mode)

    # Act
    v3.check_raw_identity(raw, envelope)

    # Assert
    assert raw["candidate"] == (str(v3.CANDIDATE) if mode == "candidate" else None)


@pytest.mark.parametrize(
    "field",
    [
        "candidate",
        "candidateArtifact",
        "candidateSourceSha256",
        "candidateArtifactSha256",
        "harnessInputs",
        "fixtureBind",
        "fixtureHost",
        "candidateProfile",
    ],
)
def test_raw_identity_rejects_candidate_or_runtime_provenance_mutation(
    field: str,
) -> None:
    # Arrange
    raw, envelope = identity("candidate")
    raw[field] = "forged"

    # Act / Assert
    with pytest.raises(ValueError, match="raw provenance"):
        v3.check_raw_identity(raw, envelope)


@pytest.mark.parametrize(
    "field",
    [
        "candidate",
        "candidateArtifact",
        "candidateSourceSha256",
        "candidateArtifactSha256",
        "candidateProfile",
    ],
)
def test_oracle_rejects_candidate_provenance(field: str) -> None:
    # Arrange
    raw, envelope = identity("oracle")
    raw[field] = "injected"

    # Act / Assert
    with pytest.raises(ValueError, match="raw provenance"):
        v3.check_raw_identity(raw, envelope)


def test_oracle_excludes_owner_preload() -> None:
    # Arrange
    oracle = v3.node_options("oracle")
    candidate = v3.node_options("candidate")

    # Act / Assert
    assert str(v3.base.OWNER) not in oracle
    assert candidate == f"{oracle} --require={v3.base.OWNER}"


def test_raw_harness_inputs_exclude_candidate_preload_from_oracle() -> None:
    # Arrange
    oracle = v3.expected_raw_harness_inputs("oracle")
    candidate = v3.expected_raw_harness_inputs("candidate")

    # Act / Assert
    assert oracle == [
        {
            "path": str(v3.SERVER_RUNTIME),
            "sha256": v3.base.base.digest(v3.SERVER_RUNTIME),
        }
    ]
    assert candidate == oracle + [
        {
            "path": str(v3.CANDIDATE_PRELOAD),
            "sha256": v3.base.base.digest(v3.CANDIDATE_PRELOAD),
        }
    ]


def test_raw_identity_rejects_mutated_harness_input_path() -> None:
    # Arrange
    raw, envelope = identity("candidate")
    raw["harnessInputs"][0]["path"] = "/untracked/server-runtime.cjs"

    # Act / Assert
    with pytest.raises(ValueError, match="raw provenance"):
        v3.check_raw_identity(raw, envelope)


@pytest.mark.parametrize(
    "environment",
    [
        {"NODE_OPTIONS": "--require=/untracked.cjs"},
        {"PWRIGHT_NODE_CANDIDATE": "/other/package"},
        {"PWRIGHT_NATIVE_PROFILE": "other-profile"},
        {"PWRIGHT_CORE_BROWSER_TYPE_OWNER_RECEIPT": "/untracked/receipt"},
        {"PWRIGHT_OFFICIAL_RUNSERVER_EXPECT_BIND": "127.0.0.1"},
    ],
)
def test_ambient_execution_override_is_rejected(environment: dict[str, str]) -> None:
    # Arrange / Act / Assert
    with pytest.raises(ValueError, match="ambient"):
        v3.check_ambient(environment)


def test_owner_receipt_requires_candidate_worker(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    # Arrange
    monkeypatch.setattr(v3.base.base, "require_file", lambda _path, _parent: None)
    receipt = tmp_path / "owner.jsonl"
    receipt.write_text(
        json.dumps(
            {
                "pid": 123,
                "workerIndex": None,
                "candidate": str(v3.CANDIDATE),
                "ownerBound": True,
            }
        )
        + "\n"
    )

    # Act / Assert
    with pytest.raises(ValueError, match="candidate worker"):
        v3.verify_owner_receipt(receipt, str(v3.CANDIDATE))


def test_owner_receipt_rejects_wrong_candidate(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    # Arrange
    monkeypatch.setattr(v3.base.base, "require_file", lambda _path, _parent: None)
    receipt = tmp_path / "owner.jsonl"
    receipt.write_text(
        json.dumps(
            {
                "pid": 123,
                "workerIndex": "0",
                "candidate": "/another/package",
                "ownerBound": True,
            }
        )
        + "\n"
    )

    # Act / Assert
    with pytest.raises(ValueError, match="fixture-owner receipt"):
        v3.verify_owner_receipt(receipt, str(v3.CANDIDATE))


@pytest.mark.parametrize("mutation", ["line", "title", "projectId"])
def test_unchanged_official_row_rejects_selection_mutation(
    mutation: str, tmp_path: Path
) -> None:
    # Arrange
    report: dict[str, object] = {
        "suites": [
            {
                "specs": [
                    {
                        "id": "pinned-id",
                        "file": "library/browsertype-connect.spec.ts",
                        "line": 1046,
                        "title": "should refuse connecting when versions do not match",
                        "tests": [
                            {
                                "projectId": "chromium-library",
                                "status": "expected",
                                "results": [{"status": "passed"}],
                            }
                        ],
                    }
                ]
            }
        ]
    }
    result = tmp_path / "result.json"
    result.write_text(json.dumps(report))

    # Act
    selected = v3.selected_result(result)

    # Assert
    assert selected == ("pinned-id", "expected")
    spec = report["suites"][0]["specs"][0]
    if mutation == "projectId":
        spec["tests"][0]["projectId"] = "firefox-library"
    elif mutation == "line":
        spec["line"] = 1056
    else:
        spec["title"] = "changed assertion"
    result.write_text(json.dumps(report))
    with pytest.raises(ValueError, match="selection differs"):
        v3.selected_result(result)


def fake_artifact_snapshot(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> tuple[Path, Path, Path]:
    root = tmp_path / "artifact-root"
    candidate = root / "candidate-package"
    source = candidate / "src" / "index.cjs"
    artifact = candidate / "native" / v3.PROFILE / "pwright.node"
    addon = root / "pwright.node"
    source.parent.mkdir(parents=True)
    artifact.parent.mkdir(parents=True)
    (candidate / "package.json").write_text('{"name":"isolated-candidate"}\n')
    source.write_text("module.exports = {};\n")
    addon.write_bytes(b"pinned-native-addon")
    artifact.write_bytes(addon.read_bytes())
    digest = sha256(addon.read_bytes()).hexdigest()
    monkeypatch.setattr(v3, "ARTIFACT_ROOT", root)
    monkeypatch.setattr(v3, "ADDON", addon)
    monkeypatch.setattr(v3, "CANDIDATE", candidate)
    monkeypatch.setattr(v3, "CANDIDATE_ARTIFACT", artifact)
    monkeypatch.setattr(v3, "EXPECTED_ADDON_SHA256", digest)
    monkeypatch.setattr(
        v3, "EXPECTED_SOURCE_SHA256", _candidate_source_sha256(candidate)
    )
    return addon, source, artifact


def test_artifact_snapshot_accepts_matching_isolated_source_and_addon(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    # Arrange
    addon, _source, artifact = fake_artifact_snapshot(monkeypatch, tmp_path)

    # Act
    snapshot = v3.check_artifact_snapshot()

    # Assert
    assert snapshot == {
        "artifactRoot": str(v3.ARTIFACT_ROOT),
        "addon": str(addon),
        "addonSha256": v3.EXPECTED_ADDON_SHA256,
        "candidate": str(v3.CANDIDATE),
        "candidateSourceSha256": v3.EXPECTED_SOURCE_SHA256,
        "candidateArtifact": str(artifact),
        "candidateArtifactSha256": v3.EXPECTED_ADDON_SHA256,
    }


@pytest.mark.parametrize("mutation", ["addon", "candidate-artifact", "source"])
def test_artifact_snapshot_rejects_mutated_addon_or_source(
    mutation: str, monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    # Arrange
    addon, source, artifact = fake_artifact_snapshot(monkeypatch, tmp_path)
    target = {"addon": addon, "candidate-artifact": artifact, "source": source}[
        mutation
    ]
    target.write_bytes(target.read_bytes() + b"forged")

    # Act / Assert
    with pytest.raises((ValueError, AssertionError)):
        v3.check_artifact_snapshot()


@pytest.mark.parametrize("mode", ["candidate", "oracle"])
def test_immutable_destination_rejects_an_existing_result(
    mode: str, monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    # Arrange
    monkeypatch.setattr(v3, "RESULT_ROOT", tmp_path)
    result = tmp_path / "strict-pair" / f"{mode}.json"
    result.parent.mkdir()
    result.write_text("sealed\n")

    # Act / Assert
    with pytest.raises(ValueError, match="destination"):
        v3.check_immutable_destination(mode, result)


def test_immutable_destination_rejects_a_dangling_symlink(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    # Arrange
    monkeypatch.setattr(v3, "RESULT_ROOT", tmp_path)
    result = tmp_path / "strict-pair" / "candidate.json"
    result.parent.mkdir()
    result.symlink_to(tmp_path / "missing-result.json")

    # Act / Assert
    with pytest.raises(ValueError, match="v3 result path"):
        v3.check_immutable_destination("candidate", result)


def test_immutable_destination_rejects_an_existing_provenance_sibling(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    # Arrange
    monkeypatch.setattr(v3, "RESULT_ROOT", tmp_path)
    result = tmp_path / "strict-pair" / "candidate.json"
    result.parent.mkdir()
    result.with_suffix(".provenance.json").write_text("sealed\n")

    # Act / Assert
    with pytest.raises(ValueError, match="destination"):
        v3.check_immutable_destination("candidate", result)


def test_immutable_destination_rejects_a_noncanonical_result_name(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    # Arrange
    monkeypatch.setattr(v3, "RESULT_ROOT", tmp_path)
    result = tmp_path / "strict-pair" / "candidate-retry.json"

    # Act / Assert
    with pytest.raises(ValueError, match="named child"):
        v3.check_immutable_destination("candidate", result)


def test_immutable_destination_rejects_result_outside_v3_root(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    # Arrange
    monkeypatch.setattr(v3, "RESULT_ROOT", tmp_path / "v3-results")
    result = tmp_path / "other-results" / "strict-pair" / "candidate.json"

    # Act / Assert
    with pytest.raises(ValueError, match="named child"):
        v3.check_immutable_destination("candidate", result)


def test_verify_envelope_rejects_unexpected_fields(tmp_path: Path) -> None:
    # Arrange
    envelope = tmp_path / "candidate.runserver-url-owner-v3-attestation.json"
    envelope.write_text(json.dumps({"schema": 3, "untracked": True}))

    # Act / Assert
    with pytest.raises(ValueError, match="envelope shape"):
        v3.verify_envelope(envelope)


def test_verify_envelope_rejects_a_symlink(tmp_path: Path) -> None:
    # Arrange
    target = tmp_path / "target.json"
    target.write_text("{}")
    envelope = tmp_path / "candidate.runserver-url-owner-v3-attestation.json"
    envelope.symlink_to(target)

    # Act / Assert
    with pytest.raises(ValueError, match="missing or linked"):
        v3.verify_envelope(envelope)
