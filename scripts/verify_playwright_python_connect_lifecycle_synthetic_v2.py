#!/usr/bin/env python3
"""Independently verify the six-case, fixture-adapted Python state diagnostic."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path
from typing import Any

import verify_playwright_python_connect_lifecycle_synthetic_v1 as v1

ROOT = v1.ROOT
UPSTREAM = v1.UPSTREAM
CORE = v1.CORE
RESULT_PARENT = v1.RESULT_PARENT
RUNTIME = v1.RUNTIME
SHIM = v1.SHIM
LEGACY_PLUGIN_DIR = v1.PLUGIN_DIR
PLUGIN_DIR = ROOT / "packages/pwright-python/tests/official/connect_lifecycle_synthetic_v2"
PLUGIN = PLUGIN_DIR / "pwright_python_connect_lifecycle_synthetic_v2.py"
MANIFEST = PLUGIN_DIR / "manifest.json"
SERVER = v1.SERVER
WHEEL_NAME = v1.WHEEL_NAME
WHEEL_SHA256 = "32c414d3eb2a45417c4a3ba79705be96179dc2ec2dd20896cd0cda7d8169e3e8"
FUNCTION = "test_browser_type_connect_set_browser_connected_state"
ENGINES = v1.ENGINES
TEST_BODIES = tuple(
    f"tests/{api}/test_browsertype_connect.py::{FUNCTION}" for api in ("async", "sync")
)
EXPECTED_IDS = tuple(f"{body}[{engine}]" for engine in ENGINES for body in TEST_BODIES)
FIXTURE_SOURCE_HASHES = {
    "manifest.json": "43120394b192fd1e70ff1130620390ed692782f9c914873bb050306404b71d1d",
    "pwright_python_connect_lifecycle_synthetic_v1.py": (
        "fc4f4c4a612af65a8cb8c8ac5018c5c1f81eabe3914883d5a4778e1b6722dc5d"
    ),
    "server.cjs": "7bac458b95db9153e1feea7a4d4710d39c7ad7e5025e88ce0b2f69bed744d48b",
}
HARNESS_INPUTS = (
    MANIFEST,
    PLUGIN,
    LEGACY_PLUGIN_DIR / "manifest.json",
    LEGACY_PLUGIN_DIR / "pwright_python_connect_lifecycle_synthetic_v1.py",
    SERVER,
    RUNTIME / "pwright_official_server_runtime.py",
    RUNTIME / "pwright_official_oracle_runtime.py",
    SHIM / "pwright_candidate_runtime.py",
    SHIM / "playwright/async_api.py",
    SHIM / "playwright/sync_api.py",
    ROOT / "scripts/run_playwright_python_connect_lifecycle_synthetic_v2.py",
    Path(__file__).resolve(),
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def sha256(path: Path) -> str:
    return v1.sha256(path)


def read_json(path: Path) -> dict[str, Any]:
    require(path.is_file() and not path.is_symlink(), f"missing regular evidence: {path}")
    value = json.loads(path.read_text())
    require(isinstance(value, dict), f"evidence is not a JSON object: {path}")
    return value


def harness_hashes() -> dict[str, str]:
    return {str(path.relative_to(ROOT)): sha256(path) for path in HARNESS_INPUTS}


def verify_sources() -> dict[str, Any]:
    source = v1.verify_sources()
    for name, digest in FIXTURE_SOURCE_HASHES.items():
        path = LEGACY_PLUGIN_DIR / name
        require(path.is_file() and not path.is_symlink(), f"fixture source missing: {name}")
        require(sha256(path) == digest, f"fixture source drifted: {name}")
    manifest = read_json(MANIFEST)
    expected = {
        "schema": 2,
        "kind": "playwright-python-connect-lifecycle-synthetic-v2",
        "classification": "official-derived-fixture-adapted-internal-diagnostic",
        "repository": "https://github.com/microsoft/playwright-python.git",
        "tag": "v1.58.0",
        "commit": v1.UPSTREAM_COMMIT,
        "coreProtocolCommit": v1.CORE_COMMIT,
        "requiresBrowser": False,
        "fixtureAdapted": True,
        "adaptedFixture": "launch_server",
        "testSourceAdapted": False,
        "promotableToOfficialLedger": False,
        "candidateAbi": 125,
        "candidateSelectedFeatures": list(v1.FEATURES),
        "candidateWheelSha256": WHEEL_SHA256,
        "testBodies": list(TEST_BODIES),
        "engines": list(ENGINES),
        "expandedCaseIds": list(EXPECTED_IDS),
        "upstreamSourceSha256": v1.SOURCE_HASHES,
        "fixtureHarnessVersion": "connect_lifecycle_synthetic_v1",
        "fixtureSourceSha256": FIXTURE_SOURCE_HASHES,
        "toolingVersions": v1.TOOLING,
    }
    require(manifest == expected, "six-case lifecycle manifest changed")
    return source


def verify_wheel_install(wheel: Path, site_packages: Path) -> dict[str, str]:
    require(wheel.is_file() and not wheel.is_symlink(), "frozen selected wheel missing")
    require(sha256(wheel) == WHEEL_SHA256, "frozen selected wheel hash changed")
    installed: dict[str, str] = {}
    with zipfile.ZipFile(wheel) as archive:
        for name in archive.namelist():
            if not (name.startswith("pwright/") or name.startswith("pwright.libs/")):
                continue
            if name.endswith("/"):
                continue
            target = site_packages / name
            require(target.is_file() and not target.is_symlink(), f"wheel file missing: {name}")
            digest = hashlib.sha256(archive.read(name)).hexdigest()
            require(sha256(target) == digest, f"installed wheel file drifted: {name}")
            installed[name] = digest
    require(
        "pwright/_native.abi3.so" in installed and "pwright/async_api.py" in installed,
        "selected wheel lacks native extension or async facade",
    )
    return installed


def junit(path: Path, engine: str) -> dict[str, Any]:
    require(path.is_file() and not path.is_symlink(), f"missing JUnit: {path}")
    root = ET.parse(path).getroot()
    suite = root.find("testsuite") if root.tag == "testsuites" else root
    require(suite is not None and suite.tag == "testsuite", "invalid JUnit suite")
    cases = list(suite.findall("testcase"))
    expected = {case for case in EXPECTED_IDS if case.endswith(f"[{engine}]")}
    actual: dict[str, str] = {}
    for case in cases:
        classname = case.attrib.get("classname", "")
        api = "async" if ".async." in f".{classname}." else "sync" if ".sync." in f".{classname}." else None
        require(api is not None, f"unrecognized JUnit class: {classname}")
        case_id = f"tests/{api}/test_browsertype_connect.py::{case.attrib.get('name', '')}"
        require(case_id not in actual, "duplicate JUnit case")
        if case.find("skipped") is not None:
            outcome = "skipped"
        elif case.find("failure") is not None or case.find("error") is not None:
            outcome = "failed"
        else:
            outcome = "passed"
        actual[case_id] = outcome
    require(set(actual) == expected and len(cases) == 2, "JUnit selection differs from two pinned cases")
    require(all(value != "skipped" for value in actual.values()), "diagnostic skipped a case")
    passed = sum(value == "passed" for value in actual.values())
    require(int(suite.attrib.get("tests", "-1")) == 2, "JUnit test count changed")
    require(
        int(suite.attrib.get("failures", "-1")) + int(suite.attrib.get("errors", "-1"))
        == 2 - passed,
        "JUnit failure accounting changed",
    )
    return {"cases": actual, "passed": passed}


def receipt(
    path: Path,
    *,
    mode: str,
    engine: str,
    expected: dict[str, str],
    bind: str,
    host: str,
    result_dir: Path,
) -> dict[str, int]:
    require(path.is_file() and not path.is_symlink(), f"missing receipt: {path}")
    records = [json.loads(line) for line in path.read_text().splitlines() if line]
    require(records and all(isinstance(record, dict) for record in records), "invalid receipt JSONL")
    kinds = (
        "runtime-import", "fixture-adapted", "selected", "fixture-created",
        "fixture-stopped", "case-call", "listening", "upgrade", "root-initialize",
        "bootstrap-sent", "close", "stop-requested", "stop",
    )
    require(all(record.get("kind") in kinds for record in records), "unexpected receipt event")
    require(
        all(
            record.get("mode") == mode
            and record.get("fixtureHost") == host
            and record.get("fixtureBind") == bind
            for record in records
        ),
        "mixed receipt lane or fixture address",
    )
    by_kind = {kind: [record for record in records if record.get("kind") == kind] for kind in kinds}
    require(
        len(by_kind["runtime-import"]) == len(by_kind["fixture-adapted"]) == len(by_kind["selected"]) == 1,
        "runtime import, fixture adaptation, or selection missing",
    )
    imported = by_kind["runtime-import"][0]
    site_packages = result_dir / "venv/lib/python3.12/site-packages"
    if mode == "oracle":
        require(
            imported.get("candidatePackagePath") is None
            and str(imported.get("publicApiPath", "")).startswith(str(site_packages / "playwright/")),
            "oracle did not import the independent Playwright package",
        )
    else:
        require(
            imported.get("publicApiPath") == str((SHIM / "playwright/async_api.py").resolve())
            and imported.get("candidatePackagePath") == str((site_packages / "pwright/__init__.py").resolve()),
            "candidate did not import frozen wheel through public shim",
        )
    adapted = by_kind["fixture-adapted"][0]
    require(
        adapted.get("fixture") == "launch_server"
        and adapted.get("fixtureBindingIdentity") is True
        and adapted.get("fixtureAdapted") is True
        and adapted.get("testSourceAdapted") is False
        and adapted.get("browserProcessLaunched") is False
        and adapted.get("cdpEndpointPresent") is False
        and adapted.get("upstreamCommit") == v1.UPSTREAM_COMMIT
        and adapted.get("upstreamFixtureSha256") == v1.SOURCE_HASHES["tests/conftest.py"],
        "pinned launch_server adaptation absent",
    )
    require(
        by_kind["selected"][0].get("testSourceAdapted") is False
        and by_kind["selected"][0].get("caseIds") == sorted(expected),
        "selected cases changed",
    )
    calls = by_kind["case-call"]
    require(
        len(calls) == 2 and {item.get("caseId") for item in calls} == set(expected),
        "case-call receipt missing or retried",
    )
    require(
        all(item.get("outcome") == expected[item["caseId"]] for item in calls),
        "case-call outcome differs from JUnit",
    )
    created = by_kind["fixture-created"]
    stopped = by_kind["fixture-stopped"]
    pids = {item.get("serverPid") for item in created}
    require(
        len(created) == len(stopped) == len(pids) == 2
        and all(isinstance(pid, int) and pid > 0 for pid in pids)
        and {item.get("serverPid") for item in stopped} == pids,
        "two distinct fixture processes and teardowns not observed",
    )
    require(
        all(
            item.get("engine") == engine
            and item.get("fixtureAdapted") is True
            and item.get("testSourceAdapted") is False
            and item.get("browserProcessLaunched") is False
            and item.get("endpointPath") == "/browser-synthetic-python"
            for item in created
        ) and all(item.get("returnCode") == 0 for item in stopped),
        "fixture adaptation or teardown changed",
    )
    for kind in ("listening", "upgrade", "root-initialize", "bootstrap-sent", "close", "stop-requested", "stop"):
        events = by_kind[kind]
        require(len(events) == 2 and {item.get("pid") for item in events} == pids, f"{kind} process evidence incomplete")
        require(
            all(
                item.get("synthetic") is True
                and item.get("browserProcessLaunched") is False
                and item.get("engine") == engine
                for item in events
            ),
            f"{kind} claimed browser launch or wrong engine",
        )
    require(
        all(item.get("endpointPath") == "/browser-synthetic-python" for item in by_kind["listening"])
        and all(item.get("browserHeader") == engine and item.get("requestPath") == "/browser-synthetic-python" for item in by_kind["upgrade"])
        and all(item.get("id") == 1 and item.get("sdkLanguage") == "python" for item in by_kind["root-initialize"])
        and all(item.get("frames") == 8 and item.get("version") == "155.0" for item in by_kind["bootstrap-sent"])
        and all(item.get("forcedByFixtureTeardown") is False for item in by_kind["close"]),
        "synthetic protocol or clean close evidence changed",
    )
    for pid in pids:
        indices = [index for index, item in enumerate(records) if item.get("serverPid") == pid and item.get("kind") == "fixture-created"]
        starts = [index for index, item in enumerate(records) if item.get("serverPid") == pid and item.get("kind") == "fixture-stopped"]
        require(len(indices) == len(starts) == 1 and indices[0] < starts[0], "fixture process order changed")
        segment = records[indices[0]:starts[0] + 1]
        segment_calls = [item for item in segment if item.get("kind") == "case-call"]
        require(len(segment_calls) == 1, "fixture process does not own exactly one case-call")
        require(
            all(any(item.get("kind") == kind and item.get("pid") == pid for item in segment) for kind in ("upgrade", "root-initialize", "bootstrap-sent", "close", "stop")),
            "fixture process protocol chain incomplete",
        )
    return {kind: len(items) for kind, items in by_kind.items()}


def expected_command(python: Path, engine: str, xml: Path, mode: str) -> list[str]:
    return [
        str(python), "-m", "pytest", "-q", "--tb=short", "--override-ini", "addopts=",
        "--import-mode=importlib", "-c", str(UPSTREAM / "pyproject.toml"),
        "-p", "pwright_official_server_runtime", "-p", "pwright_python_connect_lifecycle_synthetic_v2",
        "-p", "pwright_candidate_runtime" if mode == "candidate" else "pwright_official_oracle_runtime",
        f"--browser={engine}", "--timeout=30", "--timeout-method=signal", f"--junitxml={xml}",
        *(str(UPSTREAM / body.split("::", 1)[0]) + "::" + body.split("::", 1)[1] for body in TEST_BODIES),
    ]


def verify_directory(result_dir: Path) -> dict[str, Any]:
    require(not result_dir.is_symlink(), "result destination is symlinked")
    result_dir = result_dir.resolve()
    require(
        result_dir.is_dir()
        and result_dir.parent == RESULT_PARENT.resolve()
        and result_dir.name.startswith("python-connect-lifecycle-synthetic-v2-"),
        "result directory is outside the v2 diagnostic parent",
    )
    source = verify_sources()
    wheel = result_dir / WHEEL_NAME
    site_packages = result_dir / "venv/lib/python3.12/site-packages"
    installed = verify_wheel_install(wheel, site_packages)
    native = site_packages / "pwright/_native.abi3.so"
    oracle_tree = v1._package_tree_sha256(site_packages / "playwright")
    freeze_path = result_dir / "tooling-freeze.txt"
    require(freeze_path.is_file() and not freeze_path.is_symlink(), "pinned tooling freeze missing")
    frozen_versions = dict(
        line.split("==", 1) for line in freeze_path.read_text().splitlines() if "==" in line
    )
    require(
        all(frozen_versions.get(name) == version for name, version in v1.TOOLING.items() if name != "python"),
        "pinned Python tooling versions drifted",
    )
    python = result_dir / "venv/bin/python"
    runtime = subprocess.check_output(
        [str(python), "-c", "import json, pwright; print(json.dumps(pwright.native_capabilities(), sort_keys=True))"],
        text=True,
    )
    capabilities = json.loads(runtime)
    require(
        capabilities.get("abi") == 125
        and capabilities.get("features") == list(v1.FEATURES)
        and capabilities.get("profile") == "+".join(v1.FEATURES),
        "selected wheel ABI or exact feature closure changed",
    )
    lanes: dict[str, Any] = {}
    fixture_address: tuple[str, str] | None = None
    for mode in ("oracle", "candidate"):
        cases: dict[str, str] = {}
        engines: dict[str, Any] = {}
        for engine in ENGINES:
            prefix = f"{mode}-{engine}"
            xml = result_dir / f"{prefix}.xml"
            receipt_path = result_dir / f"{prefix}.receipt.jsonl"
            log = result_dir / f"{prefix}.log"
            provenance_path = result_dir / f"{prefix}.provenance.json"
            provenance = read_json(provenance_path)
            require(log.is_file() and not log.is_symlink(), "pytest log missing")
            result = junit(xml, engine)
            require(
                provenance.get("schema") == 2
                and provenance.get("mode") == mode
                and provenance.get("engine") == engine
                and provenance.get("wheelSha256") == WHEEL_SHA256
                and provenance.get("xmlSha256") == sha256(xml)
                and provenance.get("receiptSha256") == sha256(receipt_path)
                and provenance.get("logSha256") == sha256(log)
                and provenance.get("exitCode") == (0 if result["passed"] == 2 else 1)
                and provenance.get("command") == expected_command(python, engine, xml, mode),
                "lane provenance, command, or artifacts differ",
            )
            environment = provenance.get("environment")
            require(isinstance(environment, dict), "lane environment missing")
            pair = (
                environment.get("PWRIGHT_OFFICIAL_TAILSCALE_IP"),
                environment.get("PWRIGHT_OFFICIAL_TAILSCALE_HOST"),
            )
            require(
                isinstance(pair[0], str) and pair[0].startswith("100.")
                and isinstance(pair[1], str) and pair[1].endswith(".ts.net"),
                "synthetic fixture address invalid",
            )
            if fixture_address is None:
                fixture_address = pair
            require(pair == fixture_address, "paired fixture address drifted")
            expected_pythonpath = [str(SHIM), str(RUNTIME), str(PLUGIN_DIR), str(LEGACY_PLUGIN_DIR)] if mode == "candidate" else [str(RUNTIME), str(PLUGIN_DIR), str(LEGACY_PLUGIN_DIR)]
            expected_environment = {
                "PYTHONPATH": ":".join(expected_pythonpath),
                "PWRIGHT_OFFICIAL_LAUNCH_MODE": "owned",
                "PWRIGHT_OFFICIAL_TAILSCALE_IP": pair[0],
                "PWRIGHT_OFFICIAL_TAILSCALE_HOST": pair[1],
                "PWRIGHT_PYTHON_LIFECYCLE_MODE": mode,
                "PWRIGHT_PYTHON_LIFECYCLE_ENGINE_FILTER": engine,
                "PWRIGHT_PYTHON_LIFECYCLE_RECEIPT": str(receipt_path),
                "PWRIGHT_PYTHON_LIFECYCLE_SERVER": str(SERVER),
                "PWRIGHT_PYTHON_LIFECYCLE_UPSTREAM": str(UPSTREAM),
                "PWRIGHT_PYTHON_LIFECYCLE_CORE_UPSTREAM": str(CORE),
            }
            require(environment == expected_environment, "oracle/candidate import or fixture isolation changed")
            counts = receipt(
                receipt_path, mode=mode, engine=engine, expected=result["cases"],
                bind=pair[0], host=pair[1], result_dir=result_dir,
            )
            cases.update(result["cases"])
            engines[engine] = {
                "passed": result["passed"],
                "xmlSha256": sha256(xml),
                "receiptSha256": sha256(receipt_path),
                "logSha256": sha256(log),
                "provenanceSha256": sha256(provenance_path),
                "wireReceiptCounts": counts,
            }
        require(set(cases) == set(EXPECTED_IDS), "lane did not cover all six exact case IDs")
        passed = sum(value == "passed" for value in cases.values())
        if mode == "oracle":
            require(passed == 6, "independent Playwright oracle did not pass all six assertions")
        lanes[mode] = {"passed": passed, "cases": cases, "engines": engines}
    require(fixture_address is not None, "fixture address absent")
    return {
        "schema": 2,
        "classification": "official-derived-fixture-adapted-internal-diagnostic",
        "fixtureAdapted": True,
        "testSourceAdapted": False,
        "publicApiParityEvidence": False,
        "unadaptedOfficialPass": False,
        "realPlaywrightServerSupport": False,
        "promotedToPythonLedger": False,
        "upstreamCommit": v1.UPSTREAM_COMMIT,
        "coreProtocolCommit": v1.CORE_COMMIT,
        "upstreamSourceSha256": v1.SOURCE_HASHES,
        "manifestSha256": sha256(MANIFEST),
        "pluginSha256": sha256(PLUGIN),
        "fixtureSourceSha256": FIXTURE_SOURCE_HASHES,
        "wsModulePath": source["wsModulePath"],
        "wsModuleSha256": source["wsModuleSha256"],
        "wheelSha256": sha256(wheel),
        "installedNativeSha256": sha256(native),
        "installedWheelFilesSha256": installed,
        "oraclePackageTreeSha256": oracle_tree,
        "toolingVersions": v1.TOOLING,
        "toolingFreezeSha256": sha256(freeze_path),
        "candidateCapabilities": capabilities,
        "fixtureBind": fixture_address[0],
        "fixtureHost": fixture_address[1],
        "lanes": lanes,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("result_dir", type=Path)
    args = parser.parse_args()
    attestation = verify_directory(args.result_dir)
    saved_path = args.result_dir / "attestation.json"
    if saved_path.is_file():
        expected = dict(attestation)
        expected["harnessInputs"] = harness_hashes()
        require(read_json(saved_path) == expected, "saved attestation differs from verified evidence")
    print(
        "verified fixture-adapted Python connect state: "
        f"oracle {attestation['lanes']['oracle']['passed']}/6; "
        f"candidate {attestation['lanes']['candidate']['passed']}/6"
    )


if __name__ == "__main__":
    main()
