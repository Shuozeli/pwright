"""Source-only negative controls for the Python connect close-event verifier."""

from __future__ import annotations

import json
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import Any

import pytest
import verify_playwright_python_connect_close_events_synthetic_v1 as verify

HOST = "macbookpro-yuacx.tail8f3b66.ts.net"
BIND = "100.78.97.63"
PID = 1001


def _receipt_records(site_packages: Path) -> list[dict[str, Any]]:
    shared = {"mode": "oracle", "fixtureHost": HOST, "fixtureBind": BIND}
    server = {
        **shared,
        "pid": PID,
        "synthetic": True,
        "browserProcessLaunched": False,
    }
    return [
        {
            **shared,
            "kind": "runtime-import",
            "publicApiPath": str(site_packages / "playwright/sync_api/__init__.py"),
            "candidatePackagePath": None,
        },
        {
            **shared,
            "kind": "fixture-adapted",
            "fixture": "launch_server",
            "testSourceAdapted": False,
            "browserProcessLaunched": False,
            "upstreamCommit": verify.UPSTREAM_COMMIT,
        },
        {**shared, "kind": "selected", "caseId": verify.CASE_ID},
        {
            **shared,
            "kind": "fixture-created",
            "serverPid": PID,
            "fixtureAdapted": True,
            "testSourceAdapted": False,
            "browserProcessLaunched": False,
            "engine": "chromium",
            "endpointPath": "/browser-synthetic-python-close-events",
        },
        {**server, "kind": "listening"},
        {**server, "kind": "upgrade"},
        {**server, "kind": "command", "guid": "", "method": "initialize"},
        {**server, "kind": "command", "guid": "browser@1", "method": "newContext"},
        {**server, "kind": "command", "guid": "context@1", "method": "newPage"},
        {**server, "kind": "bootstrap-sent"},
        {**server, "kind": "close", "phase": 3},
        {**shared, "kind": "case-call", "caseId": verify.CASE_ID, "outcome": "passed"},
        {**server, "kind": "stop"},
        {**shared, "kind": "fixture-stopped", "serverPid": PID, "returnCode": 0},
    ]


def _write_receipt(tmp_path: Path, records: list[dict[str, Any]]) -> Path:
    path = tmp_path / "receipt.jsonl"
    path.write_text("".join(json.dumps(record) + "\n" for record in records))
    return path


def _verify_receipt(tmp_path: Path, records: list[dict[str, Any]]) -> dict[str, int]:
    path = _write_receipt(tmp_path, records)
    return verify._receipt(
        path,
        mode="oracle",
        bind=BIND,
        host=HOST,
        site_packages=tmp_path / "venv/lib/python3.12/site-packages",
    )


def _junit_file(tmp_path: Path, *, tamper: str | None = None) -> Path:
    suite = ET.Element("testsuite", tests="1", failures="0", errors="0", skipped="0")
    case = ET.SubElement(
        suite,
        "testcase",
        classname="tests.sync.test_browsertype_connect",
        name=verify.CASE_ID.split("::", 1)[1],
    )
    if tamper == "skipped":
        suite.set("skipped", "1")
        ET.SubElement(case, "skipped")
    elif tamper == "extra-case":
        ET.SubElement(suite, "testcase", classname="tests.sync.other", name="other")
    elif tamper == "wrong-case":
        case.set("name", "test_other[chromium]")
    path = tmp_path / "result.xml"
    ET.ElementTree(suite).write(path, encoding="unicode")
    return path


def test_complete_browser_free_receipt_is_accepted(tmp_path: Path) -> None:
    # Arrange
    records = _receipt_records(tmp_path / "venv/lib/python3.12/site-packages")

    # Act
    counts = _verify_receipt(tmp_path, records)

    # Assert
    assert counts["command"] == 3
    assert counts["close"] == counts["case-call"] == 1


