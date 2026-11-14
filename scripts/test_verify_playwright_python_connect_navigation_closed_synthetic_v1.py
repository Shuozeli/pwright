"""Source-only negative controls for the navigation-after-close verifier."""

from __future__ import annotations

import json
import xml.etree.ElementTree as ET
from pathlib import Path

import pytest
import verify_playwright_python_connect_navigation_closed_synthetic_v1 as verify

HOST = "macbookpro-yuacx.tail8f3b66.ts.net"
BIND = "100.78.97.63"
PID = 1001
ENGINE = "chromium"


def _junit(tmp_path: Path, mode: str, tamper: str | None = None) -> Path:
    failure_count = "0" if mode == "oracle" else "1"
    suite = ET.Element(
        "testsuite", tests="1", failures=failure_count, errors="0", skipped="0"
    )
    case = ET.SubElement(
        suite,
        "testcase",
        classname="tests.async.test_browsertype_connect",
        name=verify.CASE_BODY.split("::", 1)[1] + f"[{ENGINE}]",
    )
    if mode == "candidate":
        failure = ET.SubElement(
            case,
            "failure",
            message="_native.UnsupportedError: Page.goto remote operation is not installed",
        )
        failure.text = (
            "tests/async/test_browsertype_connect.py:179: in "
            "test_browser_type_connect_should_reject_navigation_when_browser_closes\n"
            "    await page.goto(server.PREFIX + '/one-style.html')\n"
            "E   _native.UnsupportedError: Page.goto remote operation is not installed"
        )
    if tamper == "skipped":
        suite.set("skipped", "1")
        ET.SubElement(case, "skipped")
    elif tamper == "extra-case":
        ET.SubElement(suite, "testcase", classname="tests.async.other", name="other")
    elif tamper == "wrong-case":
        case.set("name", "test_other[chromium]")
    elif tamper == "unexpected-green":
        suite.set("failures", "0")
        for child in list(case):
            case.remove(child)
    elif tamper == "wrong-failure":
        failure = case.find("failure")
        assert failure is not None
        failure.set("message", "RuntimeError: synthetic fixture failed")
        failure.text = "tests/async/test_browsertype_connect.py:179: setup failed"
    elif tamper == "setup-error":
        suite.set("errors", "1")
        ET.SubElement(case, "error")
    path = tmp_path / "result.xml"
    ET.ElementTree(suite).write(path, encoding="unicode")
    return path


@pytest.mark.parametrize("mode", ("oracle", "candidate"))
def test_expected_junit_outcome_is_accepted(tmp_path: Path, mode: str) -> None:
    # Arrange
    path = _junit(tmp_path, mode)

    # Act
    outcome = verify._junit(path, mode, ENGINE)

    # Assert
    assert outcome == ("passed" if mode == "oracle" else "failed")


@pytest.mark.parametrize(
    ("tamper", "message"),
    (
        ("skipped", "skip"),
        ("extra-case", "exactly one case"),
        ("wrong-case", "case identity"),
        ("unexpected-green", "real test-body failure"),
        ("wrong-failure", "Page.goto unsupported-operation"),
        ("setup-error", "setup error"),
    ),
)
def test_candidate_junit_tampering_is_rejected(
    tmp_path: Path, tamper: str, message: str
) -> None:
    # Arrange
    path = _junit(tmp_path, "candidate", tamper)

    # Act
    with pytest.raises(ValueError, match=message):
        verify._junit(path, "candidate", ENGINE)


