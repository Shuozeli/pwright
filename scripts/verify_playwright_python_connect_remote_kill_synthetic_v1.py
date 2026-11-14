#!/usr/bin/env python3
"""Independently verify the four-case fixture-adapted Python remote-kill pair."""

from __future__ import annotations

import argparse
import hashlib
import ipaddress
import json
import subprocess
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-python")
CORE = Path("/Users/cyuan/projects/thirdparty/playwright-v1.58.0")
RESULT_PARENT = ROOT / ".work/official-conformance/results"
PLUGIN_DIR = ROOT / "packages/pwright-python/tests/official/connect_remote_kill_synthetic_v1"
LEGACY_DIR = ROOT / "packages/pwright-python/tests/official/connect_close_events_synthetic_v1"
RUNTIME = ROOT / "packages/pwright-python/tests/official/runtime"
SHIM = ROOT / "packages/pwright-python/tests/official/shim"
MANIFEST = PLUGIN_DIR / "manifest.json"
PLUGIN = PLUGIN_DIR / "pwright_python_connect_remote_kill_synthetic_v1.py"
SERVER = LEGACY_DIR / "server.cjs"
LEGACY_PLUGIN = LEGACY_DIR / "pwright_python_connect_close_events_synthetic_v1.py"
WHEEL_NAME = "shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl"
WHEEL_SHA256 = "32c414d3eb2a45417c4a3ba79705be96179dc2ec2dd20896cd0cda7d8169e3e8"
UPSTREAM_COMMIT = "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
CORE_COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
CASE_IDS = (
    "tests/async/test_browsertype_connect.py::test_browser_type_connect_disconnected_event_should_be_emitted_when_remote_killed_connection[chromium]",
    "tests/async/test_browsertype_connect.py::test_browser_type_connect_should_throw_when_used_after_is_connected_returns_false[chromium]",
    "tests/sync/test_browsertype_connect.py::test_browser_type_connect_should_forward_close_events_on_remote_kill[chromium]",
    "tests/sync/test_browsertype_connect.py::test_browser_type_connect_should_throw_when_used_after_is_connected_returns_false[chromium]",
)
SOURCE_HASHES = {
    "tests/async/test_browsertype_connect.py": "d47290f5acb45246ce2bae150d3c583d340806821b660be08d798ac3572e7b11",
    "tests/sync/test_browsertype_connect.py": "c71497683c6ed7b932dc12a02ac304d51370e32fdfe049aca22d745754d0a58b",
    "tests/conftest.py": "c4236e98ff1731fff1f7ef1c00b37fe3a0cb1e4a3fd07e89a491dc7129ea93cc",
    "tests/async/conftest.py": "3001008bb97ba0cc1ddb397a53df530b6881e3e3ba69a1ed2cac0de94cab4cdb",
    "tests/sync/conftest.py": "9ec5a65a98c312de67590792e80d3d9ea45312d56188f45101e71df6f7773d0b",
    "tests/server.py": "2cf1a6b805f99c1c82c5351b67df995f06f2a9a18d21dbd48d4d4dc6079c3030",
    "pyproject.toml": "96e2d4dcd75b40a101c77d12b4e29ea4d91efc6fd58992a67ed8cc837e2e1e08",
}
FIXTURE_HASHES = {
    "server.cjs": "cd23662f9da812f1189e1f4c0e70378bf3e0aec3ef613bc93dc135e60db60062",
    "pwright_python_connect_close_events_synthetic_v1.py": "efef316ef9bca8948eb87c48d538b41a35256696eaa8d23b09bf9059728e4490",
}
TOOLING = {
    "python": "3.12", "playwright": "1.58.0", "pytest": "8.4.2",
    "pytest-asyncio": "1.2.0", "pytest-timeout": "2.4.0",
    "pytest-rerunfailures": "15.1", "pytest-repeat": "0.9.4",
    "pillow": "11.3.0", "pixelmatch": "0.3.0", "twisted": "25.5.0",
    "pyopenssl": "25.1.0", "service-identity": "24.2.0",
    "autobahn": "23.1.2", "requests": "2.32.5",
}
LEDGERS = {
    "candidate": (
        ROOT / ".work/official-conformance/outcomes/python-connect-refused-strict-paired-v1/playwright-python-candidate-browser-connect-refused-strict-paired-v1.jsonl",
        "3095771a0928ecc076ecdec8eee5d2946d70de9dbd1ea15e00f61ad7fa79e836",
    ),
    "oracle": (
        ROOT / ".work/official-conformance/outcomes/python-connect-refused-strict-paired-v1/playwright-python-oracle-browser-connect-refused-strict-paired-v1.jsonl",
        "11afbc5f482a3d9cefa36c59598f1771903b74f93749dcd11e8b5b178a7a6208",
    ),
}
HARNESS_INPUTS = (
    MANIFEST, PLUGIN, SERVER, LEGACY_PLUGIN,
    RUNTIME / "pwright_official_server_runtime.py",
    RUNTIME / "pwright_official_oracle_runtime.py",
    SHIM / "pwright_candidate_runtime.py",
    SHIM / "playwright/async_api.py",
    SHIM / "playwright/sync_api.py",
    ROOT / "scripts/run_playwright_python_connect_remote_kill_synthetic_v1.py",
    ROOT / "scripts/test_verify_playwright_python_connect_remote_kill_synthetic_v1.py",
    Path(__file__).resolve(),
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def read_json(path: Path) -> dict[str, Any]:
    require(path.is_file() and not path.is_symlink(), f"missing regular JSON: {path}")
    value = json.loads(path.read_text())
    require(isinstance(value, dict), f"not a JSON object: {path}")
    return value


def harness_hashes() -> dict[str, str]:
    for path in HARNESS_INPUTS:
        require(path.is_file() and not path.is_symlink(), f"harness source missing: {path}")
    return {str(path.relative_to(ROOT)): sha256(path) for path in HARNESS_INPUTS}


def _git(checkout: Path, *args: str) -> str:
    return subprocess.check_output(["git", "-C", str(checkout), *args], text=True).strip()


def verify_sources() -> dict[str, str]:
    require(_git(UPSTREAM, "rev-parse", "HEAD") == UPSTREAM_COMMIT, "Python upstream drifted")
    require(not _git(UPSTREAM, "status", "--porcelain"), "Python upstream is dirty")
    require(_git(CORE, "rev-parse", "HEAD") == CORE_COMMIT, "Core upstream drifted")
    require(not _git(CORE, "status", "--porcelain", "--untracked-files=no"), "Core tracked source is dirty")
    for relative, expected in SOURCE_HASHES.items():
        path = UPSTREAM / relative
        require(path.is_file() and not path.is_symlink() and sha256(path) == expected, f"Python source changed: {relative}")
    for name, expected in FIXTURE_HASHES.items():
        path = LEGACY_DIR / name
        require(path.is_file() and not path.is_symlink() and sha256(path) == expected, f"synthetic fixture changed: {name}")
    expected_manifest = {
        "schema": 1, "kind": "playwright-python-connect-remote-kill-synthetic-v1",
        "classification": "official-derived-fixture-adapted-internal-diagnostic",
        "repository": "https://github.com/microsoft/playwright-python.git",
        "tag": "v1.58.0", "commit": UPSTREAM_COMMIT,
        "coreProtocolCommit": CORE_COMMIT,
        "requiresBrowser": False, "browserProcessLaunched": False,
        "fixtureAdapted": True, "adaptedFixture": "launch_server",
        "testSourceAdapted": False, "publicApiParityEvidence": False,
        "unadaptedOfficialPass": False, "promotableToOfficialLedger": False,
        "promotedToPythonLedger": False, "candidateAbi": 125,
        "candidateSelectedFeatures": [
            "browser-registry", "playwright-server-initialize",
            "playwright-server-connect-session",
        ],
        "candidateWheelSha256": WHEEL_SHA256, "engines": ["chromium"],
        "expandedCaseIds": list(CASE_IDS),
        "ledgerZeroBasedIndices": [808, 823, 4969, 4978],
        "upstreamSourceSha256": SOURCE_HASHES,
        "fixtureSourceSha256": FIXTURE_HASHES,
        "toolingVersions": TOOLING,
    }
    require(read_json(MANIFEST) == expected_manifest, "remote-kill manifest changed")
    for mode, (path, expected) in LEDGERS.items():
        require(path.is_file() and not path.is_symlink() and sha256(path) == expected, f"{mode} predecessor ledger changed")
        rows = [json.loads(line) for line in path.read_text().splitlines()]
        require(len(rows) == 6526, f"{mode} ledger length changed")
        for index, case_id in zip((808, 823, 4969, 4978), CASE_IDS, strict=True):
            row = rows[index]
            require(row.get("id") == case_id and row.get("outcome") == "not-run" and row.get("attempts") == 0, f"{mode} target ledger row changed")
    return {mode: expected for mode, (_, expected) in LEDGERS.items()}


def expected_command(python: Path, xml: Path, mode: str) -> list[str]:
    command = [
        str(python), "-m", "pytest", "-q", "--tb=short", "--override-ini", "addopts=",
        "--import-mode=importlib", "-c", str(UPSTREAM / "pyproject.toml"),
        "-p", "pwright_official_server_runtime",
        "-p", "pwright_python_connect_remote_kill_synthetic_v1",
        "-p", "pwright_candidate_runtime" if mode == "candidate" else "pwright_official_oracle_runtime",
        "--browser=chromium", "--timeout=30", "--timeout-method=signal", f"--junitxml={xml}",
    ]
    return command + [str(UPSTREAM / case.split("::", 1)[0]) + "::" + case.split("::", 1)[1] for case in CASE_IDS]


def junit(path: Path) -> dict[str, str]:
    require(path.is_file() and not path.is_symlink(), "JUnit missing")
    root = ET.parse(path).getroot()
    suite = root.find("testsuite") if root.tag == "testsuites" else root
    require(suite is not None and suite.tag == "testsuite", "invalid JUnit suite")
    cases = list(suite.findall("testcase"))
    require(len(cases) == 4 and int(suite.attrib.get("tests", "-1")) == 4, "JUnit case count changed")
    outcomes: dict[str, str] = {}
    for case in cases:
        classname = case.attrib.get("classname", "")
        api = "async" if ".async." in f".{classname}." else "sync" if ".sync." in f".{classname}." else None
        require(api is not None, "JUnit class does not identify public API lane")
        case_id = f"tests/{api}/test_browsertype_connect.py::{case.attrib.get('name', '')}"
        require(case_id not in outcomes, "duplicate JUnit case")
        require(case.find("skipped") is None, "JUnit skipped a selected case")
        outcomes[case_id] = "failed" if case.find("failure") is not None or case.find("error") is not None else "passed"
    require(set(outcomes) == set(CASE_IDS), "JUnit selected case IDs changed")
    failures = sum(status == "failed" for status in outcomes.values())
    require(int(suite.attrib.get("failures", "-1")) + int(suite.attrib.get("errors", "-1")) == failures, "JUnit failure accounting changed")
    require(int(suite.attrib.get("skipped", "-1")) == 0, "JUnit skip accounting changed")
    return outcomes


def receipt(path: Path, mode: str, outcomes: dict[str, str], bind: str, host: str, site_packages: Path) -> dict[str, int]:
    require(path.is_file() and not path.is_symlink(), "receipt missing")
    records = [json.loads(line) for line in path.read_text().splitlines() if line]
    require(records and all(isinstance(item, dict) for item in records), "invalid receipt JSONL")
    require(all(item.get("mode") == mode and item.get("fixtureBind") == bind and item.get("fixtureHost") == host for item in records), "receipt lane or fixture address changed")
    kinds = {name: [item for item in records if item.get("kind") == name] for name in {
        "runtime-import", "runtime-import-async", "fixture-adapted", "selected", "fixture-created",
        "fixture-stopped", "case-call", "listening", "upgrade", "command", "bootstrap-sent",
        "close", "stop", "start-error", "websocket-error", "fixture-premature-exit",
    }}
    require(sum(map(len, kinds.values())) == len(records), "receipt contains unknown event")
    require(not kinds["start-error"] and not kinds["websocket-error"] and not kinds["fixture-premature-exit"], "synthetic fixture errored")
    for name in ("runtime-import", "runtime-import-async", "fixture-adapted", "selected"):
        require(len(kinds[name]) == 1, f"receipt {name} count changed")
    require(kinds["selected"][0].get("caseIds") == list(CASE_IDS) and kinds["selected"][0].get("testSourceAdapted") is False, "selected upstream cases changed")
    adapted = kinds["fixture-adapted"][0]
    require(adapted.get("fixture") == "launch_server" and adapted.get("testSourceAdapted") is False and adapted.get("browserProcessLaunched") is False and adapted.get("upstreamCommit") == UPSTREAM_COMMIT, "launch_server adaptation changed")
    imported = kinds["runtime-import"][0]
    imported_async = kinds["runtime-import-async"][0]
    if mode == "oracle":
        require(imported.get("candidatePackagePath") is None and str(imported.get("publicApiPath", "")).startswith(str(site_packages / "playwright/")), "oracle sync API was not independent official Playwright")
        require(str(imported_async.get("publicApiPath", "")).startswith(str(site_packages / "playwright/")), "oracle async API was not independent official Playwright")
    else:
        require(imported.get("candidatePackagePath") == str((site_packages / "pwright/__init__.py").resolve()), "candidate did not import frozen wheel")
        require(imported.get("publicApiPath") == str((SHIM / "playwright/sync_api.py").resolve()), "candidate sync API shim changed")
        require(imported_async.get("publicApiPath") == str((SHIM / "playwright/async_api.py").resolve()), "candidate async API shim changed")
    created = kinds["fixture-created"]
    stopped = kinds["fixture-stopped"]
    calls = kinds["case-call"]
    require(len(created) == len(stopped) == len(calls) == 4, "four fixture lifetimes/case calls not observed")
    require([item.get("caseId") for item in calls] == list(CASE_IDS), "case call order changed")
    require(all(item.get("outcome") == outcomes[item["caseId"]] for item in calls), "case call outcome differs from JUnit")
    pids = [item.get("serverPid") for item in created]
    require(all(isinstance(pid, int) and pid > 0 for pid in pids) and len(set(pids)) == 4, "fixture process identities missing or reused")
    require({item.get("serverPid") for item in stopped} == set(pids) and all(item.get("returnCode") == 0 for item in stopped), "fixture process not reaped cleanly")
    for created_item, stopped_item, call in zip(created, stopped, calls, strict=True):
        pid = created_item["serverPid"]
        require(stopped_item.get("serverPid") == pid, "fixture process/case lifetime order changed")
        require(created_item.get("engine") == "chromium" and created_item.get("fixtureAdapted") is True and created_item.get("testSourceAdapted") is False and created_item.get("browserProcessLaunched") is False and created_item.get("endpointPath") == "/browser-synthetic-python-close-events", "fixture was not browser-free")
        server_records = [item for item in records if item.get("synthetic") is True and item.get("pid") == pid]
        require(server_records and all(item.get("browserProcessLaunched") is False for item in server_records), "synthetic server ownership changed")
        server_kinds = [item.get("kind") for item in server_records]
        require(server_kinds.count("listening") == 1 and server_kinds.count("stop") == 1 and server_kinds.count("close") == 1, "server lifecycle receipt missing")
        require(server_kinds.count("upgrade") == 1 and server_kinds.count("bootstrap-sent") == 1, "case did not connect")
        commands = [(item.get("guid"), item.get("method")) for item in server_records if item.get("kind") == "command"]
        require(commands == [("", "initialize"), ("browser@1", "newContext"), ("context@1", "newPage")], "case protocol path changed")
        close = next(item for item in server_records if item.get("kind") == "close")
        require(close.get("phase") == 3, "case did not reach Page before remote kill")
        positions = {id(item): index for index, item in enumerate(records)}
        require(
            positions[id(created_item)] < positions[id(close)]
            < positions[id(stopped_item)] < positions[id(call)],
            "remote-kill fixture/call order changed",
        )
    return {name: len(items) for name, items in kinds.items() if items}


def verify_wheel_install(wheel: Path, site_packages: Path) -> dict[str, str]:
    require(wheel.is_file() and not wheel.is_symlink() and sha256(wheel) == WHEEL_SHA256, "frozen candidate wheel changed")
    installed: dict[str, str] = {}
    with zipfile.ZipFile(wheel) as archive:
        for name in archive.namelist():
            if not name.startswith(("pwright/", "pwright.libs/")) or name.endswith("/"):
                continue
            target = site_packages / name
            require(target.is_file() and not target.is_symlink(), f"wheel install file missing: {name}")
            digest = hashlib.sha256(archive.read(name)).hexdigest()
            require(sha256(target) == digest, f"installed wheel differs from archive: {name}")
            installed[name] = digest
    require(all(name in installed for name in ("pwright/_native.abi3.so", "pwright/async_api.py", "pwright/sync_api.py")), "selected wheel lacks remote Python API")
    return installed


def verify_directory(result_dir: Path) -> dict[str, Any]:
    require(not result_dir.is_symlink(), "result directory is symlinked")
    result_dir = result_dir.resolve()
    require(result_dir.is_dir() and result_dir.parent == RESULT_PARENT.resolve() and result_dir.name.startswith("python-connect-remote-kill-synthetic-v1-"), "result directory is outside immutable diagnostic parent")
    ledger_hashes = verify_sources()
    inputs = harness_hashes()
    wheel = result_dir / WHEEL_NAME
    site_packages = result_dir / "venv/lib/python3.12/site-packages"
    installed = verify_wheel_install(wheel, site_packages)
    lanes: dict[str, Any] = {}
    for mode in ("oracle", "candidate"):
        provenance = read_json(result_dir / f"{mode}.provenance.json")
        xml = result_dir / f"{mode}.xml"
        log = result_dir / f"{mode}.log"
        receipt_path = result_dir / f"{mode}.receipt.jsonl"
        require(provenance.get("mode") == mode and provenance.get("schema") == 1 and provenance.get("wheelSha256") == WHEEL_SHA256, f"{mode} provenance identity changed")
        require(provenance.get("command") == expected_command(result_dir / "venv/bin/python", xml, mode), f"{mode} pytest command changed")
        environment = provenance.get("environment")
        require(isinstance(environment, dict), f"{mode} environment absent")
        env = environment
        bind = str(env.get("PWRIGHT_OFFICIAL_TAILSCALE_IP", ""))
        host = str(env.get("PWRIGHT_OFFICIAL_TAILSCALE_HOST", ""))
        require(ipaddress.ip_address(bind) in ipaddress.ip_network("100.64.0.0/10") and host.endswith(".ts.net"), f"{mode} Tailscale bind/host changed")
        require(env.get("PWRIGHT_PYTHON_CLOSE_EVENTS_MODE") == mode and env.get("PWRIGHT_PYTHON_CLOSE_EVENTS_ENGINE_FILTER") == "chromium" and env.get("PWRIGHT_PYTHON_CLOSE_EVENTS_RECEIPT") == str(receipt_path) and env.get("PWRIGHT_PYTHON_CLOSE_EVENTS_SERVER") == str(SERVER) and env.get("PWRIGHT_PYTHON_CLOSE_EVENTS_UPSTREAM") == str(UPSTREAM), f"{mode} fixture environment changed")
        require(env.get("PWRIGHT_OFFICIAL_LAUNCH_MODE") == "owned" and env.get("PWRIGHT_CDP_ENDPOINT") is None and env.get("PWRIGHT_CDP_WS_URL") is None, f"{mode} inherited browser/CDP setting")
        py_paths = str(env.get("PYTHONPATH", "")).split(":")
        expected_paths = [str(SHIM), str(RUNTIME), str(PLUGIN_DIR), str(LEGACY_DIR)] if mode == "candidate" else [str(RUNTIME), str(PLUGIN_DIR), str(LEGACY_DIR)]
        require(py_paths == expected_paths, f"{mode} import isolation changed")
        require(all(path.is_file() and not path.is_symlink() for path in (xml, log, receipt_path)), f"{mode} evidence file missing")
        require(provenance.get("fileSha256") == {"xml": sha256(xml), "log": sha256(log), "receipt.jsonl": sha256(receipt_path)}, f"{mode} evidence hash changed")
        outcomes = junit(xml)
        passed = sum(status == "passed" for status in outcomes.values())
        require(provenance.get("exitCode") == (0 if passed == 4 else 1), f"{mode} pytest exit/JUnit mismatch")
        counts = receipt(receipt_path, mode, outcomes, bind, host, site_packages)
        require(mode != "oracle" or passed == 4, "oracle did not validate the synthetic test semantics")
        lanes[mode] = {"cases": outcomes, "passed": passed, "receiptCounts": counts, "fileSha256": provenance["fileSha256"], "provenanceSha256": sha256(result_dir / f"{mode}.provenance.json")}
    return {
        "schema": 1, "classification": "official-derived-fixture-adapted-internal-diagnostic",
        "fixtureAdapted": True, "testSourceAdapted": False,
        "requiresBrowser": False, "browserProcessLaunched": False,
        "publicApiParityEvidence": False, "unadaptedOfficialPass": False,
        "promotedToPythonLedger": False, "candidateAbi": 125,
        "candidateWheelSha256": WHEEL_SHA256,
        "upstreamCommit": UPSTREAM_COMMIT, "coreProtocolCommit": CORE_COMMIT,
        "caseIds": list(CASE_IDS), "ledgerZeroBasedIndices": [808, 823, 4969, 4978],
        "predecessorLedgerSha256": ledger_hashes,
        "upstreamSourceSha256": SOURCE_HASHES,
        "harnessInputs": inputs, "installedWheelFilesSha256": installed,
        "lanes": lanes,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("result_dir", type=Path)
    args = parser.parse_args()
    result = verify_directory(args.result_dir)
    attestation = args.result_dir / "attestation.json"
    if attestation.exists():
        require(read_json(attestation) == result, "saved attestation differs from independent verification")
    print(f"verified fixture-adapted remote kill: oracle {result['lanes']['oracle']['passed']}/4; candidate {result['lanes']['candidate']['passed']}/4")


if __name__ == "__main__":
    main()
