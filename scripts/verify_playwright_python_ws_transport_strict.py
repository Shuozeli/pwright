#!/usr/bin/env python3
"""Verify exact official Python CDP WebSocket tests with disclosed URL remapping."""

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
from typing import Any, NoReturn, cast

ROOT = Path(__file__).resolve().parents[1]
OFFICIAL = ROOT / "packages/pwright-python/tests/official"
POLICY = OFFICIAL / "async-cdp-websocket-transport-strict-v1-manifest.json"
POLICY_SHA256 = "029a4a0243ff2eab2d6a3bb40bd16809748bee85786ab13bafa91b8855c9bd1f"
COMMIT = "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
KIND = "playwright-python-cdp-websocket-transport-strict-v1"
SOURCE = "tests/async/test_browsertype_connect_cdp.py"
TEST_NAMES = (
    "test_connect_over_cdp_passing_header_works",
    "test_should_print_custom_ws_close_error",
)
TEST_IDS = tuple(f"{SOURCE}::{name}[chromium]" for name in TEST_NAMES)
GUARD = OFFICIAL / "ws_guard_strict"
_SHA = re.compile(r"[0-9a-f]{64}\Z")
_NONCE = re.compile(r"[0-9a-f]{32}\Z")
_PAYLOAD_KEYS = frozenset(
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


class StrictWebSocketEvidenceError(ValueError):
    """The capture is not strict official evidence."""


def _fail(message: str) -> NoReturn:
    raise StrictWebSocketEvidenceError(message)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def sha256_tree(root: Path) -> str:
    if (
        root.is_symlink()
        or not root.is_dir()
        or any(p.is_symlink() for p in root.rglob("*"))
    ):
        _fail(f"tree contains a symlink or is missing: {root}")
    digest = hashlib.sha256()
    files = sorted(
        (
            p
            for p in root.rglob("*")
            if p.is_file() and "__pycache__" not in p.parts and p.suffix != ".pyc"
        ),
        key=lambda p: p.relative_to(root).as_posix(),
    )
    for path in files:
        relative = path.relative_to(root).as_posix().encode()
        digest.update(len(relative).to_bytes(8, "big"))
        digest.update(relative)
        digest.update(path.stat().st_size.to_bytes(8, "big"))
        with path.open("rb") as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(chunk)
    return digest.hexdigest()


def _object(value: object, label: str) -> dict[str, Any]:
    if not isinstance(value, dict) or not all(isinstance(key, str) for key in value):
        _fail(f"{label} must be an object with string keys")
    return cast(dict[str, Any], value)


def _string(value: object, label: str) -> str:
    if not isinstance(value, str) or not value:
        _fail(f"{label} must be non-empty text")
    return value


def _digest(value: object, label: str) -> str:
    if not isinstance(value, str) or not _SHA.fullmatch(value):
        _fail(f"{label} must be a lowercase SHA-256")
    return value


def _regular(path: Path, label: str) -> Path:
    if path.is_symlink() or not path.is_file():
        _fail(f"{label} is not a regular file: {path}")
    return path


def _file(path: Path, digest: object, label: str) -> None:
    if sha256_file(_regular(path, label)) != _digest(digest, label):
        _fail(f"{label} SHA-256 differs: {path}")


def _policy(raw: object) -> dict[str, Any]:
    policy = _object(raw, "WebSocket policy")
    if set(policy) != {
        "schema",
        "kind",
        "repository",
        "tag",
        "commit",
        "browser",
        "requiresBrowser",
        "requiresFixtureServers",
        "fixtureTransportAdaptation",
        "tests",
        "upstreamSources",
        "supportSources",
        "pythonRuntimeSha256",
        "oraclePackageTreeSha256",
        "oracleDriverNodeSha256",
        "captureReady",
        "captureSourceSha256",
    }:
        _fail("WebSocket policy fields differ")
    if (
        policy["schema"] != 1
        or policy["kind"] != KIND
        or policy["repository"] != "https://github.com/microsoft/playwright-python.git"
        or policy["tag"] != "v1.58.0"
        or policy["commit"] != COMMIT
        or policy["browser"] != "chromium"
        or policy["requiresBrowser"] is not False
        or policy["requiresFixtureServers"] is not True
        or policy["fixtureTransportAdaptation"]
        != "exact-127.0.0.1-ws-to-tailscale-magicdns-only"
        or policy["tests"] != list(TEST_IDS)
    ):
        _fail("WebSocket policy is not the exact two-case Chromium policy")
    upstream = _object(policy["upstreamSources"], "upstreamSources")
    if set(upstream) != {
        SOURCE,
        "tests/conftest.py",
        "tests/async/conftest.py",
        "tests/server.py",
        "tests/async/utils.py",
        "pyproject.toml",
    }:
        _fail("WebSocket upstream source set differs")
    support = _object(policy["supportSources"], "supportSources")
    if set(support) != {
        "packages/pwright-python/tests/official/shim/playwright/__init__.py",
        "packages/pwright-python/tests/official/shim/playwright/async_api.py",
        "packages/pwright-python/tests/official/shim/playwright/_public_types.py",
        "packages/pwright-python/tests/official/shim/pwright_candidate_runtime.py",
        "packages/pwright-python/tests/official/runtime/pwright_official_server_runtime.py",
        "packages/pwright-python/tests/official/runtime/pwright_official_oracle_runtime.py",
    }:
        _fail("WebSocket support source set differs")
    captures = _object(policy["captureSourceSha256"], "captureSourceSha256")
    if policy["captureReady"] is not True or set(captures) != {
        "scripts/run_playwright_python_ws_transport_strict.py",
        "packages/pwright-python/tests/official/ws_guard_strict/pwright_official_ws_guard.py",
    }:
        _fail("WebSocket capture sources are not frozen")
    for label, digest in {**upstream, **support, **captures}.items():
        _digest(digest, label)
    for label in (
        "pythonRuntimeSha256",
        "oraclePackageTreeSha256",
        "oracleDriverNodeSha256",
    ):
        _digest(policy[label], label)
    return policy


def _sources(root: Path, sources: dict[str, Any], label: str) -> None:
    if root.is_symlink() or not root.is_dir():
        _fail(f"{label} root is a symlink or missing")
    for relative, digest in sources.items():
        path = root / relative
        if path.is_symlink() or not path.resolve().is_relative_to(root.resolve()):
            _fail(f"{label} source escapes root: {relative}")
        _file(path, digest, f"{label} {relative}")


def _upstream(policy: dict[str, Any], path: Path) -> None:
    if not path.is_absolute() or path.is_symlink() or not path.is_dir():
        _fail("upstream checkout must be an absolute, non-symlink directory")
    try:
        head = subprocess.run(
            ["git", "-C", str(path), "rev-parse", "HEAD"],
            check=True,
            capture_output=True,
            text=True,
            timeout=10,
        ).stdout.strip()
        dirty = subprocess.run(
            ["git", "-C", str(path), "status", "--porcelain"],
            check=True,
            capture_output=True,
            text=True,
            timeout=10,
        ).stdout.strip()
    except subprocess.CalledProcessError as error:
        raise StrictWebSocketEvidenceError("upstream Git check failed") from error
    if head != COMMIT or dirty:
        _fail("upstream checkout commit or clean status differs")
    _sources(path, _object(policy["upstreamSources"], "upstreamSources"), "upstream")


def _wheel_installed(wheel: Path, site: Path) -> None:
    roots = ("pwright/", "shuozeli_pwright.dylibs/")
    with zipfile.ZipFile(wheel) as archive:
        names = [
            name
            for name in archive.namelist()
            if name.startswith(roots) and not name.endswith("/")
        ]
        if len(names) != len(set(names)) or not any(
            name == "pwright/_native.abi3.so" for name in names
        ):
            _fail("candidate wheel has duplicate or missing package entries")
        installed = {
            path.relative_to(site).as_posix()
            for root in roots
            for path in (site / root).rglob("*")
            if path.is_file()
            and "__pycache__" not in path.parts
            and path.suffix != ".pyc"
        }
        if installed != set(names):
            _fail("installed candidate package differs from wheel entry set")
        for name in names:
            path = _regular(site / name, f"installed wheel entry {name}")
            if path.read_bytes() != archive.read(name):
                _fail(f"installed candidate file differs from wheel: {name}")


def _runtime(policy: dict[str, Any], payload: dict[str, Any]) -> None:
    python = Path(_string(payload["pythonExecutable"], "pythonExecutable"))
    if not python.is_absolute() or not python.is_symlink() or not python.is_file():
        _fail("selected Python must be a venv interpreter symlink")
    runtime = python.resolve()
    _file(runtime, policy["pythonRuntimeSha256"], "Python runtime")
    if (
        payload["pythonRuntimeSha256"] != policy["pythonRuntimeSha256"]
        or payload["pythonVersion"] != "3.12.14"
    ):
        _fail("selected Python runtime/version differs")
    site = python.parent.parent / "lib/python3.12/site-packages"
    official = site / "playwright"
    if Path(payload["playwrightPackage"]) != official:
        _fail("official Playwright package path differs")
    if (
        sha256_tree(official) != policy["oraclePackageTreeSha256"]
        or payload["playwrightPackageSha256"] != policy["oraclePackageTreeSha256"]
    ):
        _fail("official Playwright package differs")
    driver = official / "driver/node"
    if Path(payload["driverNode"]) != driver:
        _fail("official driver Node path differs")
    _file(driver, policy["oracleDriverNodeSha256"], "official driver Node")
    if payload["driverNodeSha256"] != policy["oracleDriverNodeSha256"]:
        _fail("official driver Node receipt differs")
    wheel = Path(_string(payload["wheelPath"], "wheelPath"))
    if not wheel.is_absolute():
        _fail("candidate wheel path must be absolute")
    _file(wheel, payload["wheelSha256"], "candidate wheel")
    if payload["mode"] == "candidate":
        package = site / "pwright"
        native = package / "_native.abi3.so"
        if (
            Path(payload["pwrightPackage"]) != package
            or Path(payload["nativePath"]) != native
        ):
            _fail("candidate installed package/native path differs")
        if sha256_tree(package) != payload["pwrightPackageSha256"]:
            _fail("candidate package digest differs")
        _file(native, payload["nativeSha256"], "candidate native artifact")
        dylib_dir = site / "shuozeli_pwright.dylibs"
        libraries = _object(payload["runtimeLibraries"], "runtimeLibraries")
        actual = (
            {
                path.name: sha256_file(path)
                for path in dylib_dir.iterdir()
                if path.is_file()
            }
            if dylib_dir.is_dir()
            else {}
        )
        if actual != libraries or any(
            path.is_symlink() for path in dylib_dir.iterdir()
        ):
            _fail("candidate adjacent runtime libraries differ")
        _wheel_installed(wheel, site)
    elif payload["mode"] == "oracle":
        if (
            any(
                payload[key] is not None
                for key in (
                    "pwrightPackage",
                    "pwrightPackageSha256",
                    "nativePath",
                    "nativeSha256",
                )
            )
            or payload["runtimeLibraries"] != {}
            or (site / "pwright").exists()
        ):
            _fail("oracle interpreter contains candidate artifact")
    else:
        _fail("mode must be candidate or oracle")


def _fixture(payload: dict[str, Any]) -> None:
    port = payload["fixturePort"]
    if not isinstance(port, int) or isinstance(port, bool) or not 1024 <= port < 65535:
        _fail("fixture port is invalid")
    try:
        ip = ipaddress.ip_address(_string(payload["fixtureBind"], "fixtureBind"))
    except ValueError as error:
        raise StrictWebSocketEvidenceError(
            "fixture bind is not an IP address"
        ) from error
    host = _string(payload["fixtureHost"], "fixtureHost")
    if ip not in ipaddress.ip_network("100.64.0.0/10") or not re.fullmatch(
        r"[a-z0-9-]+(?:\.[a-z0-9-]+)*\.ts\.net", host
    ):
        _fail("fixture must use Tailscale IPv4 and full MagicDNS")
    if (
        payload["fixtureTransportAdaptation"]
        != "exact-127.0.0.1-ws-to-tailscale-magicdns-only"
    ):
        _fail("transport adaptation disclosure differs")


def sandbox_profile(
    python: Path, driver: Path, mode: str, port: int
) -> tuple[str, list[str]]:
    allowed = [str(python), str(python.resolve())]
    if mode == "oracle":
        allowed.append(str(driver))
    profile = "\n".join(
        [
            "(version 1)",
            "(allow default)",
            "(deny process-exec)",
            "(deny network-outbound)",
            f'(allow network-outbound (remote ip "localhost:{port}"))',
            '(allow network-outbound (literal "/private/var/run/mDNSResponder"))',
        ]
        + [f"(allow process-exec (literal {json.dumps(path)}))" for path in allowed]
    )
    return profile, allowed


def _environment(payload: dict[str, Any]) -> None:
    mode = payload["mode"]
    python_paths = (
        [OFFICIAL / "shim", Path(payload["upstream"]), OFFICIAL / "runtime", GUARD]
        if mode == "candidate"
        else [OFFICIAL / "runtime", GUARD]
    )
    expected = {
        "PATH": "/opt/homebrew/bin:/usr/bin:/bin",
        "PYTHONPATH": ":".join(map(str, python_paths)),
        "PYTHONNOUSERSITE": "1",
        "PYTEST_DISABLE_PLUGIN_AUTOLOAD": "1",
        "PWRIGHT_OFFICIAL_MODE": mode,
        "PWRIGHT_OFFICIAL_LAUNCH_MODE": "owned",
        "PWRIGHT_OFFICIAL_BROWSER_CHANNEL": "",
        "PWRIGHT_OFFICIAL_UPSTREAM_TESTS": str(Path(payload["upstream"]) / "tests"),
        "PWRIGHT_OFFICIAL_TAILSCALE_IP": payload["fixtureBind"],
        "PWRIGHT_OFFICIAL_TAILSCALE_HOST": payload["fixtureHost"],
        "PWRIGHT_PYTHON_WS_EXPECTED_TEST": payload["testId"],
        "PWRIGHT_PYTHON_WS_UPSTREAM": payload["upstream"],
        "PWRIGHT_PYTHON_WS_PORT": str(payload["fixturePort"]),
        "PWRIGHT_PYTHON_WS_RECEIPT": payload["callReceipt"],
        "PWRIGHT_PYTHON_WS_NONCE": payload["runNonce"],
    }
    if payload["environment"] != expected:
        _fail("strict WebSocket environment contains an override or missing field")


def _command(payload: dict[str, Any]) -> None:
    test_id = payload["testId"]
    name = test_id.split("::", 1)[1].removesuffix("[chromium]")
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
        "pwright_official_ws_guard",
        "--browser=chromium",
        "--timeout=20",
        "--timeout-method=signal",
        f"{Path(payload['upstream']) / SOURCE}::{name}",
        f"--junitxml={payload['result']}",
    ]
    if payload["command"] != expected:
        _fail("strict WebSocket command differs from pinned official selection")


