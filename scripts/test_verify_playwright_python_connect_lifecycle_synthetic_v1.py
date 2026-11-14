"""Browser-free negative controls for the fixture-adapted Python lifecycle verifier."""

from __future__ import annotations

import json
import xml.etree.ElementTree as ET

import pytest
import verify_playwright_python_connect_lifecycle_synthetic_v1 as verify

HOST = "macbookpro-yuacx.tail8f3b66.ts.net"
BIND = "100.78.97.63"
ENGINE = "chromium"
EXPECTED = {case_id: "passed" for case_id in verify.EXPECTED_IDS if case_id.endswith("[chromium]")}


def _junit_file(tmp_path, *, case_ids: list[str] | None = None):
    suite = ET.Element("testsuite", tests="4", failures="0", errors="0")
    for case_id in case_ids or list(EXPECTED):
        body, name = case_id.split("::", maxsplit=1)
        api = body.split("/")[1]
        ET.SubElement(
            suite,
            "testcase",
            classname=f"tests.{api}.test_browsertype_connect",
            name=name,
        )
    path = tmp_path / "selection.xml"
    ET.ElementTree(suite).write(path, encoding="unicode")
    return path


def _receipt_records(result_dir) -> list[dict]:
    shared = {"mode": "oracle", "fixtureHost": HOST, "fixtureBind": BIND}
    site_packages = result_dir / "venv/lib/python3.12/site-packages"
    records = [
        {
            **shared,
            "kind": "runtime-import",
            "publicApiPath": str(site_packages / "playwright/async_api/__init__.py"),
            "candidatePackagePath": None,
        },
        {
            **shared,
            "kind": "fixture-adapted",
            "fixture": "launch_server",
            "fixtureBindingIdentity": True,
            "fixtureAdapted": True,
            "testSourceAdapted": False,
            "browserProcessLaunched": False,
            "cdpEndpointPresent": False,
            "upstreamCommit": verify.UPSTREAM_COMMIT,
            "upstreamFixtureSha256": verify.SOURCE_HASHES["tests/conftest.py"],
        },
        {**shared, "kind": "selected", "caseIds": list(EXPECTED)},
    ]
    for offset, case_id in enumerate(EXPECTED):
        pid = 1001 + offset
        server = {
            **shared,
            "pid": pid,
            "engine": ENGINE,
            "synthetic": True,
            "browserProcessLaunched": False,
        }
        records.extend(
            [
                {
                    **server,
                    "kind": "listening",
                    "endpointPath": "/browser-synthetic-python",
                },
                {
                    **shared,
                    "kind": "fixture-created",
                    "serverPid": pid,
                    "engine": ENGINE,
                    "fixtureAdapted": True,
                    "testSourceAdapted": False,
                    "browserProcessLaunched": False,
                },
                {**server, "kind": "upgrade", "browserHeader": ENGINE},
                {**server, "kind": "root-initialize", "sdkLanguage": "python", "id": 1},
                {**server, "kind": "bootstrap-sent", "frames": 8, "version": "155.0"},
                {**server, "kind": "close", "forcedByFixtureTeardown": False},
                {**shared, "kind": "case-call", "caseId": case_id, "outcome": "passed"},
                {**server, "kind": "stop-requested"},
                {**server, "kind": "stop"},
                {
                    **shared,
                    "kind": "fixture-stopped",
                    "serverPid": pid,
                    "returnCode": 0,
                },
            ]
        )
    return records


def _write_receipt(tmp_path, records: list[dict]):
    path = tmp_path / "receipt.jsonl"
    path.write_text("".join(json.dumps(record) + "\n" for record in records))
    return path


def _verify_receipt(path, tmp_path):
    return verify._receipt(
        path,
        mode="oracle",
        engine=ENGINE,
        expected=EXPECTED,
        bind=BIND,
        host=HOST,
        result_dir=tmp_path,
    )


def test_exact_four_engine_expanded_case_ids_are_accepted(tmp_path) -> None:
    # Arrange
    path = _junit_file(tmp_path)

    # Act
    result = verify._junit(path, ENGINE)

    # Assert
    assert result == {"cases": EXPECTED, "passed": 4}


def test_tampered_official_case_id_is_rejected(tmp_path) -> None:
    # Arrange
    cases = list(EXPECTED)
    cases[0] = cases[0].replace("[chromium]", "[invented]")
    path = _junit_file(tmp_path, case_ids=cases)

    # Act / Assert
    with pytest.raises(ValueError, match="JUnit selection differs"):
        verify._junit(path, ENGINE)


def test_complete_browser_free_receipt_is_accepted(tmp_path) -> None:
    # Arrange
    path = _write_receipt(tmp_path, _receipt_records(tmp_path))

    # Act
    counts = _verify_receipt(path, tmp_path)

    # Assert
    assert counts["bootstrap-sent"] == counts["close"] == 4


@pytest.mark.parametrize("tamper", ["missing", "forced"])
def test_missing_or_forced_close_receipt_is_rejected(tmp_path, tamper: str) -> None:
    # Arrange
    records = _receipt_records(tmp_path)
    closes = [record for record in records if record["kind"] == "close"]
    if tamper == "missing":
        records.remove(closes[-1])
    else:
        closes[-1]["forcedByFixtureTeardown"] = True
    path = _write_receipt(tmp_path, records)

    # Act / Assert
    with pytest.raises(ValueError, match="distinct clean WS close receipts"):
        _verify_receipt(path, tmp_path)


@pytest.mark.parametrize("tamper", ["fixture-identity", "browser-launch", "cdp"])
def test_unsafe_fixture_adaptation_is_rejected(tmp_path, tamper: str) -> None:
    # Arrange
    records = _receipt_records(tmp_path)
    adapted = next(record for record in records if record["kind"] == "fixture-adapted")
    if tamper == "fixture-identity":
        adapted["fixtureBindingIdentity"] = False
    elif tamper == "browser-launch":
        adapted["browserProcessLaunched"] = True
    else:
        adapted["cdpEndpointPresent"] = True
    path = _write_receipt(tmp_path, records)

    # Act / Assert
    with pytest.raises(ValueError, match="launch_server substitution absent"):
        _verify_receipt(path, tmp_path)


def test_missing_root_initialize_is_rejected(tmp_path) -> None:
    # Arrange
    records = _receipt_records(tmp_path)
    records.remove(next(record for record in records if record["kind"] == "root-initialize"))
    path = _write_receipt(tmp_path, records)

    # Act / Assert
    with pytest.raises(ValueError, match="matching Root.initialize bootstrap"):
        _verify_receipt(path, tmp_path)
