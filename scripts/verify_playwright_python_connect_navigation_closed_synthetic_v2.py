#!/usr/bin/env python3
"""Independently verify the GREEN successor for the same pinned three-engine case."""

from __future__ import annotations

import argparse
import hashlib
import ipaddress
import json
import subprocess
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path
from typing import cast

ROOT = Path(__file__).resolve().parents[1]
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-python")
CORE = Path("/Users/cyuan/projects/thirdparty/playwright-v1.58.0")
RESULT_PARENT = ROOT / ".work/official-conformance/results"
PLUGIN_DIR = (
    ROOT
    / "packages/pwright-python/tests/official/connect_navigation_closed_synthetic_v1"
)
MANIFEST = (
    ROOT
    / "packages/pwright-python/tests/official/connect_navigation_closed_synthetic_v2/manifest.json"
)
SERVER = PLUGIN_DIR / "server.cjs"
PLUGIN = PLUGIN_DIR / "pwright_python_connect_navigation_closed_synthetic_v1.py"
RUNTIME = ROOT / "packages/pwright-python/tests/official/runtime"
SHIM = ROOT / "packages/pwright-python/tests/official/shim"
WHEEL_NAME = "shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl"
WHEEL_SHA256 = "32c414d3eb2a45417c4a3ba79705be96179dc2ec2dd20896cd0cda7d8169e3e8"
FIXTURE_HASHES = {
    "server.cjs": "aed2dff558ba8fea0dc177f5409e51c01e90375b69a6f60210cc288c897d2f39",
    "pwright_python_connect_navigation_closed_synthetic_v1.py": (
        "2ba90f2bc4ca5bc862f42e571fce0b1dd6fbd96e9360fd9c1afec6fad175bbfc"
    ),
}
UPSTREAM_COMMIT = "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
CORE_COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
CASE_BODY = (
    "tests/async/test_browsertype_connect.py::"
    "test_browser_type_connect_should_reject_navigation_when_browser_closes"
)
ENGINES = ("chromium", "firefox", "webkit")
FEATURES = (
    "browser-registry",
    "playwright-server-initialize",
    "playwright-server-connect-session",
)
ENDPOINT_PATH = "/browser-synthetic-python-navigation-closed-v1"
SOURCE_HASHES = {
    "tests/async/test_browsertype_connect.py": (
        "d47290f5acb45246ce2bae150d3c583d340806821b660be08d798ac3572e7b11"
    ),
    "tests/conftest.py": "c4236e98ff1731fff1f7ef1c00b37fe3a0cb1e4a3fd07e89a491dc7129ea93cc",
    "tests/async/conftest.py": "3001008bb97ba0cc1ddb397a53df530b6881e3e3ba69a1ed2cac0de94cab4cdb",
    "tests/server.py": "2cf1a6b805f99c1c82c5351b67df995f06f2a9a18d21dbd48d4d4dc6079c3030",
    "pyproject.toml": "96e2d4dcd75b40a101c77d12b4e29ea4d91efc6fd58992a67ed8cc837e2e1e08",
}
TOOLING = {
    "python": "3.12",
    "playwright": "1.58.0",
    "pytest": "8.4.2",
    "pytest-asyncio": "1.2.0",
    "pytest-timeout": "2.4.0",
    "pytest-rerunfailures": "15.1",
    "pytest-repeat": "0.9.4",
    "pillow": "11.3.0",
    "pixelmatch": "0.3.0",
    "twisted": "25.5.0",
    "pyopenssl": "25.1.0",
    "service-identity": "24.2.0",
    "autobahn": "23.1.2",
    "requests": "2.32.5",
}
HARNESS_INPUTS = (
    MANIFEST,
    SERVER,
    PLUGIN,
    RUNTIME / "pwright_official_server_runtime.py",
    RUNTIME / "pwright_official_oracle_runtime.py",
    SHIM / "pwright_candidate_runtime.py",
    SHIM / "playwright/async_api.py",
    ROOT / "scripts/run_playwright_python_connect_navigation_closed_synthetic_v2.py",
    ROOT
    / "scripts/test_verify_playwright_python_connect_navigation_closed_synthetic_v2.py",
    Path(__file__).resolve(),
)


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _json(path: Path) -> dict[str, object]:
    _require(
        path.is_file() and not path.is_symlink(), f"missing regular evidence: {path}"
    )
    value = json.loads(path.read_text())
    _require(isinstance(value, dict), f"evidence is not a JSON object: {path}")
    return cast(dict[str, object], value)


