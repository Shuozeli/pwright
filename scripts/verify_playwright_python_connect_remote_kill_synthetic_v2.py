#!/usr/bin/env python3
"""Independently attest the fixed-wheel, repeated Python remote-kill diagnostic."""

from __future__ import annotations

import argparse
import hashlib
import ipaddress
import json
import zipfile
from pathlib import Path
from typing import Any

import verify_playwright_python_connect_remote_kill_synthetic_v1 as prior

ROOT = prior.ROOT
RESULT_PARENT = prior.RESULT_PARENT
MANIFEST = ROOT / "packages/pwright-python/tests/official/connect_remote_kill_synthetic_v2/manifest.json"
BUILD_WHEEL = ROOT / ".work/official-conformance/builds/python-remote-terminal-fix-oct20" / prior.WHEEL_NAME
PREDECESSOR = RESULT_PARENT / "python-connect-remote-kill-synthetic-v1-oct20-final"
PREDECESSOR_SHA256 = "6d75f73094c46e7127ee87cc7319bd4e1edadde2986dc4505601aadb37590a59"
WHEEL_NAME = prior.WHEEL_NAME
WHEEL_SHA256 = "1f9836dcc8548b9938c6acdf10b10faf49c757b71b8a284d764e7efd8f7c05c0"
RESULT_PREFIX = "python-connect-remote-kill-synthetic-v2-terminal-fix-"
CASE_IDS = prior.CASE_IDS
CANDIDATE_REPEATS = 5
SOURCE_HASHES = {
    "packages/pwright-python/python/pwright/__init__.py": "ee807562416e961ed784e9b480f0685703e71ca288fd6652ed05bc6365908cb7",
    "packages/pwright-python/python/pwright/async_api.py": "270493c6d5b4bd1956d85d7b040af23fbb5a1792a88e9084cac3a253925972e2",
    "packages/pwright-python/python/pwright/sync_api.py": "8da383a74361e56b9ab6c24a791e30aa65ebdd921f03c316dd799897e3198f52",
    "crates/pwright-node-addon/src/remote_browser.rs": "e5167167e3d58b09cfbbcded9d20b5573150947913877e52a8dad20087d153c8",
    "crates/pwright-playwright-server-client-core/src/lib.rs": "eaaa55a8d424efcc7d8e5db25e22ab4295f63910f809ad1bb2095cd5eb0ad349",
}
HARNESS_INPUTS = tuple(dict.fromkeys((
    *prior.HARNESS_INPUTS,
    MANIFEST,
    ROOT / "scripts/run_playwright_python_connect_remote_kill_synthetic_v2.py",
    ROOT / "scripts/test_verify_playwright_python_connect_remote_kill_synthetic_v2.py",
    Path(__file__).resolve(),
)))


def harness_hashes() -> dict[str, str]:
    for path in HARNESS_INPUTS:
        prior.require(path.is_file() and not path.is_symlink(), f"harness input missing: {path}")
    return {str(path.relative_to(ROOT)): prior.sha256(path) for path in HARNESS_INPUTS}


def verify_sources() -> dict[str, str]:
    ledger_hashes = prior.verify_sources()
    old_attestation = PREDECESSOR / "attestation.json"
    prior.require(old_attestation.is_file() and not old_attestation.is_symlink()
                  and prior.sha256(old_attestation) == PREDECESSOR_SHA256,
                  "predecessor RED attestation changed")
    old_verified = prior.verify_directory(PREDECESSOR)
    prior.require(prior.read_json(old_attestation) == old_verified,
                  "predecessor RED evidence no longer verifies")
    prior.require(old_verified["lanes"]["oracle"]["passed"] == 4
                  and old_verified["lanes"]["candidate"]["passed"] == 2,
                  "predecessor RED baseline changed")
    for relative, digest in SOURCE_HASHES.items():
        source = ROOT / relative
        prior.require(source.is_file() and not source.is_symlink()
                      and prior.sha256(source) == digest,
                      f"fixed-wheel source changed: {relative}")
    expected = {
        "schema": 1,
        "kind": "playwright-python-connect-remote-kill-synthetic-v2-terminal-fix",
        "classification": "official-derived-fixture-adapted-internal-diagnostic",
        "repository": "https://github.com/microsoft/playwright-python.git",
        "tag": "v1.58.0", "commit": prior.UPSTREAM_COMMIT,
        "coreProtocolCommit": prior.CORE_COMMIT,
        "requiresBrowser": False, "browserProcessLaunched": False,
        "fixtureAdapted": True, "adaptedFixture": "launch_server",
        "testSourceAdapted": False, "publicApiParityEvidence": False,
        "unadaptedOfficialPass": False, "promotableToOfficialLedger": False,
        "promotedToPythonLedger": False, "candidateAbi": 125,
        "candidateWheelSha256": WHEEL_SHA256,
        "predecessorRedAttestationSha256": PREDECESSOR_SHA256,
        "predecessorRedWheelSha256": prior.WHEEL_SHA256,
        "engines": ["chromium"], "candidateRepeats": CANDIDATE_REPEATS,
        "expandedCaseIds": list(CASE_IDS),
        "ledgerZeroBasedIndices": [808, 823, 4969, 4978],
        "candidateSourceSha256": SOURCE_HASHES,
        "toolingVersions": prior.TOOLING,
    }
    prior.require(prior.read_json(MANIFEST) == expected, "fixed-wheel manifest changed")
    prior.require(BUILD_WHEEL.is_file() and not BUILD_WHEEL.is_symlink()
                  and prior.sha256(BUILD_WHEEL) == WHEEL_SHA256,
                  "requested fixed-wheel build changed")
    return ledger_hashes


