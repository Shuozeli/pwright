"""Verify one unchanged official closed-endpoint BrowserType.connect case.

This is a fixture-adapted transport slice, not evidence that a Playwright
BrowserServer starts or that a successful remote Browser can be constructed.
"""

from __future__ import annotations

import argparse
import errno
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

POLICY = OFFICIAL / "browser-connect-refused-strict-v1-manifest.json"
POLICY_SHA256 = "87f44977cc9d66b641e03b999e0178c518c50e9ef5014bd2e897afce1b42976b"
KIND = "playwright-python-browser-connect-refused-strict-v1"
SCOPE = "strict-public-behavior-connect-refused-with-fixture-only-adaptation"
NAME = "test_connect_to_closed_server_without_hangs"
SOURCES = (
    "tests/async/test_browsertype_connect.py",
    "tests/sync/test_browsertype_connect.py",
)
TEST_IDS = tuple(
    f"{source}::{NAME}[{engine}]"
    for source in SOURCES
    for engine in ("chromium", "firefox", "webkit")
)
GUARD = OFFICIAL / "ws_connect_refused_guard_strict_v1"
ADAPTATION = "fixture-only-real-tailscale-listener-closed-before-public-connect"
_HOST = re.compile(r"[a-z0-9-]+(?:\.[a-z0-9-]+)*\.ts\.net\Z")
_NONCE = re.compile(r"[0-9a-f]{32}\Z")
_CAPTURE_PATHS = (
    "scripts/run_playwright_python_connect_refused_strict_v1.py",
    "packages/pwright-python/tests/official/ws_connect_refused_guard_strict_v1/pwright_official_connect_refused_guard_v1.py",
)
_UPSTREAM_PATHS = frozenset(
    {
        *SOURCES,
        "tests/conftest.py",
        "tests/async/conftest.py",
        "tests/sync/conftest.py",
        "tests/server.py",
        "pyproject.toml",
    }
)
_SUPPORT_PATHS = frozenset(
    {
        "packages/pwright-python/tests/official/shim/playwright/__init__.py",
        "packages/pwright-python/tests/official/shim/playwright/async_api.py",
        "packages/pwright-python/tests/official/shim/playwright/sync_api.py",
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
        "fixtureAdaptation",
        "tests",
        "expectedOutcomes",
        "upstreamSources",
        "supportSources",
        "pythonRuntimeSha256",
        "oraclePackageTreeSha256",
        "oracleDriverNodeSha256",
        "candidateAbi",
        "candidateRequiredFeature",
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
        "fixtureAdaptation",
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


class StrictConnectRefusedV1EvidenceError(StrictWebSocketEvidenceError):
    """The closed-endpoint run cannot receive official conformance credit."""


def _fail(message: str) -> NoReturn:
    raise StrictConnectRefusedV1EvidenceError(message)


def _policy(raw: object, *, require_ready: bool = True) -> dict[str, Any]:
    policy = _object(raw, "connect-refused v1 policy")
    if set(policy) != _POLICY_KEYS:
        _fail("connect-refused v1 policy fields differ")
    if (
        policy["schema"] != 1
        or policy["kind"] != KIND
        or policy["evidenceScope"] != SCOPE
        or policy["repository"] != "https://github.com/microsoft/playwright-python.git"
        or policy["tag"] != "v1.58.0"
        or policy["commit"] != COMMIT
        or policy["requiresBrowser"] is not False
        or policy["requiresFixtureServers"] is not True
        or policy["fixtureAdaptation"] != ADAPTATION
        or policy["candidateAbi"] != 123
        or policy["candidateRequiredFeature"] != "playwright-server-initialize"
        or policy["tests"] != list(TEST_IDS)
        or policy["expectedOutcomes"] != {"oracle": "passed", "candidate": "passed"}
        or policy["captureSourcePaths"] != list(_CAPTURE_PATHS)
    ):
        _fail("connect-refused v1 policy is not the exact six-case scope")
    upstream = _object(policy["upstreamSources"], "upstreamSources")
    support = _object(policy["supportSources"], "supportSources")
    captures = _object(policy["captureSourceSha256"], "captureSourceSha256")
    libraries = _object(
        policy["candidateRuntimeLibraries"], "candidateRuntimeLibraries"
    )
    if set(upstream) != _UPSTREAM_PATHS or set(support) != _SUPPORT_PATHS:
        _fail("connect-refused v1 upstream or support source set differs")
    for name, digest in {**upstream, **support}.items():
        _digest(digest, name)
    for name in (
        "pythonRuntimeSha256",
        "oraclePackageTreeSha256",
        "oracleDriverNodeSha256",
    ):
        _digest(policy[name], name)
    if type(policy["captureReady"]) is not bool:
        _fail("connect-refused v1 captureReady is not a bool")
    if require_ready and not policy["captureReady"]:
        _fail("connect-refused v1 capture is not ready")
    if policy["captureReady"]:
        if set(captures) != set(_CAPTURE_PATHS) or not libraries:
            _fail(
                "connect-refused v1 capture sources or runtime libraries are incomplete"
            )
        for name, digest in {**captures, **libraries}.items():
            _digest(digest, name)
        for name in (
            "candidateWheelSha256",
            "candidatePackageTreeSha256",
            "candidateNativeSha256",
        ):
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
        _fail("unready connect-refused v1 policy has partial candidate pins")
    return policy


def _method_probe(payload: dict[str, Any]) -> bool:
    module = "pwright" if payload["mode"] == "candidate" else "playwright"
    script = (
        f"from {module}.async_api import BrowserType as A; "
        f"from {module}.sync_api import BrowserType as S; "
        "print(int(callable(getattr(A, 'connect', None)) and callable(getattr(S, 'connect', None))))"
    )
    completed = subprocess.run(
        [payload["pythonExecutable"], "-c", script],
        check=True,
        capture_output=True,
        text=True,
        timeout=10,
        env={"PATH": "/usr/bin:/bin", "PYTHONNOUSERSITE": "1"},
    )
    if completed.stdout.strip() not in ("0", "1"):
        _fail("public BrowserType.connect probe returned unexpected output")
    return completed.stdout.strip() == "1"


def _candidate_feature_probe(payload: dict[str, Any], policy: dict[str, Any]) -> bool:
    if payload["mode"] != "candidate":
        return True
    script = (
        "from pwright import native_capabilities; "
        "from pwright._native import selected_features; "
        f"print(int(native_capabilities()['abi'] == {policy['candidateAbi']} "
        f"and {policy['candidateRequiredFeature']!r} in selected_features()))"
    )
    completed = subprocess.run(
        [payload["pythonExecutable"], "-c", script],
        check=True,
        capture_output=True,
        text=True,
        timeout=10,
        env={"PATH": "/usr/bin:/bin", "PYTHONNOUSERSITE": "1"},
    )
    if completed.stdout.strip() not in ("0", "1"):
        _fail("candidate ABI/feature probe returned unexpected output")
    return completed.stdout.strip() == "1"


def _fixture(payload: dict[str, Any]) -> None:
    port = payload["fixturePort"]
    if type(port) is not int or not 1024 <= port < 65535:
        _fail("fixture port is invalid")
    try:
        bind = ipaddress.ip_address(_string(payload["fixtureBind"], "fixtureBind"))
    except ValueError as error:
        raise StrictConnectRefusedV1EvidenceError(
            "fixture bind is not an IP"
        ) from error
    if (
        bind not in ipaddress.ip_network("100.64.0.0/10")
        or not _HOST.fullmatch(_string(payload["fixtureHost"], "fixtureHost"))
        or payload["fixtureAdaptation"] != ADAPTATION
    ):
        _fail("fixture is not the declared Tailscale-bound closed endpoint")


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
        "PWRIGHT_PYTHON_REFUSED_V1_EXPECTED_TEST": payload["testId"],
        "PWRIGHT_PYTHON_REFUSED_V1_UPSTREAM": payload["upstream"],
        "PWRIGHT_PYTHON_REFUSED_V1_PORT": str(payload["fixturePort"]),
        "PWRIGHT_PYTHON_REFUSED_V1_RECEIPT": payload["callReceipt"],
        "PWRIGHT_PYTHON_REFUSED_V1_NONCE": payload["runNonce"],
    }
    if payload["environment"] != expected:
        _fail("connect-refused v1 environment contains an override")


def _command(payload: dict[str, Any]) -> None:
    source, _, selected = payload["testId"].partition("::")
    engine = selected.rsplit("[", 1)[1].removesuffix("]")
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
        "pwright_official_connect_refused_guard_v1",
        f"--browser={engine}",
        "--timeout=20",
        "--timeout-method=signal",
        f"{Path(payload['upstream']) / source}::{NAME}",
        f"--junitxml={payload['result']}",
    ]
    if payload["command"] != expected:
        _fail("connect-refused v1 command differs from pinned official selection")


