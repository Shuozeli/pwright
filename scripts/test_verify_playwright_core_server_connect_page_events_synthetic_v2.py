"""Mutation controls for the positive Page-event wire attestation."""

from __future__ import annotations

import json

import pytest
from verify_playwright_core_server_connect_page_events_synthetic_v2 import (
    _verify_positive_receipt,
)

HOST = "macbookpro-yuacx.tail8f3b66.ts.net"
BIND = "100.78.97.63"


def _records(mode: str) -> list[dict]:
    common = {
        "mode": mode,
        "fixtureHost": HOST,
        "fixtureBind": BIND,
        "synthetic": True,
        "browserProcessLaunched": False,
        "pid": 101,
    }

    def event(kind: str, **extra: object) -> dict:
        return {**common, "kind": kind, **extra}

    records = [event(kind) for kind in (
        "listening", "upgrade", "root-initialize", "bootstrap-sent",
        "new-context-replied", "new-page-replied",
    )]
    records[0].update({
        "endpointPath": "/browser-synthetic-page-events-v2",
        "positiveProbeEnabled": True,
    })
    records.extend([
        event("subscription-replied", guid="page@1", event="request", enabled=True),
        event("subscription-replied", guid="page@1", event="response", enabled=True),
    ])
    if mode == "oracle":
        records.append(event("wait-event-info-replied", guid="page@1",
                             phase="before", event="download"))
    records.extend([
        event("positive-trigger", subscriptionsAcknowledged=["request", "response"],
              downloadWaitArmed=mode == "oracle"),
        event("positive-event-sent", event="request", pageGuid="page@1",
              requestGuid="request@positive", method="POST",
              url="https://example.test/report"),
        event("positive-event-sent", event="response", pageGuid="page@1",
              requestGuid="request@positive", responseGuid="response@positive",
              status=201, url="https://example.test/report"),
        event("positive-event-sent", event="download", pageGuid="page@1",
              artifactGuid="artifact@positive", url="https://example.test/report.csv",
              suggestedFilename="report.csv"),
        event("close", phase=3), event("stop-requested"), event("stop"),
    ])
    return records


def _check(tmp_path, mode: str, records: list[dict]) -> dict:
    receipt = tmp_path / "positive.receipt.jsonl"
    receipt.write_text("".join(json.dumps(item) + "\n" for item in records))
    return _verify_positive_receipt(receipt, mode, HOST, BIND)


@pytest.mark.parametrize("mode", ["oracle", "candidate"])
def test_complete_positive_wire_is_accepted(tmp_path, mode: str) -> None:
    # Arrange
    records = _records(mode)

    # Act
    evidence = _check(tmp_path, mode, records)

    # Assert
    assert evidence["eventOrder"] == ["request", "response", "download"]
    assert evidence["subscriptionsAcknowledgedBeforePublication"] is True


def test_missing_response_subscription_is_rejected(tmp_path) -> None:
    # Arrange
    records = [item for item in _records("candidate") if not (
        item["kind"] == "subscription-replied" and item["event"] == "response"
    )]

    # Act / Assert
    with pytest.raises(ValueError, match="subscriptions were not acknowledged"):
        _check(tmp_path, "candidate", records)


def test_mismatched_request_response_identity_is_rejected(tmp_path) -> None:
    # Arrange
    records = _records("candidate")
    response = next(item for item in records if item.get("event") == "response"
                    and item["kind"] == "positive-event-sent")
    response["requestGuid"] = "request@other"

    # Act / Assert
    with pytest.raises(ValueError, match="wire identities changed"):
        _check(tmp_path, "candidate", records)


def test_missing_download_delivery_is_rejected(tmp_path) -> None:
    # Arrange
    records = [item for item in _records("oracle") if not (
        item["kind"] == "positive-event-sent" and item["event"] == "download"
    )]

    # Act / Assert
    with pytest.raises(ValueError, match="missing or out of order"):
        _check(tmp_path, "oracle", records)