def _git(checkout: Path, *args: str) -> str:
    return subprocess.check_output(
        ["git", "-C", str(checkout), *args], text=True
    ).strip()


def harness_hashes() -> dict[str, str]:
    hashes: dict[str, str] = {}
    for path in HARNESS_INPUTS:
        _require(
            path.is_file() and not path.is_symlink(), f"harness input missing: {path}"
        )
        hashes[str(path.relative_to(ROOT))] = sha256(path)
    return hashes


def verify_sources() -> dict[str, str]:
    _require(
        len(WHEEL_SHA256) == 64
        and all(digit in "0123456789abcdef" for digit in WHEEL_SHA256),
        "successor wheel SHA256 has not been pinned",
    )
    for name, digest in FIXTURE_HASHES.items():
        _require(
            sha256(PLUGIN_DIR / name) == digest, f"frozen v1 fixture changed: {name}"
        )
    _require(
        _git(UPSTREAM, "rev-parse", "HEAD") == UPSTREAM_COMMIT, "Python pin drifted"
    )
    _require(not _git(UPSTREAM, "status", "--porcelain"), "Python upstream is dirty")
    _require(_git(CORE, "rev-parse", "HEAD") == CORE_COMMIT, "Core pin drifted")
    _require(
        not _git(CORE, "status", "--porcelain", "--untracked-files=no"),
        "Core tracked source drifted",
    )
    for relative, digest in SOURCE_HASHES.items():
        _require(
            sha256(UPSTREAM / relative) == digest,
            f"upstream source drifted: {relative}",
        )
    manifest = _json(MANIFEST)
    _require(
        manifest.get("schema") == 1
        and manifest.get("kind")
        == "playwright-python-connect-navigation-closed-synthetic-v2"
        and manifest.get("classification")
        == "official-derived-fixture-adapted-internal-diagnostic"
        and manifest.get("repository")
        == "https://github.com/microsoft/playwright-python.git"
        and manifest.get("tag") == "v1.58.0"
        and manifest.get("commit") == UPSTREAM_COMMIT
        and manifest.get("coreProtocolCommit") == CORE_COMMIT,
        "manifest provenance changed",
    )
    _require(
        manifest.get("requiresBrowser") is False
        and manifest.get("fixtureAdapted") is True
        and manifest.get("adaptedFixture") == "launch_server"
        and manifest.get("testSourceAdapted") is False
        and manifest.get("promotableToOfficialLedger") is False
        and manifest.get("fixtureHarnessVersion")
        == "connect_navigation_closed_synthetic_v1"
        and manifest.get("fixtureSourceSha256") == FIXTURE_HASHES,
        "manifest classification changed",
    )
    _require(
        manifest.get("candidateAbi") == 125
        and manifest.get("candidateSelectedFeatures") == list(FEATURES)
        and manifest.get("candidateWheelSha256") == WHEEL_SHA256
        and manifest.get("testBodies") == [CASE_BODY]
        and manifest.get("engines") == list(ENGINES)
        and manifest.get("expandedCaseIds")
        == [f"{CASE_BODY}[{engine}]" for engine in ENGINES]
        and manifest.get("upstreamSourceSha256") == SOURCE_HASHES
        and manifest.get("toolingVersions") == TOOLING,
        "manifest selection or candidate identity changed",
    )
    hashes = harness_hashes()
    ws = subprocess.check_output(
        ["node", "-p", f"require.resolve('ws', {{paths: [{str(CORE)!r}]}})"], text=True
    ).strip()
    ws_path = Path(ws).resolve()
    _require(ws_path.is_file() and ws_path.is_relative_to(CORE), "pinned Core ws moved")
    hashes[str(ws_path)] = sha256(ws_path)
    return hashes


