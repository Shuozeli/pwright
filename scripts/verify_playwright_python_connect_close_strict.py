#!/usr/bin/env python3
"""Independently verify three official BrowserType.connect close-error captures."""

from __future__ import annotations

import argparse
import hashlib
import ipaddress
import json
import re
import subprocess
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path
from typing import Any, NoReturn

from verify_playwright_python_ws_transport_strict import (
    COMMIT,
    OFFICIAL,
    ROOT,
    StrictWebSocketEvidenceError,
    _file,
    _object,
    _regular,
    _runtime,
    _sources,
    _string,
    _upstream,
    sandbox_profile,
    sha256_file,
)

POLICY = OFFICIAL / "async-browser-connect-close-strict-v1-manifest.json"
POLICY_SHA256 = "67f3e64861930baa24d5d93b415546c79ead206f71f314ba475b12d49e2eca36"
KIND = "playwright-python-browser-connect-close-strict-v1"
SOURCE = "tests/async/test_browsertype_connect.py"
NAME = "test_should_print_custom_ws_close_error"
ENGINES = ("chromium", "firefox", "webkit")
TEST_IDS = tuple(f"{SOURCE}::{NAME}[{engine}]" for engine in ENGINES)
GUARD = OFFICIAL / "ws_connect_guard_strict"
ADAPTATION = "exact-localhost-ws-to-tailscale-magicdns-only"
WHEEL_SHA256 = "20033ac820480165caf4c420961e22f0fd8c4c29732d2fbcde582c05adabe46e"
_SHA = re.compile(r"[0-9a-f]{64}\Z")
_NONCE = re.compile(r"[0-9a-f]{32}\Z")
_HOST = re.compile(r"[a-z0-9-]+(?:\.[a-z0-9-]+)*\.ts\.net\Z")
_KEYS = frozenset(
    {
        "schema",
        "kind",
        "mode",
        "testId",
        "manifest",
        "manifestSha256",
        "upstream",
        "commit",
        "toolingRoot",
        "toolingSources",
        "pythonExecutable",
        "pythonRuntimeSha256",
        "pythonVersion",
        "wheelPath",
        "wheelSha256",
        "playwrightPackage",
        "playwrightPackageSha256",
        "driverNode",
        "driverNodeSha256",
        "pwrightPackage",
        "pwrightPackageSha256",
        "nativePath",
        "nativeSha256",
        "runtimeLibraries",
        "publicConnectPresent",
        "fixtureBind",
        "fixtureHost",
        "fixturePort",
        "fixtureTransportAdaptation",
        "environment",
        "sandboxProfile",
        "sandboxProfileSha256",
        "sandboxAllowedExecutables",
        "sandboxProbe",
        "command",
        "runNonce",
        "exitCode",
        "outcome",
        "result",
        "resultSha256",
        "callReceipt",
        "callReceiptSha256",
        "verifierSha256",
    }
)


class StrictBrowserConnectEvidenceError(StrictWebSocketEvidenceError):
    """The capture cannot receive official conformance credit."""


def _fail(message: str) -> NoReturn:
    raise StrictBrowserConnectEvidenceError(message)


def _digest(value: object, label: str) -> None:
    if not isinstance(value, str) or not _SHA.fullmatch(value):
        _fail(f"{label} must be a lowercase SHA-256")