def verify_wheel_install(wheel: Path, site_packages: Path) -> dict[str, str]:
    prior.require(wheel.is_file() and not wheel.is_symlink()
                  and prior.sha256(wheel) == WHEEL_SHA256,
                  "fixed candidate wheel changed")
    installed: dict[str, str] = {}
    with zipfile.ZipFile(wheel) as archive:
        for name in archive.namelist():
            if not name.startswith(("pwright/", "pwright.libs/")) or name.endswith("/"):
                continue
            target = site_packages / name
            prior.require(target.is_file() and not target.is_symlink(),
                          f"fixed wheel install file missing: {name}")
            digest = hashlib.sha256(archive.read(name)).hexdigest()
            prior.require(prior.sha256(target) == digest,
                          f"installed fixed wheel differs from archive: {name}")
            installed[name] = digest
        for source_relative, digest in SOURCE_HASHES.items():
            if not source_relative.startswith("packages/pwright-python/python/pwright/"):
                continue
            archive_name = source_relative.removeprefix("packages/pwright-python/python/")
            prior.require(archive_name in installed and installed[archive_name] == digest,
                          f"fixed wheel Python source mismatch: {archive_name}")
    prior.require(all(name in installed for name in (
        "pwright/_native.abi3.so", "pwright/async_api.py", "pwright/sync_api.py")),
        "fixed wheel missing required API/native artifacts")
    return installed


def verify_tooling(result_dir: Path, wheel: Path) -> dict[str, str]:
    install_log = result_dir / "install.log"
    freeze = result_dir / "tooling-freeze.txt"
    for path in (install_log, freeze):
        prior.require(path.is_file() and not path.is_symlink() and path.stat().st_size > 0,
                      f"tooling evidence missing: {path}")
    frozen = freeze.read_text().splitlines()
    for name, version in prior.TOOLING.items():
        if name != "python":
            prior.require(f"{name}=={version}" in frozen,
                          f"tooling version changed: {name}")
    prior.require(f"shuozeli-pwright @ file://{wheel}" in frozen,
                  "installed candidate wheel source changed")
    log = install_log.read_text()
    prior.require("playwright==1.58.0" in log and "shuozeli-pwright==0.1.0" in log,
                  "tooling installation log changed")
    return {"install.log": prior.sha256(install_log),
            "tooling-freeze.txt": prior.sha256(freeze)}


def lane_labels() -> tuple[str, ...]:
    return ("oracle", *(f"candidate-{index}" for index in range(1, CANDIDATE_REPEATS + 1)))


