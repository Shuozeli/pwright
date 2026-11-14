"""Independently verify the six-case Python sync network strict pair v10.

The first candidate capture is an infrastructure-invalid six-setup-error run.
Only the second candidate capture and the official Oracle capture replace the
six Chromium rows in each lane; all other 6,520 rows retain their v9 bytes.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import subprocess
import xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path
from typing import Any, NoReturn

import verify_playwright_python_navigation_recheck_strict_paired_v9 as prior
import verify_playwright_python_official_provenance as official

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / ".work/official-conformance"
BASE_DIR = prior.OUTPUT_DIR
OUTPUT_DIR = WORK / "outcomes/python-sync-network6-chromium-strict-paired-v10"
INVENTORY = prior.INVENTORY
MANIFEST = ROOT / "packages/pwright-python/tests/official/sync-network-full-v1-manifest.json"
UPSTREAM = prior.UPSTREAM
NETWORK_SOURCE = UPSTREAM / "tests/sync/test_network.py"
SOURCE_PINS = OUTPUT_DIR / "source-pins.json"
PROMOTER = ROOT / "scripts/promote_playwright_python_sync_network6_strict_paired_v10.py"
BASE_PROMOTER = ROOT / "scripts/promote_playwright_python_navigation_recheck_strict_paired_v9.py"
BASE_VERIFIER = ROOT / "scripts/verify_playwright_python_navigation_recheck_strict_paired_v9.py"
OFFICIAL_VERIFIER = ROOT / "scripts/verify_playwright_python_official_provenance.py"
MODES = ("candidate", "oracle")
RECORDS = 6_526
TARGET_INDEXES = (5899, 5902, 5905, 5908, 5911, 5914)
GENERATED = frozenset({"attempts", "durationSeconds", "error", "outcome", "resultFile"})
KIND = "playwright-python-sync-network6-chromium-strict-paired-promotion-v10"
SCOPE = "tests/sync/test_network.py-chromium-manifest-six"
COMMIT = "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
INVENTORY_SHA256 = "3dfb11b6ad7eb4736166ee2c4f7fa32cda78063324ad8314ce12f3e2d49b9d41"
MANIFEST_SHA256 = "06665e258a533e542459b92f8d704d8fd5892b4b9ef61d2f0fd395639528b981"
NETWORK_SHA256 = "db831f837f94b2ee4eefb9dd6b271e7af42c573a59e4c3f49c20d83c572128ff"
BASE_RECEIPT_SHA256 = "42738259dba662f23f324451ad094d0f31f2d9137ad01b1e3df14cd599bc4763"
BASE_VERIFIER_SHA256 = "09f698b904ecabab2af2f74cd3d2d4164ee2152cbb8bf4205c0a6691fd095f39"
BASE_PROMOTER_SHA256 = "03564f6f093bb244c8a3cfce4626b4d5b7497b7d111de28cc0474896855daaa5"
OFFICIAL_VERIFIER_SHA256 = "cb451d9977ff574efb0778b842ff8662a95471fc2d1ca5dff450bef39c6e50d7"
BASE_HASHES = {
    "candidate": (
        "d7e2dcebe40d0646f6dc0ce215b584cc020743d7ce1def3fac0bb0a790e94756",
        "593c3a021dba06b004ff95fcf19716f33a9df50c7368a22decb8f6450733a92c",
    ),
    "oracle": (
        "5993775f11480d3e24ed6a56a4d9a7ccd4b3c4dd3722141ac4c27c3a4132907a",
        "3661a099ddeb9deb52b19c5481a354b342b1bbf4fd59157850596900fbe5c511",
    ),
}
REPORTS = {
    "candidateInfrastructureInvalid": WORK / "results/python-sync-network6-chromium-candidate-20261021-v1.xml",
    "candidateQualified": WORK / "results/python-sync-network6-chromium-candidate-20261021-v2.xml",
    "oracleQualified": WORK / "results/python-sync-network6-chromium-oracle-20261021-v1.xml",
}
REPORT_SHA256 = {
    "candidateInfrastructureInvalid": "5f90ecdfd98b3ff068120f68250ea13d03745c0aad7e9349be1cab4eb8e26dde",
    "candidateQualified": "31fe1f52deac24b50d6a0b5af13a413c9982c90e6c4543cd7acf78f4f2486a36",
    "oracleQualified": "5a4ddedfaefd3a8679c5a44a9e8b4a18b4988c8e697c51151acef8f98bebad0b",
}
PROVENANCE_SHA256 = {
    "candidateInfrastructureInvalid": "a63af56b5f5e983058fcba66bf758d2876756a5d524b1b371bb3e9059191b915",
    "candidateQualified": "a688269d4eeaa677a0f52e6027a2ded5910a942065e5cb20c6f0182c7a8af164",
    "oracleQualified": "cee708fae936064177018ca4951288ee557c15f296fda7168eec24f0cf493f7a",
}
EXPECTED_COUNTS = {
    "candidateInfrastructureInvalid": {"error": 6},
    "candidateQualified": {"passed": 1, "failed": 5},
    "oracleQualified": {"passed": 3, "failed": 3},
}
EXPECTED_SUCCESSOR_COUNTS = {
    "candidate": {"error": 3, "failed": 124, "not-run": 4711, "passed": 1651, "skipped": 37},
    "oracle": {"failed": 13, "not-run": 5040, "passed": 1436, "skipped": 37},
}
SHARED_RED = (
    "test_response_server_addr",
    "test_should_report_if_response_was_from_service_worker",
    "test_should_report_service_worker_request",
)
CANDIDATE_ONLY_RED = (
    "test_response_security_details",
    "test_response_security_details_none_without_https",
)
SHARED_GREEN = ("test_should_fulfill_with_global_fetch_result",)
SOURCE_PINS_SHA256 = "88981dad8637da9a0d3dbdd37423261cbb537e8b4848da754071edd64af850e5"
PROMOTER_SHA256 = "78adc236548188d3fb05fc4c56402db1dc1d4f61eb7d9f5d71a1f515b93fc98f"
ARCHIVE_FINGERPRINT = {"files": 59, "bytes": 41473811, "sha256": "6e1e294e10133bebc685213c376e16ebda94f7595baf7e605b0bd0dd49bc49d1"}


class SyncNetwork6ValidationError(ValueError):
    """A source, report, or successor differs from the sealed v10 evidence."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SyncNetwork6ValidationError(message)


def sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def sealed(path: Path, expected: str, label: str) -> bytes:
    require(len(expected) == 64 and all(c in "0123456789abcdef" for c in expected), f"{label} is unsealed")
    require(path.is_file() and not path.is_symlink(), f"{label} is missing or linked")
    raw = path.read_bytes()
    require(sha256(raw) == expected, f"{label} SHA-256 differs")
    return raw


def json_object(raw: bytes, label: str) -> dict[str, Any]:
    try:
        value = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise SyncNetwork6ValidationError(f"{label} is invalid JSON") from error
    require(isinstance(value, dict), f"{label} is not an object")
    return value


def jsonl(raw: bytes, label: str) -> tuple[list[bytes], list[dict[str, Any]]]:
    require(raw.endswith(b"\n"), f"{label} is not newline terminated")
    lines = raw.splitlines(keepends=True)
    require(len(lines) == RECORDS, f"{label} row count differs")
    return lines, [json_object(line, f"{label} row {index}") for index, line in enumerate(lines)]


def base_name(mode: str) -> str:
    require(mode in MODES, "unknown base lane")
    return f"playwright-python-{mode}-navigation-recheck-chromium-strict-paired-v9.jsonl"


def output_name(mode: str) -> str:
    require(mode in MODES, "unknown output lane")
    return f"playwright-python-{mode}-sync-network6-chromium-strict-paired-v10.jsonl"


def archive_fingerprint(root: Path | None = None) -> dict[str, Any]:
    """Fingerprint every file and directory, including ignored cache entries."""
    sources = {
        "pwright": (root / "pwright" if root else ROOT / "packages/pwright-python/python/pwright"),
        "shuozeli_pwright.dylibs": (root / "shuozeli_pwright.dylibs" if root else ROOT / "packages/pwright-python/python/shuozeli_pwright.dylibs"),
        "harness/shim": (root / "harness/shim" if root else ROOT / "packages/pwright-python/tests/official/shim"),
        "harness/runtime": (root / "harness/runtime" if root else ROOT / "packages/pwright-python/tests/official/runtime"),
    }
    entries: list[dict[str, object]] = []
    files = 0
    total_bytes = 0
    for label, source in sorted(sources.items()):
        require(source.is_dir() and not source.is_symlink(), f"archive source {label} is missing or linked")
        for path in sorted(source.rglob("*")):
            require(not path.is_symlink(), f"archive source {label} contains a symlink")
            relative = f"{label}/{path.relative_to(source).as_posix()}"
            if path.is_dir():
                entries.append({"path": relative, "type": "directory"})
            else:
                require(path.is_file(), f"archive source {label} contains a special entry")
                raw = path.read_bytes()
                entries.append({"path": relative, "type": "file", "sha256": sha256(raw), "bytes": len(raw)})
                files += 1
                total_bytes += len(raw)
    digest = sha256(json.dumps(entries, sort_keys=True, separators=(",", ":")).encode())
    return {"files": files, "bytes": total_bytes, "sha256": digest}


