"""Mutation controls for the independent four-case remote-kill verifier."""

from __future__ import annotations

import copy
import json
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import Any

import pytest
import verify_playwright_python_connect_remote_kill_synthetic_v1 as verify

HOST = "macbookpro-yuacx.tail8f3b66.ts.net"
BIND = "100.78.97.63"


def make_junit(tmp_path: Path, statuses: tuple[str, ...]) -> Path:
    suite = ET.Element(
        "testsuite", tests="4", failures=str(statuses.count("failed")),
        errors="0", skipped="0",
    )
    for case_id, status in zip(verify.CASE_IDS, statuses, strict=True):
        api = case_id.split("/")[1]
        case = ET.SubElement(
            suite, "testcase", classname=f"tests.{api}.test_browsertype_connect",
            name=case_id.split("::", 1)[1],
        )
        if status == "failed":
            ET.SubElement(case, "failure", message="original upstream assertion failed")
    path = tmp_path / "cases.xml"
    ET.ElementTree(suite).write(path, encoding="unicode")
    return path


def make_receipt(site_packages: Path, mode: str, statuses: tuple[str, ...]) -> list[dict[str, Any]]:
    shared = {"mode": mode, "fixtureHost": HOST, "fixtureBind": BIND}
    public = (
        verify.SHIM / "playwright/sync_api.py"
        if mode == "candidate" else site_packages / "playwright/sync_api/__init__.py"
    )
    async_public = (
        verify.SHIM / "playwright/async_api.py"
        if mode == "candidate" else site_packages / "playwright/async_api/__init__.py"
    )
    records: list[dict[str, Any]] = [
        {**shared, "kind": "runtime-import", "publicApiPath": str(public),
         "candidatePackagePath": (
             str(site_packages / "pwright/__init__.py") if mode == "candidate" else None
         )},
        {**shared, "kind": "fixture-adapted", "fixture": "launch_server",
         "testSourceAdapted": False, "browserProcessLaunched": False,
         "upstreamCommit": verify.UPSTREAM_COMMIT},
        {**shared, "kind": "runtime-import-async", "publicApiPath": str(async_public)},
        {**shared, "kind": "selected", "caseIds": list(verify.CASE_IDS),
         "testSourceAdapted": False},
    ]
    for index, (case_id, status) in enumerate(zip(verify.CASE_IDS, statuses, strict=True)):
        pid = 1001 + index
        server = {**shared, "pid": pid, "synthetic": True, "browserProcessLaunched": False}
        records.extend([
            {**server, "kind": "listening"},
            {**shared, "kind": "fixture-created", "serverPid": pid,
             "fixtureAdapted": True, "testSourceAdapted": False,
             "browserProcessLaunched": False, "engine": "chromium",
             "endpointPath": "/browser-synthetic-python-close-events"},
            {**server, "kind": "upgrade"},
            {**server, "kind": "command", "guid": "", "method": "initialize"},
            {**server, "kind": "bootstrap-sent"},
            {**server, "kind": "command", "guid": "browser@1", "method": "newContext"},
            {**server, "kind": "command", "guid": "context@1", "method": "newPage"},
            {**server, "kind": "close", "phase": 3},
            {**server, "kind": "stop"},
            {**shared, "kind": "fixture-stopped", "serverPid": pid, "returnCode": 0},
            {**shared, "kind": "case-call", "caseId": case_id, "outcome": status},
        ])
    return records


def check_receipt(tmp_path: Path, mode: str, statuses: tuple[str, ...], records: list[dict[str, Any]]) -> dict[str, int]:
    path = tmp_path / "receipt.jsonl"
    path.write_text("".join(json.dumps(item) + "\n" for item in records))
    return verify.receipt(
        path, mode, dict(zip(verify.CASE_IDS, statuses, strict=True)),
        BIND, HOST, tmp_path / "venv/lib/python3.12/site-packages",
    )


@pytest.mark.parametrize(
    ("mode", "statuses"),
    [
        ("oracle", ("passed",) * 4),
        ("candidate", ("failed", "passed", "failed", "passed")),
    ],
)
def test_complete_receipt_including_red_case_protocol_is_accepted(
    tmp_path: Path, mode: str, statuses: tuple[str, ...]
) -> None:
    # Arrange
    records = make_receipt(tmp_path / "venv/lib/python3.12/site-packages", mode, statuses)

    # Act
    counts = check_receipt(tmp_path, mode, statuses, records)

    # Assert
    assert counts["case-call"] == counts["fixture-created"] == counts["close"] == 4
    assert counts["command"] == 12