def _receipt_records(site: Path, mode: str) -> list[dict[str, object]]:
    shared = {
        "mode": mode,
        "engine": ENGINE,
        "fixtureBind": BIND,
        "fixtureHost": HOST,
    }
    server = {**shared, "pid": PID, "synthetic": True, "browserProcessLaunched": False}
    expected_case = f"{verify.CASE_BODY}[{ENGINE}]"
    return [
        {
            **shared,
            "kind": "runtime-import",
            "publicApiPath": str(
                verify.SHIM / "playwright/async_api.py"
                if mode == "candidate"
                else site / "playwright/async_api/__init__.py"
            ),
            "candidatePackagePath": (
                str(site / "pwright/__init__.py") if mode == "candidate" else None
            ),
        },
        {
            **shared,
            "kind": "fixture-adapted",
            "fixture": "launch_server",
            "testSourceAdapted": False,
            "browserProcessLaunched": False,
            "upstreamCommit": verify.UPSTREAM_COMMIT,
        },
        {**shared, "kind": "selected", "caseId": expected_case},
        {
            **shared,
            "kind": "fixture-created",
            "serverPid": PID,
            "endpointPath": verify.ENDPOINT_PATH,
            "fixtureAdapted": True,
            "testSourceAdapted": False,
            "browserProcessLaunched": False,
        },
        {**server, "kind": "listening", "endpointPath": verify.ENDPOINT_PATH},
        {**server, "kind": "upgrade"},
        {**server, "kind": "command", "guid": "", "method": "initialize", "phase": 0},
        {**server, "kind": "bootstrap-sent"},
        {
            **server,
            "kind": "command",
            "guid": "browser@1",
            "method": "newContext",
            "phase": 1,
        },
        {
            **server,
            "kind": "command",
            "guid": "context@1",
            "method": "newPage",
            "phase": 2,
        },
        {**server, "kind": "close", "phase": 3},
        {
            **shared,
            "kind": "case-call",
            "caseId": expected_case,
            "outcome": "passed" if mode == "oracle" else "failed",
        },
        {**server, "kind": "stop"},
        {**shared, "kind": "fixture-stopped", "serverPid": PID, "returnCode": 0},
    ]


def _receipt(tmp_path: Path, records: list[dict[str, object]], mode: str) -> None:
    path = tmp_path / "receipt.jsonl"
    path.write_text("".join(json.dumps(record) + "\n" for record in records))
    verify._receipt(path, mode, ENGINE, BIND, HOST, tmp_path / "site")


@pytest.mark.parametrize("mode", ("oracle", "candidate"))
def test_complete_browser_free_receipt_is_accepted(tmp_path: Path, mode: str) -> None:
    # Arrange
    records = _receipt_records(tmp_path / "site", mode)

    # Act
    _receipt(tmp_path, records, mode)

    # Assert
    assert len(records) == 14


@pytest.mark.parametrize(
    ("tamper", "message"),
    (
        ("lane", "lane, engine, or Tailscale"),
        ("engine", "lane, engine, or Tailscale"),
        ("address", "lane, engine, or Tailscale"),
        ("runtime", "runtime import was not independent"),
        ("fixture", "fixture substitution absent"),
        ("browser-launch", "synthetic launch fixture identity changed"),
        ("missing-close", "receipt close count changed"),
        ("wrong-phase", "did not reach Page"),
        ("outcome", "test-body call outcome changed"),
        ("extra-command", "protocol command sequence changed"),
        ("missing-stop", "receipt stop count changed"),
        ("protocol-error", "synthetic protocol fixture failed"),
    ),
)
def test_tampered_receipt_is_rejected(
    tmp_path: Path, tamper: str, message: str
) -> None:
    # Arrange
    records = _receipt_records(tmp_path / "site", "candidate")

    def one(kind: str) -> dict[str, object]:
        return next(record for record in records if record["kind"] == kind)

    if tamper == "lane":
        records[0]["mode"] = "oracle"
    elif tamper == "engine":
        records[0]["engine"] = "firefox"
    elif tamper == "address":
        records[0]["fixtureBind"] = "127.0.0.1"
    elif tamper == "runtime":
        one("runtime-import")["publicApiPath"] = str(tmp_path / "fake/async_api.py")
    elif tamper == "fixture":
        one("fixture-adapted")["testSourceAdapted"] = True
    elif tamper == "browser-launch":
        one("fixture-created")["browserProcessLaunched"] = True
    elif tamper == "missing-close":
        records.remove(one("close"))
    elif tamper == "wrong-phase":
        one("close")["phase"] = 2
    elif tamper == "outcome":
        one("case-call")["outcome"] = "skipped"
    elif tamper == "extra-command":
        records.append({**records[6], "method": "goto", "guid": "page@1"})
    elif tamper == "missing-stop":
        records.remove(one("stop"))
    elif tamper == "protocol-error":
        records.append({**records[6], "kind": "protocol-error"})

    # Act
    with pytest.raises(ValueError, match=message):
        _receipt(tmp_path, records, "candidate")