def _receipt_events(payload: dict[str, Any]) -> bool:
    receipt = Path(_string(payload["callReceipt"], "callReceipt"))
    _file(receipt, payload["callReceiptSha256"], "connect-refused v1 call receipt")
    raw = receipt.read_bytes()
    lines = raw.splitlines()
    if len(lines) != 4 or not raw.endswith(b"\n"):
        _fail("connect-refused v1 requires four newline-terminated events")
    try:
        events = [_object(json.loads(line), "connect-refused event") for line in lines]
    except (json.JSONDecodeError, UnicodeDecodeError) as error:
        raise StrictConnectRefusedV1EvidenceError(
            "connect-refused receipt is malformed"
        ) from error
    common = {
        "schema": 1,
        "mode": payload["mode"],
        "testId": payload["testId"],
        "runNonce": payload["runNonce"],
    }
    engine = payload["testId"].rsplit("[", 1)[1].removesuffix("]")
    endpoint = f"ws://{payload['fixtureHost']}:{payload['fixturePort']}/ws"
    if any(type(event.get("schema")) is not int for event in events):
        _fail("connect-refused receipt schema must be an integer")
    if events[0] != {
        **common,
        "event": "fixtureListening",
        "browserName": engine,
        "bindIp": payload["fixtureBind"],
        "fixturePort": payload["fixturePort"],
        "endpoint": endpoint,
        "reachableConnectErrno": 0,
        "originalRemoteServerLaunched": False,
    }:
        _fail("real fixture listener receipt differs")
    if (
        type(events[0]["reachableConnectErrno"]) is not int
        or events[0]["originalRemoteServerLaunched"] is not False
    ):
        _fail("fixture listener probe or browser-launch type differs")
    if events[1] != {
        **common,
        "event": "fixtureClosed",
        "refusedConnectErrno": errno.ECONNREFUSED,
    }:
        _fail("real fixture refusal receipt differs")
    if type(events[1]["refusedConnectErrno"]) is not int:
        _fail("fixture refusal errno must be an integer")
    if events[2] != {
        **common,
        "event": "publicCall",
        "browserName": engine,
        "method": "connect",
        "endpoint": endpoint,
        "preCallRefusedConnectErrno": errno.ECONNREFUSED,
    }:
        _fail("public connect call or pre-call refusal differs")
    if type(events[2]["preCallRefusedConnectErrno"]) is not int:
        _fail("public connect pre-call refusal errno must be an integer")
    error = events[3]
    fields = {"event", "errorClass", "messageSha256", "hasRequiredPrefix"}
    if (
        set(error) != set(common) | fields
        or any(error[key] != value for key, value in common.items())
        or error["event"] != "publicError"
        or not isinstance(error["errorClass"], str)
        or not error["errorClass"]
        or type(error["hasRequiredPrefix"]) is not bool
    ):
        _fail("public connect error receipt differs")
    _digest(error["messageSha256"], "public connect error digest")
    return error["hasRequiredPrefix"]


