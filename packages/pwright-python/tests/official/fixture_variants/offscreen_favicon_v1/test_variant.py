"""Offline checks for the opt-in favicon fixture and companion receipt."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path
from types import SimpleNamespace
from typing import Callable

import pytest

ROOT = Path(__file__).resolve().parents[6]
SCRIPT = ROOT / "scripts/playwright_python_official_favicon_v1.py"
PLUGIN = Path(__file__).with_name("pwright_official_favicon_v1.py")


def _load(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _junit(path: Path, properties: dict[str, str]) -> None:
    suite = ET.Element("testsuite")
    case = ET.SubElement(
        suite,
        "testcase",
        {"classname": "tests.async.test_click", "name": "test_click_offscreen_buttons[chromium]"},
    )
    container = ET.SubElement(case, "properties")
    for name, value in properties.items():
        ET.SubElement(container, "property", {"name": name, "value": value})
    ET.ElementTree(suite).write(path, encoding="utf-8", xml_declaration=True)


def test_fixture_serves_only_real_icon_and_restores_route() -> None:
    # Arrange
    plugin = _load(PLUGIN, "pwright_official_favicon_v1_unit")
    node = SimpleNamespace(nodeid=plugin.CASE_ID, user_properties=[])
    request = SimpleNamespace(node=node)

    class Server:
        def __init__(self) -> None:
            self.routes: dict[str, Callable[[HttpRequest], None]] = {}

        def set_route(self, path: str, callback: Callable[[HttpRequest], None]) -> None:
            self.routes[path] = callback

    class HttpRequest:
        method = b"GET"

        def __init__(self) -> None:
            self.code: int | None = None
            self.headers: dict[bytes, bytes] = {}
            self.body: bytes = b""
            self.finished = False

        def setResponseCode(self, code: int) -> None:
            self.code = code

        def setHeader(self, name: bytes, value: bytes) -> None:
            self.headers[name] = value

        def write(self, body: bytes) -> None:
            self.body += body

        def finish(self) -> None:
            self.finished = True

    server = Server()
    http_request = HttpRequest()
    fixture = plugin.serve_offscreen_favicon_v1.__wrapped__(request, server)
    next(fixture)

    # Act
    server.routes[plugin.ICON_ROUTE](http_request)
    with pytest.raises(StopIteration):
        next(fixture)

    # Assert
    icon = plugin._icon_bytes()
    assert icon[:6] == b"\x00\x00\x01\x00\x01\x00"
    assert http_request.code == 200
    assert http_request.headers[b"content-type"] == b"image/x-icon"
    assert http_request.headers[b"content-length"] == b"70"
    assert http_request.body == icon
    assert http_request.finished
    assert server.routes == {}
    assert (plugin.PROPERTY_SERVED, plugin.ICON_ROUTE) in node.user_properties


def test_companion_rejects_missing_served_marker(tmp_path: Path) -> None:
    # Arrange
    capture = _load(SCRIPT, "playwright_python_official_favicon_v1_unit")
    junit = tmp_path / "case.xml"
    icon_sha = hashlib.sha256(capture._icon_bytes()).hexdigest()
    _junit(
        junit,
        {
            capture.PROPERTY_VARIANT: capture.VARIANT_ID,
            capture.PROPERTY_ICON_SHA256: icon_sha,
        },
    )

    # Act / Assert
    with pytest.raises(capture.FixtureVariantError, match="not observed"):
        capture._require_junit_variant(junit, icon_sha)


def test_runner_uses_exact_one_case_manifest_execution(tmp_path: Path) -> None:
    # Arrange
    capture = _load(SCRIPT, "pwright_official_favicon_v1_command_unit")

    # Act
    command = capture._runner_command(
        python=Path(sys.executable),
        mode="candidate",
        upstream=tmp_path,
        fixture_bind="100.78.97.63",
        fixture_host="macbookpro-yuacx.tail8f3b66.ts.net",
        result=tmp_path / "case.xml",
    )

    # Assert
    assert json.loads(capture.MANIFEST.read_text())["cases"] == [
        {
            "id": capture.CASE_SELECTION,
            "capabilities": ["page.click", "async-api"],
        }
    ]
    assert "--manifest-cases" in command
    assert command[command.index("--manifest-api") + 1] == "async"
    assert "--test" not in command


def test_plugin_marks_actual_pytest_junit_when_icon_is_served(tmp_path: Path) -> None:
    # Arrange
    capture = _load(SCRIPT, "playwright_python_official_favicon_v1_pytest_unit")
    test_file = tmp_path / "tests/async/test_click.py"
    test_file.parent.mkdir(parents=True)
    test_file.write_text(
        """import pytest

class Server:
    def __init__(self):
        self.routes = {}
    def set_route(self, path, callback):
        self.routes[path] = callback

class Request:
    method = b'GET'
    def __init__(self):
        self.code = None
        self.headers = {}
        self.body = b''
    def setResponseCode(self, code):
        self.code = code
    def setHeader(self, name, value):
        self.headers[name] = value
    def write(self, body):
        self.body += body
    def finish(self):
        pass

@pytest.fixture
def server():
    return Server()

@pytest.mark.parametrize('browser', ['chromium'])
def test_click_offscreen_buttons(browser, server):
    response = Request()
    server.routes['/favicon.ico'](response)
    assert response.code == 200
    assert response.headers[b'content-type'] == b'image/x-icon'
    assert len(response.body) == 70
