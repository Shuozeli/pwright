"""Bounded mutation controls for the independent navigation ledger verifier."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pytest
import verify_playwright_python_navigation_full_strict_paired_v1 as verify


def _line(row: dict[str, object]) -> bytes:
    return (json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n").encode()


def _fixture() -> tuple[
    list[bytes], list[bytes], list[dict[str, object]], dict[str, dict[str, object]]
]:
    inventory = [
        {
            "id": "tests/async/test_other.py::test_before[chromium]",
            "sourceClassification": "public-api-source",
        },
        {
            "id": "tests/async/test_navigation.py::test_navigation[chromium]",
            "sourceClassification": "public-api-source",
        },
        {
            "id": "tests/async/test_other.py::test_after[chromium]",
            "sourceClassification": "public-api-source",
        },
    ]
    base = [_line({**row, "outcome": "not-run", "attempts": 0}) for row in inventory]
    expected = {str(inventory[1]["id"]): {"outcome": "passed", "durationSeconds": 0.25}}
    successor = list(base)
    successor[1] = _line(
        {
            **inventory[1],
            **expected[str(inventory[1]["id"])],
            "attempts": 1,
            "resultFile": "/evidence/result.xml",
        }
    )
    return base, successor, inventory, expected


@pytest.mark.parametrize("mode", ("candidate", "oracle"))
def test_exact_reconciliation_is_accepted(mode: str) -> None:
    # Arrange
    base, successor, inventory, expected = _fixture()

    # Act
    counts = verify.check_reconciliation(
        base, successor, inventory, [1], expected, "/evidence/result.xml", mode
    )

    # Assert
    assert counts["passed"] == 1
    assert counts["not-run"] == 2


@pytest.mark.parametrize(
    "mutation",
    (
        "outcome",
        "duration",
        "attempts",
        "result-file",
        "extra-error",
        "missing-id",
        "duplicate-id",
        "off-target-byte",
    ),
)
def test_changed_ledger_is_rejected(mutation: str) -> None:
    # Arrange
    base, successor, inventory, expected = _fixture()
    target = json.loads(successor[1])
    if mutation == "outcome":
        target["outcome"] = "failed"
    elif mutation == "duration":
        target["durationSeconds"] = 0.5
    elif mutation == "attempts":
        target["attempts"] = 2
    elif mutation == "result-file":
        target["resultFile"] = "/other/result.xml"
    elif mutation == "extra-error":
        target["error"] = "fabricated failure"
    elif mutation == "missing-id":
        del target["id"]
    elif mutation == "duplicate-id":
        target["id"] = inventory[0]["id"]
    elif mutation == "off-target-byte":
        # Identical JSON meaning is insufficient: untouched rows retain raw bytes.
        successor[0] = b" " + successor[0]
    successor[1] = _line(target)

    # Act / Assert
    with pytest.raises(verify.NavigationPromotionValidationError):
        verify.check_reconciliation(
            base,
            successor,
            inventory,
            [1],
            expected,
            "/evidence/result.xml",
            "candidate",
        )


def test_failed_target_preserves_recorded_failure() -> None:
    # Arrange
    base, successor, inventory, expected = _fixture()
    case_id = str(inventory[1]["id"])
    expected[case_id] = {
        "outcome": "failed",
        "durationSeconds": 0.25,
        "error": "assertion failed",
    }
    successor[1] = _line(
        {
            **inventory[1],
            **expected[case_id],
            "attempts": 1,
            "resultFile": "/evidence/result.xml",
        }
    )

    # Act
    counts = verify.check_reconciliation(
        base, successor, inventory, [1], expected, "/evidence/result.xml", "candidate"
    )

    # Assert
    assert counts["failed"] == 1


def _scope() -> dict[str, object]:
    return {
        "schema": 2,
        "kind": "playwright-python-navigation-full-chromium-strict-paired-promotion-v1",
        "scope": "tests/async/test_navigation.py-chromium-full-file-80",
        "publicApiParityEvidence": False,
        "fullCorpusExecution": False,
        "completeFileExecution": True,
        "fixtureAdapted": True,
        "launchMode": "cdp-adapter",
        "chromeVersionObserved": "155.0.8059.85",
    }


def test_bounded_adapted_scope_is_accepted() -> None:
    # Arrange
    receipt = _scope()

    # Act
    result = verify.check_receipt_scope(receipt)

    # Assert
    assert result is None


@pytest.mark.parametrize(
    ("field", "value"),
    (
        ("schema", 1),
        ("kind", "other"),
        ("scope", "all-python"),
        ("publicApiParityEvidence", True),
        ("fullCorpusExecution", True),
        ("completeFileExecution", False),
        ("fixtureAdapted", False),
        ("launchMode", "native"),
        ("chromeVersionObserved", "other"),
    ),
)
def test_false_scope_claim_is_rejected(field: str, value: object) -> None:
    # Arrange
    receipt = {**_scope(), field: value}

    # Act / Assert
    with pytest.raises(verify.NavigationPromotionValidationError):
        verify.check_receipt_scope(receipt)


def test_sealed_sidecar_is_accepted(tmp_path: Path) -> None:
    # Arrange
    payload = b'{"wheelSha256":"fixed-fixture"}\n'
    path = tmp_path / "source-pins.json"
    path.write_bytes(payload)
    digest = hashlib.sha256(payload).hexdigest()

    # Act
    observed = verify.sealed(path, digest, "source pins")

    # Assert
    assert observed == payload


def test_changed_sidecar_is_rejected(tmp_path: Path) -> None:
    # Arrange
    payload = b'{"wheelSha256":"fixed-fixture"}\n'
    path = tmp_path / "source-pins.json"
    digest = hashlib.sha256(payload).hexdigest()
    path.write_bytes(payload + b" ")

    # Act / Assert
    with pytest.raises(verify.NavigationPromotionValidationError):
        verify.sealed(path, digest, "source pins")
