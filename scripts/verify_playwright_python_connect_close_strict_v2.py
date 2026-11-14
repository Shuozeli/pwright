#!/usr/bin/env python3
"""Independently verify early-WS-close BrowserType.connect v2 evidence.

The candidate and oracle must both invoke the public method, send a nonempty
message to the real fixture, and make the upstream fixture send close 4123.
This is not evidence for a successful remote browser connection.
"""

from __future__ import annotations

import argparse
import hashlib
import ipaddress
import json
import re
import subprocess
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import Any, NoReturn

from verify_playwright_python_ws_transport_strict import (
    COMMIT,
    OFFICIAL,
    ROOT,
    StrictWebSocketEvidenceError,
    _digest,
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

POLICY = OFFICIAL / "async-browser-connect-close-strict-v2-manifest.json"
POLICY_SHA256 = "2d96647c6d06f6ce4db7ff043c6d8cd1561c0486d1d5414b88890c9695699a2c"
KIND = "playwright-python-browser-connect-close-strict-v2"
SCOPE = "strict-public-behavior-browser-connect-early-ws-close-post-fix"
SOURCE = "tests/async/test_browsertype_connect.py"
NAME = "test_should_print_custom_ws_close_error"
TEST_IDS = tuple(f"{SOURCE}::{NAME}[{engine}]" for engine in ("chromium", "firefox", "webkit"))
GUARD = OFFICIAL / "ws_connect_guard_strict_v2"
ADAPTATION = "exact-localhost-ws-to-tailscale-magicdns-only"
_NONCE = re.compile(r"[0-9a-f]{32}\Z")
_HOST = re.compile(r"[a-z0-9-]+(?:\.[a-z0-9-]+)*\.ts\.net\Z")
_CAPTURE_PATHS = (
    "scripts/run_playwright_python_connect_close_strict_v2.py",
    "packages/pwright-python/tests/official/ws_connect_guard_strict_v2/pwright_official_connect_guard_v2.py",
)
_UPSTREAM_PATHS = frozenset(
    {
        SOURCE,
        "tests/conftest.py",
        "tests/async/conftest.py",
        "tests/server.py",
        "tests/async/utils.py",
        "pyproject.toml",
    }
)
_SUPPORT_PATHS = frozenset(
    {
        "packages/pwright-python/tests/official/shim/playwright/__init__.py",
        "packages/pwright-python/tests/official/shim/playwright/async_api.py",
        "packages/pwright-python/tests/official/shim/playwright/_public_types.py",
        "packages/pwright-python/tests/official/shim/pwright_candidate_runtime.py",
        "packages/pwright-python/tests/official/runtime/pwright_official_server_runtime.py",
        "packages/pwright-python/tests/official/runtime/pwright_official_oracle_runtime.py",
        "scripts/run_playwright_python_ws_transport_strict.py",
        "scripts/verify_playwright_python_ws_transport_strict.py",
    }
)
_POLICY_KEYS = frozenset(
    {
        "schema",
        "kind",
        "evidenceScope",
        "repository",
        "tag",
        "commit",
        "requiresBrowser",
        "requiresFixtureServers",
        "fixtureTransportAdaptation",
        "tests",
        "expectedOutcomes",
        "upstreamSources",
        "supportSources",
        "pythonRuntimeSha256",
        "oraclePackageTreeSha256",
        "oracleDriverNodeSha256",
        "candidateWheelSha256",
        "candidatePackageTreeSha256",
        "candidateNativeSha256",
        "candidateRuntimeLibraries",
        "captureReady",
        "captureSourceSha256",
        "captureSourcePaths",
    }
)
_EVIDENCE_KEYS = frozenset(
    {
        "schema",
        "kind",
        "evidenceScope",
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


class StrictConnectCloseV2EvidenceError(StrictWebSocketEvidenceError):
    """A v2 capture cannot receive official conformance credit."""


def _fail(message: str) -> NoReturn:
    raise StrictConnectCloseV2EvidenceError(message)


def _policy(raw: object, *, require_ready: bool = True) -> dict[str, Any]:
    policy = _object(raw, "connect-close v2 policy")
    if set(policy) != _POLICY_KEYS:
        _fail("connect-close v2 policy fields differ")
    if (
        policy["schema"] != 2
        or policy["kind"] != KIND
        or policy["evidenceScope"] != SCOPE
        or policy["repository"] != "https://github.com/microsoft/playwright-python.git"
        or policy["tag"] != "v1.58.0"
        or policy["commit"] != COMMIT
        or policy["requiresBrowser"] is not False
        or policy["requiresFixtureServers"] is not True
        or policy["fixtureTransportAdaptation"] != ADAPTATION
        or policy["tests"] != list(TEST_IDS)
        or policy["expectedOutcomes"] != {"oracle": "passed", "candidate": "passed"}
        or policy["captureSourcePaths"] != list(_CAPTURE_PATHS)
    ):
        _fail("connect-close v2 policy is not the exact three-case scope")
    upstream = _object(policy["upstreamSources"], "upstreamSources")
    support = _object(policy["supportSources"], "supportSources")
    captures = _object(policy["captureSourceSha256"], "captureSourceSha256")
    libraries = _object(policy["candidateRuntimeLibraries"], "candidateRuntimeLibraries")
    if set(upstream) != _UPSTREAM_PATHS or set(support) != _SUPPORT_PATHS:
        _fail("connect-close v2 upstream or support source set differs")
    for name, digest in {**upstream, **support}.items():
        _digest(digest, name)
    for name in ("pythonRuntimeSha256", "oraclePackageTreeSha256", "oracleDriverNodeSha256"):
        _digest(policy[name], name)
    if policy["captureReady"] is not False and policy["captureReady"] is not True:
        _fail("connect-close v2 captureReady is not a bool")
    if require_ready and policy["captureReady"] is not True:
        _fail("connect-close v2 capture is not ready")
    if policy["captureReady"] is True:
        if set(captures) != set(_CAPTURE_PATHS) or not libraries:
            _fail("connect-close v2 source or runtime library pins are incomplete")
        for name, digest in {**captures, **libraries}.items():
            _digest(digest, name)
        for name in ("candidateWheelSha256", "candidatePackageTreeSha256", "candidateNativeSha256"):
            _digest(policy[name], name)
    elif (
        captures
        or libraries
        or any(
            policy[name]
            for name in (
                "candidateWheelSha256",
                "candidatePackageTreeSha256",
                "candidateNativeSha256",
            )
        )
    ):
        _fail("unready connect-close v2 policy has partial candidate pins")
    return policy


def _method_probe(payload: dict[str, Any]) -> bool:
    module = "pwright.async_api" if payload["mode"] == "candidate" else "playwright.async_api"
    script = (
        f"from {module} import BrowserType; "
        "print(int(callable(getattr(BrowserType, 'connect', None))))"
    )
    completed = subprocess.run(
        [
            payload["pythonExecutable"],
            "-c",
            script,
        ],
        check=True,
        capture_output=True,
        text=True,
        timeout=10,
        env={"PATH": "/usr/bin:/bin", "PYTHONNOUSERSITE": "1"},
    )
    if completed.stdout.strip() not in ("0", "1"):
        _fail("public BrowserType.connect probe returned unexpected output")
    return completed.stdout.strip() == "1"


def _fixture(payload: dict[str, Any]) -> None:
    port = payload["fixturePort"]
    if type(port) is not int or not 1024 <= port < 65535:
        _fail("fixture port is invalid")
    try:
        bind = ipaddress.ip_address(_string(payload["fixtureBind"], "fixtureBind"))
    except ValueError as error:
        raise StrictConnectCloseV2EvidenceError("fixture bind is not an IP") from error
    if (
        bind not in ipaddress.ip_network("100.64.0.0/10")
        or not _HOST.fullmatch(_string(payload["fixtureHost"], "fixtureHost"))
        or payload["fixtureTransportAdaptation"] != ADAPTATION
    ):
        _fail("fixture is not exact Tailscale-bound WS adaptation")


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
        "PWRIGHT_PYTHON_CONNECT_V2_EXPECTED_TEST": payload["testId"],
        "PWRIGHT_PYTHON_CONNECT_V2_UPSTREAM": payload["upstream"],
        "PWRIGHT_PYTHON_CONNECT_V2_PORT": str(payload["fixturePort"]),
        "PWRIGHT_PYTHON_CONNECT_V2_RECEIPT": payload["callReceipt"],
        "PWRIGHT_PYTHON_CONNECT_V2_NONCE": payload["runNonce"],
    }
    if payload["environment"] != expected:
        _fail("connect-close v2 environment contains an override")


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
        "pwright_official_connect_guard_v2",
        f"--browser={engine}",
        "--timeout=20",
        "--timeout-method=signal",
        f"{Path(payload['upstream']) / SOURCE}::{NAME}",
        f"--junitxml={payload['result']}",
    ]
    if payload["command"] != expected:
        _fail("connect-close v2 command differs from pinned official selection")


def _receipt_events(payload: dict[str, Any]) -> None:
    receipt = Path(_string(payload["callReceipt"], "callReceipt"))
    _file(receipt, payload["callReceiptSha256"], "connect-close v2 transport receipt")
    raw = receipt.read_bytes()
    lines = raw.splitlines()
    if len(lines) != 4 or not raw.endswith(b"\n"):
        _fail("connect-close v2 requires four newline-terminated transport events")
    try:
        events = [_object(json.loads(line), "transport event") for line in lines]
    except (json.JSONDecodeError, UnicodeDecodeError) as error:
        raise StrictConnectCloseV2EvidenceError("transport receipt is malformed") from error
    common = {
        "schema": 2,
        "mode": payload["mode"],
        "testId": payload["testId"],
        "runNonce": payload["runNonce"],
    }
    engine = payload["testId"].rsplit("[", 1)[1].removesuffix("]")
    expected_call = {
        **common,
        "event": "publicCall",
        "browserName": engine,
        "method": "connect",
        "fixturePort": payload["fixturePort"],
        "originalEndpoint": f"ws://localhost:{payload['fixturePort']}/ws",
        "forwardedEndpoint": f"ws://{payload['fixtureHost']}:{payload['fixturePort']}/ws",
    }
    if events[0] != expected_call or events[1] != {**common, "event": "open"}:
        _fail("public call or fixture open event differs")
    message = events[2]
    protocol_fields = {
        "event",
        "payloadLength",
        "payloadSha256",
        "isBinary",
        "guid",
        "method",
        "sdkLanguage",
        "messageId",
        "apiName",
        "wallTime",
    }
    if (
        set(message) != set(common) | protocol_fields
        or any(message[key] != value for key, value in common.items())
        or message["event"] != "message"
        or type(message["payloadLength"]) is not int
        or message["payloadLength"] <= 0
        or message["isBinary"] is not False
        or message["guid"] != ""
        or message["method"] != "initialize"
        or message["sdkLanguage"] != "python"
        or type(message["messageId"]) is not int
        or message["messageId"] <= 0
        or message["apiName"] != "Connection.init"
        or type(message["wallTime"]) is not int
        or message["wallTime"] <= 0
    ):
        _fail("inbound initialize protocol frame receipt differs")
    _digest(message["payloadSha256"], "inbound message digest")
    if events[3] != {**common, "event": "sendClose", "code": 4123, "reason": "Oh my!"}:
        _fail("real fixture close 4123/Oh my! receipt differs")


def report_outcome(payload: dict[str, Any]) -> str:
    result = Path(_string(payload["result"], "result"))
    if not result.is_absolute() or result.with_suffix(".call.jsonl") != Path(
        payload["callReceipt"]
    ):
        _fail("connect-close v2 output paths differ")
    _file(result, payload["resultSha256"], "official connect-close v2 JUnit")
    try:
        root = ET.fromstring(result.read_bytes())
    except ET.ParseError as error:
        raise StrictConnectCloseV2EvidenceError("official JUnit is malformed") from error
    suites = root.findall("testsuite") if root.tag == "testsuites" else []
    if len(suites) != 1 or len(list(root.iter("testcase"))) != 1:
        _fail("official JUnit must contain one suite and one case")
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
        _fail("official JUnit identity or skip count differs")
    failure = cases[0].find("failure")
    error = cases[0].find("error")
    if (
        failure is None
        and error is None
        and suite.get("failures") == "0"
        and suite.get("errors") == "0"
        and payload["exitCode"] == 0
    ):
        return "passed"
    if (
        payload["mode"] == "candidate"
        and failure is not None
        and error is None
        and suite.get("failures") == "1"
        and suite.get("errors") == "0"
        and payload["exitCode"] == 1
    ):
        return "failed"
    _fail("official JUnit exit or outcome differs")


def verify_evidence(policy: dict[str, Any], payload: dict[str, Any]) -> str:
    if (
        set(payload) != _EVIDENCE_KEYS
        or payload["schema"] != 2
        or payload["kind"] != KIND
        or payload["evidenceScope"] != SCOPE
    ):
        _fail("connect-close v2 sidecar schema differs")
    if payload["mode"] not in ("candidate", "oracle") or payload["testId"] not in TEST_IDS:
        _fail("connect-close v2 mode or test ID differs")
    if Path(payload["manifest"]) != POLICY or payload["manifestSha256"] != sha256_file(POLICY):
        _fail("connect-close v2 policy identity differs")
    if (
        payload["commit"] != COMMIT
        or Path(payload["toolingRoot"]) != ROOT
        or payload["toolingSources"] != policy["captureSourceSha256"]
    ):
        _fail("connect-close v2 source identity differs")
    _upstream(policy, Path(_string(payload["upstream"], "upstream")))
    _sources(ROOT, _object(policy["supportSources"], "supportSources"), "support")
    _sources(ROOT, _object(policy["captureSourceSha256"], "captureSourceSha256"), "capture")
    if payload["wheelSha256"] != policy["candidateWheelSha256"]:
        _fail("candidate wheel differs from frozen opt-in artifact")
    _runtime({**policy, "wheelSha256": policy["candidateWheelSha256"]}, payload)
    if payload["mode"] == "candidate" and (
        payload["pwrightPackageSha256"] != policy["candidatePackageTreeSha256"]
        or payload["nativeSha256"] != policy["candidateNativeSha256"]
        or payload["runtimeLibraries"] != policy["candidateRuntimeLibraries"]
    ):
        _fail("candidate installed artifact differs from policy")
    if payload["publicConnectPresent"] is not True or not _method_probe(payload):
        _fail("real public BrowserType.connect is absent")
    _fixture(payload)
    if not isinstance(payload["runNonce"], str) or not _NONCE.fullmatch(payload["runNonce"]):
        _fail("connect-close v2 nonce is invalid")
    profile, allowed = sandbox_profile(
        Path(payload["pythonExecutable"]),
        Path(payload["driverNode"]),
        payload["mode"],
        payload["fixturePort"],
    )
    if (
        payload["sandboxProfile"] != profile
        or payload["sandboxAllowedExecutables"] != allowed
        or payload["sandboxProfileSha256"] != hashlib.sha256(profile.encode()).hexdigest()
        or payload["sandboxProbe"]
        != {
            "allowedMagicDnsConnection": True,
            "otherPortDenied": True,
            "unlistedExecDenied": True,
        }
    ):
        _fail("connect-close v2 sandbox or probes differ")
    _environment(payload)
    _command(payload)
    _receipt_events(payload)
    outcome = report_outcome(payload)
    if payload["outcome"] != outcome:
        _fail("connect-close v2 outcome differs from JUnit")
    _file(Path(__file__), payload["verifierSha256"], "connect-close v2 verifier")
    return outcome


def verify_provenance(path: Path) -> dict[str, Any]:
    _file(POLICY, POLICY_SHA256, "connect-close v2 manifest")
    policy = _policy(json.loads(POLICY.read_text()))
    payload = _object(json.loads(_regular(path, "connect-close v2 sidecar").read_text()), "sidecar")
    if path != Path(_string(payload.get("result"), "result")).with_suffix(".provenance.json"):
        _fail("connect-close v2 sidecar path does not match its JUnit")
    verify_evidence(policy, payload)
    return payload


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("provenance", type=Path)
    args = parser.parse_args()
    try:
        payload = verify_provenance(args.provenance)
    except (OSError, ValueError, subprocess.TimeoutExpired, subprocess.CalledProcessError) as error:
        parser.error(str(error))
    label = f"{payload['mode']}/{payload['testId']}"
    print(f"verified strict connect-close v2 {label}: {payload['outcome']}")


if __name__ == "__main__":
    main()
