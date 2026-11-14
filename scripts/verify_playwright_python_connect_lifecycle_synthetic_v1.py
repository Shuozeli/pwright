#!/usr/bin/env python3
"""Verify the fixture-adapted Python connect lifecycle capture without promotion."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path
from typing import Any

ROOT = Path(__file__).parents[1].resolve()
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-python")
CORE = Path("/Users/cyuan/projects/thirdparty/playwright-v1.58.0")
RESULT_PARENT = ROOT / ".work/official-conformance/results"
PLUGIN_DIR = ROOT / "packages/pwright-python/tests/official/connect_lifecycle_synthetic_v1"
MANIFEST = PLUGIN_DIR / "manifest.json"
SERVER = PLUGIN_DIR / "server.cjs"
PLUGIN = PLUGIN_DIR / "pwright_python_connect_lifecycle_synthetic_v1.py"
RUNTIME = ROOT / "packages/pwright-python/tests/official/runtime"
SHIM = ROOT / "packages/pwright-python/tests/official/shim"
WHEEL_NAME = "shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl"
WHEEL_SHA256 = "3d8b4aa56f2a9cfe0777daf8b40bb258cbd5025756f7e59aba4d151aaf0f2349"
UPSTREAM_COMMIT = "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
CORE_COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
ENGINES = ("chromium", "firefox", "webkit")
TEST_BODIES = (
    "tests/async/test_browsertype_connect.py::test_browser_type_disconnected_event_should_have_browser_as_argument",
    "tests/async/test_browsertype_connect.py::test_browser_type_connect_set_browser_connected_state",
    "tests/sync/test_browsertype_connect.py::test_browser_type_disconnected_event_should_have_browser_as_argument",
    "tests/sync/test_browsertype_connect.py::test_browser_type_connect_set_browser_connected_state",
)
EXPECTED_IDS = tuple(f"{body}[{engine}]" for engine in ENGINES for body in TEST_BODIES)
SOURCE_HASHES = {
    "tests/async/test_browsertype_connect.py": (
        "d47290f5acb45246ce2bae150d3c583d340806821b660be08d798ac3572e7b11"
    ),
    "tests/sync/test_browsertype_connect.py": (
        "c71497683c6ed7b932dc12a02ac304d51370e32fdfe049aca22d745754d0a58b"
    ),
    "tests/conftest.py": "c4236e98ff1731fff1f7ef1c00b37fe3a0cb1e4a3fd07e89a491dc7129ea93cc",
    "tests/async/conftest.py": "3001008bb97ba0cc1ddb397a53df530b6881e3e3ba69a1ed2cac0de94cab4cdb",
    "tests/sync/conftest.py": "9ec5a65a98c312de67590792e80d3d9ea45312d56188f45101e71df6f7773d0b",
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
FEATURES = (
    "browser-registry",
    "playwright-server-initialize",
    "playwright-server-connect-session",
)
HARNESS_INPUTS = (
    MANIFEST,
    PLUGIN,
    SERVER,
    RUNTIME / "pwright_official_server_runtime.py",
    RUNTIME / "pwright_official_oracle_runtime.py",
    SHIM / "pwright_candidate_runtime.py",
    SHIM / "playwright/async_api.py",
    SHIM / "playwright/sync_api.py",
    ROOT / "scripts/run_playwright_python_connect_lifecycle_synthetic_v1.py",
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


def verify_sources() -> dict[str, Any]:
    _require(
        _git(UPSTREAM, "rev-parse", "HEAD") == UPSTREAM_COMMIT,
        "Python upstream commit drifted",
    )
    _require(not _git(UPSTREAM, "status", "--porcelain"), "Python upstream checkout is dirty")
    _require(
        _git(CORE, "rev-parse", "HEAD") == CORE_COMMIT,
        "Core protocol checkout commit drifted",
    )
    _require(
        not _git(CORE, "status", "--porcelain", "--untracked-files=no"),
        "Core checkout tracked source drifted",
    )
    for relative, digest in SOURCE_HASHES.items():
        _require(
            sha256(UPSTREAM / relative) == digest,
            f"pinned Python source drifted: {relative}",
        )
    manifest = _json(MANIFEST)
    _require(
        manifest
        == {
            "schema": 1,
            "kind": "playwright-python-connect-lifecycle-synthetic-v1",
            "classification": "official-derived-fixture-adapted-internal-diagnostic",
            "repository": "https://github.com/microsoft/playwright-python.git",
            "tag": "v1.58.0",
            "commit": UPSTREAM_COMMIT,
            "coreProtocolCommit": CORE_COMMIT,
            "requiresBrowser": False,
            "fixtureAdapted": True,
            "testSourceAdapted": False,
            "promotableToOfficialLedger": False,
            "candidateAbi": 125,
            "candidateSelectedFeatures": list(FEATURES),
            "candidateWheelSha256": WHEEL_SHA256,
            "testBodies": list(TEST_BODIES),
            "engines": list(ENGINES),
            "expandedCaseIds": list(EXPECTED_IDS),
            "upstreamSourceSha256": SOURCE_HASHES,
            "toolingVersions": TOOLING,
        },
        "synthetic Python lifecycle manifest changed",
    )
    ws = subprocess.check_output(
        [
            "node",
            "-p",
            f"require.resolve('ws', {{paths: [{str(CORE)!r}]}})",
        ],
        text=True,
    ).strip()
    ws_path = Path(ws).resolve()
    _require(
        ws_path.is_file() and ws_path.is_relative_to(CORE),
        "Core protocol ws dependency moved",
    )
    return {"wsModulePath": str(ws_path), "wsModuleSha256": sha256(ws_path)}


def _package_tree_sha256(root: Path) -> str:
    _require(root.is_dir() and not root.is_symlink(), f"package tree missing: {root}")
    digest = hashlib.sha256()
    for path in sorted(root.rglob("*")):
        if "__pycache__" in path.parts or not path.is_file():
            continue
        _require(not path.is_symlink(), f"package source is symlinked: {path}")
        digest.update(path.relative_to(root).as_posix().encode())
        digest.update(bytes.fromhex(sha256(path)))
    return digest.hexdigest()


def _verify_wheel_install(wheel: Path, site_packages: Path) -> dict[str, str]:
    _require(sha256(wheel) == WHEEL_SHA256, "frozen selected wheel hash changed")
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
                f"frozen wheel file missing: {name}",
            )
            digest = hashlib.sha256(archive.read(name)).hexdigest()
            _require(
                sha256(target) == digest,
                f"installed candidate differs from wheel: {name}",
            )
            installed[name] = digest
    _require(
        "pwright/_native.abi3.so" in installed and "pwright/async_api.py" in installed,
        "selected wheel lacks remote facade or native addon",
    )
    return installed


def _junit(path: Path, engine: str) -> dict[str, Any]:
    _require(path.is_file() and not path.is_symlink(), f"missing JUnit: {path}")
    root = ET.parse(path).getroot()
    suite = root.find("testsuite") if root.tag == "testsuites" else root
    _require(suite is not None and suite.tag == "testsuite", "invalid JUnit suite")
    cases = list(suite.findall("testcase"))
    expected = {case_id for case_id in EXPECTED_IDS if case_id.endswith(f"[{engine}]")}
    actual: dict[str, str] = {}
    for case in cases:
        classname = case.attrib.get("classname", "")
        api = (
            "async"
            if ".async." in f".{classname}."
            else "sync"
            if ".sync." in f".{classname}."
            else None
        )
        _require(api is not None, f"unrecognized JUnit class: {classname}")
        case_id = f"tests/{api}/test_browsertype_connect.py::{case.attrib.get('name', '')}"
        _require(case_id not in actual, "duplicate JUnit case")
        if case.find("skipped") is not None:
            status = "skipped"
        elif case.find("failure") is not None or case.find("error") is not None:
            status = "failed"
        else:
            status = "passed"
        actual[case_id] = status
    _require(
        set(actual) == expected and len(cases) == 4,
        "JUnit selection differs from four pinned cases",
    )
    _require(
        all(status != "skipped" for status in actual.values()),
        "synthetic diagnostic skipped a case",
    )
    passed = sum(status == "passed" for status in actual.values())
    _require(int(suite.attrib.get("tests", "-1")) == 4, "JUnit test count changed")
    _require(
        int(suite.attrib.get("failures", "-1")) + int(suite.attrib.get("errors", "-1"))
        == 4 - passed,
        "JUnit failure accounting changed",
    )
    return {"cases": actual, "passed": passed}


def _receipt(
    path: Path,
    *,
    mode: str,
    engine: str,
    expected: dict[str, str],
    bind: str,
    host: str,
    result_dir: Path,
) -> dict[str, int]:
    _require(path.is_file() and not path.is_symlink(), f"missing receipt: {path}")
    records = [json.loads(line) for line in path.read_text().splitlines() if line]
    _require(
        records and all(isinstance(item, dict) for item in records),
        "invalid receipt JSONL",
    )
    _require(
        all(
            item.get("mode") == mode
            and item.get("fixtureHost") == host
            and item.get("fixtureBind") == bind
            for item in records
        ),
        "mixed receipt lane or address",
    )
    by_kind = {
        kind: [record for record in records if record.get("kind") == kind]
        for kind in (
            "runtime-import",
            "fixture-adapted",
            "selected",
            "fixture-created",
            "fixture-stopped",
            "case-call",
            "listening",
            "upgrade",
            "root-initialize",
            "bootstrap-sent",
            "close",
            "stop-requested",
            "stop",
            "fixture-premature-exit",
            "websocket-error",
            "start-error",
        )
    }
    _require(
        len(by_kind["runtime-import"])
        == len(by_kind["fixture-adapted"])
        == len(by_kind["selected"])
        == 1,
        "fixture/import/selection receipt missing",
    )
    _require(
        not any(
            by_kind[kind] for kind in ("fixture-premature-exit", "websocket-error", "start-error")
        ),
        "synthetic fixture reported an error",
    )
    adapted = by_kind["fixture-adapted"][0]
    _require(
        adapted.get("fixture") == "launch_server"
        and adapted.get("fixtureBindingIdentity") is True
        and adapted.get("fixtureAdapted") is True
        and adapted.get("testSourceAdapted") is False
        and adapted.get("browserProcessLaunched") is False
        and adapted.get("cdpEndpointPresent") is False
        and adapted.get("upstreamCommit") == UPSTREAM_COMMIT
        and adapted.get("upstreamFixtureSha256") == SOURCE_HASHES["tests/conftest.py"],
        "source-pinned launch_server substitution absent",
    )
    _require(
        set(by_kind["selected"][0].get("caseIds", [])) == set(expected),
        "selected case receipt changed",
    )
    calls = by_kind["case-call"]
    _require(
        len(calls) == 4 and {item.get("caseId") for item in calls} == set(expected),
        "case-call receipt is incomplete or retried",
    )
    _require(
        all(item.get("outcome") == expected[item["caseId"]] for item in calls),
        "case-call outcome differs from JUnit",
    )
    paths = by_kind["runtime-import"][0]
    site_packages = result_dir / "venv/lib/python3.12/site-packages"
    if mode == "oracle":
        _require(
            paths.get("candidatePackagePath") is None
            and str(paths.get("publicApiPath", "")).startswith(str(site_packages / "playwright/")),
            "oracle did not import the independently installed Playwright package",
        )
    else:
        _require(
            paths.get("publicApiPath") == str((SHIM / "playwright/async_api.py").resolve())
            and paths.get("candidatePackagePath")
            == str((site_packages / "pwright/__init__.py").resolve()),
            "candidate did not import frozen wheel through public shim",
        )
    _require(
        len(by_kind["fixture-created"]) == len(by_kind["fixture-stopped"]) == 4,
        "four isolated synthetic fixture lifetimes not observed",
    )
    pids = {item.get("serverPid") for item in by_kind["fixture-created"]}
    _require(
        len(pids) == 4 and pids == {item.get("serverPid") for item in by_kind["fixture-stopped"]},
        "synthetic fixture process reuse or teardown drifted",
    )
    _require(
        all(
            item.get("engine") == engine
            and item.get("fixtureAdapted") is True
            and item.get("testSourceAdapted") is False
            and item.get("browserProcessLaunched") is False
            for item in by_kind["fixture-created"]
        ),
        "fixture creation did not preserve adaptation boundary",
    )
    _require(
        all(item.get("returnCode") == 0 for item in by_kind["fixture-stopped"]),
        "synthetic server teardown was not clean",
    )
    synthetic = [item for item in records if item.get("synthetic") is True]
    _require(
        synthetic
        and all(
            item.get("browserProcessLaunched") is False and item.get("engine") == engine
            for item in synthetic
        ),
        "synthetic receipt claimed browser launch or wrong engine",
    )
    _require(
        len(by_kind["listening"]) == len(by_kind["stop-requested"]) == len(by_kind["stop"]) == 4,
        "synthetic server lifecycle receipt missing",
    )
    _require(
        {item.get("pid") for item in by_kind["listening"]} == pids,
        "synthetic listener does not match fixture process",
    )
    _require(
        all(
            item.get("endpointPath") == "/browser-synthetic-python" for item in by_kind["listening"]
        ),
        "synthetic WebSocket path drifted",
    )
    passed = sum(status == "passed" for status in expected.values())
    _require(
        len(by_kind["upgrade"]) == len(by_kind["root-initialize"]) == len(by_kind["bootstrap-sent"])
        and passed <= len(by_kind["bootstrap-sent"]) <= 4,
        "successful cases lack matching Root.initialize bootstrap receipts",
    )
    _require(
        all(item.get("browserHeader") == engine for item in by_kind["upgrade"]),
        "browser engine header changed",
    )
    _require(
        all(
            item.get("sdkLanguage") == "python" and item.get("id") == 1
            for item in by_kind["root-initialize"]
        ),
        "Root.initialize command changed",
    )
    _require(
        all(
            item.get("frames") == 8 and item.get("version") == "155.0"
            for item in by_kind["bootstrap-sent"]
        ),
        "synthetic protocol graph changed",
    )
    clean_close_pids = {
        item.get("pid") for item in by_kind["close"] if item.get("forcedByFixtureTeardown") is False
    }
    _require(
        len(clean_close_pids) >= passed,
        "successful cases lack distinct clean WS close receipts",
    )
    return {kind: len(items) for kind, items in by_kind.items() if items}


def verify_directory(result_dir: Path) -> dict[str, Any]:
    _require(not result_dir.is_symlink(), "result destination is symlinked")
    result_dir = result_dir.resolve()
    _require(
        result_dir.is_dir() and result_dir.parent == RESULT_PARENT.resolve(),
        "result directory is outside the diagnostic parent",
    )
    source = verify_sources()
    wheel = result_dir / WHEEL_NAME
    _require(wheel.is_file() and not wheel.is_symlink(), "frozen wheel missing")
    site_packages = result_dir / "venv/lib/python3.12/site-packages"
    installed = _verify_wheel_install(wheel, site_packages)
    native = site_packages / "pwright/_native.abi3.so"
    oracle_tree = _package_tree_sha256(site_packages / "playwright")
    tooling_freeze = result_dir / "tooling-freeze.txt"
    _require(
        tooling_freeze.is_file() and not tooling_freeze.is_symlink(),
        "pinned tooling freeze missing",
    )
    frozen_versions = dict(
        line.split("==", maxsplit=1)
        for line in tooling_freeze.read_text().splitlines()
        if "==" in line
    )
    _require(
        all(
            frozen_versions.get(name) == version
            for name, version in TOOLING.items()
            if name != "python"
        ),
        "pinned Python tooling versions drifted",
    )
    python = result_dir / "venv/bin/python"
    runtime = subprocess.check_output(
        [
            str(python),
            "-c",
            "import json, pwright; "
            "print(json.dumps(pwright.native_capabilities(), sort_keys=True))",
        ],
        text=True,
    )
    capabilities = json.loads(runtime)
    _require(
        capabilities.get("abi") == 125
        and capabilities.get("features") == list(FEATURES)
        and capabilities.get("profile") == "+".join(FEATURES),
        "selected wheel ABI or exact feature closure changed",
    )
    lanes: dict[str, Any] = {}
    bind: str | None = None
    host: str | None = None
    for mode in ("oracle", "candidate"):
        cases: dict[str, str] = {}
        engine_results: dict[str, Any] = {}
        for engine in ENGINES:
            prefix = f"{mode}-{engine}"
            xml = result_dir / f"{prefix}.xml"
            receipt = result_dir / f"{prefix}.receipt.jsonl"
            log = result_dir / f"{prefix}.log"
            provenance_path = result_dir / f"{prefix}.provenance.json"
            provenance = _json(provenance_path)
            _require(log.is_file() and not log.is_symlink(), "pytest log missing")
            junit = _junit(xml, engine)
            _require(
                provenance.get("schema") == 1
                and provenance.get("mode") == mode
                and provenance.get("engine") == engine
                and provenance.get("xmlSha256") == sha256(xml)
                and provenance.get("receiptSha256") == sha256(receipt)
                and provenance.get("logSha256") == sha256(log)
                and provenance.get("exitCode") == (0 if junit["passed"] == 4 else 1),
                "lane provenance or pytest exit differs from artifacts",
            )
            command = provenance.get("command")
            _require(
                isinstance(command, list)
                and command[:3] == [str(python), "-m", "pytest"]
                and f"--browser={engine}" in command
                and f"--junitxml={xml}" in command
                and set(command[-4:])
                == {
                    str(UPSTREAM / body.split("::", maxsplit=1)[0])
                    + "::"
                    + body.split("::", maxsplit=1)[1]
                    for body in TEST_BODIES
                },
                "pytest command changed selected test bodies",
            )
            environment = provenance.get("environment")
            _require(
                isinstance(environment, dict)
                and environment.get("PWRIGHT_OFFICIAL_LAUNCH_MODE") == "owned"
                and environment.get("PWRIGHT_PYTHON_LIFECYCLE_MODE") == mode
                and environment.get("PWRIGHT_PYTHON_LIFECYCLE_ENGINE_FILTER") == engine
                and environment.get("PWRIGHT_PYTHON_LIFECYCLE_RECEIPT") == str(receipt)
                and environment.get("PWRIGHT_PYTHON_LIFECYCLE_SERVER") == str(SERVER)
                and environment.get("PWRIGHT_PYTHON_LIFECYCLE_UPSTREAM") == str(UPSTREAM)
                and environment.get("PWRIGHT_PYTHON_LIFECYCLE_CORE_UPSTREAM") == str(CORE),
                "synthetic fixture environment changed",
            )
            if bind is None:
                bind = environment.get("PWRIGHT_OFFICIAL_TAILSCALE_IP")
                host = environment.get("PWRIGHT_OFFICIAL_TAILSCALE_HOST")
            _require(
                environment.get("PWRIGHT_OFFICIAL_TAILSCALE_IP") == bind
                and environment.get("PWRIGHT_OFFICIAL_TAILSCALE_HOST") == host
                and isinstance(bind, str)
                and bind.startswith("100.")
                and isinstance(host, str)
                and host.endswith(".ts.net"),
                "paired fixture address drifted",
            )
            expected_pythonpath = (
                [str(SHIM), str(RUNTIME), str(PLUGIN_DIR)]
                if mode == "candidate"
                else [str(RUNTIME), str(PLUGIN_DIR)]
            )
            _require(
                environment.get("PYTHONPATH") == ":".join(expected_pythonpath),
                "oracle/candidate import isolation changed",
            )
            counts = _receipt(
                receipt,
                mode=mode,
                engine=engine,
                expected=junit["cases"],
                bind=bind,
                host=host,
                result_dir=result_dir,
            )
            cases.update(junit["cases"])
            engine_results[engine] = {
                "passed": junit["passed"],
                "xmlSha256": sha256(xml),
                "receiptSha256": sha256(receipt),
                "logSha256": sha256(log),
                "provenanceSha256": sha256(provenance_path),
                "wireReceiptCounts": counts,
            }
        _require(
            set(cases) == set(EXPECTED_IDS),
            "lane did not cover all twelve exact engine IDs",
        )
        passed = sum(status == "passed" for status in cases.values())
        if mode == "oracle":
            _require(
                passed == 12,
                "independent Playwright oracle did not pass all twelve assertions",
            )
        lanes[mode] = {"passed": passed, "cases": cases, "engines": engine_results}
    return {
        "schema": 1,
        "classification": "official-derived-fixture-adapted-internal-diagnostic",
        "publicApiParityEvidence": False,
        "unadaptedOfficialPass": False,
        "realPlaywrightServerSupport": False,
        "promotedToPythonLedger": False,
        "upstreamCommit": UPSTREAM_COMMIT,
        "coreProtocolCommit": CORE_COMMIT,
        "upstreamSourceSha256": SOURCE_HASHES,
        "manifestSha256": sha256(MANIFEST),
        "pluginSha256": sha256(PLUGIN),
        "syntheticServerSha256": sha256(SERVER),
        "wsModulePath": source["wsModulePath"],
        "wsModuleSha256": source["wsModuleSha256"],
        "wheelSha256": sha256(wheel),
        "installedNativeSha256": sha256(native),
        "installedWheelFilesSha256": installed,
        "oraclePackageTreeSha256": oracle_tree,
        "toolingVersions": TOOLING,
        "toolingFreezeSha256": sha256(tooling_freeze),
        "candidateCapabilities": capabilities,
        "fixtureBind": bind,
        "fixtureHost": host,
        "lanes": lanes,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("result_dir", type=Path)
    args = parser.parse_args()
    attestation = verify_directory(args.result_dir)
    saved_path = args.result_dir / "attestation.json"
    if saved_path.is_file():
        saved = _json(saved_path)
        expected = dict(attestation)
        expected["harnessInputs"] = {
            str(path.relative_to(ROOT)): sha256(path) for path in HARNESS_INPUTS
        }
        _require(
            saved == expected,
            "saved attestation differs from independently verified evidence",
        )
    print(
        "verified fixture-adapted Python connect lifecycle: "
        f"oracle {attestation['lanes']['oracle']['passed']}/12; "
        f"candidate {attestation['lanes']['candidate']['passed']}/12"
    )


if __name__ == "__main__":
    main()
