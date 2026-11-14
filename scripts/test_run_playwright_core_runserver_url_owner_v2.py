"""Fail-closed checks for the separate v2 browserless diagnostic envelope."""

from __future__ import annotations

import json
from pathlib import Path

import pytest
import run_playwright_core_runserver_url_owner_v2 as v2


def identity(mode: str) -> tuple[dict[str, object], dict[str, object]]:
    envelope: dict[str, object] = {
        "mode": mode,
        "upstream": "/pinned/upstream",
        "result": "/pinned/result.json",
        "runnerExitCode": 0,
        "bindIp": "100.78.97.63",
        "magicDnsHost": "macbookpro-yuacx.tail8f3b66.ts.net",
    }
    raw: dict[str, object] = {
        "mode": mode,
        "suite": v2.SUITE,
        "suiteRole": "browser-independent-runserver-url-only-owner-fixture-diagnostic",
        "manifest": str(v2.MANIFEST),
        "upstream": envelope["upstream"],
        "result": envelope["result"],
        "exitCode": 0,
        "fixtureBind": envelope["bindIp"],
        "fixtureHost": envelope["magicDnsHost"],
        "candidateProfile": v2.PROFILE if mode == "candidate" else None,
        "candidate": str(v2.ROOT / "packages/pwright") if mode == "candidate" else None,
    }
    return raw, envelope


@pytest.mark.parametrize("mode", ["candidate", "oracle"])
def test_raw_identity_accepts_exact_bind_host_and_profile(mode: str) -> None:
    # Arrange
    raw, envelope = identity(mode)

    # Act
    v2.check_raw_identity(raw, envelope)

    # Assert
    assert raw["fixtureBind"] == envelope["bindIp"]


@pytest.mark.parametrize("field", ["fixtureBind", "fixtureHost", "candidateProfile"])
def test_raw_identity_rejects_mutated_binding_or_profile(field: str) -> None:
    # Arrange
    raw, envelope = identity("candidate")
    raw[field] = "forged"

    # Act / Assert
    with pytest.raises(ValueError, match="raw provenance"):
        v2.check_raw_identity(raw, envelope)


def test_oracle_cannot_inherit_owner_preload() -> None:
    # Arrange
    oracle = v2.node_options("oracle")
    candidate = v2.node_options("candidate")

    # Act / Assert
    assert str(v2.OWNER) not in oracle
    assert candidate == f"{oracle} --require={v2.OWNER}"


@pytest.mark.parametrize(
    "environment",
    [
        {"NODE_OPTIONS": "--require=/untracked.cjs"},
        {"PWRIGHT_CANDIDATE_CDP_ENDPOINT": "http://127.0.0.1:9222"},
        {"PWRIGHT_CORE_BROWSER_TYPE_OWNER_RECEIPT": "/untracked/receipt"},
        {"PWRIGHT_OFFICIAL_RUNSERVER_EXPECT_BIND": "127.0.0.1"},
    ],
)
def test_ambient_execution_override_is_rejected(environment: dict[str, str]) -> None:
    # Arrange / Act / Assert
    with pytest.raises(ValueError, match="ambient"):
        v2.check_ambient(environment)


def test_owner_receipt_requires_candidate_worker(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    # Arrange
    monkeypatch.setattr(v2.base, "require_file", lambda _path, _parent: None)
    receipt = tmp_path / "owner.jsonl"
    record = {
        "pid": 123,
        "workerIndex": None,
        "candidate": "/candidate",
        "ownerBound": True,
    }
    receipt.write_text(json.dumps(record) + "\n")

    # Act / Assert
    with pytest.raises(ValueError, match="candidate worker"):
        v2.verify_owner_receipt(receipt, "/candidate")


def test_owner_receipt_rejects_unbound_worker(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    # Arrange
    monkeypatch.setattr(v2.base, "require_file", lambda _path, _parent: None)
    receipt = tmp_path / "owner.jsonl"
    record = {
        "pid": 123,
        "workerIndex": "0",
        "candidate": "/candidate",
        "ownerBound": False,
    }
    receipt.write_text(json.dumps(record) + "\n")

    # Act / Assert
    with pytest.raises(ValueError, match="fixture-owner receipt"):
        v2.verify_owner_receipt(receipt, "/candidate")
