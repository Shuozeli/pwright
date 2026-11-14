#!/usr/bin/env python3
"""Fail-closed verifier for guarded, browser-free official launch evidence."""

from __future__ import annotations

import argparse
import hashlib
import ipaddress
import json
import re
import subprocess
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import Any, NoReturn, cast

ROOT = Path(__file__).resolve().parents[1]
POLICY = (
    ROOT / "packages/pwright-python/tests/official/"
    "dummy-node-launch-abi119-strict-v1-manifest.json"
)
POLICY_SHA256 = "b68e2a021f6d29dc399d13bd2864ffffe1d972c14960c2f303d5e261d8fc6b78"
KIND = "playwright-python-guarded-dummy-node-strict-v1"
SCOPE = "strict-public-behavior-guarded-dummy-exit"
COMMIT = "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
ASSET = "tests/assets/dummy_bad_browser_executable.js"
TEST_NAME = (
    "test_browser_type_launch_should_reject_if_launched_browser_fails_immediately"
)
_SHA = re.compile(r"^[0-9a-f]{64}$")
_NONCE = re.compile(r"^[0-9a-f]{32}$")
_SIDECAR_KEYS = frozenset(
    {
        "schema",
        "kind",
        "evidenceScope",
        "mode",
        "api",
        "testId",
        "manifest",
        "manifestSha256",
        "upstream",
        "commit",
        "toolingRoot",
        "toolingSources",
        "pythonExecutable",
        "pythonVersion",
        "wheelPath",
        "wheelSha256",
        "playwrightPackage",
        "playwrightPackageSha256",
        "pwrightPackage",
        "pwrightPackageSha256",
        "nativePath",
        "nativeSha256",
        "runtimeLibraries",
        "driverNode",
        "driverNodeSha256",
        "systemNode",
        "systemNodeSha256",
        "launchMode",
        "endpoint",
        "collectOnly",
        "bootstrapOnly",
        "fixtureBind",
        "fixtureHost",
        "environment",
        "sandboxProfile",
        "sandboxProfileSha256",
        "sandboxAllowedExecutables",
        "sandboxProbe",
        "command",
        "runNonce",
        "exitCode",
        "result",
        "resultSha256",
        "publicReceipt",
        "publicReceiptSha256",
        "processReceipt",
        "processReceiptSha256",
        "verifierSha256",
    }
)


class StrictEvidenceError(ValueError):
    """The guarded official result is not eligible for strict evidence."""


def _fail(message: str) -> NoReturn:
    raise StrictEvidenceError(message)


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def tree_sha256(root: Path) -> str:
    files = [
        path
        for path in root.rglob("*")
        if path.is_file()
        and "__pycache__" not in path.relative_to(root).parts
        and path.suffix != ".pyc"
    ]
    digest = hashlib.sha256()
    for path in sorted(files, key=lambda item: item.relative_to(root).as_posix()):
        relative = path.relative_to(root).as_posix().encode()
        digest.update(len(relative).to_bytes(8, "big"))
        digest.update(relative)
        size = path.stat().st_size
        digest.update(size.to_bytes(8, "big"))
        with path.open("rb") as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(chunk)
    return digest.hexdigest()


def pwright_runtime_library_paths(package: Path) -> list[Path]:
    directory = package.parent / "shuozeli_pwright.dylibs"
    if directory.is_symlink():
        raise ValueError("candidate adjacent dylib directory is a symlink")
    if not directory.exists():
        return []
    if not directory.is_dir():
        raise ValueError("candidate adjacent dylib directory is not a directory")
    paths = sorted(directory.iterdir(), key=lambda path: path.name)
    if any(
        path.is_symlink() or not path.is_file() or path.suffix != ".dylib"
        for path in paths
    ):
        raise ValueError("candidate adjacent dylib set is invalid")
    return paths


def _object(value: object, label: str) -> dict[str, Any]:
    if not isinstance(value, dict) or not all(isinstance(k, str) for k in value):
        _fail(f"{label} must be an object with string keys")
    return cast(dict[str, Any], value)


