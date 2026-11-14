#!/usr/bin/env python3
"""Independently verify the fixture-adapted Python close-event pair."""

from __future__ import annotations

import argparse
import hashlib
import ipaddress
import json
import subprocess
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path
from typing import Any, cast

ROOT = Path(__file__).resolve().parents[1]
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-python")
CORE = Path("/Users/cyuan/projects/thirdparty/playwright-v1.58.0")
RESULT_PARENT = ROOT / ".work/official-conformance/results"
PLUGIN_DIR = ROOT / "packages/pwright-python/tests/official/connect_close_events_synthetic_v1"
MANIFEST = PLUGIN_DIR / "manifest.json"
SERVER = PLUGIN_DIR / "server.cjs"
PLUGIN = PLUGIN_DIR / "pwright_python_connect_close_events_synthetic_v1.py"
SOURCE_PROBE = PLUGIN_DIR / "source_probe.py"
RUNTIME = ROOT / "packages/pwright-python/tests/official/runtime"
SHIM = ROOT / "packages/pwright-python/tests/official/shim"
WHEEL_NAME = "shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl"
WHEEL_SHA256 = "c065171eb151fca881927500670ece4d05f749697deaed69e5cc0c64bf8ffdcb"
UPSTREAM_COMMIT = "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
CORE_COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
CASE_BODY = (
    "tests/sync/test_browsertype_connect.py::"
    "test_browser_type_connect_should_forward_close_events_to_pages"
)
CASE_ID = f"{CASE_BODY}[chromium]"
FEATURES = (
    "browser-registry",
    "playwright-server-initialize",
    "playwright-server-connect-session",
)
SOURCE_HASHES = {
    "tests/sync/test_browsertype_connect.py": (
        "c71497683c6ed7b932dc12a02ac304d51370e32fdfe049aca22d745754d0a58b"
    ),
    "tests/conftest.py": "c4236e98ff1731fff1f7ef1c00b37fe3a0cb1e4a3fd07e89a491dc7129ea93cc",
    "tests/sync/conftest.py": "9ec5a65a98c312de67590792e80d3d9ea45312d56188f45101e71df6f7773d0b",
    "tests/server.py": "2cf1a6b805f99c1c82c5351b67df995f06f2a9a18d21dbd48d4d4dc6079c3030",
    "pyproject.toml": "96e2d4dcd75b40a101c77d12b4e29ea4d91efc6fd58992a67ed8cc837e2e1e08",
}
HARNESS_INPUTS = (
    MANIFEST,
    SERVER,
    PLUGIN,
    SOURCE_PROBE,
    RUNTIME / "pwright_official_server_runtime.py",
    RUNTIME / "pwright_official_oracle_runtime.py",
    SHIM / "pwright_candidate_runtime.py",
    SHIM / "playwright/sync_api.py",
    ROOT / "scripts/run_playwright_python_connect_close_events_synthetic_v1.py",
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


def _json(path: Path) -> dict[str, Any]:
    _require(path.is_file() and not path.is_symlink(), f"missing regular evidence: {path}")
    value = json.loads(path.read_text())
    _require(isinstance(value, dict), f"evidence is not a JSON object: {path}")
    return value


def _git(checkout: Path, *args: str) -> str:
    return subprocess.check_output(["git", "-C", str(checkout), *args], text=True).strip()


def verify_sources() -> dict[str, str]:
    _require(_git(UPSTREAM, "rev-parse", "HEAD") == UPSTREAM_COMMIT, "Python upstream drifted")
    _require(not _git(UPSTREAM, "status", "--porcelain"), "Python upstream is dirty")
    _require(_git(CORE, "rev-parse", "HEAD") == CORE_COMMIT, "Core upstream drifted")
    _require(
        not _git(CORE, "status", "--porcelain", "--untracked-files=no"),
        "Core upstream tracked source drifted",
    )
    for relative, expected in SOURCE_HASHES.items():
        _require(sha256(UPSTREAM / relative) == expected, f"upstream source drifted: {relative}")
    manifest = _json(MANIFEST)
    _require(manifest.get("schema") == 1, "manifest schema changed")
    _require(
        manifest.get("kind") == "playwright-python-connect-close-events-synthetic-v1"
        and manifest.get("classification") == "official-derived-fixture-adapted-internal-diagnostic"
        and manifest.get("repository") == "https://github.com/microsoft/playwright-python.git"
        and manifest.get("tag") == "v1.58.0"
        and manifest.get("commit") == UPSTREAM_COMMIT
        and manifest.get("coreProtocolCommit") == CORE_COMMIT,
        "manifest provenance changed",
    )
    _require(
        manifest.get("requiresBrowser") is False
        and manifest.get("fixtureAdapted") is True
        and manifest.get("testSourceAdapted") is False
        and manifest.get("promotableToOfficialLedger") is False,
        "manifest classification changed",
    )
    _require(
        manifest.get("candidateAbi") == 125
        and manifest.get("candidateSelectedFeatures") == list(FEATURES)
        and manifest.get("candidateWheelSha256") == WHEEL_SHA256,
        "manifest candidate identity changed",
    )
    _require(
        manifest.get("testBodies") == [CASE_BODY]
        and manifest.get("engines") == ["chromium"]
        and manifest.get("expandedCaseIds") == [CASE_ID]
        and manifest.get("upstreamSourceSha256") == SOURCE_HASHES,
        "manifest test selection changed",
    )
    for path in HARNESS_INPUTS:
        _require(path.is_file() and not path.is_symlink(), f"harness input missing: {path}")
    ws = subprocess.check_output(
        ["node", "-p", f"require.resolve('ws', {{paths: [{str(CORE)!r}]}})"], text=True
    ).strip()
    ws_path = Path(ws).resolve()
    _require(ws_path.is_file() and ws_path.is_relative_to(CORE), "Core ws dependency moved")
    return {"wsModulePath": str(ws_path), "wsModuleSha256": sha256(ws_path)}


def _installed_wheel(wheel: Path, site_packages: Path) -> dict[str, str]:
    _require(sha256(wheel) == WHEEL_SHA256, "frozen candidate wheel hash changed")
    installed: dict[str, str] = {}
    with zipfile.ZipFile(wheel) as archive:
        for name in archive.namelist():
            if not (
                name.startswith("pwright/") or name.startswith("pwright.libs/")
            ) or name.endswith("/"):
                continue
            target = site_packages / name
            _require(target.is_file() and not target.is_symlink(), f"wheel file missing: {name}")
            digest = hashlib.sha256(archive.read(name)).hexdigest()
            _require(sha256(target) == digest, f"wheel install differs: {name}")
            installed[name] = digest
    _require(
        "pwright/_native.abi3.so" in installed and "pwright/sync_api.py" in installed,
        "selected wheel lacks Python remote binding",
    )
    return installed


def _junit(path: Path) -> str:
    _require(path.is_file() and not path.is_symlink(), f"JUnit missing: {path}")
    root = ET.parse(path).getroot()
    suite = root.find("testsuite") if root.tag == "testsuites" else root
    if suite is None or suite.tag != "testsuite":
        raise ValueError("invalid JUnit suite")
    cases = list(suite.findall("testcase"))
    _require(len(cases) == 1, "JUnit did not contain exactly one case")
    case = cases[0]
    _require(
        case.attrib.get("name") == CASE_ID.split("::", 1)[1]
        and "sync" in case.attrib.get("classname", ""),
        "JUnit case identity changed",
    )
    _require(
        case.find("failure") is None
        and case.find("error") is None
        and case.find("skipped") is None
        and int(suite.attrib.get("tests", "-1")) == 1
        and int(suite.attrib.get("failures", "-1")) == 0
        and int(suite.attrib.get("errors", "-1")) == 0
        and int(suite.attrib.get("skipped", "-1")) == 0,
        "JUnit candidate/oracle case did not pass without skips",
    )
    return "passed"


def _receipt(path: Path, mode: str, bind: str, host: str, site_packages: Path) -> dict[str, int]:
    _require(path.is_file() and not path.is_symlink(), f"receipt missing: {path}")
    records = [json.loads(line) for line in path.read_text().splitlines() if line]
    _require(
        bool(records) and all(isinstance(item, dict) for item in records), "invalid receipt JSONL"
    )
    _require(
        all(
            item.get("mode") == mode
            and item.get("fixtureBind") == bind
            and item.get("fixtureHost") == host
            for item in records
        ),
        "mixed receipt lane or address",
    )
    kinds = {
        kind: [item for item in records if item.get("kind") == kind]
        for kind in {
            "runtime-import",
            "fixture-adapted",
            "selected",
            "fixture-created",
            "fixture-stopped",
            "case-call",
            "listening",
            "upgrade",
            "command",
            "bootstrap-sent",
            "close",
            "stop",
            "start-error",
            "websocket-error",
        }
    }
    _require(not kinds["start-error"] and not kinds["websocket-error"], "fixture errored")
    for kind in (
        "runtime-import",
        "fixture-adapted",
        "selected",
        "fixture-created",
        "fixture-stopped",
        "case-call",
        "listening",
        "upgrade",
        "bootstrap-sent",
        "close",
        "stop",
    ):
        _require(len(kinds[kind]) == 1, f"fixture receipt {kind} count changed")
    _require(len(kinds["command"]) == 3, "protocol command count changed")
    _require(
        [(item.get("guid"), item.get("method")) for item in kinds["command"]]
        == [("", "initialize"), ("browser@1", "newContext"), ("context@1", "newPage")],
        "protocol command sequence changed",
    )
    _require(
        kinds["case-call"][0].get("caseId") == CASE_ID
        and kinds["case-call"][0].get("outcome") == "passed"
        and kinds["selected"][0].get("caseId") == CASE_ID,
        "test body call or selection changed",
    )
    _require(
        kinds["fixture-adapted"][0].get("fixture") == "launch_server"
        and kinds["fixture-adapted"][0].get("testSourceAdapted") is False
        and kinds["fixture-adapted"][0].get("browserProcessLaunched") is False
        and kinds["fixture-adapted"][0].get("upstreamCommit") == UPSTREAM_COMMIT,
        "launch_server substitution absent",
    )
    created = kinds["fixture-created"][0]
    pid = created.get("serverPid")
    _require(
        created.get("fixtureAdapted") is True
        and created.get("testSourceAdapted") is False
        and created.get("browserProcessLaunched") is False
        and created.get("engine") == "chromium"
        and created.get("endpointPath") == "/browser-synthetic-python-close-events"
        and kinds["fixture-stopped"][0].get("serverPid") == pid
        and kinds["fixture-stopped"][0].get("returnCode") == 0,
        "synthetic server lifetime changed",
    )
    _require(
        all(
            item.get("pid") == pid and item.get("browserProcessLaunched") is False
            for item in records
            if item.get("synthetic") is True
        )
        and kinds["close"][0].get("phase") == 3,
        "server did not complete the browser-free Page/Context fixture",
    )
    runtime = kinds["runtime-import"][0]
    if mode == "oracle":
        _require(
            runtime.get("candidatePackagePath") is None
            and str(runtime.get("publicApiPath", "")).startswith(
                str(site_packages / "playwright/")
            ),
            "oracle did not import independent official Playwright",
        )
    else:
        _require(
            runtime.get("publicApiPath") == str((SHIM / "playwright/sync_api.py").resolve())
            and runtime.get("candidatePackagePath")
            == str((site_packages / "pwright/__init__.py").resolve()),
            "candidate did not import frozen wheel through public shim",
        )
    return {kind: len(items) for kind, items in kinds.items() if items}


def verify_directory(result_dir: Path) -> dict[str, Any]:
    _require(not result_dir.is_symlink(), "result destination is symlinked")
    result_dir = result_dir.resolve()
    _require(
        result_dir.is_dir()
        and result_dir.parent == RESULT_PARENT.resolve()
        and result_dir.name.startswith("python-connect-close-events-synthetic-v1-"),
        "result destination is outside versioned diagnostic parent",
    )
    source = verify_sources()
    frozen_wheel = result_dir / WHEEL_NAME
    site_packages = result_dir / "venv/lib/python3.12/site-packages"
    installed = _installed_wheel(frozen_wheel, site_packages)
    harness_inputs = {str(path.relative_to(ROOT)): sha256(path) for path in HARNESS_INPUTS}
    lanes: dict[str, Any] = {}
    for mode in ("oracle", "candidate"):
        provenance = _json(result_dir / f"{mode}.provenance.json")
        _require(
            provenance.get("mode") == mode and provenance.get("exitCode") == 0,
            f"{mode} process failed",
        )
        _require(provenance.get("wheelSha256") == WHEEL_SHA256, f"{mode} wheel drifted")
        raw_command = provenance.get("command")
        if not isinstance(raw_command, list) or not all(
            isinstance(value, str) for value in cast(list[object], raw_command)
        ):
            raise ValueError(f"{mode} pytest command is not a string list")
        command = cast(list[str], raw_command)
        _require(
            command[-1]
            == str(UPSTREAM / CASE_BODY.split("::", 1)[0]) + "::" + CASE_BODY.split("::", 1)[1]
            and command.count("-p") == 3
            and (
                "pwright_candidate_runtime"
                if mode == "candidate"
                else "pwright_official_oracle_runtime"
            )
            in command,
            f"{mode} pytest command changed",
        )
        raw_env = provenance.get("environment")
        if not isinstance(raw_env, dict):
            raise ValueError(f"{mode} environment absent")
        env = cast(dict[str, object], raw_env)
        bind = str(env.get("PWRIGHT_OFFICIAL_TAILSCALE_IP", ""))
        host = str(env.get("PWRIGHT_OFFICIAL_TAILSCALE_HOST", ""))
        _require(
            ipaddress.ip_address(bind) in ipaddress.ip_network("100.64.0.0/10")
            and host.endswith(".ts.net")
            and env.get("PWRIGHT_PYTHON_CLOSE_EVENTS_MODE") == mode,
            f"{mode} fixture address or mode changed",
        )
        files = {
            suffix: result_dir / f"{mode}.{suffix}" for suffix in ("xml", "log", "receipt.jsonl")
        }
        for suffix, path in files.items():
            _require(path.is_file() and not path.is_symlink(), f"{mode} {suffix} missing")
            _require(
                provenance.get("fileSha256", {}).get(suffix) == sha256(path),
                f"{mode} {suffix} hash changed",
            )
        status = _junit(files["xml"])
        counts = _receipt(files["receipt.jsonl"], mode, bind, host, site_packages)
        lanes[mode] = {
            "passed": int(status == "passed"),
            "case": CASE_ID,
            "receiptCounts": counts,
            "fileSha256": {suffix: sha256(path) for suffix, path in files.items()},
            "provenanceSha256": sha256(result_dir / f"{mode}.provenance.json"),
        }
    _require(
        lanes["oracle"]["passed"] == lanes["candidate"]["passed"] == 1,
        "paired lanes did not both pass",
    )
    return {
        "schema": 1,
        "classification": "official-derived-fixture-adapted-internal-diagnostic",
        "fixtureAdapted": True,
        "testSourceAdapted": False,
        "promotableToOfficialLedger": False,
        "upstreamCommit": UPSTREAM_COMMIT,
        "coreProtocolCommit": CORE_COMMIT,
        "caseId": CASE_ID,
        "candidateWheelSha256": WHEEL_SHA256,
        "candidateAbi": 125,
        "candidateSelectedFeatures": list(FEATURES),
        "source": source,
        "installedWheelFiles": installed,
        "harnessInputs": harness_inputs,
        "lanes": lanes,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("result_dir", type=Path)
    args = parser.parse_args()
    actual = verify_directory(args.result_dir)
    attestation = _json(args.result_dir / "attestation.json")
    _require(actual == attestation, "attestation does not match independent replay")
    print("fixture-adapted Python close events: oracle 1/1, candidate 1/1; verified")


if __name__ == "__main__":
    main()