def _installed_wheel(wheel: Path, site_packages: Path) -> dict[str, str]:
    _require(sha256(wheel) == WHEEL_SHA256, "frozen wheel hash changed")
    installed: dict[str, str] = {}
    with zipfile.ZipFile(wheel) as archive:
        for name in archive.namelist():
            if not (
                name.startswith("pwright/") or name.startswith("pwright.libs/")
            ) or name.endswith("/"):
                continue
            target = site_packages / name
            _require(
                target.is_file() and not target.is_symlink(),
                f"wheel file missing: {name}",
            )
            digest = hashlib.sha256(archive.read(name)).hexdigest()
            _require(sha256(target) == digest, f"wheel install differs: {name}")
            installed[name] = digest
    _require(
        "pwright/_native.abi3.so" in installed and "pwright/async_api.py" in installed,
        "selected wheel lacks the ABI125 async remote binding",
    )
    return installed


def _junit(path: Path, mode: str, engine: str) -> str:
    _require(mode in ("oracle", "candidate"), "unknown diagnostic lane")
    _require(path.is_file() and not path.is_symlink(), f"JUnit missing: {path}")
    root = ET.parse(path).getroot()
    suite = root.find("testsuite") if root.tag == "testsuites" else root
    if suite is None or suite.tag != "testsuite":
        raise ValueError("invalid JUnit suite")
    cases = list(suite.findall("testcase"))
    _require(len(cases) == 1, "JUnit did not contain exactly one case")
    case = cases[0]
    _require(
        case.attrib.get("name") == CASE_BODY.split("::", 1)[1] + f"[{engine}]"
        and "async" in case.attrib.get("classname", ""),
        "JUnit case identity changed",
    )
    _require(
        case.find("skipped") is None
        and case.find("error") is None
        and int(suite.attrib.get("tests", "-1")) == 1
        and int(suite.attrib.get("errors", "-1")) == 0
        and int(suite.attrib.get("skipped", "-1")) == 0,
        "JUnit has a skip, setup error, or extra case",
    )
    _require(
        case.find("failure") is None and int(suite.attrib.get("failures", "-1")) == 0,
        f"{mode} did not pass the unchanged official test body",
    )
    return "passed"


