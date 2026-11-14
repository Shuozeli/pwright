"""Assert that the immutable paired successor changes only three pinned rows."""

from __future__ import annotations

import json

import pytest
from promote_playwright_core_browser_type_owner_strict_v2 import (
    BASE,
    OUTPUT,
    SCOPE,
    TARGETS,
    prepare_lane,
    verify_existing,
)


@pytest.mark.parametrize("lane", ["candidate", "oracle"])
def test_only_three_pinned_rows_change(lane: str) -> None:
    # Arrange
    before = (BASE / f"{lane}.jsonl").read_bytes().splitlines(keepends=True)
    # Act
    successor, _, _ = prepare_lane(lane)
    # Assert
    after = successor.splitlines(keepends=True)
    assert len(before) == len(after) == 48_477
    changed = [
        index for index, (old, new) in enumerate(zip(before, after, strict=True)) if old != new
    ]
    assert changed == sorted(TARGETS)
    for index in changed:
        original = json.loads(before[index])
        promoted = json.loads(after[index])
        assert original["sourceClassification"] == promoted["sourceClassification"]
        assert original["classificationReasons"] == promoted["classificationReasons"]
        assert original["outcome"] == "unexpected"
        assert promoted["outcome"] == "expected"
        assert promoted["evidenceScope"] == SCOPE
        assert promoted["candidateRuntimeRequired"] is (lane == "candidate")
        assert "error" not in promoted


def test_published_bundle_reconstructs_byte_for_byte() -> None:
    # Act
    receipt = verify_existing()
    # Assert
    assert receipt["targetZeroBasedIndices"] == sorted(TARGETS)
    assert OUTPUT.is_dir()