def _policy(raw: object) -> dict[str, Any]:
    policy = _object(raw, "BrowserType.connect policy")
    if set(policy) != {
        "schema",
        "kind",
        "repository",
        "tag",
        "commit",
        "requiresBrowser",
        "requiresFixtureServers",
        "fixtureTransportAdaptation",
        "tests",
        "upstreamSources",
        "supportSources",
        "pythonRuntimeSha256",
        "oraclePackageTreeSha256",
        "oracleDriverNodeSha256",
        "wheelSha256",
        "captureReady",
        "captureSourceSha256",
    }:
        _fail("BrowserType.connect policy fields differ")
    if (
        policy["schema"] != 1
        or policy["kind"] != KIND
        or policy["repository"] != "https://github.com/microsoft/playwright-python.git"
        or policy["tag"] != "v1.58.0"
        or policy["commit"] != COMMIT
        or policy["requiresBrowser"] is not False
        or policy["requiresFixtureServers"] is not True
        or policy["fixtureTransportAdaptation"] != ADAPTATION
        or policy["tests"] != list(TEST_IDS)
        or policy["wheelSha256"] != WHEEL_SHA256
        or policy["captureReady"] is not True
    ):
        _fail("BrowserType.connect policy is not the frozen three-engine policy")
    upstream = _object(policy["upstreamSources"], "upstreamSources")
    if set(upstream) != {
        SOURCE,
        "tests/conftest.py",
        "tests/async/conftest.py",
        "tests/server.py",
        "tests/async/utils.py",
        "pyproject.toml",
    }:
        _fail("BrowserType.connect upstream source set differs")
    support = _object(policy["supportSources"], "supportSources")
    if set(support) != {
        "packages/pwright-python/tests/official/shim/playwright/__init__.py",
        "packages/pwright-python/tests/official/shim/playwright/async_api.py",
        "packages/pwright-python/tests/official/shim/playwright/_public_types.py",
        "packages/pwright-python/tests/official/shim/pwright_candidate_runtime.py",
        "packages/pwright-python/tests/official/runtime/pwright_official_server_runtime.py",
        "packages/pwright-python/tests/official/runtime/pwright_official_oracle_runtime.py",
        "scripts/run_playwright_python_ws_transport_strict.py",
        "scripts/verify_playwright_python_ws_transport_strict.py",
    }:
        _fail("BrowserType.connect support source set differs")
    capture = _object(policy["captureSourceSha256"], "captureSourceSha256")
    if set(capture) != {
        "scripts/run_playwright_python_connect_close_strict.py",
        "packages/pwright-python/tests/official/ws_connect_guard_strict/pwright_official_connect_guard.py",
    }:
        _fail("BrowserType.connect capture source set differs")
    for name, digest in {**upstream, **support, **capture}.items():
        _digest(digest, name)
    for name in (
        "pythonRuntimeSha256",
        "oraclePackageTreeSha256",
        "oracleDriverNodeSha256",
        "wheelSha256",
    ):
        _digest(policy[name], name)
    return policy


def _fixture(payload: dict[str, Any]) -> None:
    port = payload["fixturePort"]
    if not isinstance(port, int) or isinstance(port, bool) or not 1024 <= port < 65535:
        _fail("fixture port is invalid")
    try:
        bind = ipaddress.ip_address(_string(payload["fixtureBind"], "fixtureBind"))
    except ValueError as error:
        raise StrictBrowserConnectEvidenceError("fixture bind is not an IP") from error
    if (
        bind not in ipaddress.ip_network("100.64.0.0/10")
        or not _HOST.fullmatch(_string(payload["fixtureHost"], "fixtureHost"))
        or payload["fixtureTransportAdaptation"] != ADAPTATION
    ):
        _fail("fixture is not the Tailscale-bound same-port WebSocket adaptation")


def _method_probe(payload: dict[str, Any]) -> bool:
    module = (
        "pwright.async_api"
        if payload["mode"] == "candidate"
        else "playwright.async_api"
    )
    completed = subprocess.run(
        [
            payload["pythonExecutable"],
            "-c",
            f"from {module} import BrowserType; print(int(callable(getattr(BrowserType, 'connect', None))))",
        ],
        check=True,
        capture_output=True,
        text=True,
        timeout=10,
        env={"PATH": "/usr/bin:/bin", "PYTHONNOUSERSITE": "1"},
    )
    value = completed.stdout.strip()
    if value not in ("0", "1"):
        _fail("public connect method probe returned unexpected output")
    return value == "1"


def _environment(payload: dict[str, Any]) -> None:
    mode = payload["mode"]
    paths = (
        [OFFICIAL / "shim", Path(payload["upstream"]), OFFICIAL / "runtime", GUARD]
        if mode == "candidate"
        else [OFFICIAL / "runtime", GUARD]
    )
    expected = {
        "PATH": "/opt/homebrew/bin:/usr/bin:/bin",
        "PYTHONPATH": ":".join(map(str, paths)),
        "PYTHONNOUSERSITE": "1",
        "PYTEST_DISABLE_PLUGIN_AUTOLOAD": "1",
        "PWRIGHT_OFFICIAL_MODE": mode,
        "PWRIGHT_OFFICIAL_LAUNCH_MODE": "owned",
        "PWRIGHT_OFFICIAL_BROWSER_CHANNEL": "",
        "PWRIGHT_OFFICIAL_UPSTREAM_TESTS": str(Path(payload["upstream"]) / "tests"),
        "PWRIGHT_OFFICIAL_TAILSCALE_IP": payload["fixtureBind"],
        "PWRIGHT_OFFICIAL_TAILSCALE_HOST": payload["fixtureHost"],
        "PWRIGHT_PYTHON_CONNECT_EXPECTED_TEST": payload["testId"],
        "PWRIGHT_PYTHON_CONNECT_UPSTREAM": payload["upstream"],
        "PWRIGHT_PYTHON_CONNECT_PORT": str(payload["fixturePort"]),
        "PWRIGHT_PYTHON_CONNECT_RECEIPT": payload["callReceipt"],
        "PWRIGHT_PYTHON_CONNECT_NONCE": payload["runNonce"],
    }
    if payload["environment"] != expected:
        _fail("BrowserType.connect environment contains an override")