def _text(value: object, label: str) -> str:
    if not isinstance(value, str) or not value:
        _fail(f"{label} must be a non-empty string")
    return value


def _digest(value: object, label: str) -> str:
    if not isinstance(value, str) or not _SHA.fullmatch(value):
        _fail(f"{label} must be a lowercase SHA-256")
    return value


def _regular(path: Path, label: str) -> Path:
    if path.is_symlink() or not path.is_file():
        _fail(f"{label} must be a regular non-symlink file: {path}")
    return path


def _file(path: Path, digest: object, label: str) -> None:
    _regular(path, label)
    if file_sha256(path) != _digest(digest, label):
        _fail(f"{label} SHA-256 differs: {path}")


def _tree(path: Path, digest: object, label: str) -> None:
    if path.is_symlink() or not path.is_dir():
        _fail(f"{label} must be a non-symlink directory: {path}")
    if any(entry.is_symlink() for entry in path.rglob("*")):
        _fail(f"{label} contains a symlink: {path}")
    if tree_sha256(path) != _digest(digest, label):
        _fail(f"{label} tree SHA-256 differs: {path}")


def _policy(raw: object) -> dict[str, Any]:
    policy = _object(raw, "strict policy")
    expected = {
        "schema",
        "kind",
        "evidenceScope",
        "repository",
        "tag",
        "commit",
        "executionPolicy",
        "browser",
        "launchMode",
        "requiresBrowser",
        "requiresFixtureServers",
        "tests",
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
        "captureReady",
        "captureSourceSha256",
        "captureSourcePaths",
    }
    if set(policy) != expected:
        _fail("strict policy fields differ")
    if (
        policy["schema"] != 3
        or policy["kind"] != KIND
        or policy["evidenceScope"] != SCOPE
        or policy["repository"] != "https://github.com/microsoft/playwright-python.git"
        or policy["tag"] != "v1.58.0"
        or policy["commit"] != COMMIT
        or policy["executionPolicy"] != "macos-sandbox-dummy-node-strict-v1"
        or policy["browser"] != "chromium"
        or policy["launchMode"] != "owned"
        or policy["requiresBrowser"] is not False
        or policy["requiresFixtureServers"] is not True
    ):
        _fail("strict policy is not the pinned browser-free owned launch policy")
    tests = _object(policy["tests"], "tests")
    if set(tests) != {"async", "sync"} or any(
        tests[api] != f"tests/{api}/test_launcher.py::{TEST_NAME}[chromium]"
        for api in ("async", "sync")
    ):
        _fail("strict policy must select the exact two Chromium IDs")
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
        _fail("strict policy upstream source set differs")
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
    for label, digest in sources.items():
        _digest(digest, label)
    for label, digest in _object(
        policy["candidateRuntimeLibraries"], "candidateRuntimeLibraries"
    ).items():
        _digest(digest, label)
    support = _object(policy["supportTrees"], "supportTrees")
    if set(support) != {
        "packages/pwright-python/tests/official/shim",
        "packages/pwright-python/tests/official/runtime",
    }:
        _fail("strict support tree set differs")
    for label, digest in support.items():
        _digest(digest, label)
    paths = policy["captureSourcePaths"]
    hashes = _object(policy["captureSourceSha256"], "captureSourceSha256")
    if (
        not isinstance(paths, list)
        or len(paths) != 3
        or not all(isinstance(path, str) for path in paths)
    ):
        _fail("strict policy must name exactly three capture sources")
    if len(set(paths)) != 3:
        _fail("strict policy capture source paths repeat")
    if not all(
        isinstance(path, str) and path and not Path(path).is_absolute()
        for path in paths
    ):
        _fail("strict policy capture source paths are invalid")
    if policy["captureReady"] is not True or set(hashes) != set(paths):
        _fail("strict capture policy is not ready or lacks pinned source hashes")
    for label, digest in hashes.items():
        _digest(digest, label)
    return policy


