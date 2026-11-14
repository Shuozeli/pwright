"""Independently verify the Python Chromium network50 strict paired v8 ledger.

Both official lanes replace exactly the 50 public-API network targets. The
other 6,476 raw rows per lane stay byte-identical to the sealed v7 pair.
Official failures, including three shared fixture-sensitive REDs, stay RED.
This verifier never imports the v8 publisher.
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

import verify_playwright_python_navigation_full_strict_paired_v6 as abi130
import verify_playwright_python_navigation_full_strict_paired_v7 as prior
import verify_playwright_python_official_provenance as official

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / ".work/official-conformance"
BASE_DIR = WORK / "outcomes/python-navigation-full-chromium-strict-paired-v7"
OUTPUT_DIR = WORK / "outcomes/python-network50-chromium-strict-paired-v8"
INVENTORY = prior.INVENTORY
MANIFEST = (
    ROOT
    / "packages/pwright-python/tests/official/async-network-full-chromium-v1-manifest.json"
)
UPSTREAM = prior.UPSTREAM
NETWORK_SOURCE = UPSTREAM / "tests/async/test_network.py"
SOURCE_PINS = OUTPUT_DIR / "source-pins.json"
PROMOTER = ROOT / "scripts/promote_playwright_python_network50_strict_paired_v8.py"
BASE_PROMOTER = (
    ROOT / "scripts/promote_playwright_python_navigation_full_strict_paired_v7.py"
)
BASE_VERIFIER = (
    ROOT / "scripts/verify_playwright_python_navigation_full_strict_paired_v7.py"
)
OFFICIAL_VERIFIER = ROOT / "scripts/verify_playwright_python_official_provenance.py"
REPORTS = {
    "candidate": WORK / "results/python-network50-candidate-abi130-v2-oct21.xml",
    "oracle": WORK / "results/python-network50-oracle-oct21.xml",
}
REPORT_SHA256 = {
    "candidate": "64c86efc4e2fae25d31a5fdb24807566e9dea7740858585cb3441c336862c4af",
    "oracle": "7441c48d3c1af0497a07349e16060e08266e088cc3203524ff7531bfd46efd28",
}
PROVENANCE_SHA256 = {
    "candidate": "7ef75c023ebd0237a0a6da48c7e1d12b6f44a2735ee5d670ea4e4638ecc7e2cf",
    "oracle": "22b92d94fa49886133b00109d8200c93c54146e763355755919e0df1ed7c21d4",
}
CASE_REPORTS_SHA256 = {
    "candidate": "17ceb0d4e6f676258727b1039a764a8c3fe78571ee35333d3e560bc6a61bbe9e",
    "oracle": "c0b39bd4ad1254fa95f4f536bf43a55048a398a79a079357d731be631efd9f65",
}
MODES = ("candidate", "oracle")
RECORDS = 6_526
TARGET_INDEXES = tuple(range(2920, 3070, 3))
GENERATED = frozenset({"attempts", "durationSeconds", "error", "outcome", "resultFile"})
KIND = "playwright-python-network50-chromium-strict-paired-promotion-v8"
SCOPE = "tests/async/test_network.py-chromium-manifest-public-api-50"
COMMIT = prior.COMMIT
INVENTORY_SHA256 = prior.INVENTORY_SHA256
MANIFEST_SHA256 = "f5063a1cba6afd0e07747919e2951ff333f28bc21fb42a0e11a49dcd1553e5ef"
NETWORK_SHA256 = "06ce41cadf74e6109d09fa33fb12a2a96f70b577f8657b198dbdc0567d480c2a"
BASE_VERIFIER_SHA256 = (
    "ec6fc58704c929ca6e806ceb047d43cf3fe3a7cca917b7db90e43501d0bac603"
)
BASE_PROMOTER_SHA256 = (
    "f8db0cfa62eb36f0ccc4355be4c785392233214d255004ce49092372eb461b80"
)
BASE_RECEIPT_SHA256 = "027c37acdfb4326458e96489d306bfa5a5058a6573d5184195f60774611b8944"
OFFICIAL_VERIFIER_SHA256 = prior.OFFICIAL_VERIFIER_SHA256
BASE_HASHES = {
    "candidate": (
        "ce203b6f91210c0121346370479d078673c108e7b323f149ea2fdd416de0a891",
        "4cb50d1b4a9f855e9450f3bcfa401a68484b33eade7071e3a6b355be54a5e547",
    ),
    "oracle": (
        "b209bf22f7777283798842abfd2a9774e4bb8f1659ae1b67e6015a3dc5a22479",
        "1e52ca657b520331dc27e39cd61bb52eb341d7b2903d96755780203dfe852a8b",
    ),
}
ISOLATED_IDS_SHA256 = "ae1750052439df99b85bbd7388b9d2b8a2180cfe5d892027b1ff7ac8ffbd393c"
ORACLE_PACKAGE = Path(
    "/Users/cyuan/projects/shuozeli/applications/pwright/.work/official-conformance/"
    "builds/python-navigation-full-abi127-oct20.bB5H3w/oracle/lib/python3.12/site-packages/playwright"
)
ORACLE_PYTHON = ORACLE_PACKAGE.parents[3] / "bin/python"
ORACLE_PACKAGE_SHA256 = (
    "d7655707035107f13a81db0e1d52d839c2bf42bf2008478fba610299cd6da149"
)
EXPECTED_REPORT_COUNTS = {
    "candidate": {"passed": 35, "failed": 15},
    "oracle": {"passed": 47, "failed": 3},
}
EXPECTED_SUCCESSOR_COUNTS = {
    "candidate": {
        "error": 5,
        "failed": 119,
        "not-run": 4717,
        "passed": 1648,
        "skipped": 37,
    },
    "oracle": {"failed": 10, "not-run": 5046, "passed": 1433, "skipped": 37},
}
SOURCE_PINS_SHA256 = "e2ee0560ef130928d875c6c4882b3e23948dad3614dce569daf5e82c63627c68"
PROMOTER_SHA256 = "5a3445128e68468267bd8c604340b13500acf26b407e3a4383b0366d33a2a6a9"


class Network50ValidationError(ValueError):
    """The immutable v8 network evidence or successor differs."""


def fail(message: str) -> NoReturn:
    raise Network50ValidationError(message)


def require(condition: bool, message: str) -> None:
    if not condition:
        fail(message)


def sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def sealed(path: Path, expected_sha: str, label: str) -> bytes:
    require(
        len(expected_sha) == 64
        and expected_sha != "0" * 64
        and all(character in "0123456789abcdef" for character in expected_sha),
        f"{label} has an unsealed SHA-256 pin",
    )
    require(path.is_file() and not path.is_symlink(), f"{label} missing or linked")
    raw = path.read_bytes()
    require(sha256(raw) == expected_sha, f"{label} SHA-256 differs")
    return raw


def json_object(raw: bytes, label: str) -> dict[str, Any]:
    try:
        value = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise Network50ValidationError(f"{label} is invalid JSON") from error
    require(isinstance(value, dict), f"{label} is not an object")
    return value


def jsonl(raw: bytes, label: str) -> tuple[list[bytes], list[dict[str, Any]]]:
    require(raw.endswith(b"\n"), f"{label} is not newline terminated")
    lines = raw.splitlines(keepends=True)
    require(len(lines) == RECORDS, f"{label} row count differs")
    return lines, [
        json_object(line, f"{label} row {index}") for index, line in enumerate(lines)
    ]


def base_name(mode: str) -> str:
    require(mode in MODES, "unknown lane")
    return f"playwright-python-{mode}-navigation-full-chromium-strict-paired-v7.jsonl"


def output_name(mode: str) -> str:
    require(mode in MODES, "unknown lane")
    return f"playwright-python-{mode}-network50-chromium-strict-paired-v8.jsonl"


def check_receipt_scope(receipt: dict[str, Any]) -> None:
    require(
        type(receipt.get("schema")) is int and receipt["schema"] == 2,
        "receipt schema differs",
    )
    require(
        receipt.get("kind") == KIND and receipt.get("scope") == SCOPE,
        "receipt kind or scope differs",
    )
    for field, expected in (
        ("publicApiParityEvidence", False),
        ("fullCorpusExecution", False),
        ("completeFileExecution", False),
        ("completeManifestExecution", True),
        ("fixtureAdapted", True),
    ):
        require(receipt.get(field) is expected, f"receipt {field} differs")
    require(receipt.get("sharedFailedCases") == 3, "shared RED count differs")
    require(
        receipt.get("candidateOnlyFailedCases") == 12,
        "candidate-only RED count differs",
    )
    require(receipt.get("oracleOnlyFailedCases") == 0, "oracle-only RED count differs")
    require(receipt.get("launchMode") == "cdp-adapter", "receipt launch mode differs")


def _base() -> tuple[dict[str, Any], list[dict[str, Any]], dict[str, list[bytes]]]:
    require(
        Path(prior.__file__).resolve() == BASE_VERIFIER, "v7 verifier module differs"
    )
    sealed(BASE_VERIFIER, BASE_VERIFIER_SHA256, "v7 verifier source")
    sealed(BASE_PROMOTER, BASE_PROMOTER_SHA256, "v7 promoter source")
    receipt = json_object(
        sealed(BASE_DIR / "promotion-receipt.json", BASE_RECEIPT_SHA256, "v7 receipt"),
        "v7 receipt",
    )
    require(prior.verify_bundle(BASE_DIR) == receipt, "v7 source-bound chain differs")
    _, inventory = jsonl(
        sealed(INVENTORY, INVENTORY_SHA256, "profiled inventory"), "inventory"
    )
    targets = [inventory[index].get("id") for index in TARGET_INDEXES]
    require(
        len(targets) == len(set(targets)) == 50
        and all(
            isinstance(test_id, str)
            and test_id.startswith("tests/async/test_network.py::")
            and test_id.endswith("[chromium]")
            and inventory[index].get("sourceClassification") == "public-api-source"
            for index, test_id in zip(TARGET_INDEXES, targets)
        )
        and sum(
            isinstance(row.get("id"), str)
            and row["id"].startswith("tests/async/test_network.py::")
            and row["id"].endswith("[chromium]")
            for row in inventory
        )
        == 50,
        "50 Chromium network target IDs differ",
    )
    originals: dict[str, list[bytes]] = {}
    for mode in MODES:
        ledger = BASE_DIR / base_name(mode)
        ledger_sha, summary_sha = BASE_HASHES[mode]
        lines, rows = jsonl(
            sealed(ledger, ledger_sha, f"{mode} v7 ledger"), f"{mode} v7 ledger"
        )
        summary = json_object(
            sealed(
                ledger.with_suffix(".summary.json"), summary_sha, f"{mode} v7 summary"
            ),
            f"{mode} v7 summary",
        )
        require(
            receipt["successors"].get(mode) == summary, f"{mode} v7 summary differs"
        )
        require(
            all(
                row.get("id") == inventory[index].get("id")
                for index, row in enumerate(rows)
            ),
            f"{mode} row order differs",
        )
        prior_counts = Counter(rows[index].get("outcome") for index in TARGET_INDEXES)
        require(
            prior_counts
            == (
                {"not-run": 47, "passed": 3} if mode == "candidate" else {"not-run": 50}
            ),
            f"{mode} v7 network50 baseline differs",
        )
        originals[mode] = lines
    return receipt, inventory, originals


def check_report_metadata(
    payload: dict[str, Any], mode: str, targets: list[str]
) -> None:
    require(mode in MODES, "unknown report mode")
    report = REPORTS[mode]
    expected = {
        "schema": 5,
        "mode": mode,
        "repository": "https://github.com/microsoft/playwright-python.git",
        "tag": "v1.58.0",
        "commit": COMMIT,
        "manifest": str(MANIFEST),
        "manifestSha256": MANIFEST_SHA256,
        "upstream": str(UPSTREAM),
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "scope": "manifest-cases-all",
        "manifestCases": True,
        "manifestApi": None,
        "endpoint": "http://127.0.0.1:9222",
        "fixtureBind": "100.78.97.63",
        "fixtureHost": "macbookpro-yuacx.tail8f3b66.ts.net",
        "caseTimeout": 45.0,
        "isolatedCaseIdsSha256": ISOLATED_IDS_SHA256,
        "isolatedCaseCount": 50,
        "selections": [test_id.removesuffix("[chromium]") for test_id in targets],
        "browsers": ["chromium"],
        "launchMode": "cdp-adapter",
        "isolateCases": True,
        "resumeIsolated": False,
        "collectOnly": False,
        "bootstrapOnly": False,
        "secureHttpFixture": False,
        "result": str(report),
        "resultSha256": REPORT_SHA256[mode],
        "caseReports": str(Path(f"{report}.cases")),
        "caseReportsSha256": CASE_REPORTS_SHA256[mode],
        "exitCode": 1,
    }
    if mode == "candidate":
        expected.update(
            pythonExecutable=str(abi130.PYTHON_EXECUTABLE),
            playwrightPackage=str(
                ROOT / "packages/pwright-python/tests/official/shim/playwright"
            ),
            playwrightPackageSha256=abi130.PLAYWRIGHT_PACKAGE_SHA256,
            pwrightPackage=str(abi130.CANDIDATE_PACKAGE),
            pwrightPackageSha256=abi130.PWRIGHT_PACKAGE_SHA256,
            pwrightArtifact=str(abi130.CANDIDATE_ARTIFACT),
            pwrightArtifactSha256=abi130.PWRIGHT_ARTIFACT_SHA256,
        )
    else:
        expected.update(
            pythonExecutable=str(ORACLE_PYTHON),
            playwrightPackage=str(ORACLE_PACKAGE),
            playwrightPackageSha256=ORACLE_PACKAGE_SHA256,
            pwrightPackage=None,
            pwrightPackageSha256=None,
            pwrightArtifact=None,
            pwrightArtifactSha256=None,
        )
    require(
        all(payload.get(key) == value for key, value in expected.items()),
        f"{mode} source-bound report identity differs",
    )


def _junit_cases(
    raw: bytes, target_ids: list[str], mode: str
) -> dict[str, dict[str, object]]:
    try:
        tree = ET.fromstring(raw)
    except ET.ParseError as error:
        raise Network50ValidationError(
            f"{mode} network50 JUnit is invalid XML"
        ) from error
    cases = list(tree.iter("testcase"))
    require(len(cases) == 50, f"{mode} network50 JUnit does not have 50 cases")
    mapped: dict[str, dict[str, object]] = {}
    for case in cases:
        require(
            case.get("classname") == "tests.async.test_network",
            f"{mode} JUnit case is outside pinned source",
        )
        name = case.get("name")
        require(
            isinstance(name, str) and name.endswith("[chromium]"),
            f"{mode} JUnit case name differs",
        )
        test_id = f"tests/async/test_network.py::{name}"
        require(test_id not in mapped, f"{mode} JUnit duplicate case ID")
        try:
            duration = float(case.get("time", ""))
        except ValueError as error:
            raise Network50ValidationError(
                f"{mode} JUnit duration is invalid"
            ) from error
        require(
            math.isfinite(duration) and duration >= 0,
            f"{mode} JUnit duration is invalid",
        )
        terminals = [
            node
            for tag in ("failure", "error", "skipped")
            for node in case.findall(tag)
        ]
        require(
            len(terminals) <= 1 and (not terminals or terminals[0].tag == "failure"),
            f"{mode} JUnit terminal status differs",
        )
        row: dict[str, object] = {
            "outcome": "failed" if terminals else "passed",
            "durationSeconds": duration,
        }
        if terminals:
            message = (
                terminals[0].get("message") or terminals[0].text or ""
            ).splitlines()
            require(bool(message and message[0]), f"{mode} failure diagnostic missing")
            row["error"] = message[0]
        mapped[test_id] = row
    require(list(mapped) == target_ids, f"{mode} JUnit IDs/order differ from inventory")
    require(
        dict(Counter(row["outcome"] for row in mapped.values()))
        == EXPECTED_REPORT_COUNTS[mode],
        f"{mode} network50 observed outcomes differ",
    )
    return mapped


def _reports(
    target_ids: list[str],
) -> tuple[
    dict[str, dict[str, Any]],
    dict[str, dict[str, dict[str, object]]],
    dict[str, list[str]],
]:
    require(
        Path(official.__file__).resolve() == OFFICIAL_VERIFIER,
        "official provenance verifier module differs",
    )
    sealed(
        OFFICIAL_VERIFIER,
        OFFICIAL_VERIFIER_SHA256,
        "official provenance verifier source",
    )
    provenance: dict[str, dict[str, Any]] = {}
    cases: dict[str, dict[str, dict[str, object]]] = {}
    for mode in MODES:
        report = REPORTS[mode]
        sidecar = report.with_suffix(".provenance.json")
        payload = json_object(
            sealed(sidecar, PROVENANCE_SHA256[mode], f"{mode} provenance"),
            f"{mode} provenance",
        )
        raw = sealed(report, REPORT_SHA256[mode], f"{mode} network50 JUnit")
        try:
            official.verify_provenance(payload, provenance_path=sidecar)
        except (OSError, ValueError, subprocess.CalledProcessError) as error:
            raise Network50ValidationError(
                f"{mode} source-bound provenance failed: {error}"
            ) from error
        check_report_metadata(payload, mode, target_ids)
        provenance[mode] = payload
        cases[mode] = _junit_cases(raw, target_ids, mode)
    return provenance, cases, compare_cases(cases, target_ids)


def compare_cases(
    cases: dict[str, dict[str, dict[str, object]]], target_ids: list[str]
) -> dict[str, list[str]]:
    require(
        len(cases["candidate"]) == len(cases["oracle"]) == 50
        and set(cases["candidate"]) == set(cases["oracle"]) == set(target_ids),
        "paired network50 case ID sets differ",
    )
    shared = [
        test_id
        for test_id in target_ids
        if all(cases[mode][test_id]["outcome"] == "failed" for mode in MODES)
    ]
    candidate_only = [
        test_id
        for test_id in target_ids
        if cases["candidate"][test_id]["outcome"] == "failed"
        and cases["oracle"][test_id]["outcome"] == "passed"
    ]
    oracle_only = [
        test_id
        for test_id in target_ids
        if cases["candidate"][test_id]["outcome"] == "passed"
        and cases["oracle"][test_id]["outcome"] == "failed"
    ]
    require(
        len(shared) == 3 and len(candidate_only) == 12 and not oracle_only,
        "paired network50 RED attribution differs",
    )
    return {
        "shared": shared,
        "candidateOnly": candidate_only,
        "oracleOnly": oracle_only,
    }


def source_pins_expected(provenance: dict[str, dict[str, Any]]) -> dict[str, Any]:
    for mode in MODES:
        for field in ("harnessInputs", "toolingSources"):
            require(
                isinstance(provenance[mode].get(field), list)
                and bool(provenance[mode][field]),
                f"{mode} {field} missing",
            )
    require(
        isinstance(provenance["candidate"].get("pwrightRuntimeLibraries"), list)
        and bool(provenance["candidate"]["pwrightRuntimeLibraries"]),
        "candidate runtime libraries missing",
    )
    return {
        "schema": 1,
        "kind": "playwright-python-network50-strict-paired-v8",
        "upstreamCommit": COMMIT,
        "upstreamNetworkSha256": NETWORK_SHA256,
        "officialManifestSha256": MANIFEST_SHA256,
        "profiledInventorySha256": INVENTORY_SHA256,
        "chromiumExpandedCasesPerLane": 50,
        "candidateFrozenWheel": str(abi130.WHEEL),
        "candidateFrozenWheelSha256": abi130.WHEEL_SHA256,
        "candidateNativeAbi": 130,
        "candidatePackageSha256": abi130.PWRIGHT_PACKAGE_SHA256,
        "candidateArtifactSha256": abi130.PWRIGHT_ARTIFACT_SHA256,
        "candidatePlaywrightPackageSha256": abi130.PLAYWRIGHT_PACKAGE_SHA256,
        "oraclePlaywrightPackageSha256": ORACLE_PACKAGE_SHA256,
        "reports": {
            mode: {
                "result": str(REPORTS[mode]),
                "resultSha256": REPORT_SHA256[mode],
                "provenance": str(REPORTS[mode].with_suffix(".provenance.json")),
                "provenanceSha256": PROVENANCE_SHA256[mode],
                "caseReportsSha256": CASE_REPORTS_SHA256[mode],
                "harnessInputs": provenance[mode]["harnessInputs"],
                "toolingSources": provenance[mode]["toolingSources"],
            }
            for mode in MODES
        },
        "candidateRuntimeLibraries": provenance["candidate"]["pwrightRuntimeLibraries"],
    }


def check_reconciliation(
    base_lines: list[bytes],
    successor_lines: list[bytes],
    inventory: list[dict[str, Any]],
    cases: dict[str, dict[str, object]],
    mode: str,
) -> Counter[str]:
    require(mode in MODES, "unknown ledger mode")
    require(
        len(base_lines) == len(successor_lines) == len(inventory) == RECORDS,
        f"{mode} ledger row count differs",
    )
    require(
        set(cases) == {inventory[index].get("id") for index in TARGET_INDEXES},
        f"{mode} report target set differs",
    )
    targets = set(TARGET_INDEXES)
    changed = 0
    counts: Counter[str] = Counter()
    for index, (before_raw, after_raw, source) in enumerate(
        zip(base_lines, successor_lines, inventory)
    ):
        before = json_object(before_raw, f"{mode} v7 row {index}")
        after = json_object(after_raw, f"{mode} v8 row {index}")
        require(
            before.get("id") == after.get("id") == source.get("id"),
            f"{mode} row ID moved at {index}",
        )
        require(
            {key: value for key, value in before.items() if key not in GENERATED}
            == source,
            f"{mode} base metadata differs at {index}",
        )
        require(
            {key: value for key, value in after.items() if key not in GENERATED}
            == source,
            f"{mode} successor metadata differs at {index}",
        )
        if index in targets:
            require(
                (
                    before.get("outcome") in {"passed", "not-run"}
                    and before.get("attempts") in {0, 1}
                )
                and source.get("sourceClassification") == "public-api-source",
                f"{mode} prior target or classification differs at {index}",
            )
            case = cases[source["id"]]
            expected: dict[str, object] = {
                **source,
                "attempts": 1,
                "durationSeconds": case["durationSeconds"],
                "outcome": case["outcome"],
                "resultFile": str(REPORTS[mode]),
            }
            if "error" in case:
                expected["error"] = case["error"]
            canonical = (
                json.dumps(expected, sort_keys=True, separators=(",", ":")) + "\n"
            ).encode()
            require(
                after == expected
                and after_raw == canonical
                and after_raw != before_raw,
                f"{mode} target row differs at {index}",
            )
        else:
            require(
                after_raw == before_raw, f"{mode} off-target raw row changed at {index}"
            )
        if after_raw != before_raw:
            changed += 1
        outcome = after.get("outcome")
        require(isinstance(outcome, str), f"{mode} outcome malformed at {index}")
        counts[outcome] += 1
    require(changed == len(TARGET_INDEXES), f"{mode} changed-row count differs")
    require(
        dict(counts) == EXPECTED_SUCCESSOR_COUNTS[mode],
        f"{mode} aggregate outcomes differ",
    )
    return counts


def evidence() -> tuple[
    dict[str, Any],
    list[dict[str, Any]],
    dict[str, list[bytes]],
    dict[str, dict[str, Any]],
    dict[str, dict[str, dict[str, object]]],
    dict[str, list[str]],
]:
    base_receipt, inventory, base_lines = _base()
    targets = [inventory[index]["id"] for index in TARGET_INDEXES]
    provenance, cases, comparison = _reports(targets)
    try:
        abi130._verify_wheel(provenance["candidate"])
    except abi130.FullNavigationValidationError as error:
        raise Network50ValidationError(
            f"candidate ABI130 wheel failed: {error}"
        ) from error
    sealed(MANIFEST, MANIFEST_SHA256, "official manifest")
    sealed(NETWORK_SOURCE, NETWORK_SHA256, "upstream network source")
    return base_receipt, inventory, base_lines, provenance, cases, comparison


def verify_bundle(output_dir: Path = OUTPUT_DIR) -> dict[str, Any]:
    require(
        output_dir == OUTPUT_DIR
        and output_dir.is_dir()
        and not output_dir.is_symlink(),
        "v8 successor directory missing or moved",
    )
    expected_files = {"promotion-receipt.json", "source-pins.json"} | {
        name
        for mode in MODES
        for name in (
            output_name(mode),
            Path(output_name(mode)).with_suffix(".summary.json").name,
        )
    }
    require(
        {path.name for path in output_dir.iterdir()} == expected_files,
        "v8 successor file set differs",
    )
    require(
        all(
            (output_dir / name).is_file() and not (output_dir / name).is_symlink()
            for name in expected_files
        ),
        "v8 successor file missing or linked",
    )
    receipt = json_object(
        (output_dir / "promotion-receipt.json").read_bytes(), "v8 receipt"
    )
    check_receipt_scope(receipt)
    base_receipt, inventory, base_lines, provenance, cases, comparison = evidence()
    target_ids = [inventory[index]["id"] for index in TARGET_INDEXES]
    pins = json_object(
        sealed(SOURCE_PINS, SOURCE_PINS_SHA256, "v8 source pins"), "v8 source pins"
    )
    require(pins == source_pins_expected(provenance), "v8 source-pin identity differs")
    sealed(PROMOTER, PROMOTER_SHA256, "v8 promoter source")
    expected_fields = {
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "recordsPerLane": RECORDS,
        "targetZeroBasedIndexes": list(TARGET_INDEXES),
        "targetTestIds": target_ids,
        "manifest": str(MANIFEST),
        "manifestSha256": MANIFEST_SHA256,
        "upstream": str(UPSTREAM),
        "upstreamCommit": COMMIT,
        "upstreamNetworkSha256": NETWORK_SHA256,
        "sourcePins": str(SOURCE_PINS),
        "sourcePinsSha256": SOURCE_PINS_SHA256,
        "promoterSha256": PROMOTER_SHA256,
        "basePromoterSha256": BASE_PROMOTER_SHA256,
        "baseVerifierSha256": BASE_VERIFIER_SHA256,
        "baseReceipt": str(BASE_DIR / "promotion-receipt.json"),
        "baseReceiptSha256": BASE_RECEIPT_SHA256,
        "candidateFrozenWheel": str(abi130.WHEEL),
        "candidateFrozenWheelSha256": abi130.WHEEL_SHA256,
        "crossLaneComparison": comparison,
    }
    require(
        all(receipt.get(key) == value for key, value in expected_fields.items()),
        "v8 receipt source, target or RED comparison differs",
    )
    require(base_receipt.get("successors") is not None, "v7 receipt has no successors")
    require(
        isinstance(receipt.get("bases"), dict) and set(receipt["bases"]) == set(MODES),
        "v8 base lanes differ",
    )
    require(
        isinstance(receipt.get("reports"), dict)
        and set(receipt["reports"]) == set(MODES),
        "v8 report lanes differ",
    )
    require(
        isinstance(receipt.get("successors"), dict)
        and set(receipt["successors"]) == set(MODES),
        "v8 successor lanes differ",
    )
    for mode in MODES:
        report = REPORTS[mode]
        expected_report = {
            "provenance": str(report.with_suffix(".provenance.json")),
            "provenanceSha256": PROVENANCE_SHA256[mode],
            "result": str(report),
            "resultSha256": REPORT_SHA256[mode],
            "caseReportsSha256": CASE_REPORTS_SHA256[mode],
            "isolatedCaseIdsSha256": ISOLATED_IDS_SHA256,
            "observedOutcomes": EXPECTED_REPORT_COUNTS[mode],
        }
        require(
            receipt["reports"][mode] == expected_report,
            f"{mode} v8 receipt report differs",
        )
        base_ledger = BASE_DIR / base_name(mode)
        base_sha, base_summary_sha = BASE_HASHES[mode]
        require(
            receipt["bases"][mode]
            == {
                "ledger": str(base_ledger),
                "ledgerSha256": base_sha,
                "summary": str(base_ledger.with_suffix(".summary.json")),
                "summarySha256": base_summary_sha,
            },
            f"{mode} v8 receipt base differs",
        )
        output_ledger = output_dir / output_name(mode)
        raw = output_ledger.read_bytes()
        successor_lines, _ = jsonl(raw, f"{mode} v8 ledger")
        counts = check_reconciliation(
            base_lines[mode], successor_lines, inventory, cases[mode], mode
        )
        summary = json_object(
            output_ledger.with_suffix(".summary.json").read_bytes(),
            f"{mode} v8 summary",
        )
        expected_summary = {
            "schema": 1,
            "inventory": str(INVENTORY),
            "records": RECORDS,
            "outcomes": dict(sorted(counts.items())),
            "ledgerSha256": sha256(raw),
            "ledger": str(output_ledger),
        }
        require(
            summary == expected_summary and receipt["successors"][mode] == summary,
            f"{mode} v8 summary differs",
        )
    return receipt


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify-existing", action="store_true")
    args = parser.parse_args()
    if not args.verify_existing:
        parser.error("pass --verify-existing for read-only verification")
    try:
        receipt = verify_bundle()
    except (
        OSError,
        Network50ValidationError,
        prior.FullNavigationV7ValidationError,
        abi130.FullNavigationValidationError,
        ET.ParseError,
    ) as error:
        parser.error(str(error))
    print(
        json.dumps(
            {"verified": str(OUTPUT_DIR), "successors": receipt["successors"]},
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