def _command(payload: dict[str, Any]) -> None:
    engine = payload["testId"].rsplit("[", 1)[1].removesuffix("]")
    expected = [
        "/usr/bin/sandbox-exec",
        "-p",
        payload["sandboxProfile"],
        payload["pythonExecutable"],
        "-m",
        "pytest",
        "-q",
        "--import-mode=importlib",
        "-c",
        str(Path(payload["upstream"]) / "pyproject.toml"),
        "-p",
        "pytest_asyncio.plugin",
        "-p",
        "pytest_timeout",
        "-p",
        "pwright_official_server_runtime",
        "-p",
        "pwright_candidate_runtime"
        if payload["mode"] == "candidate"
        else "pwright_official_oracle_runtime",
        "-p",
        "pwright_official_connect_guard",
        f"--browser={engine}",
        "--timeout=20",
        "--timeout-method=signal",
        f"{Path(payload['upstream']) / SOURCE}::{NAME}",
        f"--junitxml={payload['result']}",
    ]
    if payload["command"] != expected:
        _fail("BrowserType.connect command differs from pinned official selection")


def report_outcome(payload: dict[str, Any]) -> str:
    result = Path(_string(payload["result"], "result"))
    if not result.is_absolute() or result.with_suffix(".call.jsonl") != Path(
        payload["callReceipt"]
    ):
        _fail("BrowserType.connect output paths differ")
    _file(result, payload["resultSha256"], "official BrowserType.connect JUnit")
    try:
        root = ET.fromstring(result.read_bytes())
    except ET.ParseError as error:
        raise StrictBrowserConnectEvidenceError(
            "official JUnit is malformed"
        ) from error
    suites = root.findall("testsuite") if root.tag == "testsuites" else []
    if len(suites) != 1 or len(list(root.iter("testcase"))) != 1:
        _fail("official JUnit must contain exactly one suite and case")
    suite = suites[0]
    cases = suite.findall("testcase")
    if (
        len(cases) != 1
        or cases[0].get("classname") != "tests.async.test_browsertype_connect"
        or cases[0].get("name") != payload["testId"].split("::", 1)[1]
        or suite.get("tests") != "1"
        or suite.get("skipped") != "0"
        or cases[0].find("skipped") is not None
    ):
        _fail("official JUnit identity, count, or skip status differs")
    case = cases[0]
    failure = case.find("failure")
    error = case.find("error")
    if payload["mode"] == "candidate":
        failure_text = (
            ""
            if failure is None
            else (failure.get("message") or "") + (failure.text or "")
        )
        if (
            failure is None
            or error is not None
            or suite.get("failures") != "1"
            or suite.get("errors") != "0"
            or payload["exitCode"] != 1
            or "AttributeError" not in failure_text
            or "'BrowserType' object has no attribute 'connect'" not in failure_text
        ):
            _fail("candidate did not show the exact missing public connect failure")
        return "failed"
    if (
        failure is None
        and error is None
        and suite.get("failures") == "0"
        and suite.get("errors") == "0"
        and payload["exitCode"] == 0
    ):
        return "passed"
    _fail("oracle JUnit did not pass the unchanged upstream assertion")