"""
    )
    config = tmp_path / "pytest.ini"
    config.write_text("[pytest]\n")
    junit = tmp_path / "result.xml"
    env = os.environ.copy()
    env["PYTHONPATH"] = str(capture.VARIANT_DIR)

    # Act
    completed = subprocess.run(
        [
            sys.executable,
            "-m",
            "pytest",
            "-q",
            "-p",
            "pwright_official_favicon_v1",
            "-p",
            "no:cacheprovider",
            "-c",
            str(config),
            f"--junitxml={junit}",
            str(test_file) + "::test_click_offscreen_buttons[chromium]",
        ],
        cwd=tmp_path,
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )

    # Assert
    assert completed.returncode == 0, completed.stdout + completed.stderr
    capture._require_junit_variant(junit, hashlib.sha256(capture._icon_bytes()).hexdigest())


def test_companion_checks_exact_bound_metadata(tmp_path: Path) -> None:
    # Arrange: synthetic metadata exercises the extension check, not base schema-5.
    capture = _load(SCRIPT, "playwright_python_official_favicon_v1_metadata_unit")
    result = tmp_path / "case.xml"
    base = result.with_suffix(".provenance.json")
    icon_sha = hashlib.sha256(capture._icon_bytes()).hexdigest()
    _junit(
        result,
        {
            capture.PROPERTY_VARIANT: capture.VARIANT_ID,
            capture.PROPERTY_ICON_SHA256: icon_sha,
            capture.PROPERTY_SERVED: capture.ICON_ROUTE,
        },
    )
    fixture_bind = "100.78.97.63"
    fixture_host = "macbookpro-yuacx.tail8f3b66.ts.net"
    base.write_text(
        json.dumps(
            {
                "schema": 5,
                "mode": "oracle",
                "selections": [capture.CASE_SELECTION],
                "manifestCases": True,
                "manifestApi": "async",
                "browsers": ["chromium"],
                "browserChannel": "chrome",
                "manifest": str(capture.MANIFEST),
                "result": str(result),
                "exitCode": 0,
                "fixtureBind": fixture_bind,
                "fixtureHost": fixture_host,
                "pythonExecutable": sys.executable,
            }
        )
    )
    receipt = {
        "schema": 1,
        "variant": capture.VARIANT_ID,
        "mode": "oracle",
        "caseId": capture.CASE_ID,
        "pythonExecutable": sys.executable,
        "upstream": str(tmp_path),
        "fixtureBind": fixture_bind,
        "fixtureHost": fixture_host,
        "result": str(result),
        "resultSha256": capture._file_sha256(result),
        "baseProvenance": str(base),
        "baseProvenanceSha256": capture._file_sha256(base),
        "manifestSha256": capture._source_sha256(capture.MANIFEST),
        "pluginSha256": capture._source_sha256(capture.PLUGIN),
        "iconAssetSha256": capture._source_sha256(capture.ICON),
        "iconBytesSha256": icon_sha,
        "captureToolSha256": capture._source_sha256(SCRIPT),
        "runnerSha256": capture._source_sha256(capture.RUNNER),
        "baseVerifierSha256": capture._source_sha256(capture.BASE_VERIFIER),
        "runnerCommand": capture._runner_command(
            python=Path(sys.executable),
            mode="oracle",
            upstream=tmp_path,
            fixture_bind=fixture_bind,
            fixture_host=fixture_host,
            result=result,
        ),
        "runnerExitCode": 0,
        "pluginPythonPath": str(capture.VARIANT_DIR),
        "pluginPytestAddopts": capture.PLUGIN_ADDOPTS,
    }

    # Act
    verified = capture._verify_payload(
        receipt, result.with_suffix(".fixture-variant.json"), verify_base=False
    )

    # Assert
    assert verified == receipt


@pytest.mark.parametrize("drift_key", ["manifestSha256", "pluginSha256", "iconAssetSha256"])
def test_companion_rejects_variant_source_hash_drift(tmp_path: Path, drift_key: str) -> None:
    # Arrange
    capture = _load(SCRIPT, "playwright_python_official_favicon_v1_unit")
    result = tmp_path / "case.xml"
    base = result.with_suffix(".provenance.json")
    _junit(result, {})
    base.write_text("{}")
    receipt: dict[str, object] = {key: None for key in capture.RECEIPT_KEYS}
    receipt.update(
        schema=1,
        variant=capture.VARIANT_ID,
        mode="oracle",
        caseId=capture.CASE_ID,
        result=str(result),
        resultSha256="0" * 64,
        baseProvenance=str(base),
        baseProvenanceSha256="0" * 64,
        pythonExecutable=sys.executable,
        upstream=str(tmp_path),
        fixtureBind="100.78.97.63",
        fixtureHost="macbookpro-yuacx.tail8f3b66.ts.net",
        manifestSha256=capture._source_sha256(capture.MANIFEST),
        pluginSha256=capture._source_sha256(capture.PLUGIN),
        iconAssetSha256=capture._source_sha256(capture.ICON),
        iconBytesSha256=hashlib.sha256(capture._icon_bytes()).hexdigest(),
        captureToolSha256=capture._source_sha256(SCRIPT),
        runnerSha256=capture._source_sha256(capture.RUNNER),
        baseVerifierSha256=capture._source_sha256(capture.BASE_VERIFIER),
        runnerCommand=[],
        runnerExitCode=0,
        pluginPythonPath=str(capture.VARIANT_DIR),
        pluginPytestAddopts=capture.PLUGIN_ADDOPTS,
    )
    receipt[drift_key] = "0" * 64

    # Act / Assert
    with pytest.raises(capture.FixtureVariantError, match="source hash drifted"):
        capture._verify_payload(
            receipt, result.with_suffix(".fixture-variant.json"), verify_base=False
        )
