"""Negative controls for the official Python marker-skip diagnostic lane."""

from __future__ import annotations

import json
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import cast
from unittest.mock import patch

import pytest
import run_playwright_python_upstream_skip_callable_fixtures_strict_v2 as runner
import verify_playwright_python_upstream_skip_callable_fixtures_strict_v2 as strict

TEST_ID = strict.TEST_IDS[0]
NONCE = "a" * 32


def _receipt_payload(tmp_path: Path, events: list[dict[str, object]]) -> dict[str, object]:
    receipt = tmp_path / "one.call.jsonl"
    receipt.write_text(
        "".join(json.dumps(event, sort_keys=True) + "\n" for event in events),
        encoding="utf-8",
    )
    return {
        "callReceipt": str(receipt),
        "callReceiptSha256": strict.sha256_file(receipt),
        "mode": "candidate",
        "testId": TEST_ID,
        "runNonce": NONCE,
    }


def _junit_payload(tmp_path: Path) -> tuple[dict[str, object], ET.Element]:
    source, selection = TEST_ID.split("::", 1)
    root = ET.Element("testsuites")
    suite = ET.SubElement(root, "testsuite", tests="1", skipped="1", failures="0", errors="0")
    case = ET.SubElement(
        suite,
        "testcase",
        classname=source.removesuffix(".py").replace("/", "."),
        name=selection,
    )
    skipped = ET.SubElement(
        case,
        "skipped",
        type="pytest.skip",
        message="skipped for this browser: firefox",
    )
    skipped.text = "official upstream marker skip"
    result = tmp_path / "one.xml"
    ET.ElementTree(root).write(result, encoding="utf-8", xml_declaration=True)
    return {
        "result": str(result),
        "resultSha256": strict.sha256_file(result),
        "testId": TEST_ID,
        "exitCode": 0,
        "outcome": "skipped",
    }, root


def test_missing_setup_receipt_rejects_skip_credit(tmp_path: Path) -> None:
    # Arrange
    common = {"schema": 1, "mode": "candidate", "testId": TEST_ID, "runNonce": NONCE}
    payload = _receipt_payload(tmp_path, [{**common, "event": "collection", "selected": [TEST_ID]}])

    # Act / Assert
    with pytest.raises(
        strict.StrictUpstreamSkipEvidenceError, match="exactly collection and setup"
    ):
        strict._receipt(payload)


def test_wrong_upstream_skip_reason_is_rejected(tmp_path: Path) -> None:
    # Arrange
    payload, root = _junit_payload(tmp_path)
    skipped = root.find("./testsuite/testcase/skipped")
    assert skipped is not None
    skipped.set("message", "skipped by local patch")
    result = Path(cast(str, payload["result"]))
    ET.ElementTree(root).write(result, encoding="utf-8", xml_declaration=True)
    payload["resultSha256"] = strict.sha256_file(result)

    # Act / Assert
    with pytest.raises(strict.StrictUpstreamSkipEvidenceError, match="marker reason"):
        strict._junit(payload)


def test_upstream_source_hash_drift_is_rejected(tmp_path: Path) -> None:
    # Arrange
    source = tmp_path / "official_source.py"
    source.write_text("def test_official(): pass\n", encoding="utf-8")

    # Act / Assert
    with pytest.raises(strict.StrictUpstreamSkipEvidenceError, match="digest differs"):
        strict._digest_file(source, "0" * 64, "upstream source")


def test_candidate_oracle_different_node_is_rejected() -> None:
    # Arrange
    candidate = {
        "mode": "candidate",
        "testId": TEST_ID,
        "manifestSha256": "a" * 64,
        "fixtureBind": "100.64.0.1",
        "fixtureHost": "host.tail8f3b66.ts.net",
    }
    oracle = {**candidate, "mode": "oracle", "testId": strict.TEST_IDS[1]}

    # Act / Assert
    with (
        patch.object(strict, "verify_provenance", side_effect=[candidate, oracle]),
        pytest.raises(strict.StrictUpstreamSkipEvidenceError, match="exact paired"),
    ):
        strict.verify_pair(Path("candidate.json"), Path("oracle.json"))


def test_extra_junit_case_is_rejected(tmp_path: Path) -> None:
    # Arrange
    payload, root = _junit_payload(tmp_path)
    suite = root.find("testsuite")
    assert suite is not None
    ET.SubElement(suite, "testcase", classname="unexpected", name="extra")
    result = Path(cast(str, payload["result"]))
    ET.ElementTree(root).write(result, encoding="utf-8", xml_declaration=True)
    payload["resultSha256"] = strict.sha256_file(result)

    # Act / Assert
    with pytest.raises(strict.StrictUpstreamSkipEvidenceError, match="exactly one case"):
        strict._junit(payload)


def test_runner_never_overwrites_an_existing_output(tmp_path: Path) -> None:
    # Arrange
    result = tmp_path / "one.xml"
    result.write_text("keep this", encoding="utf-8")

    # Act / Assert
    with pytest.raises(strict.StrictUpstreamSkipEvidenceError, match="already exists"):
        runner._fresh_result(result)
    assert result.read_text(encoding="utf-8") == "keep this"