def verify_evidence(policy: dict[str, Any], payload: dict[str, Any]) -> str:
    if set(payload) != _KEYS or payload["schema"] != 1 or payload["kind"] != KIND:
        _fail("BrowserType.connect evidence schema differs")
    if (
        payload["mode"] not in ("candidate", "oracle")
        or payload["testId"] not in TEST_IDS
    ):
        _fail("BrowserType.connect mode or test ID differs")
    if Path(payload["manifest"]) != POLICY or payload["manifestSha256"] != sha256_file(
        POLICY
    ):
        _fail("BrowserType.connect manifest identity differs")
    if payload["commit"] != COMMIT or Path(payload["toolingRoot"]) != ROOT:
        _fail("BrowserType.connect checkout/tooling identity differs")
    if payload["toolingSources"] != policy["captureSourceSha256"]:
        _fail("BrowserType.connect capture source hashes differ")
    _upstream(policy, Path(_string(payload["upstream"], "upstream")))
    _sources(ROOT, _object(policy["supportSources"], "supportSources"), "support")
    _sources(
        ROOT, _object(policy["captureSourceSha256"], "captureSourceSha256"), "capture"
    )
    if payload["wheelSha256"] != WHEEL_SHA256:
        _fail("candidate wheel is not the frozen ABI121 artifact")
    _runtime(policy, payload)
    present = _method_probe(payload)
    if (
        type(payload["publicConnectPresent"]) is not bool
        or payload["publicConnectPresent"] != present
        or present != (payload["mode"] == "oracle")
    ):
        _fail("public BrowserType.connect method presence differs")
    _fixture(payload)
    if not isinstance(payload["runNonce"], str) or not _NONCE.fullmatch(
        payload["runNonce"]
    ):
        _fail("BrowserType.connect nonce is invalid")
    profile, allowed = sandbox_profile(
        Path(payload["pythonExecutable"]),
        Path(payload["driverNode"]),
        payload["mode"],
        payload["fixturePort"],
    )
    if (
        payload["sandboxProfile"] != profile
        or payload["sandboxAllowedExecutables"] != allowed
        or payload["sandboxProfileSha256"]
        != hashlib.sha256(profile.encode()).hexdigest()
        or payload["sandboxProbe"]
        != {
            "allowedMagicDnsConnection": True,
            "otherPortDenied": True,
            "unlistedExecDenied": True,
        }
    ):
        _fail("BrowserType.connect sandbox or live probes differ")
    _environment(payload)
    _command(payload)
    receipt = Path(_string(payload["callReceipt"], "callReceipt"))
    _file(receipt, payload["callReceiptSha256"], "public connect call receipt")
    lines = receipt.read_text().splitlines()
    if payload["mode"] == "candidate":
        if lines or receipt.read_bytes():
            _fail("candidate unexpectedly called public BrowserType.connect")
    else:
        if len(lines) != 1 or not receipt.read_bytes().endswith(b"\n"):
            _fail("oracle must have one newline-terminated public connect call")
        try:
            call = _object(json.loads(lines[0]), "public connect call receipt")
        except json.JSONDecodeError as error:
            raise StrictBrowserConnectEvidenceError(
                "connect receipt is malformed"
            ) from error
        engine = payload["testId"].rsplit("[", 1)[1].removesuffix("]")
        if call != {
            "schema": 1,
            "testId": payload["testId"],
            "mode": "oracle",
            "runNonce": payload["runNonce"],
            "browserName": engine,
            "method": "connect",
            "fixturePort": payload["fixturePort"],
            "originalEndpoint": f"ws://localhost:{payload['fixturePort']}/ws",
            "forwardedEndpoint": f"ws://{payload['fixtureHost']}:{payload['fixturePort']}/ws",
        }:
            _fail("oracle public connect invocation differs from unchanged upstream")
    outcome = report_outcome(payload)
    if payload["outcome"] != outcome:
        _fail("BrowserType.connect recorded outcome differs from JUnit")
    _file(Path(__file__), payload["verifierSha256"], "BrowserType.connect verifier")
    return outcome


def verify_provenance(path: Path) -> dict[str, Any]:
    _file(POLICY, POLICY_SHA256, "BrowserType.connect strict manifest")
    policy = _policy(json.loads(POLICY.read_text()))
    payload = _object(
        json.loads(_regular(path, "strict sidecar").read_text()), "strict sidecar"
    )
    verify_evidence(policy, payload)
    return payload


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("provenance", type=Path)
    args = parser.parse_args()
    try:
        payload = verify_provenance(args.provenance)
    except (
        OSError,
        ValueError,
        zipfile.BadZipFile,
        subprocess.TimeoutExpired,
        subprocess.CalledProcessError,
        json.JSONDecodeError,
    ) as error:
        parser.error(str(error))
    print(
        f"verified strict BrowserType.connect {payload['mode']}/{payload['testId']}: {payload['outcome']}"
    )


if __name__ == "__main__":
    main()