def report_outcome(payload: dict[str, Any]) -> str:
    result = Path(_string(payload["result"], "result"))
    if not result.is_absolute() or result.with_suffix(".call.jsonl") != Path(
        payload["callReceipt"]
    ):
        _fail("connect-refused v1 output paths differ")
    _file(result, payload["resultSha256"], "official connect-refused v1 JUnit")
    try:
        root = ET.fromstring(result.read_bytes())
    except ET.ParseError as error:
        raise StrictConnectRefusedV1EvidenceError(
            "official JUnit is malformed"
        ) from error
    suites = root.findall("testsuite") if root.tag == "testsuites" else []
    if len(suites) != 1 or len(list(root.iter("testcase"))) != 1:
        _fail("official JUnit must contain one suite and one case")
    suite = suites[0]
    cases = suite.findall("testcase")
    source = payload["testId"].split("::", 1)[0]
    expected_class = (
        "tests.async.test_browsertype_connect"
        if source.startswith("tests/async/")
        else "tests.sync.test_browsertype_connect"
    )
    if (
        len(cases) != 1
        or cases[0].get("classname") != expected_class
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
        or payload["schema"] != 1
        or payload["kind"] != KIND
        or payload["evidenceScope"] != SCOPE
    ):
        _fail("connect-refused v1 sidecar schema differs")
    if (
        payload["mode"] not in ("candidate", "oracle")
        or payload["testId"] not in TEST_IDS
    ):
        _fail("connect-refused v1 mode or test ID differs")
    if Path(payload["manifest"]) != POLICY or payload["manifestSha256"] != sha256_file(
        POLICY
    ):
        _fail("connect-refused v1 policy identity differs")
    if (
        payload["commit"] != COMMIT
        or Path(payload["toolingRoot"]) != ROOT
        or payload["toolingSources"] != policy["captureSourceSha256"]
    ):
        _fail("connect-refused v1 source identity differs")
    _upstream(policy, Path(_string(payload["upstream"], "upstream")))
    _sources(ROOT, _object(policy["supportSources"], "supportSources"), "support")
    _sources(
        ROOT, _object(policy["captureSourceSha256"], "captureSourceSha256"), "capture"
    )
    if payload["wheelSha256"] != policy["candidateWheelSha256"]:
        _fail("candidate wheel differs from frozen opt-in artifact")
    _runtime({**policy, "wheelSha256": policy["candidateWheelSha256"]}, payload)
    if payload["mode"] == "candidate" and (
        payload["pwrightPackageSha256"] != policy["candidatePackageTreeSha256"]
        or payload["nativeSha256"] != policy["candidateNativeSha256"]
        or payload["runtimeLibraries"] != policy["candidateRuntimeLibraries"]
    ):
        _fail("candidate installed artifact differs from policy")
    if (
        payload["publicConnectPresent"] is not True
        or not _method_probe(payload)
        or not _candidate_feature_probe(payload, policy)
    ):
        _fail("ABI123 public BrowserType.connect or selected feature is absent")
    _fixture(payload)
    if not isinstance(payload["runNonce"], str) or not _NONCE.fullmatch(
        payload["runNonce"]
    ):
        _fail("connect-refused v1 nonce is invalid")
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
        _fail("connect-refused v1 sandbox or probes differ")
    _environment(payload)
    _command(payload)
    has_required_prefix = _receipt_events(payload)
    outcome = report_outcome(payload)
    if has_required_prefix != (outcome == "passed"):
        _fail("public error prefix does not explain the official assertion outcome")
    if payload["outcome"] != outcome:
        _fail("connect-refused v1 outcome differs from JUnit")
    _file(Path(__file__), payload["verifierSha256"], "connect-refused v1 verifier")
    return outcome


def verify_provenance(path: Path) -> dict[str, Any]:
    _file(POLICY, POLICY_SHA256, "connect-refused v1 manifest")
    policy = _policy(json.loads(POLICY.read_text()))
    payload = _object(
        json.loads(_regular(path, "connect-refused v1 sidecar").read_text()), "sidecar"
    )
    if path != Path(_string(payload.get("result"), "result")).with_suffix(
        ".provenance.json"
    ):
        _fail("connect-refused v1 sidecar path does not match its JUnit")
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
        subprocess.TimeoutExpired,
        subprocess.CalledProcessError,
    ) as error:
        parser.error(str(error))
    print(
        f"verified strict connect-refused v1 {payload['mode']}/{payload['testId']}: {payload['outcome']}"
    )


if __name__ == "__main__":
    main()
