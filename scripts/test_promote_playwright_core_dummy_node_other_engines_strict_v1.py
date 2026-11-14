"""Mutation and byte-preservation checks for the two-engine strict ledger gate."""

from __future__ import annotations

import io
import json
import sys
from itertools import zip_longest
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
import promote_playwright_core_dummy_node_other_engines_strict_v1 as promote  # noqa: E402


def test_exact_two_rows_per_lane_are_promoted_without_rewriting_others() -> None:
    # Arrange
    prior = promote.check_prior_bundle()
    evidence = promote.check_evidence()

    # Act
    outputs = {
        lane: promote.transform_lane(lane, evidence, prior[lane])
        for lane in promote.LANES
    }

    # Assert
    for lane, (payload, outcomes, scopes) in outputs.items():
        with (promote.BASE / f"{lane}.jsonl").open("rb") as baseline:
            for index, (before, after) in enumerate(
                zip_longest(baseline, io.BytesIO(payload))
            ):
                assert before is not None and after is not None
                if index not in promote.TARGET_INDEX.values():
                    assert after == before
                else:
                    engine = next(
                        name
                        for name, target in promote.TARGET_INDEX.items()
                        if target == index
                    )
                    row = json.loads(after)
                    assert row["specId"] == promote.SPEC_IDS[engine]
                    assert row["attempts"] == 1
                    assert row["outcome"] == (
                        "unexpected" if lane == "candidate" else "expected"
                    )
                    assert row["resultFile"] == evidence[(engine, lane)]["result"]
                    assert row["evidenceScope"] == promote.SCOPE
                    assert row["candidateRuntimeRequired"] is False
                    assert row["candidateNativeRuntimeInvoked"] is False
                    assert row["publicApiParityEvidence"] is False
            assert index + 1 == promote.COUNT
        assert sum(outcomes.values()) == promote.COUNT
        assert scopes[promote.SCOPE] == 2


def test_predecessor_receipt_hash_mutation_fails_closed(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(promote, "BASE_RECEIPT_SHA256", "0" * 64)

    # Act
    with pytest.raises(ValueError, match="predecessor receipt SHA-256 drifted"):
        promote.check_prior_bundle()

    # Assert
    assert promote.BASE_RECEIPT_SHA256 == "0" * 64


def test_strict_sidecar_hash_mutation_fails_closed(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    altered = {**promote.SIDECAR_SHA256, ("firefox", "candidate"): "0" * 64}
    monkeypatch.setattr(promote, "SIDECAR_SHA256", altered)

    # Act
    with pytest.raises(
        ValueError, match="firefox candidate strict sidecar SHA-256 drifted"
    ):
        promote.check_evidence()

    # Assert
    assert altered[("firefox", "candidate")] == "0" * 64


def test_verifier_source_hash_mutation_fails_closed(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(promote, "VERIFIER_SHA256", "0" * 64)

    # Act
    with pytest.raises(ValueError, match="independent strict verifier SHA-256 drifted"):
        promote.check_evidence()

    # Assert
    assert promote.VERIFIER_SHA256 == "0" * 64


def test_target_must_still_be_unrun() -> None:
    # Arrange
    with (
        promote.INVENTORY.open("rb") as inventory,
        (promote.BASE / "candidate.jsonl").open("rb") as baseline,
    ):
        for index, (source_line, row_line) in enumerate(zip(inventory, baseline)):
            if index == promote.TARGET_INDEX["firefox"]:
                source = json.loads(source_line)
                row = json.loads(row_line)
                break
    row["attempts"] = 1

    # Act
    with pytest.raises(ValueError, match="not the exact unrun public API row"):
        promote.target_row(source, row, "firefox")

    # Assert
    assert row["attempts"] == 1


def test_publication_refuses_existing_destination(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    # Arrange
    destination = tmp_path / "already-published"
    destination.mkdir()
    monkeypatch.setattr(promote, "OUTPUT", destination)

    # Act
    with pytest.raises(ValueError, match="successor already exists"):
        promote.publish({})

    # Assert
    assert destination.is_dir() and list(destination.iterdir()) == []


def test_verify_existing_accepts_exact_immutable_successor(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    # Arrange
    destination = tmp_path / "paired-successor"
    monkeypatch.setattr(promote, "OUTPUT", destination)
    files = promote.prepare_pair()
    destination.mkdir()
    for name, payload in files.items():
        (destination / name).write_bytes(payload)

    # Act
    receipt = promote.verify_existing()

    # Assert
    assert receipt["scope"] == promote.SCOPE
    assert receipt["lanes"]["candidate"]["newOutcome"] == "unexpected"
    assert receipt["lanes"]["oracle"]["newOutcome"] == "expected"


def test_verify_existing_rejects_mutated_successor(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    # Arrange
    destination = tmp_path / "paired-successor"
    monkeypatch.setattr(promote, "OUTPUT", destination)
    files = promote.prepare_pair()
    destination.mkdir()
    for name, payload in files.items():
        (destination / name).write_bytes(payload)
    with (destination / "candidate.jsonl").open("ab") as stream:
        stream.write(b"\n")

    # Act
    with pytest.raises(
        ValueError, match="immutable successor drifted: candidate.jsonl"
    ):
        promote.verify_existing()

    # Assert
    assert (destination / "candidate.jsonl").stat().st_size == len(
        files["candidate.jsonl"]
    ) + 1
