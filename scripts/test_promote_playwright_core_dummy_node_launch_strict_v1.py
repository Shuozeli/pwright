"""Positive and mutation controls for one-row Core dummy-Node promotion."""

from __future__ import annotations

import copy
import json
from pathlib import Path

import promote_playwright_core_dummy_node_launch_strict_v1 as promoter
import pytest


def _line(row: dict, *, spaced: bool = False) -> bytes:
    if spaced:
        return (json.dumps(row, separators=(", ", ": ")) + "\n").encode()
    return (json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n").encode()


def _target() -> dict:
    return {
        "classificationReasons": [],
        "column": 3,
        "expectedStatus": "passed",
        "file": promoter.SPEC_FILE,
        "line": 64,
        "projectId": "chromium-library",
        "projectName": "chromium-library",
        "role": "official-entrypoint",
        "sourceClassification": "public-api-candidate",
        "sourceFile": promoter.SOURCE_FILE,
        "specId": promoter.SPEC_ID,
        "suite": "library",
        "titlePath": [promoter.SPEC_FILE, promoter.TITLE],
    }


def _rows() -> tuple[list[bytes], list[bytes]]:
    first = {
        "specId": "first",
        "projectId": "chromium-library",
        "sourceClassification": "public-api-candidate",
        "classificationReasons": [],
    }
    last = {
        "specId": "last",
        "projectId": "chromium-library",
        "sourceClassification": "public-api-candidate",
        "classificationReasons": [],
    }
    inventory = [_line(first), _line(_target()), _line(last)]
    baseline = [
        _line({**first, "attempts": 0, "outcome": "not-run"}, spaced=True),
        _line({**_target(), "attempts": 0, "outcome": "not-run"}),
        _line(
            {**last, "attempts": 1, "outcome": "expected", "evidenceScope": "prior"},
            spaced=True,
        ),
    ]
    return inventory, baseline


def test_canonical_predecessor_is_exact() -> None:
    # Arrange / Act
    summaries = promoter.check_prior_bundle()

    # Assert
    assert set(summaries) == set(promoter.LANES)
    assert all(summary["records"] == promoter.COUNT for summary in summaries.values())


@pytest.mark.parametrize("lane", promoter.LANES)
def test_full_canonical_successor_changes_only_target_bytes(lane: str) -> None:
    # Arrange
    summary = promoter.check_prior_bundle()[lane]
    base = promoter.BASE / f"{lane}.jsonl"

    # Act
    with (
        promoter.INVENTORY.open("rb") as inventory_stream,
        base.open("rb") as base_stream,
    ):
        successor, outcomes, scopes = promoter.transform_lines(
            inventory_stream,
            base_stream,
            lane=lane,
            result_file=f"/strict/{lane}.json",
            count=promoter.COUNT,
            target_index=promoter.TARGET_INDEX,
            prior_outcomes=summary["outcomes"],
            prior_scopes=summary["explicitEvidenceScopes"],
        )

    # Assert
    changed = []
    with base.open("rb") as base_stream:
        for index, (before, after) in enumerate(
            zip(base_stream, successor.splitlines(keepends=True), strict=True)
        ):
            if before != after:
                changed.append(index)
    assert changed == [promoter.TARGET_INDEX]
    assert outcomes["not-run"] == summary["outcomes"]["not-run"] - 1
    assert outcomes["expected"] == summary["outcomes"]["expected"] + 1
    assert scopes[promoter.SCOPE] == 1


def _transform(inventory: list[bytes], baseline: list[bytes], lane: str):
    return promoter.transform_lines(
        inventory,
        baseline,
        lane=lane,
        result_file=f"/strict/{lane}.json",
        count=3,
        target_index=1,
        prior_outcomes={"not-run": 2, "expected": 1},
        prior_scopes={"prior": 1},
    )


@pytest.mark.parametrize("lane", promoter.LANES)
def test_exact_target_promotion_preserves_other_bytes(lane: str) -> None:
    # Arrange
    inventory, baseline = _rows()

    # Act
    successor, outcomes, scopes = _transform(inventory, baseline, lane)

    # Assert
    lines = successor.splitlines(keepends=True)
    assert len(lines) == 3
    assert lines[0] == baseline[0]
    assert lines[2] == baseline[2]
    assert lines[1] != baseline[1]
    assert json.loads(lines[1]) == {
        **_target(),
        "attempts": 1,
        "outcome": "expected",
        "resultFile": f"/strict/{lane}.json",
        "evidenceScope": promoter.SCOPE,
        "candidateRuntimeRequired": lane == "candidate",
    }
    assert outcomes == {"expected": 2, "not-run": 1}
    assert scopes == {"prior": 1, promoter.SCOPE: 1}


@pytest.mark.parametrize(
    "mutation",
    [
        "missing-target",
        "duplicate-target",
        "wrong-position",
        "wrong-classification",
        "prior-attempt",
        "prior-outcome",
        "prior-extra-field",
        "inventory-metadata",
        "truncated-ledger",
        "unterminated-row",
        "summary-count",
    ],
)
def test_row_mutations_fail_closed(mutation: str) -> None:
    # Arrange
    inventory, baseline = _rows()
    if mutation == "missing-target":
        baseline[1] = _line(
            {**_target(), "specId": "other", "attempts": 0, "outcome": "not-run"}
        )
    elif mutation == "duplicate-target":
        baseline[2] = _line({**_target(), "attempts": 1, "outcome": "expected"})
    elif mutation == "wrong-position":
        baseline[0], baseline[1] = baseline[1], baseline[0]
    elif mutation == "wrong-classification":
        row = json.loads(baseline[1])
        row["sourceClassification"] = "implementation-internal"
        baseline[1] = _line(row)
    elif mutation == "prior-attempt":
        row = json.loads(baseline[1])
        row["attempts"] = 1
        baseline[1] = _line(row)
    elif mutation == "prior-outcome":
        row = json.loads(baseline[1])
        row["outcome"] = "expected"
        baseline[1] = _line(row)
    elif mutation == "prior-extra-field":
        row = json.loads(baseline[1])
        row["resultFile"] = "/historical/diagnostic.json"
        baseline[1] = _line(row)
    elif mutation == "inventory-metadata":
        row = json.loads(inventory[1])
        row["line"] = 65
        inventory[1] = _line(row)
    elif mutation == "truncated-ledger":
        baseline.pop()
    elif mutation == "unterminated-row":
        baseline[1] = baseline[1].rstrip(b"\n")
    elif mutation == "summary-count":
        row = json.loads(baseline[2])
        row["outcome"] = "unexpected"
        baseline[2] = _line(row)

    # Act / Assert
    with pytest.raises(ValueError):
        _transform(inventory, baseline, "candidate")


def _strict(lane: str, result: Path) -> dict:
    manifest = json.loads(promoter.STRICT_MANIFEST.read_text())
    return {
        "schema": 2,
        "scope": promoter.STRICT_SCOPE,
        "mode": lane,
        "suite": promoter.SUITE,
        "specId": promoter.SPEC_ID,
        "selector": promoter.SELECTOR,
        "upstreamCommit": promoter.COMMIT,
        "upstreamSources": manifest["suite"]["upstreamSources"],
        "testStatus": "expected",
        "exitCode": 0,
        "publicApiParityEvidence": True,
        "diagnosticOnly": False,
        "upstreamAssertionsModified": False,
        "fixtureAdapted": True,
        "fixtureAdaptationEligible": True,
        "unadaptedOfficialPass": False,
        "ledgerEligible": True,
        "processOwnershipProof": {"verified": True},
        "runId": f"fresh-{lane}",
        "manifest": str(promoter.STRICT_MANIFEST),
        "manifestSha256": promoter.STRICT_MANIFEST_SHA256,
        "result": str(result),
        "resultSha256": promoter.digest(result.read_bytes()),
        "candidateArtifactSha256": promoter.CANDIDATE_ARTIFACT_SHA256
        if lane == "candidate"
        else None,
        "candidateSourceSha256": promoter.CANDIDATE_SOURCE_SHA256
        if lane == "candidate"
        else None,
        "candidateProfile": "playwright-trace" if lane == "candidate" else None,
    }


def _sidecars(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    sidecars = {}
    reports = {}
    for lane in promoter.LANES:
        result = tmp_path / f"{lane}.json"
        result.write_text('{"suites":[]}\n')
        sidecar = tmp_path / f"{lane}.strict.json"
        sidecar.write_text('{"schema":2}\n')
        sidecars[lane] = (sidecar, promoter.digest(sidecar.read_bytes()))
        reports[lane] = _strict(lane, result)
    monkeypatch.setattr(promoter, "verify_pair", lambda *_paths: reports)
    monkeypatch.setattr(promoter, "STRICT_SIDECARS", copy.deepcopy(sidecars))
    return sidecars, reports


def test_verified_eligible_pair_is_accepted(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    sidecars, reports = _sidecars(tmp_path, monkeypatch)

    # Act
    verified, hashes = promoter.check_pair(sidecars)

    # Assert
    assert verified == reports
    assert hashes == {lane: value[1] for lane, value in sidecars.items()}


@pytest.mark.parametrize(
    "field,value",
    [
        ("publicApiParityEvidence", False),
        ("diagnosticOnly", True),
        ("upstreamAssertionsModified", True),
        ("fixtureAdapted", False),
        ("fixtureAdaptationEligible", False),
        ("unadaptedOfficialPass", True),
        ("ledgerEligible", False),
        ("processOwnershipProof", {}),
        ("testStatus", "unexpected"),
        ("exitCode", 1),
        ("specId", "wrong"),
        ("scope", "historical-diagnostic"),
    ],
)
def test_ineligible_or_diagnostic_pair_is_rejected(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, field: str, value: object
) -> None:
    # Arrange
    sidecars, reports = _sidecars(tmp_path, monkeypatch)
    reports["candidate"][field] = value

    # Act / Assert
    with pytest.raises(ValueError):
        promoter.check_pair(sidecars)


@pytest.mark.parametrize(
    "mutation",
    [
        "same-run",
        "different-manifest",
        "missing-oracle",
        "same-sidecar",
        "drifted-sidecar",
        "linked-sidecar",
    ],
)
def test_pair_provenance_mutations_fail_closed(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, mutation: str
) -> None:
    # Arrange
    sidecars, reports = _sidecars(tmp_path, monkeypatch)
    if mutation == "same-run":
        reports["oracle"]["runId"] = reports["candidate"]["runId"]
    elif mutation == "different-manifest":
        reports["oracle"]["manifestSha256"] = "c" * 64
    elif mutation == "missing-oracle":
        del sidecars["oracle"]
    elif mutation == "same-sidecar":
        sidecars["oracle"] = sidecars["candidate"]
    elif mutation == "drifted-sidecar":
        sidecars["oracle"][0].write_text('{"schema":1}\n')
    else:
        path, sha = sidecars["oracle"]
        path.unlink()
        path.symlink_to(sidecars["candidate"][0])
        sidecars["oracle"] = (path, sha)

    # Act / Assert
    with pytest.raises(ValueError):
        promoter.check_pair(sidecars)


def test_immutable_publication_uses_no_replace(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    monkeypatch.setattr(promoter, "OUTCOMES", tmp_path)
    monkeypatch.setattr(promoter, "OUTPUT", tmp_path / "successor")
    files = {
        "candidate.jsonl": b"candidate\n",
        "oracle.jsonl": b"oracle\n",
        "promotion-receipt.json": (
            b'{"lanes":{"candidate":{"strictSidecar":"/candidate.strict.json",'
            b'"strictSidecarSha256":"' + b"a" * 64 + b'"},"oracle":{'
            b'"strictSidecar":"/oracle.strict.json","strictSidecarSha256":"'
            + b"b" * 64
            + b'"}},"scope":"synthetic"}\n'
        ),
    }
    monkeypatch.setattr(
        promoter, "prepare_pair", lambda _sidecars: copy.deepcopy(files)
    )

    # Act
    promoter.publish(files)

    # Assert
    assert promoter.OUTPUT.is_dir()
    assert {path.name for path in promoter.OUTPUT.iterdir()} == set(files)
    assert promoter.verify_existing()["scope"] == "synthetic"
    with pytest.raises(ValueError, match="already exists"):
        promoter.publish(files)
    (promoter.OUTPUT / "oracle.jsonl").write_bytes(b"tampered\n")
    with pytest.raises(ValueError, match="immutable successor drifted"):
        promoter.verify_existing()