def _base() -> tuple[list[dict[str, Any]], dict[str, list[bytes]]]:
    require(Path(prior.__file__).resolve() == BASE_VERIFIER, "v9 verifier module differs")
    sealed(BASE_VERIFIER, BASE_VERIFIER_SHA256, "v9 verifier")
    sealed(BASE_PROMOTER, BASE_PROMOTER_SHA256, "v9 promoter")
    receipt = json_object(sealed(BASE_DIR / "promotion-receipt.json", BASE_RECEIPT_SHA256, "v9 receipt"), "v9 receipt")
    require(prior.verify_bundle(BASE_DIR) == receipt, "v9 source-bound chain differs")
    _, inventory = jsonl(sealed(INVENTORY, INVENTORY_SHA256, "profiled inventory"), "inventory")
    manifest = json_object(sealed(MANIFEST, MANIFEST_SHA256, "six-case manifest"), "manifest")
    sealed(NETWORK_SOURCE, NETWORK_SHA256, "pinned upstream sync network source")
    require(manifest.get("commit") == COMMIT and manifest.get("files", {}).get("tests/sync/test_network.py") == NETWORK_SHA256, "manifest upstream source differs")
    selections = [case.get("id") for case in manifest.get("cases", [])]
    require(len(selections) == len(set(selections)) == 6 and all(isinstance(value, str) and value.startswith("tests/sync/test_network.py::") for value in selections), "manifest six selections differ")
    targets = [inventory[index].get("id") for index in TARGET_INDEXES]
    require(set(targets) == {f"{selection}[chromium]" for selection in selections}, "six Chromium inventory IDs differ")
    require(all(inventory[index].get("sourceClassification") == "public-api-source" for index in TARGET_INDEXES), "non-public target")
    require(sum(isinstance(row.get("id"), str) and row["id"].startswith("tests/sync/test_network.py::") and row["id"].endswith("[chromium]") for row in inventory) == 6, "unaccounted sync network Chromium ID")
    require({value.rsplit("::", 1)[1] for value in targets} == {f"{name}[chromium]" for name in SHARED_RED + CANDIDATE_ONLY_RED + SHARED_GREEN}, "target names differ")
    originals: dict[str, list[bytes]] = {}
    for mode in MODES:
        path = BASE_DIR / base_name(mode)
        ledger_sha, summary_sha = BASE_HASHES[mode]
        lines, rows = jsonl(sealed(path, ledger_sha, f"{mode} v9 ledger"), f"{mode} v9 ledger")
        summary = json_object(sealed(path.with_suffix(".summary.json"), summary_sha, f"{mode} v9 summary"), f"{mode} v9 summary")
        require(receipt["successors"].get(mode) == summary, f"{mode} v9 receipt summary differs")
        require(all(row.get("id") == inventory[index].get("id") for index, row in enumerate(rows)), f"{mode} v9 order differs")
        require(all(rows[index].get("outcome") == "not-run" and rows[index].get("attempts") == 0 for index in TARGET_INDEXES), f"{mode} six-case baseline differs")
        originals[mode] = lines
    return inventory, originals