@pytest.mark.parametrize(
    ("tamper", "message"),
    [
        ("missing-close", "server lifecycle receipt missing"),
        ("missing-page-command", "case protocol path changed"),
        ("early-kill", "case did not reach Page before remote kill"),
        ("out-of-order", "remote-kill fixture/call order changed"),
        ("wrong-case", "case call order changed"),
        ("wrong-outcome", "case call outcome differs from JUnit"),
        ("browser-launch", "synthetic server ownership changed"),
        ("wrong-fixture", "launch_server adaptation changed"),
        ("wrong-api", "candidate async API shim changed"),
        ("mixed-lane", "receipt lane or fixture address changed"),
    ],
)
def test_tampered_receipt_is_rejected_even_when_candidate_is_red(
    tmp_path: Path, tamper: str, message: str
) -> None:
    # Arrange
    statuses = ("failed", "passed", "failed", "passed")
    records = make_receipt(tmp_path / "venv/lib/python3.12/site-packages", "candidate", statuses)
    records = copy.deepcopy(records)
    if tamper == "missing-close":
        records.remove(next(item for item in records if item["kind"] == "close"))
    elif tamper == "missing-page-command":
        records.remove(next(item for item in records if item.get("method") == "newPage"))
    elif tamper == "early-kill":
        next(item for item in records if item["kind"] == "close")["phase"] = 2
    elif tamper == "out-of-order":
        close = next(item for item in records if item["kind"] == "close")
        call = next(item for item in records if item["kind"] == "case-call")
        close_index, call_index = records.index(close), records.index(call)
        records[close_index], records[call_index] = call, close
    elif tamper == "wrong-case":
        next(item for item in records if item["kind"] == "case-call")["caseId"] = "other"
    elif tamper == "wrong-outcome":
        next(item for item in records if item["kind"] == "case-call")["outcome"] = "passed"
    elif tamper == "browser-launch":
        next(item for item in records if item["kind"] == "close")["browserProcessLaunched"] = True
    elif tamper == "wrong-fixture":
        next(item for item in records if item["kind"] == "fixture-adapted")["testSourceAdapted"] = True
    elif tamper == "wrong-api":
        next(item for item in records if item["kind"] == "runtime-import-async")["publicApiPath"] = "/wrong/api.py"
    elif tamper == "mixed-lane":
        records[0]["mode"] = "oracle"

    # Act / Assert
    with pytest.raises(ValueError, match=message):
        check_receipt(tmp_path, "candidate", statuses, records)


def test_exact_four_original_junit_cases_preserve_red_and_green(tmp_path: Path) -> None:
    # Arrange
    statuses = ("failed", "passed", "failed", "passed")
    path = make_junit(tmp_path, statuses)

    # Act
    outcomes = verify.junit(path)

    # Assert
    assert outcomes == dict(zip(verify.CASE_IDS, statuses, strict=True))


@pytest.mark.parametrize(
    ("tamper", "message"),
    [
        ("skip", "JUnit skipped a selected case"),
        ("extra", "JUnit case count changed"),
        ("wrong-id", "JUnit selected case IDs changed"),
        ("accounting", "JUnit failure accounting changed"),
    ],
)
def test_junit_mutations_are_rejected(tmp_path: Path, tamper: str, message: str) -> None:
    # Arrange
    path = make_junit(tmp_path, ("passed",) * 4)
    root = ET.parse(path)
    suite = root.getroot()
    if tamper == "skip":
        ET.SubElement(suite.findall("testcase")[0], "skipped")
        suite.set("skipped", "1")
    elif tamper == "extra":
        ET.SubElement(suite, "testcase", classname="tests.sync.other", name="other")
    elif tamper == "wrong-id":
        suite.findall("testcase")[0].set("name", "test_other[chromium]")
    elif tamper == "accounting":
        suite.set("failures", "1")
    root.write(path, encoding="unicode")

    # Act / Assert
    with pytest.raises(ValueError, match=message):
        verify.junit(path)


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("fixtureAdapted", False),
        ("testSourceAdapted", True),
        ("publicApiParityEvidence", True),
        ("unadaptedOfficialPass", True),
        ("promotableToOfficialLedger", True),
        ("promotedToPythonLedger", True),
    ],
)
def test_manifest_cannot_claim_official_credit(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, field: str, value: bool
) -> None:
    # Arrange
    manifest = json.loads(verify.MANIFEST.read_text())
    manifest[field] = value
    tampered = tmp_path / "manifest.json"
    tampered.write_text(json.dumps(manifest))
    monkeypatch.setattr(verify, "MANIFEST", tampered)

    # Act / Assert
    with pytest.raises(ValueError, match="remote-kill manifest changed"):
        verify.verify_sources()