def _upstream(policy: dict[str, Any], upstream: Path, *, check_checkout: bool) -> Path:
    if upstream.is_symlink() or not upstream.is_dir():
        _fail("pinned upstream must be a non-symlink directory")
    if check_checkout:
        try:
            head = subprocess.run(
                ["git", "-C", str(upstream), "rev-parse", "HEAD"],
                check=True,
                capture_output=True,
                text=True,
            ).stdout.strip()
            dirty = subprocess.run(
                ["git", "-C", str(upstream), "status", "--porcelain"],
                check=True,
                capture_output=True,
                text=True,
            ).stdout.strip()
        except subprocess.CalledProcessError as error:
            raise StrictEvidenceError("pinned upstream Git check failed") from error
        if head != COMMIT or dirty:
            _fail("pinned upstream commit or clean status differs")
    for relative, digest in _object(
        policy["upstreamSources"], "upstreamSources"
    ).items():
        path = upstream / relative
        if not path.resolve().is_relative_to(upstream.resolve()):
            _fail(f"upstream source escapes checkout: {relative}")
        _file(path, digest, f"upstream source {relative}")
    asset = upstream / ASSET
    if (
        not asset.stat().st_mode & 0o111
        or asset.read_bytes() != b"#!/usr/bin/env node\n\nprocess.exit(1);\n"
    ):
        _fail("dummy executable does not contain the inert pinned program")
    return asset


def _tooling(policy: dict[str, Any], payload: dict[str, Any], root: Path) -> Path:
    if root.is_symlink() or not root.is_dir():
        _fail("tooling root must be a non-symlink directory")
    sources = _object(payload["toolingSources"], "toolingSources")
    expected = _object(policy["captureSourceSha256"], "captureSourceSha256")
    if sources != expected:
        _fail("capture source hashes differ from the strict policy")
    for relative, digest in sources.items():
        path = root / relative
        if not path.resolve().is_relative_to(root.resolve()):
            _fail(f"capture source escapes tooling root: {relative}")
        _file(path, digest, f"capture source {relative}")
    for relative, digest in _object(policy["supportTrees"], "supportTrees").items():
        path = root / relative
        if not path.resolve().is_relative_to(root.resolve()):
            _fail(f"support tree escapes tooling root: {relative}")
        _tree(path, digest, f"support tree {relative}")
    return root / "packages/pwright-python/tests/official/dummy_guard_strict"