def _junit_cases(raw: bytes, key: str, selections: list[str]) -> dict[str, dict[str, object]]:
    try:
        root = ET.fromstring(raw)
    except ET.ParseError as error:
        raise SyncNetwork6ValidationError(f"{key} JUnit is invalid XML") from error
    suites = list(root.iter("testsuite"))
    require(len(suites) == 1, f"{key} JUnit suite count differs")
    suite = suites[0]
    cases = list(root.iter("testcase"))
    require(len(cases) == 6 and suite.get("tests") == "6", f"{key} JUnit case count differs")
    mapped: dict[str, dict[str, object]] = {}
    for node in cases:
        require(node.get("classname") == "tests.sync.test_network", f"{key} JUnit classname differs")
        name = node.get("name")
        require(isinstance(name, str) and name.endswith("[chromium]"), f"{key} JUnit browser differs")
        test_id = f"tests/sync/test_network.py::{name}"
        require(test_id not in mapped, f"{key} JUnit duplicate ID")
        try:
            duration = float(node.get("time", ""))
        except ValueError as error:
            raise SyncNetwork6ValidationError(f"{key} JUnit duration differs") from error
        require(math.isfinite(duration) and duration >= 0, f"{key} JUnit duration differs")
        terminal = [child for child in node if child.tag in {"failure", "error", "skipped"}]
        require(len(terminal) <= 1, f"{key} JUnit terminal count differs")
        outcome = {"failure": "failed", "error": "error", "skipped": "skipped"}.get(terminal[0].tag, "passed") if terminal else "passed"
        row: dict[str, object] = {"outcome": outcome, "durationSeconds": duration}
        if terminal:
            message = terminal[0].get("message")
            require(isinstance(message, str) and bool(message), f"{key} JUnit diagnostic missing")
            row["error"] = message
        mapped[test_id] = row
    require(list(mapped) == [f"{selection}[chromium]" for selection in selections], f"{key} JUnit IDs/order differ from manifest")
    require(dict(Counter(case["outcome"] for case in mapped.values())) == EXPECTED_COUNTS[key], f"{key} JUnit outcomes differ")
    require(suite.get("errors") == str(EXPECTED_COUNTS[key].get("error", 0)) and suite.get("failures") == str(EXPECTED_COUNTS[key].get("failed", 0)) and suite.get("skipped") == "0", f"{key} JUnit summary differs")
    if key == "candidateInfrastructureInvalid":
        require(all(str(case["error"]).startswith('failed on setup with "_native.Error: Topology initialization failed:') and "Page-lane limit of 8 was exceeded" in str(case["error"]) for case in mapped.values()), "invalid run was not the six Page-lane setup errors")
    return mapped


def _reports(archive_root: Path | None = None) -> tuple[dict[str, dict[str, Any]], dict[str, dict[str, dict[str, object]]]]:
    require(Path(official.__file__).resolve() == OFFICIAL_VERIFIER, "official verifier module differs")
    sealed(OFFICIAL_VERIFIER, OFFICIAL_VERIFIER_SHA256, "official provenance verifier")
    manifest = json_object(sealed(MANIFEST, MANIFEST_SHA256, "six-case manifest"), "manifest")
    selections = [case["id"] for case in manifest["cases"]]
    payloads: dict[str, dict[str, Any]] = {}
    results: dict[str, dict[str, dict[str, object]]] = {}
    for key, result in REPORTS.items():
        mode = "oracle" if key == "oracleQualified" else "candidate"
        provenance = result.with_suffix(".provenance.json")
        payload = json_object(sealed(provenance, PROVENANCE_SHA256[key], f"{key} provenance"), f"{key} provenance")
        raw = sealed(result, REPORT_SHA256[key], f"{key} JUnit")
        try:
            official.verify_provenance(
                payload,
                provenance_path=provenance,
                harness_root=archive_root / "harness" if archive_root else None,
                package_root=archive_root / "pwright" if archive_root and mode == "candidate" else None,
            )
        except (OSError, ValueError, subprocess.CalledProcessError) as error:
            raise SyncNetwork6ValidationError(f"{key} source-bound provenance failed: {error}") from error
        expected = {
            "schema": 5,
            "mode": mode,
            "repository": "https://github.com/microsoft/playwright-python.git",
            "tag": "v1.58.0",
            "commit": COMMIT,
            "manifest": str(MANIFEST),
            "manifestSha256": MANIFEST_SHA256,
            "upstream": str(UPSTREAM),
            "scope": "manifest-cases-sync",
            "manifestCases": True,
            "manifestApi": "sync",
            "browsers": ["chromium"],
            "selections": selections,
            "launchMode": "cdp-adapter",
            "endpoint": "http://127.0.0.1:9222",
            "fixtureBind": "100.78.97.63",
            "fixtureHost": "macbookpro-yuacx.tail8f3b66.ts.net",
            "secureHttpFixture": False,
            "caseTimeout": 45.0,
            "isolateCases": False,
            "resumeIsolated": False,
            "bootstrapOnly": False,
            "collectOnly": False,
            "inventory": None,
            "inventorySha256": None,
            "caseReports": None,
            "caseReportsSha256": None,
            "result": str(result),
            "resultSha256": REPORT_SHA256[key],
            "exitCode": 1,
        }
        require(all(payload.get(field) == value for field, value in expected.items()), f"{key} provenance identity differs")
        if mode == "candidate":
            require(payload.get("playwrightPackageSha256") == "e3a7333b66af39f60b8fc68ea8ff177604a914ac2380a99f17ad596451c886e9" and payload.get("pwrightPackageSha256") == "ae49b06f267c68194edfe268a3f1ca19debbd9bb8895286acd9871f789dfa482" and payload.get("pwrightArtifactSha256") == "0e97fef9a014a3ec3c86d22b377e5a265e4a5cea093379960f5d60ab0d846b09", "candidate runtime identity differs")
        else:
            require(payload.get("playwrightPackageSha256") == "d7655707035107f13a81db0e1d52d839c2bf42bf2008478fba610299cd6da149" and payload.get("pwrightPackage") is None and payload.get("pwrightArtifact") is None, "Oracle runtime identity differs")
        payloads[key] = payload
        results[key] = _junit_cases(raw, key, selections)
    require(payloads["candidateInfrastructureInvalid"]["pwrightPackageSha256"] == payloads["candidateQualified"]["pwrightPackageSha256"] and payloads["candidateInfrastructureInvalid"]["pwrightArtifactSha256"] == payloads["candidateQualified"]["pwrightArtifactSha256"], "candidate retry runtime changed")
    return payloads, results


