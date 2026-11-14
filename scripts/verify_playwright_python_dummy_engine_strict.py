#!/usr/bin/env python3
"""Verify source-bound non-Chromium official dummy-launch outcomes."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import Any

from verify_playwright_python_dummy_node_strict import (
    ASSET,
    COMMIT,
    ROOT,
    TEST_NAME,
    StrictEvidenceError,
    _SIDECAR_KEYS,
    _digest,
    _file,
    _fixture,
    _object,
    _receipt,
    _regular,
    _runtime,
    _sandbox,
    _text,
    _tree,
    _upstream,
    file_sha256,
)

POLICY = (
    ROOT / "packages/pwright-python/tests/official/"
    "dummy-node-launch-abi121-nonchromium-strict-v1-manifest.json"
)
POLICY_SHA256 = "230664760771fb6890de4875f18dddbd89b8b42e6bf9f841b31723ef914bebf6"
KIND = "playwright-python-nonchromium-dummy-node-strict-v1"
SCOPE = "strict-public-behavior-nonchromium-dummy-exit"
_NONCE = re.compile(r"^[0-9a-f]{32}$")
_SIDECAR_FIELDS = _SIDECAR_KEYS | {"browser"}
_CAPTURE_PATHS = {
    "scripts/run_playwright_python_dummy_engine_strict.py",
    "packages/pwright-python/tests/official/dummy_engine_guard_strict/pwright_official_dummy_engine_guard.py",
    "packages/pwright-python/tests/official/dummy_engine_guard_strict/node",
}
_SUPPORT_PATHS = {
    "scripts/run_playwright_python_dummy_node_strict.py",
    "scripts/verify_playwright_python_dummy_node_strict.py",
}


def _fail(message: str) -> None:
    raise StrictEvidenceError(message)


def _policy(raw: object) -> dict[str, Any]:
    policy = _object(raw, "non-Chromium strict policy")
    expected = {
        "schema",
        "kind",
        "evidenceScope",
        "repository",
        "tag",
        "commit",
        "executionPolicy",
        "launchMode",
        "requiresBrowser",
        "requiresFixtureServers",
        "tests",
        "expectedOutcomes",
        "candidateErrorClass",
        "candidateErrorMessage",
        "oracleErrorClass",
        "upstreamSources",
        "candidateWheelSha256",
        "candidatePackageTreeSha256",
        "candidateNativeSha256",
        "candidateRuntimeLibraries",
        "oraclePackageTreeSha256",
        "oracleDriverNodeSha256",
        "systemNodeSha256",
        "pythonRuntimeSha256",
        "supportTrees",
        "supportSources",
        "captureReady",
        "captureSourceSha256",
        "captureSourcePaths",
    }
    if set(policy) != expected:
        _fail("non-Chromium strict policy fields differ")
    if (
        policy["schema"] != 4
        or policy["kind"] != KIND
        or policy["evidenceScope"] != SCOPE
        or policy["repository"] != "https://github.com/microsoft/playwright-python.git"
        or policy["tag"] != "v1.58.0"
        or policy["commit"] != COMMIT
        or policy["executionPolicy"] != "macos-sandbox-nonchromium-dummy-node-strict-v1"
        or policy["launchMode"] != "owned"
        or policy["requiresBrowser"] is not False
        or policy["requiresFixtureServers"] is not True
        or policy["expectedOutcomes"] != {"candidate": "failed", "oracle": "passed"}
        or policy["candidateErrorClass"] != "UnsupportedError"
        or policy["candidateErrorMessage"] != "launch is only available for Chromium"
        or policy["oracleErrorClass"] != {"firefox": "Error", "webkit": "TargetClosedError"}
    ):
        _fail("non-Chromium strict policy identity differs")
    tests = _object(policy["tests"], "non-Chromium tests")
    if set(tests) != {"async", "sync"}:
        _fail("non-Chromium strict policy API set differs")
    for api in ("async", "sync"):
        selections = _object(tests[api], f"{api} tests")
        if set(selections) != {"firefox", "webkit"} or any(
            selections[browser] != f"tests/{api}/test_launcher.py::{TEST_NAME}[{browser}]"
            for browser in ("firefox", "webkit")
        ):
            _fail("non-Chromium strict policy must select four exact cases")
    sources = _object(policy["upstreamSources"], "upstreamSources")
    if set(sources) != {
        "tests/async/test_launcher.py",
        "tests/sync/test_launcher.py",
        "tests/conftest.py",
        "tests/async/conftest.py",
        "tests/sync/conftest.py",
        "tests/server.py",
        "tests/utils.py",
        ASSET,
        "pyproject.toml",
    }:
        _fail("non-Chromium upstream source set differs")
    for label, digest in sources.items():
        _digest(digest, label)
    for label in (
        "candidateWheelSha256",
        "candidatePackageTreeSha256",
        "candidateNativeSha256",
        "oraclePackageTreeSha256",
        "oracleDriverNodeSha256",
        "systemNodeSha256",
        "pythonRuntimeSha256",
    ):
        _digest(policy[label], label)
    libraries = _object(policy["candidateRuntimeLibraries"], "candidateRuntimeLibraries")
    if set(libraries) != {"libcrypto-a12805a1.3.dylib", "libssl-ffd8ac69.3.dylib"}:
        _fail("non-Chromium adjacent dylib set differs")
    for label, digest in libraries.items():
        _digest(digest, label)
    support_trees = _object(policy["supportTrees"], "supportTrees")
    if set(support_trees) != {
        "packages/pwright-python/tests/official/shim",
        "packages/pwright-python/tests/official/runtime",
    }:
        _fail("non-Chromium support tree set differs")
    for label, digest in support_trees.items():
        _digest(digest, label)
    support_sources = _object(policy["supportSources"], "supportSources")
    if set(support_sources) != _SUPPORT_PATHS:
        _fail("non-Chromium support source set differs")
    for label, digest in support_sources.items():
        _digest(digest, label)
    paths = policy["captureSourcePaths"]
    hashes = _object(policy["captureSourceSha256"], "captureSourceSha256")
    if not isinstance(paths, list) or set(paths) != _CAPTURE_PATHS or len(paths) != 3:
        _fail("non-Chromium capture source paths differ")
    if policy["captureReady"] is not True or set(hashes) != _CAPTURE_PATHS:
        _fail("non-Chromium strict capture policy is not ready")
    for label, digest in hashes.items():
        _digest(digest, label)
    return policy


def _tooling(policy: dict[str, Any], payload: dict[str, Any], root: Path) -> Path:
    if root.is_symlink() or not root.is_dir():
        _fail("non-Chromium tooling root is invalid")
    sources = _object(payload["toolingSources"], "toolingSources")
    if sources != policy["captureSourceSha256"]:
        _fail("non-Chromium capture source hashes differ")
    for relative, digest in {**policy["supportSources"], **sources}.items():
        path = root / relative
        if not path.resolve().is_relative_to(root.resolve()):
            _fail("non-Chromium tooling source escapes root")
        _file(path, digest, f"tooling source {relative}")
    for relative, digest in policy["supportTrees"].items():
        path = root / relative
        if not path.resolve().is_relative_to(root.resolve()):
            _fail("non-Chromium support tree escapes root")
        _tree(path, digest, f"support tree {relative}")
    guard = root / "packages/pwright-python/tests/official/dummy_engine_guard_strict"
    if not (guard / "node").stat().st_mode & 0o111:
        _fail("non-Chromium Node wrapper is not executable")
    return guard


def _environment(payload: dict[str, Any], root: Path, guard: Path, upstream: Path) -> None:
    official = root / "packages/pwright-python/tests/official"
    python_paths = (
        [official / "shim", upstream, official / "runtime", guard]
        if payload["mode"] == "candidate"
        else [official / "runtime", guard]
    )
    expected = {
        "PATH": f"{guard}:/opt/homebrew/bin:/usr/bin:/bin",
        "PYTHONPATH": ":".join(map(str, python_paths)),
        "PYTHONNOUSERSITE": "1",
        "PYTEST_DISABLE_PLUGIN_AUTOLOAD": "1",
        "PWRIGHT_OFFICIAL_MODE": payload["mode"],
        "PWRIGHT_OFFICIAL_LAUNCH_MODE": "owned",
        "PWRIGHT_OFFICIAL_BROWSER_CHANNEL": "",
        "PWRIGHT_OFFICIAL_UPSTREAM_TESTS": str(upstream / "tests"),
        "PWRIGHT_OFFICIAL_TAILSCALE_IP": payload["fixtureBind"],
        "PWRIGHT_OFFICIAL_TAILSCALE_HOST": payload["fixtureHost"],
        "PWRIGHT_PYTHON_DUMMY_UPSTREAM": str(upstream),
        "PWRIGHT_PYTHON_DUMMY_API": payload["api"],
        "PWRIGHT_PYTHON_DUMMY_BROWSER": payload["browser"],
        "PWRIGHT_PYTHON_DUMMY_LAUNCH_RECEIPT": payload["publicReceipt"],
        "PWRIGHT_PYTHON_DUMMY_PROCESS_RECEIPT": payload["processReceipt"],
        "PWRIGHT_PYTHON_DUMMY_REAL_NODE": payload["systemNode"],
        "PWRIGHT_PYTHON_DUMMY_NONCE": payload["runNonce"],
    }
    if payload["environment"] != expected:
        _fail("non-Chromium scrubbed environment differs")


def _command(payload: dict[str, Any], upstream: Path) -> None:
    selected = upstream / f"tests/{payload['api']}/test_launcher.py"
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
        str(upstream / "pyproject.toml"),
        "-p",
        "pytest_asyncio.plugin",
        "-p",
        "pytest_timeout",
        "-p",
        "pwright_official_server_runtime",
        "-p",
        (
            "pwright_candidate_runtime"
            if payload["mode"] == "candidate"
            else "pwright_official_oracle_runtime"
        ),
        "-p",
        "pwright_official_dummy_engine_guard",
        f"--browser={payload['browser']}",
        "--timeout=20",
        "--timeout-method=signal",
        f"{selected}::{TEST_NAME}",
        f"--junitxml={payload['result']}",
    ]
    if payload["command"] != expected:
        _fail("non-Chromium exact official command differs")


def _report(payload: dict[str, Any], policy: dict[str, Any]) -> None:
    result = Path(_text(payload["result"], "result"))
    _file(result, payload["resultSha256"], "non-Chromium official JUnit")
    try:
        root = ET.fromstring(result.read_bytes())
    except ET.ParseError as error:
        raise StrictEvidenceError("non-Chromium official JUnit is malformed") from error
    suites = root.findall("testsuite") if root.tag == "testsuites" else []
    if len(suites) != 1 or len(list(root.iter("testcase"))) != 1:
        _fail("non-Chromium official JUnit must contain one case")
    suite = suites[0]
    cases = suite.findall("testcase")
    if len(cases) != 1:
        _fail("non-Chromium official JUnit has nested or extra cases")
    case = cases[0]
    if (
        case.get("classname") != f"tests.{payload['api']}.test_launcher"
        or case.get("name") != f"{TEST_NAME}[{payload['browser']}]"
        or suite.get("tests") != "1"
        or suite.get("errors") != "0"
        or suite.get("skipped") != "0"
        or case.find("error") is not None
        or case.find("skipped") is not None
    ):
        _fail("non-Chromium official JUnit selected case or status differs")
    if payload["mode"] == "oracle":
        if (
            suite.get("failures") != "0"
            or case.find("failure") is not None
            or payload["exitCode"] != 0
        ):
            _fail("non-Chromium oracle did not pass the unchanged official assertion")
    else:
        failure = case.find("failure")
        if (
            suite.get("failures") != "1"
            or failure is None
            or payload["exitCode"] != 1
            or policy["candidateErrorClass"] not in (failure.get("message") or "")
            or policy["candidateErrorMessage"] not in (failure.get("message") or "")
        ):
            _fail("non-Chromium candidate did not fail with the pinned unsupported error")


def _receipts(payload: dict[str, Any], policy: dict[str, Any], asset: Path) -> None:
    result = Path(payload["result"])
    public = Path(_text(payload["publicReceipt"], "publicReceipt"))
    process = Path(_text(payload["processReceipt"], "processReceipt"))
    if (
        not result.is_absolute()
        or public != result.with_suffix(".public-launch.jsonl")
        or process != result.with_suffix(".dummy-process.jsonl")
    ):
        _fail("non-Chromium receipts must be exact JUnit siblings")
    public_rows = _receipt(public, payload["publicReceiptSha256"], "public receipt")
    common = {
        "api": payload["api"],
        "mode": payload["mode"],
        "browser": payload["browser"],
        "runNonce": payload["runNonce"],
        "executablePath": str(asset),
        "assetSha256": policy["upstreamSources"][ASSET],
    }
    if len(public_rows) != 2 or public_rows[0] != {"event": "public-launch-invoked", **common}:
        _fail("non-Chromium public launch invocation receipt differs")
    failed = public_rows[1]
    if (
        set(failed)
        != set(common)
        | {
            "event",
            "errorClass",
            "errorMessage",
            "processReceiptSha256",
        }
        or failed.get("event") != "public-launch-failed"
        or any(failed.get(key) != value for key, value in common.items())
    ):
        _fail("non-Chromium public launch failure receipt differs")
    _file(process, payload["processReceiptSha256"], "non-Chromium process receipt")
    if payload["mode"] == "candidate":
        if (
            process.read_bytes() != b""
            or failed.get("errorClass") != policy["candidateErrorClass"]
            or failed.get("errorMessage") != policy["candidateErrorMessage"]
            or failed.get("processReceiptSha256") is not None
        ):
            _fail("non-Chromium candidate must fail before spawning the dummy asset")
        return
    rows = _receipt(process, payload["processReceiptSha256"], "process receipt")
    if len(rows) != 1:
        _fail("non-Chromium oracle process receipt count differs")
    row = rows[0]
    if (
        set(row) != set(common) | {"event", "exitCode", "childPid", "node", "nodeSha256", "argv"}
        or any(row.get(key) != value for key, value in common.items())
        or row.get("event") != "dummy-node-exited"
        or row.get("exitCode") != 1
        or type(row.get("childPid")) is not int
        or row["childPid"] <= 0
        or row.get("node") != payload["systemNode"]
        or row.get("nodeSha256") != policy["systemNodeSha256"]
        or not isinstance(row.get("argv"), list)
        or row["argv"][:2] != [payload["systemNode"], str(asset)]
        or any(not isinstance(argument, str) for argument in row["argv"])
        or failed.get("errorClass") != policy["oracleErrorClass"][payload["browser"]]
        or not isinstance(failed.get("errorMessage"), str)
        or not failed["errorMessage"]
        or failed.get("processReceiptSha256") != payload["processReceiptSha256"]
    ):
        _fail("non-Chromium oracle lacks pinned dummy-process exit proof")


def verify_evidence(
    raw: object,
    *,
    policy: dict[str, Any],
    policy_path: Path,
    sidecar_path: Path,
    tooling_root: Path = ROOT,
    check_checkout: bool = True,
) -> dict[str, Any]:
    policy = _policy(policy)
    payload = _object(raw, "non-Chromium strict provenance")
    if set(payload) != _SIDECAR_FIELDS:
        _fail("non-Chromium strict provenance fields differ")
    if (
        payload["schema"] != 2
        or payload["kind"] != KIND
        or payload["evidenceScope"] != SCOPE
        or payload["mode"] not in {"candidate", "oracle"}
        or payload["api"] not in {"async", "sync"}
        or payload["browser"] not in {"firefox", "webkit"}
        or payload["testId"] != policy["tests"][payload["api"]][payload["browser"]]
        or payload["commit"] != COMMIT
        or payload["launchMode"] != "owned"
        or payload["endpoint"] is not None
        or payload["collectOnly"] is not False
        or payload["bootstrapOnly"] is not False
        or not isinstance(payload["runNonce"], str)
        or _NONCE.fullmatch(payload["runNonce"]) is None
    ):
        _fail("non-Chromium strict provenance scope or exact test differs")
    if Path(payload["manifest"]) != policy_path or payload["manifestSha256"] != file_sha256(
        policy_path
    ):
        _fail("non-Chromium strict policy identity differs")
    if Path(payload["toolingRoot"]) != tooling_root:
        _fail("non-Chromium tooling root differs")
    if sidecar_path != Path(payload["result"]).with_suffix(".provenance.json"):
        _fail("non-Chromium sidecar is not the JUnit sibling")
    upstream = Path(_text(payload["upstream"], "upstream"))
    asset = _upstream(policy, upstream, check_checkout=check_checkout)
    guard = _tooling(policy, payload, tooling_root)
    _file(Path(__file__), payload["verifierSha256"], "non-Chromium verifier source")
    _runtime(policy, payload)
    _fixture(payload)
    _sandbox(payload, guard, asset)
    _environment(payload, tooling_root, guard, upstream)
    _command(payload, upstream)
    _report(payload, policy)
    _receipts(payload, policy, asset)
    return payload


def verify_provenance(path: Path) -> dict[str, Any]:
    _file(POLICY, POLICY_SHA256, "non-Chromium strict policy")
    _regular(path, "non-Chromium strict provenance")
    try:
        policy = json.loads(POLICY.read_text())
        payload = json.loads(path.read_text())
    except json.JSONDecodeError as error:
        raise StrictEvidenceError("non-Chromium strict JSON is malformed") from error
    return verify_evidence(payload, policy=policy, policy_path=POLICY, sidecar_path=path)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("provenance", type=Path)
    args = parser.parse_args()
    try:
        payload = verify_provenance(args.provenance)
    except (OSError, StrictEvidenceError) as error:
        parser.error(str(error))
    print(
        f"verified non-Chromium dummy {payload['mode']}/{payload['api']}/{payload['browser']} "
        f"resultSha256={payload['resultSha256']}"
    )


if __name__ == "__main__":
    main()