def verify_directory(result_dir: Path) -> dict[str, Any]:
    prior.require(not result_dir.is_symlink(), "result directory is symlinked")
    result_dir = result_dir.resolve()
    prior.require(result_dir.is_dir() and result_dir.parent == RESULT_PARENT.resolve()
                  and result_dir.name.startswith(RESULT_PREFIX),
                  "fixed-wheel result directory is outside versioned parent")
    ledger_hashes = verify_sources()
    harness = harness_hashes()
    wheel = result_dir / WHEEL_NAME
    site_packages = result_dir / "venv/lib/python3.12/site-packages"
    installed = verify_wheel_install(wheel, site_packages)
    tooling_hashes = verify_tooling(result_dir, wheel)
    lanes: dict[str, Any] = {}
    for label in lane_labels():
        mode = "oracle" if label == "oracle" else "candidate"
        provenance_path = result_dir / f"{label}.provenance.json"
        provenance = prior.read_json(provenance_path)
        xml = result_dir / f"{label}.xml"
        log = result_dir / f"{label}.log"
        receipt_path = result_dir / f"{label}.receipt.jsonl"
        prior.require(provenance.get("schema") == 1 and provenance.get("mode") == mode
                      and provenance.get("laneLabel") == label
                      and provenance.get("wheelSha256") == WHEEL_SHA256
                      and provenance.get("sourceWheelPath") == str(BUILD_WHEEL),
                      f"{label} provenance identity changed")
        prior.require(provenance.get("command") == prior.expected_command(
            result_dir / "venv/bin/python", xml, mode),
            f"{label} pytest command changed")
        env = provenance.get("environment")
        prior.require(isinstance(env, dict), f"{label} environment absent")
        bind = str(env.get("PWRIGHT_OFFICIAL_TAILSCALE_IP", ""))
        host = str(env.get("PWRIGHT_OFFICIAL_TAILSCALE_HOST", ""))
        prior.require(ipaddress.ip_address(bind) in ipaddress.ip_network("100.64.0.0/10")
                      and host.endswith(".ts.net"), f"{label} Tailscale address changed")
        prior.require(env.get("PWRIGHT_PYTHON_CLOSE_EVENTS_MODE") == mode
                      and env.get("PWRIGHT_PYTHON_CLOSE_EVENTS_ENGINE_FILTER") == "chromium"
                      and env.get("PWRIGHT_PYTHON_CLOSE_EVENTS_RECEIPT") == str(receipt_path)
                      and env.get("PWRIGHT_PYTHON_CLOSE_EVENTS_SERVER") == str(prior.SERVER)
                      and env.get("PWRIGHT_PYTHON_CLOSE_EVENTS_UPSTREAM") == str(prior.UPSTREAM)
                      and env.get("PWRIGHT_PYTHON_CLOSE_EVENTS_CORE_UPSTREAM") == str(prior.CORE),
                      f"{label} fixture environment changed")
        prior.require(env.get("PWRIGHT_OFFICIAL_LAUNCH_MODE") == "owned"
                      and env.get("PWRIGHT_CDP_ENDPOINT") is None
                      and env.get("PWRIGHT_CDP_WS_URL") is None,
                      f"{label} inherited browser/CDP setting")
        paths = str(env.get("PYTHONPATH", "")).split(":")
        expected_paths = [str(prior.SHIM), str(prior.RUNTIME), str(prior.PLUGIN_DIR),
                          str(prior.LEGACY_DIR)] if mode == "candidate" else [
                              str(prior.RUNTIME), str(prior.PLUGIN_DIR), str(prior.LEGACY_DIR)]
        prior.require(paths == expected_paths, f"{label} public API import isolation changed")
        for path in (xml, log, receipt_path):
            prior.require(path.is_file() and not path.is_symlink(), f"{label} raw evidence missing")
        hashes = {suffix: prior.sha256(result_dir / f"{label}.{suffix}")
                  for suffix in ("xml", "log", "receipt.jsonl")}
        prior.require(provenance.get("fileSha256") == hashes,
                      f"{label} raw evidence hash changed")
        outcomes = prior.junit(xml)
        passed = sum(outcome == "passed" for outcome in outcomes.values())
        prior.require(provenance.get("exitCode") == (0 if passed == 4 else 1),
                      f"{label} exit code/JUnit mismatch")
        receipt_counts = prior.receipt(receipt_path, mode, outcomes, bind, host, site_packages)
        prior.require(mode != "oracle" or passed == 4,
                      "official oracle did not validate synthetic fixture")
        lanes[label] = {
            "cases": outcomes, "passed": passed,
            "receiptCounts": receipt_counts, "fileSha256": hashes,
            "provenanceSha256": prior.sha256(provenance_path),
        }
    candidate_vectors = [tuple(lanes[f"candidate-{index}"]["cases"][case]
                               for case in CASE_IDS)
                         for index in range(1, CANDIDATE_REPEATS + 1)]
    return {
        "schema": 1,
        "classification": "official-derived-fixture-adapted-internal-diagnostic",
        "fixtureAdapted": True, "testSourceAdapted": False,
        "requiresBrowser": False, "browserProcessLaunched": False,
        "publicApiParityEvidence": False, "unadaptedOfficialPass": False,
        "promotedToPythonLedger": False, "candidateAbi": 125,
        "candidateWheelSha256": WHEEL_SHA256,
        "sourceWheelPath": str(BUILD_WHEEL),
        "predecessorRedAttestationSha256": PREDECESSOR_SHA256,
        "upstreamCommit": prior.UPSTREAM_COMMIT,
        "coreProtocolCommit": prior.CORE_COMMIT,
        "caseIds": list(CASE_IDS), "ledgerZeroBasedIndices": [808, 823, 4969, 4978],
        "candidateRepeats": CANDIDATE_REPEATS,
        "candidateOutcomeVariesAcrossRepeats": len(set(candidate_vectors)) > 1,
        "candidateSourceSha256": SOURCE_HASHES,
        "predecessorLedgerSha256": ledger_hashes,
        "harnessInputs": harness,
        "installedWheelFilesSha256": installed,
        "toolingEvidenceSha256": tooling_hashes,
        "lanes": lanes,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("result_dir", type=Path)
    args = parser.parse_args()
    verified = verify_directory(args.result_dir)
    attestation = args.result_dir / "attestation.json"
    if attestation.exists():
        prior.require(prior.read_json(attestation) == verified,
                      "saved fixed-wheel attestation differs from independent verification")
    scores = [verified["lanes"][label]["passed"] for label in lane_labels()]
    print(f"verified fixture-adapted remote kill: oracle {scores[0]}/4; "
          f"candidate repeats {scores[1:]}/4")


if __name__ == "__main__":
    main()