def paired_classification(results: dict[str, dict[str, dict[str, object]]]) -> dict[str, list[str]]:
    candidate = results["candidateQualified"]
    oracle = results["oracleQualified"]
    expected = {
        "sharedRed": [f"tests/sync/test_network.py::{name}[chromium]" for name in SHARED_RED],
        "candidateOnlyRed": [f"tests/sync/test_network.py::{name}[chromium]" for name in CANDIDATE_ONLY_RED],
        "sharedGreen": [f"tests/sync/test_network.py::{name}[chromium]" for name in SHARED_GREEN],
        "oracleOnlyRed": [],
    }
    require(set(candidate) == set(oracle) == set(sum(expected.values(), [])), "paired report IDs differ")
    require(all(candidate[test_id]["outcome"] == oracle[test_id]["outcome"] == "failed" for test_id in expected["sharedRed"]), "shared RED classification differs")
    require(all(candidate[test_id]["outcome"] == "failed" and oracle[test_id]["outcome"] == "passed" for test_id in expected["candidateOnlyRed"]), "candidate-only RED classification differs")
    require(all(candidate[test_id]["outcome"] == oracle[test_id]["outcome"] == "passed" for test_id in expected["sharedGreen"]), "shared GREEN classification differs")
    return expected


def source_pins_expected(payloads: dict[str, dict[str, Any]]) -> dict[str, Any]:
    return {
        "schema": 1,
        "kind": "playwright-python-sync-network6-strict-source-pins-v10",
        "upstreamCommit": COMMIT,
        "upstreamSyncNetworkSha256": NETWORK_SHA256,
        "manifestSha256": MANIFEST_SHA256,
        "inventorySha256": INVENTORY_SHA256,
        "baseReceiptSha256": BASE_RECEIPT_SHA256,
        "archiveFingerprint": ARCHIVE_FINGERPRINT,
        "reports": {key: {"resultSha256": REPORT_SHA256[key], "provenanceSha256": PROVENANCE_SHA256[key]} for key in REPORTS},
        "candidatePackageSha256": payloads["candidateQualified"]["pwrightPackageSha256"],
        "candidateArtifactSha256": payloads["candidateQualified"]["pwrightArtifactSha256"],
        "candidateShimSha256": payloads["candidateQualified"]["playwrightPackageSha256"],
        "oraclePlaywrightSha256": payloads["oracleQualified"]["playwrightPackageSha256"],
        "harnessInputs": payloads["candidateQualified"]["harnessInputs"],
        "toolingSources": payloads["candidateQualified"]["toolingSources"],
        "pwrightRuntimeLibraries": payloads["candidateQualified"]["pwrightRuntimeLibraries"],
    }


