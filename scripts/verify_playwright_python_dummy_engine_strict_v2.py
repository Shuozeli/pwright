#!/usr/bin/env python3
"""Verify one post-fix official Python Firefox/WebKit dummy-launch capture."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path
from typing import Any

from verify_playwright_python_dummy_node_strict import (
    _SIDECAR_KEYS,
    ASSET,
    COMMIT,
    ROOT,
    TEST_NAME,
    StrictEvidenceError,
    _digest,
    _file,
    _fixture,
    _object,
    _receipt,
    _regular,
    _text,
    _tree,
    _upstream,
    file_sha256,
    pwright_runtime_library_paths,
)

POLICY = (
    ROOT / "packages/pwright-python/tests/official/"
    "dummy-node-launch-nonchromium-strict-v2-manifest.json"
)
POLICY_SHA256 = "faa4a608951f661d26342aeeed21d76b976f8170fe606bdbe81f46b3145e9c63"
KIND = "playwright-python-nonchromium-dummy-node-strict-v2"
SCOPE = "strict-public-behavior-nonchromium-dummy-exit-post-fix"
GUARD_RELATIVE = "packages/pwright-python/tests/official/dummy_engine_guard_strict_v2"
RESULT_DIR = ROOT / ".work/official-conformance/results/python-dummy-engine-post-fix-strict-v2"
_NONCE = re.compile(r"^[0-9a-f]{32}$")
_FIELDS = _SIDECAR_KEYS | {"browser"}
_CAPTURE_PATHS = {
    "scripts/run_playwright_python_dummy_engine_strict_v2.py",
    f"{GUARD_RELATIVE}/pwright_official_dummy_engine_guard_v2.py",
    f"{GUARD_RELATIVE}/node",
}
_SUPPORT_PATHS = {
    "scripts/run_playwright_python_dummy_node_strict.py",
    "scripts/verify_playwright_python_dummy_node_strict.py",
}
_UPSTREAM_PATHS = {
    "tests/async/test_launcher.py",
    "tests/sync/test_launcher.py",
    "tests/conftest.py",
    "tests/async/conftest.py",
    "tests/sync/conftest.py",
    "tests/server.py",
    "tests/utils.py",
    ASSET,
    "pyproject.toml",
}


def _fail(message: str) -> None:
    raise StrictEvidenceError(message)


def _policy(raw: object, *, require_ready: bool = True) -> dict[str, Any]:
    policy = _object(raw, "v2 non-Chromium strict policy")
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
        "expectedErrorClass",
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
        _fail("v2 non-Chromium policy fields differ")
    if (
        policy["schema"] != 5
        or policy["kind"] != KIND
        or policy["evidenceScope"] != SCOPE
        or policy["repository"] != "https://github.com/microsoft/playwright-python.git"
        or policy["tag"] != "v1.58.0"
        or policy["commit"] != COMMIT
        or policy["executionPolicy"] != "macos-sandbox-nonchromium-dummy-node-strict-v2"
        or policy["launchMode"] != "owned"
        or policy["requiresBrowser"] is not False
        or policy["requiresFixtureServers"] is not True
        or policy["expectedOutcomes"] != {"candidate": "passed", "oracle": "passed"}
        or policy["expectedErrorClass"]
        != {
            "firefox": "Error",
            "webkit": "TargetClosedError",
        }
    ):
        _fail("v2 non-Chromium policy identity differs")
    tests = _object(policy["tests"], "v2 non-Chromium tests")
    if set(tests) != {"async", "sync"}:
        _fail("v2 non-Chromium API set differs")
    for api in ("async", "sync"):
        selections = _object(tests[api], f"{api} v2 tests")
        if set(selections) != {"firefox", "webkit"} or any(
            selections[browser] != f"tests/{api}/test_launcher.py::{TEST_NAME}[{browser}]"
            for browser in ("firefox", "webkit")
        ):
            _fail("v2 non-Chromium policy must select four exact official cases")
    upstream_sources = _object(policy["upstreamSources"], "upstreamSources")
    if set(upstream_sources) != _UPSTREAM_PATHS:
        _fail("v2 non-Chromium upstream source set differs")
    for label, digest in upstream_sources.items():
        _digest(digest, label)
    for label in (
        "oraclePackageTreeSha256",
        "oracleDriverNodeSha256",
        "systemNodeSha256",
        "pythonRuntimeSha256",
    ):
        _digest(policy[label], label)
    support_trees = _object(policy["supportTrees"], "supportTrees")
    if set(support_trees) != {
        "packages/pwright-python/tests/official/shim",
        "packages/pwright-python/tests/official/runtime",
    }:
        _fail("v2 non-Chromium support tree set differs")
    for label, digest in support_trees.items():
        _digest(digest, label)
    support_sources = _object(policy["supportSources"], "supportSources")
    if set(support_sources) != _SUPPORT_PATHS:
        _fail("v2 non-Chromium support source set differs")
    for label, digest in support_sources.items():
        _digest(digest, label)
    paths = policy["captureSourcePaths"]
    if not isinstance(paths, list) or len(paths) != 3 or set(paths) != _CAPTURE_PATHS:
        _fail("v2 non-Chromium capture source paths differ")
    sources = _object(policy["captureSourceSha256"], "captureSourceSha256")
    if policy["captureReady"] is False and not require_ready:
        if sources or any(
            policy[label] is not None
            for label in (
                "candidateWheelSha256",
                "candidatePackageTreeSha256",
                "candidateNativeSha256",
                "candidateRuntimeLibraries",
            )
        ):
            _fail("unready v2 policy must leave candidate pins empty")
        return policy
    if policy["captureReady"] is not True or set(sources) != _CAPTURE_PATHS:
        _fail("v2 non-Chromium strict capture policy is not ready")
    for label in (
        "candidateWheelSha256",
        "candidatePackageTreeSha256",
        "candidateNativeSha256",
    ):
        _digest(policy[label], label)
    libraries = _object(policy["candidateRuntimeLibraries"], "candidateRuntimeLibraries")
    if not libraries:
        _fail("v2 non-Chromium candidate adjacent dylibs are unpinned")
    for label, digest in libraries.items():
        if not label.endswith(".dylib") or Path(label).name != label:
            _fail("v2 non-Chromium candidate dylib name is invalid")
        _digest(digest, label)
    for label, digest in sources.items():
        _digest(digest, label)
    return policy


def _tooling(policy: dict[str, Any], payload: dict[str, Any], root: Path) -> Path:
    if root.is_symlink() or not root.is_dir():
        _fail("v2 non-Chromium tooling root is invalid")
    sources = _object(payload["toolingSources"], "toolingSources")
    if sources != policy["captureSourceSha256"]:
        _fail("v2 non-Chromium capture source hashes differ")
    for relative, digest in {**policy["supportSources"], **sources}.items():
        path = root / relative
        if not path.resolve().is_relative_to(root.resolve()):
            _fail("v2 non-Chromium tooling source escapes root")
        _file(path, digest, f"v2 tooling source {relative}")
    for relative, digest in policy["supportTrees"].items():
        path = root / relative
        if not path.resolve().is_relative_to(root.resolve()):
            _fail("v2 non-Chromium support tree escapes root")
        _tree(path, digest, f"v2 support tree {relative}")
    guard = root / GUARD_RELATIVE
    if not (guard / "node").stat().st_mode & 0o111:
        _fail("v2 non-Chromium Node wrapper is not executable")
    return guard


def _wheel_matches_install(wheel: Path, site: Path) -> None:
    """Prove the installed candidate package and dylibs came from the pinned wheel."""
    expected: dict[str, str] = {}
    try:
        with zipfile.ZipFile(wheel) as archive:
            for info in archive.infolist():
                name = info.filename
                if (
                    not name
                    or name.startswith("/")
                    or "\\" in name
                    or any(part in {"", ".", ".."} for part in name.rstrip("/").split("/"))
                    or (info.external_attr >> 16) & 0o170000 == 0o120000
                ):
                    _fail("v2 candidate wheel contains an unsafe or linked path")
                if info.is_dir():
                    continue
                if name in expected:
                    _fail("v2 candidate wheel contains a duplicate file")
                if name.startswith(("pwright/", "shuozeli_pwright.dylibs/")):
                    digest = hashlib.sha256()
                    with archive.open(info) as stream:
                        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                            digest.update(chunk)
                    expected[name] = digest.hexdigest()
    except (OSError, zipfile.BadZipFile) as error:
        raise StrictEvidenceError("v2 candidate wheel is not a valid zip") from error
    if not expected or "pwright/_native.abi3.so" not in expected:
        _fail("v2 candidate wheel lacks the native package")
    installed: set[str] = set()
    for directory in (site / "pwright", site / "shuozeli_pwright.dylibs"):
        if directory.is_symlink() or not directory.is_dir():
            _fail("v2 candidate installed directory is missing or linked")
        for path in directory.rglob("*"):
            if path.is_symlink():
                _fail("v2 candidate installed package contains a symlink")
            if path.is_file() and path.suffix == ".pyc" and "__pycache__" not in path.parts:
                _fail("v2 candidate installed package contains an unpinned bytecode module")
            if path.is_file() and "__pycache__" not in path.parts and path.suffix != ".pyc":
                relative = path.relative_to(site).as_posix()
                installed.add(relative)
                if expected.get(relative) != file_sha256(path):
                    _fail(f"v2 candidate installed file differs from wheel: {relative}")
    if installed != set(expected):
        _fail("v2 candidate installed file set differs from pinned wheel")


def _runtime(policy: dict[str, Any], payload: dict[str, Any]) -> None:
    python = Path(_text(payload["pythonExecutable"], "pythonExecutable"))
    if not python.is_absolute() or not python.is_symlink() or not python.is_file():
        _fail("v2 Python interpreter is missing")
    _file(python.resolve(), policy["pythonRuntimeSha256"], "v2 Python runtime")
    if payload["pythonVersion"] != "3.12.14":
        _fail("v2 strict run requires CPython 3.12.14")
    site = python.parent.parent / "lib/python3.12/site-packages"
    official = site / "playwright"
    if Path(payload["playwrightPackage"]) != official:
        _fail("v2 official helper package path differs")
    _tree(official, policy["oraclePackageTreeSha256"], "v2 official Playwright package")
    if payload["playwrightPackageSha256"] != policy["oraclePackageTreeSha256"]:
        _fail("v2 recorded official package digest differs")
    driver = official / "driver/node"
    if Path(payload["driverNode"]) != driver:
        _fail("v2 official driver Node path differs")
    _file(driver, policy["oracleDriverNodeSha256"], "v2 official driver Node")
    if payload["driverNodeSha256"] != policy["oracleDriverNodeSha256"]:
        _fail("v2 recorded driver Node digest differs")
    node = Path(_text(payload["systemNode"], "systemNode"))
    _file(node, policy["systemNodeSha256"], "v2 system Node")
    if payload["systemNodeSha256"] != policy["systemNodeSha256"]:
        _fail("v2 recorded system Node digest differs")
    if payload["mode"] == "oracle":
        if (
            any(
                payload[field] is not None
                for field in (
                    "wheelPath",
                    "wheelSha256",
                    "pwrightPackage",
                    "pwrightPackageSha256",
                    "nativePath",
                    "nativeSha256",
                )
            )
            or payload["runtimeLibraries"] != {}
            or (site / "pwright").exists()
            or (site / "shuozeli_pwright.dylibs").exists()
        ):
            _fail("v2 oracle must not carry or install a Pwright candidate artifact")
        return
    wheel = Path(_text(payload["wheelPath"], "wheelPath"))
    if wheel != Path(payload["result"]).with_suffix(".candidate-wheel.whl"):
        _fail("v2 candidate wheel must be the immutable JUnit sibling snapshot")
    _file(wheel, policy["candidateWheelSha256"], "v2 candidate wheel snapshot")
    if payload["wheelSha256"] != policy["candidateWheelSha256"]:
        _fail("v2 recorded candidate wheel digest differs")
    package = site / "pwright"
    native = package / "_native.abi3.so"
    if Path(payload["pwrightPackage"]) != package or Path(payload["nativePath"]) != native:
        _fail("v2 candidate package/native path differs")
    _tree(package, policy["candidatePackageTreeSha256"], "v2 candidate package")
    _file(native, policy["candidateNativeSha256"], "v2 candidate native artifact")
    if (
        payload["pwrightPackageSha256"] != policy["candidatePackageTreeSha256"]
        or payload["nativeSha256"] != policy["candidateNativeSha256"]
    ):
        _fail("v2 recorded candidate package/native digest differs")
    try:
        libraries = pwright_runtime_library_paths(package)
    except ValueError as error:
        raise StrictEvidenceError("v2 candidate adjacent dylibs are invalid") from error
    expected_libraries = _object(policy["candidateRuntimeLibraries"], "candidateRuntimeLibraries")
    if {path.name for path in libraries} != set(expected_libraries):
        _fail("v2 candidate adjacent dylib set differs")
    if payload["runtimeLibraries"] != expected_libraries:
        _fail("v2 recorded candidate dylib digests differ")
    for path in libraries:
        _file(path, expected_libraries[path.name], f"v2 candidate dylib {path.name}")
    _wheel_matches_install(wheel, site)


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
        _fail("v2 scrubbed worker environment differs")


def _sandbox(payload: dict[str, Any], guard: Path, asset: Path) -> None:
    python = Path(payload["pythonExecutable"])
    allowed = [
        str(python),
        str(python.resolve()),
        str(asset),
        str(guard / "node"),
        "/usr/bin/env",
        "/usr/bin/python3",
        "/Library/Developer/CommandLineTools/usr/bin/python3",
        "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/bin/python3.9",
        "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/Resources/Python.app/Contents/MacOS/Python",
        payload["systemNode"],
    ]
    if payload["mode"] == "oracle":
        allowed.append(payload["driverNode"])
    if payload["sandboxAllowedExecutables"] != allowed or len(allowed) != len(set(allowed)):
        _fail("v2 sandbox executable allowlist differs")
    profile = "\n".join(
        [
            "(version 1)",
            "(allow default)",
            "(deny process-exec)",
            "(deny network-outbound)",
        ]
        + [f"(allow process-exec (literal {json.dumps(path)}))" for path in allowed]
    )
    if payload["sandboxProfile"] != profile:
        _fail("v2 sandbox profile differs")
    if payload["sandboxProfileSha256"] != hashlib.sha256(profile.encode()).hexdigest():
        _fail("v2 sandbox profile digest differs")
    if payload["sandboxProbe"] != {"execDenied": True, "networkDenied": True}:
        _fail("v2 sandbox deny probes were not observed")


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
        "pwright_candidate_runtime"
        if payload["mode"] == "candidate"
        else "pwright_official_oracle_runtime",
        "-p",
        "pwright_official_dummy_engine_guard_v2",
        f"--browser={payload['browser']}",
        "--timeout=20",
        "--timeout-method=signal",
        f"{selected}::{TEST_NAME}",
        f"--junitxml={payload['result']}",
    ]
    if payload["command"] != expected:
        _fail("v2 exact unchanged official pytest command differs")


def _report(payload: dict[str, Any]) -> None:
    result = Path(_text(payload["result"], "result"))
    _file(result, payload["resultSha256"], "v2 official JUnit")
    try:
        root = ET.fromstring(result.read_bytes())
    except ET.ParseError as error:
        raise StrictEvidenceError("v2 official JUnit is malformed") from error
    suites = root.findall("testsuite") if root.tag == "testsuites" else []
    if len(suites) != 1 or len(list(root.iter("testcase"))) != 1:
        _fail("v2 official JUnit must contain one selected case")
    suite = suites[0]
    cases = suite.findall("testcase")
    if len(cases) != 1:
        _fail("v2 official JUnit has nested or extra cases")
    case = cases[0]
    if (
        case.get("classname") != f"tests.{payload['api']}.test_launcher"
        or case.get("name") != f"{TEST_NAME}[{payload['browser']}]"
        or any(case.find(kind) is not None for kind in ("failure", "error", "skipped"))
        or any(
            suite.get(key) != value
            for key, value in (
                ("tests", "1"),
                ("failures", "0"),
                ("errors", "0"),
                ("skipped", "0"),
            )
        )
        or payload["exitCode"] != 0
    ):
        _fail("v2 unchanged official assertion did not pass once")


def _receipts(payload: dict[str, Any], policy: dict[str, Any], asset: Path) -> None:
    result = Path(_text(payload["result"], "result"))
    public = Path(_text(payload["publicReceipt"], "publicReceipt"))
    process = Path(_text(payload["processReceipt"], "processReceipt"))
    if (
        not result.is_absolute()
        or public != result.with_suffix(".public-launch.jsonl")
        or process != result.with_suffix(".dummy-process.jsonl")
    ):
        _fail("v2 receipts must be exact JUnit siblings")
    public_rows = _receipt(public, payload["publicReceiptSha256"], "v2 public receipt")
    process_rows = _receipt(process, payload["processReceiptSha256"], "v2 process receipt")
    common = {
        "api": payload["api"],
        "mode": payload["mode"],
        "browser": payload["browser"],
        "runNonce": payload["runNonce"],
        "executablePath": str(asset),
        "assetSha256": policy["upstreamSources"][ASSET],
    }
    if len(process_rows) != 1 or len(public_rows) != 2:
        _fail("v2 requires one dummy process and two public launch events")
    row = process_rows[0]
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
    ):
        _fail("v2 dummy process receipt does not prove one pinned exit-one child")
    if public_rows[0] != {"event": "public-launch-invoked", **common}:
        _fail("v2 public launch invocation receipt differs")
    failed = public_rows[1]
    if (
        set(failed) != set(common) | {"event", "errorClass", "errorMessage", "processReceiptSha256"}
        or failed.get("event") != "public-launch-failed"
        or any(failed.get(key) != value for key, value in common.items())
        or failed.get("errorClass") != policy["expectedErrorClass"][payload["browser"]]
        or not isinstance(failed.get("errorMessage"), str)
        or not failed["errorMessage"]
        or "PWRIGHT_UNSUPPORTED" in failed["errorMessage"]
        or failed.get("processReceiptSha256") != payload["processReceiptSha256"]
    ):
        _fail("v2 public launch failure lacks matching dummy-process proof")


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
    payload = _object(raw, "v2 non-Chromium strict provenance")
    if set(payload) != _FIELDS:
        _fail("v2 non-Chromium strict provenance fields differ")
    if (
        payload["schema"] != 3
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
        _fail("v2 non-Chromium strict provenance scope or selected test differs")
    if Path(payload["manifest"]) != policy_path or payload["manifestSha256"] != file_sha256(
        policy_path
    ):
        _fail("v2 non-Chromium policy identity differs")
    if Path(payload["toolingRoot"]) != tooling_root:
        _fail("v2 non-Chromium tooling root differs")
    if sidecar_path != Path(payload["result"]).with_suffix(".provenance.json"):
        _fail("v2 non-Chromium sidecar is not the JUnit sibling")
    expected_result = RESULT_DIR / (
        f"{payload['mode']}-{payload['api']}-{payload['browser']}-v1.xml"
    )
    if Path(payload["result"]) != expected_result:
        _fail("v2 non-Chromium result path differs from the immutable lane slot")
    upstream = Path(_text(payload["upstream"], "upstream"))
    asset = _upstream(policy, upstream, check_checkout=check_checkout)
    guard = _tooling(policy, payload, tooling_root)
    _file(Path(__file__), payload["verifierSha256"], "v2 verifier source")
    _runtime(policy, payload)
    _fixture(payload)
    _sandbox(payload, guard, asset)
    _environment(payload, tooling_root, guard, upstream)
    _command(payload, upstream)
    _report(payload)
    _receipts(payload, policy, asset)
    return payload


def verify_provenance(path: Path) -> dict[str, Any]:
    _file(POLICY, POLICY_SHA256, "v2 non-Chromium strict policy")
    _regular(path, "v2 non-Chromium strict provenance")
    try:
        policy = json.loads(POLICY.read_text())
        payload = json.loads(path.read_text())
    except json.JSONDecodeError as error:
        raise StrictEvidenceError("v2 non-Chromium strict JSON is malformed") from error
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
        f"verified v2 non-Chromium dummy {payload['mode']}/{payload['api']}/{payload['browser']} "
        f"resultSha256={payload['resultSha256']}"
    )


if __name__ == "__main__":
    main()
