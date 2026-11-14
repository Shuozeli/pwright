"""Negative controls for exact six-case, fixture-adapted state evidence."""

from __future__ import annotations

import json
import xml.etree.ElementTree as ET

import pytest
import verify_playwright_python_connect_lifecycle_synthetic_v2 as verify

HOST = "macbookpro-yuacx.tail8f3b66.ts.net"
BIND = "100.78.97.63"
ENGINE = "chromium"
EXPECTED = {case: "passed" for case in verify.EXPECTED_IDS if case.endswith("[chromium]")}


def junit_file(tmp_path, names: list[str] | None = None):
    suite = ET.Element("testsuite", tests="2", failures="0", errors="0")
    for case_id in names or list(EXPECTED):
        body, name = case_id.split("::", 1)
        api = body.split("/")[1]
        ET.SubElement(suite, "testcase", classname=f"tests.{api}.test_browsertype_connect", name=name)
    path = tmp_path / "selection.xml"
    ET.ElementTree(suite).write(path, encoding="unicode")
    return path


def receipt_records(tmp_path):
    shared = {"mode": "oracle", "fixtureHost": HOST, "fixtureBind": BIND}
    site_packages = tmp_path / "venv/lib/python3.12/site-packages"
    records = [
        {**shared, "kind": "runtime-import", "publicApiPath": str(site_packages / "playwright/async_api/__init__.py"), "candidatePackagePath": None},
        {**shared, "kind": "fixture-adapted", "fixture": "launch_server", "fixtureBindingIdentity": True, "fixtureAdapted": True, "testSourceAdapted": False, "browserProcessLaunched": False, "cdpEndpointPresent": False, "upstreamCommit": verify.v1.UPSTREAM_COMMIT, "upstreamFixtureSha256": verify.v1.SOURCE_HASHES["tests/conftest.py"]},
        {**shared, "kind": "selected", "caseIds": sorted(EXPECTED), "testSourceAdapted": False},
    ]
    for offset, case_id in enumerate(sorted(EXPECTED)):
        pid = 1001 + offset
        server = {**shared, "pid": pid, "engine": ENGINE, "synthetic": True, "browserProcessLaunched": False}
        records.extend([
            {**server, "kind": "listening", "endpointPath": "/browser-synthetic-python"},
            {**shared, "kind": "fixture-created", "serverPid": pid, "engine": ENGINE, "fixtureAdapted": True, "testSourceAdapted": False, "browserProcessLaunched": False, "endpointPath": "/browser-synthetic-python"},
            {**server, "kind": "upgrade", "browserHeader": ENGINE, "requestPath": "/browser-synthetic-python"},
            {**server, "kind": "root-initialize", "sdkLanguage": "python", "id": 1},
            {**server, "kind": "bootstrap-sent", "frames": 8, "version": "155.0"},
            {**shared, "kind": "case-call", "caseId": case_id, "outcome": "passed"},
            {**server, "kind": "close", "forcedByFixtureTeardown": False},
            {**server, "kind": "stop-requested"},
            {**server, "kind": "stop"},
            {**shared, "kind": "fixture-stopped", "serverPid": pid, "returnCode": 0},
        ])
    return records


def check_receipt(tmp_path, records):
    path = tmp_path / "receipt.jsonl"
    path.write_text("".join(json.dumps(record) + "\n" for record in records))
    return verify.receipt(path, mode="oracle", engine=ENGINE, expected=EXPECTED, bind=BIND, host=HOST, result_dir=tmp_path)


def test_exact_two_engine_case_ids_are_accepted(tmp_path) -> None:
    # Arrange
    path = junit_file(tmp_path)

    # Act
    result = verify.junit(path, ENGINE)

    # Assert
    assert result == {"cases": EXPECTED, "passed": 2}


def test_junit_case_selection_mutation_is_rejected(tmp_path) -> None:
    # Arrange
    cases = list(EXPECTED)
    cases[0] = cases[0].replace("[chromium]", "[firefox]")
    path = junit_file(tmp_path, cases)

    # Act / Assert
    with pytest.raises(ValueError, match="JUnit selection differs"):
        verify.junit(path, ENGINE)


def test_complete_synthetic_receipt_is_accepted(tmp_path) -> None:
    # Arrange
    records = receipt_records(tmp_path)

    # Act
    counts = check_receipt(tmp_path, records)

    # Assert
    assert counts["case-call"] == counts["bootstrap-sent"] == counts["close"] == 2


@pytest.mark.parametrize(
    ("tamper", "message"),
    [
        ("selected", "selected cases changed"),
        ("fixture-binding", "pinned launch_server adaptation absent"),
        ("browser-launch", "pinned launch_server adaptation absent"),
        ("missing-bootstrap", "bootstrap-sent process evidence incomplete"),
        ("forced-close", "synthetic protocol or clean close evidence changed"),
        ("wrong-engine", "upgrade claimed browser launch or wrong engine"),
        ("missing-call", "case-call receipt missing or retried"),
        ("orphan-call", "fixture process does not own exactly one case-call"),
        ("wrong-oracle", "oracle did not import the independent Playwright package"),
        ("bad-teardown", "fixture adaptation or teardown changed"),
    ],
)
def test_receipt_mutations_are_rejected(tmp_path, tamper: str, message: str) -> None:
    # Arrange
    records = receipt_records(tmp_path)
    select = lambda kind: next(item for item in records if item["kind"] == kind)
    if tamper == "selected":
        select("selected")["caseIds"] = []
    elif tamper == "fixture-binding":
        select("fixture-adapted")["fixtureBindingIdentity"] = False
    elif tamper == "browser-launch":
        select("fixture-adapted")["browserProcessLaunched"] = True
    elif tamper == "missing-bootstrap":
        records.remove(select("bootstrap-sent"))
    elif tamper == "forced-close":
        select("close")["forcedByFixtureTeardown"] = True
    elif tamper == "wrong-engine":
        select("upgrade")["engine"] = "firefox"
    elif tamper == "missing-call":
        records.remove(select("case-call"))
    elif tamper == "orphan-call":
        call = select("case-call")
        records.remove(call)
        records.append(call)
    elif tamper == "wrong-oracle":
        select("runtime-import")["candidatePackagePath"] = "/injected/pwright"
    else:
        select("fixture-stopped")["returnCode"] = 1

    # Act / Assert
    with pytest.raises(ValueError, match=message):
        check_receipt(tmp_path, records)