def check_reconciliation(base_lines: list[bytes], successor_lines: list[bytes], inventory: list[dict[str, Any]], cases: dict[str, dict[str, object]], mode: str) -> Counter[str]:
    require(mode in MODES and len(base_lines) == len(successor_lines) == len(inventory) == RECORDS, f"{mode} ledger shape differs")
    targets = set(TARGET_INDEXES)
    require(set(cases) == {inventory[index]["id"] for index in targets}, f"{mode} target set differs")
    changed = 0
    counts: Counter[str] = Counter()
    for index, (before_raw, after_raw, source) in enumerate(zip(base_lines, successor_lines, inventory)):
        before = json_object(before_raw, f"{mode} v9 row {index}")
        after = json_object(after_raw, f"{mode} v10 row {index}")
        require(before.get("id") == after.get("id") == source.get("id"), f"{mode} row ID moved at {index}")
        require({key: value for key, value in before.items() if key not in GENERATED} == source and {key: value for key, value in after.items() if key not in GENERATED} == source, f"{mode} row metadata differs at {index}")
        if index in targets:
            require(before.get("outcome") == "not-run" and before.get("attempts") == 0, f"{mode} baseline target differs at {index}")
            case = cases[source["id"]]
            expected: dict[str, object] = {**source, "attempts": 1, "durationSeconds": case["durationSeconds"], "outcome": case["outcome"], "resultFile": str(REPORTS[f"{mode}Qualified"])}
            if "error" in case:
                expected["error"] = case["error"]
            canonical = (json.dumps(expected, sort_keys=True, separators=(",", ":")) + "\n").encode()
            require(after == expected and after_raw == canonical and after_raw != before_raw, f"{mode} target row differs at {index}")
        else:
            require(after_raw == before_raw, f"{mode} off-target raw bytes changed at {index}")
        changed += after_raw != before_raw
        outcome = after.get("outcome")
        require(isinstance(outcome, str), f"{mode} malformed outcome at {index}")
        counts[outcome] += 1
    require(changed == 6 and dict(counts) == EXPECTED_SUCCESSOR_COUNTS[mode], f"{mode} changed-row count or aggregate differs")
    return counts


def report_receipt(key: str) -> dict[str, Any]:
    return {
        "result": str(REPORTS[key]),
        "resultSha256": REPORT_SHA256[key],
        "provenance": str(REPORTS[key].with_suffix(".provenance.json")),
        "provenanceSha256": PROVENANCE_SHA256[key],
        "observedOutcomes": EXPECTED_COUNTS[key],
        "promoted": key != "candidateInfrastructureInvalid",
    }


def receipt_expected(summaries: dict[str, dict[str, Any]], classification: dict[str, list[str]], pins_sha: str) -> dict[str, Any]:
    return {
        "schema": 2,
        "kind": KIND,
        "scope": SCOPE,
        "publicApiParityEvidence": False,
        "fullCorpusExecution": False,
        "completeManifestExecution": True,
        "fixtureAdapted": True,
        "launchMode": "cdp-adapter",
        "candidateQualifiedAttemptsPerTarget": 1,
        "candidateInfrastructureInvalidRuns": 1,
        "infrastructureInvalidReason": "six Page-lane limit 8 setup errors before test bodies",
        "sharedRedRootCausesEstablished": False,
        "environmentLimitations": ["server_addr Oracle source asserts loopback while this lane binds the Tailscale IP", "service-worker Oracle runs use insecure MagicDNS HTTP/CDP attachment; secure-context support is not established"],
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "recordsPerLane": RECORDS,
        "targetZeroBasedIndexes": list(TARGET_INDEXES),
        "targetTestIds": [f"tests/sync/test_network.py::{name}[chromium]" for name in ("test_response_security_details", "test_response_security_details_none_without_https", "test_response_server_addr", "test_should_fulfill_with_global_fetch_result", "test_should_report_if_response_was_from_service_worker", "test_should_report_service_worker_request")],
        "manifest": str(MANIFEST),
        "manifestSha256": MANIFEST_SHA256,
        "upstream": str(UPSTREAM),
        "upstreamCommit": COMMIT,
        "upstreamSyncNetworkSha256": NETWORK_SHA256,
        "sourcePins": str(SOURCE_PINS),
        "sourcePinsSha256": pins_sha,
        "promoterSha256": PROMOTER_SHA256,
        "baseVerifierSha256": BASE_VERIFIER_SHA256,
        "basePromoterSha256": BASE_PROMOTER_SHA256,
        "baseReceipt": str(BASE_DIR / "promotion-receipt.json"),
        "baseReceiptSha256": BASE_RECEIPT_SHA256,
        "archive": {
            "root": str(OUTPUT_DIR / "archive"),
            "candidatePackage": "archive/pwright",
            "candidateNativeArtifact": "archive/pwright/_native.abi3.so",
            "candidateRuntimeLibraries": "archive/shuozeli_pwright.dylibs",
            "officialHarness": "archive/harness",
            "candidatePackageSha256": "ae49b06f267c68194edfe268a3f1ca19debbd9bb8895286acd9871f789dfa482",
            "candidateNativeArtifactSha256": "0e97fef9a014a3ec3c86d22b377e5a265e4a5cea093379960f5d60ab0d846b09",
            "fingerprint": ARCHIVE_FINGERPRINT,
        },
        "bases": {mode: {"ledger": str(BASE_DIR / base_name(mode)), "ledgerSha256": BASE_HASHES[mode][0], "summary": str((BASE_DIR / base_name(mode)).with_suffix(".summary.json")), "summarySha256": BASE_HASHES[mode][1]} for mode in MODES},
        "reports": {key: report_receipt(key) for key in REPORTS},
        "pairedClassification": classification,
        "successors": summaries,
    }