def _receipt(
    path: Path, mode: str, engine: str, bind: str, host: str, site: Path
) -> None:
    _require(path.is_file() and not path.is_symlink(), f"receipt missing: {path}")
    records: list[dict[str, object]] = []
    for line in path.read_text().splitlines():
        raw = json.loads(line)
        _require(isinstance(raw, dict), "receipt record is not a JSON object")
        records.append(cast(dict[str, object], raw))
    _require(bool(records), "receipt is empty")
    _require(
        all(
            record.get("mode") == mode
            and record.get("engine") == engine
            and record.get("fixtureBind") == bind
            and record.get("fixtureHost") == host
            for record in records
        ),
        "receipt lane, engine, or Tailscale address changed",
    )

    def one(kind: str) -> dict[str, object]:
        matches = [record for record in records if record.get("kind") == kind]
        _require(len(matches) == 1, f"receipt {kind} count changed")
        return matches[0]

    runtime = one("runtime-import")
    official_path = str((site / "playwright/async_api/__init__.py").resolve())
    candidate_path = str((site / "pwright/__init__.py").resolve())
    _require(
        runtime.get("publicApiPath")
        == (
            str((SHIM / "playwright/async_api.py").resolve())
            if mode == "candidate"
            else official_path
        )
        and runtime.get("candidatePackagePath")
        == (candidate_path if mode == "candidate" else None),
        "oracle/candidate runtime import was not independent",
    )
    adapted = one("fixture-adapted")
    _require(
        adapted.get("fixture") == "launch_server"
        and adapted.get("testSourceAdapted") is False
        and adapted.get("browserProcessLaunched") is False
        and adapted.get("upstreamCommit") == UPSTREAM_COMMIT,
        "launch_server fixture substitution absent",
    )
    expected_case = f"{CASE_BODY}[{engine}]"
    _require(one("selected").get("caseId") == expected_case, "test selection changed")
    case_call = one("case-call")
    _require(
        str(case_call.get("caseId", "")).endswith(expected_case)
        and case_call.get("outcome") == "passed",
        "test-body call outcome changed",
    )
    created = one("fixture-created")
    _require(
        created.get("endpointPath") == ENDPOINT_PATH
        and created.get("fixtureAdapted") is True
        and created.get("testSourceAdapted") is False
        and created.get("browserProcessLaunched") is False,
        "synthetic launch fixture identity changed",
    )
    pid = created.get("serverPid")
    _require(isinstance(pid, int) and pid > 0, "synthetic server PID missing")
    for kind in ("listening", "upgrade", "bootstrap-sent", "close", "stop"):
        record = one(kind)
        _require(
            record.get("pid") == pid
            and record.get("synthetic") is True
            and record.get("browserProcessLaunched") is False,
            f"synthetic server {kind} attribution changed",
        )
    _require(
        one("listening").get("endpointPath") == ENDPOINT_PATH, "endpoint path changed"
    )
    _require(one("close").get("phase") == 3, "browser did not reach Page before close")
    _require(
        one("fixture-stopped").get("serverPid") == pid
        and one("fixture-stopped").get("returnCode") == 0,
        "synthetic server did not stop cleanly",
    )
    commands = [record for record in records if record.get("kind") == "command"]
    _require(
        [
            (record.get("guid"), record.get("method"), record.get("phase"))
            for record in commands
        ]
        == [
            ("", "initialize", 0),
            ("browser@1", "newContext", 1),
            ("context@1", "newPage", 2),
        ],
        "protocol command sequence changed",
    )
    _require(
        not any(
            record.get("kind")
            in (
                "protocol-error",
                "websocket-error",
                "start-error",
                "fixture-premature-exit",
            )
            for record in records
        ),
        "synthetic protocol fixture failed",
    )