def _runtime(policy: dict[str, Any], payload: dict[str, Any]) -> None:
    python = Path(_text(payload["pythonExecutable"], "pythonExecutable"))
    if not python.is_absolute() or not python.is_symlink() or not python.is_file():
        _fail("Python interpreter is missing")
    _file(python.resolve(), policy["pythonRuntimeSha256"], "pinned Python runtime")
    if payload["pythonVersion"] != "3.12.14":
        _fail("strict ABI119 run requires CPython 3.12.14")
    site = python.parent.parent / "lib/python3.12/site-packages"
    if Path(payload["playwrightPackage"]) != site / "playwright":
        _fail("official helper package is not installed in selected interpreter")
    _tree(
        site / "playwright",
        policy["oraclePackageTreeSha256"],
        "official Playwright package",
    )
    if payload["playwrightPackageSha256"] != policy["oraclePackageTreeSha256"]:
        _fail("recorded official package digest differs")
    driver = site / "playwright/driver/node"
    if Path(payload["driverNode"]) != driver:
        _fail("official driver Node path differs")
    _file(driver, policy["oracleDriverNodeSha256"], "official driver Node")
    if payload["driverNodeSha256"] != policy["oracleDriverNodeSha256"]:
        _fail("recorded official driver Node digest differs")
    wheel = Path(_text(payload["wheelPath"], "wheelPath"))
    _file(wheel, policy["candidateWheelSha256"], "ABI119 candidate wheel")
    if payload["wheelSha256"] != policy["candidateWheelSha256"]:
        _fail("recorded wheel digest differs")
    system_node = Path(_text(payload["systemNode"], "systemNode"))
    _file(system_node, policy["systemNodeSha256"], "system Node")
    if payload["systemNodeSha256"] != policy["systemNodeSha256"]:
        _fail("recorded system Node digest differs")
    if payload["mode"] == "candidate":
        package = site / "pwright"
        native = package / "_native.abi3.so"
        if (
            Path(payload["pwrightPackage"]) != package
            or Path(payload["nativePath"]) != native
        ):
            _fail("candidate package/native is not installed in selected interpreter")
        _tree(package, policy["candidatePackageTreeSha256"], "candidate package")
        _file(native, policy["candidateNativeSha256"], "candidate native artifact")
        if (
            payload["pwrightPackageSha256"] != policy["candidatePackageTreeSha256"]
            or payload["nativeSha256"] != policy["candidateNativeSha256"]
        ):
            _fail("recorded candidate package/native digest differs")
        try:
            libraries = pwright_runtime_library_paths(package)
        except ValueError as error:
            raise StrictEvidenceError(
                "candidate adjacent dylibs are invalid"
            ) from error
        expected_libraries = _object(
            policy["candidateRuntimeLibraries"], "candidateRuntimeLibraries"
        )
        if {path.name for path in libraries} != set(expected_libraries):
            _fail("candidate adjacent dylib set differs")
        recorded = _object(payload["runtimeLibraries"], "runtimeLibraries")
        if recorded != expected_libraries:
            _fail("recorded candidate dylib digests differ")
        for path in libraries:
            _file(path, expected_libraries[path.name], f"candidate dylib {path.name}")
    elif (
        any(
            payload[field] is not None
            for field in (
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
        _fail("oracle run must not import or install a Pwright candidate artifact")


def _sandbox(payload: dict[str, Any], guard_dir: Path, asset: Path) -> None:
    python = Path(payload["pythonExecutable"])
    allowed = [
        str(python),
        str(python.resolve()),
        str(asset),
        str(guard_dir / "node"),
        "/usr/bin/env",
        "/usr/bin/python3",
        "/Library/Developer/CommandLineTools/usr/bin/python3",
        "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/bin/python3.9",
        "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/Resources/Python.app/Contents/MacOS/Python",
        payload["systemNode"],
    ]
    if payload["mode"] == "oracle":
        allowed.append(payload["driverNode"])
    if payload["sandboxAllowedExecutables"] != allowed or len(allowed) != len(
        set(allowed)
    ):
        _fail("sandbox executable allowlist differs")
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
        _fail("sandbox profile differs from exact deny-by-default policy")
    if payload["sandboxProfileSha256"] != hashlib.sha256(profile.encode()).hexdigest():
        _fail("sandbox profile digest differs")
    if payload["sandboxProbe"] != {"execDenied": True, "networkDenied": True}:
        _fail("sandbox deny probes were not both observed")


def _fixture(payload: dict[str, Any]) -> None:
    try:
        ip = ipaddress.ip_address(_text(payload["fixtureBind"], "fixtureBind"))
    except ValueError as error:
        raise StrictEvidenceError("fixture bind is not an IP address") from error
    host = _text(payload["fixtureHost"], "fixtureHost")
    if ip not in ipaddress.ip_network("100.64.0.0/10") or not host.endswith(".ts.net"):
        _fail("fixture must use Tailscale IPv4 and full MagicDNS")


def _environment(
    payload: dict[str, Any], tooling_root: Path, guard_dir: Path, upstream: Path
) -> None:
    official = tooling_root / "packages/pwright-python/tests/official"
    python_paths = (
        [official / "shim", upstream, official / "runtime", guard_dir]
        if payload["mode"] == "candidate"
        else [official / "runtime", guard_dir]
    )
    expected = {
        "PATH": f"{guard_dir}:/opt/homebrew/bin:/usr/bin:/bin",
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
        "PWRIGHT_PYTHON_DUMMY_LAUNCH_RECEIPT": payload["publicReceipt"],
        "PWRIGHT_PYTHON_DUMMY_PROCESS_RECEIPT": payload["processReceipt"],
        "PWRIGHT_PYTHON_DUMMY_REAL_NODE": payload["systemNode"],
        "PWRIGHT_PYTHON_DUMMY_NONCE": payload["runNonce"],
    }
    if payload["environment"] != expected:
        _fail("recorded scrubbed environment differs or contains an override")


def _command(payload: dict[str, Any], upstream: Path) -> None:
    api = payload["api"]
    selected = upstream / f"tests/{api}/test_launcher.py"
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
        "pwright_official_dummy_guard",
        "--browser=chromium",
        "--timeout=20",
        "--timeout-method=signal",
        f"{selected}::{TEST_NAME}",
        f"--junitxml={payload['result']}",
    ]
    if payload["command"] != expected:
        _fail("pytest command differs from the exact guarded official selection")


def _report(payload: dict[str, Any]) -> None:
    result = Path(_text(payload["result"], "result"))
    _file(result, payload["resultSha256"], "official JUnit")
    try:
        root = ET.fromstring(result.read_bytes())
    except ET.ParseError as error:
        raise StrictEvidenceError("official JUnit is malformed") from error
    suites = root.findall("testsuite") if root.tag == "testsuites" else []
    if len(suites) != 1 or len(list(root.iter("testcase"))) != 1:
        _fail("official JUnit must contain exactly one suite and one case")
    suite = suites[0]
    cases = suite.findall("testcase")
    if len(cases) != 1:
        _fail("official JUnit contains nested or extra cases")
    case = cases[0]
    if (
        case.get("classname") != f"tests.{payload['api']}.test_launcher"
        or case.get("name") != f"{TEST_NAME}[chromium]"
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
        _fail("official JUnit is not one executed passing Chromium case")


def _receipt(path: Path, digest: object, label: str) -> list[dict[str, Any]]:
    _file(path, digest, label)
    data = path.read_text()
    if not data.endswith("\n") or not data.strip():
        _fail(f"{label} must be newline-terminated JSONL")
    try:
        return [_object(json.loads(line), label) for line in data.splitlines()]
    except json.JSONDecodeError as error:
        raise StrictEvidenceError(f"{label} is malformed JSONL") from error


def _receipts(payload: dict[str, Any], policy: dict[str, Any], asset: Path) -> None:
    process_path = Path(_text(payload["processReceipt"], "processReceipt"))
    public_path = Path(_text(payload["publicReceipt"], "publicReceipt"))
    result = Path(payload["result"])
    if (
        not result.is_absolute()
        or process_path != result.with_suffix(".dummy-process.jsonl")
        or public_path != result.with_suffix(".public-launch.jsonl")
    ):
        _fail("strict process/public receipts must be absolute JUnit siblings")
    process_rows = _receipt(
        process_path, payload["processReceiptSha256"], "process receipt"
    )
    public_rows = _receipt(
        public_path, payload["publicReceiptSha256"], "public receipt"
    )
    common = {
        "api": payload["api"],
        "mode": payload["mode"],
        "runNonce": payload["runNonce"],
        "executablePath": str(asset),
        "assetSha256": policy["upstreamSources"][ASSET],
    }
    if (
        len(process_rows) != 1
        or process_rows[0].get("event") != "dummy-node-exited"
        or {
            key: value
            for key, value in process_rows[0].items()
            if key
            not in {"event", "exitCode", "childPid", "node", "nodeSha256", "argv"}
        }
        != common
        or process_rows[0].get("exitCode") != 1
        or type(process_rows[0].get("childPid")) is not int
        or process_rows[0]["childPid"] <= 0
        or process_rows[0].get("node") != payload["systemNode"]
        or process_rows[0].get("nodeSha256") != policy["systemNodeSha256"]
        or not isinstance(process_rows[0].get("argv"), list)
        or process_rows[0]["argv"][:2] != [payload["systemNode"], str(asset)]
        or not all(
            isinstance(argument, str) and argument
            for argument in process_rows[0]["argv"]
        )
    ):
        _fail("dummy process receipt does not prove one pinned exit-code-1 subprocess")
    if public_rows != [
        {"event": "public-launch-invoked", **common},
        {
            "event": "public-launch-failed",
            **common,
            "errorClass": (
                "Error" if payload["mode"] == "candidate" else "TargetClosedError"
            ),
            "errorMessage": public_rows[1].get("errorMessage")
            if len(public_rows) > 1
            else None,
            "processReceiptSha256": payload["processReceiptSha256"],
        },
    ]:
        _fail(
            "public launch receipts differ from exact invoke/process/failure sequence"
        )
    message = public_rows[1]["errorMessage"]
    if not isinstance(message, str) or not message or "PWRIGHT_UNSUPPORTED" in message:
        _fail("public launch did not report a concrete process failure")


def verify_evidence(
    raw: object,
    *,
    policy: dict[str, Any],
    policy_path: Path,
    sidecar_path: Path,
    tooling_root: Path = ROOT,
    check_checkout: bool = True,
) -> dict[str, Any]:
    """Verify one run's exact source, artifact, sandbox and process proof."""
    policy = _policy(policy)
    payload = _object(raw, "strict provenance")
    if set(payload) != _SIDECAR_KEYS:
        _fail(
            f"strict provenance fields differ: missing={sorted(_SIDECAR_KEYS - set(payload))}, extra={sorted(set(payload) - _SIDECAR_KEYS)}"
        )
    if (
        payload["schema"] != 2
        or payload["kind"] != KIND
        or payload["evidenceScope"] != SCOPE
        or payload["mode"] not in {"candidate", "oracle"}
        or payload["api"] not in {"async", "sync"}
        or payload["testId"] != policy["tests"][payload["api"]]
        or payload["commit"] != COMMIT
        or payload["launchMode"] != "owned"
        or payload["endpoint"] is not None
        or payload["collectOnly"] is not False
        or payload["bootstrapOnly"] is not False
    ):
        _fail("strict provenance scope, lane or exact test differs")
    if not isinstance(payload["runNonce"], str) or not _NONCE.fullmatch(
        payload["runNonce"]
    ):
        _fail("strict run nonce is invalid")
    if Path(payload["manifest"]) != policy_path or payload[
        "manifestSha256"
    ] != file_sha256(policy_path):
        _fail("strict manifest identity differs")
    if Path(payload["toolingRoot"]) != tooling_root:
        _fail("strict tooling root differs")
    if sidecar_path != Path(payload["result"]).with_suffix(".provenance.json"):
        _fail("strict sidecar must be the JUnit sibling")
    upstream = Path(_text(payload["upstream"], "upstream"))
    asset = _upstream(policy, upstream, check_checkout=check_checkout)
    guard_dir = _tooling(policy, payload, tooling_root)
    _file(Path(__file__), payload["verifierSha256"], "strict verifier source")
    _runtime(policy, payload)
    _fixture(payload)
    _sandbox(payload, guard_dir, asset)
    _environment(payload, tooling_root, guard_dir, upstream)
    _command(payload, upstream)
    _report(payload)
    _receipts(payload, policy, asset)
    return payload


def verify_provenance(path: Path) -> dict[str, Any]:
    """Production entrypoint: only the immutable, capture-ready policy is trusted."""
    _file(POLICY, POLICY_SHA256, "strict policy")
    _regular(path, "strict provenance sidecar")
    try:
        policy = json.loads(POLICY.read_text())
        payload = json.loads(path.read_text())
    except json.JSONDecodeError as error:
        raise StrictEvidenceError(
            "strict policy or provenance is malformed JSON"
        ) from error
    return verify_evidence(
        payload,
        policy=policy,
        policy_path=POLICY,
        sidecar_path=path,
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("provenance", type=Path)
    args = parser.parse_args()
    try:
        payload = verify_provenance(args.provenance)
    except (OSError, StrictEvidenceError) as error:
        parser.error(str(error))
    print(
        f"verified guarded dummy-node {payload['mode']}/{payload['api']} "
        f"resultSha256={payload['resultSha256']}"
    )


if __name__ == "__main__":
    main()