def report_outcome(payload: dict[str, Any]) -> str:
    result = Path(_string(payload["result"], "result"))
    if not result.is_absolute() or result.with_suffix(".call.jsonl") != Path(
        payload["callReceipt"]
    ):
        _fail("strict WebSocket result and receipt locations differ")
    _file(result, payload["resultSha256"], "official JUnit")
    try:
        root = ET.fromstring(result.read_bytes())
    except ET.ParseError as error:
        raise StrictWebSocketEvidenceError("official JUnit is malformed") from error
    suites = root.findall("testsuite") if root.tag == "testsuites" else []
    if len(suites) != 1 or len(list(root.iter("testcase"))) != 1:
        _fail("official JUnit must have exactly one suite and one case")
    suite = suites[0]
    cases = suite.findall("testcase")
    name = payload["testId"].split("::", 1)[1]
    if (
        len(cases) != 1
        or cases[0].get("classname") != "tests.async.test_browsertype_connect_cdp"
        or cases[0].get("name") != name
        or suite.get("tests") != "1"
        or suite.get("skipped") != "0"
    ):
        _fail("official JUnit case identity or count differs")
    case = cases[0]
    if case.find("skipped") is not None:
        _fail("official JUnit skipped the selected case")
    if (
        case.find("failure") is not None
        and case.find("error") is None
        and suite.get("failures") == "1"
        and suite.get("errors") == "0"
        and payload["exitCode"] == 1
    ):
        return "failed"
    if (
        case.find("error") is not None
        and case.find("failure") is None
        and suite.get("errors") == "1"
        and suite.get("failures") == "0"
        and payload["exitCode"] == 1
    ):
        return "error"
    if (
        case.find("failure") is None
        and case.find("error") is None
        and suite.get("errors") == "0"
        and suite.get("failures") == "0"
        and payload["exitCode"] == 0
    ):
        return "passed"
    _fail("official JUnit and process exit code disagree")