def verify_directory(result_dir: Path) -> dict[str, object]:
    source_hashes = verify_sources()
    result_dir = result_dir.resolve()
    _require(
        result_dir.parent == RESULT_PARENT.resolve()
        and result_dir.name.startswith(
            "python-connect-navigation-closed-synthetic-v2-"
        ),
        "result directory is outside this versioned diagnostic",
    )
    wheel = result_dir / WHEEL_NAME
    _require(wheel.is_file() and not wheel.is_symlink(), "frozen wheel missing")
    site = result_dir / "venv/lib/python3.12/site-packages"
    installed = _installed_wheel(wheel, site)
    freeze = result_dir / "tooling-freeze.txt"
    _require(freeze.is_file() and not freeze.is_symlink(), "tooling freeze missing")
    freeze_text = freeze.read_text().lower()
    _require(
        all(
            f"{name}=={version}" in freeze_text
            for name, version in TOOLING.items()
            if name != "python"
        ),
        "tooling versions changed",
    )
    lanes: dict[str, dict[str, str]] = {"oracle": {}, "candidate": {}}
    addresses: set[tuple[str, str]] = set()
    for mode in ("oracle", "candidate"):
        for engine in ENGINES:
            prefix = f"{mode}-{engine}"
            provenance = _json(result_dir / f"{prefix}.provenance.json")
            _require(
                provenance.get("schema") == 1
                and provenance.get("mode") == mode
                and provenance.get("engine") == engine
                and provenance.get("wheelSha256") == WHEEL_SHA256
                and provenance.get("exitCode") == 0,
                "lane provenance identity or exit changed",
            )
            env = cast(dict[str, str], provenance.get("environment"))
            bind = env.get("PWRIGHT_OFFICIAL_TAILSCALE_IP", "")
            host = env.get("PWRIGHT_OFFICIAL_TAILSCALE_HOST", "")
            _require(
                ipaddress.ip_address(bind) in ipaddress.ip_network("100.64.0.0/10")
                and host.endswith(".ts.net")
                and "/" not in host
                and ":" not in host,
                "lane did not use full Tailscale MagicDNS and bind",
            )
            addresses.add((bind, host))
            _require(
                env.get("PWRIGHT_PYTHON_NAV_CLOSED_MODE") == mode
                and env.get("PWRIGHT_PYTHON_NAV_CLOSED_ENGINE") == engine
                and env.get("PWRIGHT_OFFICIAL_LAUNCH_MODE") == "owned"
                and env.get("PWRIGHT_PYTHON_NAV_CLOSED_RECEIPT")
                == str(result_dir / f"{prefix}.receipt.jsonl")
                and env.get("PWRIGHT_PYTHON_NAV_CLOSED_SERVER") == str(SERVER)
                and env.get("PWRIGHT_PYTHON_NAV_CLOSED_UPSTREAM") == str(UPSTREAM)
                and env.get("PWRIGHT_PYTHON_NAV_CLOSED_CORE_UPSTREAM") == str(CORE),
                "lane environment changed",
            )
            command = cast(list[str], provenance.get("command"))
            _require(
                command[:3] == [str(result_dir / "venv/bin/python"), "-m", "pytest"]
                and "-p" in command
                and "pwright_python_connect_navigation_closed_synthetic_v1" in command
                and f"--browser={engine}" in command
                and command[-1]
                == str(UPSTREAM / CASE_BODY.split("::", 1)[0])
                + "::"
                + CASE_BODY.split("::", 1)[1],
                "lane command or exact official test selection changed",
            )
            hashes = cast(dict[str, str], provenance.get("fileSha256"))
            for suffix in ("xml", "log", "receipt.jsonl"):
                artifact = result_dir / f"{prefix}.{suffix}"
                _require(
                    sha256(artifact) == hashes.get(suffix),
                    f"{prefix} {suffix} hash changed",
                )
            outcome = _junit(result_dir / f"{prefix}.xml", mode, engine)
            _receipt(
                result_dir / f"{prefix}.receipt.jsonl", mode, engine, bind, host, site
            )
            lanes[mode][engine] = outcome
    _require(len(addresses) == 1, "oracle/candidate fixture addresses differ")
    result: dict[str, object] = {
        "schema": 1,
        "classification": "official-derived-fixture-adapted-internal-diagnostic",
        "promotableToOfficialLedger": False,
        "testSourceAdapted": False,
        "fixtureAdapted": True,
        "candidateWheelSha256": WHEEL_SHA256,
        "upstreamCommit": UPSTREAM_COMMIT,
        "harnessInputs": source_hashes,
        "installedWheelFiles": installed,
        "lanes": lanes,
        "fixtureAddress": {
            "bind": next(iter(addresses))[0],
            "host": next(iter(addresses))[1],
        },
        "result": "GREEN: oracle 3/3 passed; candidate 3/3 passed; fixture-adapted only",
    }
    attestation = result_dir / "attestation.json"
    if attestation.exists():
        _require(
            _json(attestation) == result,
            "stored attestation differs from independent verifier",
        )
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("result_dir", type=Path)
    args = parser.parse_args()
    result = verify_directory(args.result_dir)
    print(result["result"])


if __name__ == "__main__":
    main()