@pytest.mark.parametrize(
    ("tamper", "message"),
    [
        ("missing-close", "fixture receipt close count changed"),
        ("outcome", "test body call or selection changed"),
        ("fixture", "launch_server substitution absent"),
        ("browser-launch", "server did not complete the browser-free Page/Context fixture"),
        ("command", "protocol command sequence changed"),
        ("lane", "mixed receipt lane or address"),
        ("endpoint", "synthetic server lifetime changed"),
        ("runtime", "oracle did not import independent official Playwright"),
    ],
)
def test_tampered_receipt_is_rejected(tmp_path: Path, tamper: str, message: str) -> None:
    # Arrange
    records = _receipt_records(tmp_path / "venv/lib/python3.12/site-packages")
    if tamper == "missing-close":
        records = [record for record in records if record["kind"] != "close"]
    elif tamper == "outcome":
        next(record for record in records if record["kind"] == "case-call")["outcome"] = "skipped"
    elif tamper == "fixture":
        next(record for record in records if record["kind"] == "fixture-adapted")[
            "testSourceAdapted"
        ] = True
    elif tamper == "browser-launch":
        next(record for record in records if record["kind"] == "close")[
            "browserProcessLaunched"
        ] = True
    elif tamper == "command":
        next(record for record in records if record["kind"] == "command")["method"] = "noop"
    elif tamper == "lane":
        records[0]["mode"] = "candidate"
    elif tamper == "endpoint":
        next(record for record in records if record["kind"] == "fixture-created")[
            "endpointPath"
        ] = "/other"
    elif tamper == "runtime":
        next(record for record in records if record["kind"] == "runtime-import")[
            "publicApiPath"
        ] = "/wrong/playwright/sync_api.py"

    # Act / Assert
    with pytest.raises(ValueError, match=message):
        _verify_receipt(tmp_path, records)


def test_exact_one_passing_official_derived_junit_case_is_accepted(tmp_path: Path) -> None:
    # Arrange
    path = _junit_file(tmp_path)

    # Act
    status = verify._junit(path)

    # Assert
    assert status == "passed"


@pytest.mark.parametrize(
    ("tamper", "message"),
    [
        ("skipped", "JUnit candidate/oracle case did not pass without skips"),
        ("extra-case", "JUnit did not contain exactly one case"),
        ("wrong-case", "JUnit case identity changed"),
    ],
)
def test_tampered_junit_result_is_rejected(tmp_path: Path, tamper: str, message: str) -> None:
    # Arrange
    path = _junit_file(tmp_path, tamper=tamper)

    # Act / Assert
    with pytest.raises(ValueError, match=message):
        verify._junit(path)


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("requiresBrowser", True),
        ("fixtureAdapted", False),
        ("testSourceAdapted", True),
        ("promotableToOfficialLedger", True),
    ],
)
def test_tampered_manifest_classification_is_rejected(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, field: str, value: bool
) -> None:
    # Arrange: only the classification is under test; upstream repositories need not exist in CI.
    manifest = json.loads(verify.MANIFEST.read_text())
    manifest[field] = value
    tampered = tmp_path / "manifest.json"
    tampered.write_text(json.dumps(manifest))
    monkeypatch.setattr(verify, "MANIFEST", tampered)
    monkeypatch.setattr(
        verify,
        "_git",
        lambda checkout, *args: (
            verify.UPSTREAM_COMMIT
            if checkout == verify.UPSTREAM and args == ("rev-parse", "HEAD")
            else verify.CORE_COMMIT
            if checkout == verify.CORE and args == ("rev-parse", "HEAD")
            else ""
        ),
    )
    monkeypatch.setattr(
        verify,
        "sha256",
        lambda path: verify.SOURCE_HASHES[path.relative_to(verify.UPSTREAM).as_posix()],
    )

    # Act / Assert
    with pytest.raises(ValueError, match="manifest classification changed"):
        verify.verify_sources()