def verify_bundle(output_dir: Path = OUTPUT_DIR) -> dict[str, Any]:
    require(output_dir == OUTPUT_DIR and output_dir.is_dir() and not output_dir.is_symlink(), "v10 output directory missing or moved")
    expected_files = {"promotion-receipt.json", "source-pins.json", "archive"} | {name for mode in MODES for name in (output_name(mode), Path(output_name(mode)).with_suffix(".summary.json").name)}
    require({path.name for path in output_dir.iterdir()} == expected_files, "v10 output file set differs")
    require(all((output_dir / name).is_file() and not (output_dir / name).is_symlink() for name in expected_files - {"archive"}), "v10 output file missing or linked")
    archive = output_dir / "archive"
    require(archive.is_dir() and not archive.is_symlink() and {path.name for path in archive.iterdir()} == {"pwright", "shuozeli_pwright.dylibs", "harness"}, "v10 archive shape differs")
    require({path.name for path in (archive / "harness").iterdir()} == {"shim", "runtime"}, "v10 harness archive shape differs")
    require(archive_fingerprint(archive) == ARCHIVE_FINGERPRINT, "v10 archive file fingerprint differs")
    inventory, base_lines = _base()
    payloads, results = _reports(archive)
    classification = paired_classification(results)
    pins = source_pins_expected(payloads)
    require(json_object(sealed(SOURCE_PINS, SOURCE_PINS_SHA256, "v10 source pins"), "source pins") == pins, "v10 source pins differ")
    sealed(PROMOTER, PROMOTER_SHA256, "v10 publisher")
    summaries: dict[str, dict[str, Any]] = {}
    for mode in MODES:
        output = output_dir / output_name(mode)
        raw = output.read_bytes()
        lines, _ = jsonl(raw, f"{mode} v10 ledger")
        counts = check_reconciliation(base_lines[mode], lines, inventory, results[f"{mode}Qualified"], mode)
        expected_summary = {"schema": 1, "inventory": str(INVENTORY), "records": RECORDS, "outcomes": dict(sorted(counts.items())), "ledgerSha256": sha256(raw), "ledger": str(output)}
        summary = json_object(output.with_suffix(".summary.json").read_bytes(), f"{mode} v10 summary")
        require(summary == expected_summary, f"{mode} v10 summary differs")
        summaries[mode] = summary
    receipt = json_object((output_dir / "promotion-receipt.json").read_bytes(), "v10 receipt")
    require(receipt == receipt_expected(summaries, classification, SOURCE_PINS_SHA256), "v10 receipt differs")
    return receipt


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify-existing", action="store_true")
    args = parser.parse_args()
    if not args.verify_existing:
        parser.error("pass --verify-existing for read-only verification")
    try:
        receipt = verify_bundle()
    except (OSError, SyncNetwork6ValidationError, prior.NavigationRecheckValidationError, ET.ParseError) as error:
        parser.error(str(error))
    print(json.dumps({"verified": str(OUTPUT_DIR), "successors": receipt["successors"]}, sort_keys=True))


if __name__ == "__main__":
    main()