def verify_evidence(policy: dict[str, Any], payload: dict[str, Any]) -> str:
    if (
        set(payload) != _PAYLOAD_KEYS
        or payload["schema"] != 1
        or payload["kind"] != KIND
    ):
        _fail("strict WebSocket evidence schema differs")
    if (
        payload["mode"] not in ("candidate", "oracle")
        or payload["testId"] not in TEST_IDS
    ):
        _fail("strict WebSocket mode or test ID differs")
    if Path(payload["manifest"]) != POLICY or payload["manifestSha256"] != sha256_file(
        POLICY
    ):
        _fail("strict WebSocket manifest identity differs")
    if payload["commit"] != COMMIT or Path(payload["toolingRoot"]) != ROOT:
        _fail("strict WebSocket checkout/tooling identity differs")
    if payload["toolingSources"] != policy["captureSourceSha256"]:
        _fail("strict WebSocket capture source hashes differ")
    _upstream(policy, Path(_string(payload["upstream"], "upstream")))
    _sources(ROOT, _object(policy["supportSources"], "supportSources"), "support")
    _sources(
        ROOT, _object(policy["captureSourceSha256"], "captureSourceSha256"), "capture"
    )
    _runtime(policy, payload)
    _fixture(payload)
    if not isinstance(payload["runNonce"], str) or not _NONCE.fullmatch(
        payload["runNonce"]
    ):
        _fail("strict WebSocket run nonce is invalid")
    python = Path(payload["pythonExecutable"])
    profile, allowed = sandbox_profile(
        python, Path(payload["driverNode"]), payload["mode"], payload["fixturePort"]
    )
    if (
        payload["sandboxProfile"] != profile
        or payload["sandboxAllowedExecutables"] != allowed
        or payload["sandboxProfileSha256"]
        != hashlib.sha256(profile.encode()).hexdigest()
    ):
        _fail("strict WebSocket sandbox differs from port-scoped policy")
    if payload["sandboxProbe"] != {
        "allowedMagicDnsConnection": True,
        "otherPortDenied": True,
        "unlistedExecDenied": True,
    }:
        _fail("strict WebSocket sandbox probes are missing")
    _environment(payload)
    _command(payload)
    receipt = Path(_string(payload["callReceipt"], "callReceipt"))
    _file(receipt, payload["callReceiptSha256"], "public WebSocket call receipt")
    lines = receipt.read_text().splitlines()
    if len(lines) != 1 or not receipt.read_bytes().endswith(b"\n"):
        _fail("strict WebSocket receipt must contain one newline-terminated call")
    try:
        call = _object(json.loads(lines[0]), "public WebSocket call receipt")
    except json.JSONDecodeError as error:
        raise StrictWebSocketEvidenceError(
            "public WebSocket call receipt is malformed"
        ) from error
    if call != {
        "schema": 1,
        "testId": payload["testId"],
        "mode": payload["mode"],
        "runNonce": payload["runNonce"],
        "fixturePort": payload["fixturePort"],
        "originalEndpoint": f"ws://127.0.0.1:{payload['fixturePort']}/ws",
        "forwardedEndpoint": f"ws://{payload['fixtureHost']}:{payload['fixturePort']}/ws",
        "headers": {"foo": "bar"},
    }:
        _fail("strict WebSocket call was not the exact unchanged upstream invocation")
    outcome = report_outcome(payload)
    if payload["outcome"] != outcome:
        _fail("strict WebSocket recorded outcome differs from official JUnit")
    _file(Path(__file__), payload["verifierSha256"], "strict WebSocket verifier")
    return outcome


def verify_provenance(path: Path) -> dict[str, Any]:
    _file(POLICY, POLICY_SHA256, "strict WebSocket manifest")
    policy = _policy(json.loads(POLICY.read_text()))
    sidecar = _regular(path, "strict WebSocket sidecar")
    payload = _object(json.loads(sidecar.read_text()), "strict WebSocket sidecar")
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
        json.JSONDecodeError,
    ) as error:
        parser.error(str(error))
    print(
        f"verified strict WebSocket {payload['mode']}/{payload['testId']}: {payload['outcome']}"
    )


if __name__ == "__main__":
    main()
