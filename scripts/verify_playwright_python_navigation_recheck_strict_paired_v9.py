"""Independently verify the ABI131 navigation80 recheck v9 paired ledger.

The immutable v8 pair is the base. The same ABI131 wheel re-ran all 80
Chromium navigation tests: 78 passed, two known shared failures, no synthetic
JUnit errors. The two extra v7 errors were not reproduced; that observation
does not establish their root cause or prove they cannot recur.
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

import verify_playwright_python_navigation_full_strict_paired_v7 as abi131
import verify_playwright_python_network50_strict_paired_v8 as prior
import verify_playwright_python_official_provenance as official

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / ".work/official-conformance"
BASE_DIR = WORK / "outcomes/python-network50-chromium-strict-paired-v8"
OUTPUT_DIR = WORK / "outcomes/python-navigation-recheck-chromium-strict-paired-v9"
RESULT = WORK / "results/navigation-candidate-full80-abi131-recheck-oct21.xml"
PROVENANCE = RESULT.with_suffix(".provenance.json")
INVENTORY = prior.INVENTORY
MANIFEST = abi131.MANIFEST
UPSTREAM = prior.UPSTREAM
NAVIGATION_SOURCE = UPSTREAM / "tests/async/test_navigation.py"
SOURCE_PINS = OUTPUT_DIR / "source-pins.json"
BASE_PROMOTER = ROOT / "scripts/promote_playwright_python_network50_strict_paired_v8.py"
BASE_VERIFIER = ROOT / "scripts/verify_playwright_python_network50_strict_paired_v8.py"
OFFICIAL_VERIFIER = ROOT / "scripts/verify_playwright_python_official_provenance.py"
PROMOTER = (
    ROOT / "scripts/promote_playwright_python_navigation_recheck_strict_paired_v9.py"
)

MODES = ("candidate", "oracle")
RECORDS = 6_526
TARGET_INDEXES = tuple(range(2680, 2918, 3))
GENERATED = frozenset({"attempts", "durationSeconds", "error", "outcome", "resultFile"})
KIND = "playwright-python-navigation-recheck-chromium-strict-paired-promotion-v9"
SCOPE = "tests/async/test_navigation.py-chromium-full-file-recheck-80"
COMMIT = prior.COMMIT
INVENTORY_SHA256 = prior.INVENTORY_SHA256
MANIFEST_SHA256 = abi131.MANIFEST_SHA256
NAVIGATION_SHA256 = abi131.NAVIGATION_SHA256
BASE_VERIFIER_SHA256 = (
    "2ece628c5ed2c38e5af7ba29eaeb52da75e01c6f8a925b7386e644e2fe601c3d"
)
BASE_PROMOTER_SHA256 = (
    "5a3445128e68468267bd8c604340b13500acf26b407e3a4383b0366d33a2a6a9"
)
BASE_RECEIPT_SHA256 = "b4acfb11c8af4305e55c9f314c8a55193ed6b930bde1cd43b2c0239ae8f27189"
OFFICIAL_VERIFIER_SHA256 = abi131.OFFICIAL_VERIFIER_SHA256
BASE_HASHES = {
    "candidate": (
        "166cb752b3d28876ec52e7709b31c522cd49af761242f90226e008400eb2d7b1",
        "72f021bd9b72b7ee65e54bd643b46fef4d084413dc512ae3264ea6bcd55318f4",
    ),
    "oracle": (
        "5993775f11480d3e24ed6a56a4d9a7ccd4b3c4dd3722141ac4c27c3a4132907a",
        "4442cd4dbf31e37c7be3fdda35710dfa34342516aa1a0557c279ce5d71c3f540",
    ),
}
REPORT_SHA256 = "08cde7778418e85509bed40b251d8a9d0eca6512b6bbbea8a6989e5fe69e7af3"
PROVENANCE_SHA256 = "3a7a5e18e9b725c07849acb986b4de27b2171ac24cf3bc6a18505cb97213bb29"
CASE_REPORTS_SHA256 = "0098cfc2986228c3c9596f970357fd2bc94645be48b024d2c5ac1203fd931eb7"
ISOLATED_IDS_SHA256 = abi131.ISOLATED_IDS_SHA256
HISTORICAL_SYNTHETIC_ERROR_IDS = (
    "tests/async/test_navigation.py::test_frame_wait_for_nav_should_fail_when_frame_detaches[chromium]",
    "tests/async/test_navigation.py::test_wait_for_nav_should_work_when_subframe_issues_window_stop[chromium]",
)
EXPECTED_REPORT_COUNTS = {"passed": 78, "failed": 2}
EXPECTED_SUCCESSOR_COUNTS = {
    "candidate": {
        "error": 3,
        "failed": 119,
        "not-run": 4717,
        "passed": 1650,
        "skipped": 37,
    },
    "oracle": {"failed": 10, "not-run": 5046, "passed": 1433, "skipped": 37},
}
SOURCE_PINS_SHA256 = "68dc01243e9d2bc82aec0010e0f6dee49dc36d0c951b4d4312ce4fd892c1ffc4"
PROMOTER_SHA256 = "03564f6f093bb244c8a3cfce4626b4d5b7497b7d111de28cc0474896855daaa5"


class NavigationRecheckValidationError(ValueError):
    """An immutable recheck source, report, or successor differs."""


def fail(message: str) -> NoReturn:
    raise NavigationRecheckValidationError(message)


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
        raise NavigationRecheckValidationError(f"{label} is invalid JSON") from error
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
    return f"playwright-python-{mode}-network50-chromium-strict-paired-v8.jsonl"


def output_name(mode: str) -> str:
    require(mode in MODES, "unknown lane")
    return (
        f"playwright-python-{mode}-navigation-recheck-chromium-strict-paired-v9.jsonl"
    )


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
        ("completeFileExecution", True),
        ("fixtureAdapted", True),
        ("sameWheelAbi131Recheck", True),
        ("historicalSyntheticErrorsNotReproduced", True),
        ("failureNonReproductionDoesNotProveAbsence", True),
        ("rootCauseEstablished", False),
    ):
        require(receipt.get(field) is expected, f"receipt {field} differs")
    require(receipt.get("launchMode") == "cdp-adapter", "receipt launch mode differs")


def _base() -> tuple[dict[str, Any], list[dict[str, Any]], dict[str, list[bytes]]]:
    require(
        Path(prior.__file__).resolve() == BASE_VERIFIER, "v8 verifier module differs"
    )
    sealed(BASE_VERIFIER, BASE_VERIFIER_SHA256, "v8 verifier source")
    sealed(BASE_PROMOTER, BASE_PROMOTER_SHA256, "v8 promoter source")
    receipt = json_object(
        sealed(BASE_DIR / "promotion-receipt.json", BASE_RECEIPT_SHA256, "v8 receipt"),
        "v8 receipt",
    )
    require(prior.verify_bundle(BASE_DIR) == receipt, "v8 source-bound chain differs")
    _, inventory = jsonl(
        sealed(INVENTORY, INVENTORY_SHA256, "profiled inventory"), "inventory"
    )
    targets = [inventory[index].get("id") for index in TARGET_INDEXES]
    require(
        len(targets) == len(set(targets)) == 80
        and all(
            isinstance(test_id, str)
            and test_id.startswith("tests/async/test_navigation.py::")
            and test_id.endswith("[chromium]")
            and inventory[index].get("sourceClassification") == "public-api-source"
            for index, test_id in zip(TARGET_INDEXES, targets)
        )
        and sum(
            isinstance(row.get("id"), str)
            and row["id"].startswith("tests/async/test_navigation.py::")
            and row["id"].endswith("[chromium]")
            for row in inventory
        )
        == 80,
        "80 Chromium navigation target IDs differ",
    )
    originals: dict[str, list[bytes]] = {}
    for mode in MODES:
        ledger = BASE_DIR / base_name(mode)
        ledger_sha, summary_sha = BASE_HASHES[mode]
        lines, rows = jsonl(
            sealed(ledger, ledger_sha, f"{mode} v8 ledger"), f"{mode} v8 ledger"
        )
        summary = json_object(
            sealed(
                ledger.with_suffix(".summary.json"), summary_sha, f"{mode} v8 summary"
            ),
            f"{mode} v8 summary",
        )
        require(
            receipt["successors"].get(mode) == summary, f"{mode} v8 summary differs"
        )
        require(
            all(
                row.get("id") == inventory[index].get("id")
                for index, row in enumerate(rows)
            ),
            f"{mode} row order differs",
        )
        slice_counts = Counter(rows[index].get("outcome") for index in TARGET_INDEXES)
        require(
            slice_counts
            == (
                {"passed": 76, "failed": 2, "error": 2}
                if mode == "candidate"
                else {"passed": 77, "failed": 3}
            ),
            f"{mode} v8 navigation baseline differs",
        )
        if mode == "candidate":
            require(
                {
                    rows[index]["id"]
                    for index in TARGET_INDEXES
                    if rows[index].get("outcome") == "error"
                }
                == set(HISTORICAL_SYNTHETIC_ERROR_IDS),
                "historical synthetic error IDs differ",
            )
        originals[mode] = lines
    return receipt, inventory, originals


def check_report_metadata(payload: dict[str, Any]) -> None:
    expected = {
        "schema": 5,
        "mode": "candidate",
        "repository": "https://github.com/microsoft/playwright-python.git",
        "tag": "v1.58.0",
        "commit": COMMIT,
        "manifest": str(MANIFEST),
        "manifestSha256": MANIFEST_SHA256,
        "upstream": str(UPSTREAM),
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "scope": "selected",
        "manifestCases": False,
        "endpoint": "http://127.0.0.1:9222",
        "fixtureBind": "100.78.97.63",
        "fixtureHost": "macbookpro-yuacx.tail8f3b66.ts.net",
        "caseTimeout": 45.0,
        "isolatedCaseIdsSha256": ISOLATED_IDS_SHA256,
        "isolatedCaseCount": 80,
        "selections": ["tests/async/test_navigation.py"],
        "browsers": ["chromium"],
        "launchMode": "cdp-adapter",
        "isolateCases": True,
        "resumeIsolated": False,
        "collectOnly": False,
        "bootstrapOnly": False,
        "secureHttpFixture": False,
        "pythonExecutable": str(abi131.PYTHON_EXECUTABLE),
        "playwrightPackageSha256": abi131.PLAYWRIGHT_PACKAGE_SHA256,
        "pwrightPackage": str(abi131.CANDIDATE_PACKAGE),
        "pwrightPackageSha256": abi131.PWRIGHT_PACKAGE_SHA256,
        "pwrightArtifact": str(abi131.CANDIDATE_ARTIFACT),
        "pwrightArtifactSha256": abi131.PWRIGHT_ARTIFACT_SHA256,
        "result": str(RESULT),
        "resultSha256": REPORT_SHA256,
        "caseReports": str(Path(f"{RESULT}.cases")),
        "caseReportsSha256": CASE_REPORTS_SHA256,
        "exitCode": 1,
    }
    require(
        all(payload.get(key) == value for key, value in expected.items()),
        "ABI131 recheck source-bound identity differs",
    )


def _junit_cases(raw: bytes, target_ids: list[str]) -> dict[str, dict[str, object]]:
    try:
        tree = ET.fromstring(raw)
    except ET.ParseError as error:
        raise NavigationRecheckValidationError(
            "navigation80 recheck JUnit is invalid XML"
        ) from error
    cases = list(tree.iter("testcase"))
    require(len(cases) == 80, "navigation80 recheck JUnit does not have 80 cases")
    mapped: dict[str, dict[str, object]] = {}
    for case in cases:
        require(
            case.get("classname") == "tests.async.test_navigation",
            "recheck JUnit case is outside pinned source",
        )
        name = case.get("name")
        require(
            isinstance(name, str) and name.endswith("[chromium]"),
            "recheck JUnit case name differs",
        )
        test_id = f"tests/async/test_navigation.py::{name}"
        require(test_id not in mapped, "recheck JUnit duplicate case ID")
        try:
            duration = float(case.get("time", ""))
        except ValueError as error:
            raise NavigationRecheckValidationError(
                "recheck JUnit duration is invalid"
            ) from error
        require(
            math.isfinite(duration) and duration >= 0,
            "recheck JUnit duration is invalid",
        )
        terminals = [
            node
            for tag in ("failure", "error", "skipped")
            for node in case.findall(tag)
        ]
        require(
            len(terminals) <= 1 and (not terminals or terminals[0].tag == "failure"),
            "recheck JUnit terminal status differs",
        )
        row: dict[str, object] = {
            "outcome": "failed" if terminals else "passed",
            "durationSeconds": duration,
        }
        if terminals:
            message = (
                terminals[0].get("message") or terminals[0].text or ""
            ).splitlines()
            require(
                bool(message and message[0]), "recheck JUnit failure diagnostic missing"
            )
            row["error"] = message[0]
        mapped[test_id] = row
    require(list(mapped) == target_ids, "recheck JUnit IDs/order differ from inventory")
    require(
        dict(Counter(row["outcome"] for row in mapped.values()))
        == EXPECTED_REPORT_COUNTS,
        "recheck JUnit outcome counts differ",
    )
    require(
        all(
            mapped[test_id]["outcome"] == "passed"
            for test_id in HISTORICAL_SYNTHETIC_ERROR_IDS
        ),
        "historical synthetic errors did not pass in recheck",
    )
    return mapped


def _report(
    target_ids: list[str],
) -> tuple[dict[str, Any], dict[str, dict[str, object]]]:
    require(
        Path(official.__file__).resolve() == OFFICIAL_VERIFIER,
        "official verifier module differs",
    )
    sealed(OFFICIAL_VERIFIER, OFFICIAL_VERIFIER_SHA256, "official verifier source")
    payload = json_object(
        sealed(PROVENANCE, PROVENANCE_SHA256, "ABI131 recheck provenance"),
        "ABI131 recheck provenance",
    )
    raw = sealed(RESULT, REPORT_SHA256, "ABI131 recheck JUnit")
    try:
        official.verify_provenance(payload, provenance_path=PROVENANCE)
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        raise NavigationRecheckValidationError(
            f"ABI131 recheck provenance failed: {error}"
        ) from error
    check_report_metadata(payload)
    return payload, _junit_cases(raw, target_ids)


def source_pins_expected(provenance: dict[str, Any]) -> dict[str, Any]:
    for field in ("harnessInputs", "toolingSources", "pwrightRuntimeLibraries"):
        require(
            isinstance(provenance.get(field), list) and bool(provenance[field]),
            f"recheck {field} missing",
        )
    return {
        "schema": 1,
        "kind": "playwright-python-navigation80-abi131-recheck-v9",
        "upstreamCommit": COMMIT,
        "upstreamNavigationSha256": NAVIGATION_SHA256,
        "officialManifestSha256": MANIFEST_SHA256,
        "profiledInventorySha256": INVENTORY_SHA256,
        "chromiumExpandedCases": 80,
        "frozenWheel": str(abi131.WHEEL),
        "frozenWheelSha256": abi131.WHEEL_SHA256,
        "installedNativeAbi": 131,
        "playwrightPackageSha256": abi131.PLAYWRIGHT_PACKAGE_SHA256,
        "pwrightPackageSha256": abi131.PWRIGHT_PACKAGE_SHA256,
        "pwrightArtifactSha256": abi131.PWRIGHT_ARTIFACT_SHA256,
        "report": str(RESULT),
        "reportSha256": REPORT_SHA256,
        "provenance": str(PROVENANCE),
        "provenanceSha256": PROVENANCE_SHA256,
        "historicalSyntheticErrorTestIds": list(HISTORICAL_SYNTHETIC_ERROR_IDS),
        "harnessInputs": provenance["harnessInputs"],
        "toolingSources": provenance["toolingSources"],
        "pwrightRuntimeLibraries": provenance["pwrightRuntimeLibraries"],
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
        "report target set differs",
    )
    targets = set(TARGET_INDEXES)
    changed = 0
    counts: Counter[str] = Counter()
    for index, (before_raw, after_raw, source) in enumerate(
        zip(base_lines, successor_lines, inventory)
    ):
        before = json_object(before_raw, f"{mode} v8 row {index}")
        after = json_object(after_raw, f"{mode} v9 row {index}")
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
        if mode == "candidate" and index in targets:
            require(
                before.get("attempts") == 1
                and before.get("outcome") in {"passed", "failed", "error"},
                f"candidate prior target status differs at {index}",
            )
            case = cases[source["id"]]
            expected: dict[str, object] = {
                **source,
                "attempts": 1,
                "durationSeconds": case["durationSeconds"],
                "outcome": case["outcome"],
                "resultFile": str(RESULT),
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
                f"candidate target row differs at {index}",
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
    require(
        changed == (len(TARGET_INDEXES) if mode == "candidate" else 0),
        f"{mode} changed-row count differs",
    )
    require(
        dict(counts) == EXPECTED_SUCCESSOR_COUNTS[mode],
        f"{mode} aggregate outcomes differ",
    )
    return counts


def evidence() -> tuple[
    dict[str, Any],
    list[dict[str, Any]],
    dict[str, list[bytes]],
    dict[str, Any],
    dict[str, dict[str, object]],
]:
    base_receipt, inventory, base_lines = _base()
    targets = [inventory[index]["id"] for index in TARGET_INDEXES]
    provenance, cases = _report(targets)
    try:
        abi131._verify_wheel(provenance)
    except abi131.FullNavigationV7ValidationError as error:
        raise NavigationRecheckValidationError(
            f"ABI131 same wheel failed: {error}"
        ) from error
    sealed(MANIFEST, MANIFEST_SHA256, "official manifest")
    sealed(NAVIGATION_SOURCE, NAVIGATION_SHA256, "upstream navigation source")
    return base_receipt, inventory, base_lines, provenance, cases


def verify_bundle(output_dir: Path = OUTPUT_DIR) -> dict[str, Any]:
    require(
        output_dir == OUTPUT_DIR
        and output_dir.is_dir()
        and not output_dir.is_symlink(),
        "v9 successor directory missing or moved",
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
        "v9 successor file set differs",
    )
    require(
        all(
            (output_dir / name).is_file() and not (output_dir / name).is_symlink()
            for name in expected_files
        ),
        "v9 successor file missing or linked",
    )
    receipt = json_object(
        (output_dir / "promotion-receipt.json").read_bytes(), "v9 receipt"
    )
    check_receipt_scope(receipt)
    base_receipt, inventory, base_lines, provenance, cases = evidence()
    target_ids = [inventory[index]["id"] for index in TARGET_INDEXES]
    pins = json_object(
        sealed(SOURCE_PINS, SOURCE_PINS_SHA256, "v9 source pins"), "v9 source pins"
    )
    require(pins == source_pins_expected(provenance), "v9 source-pin identity differs")
    sealed(PROMOTER, PROMOTER_SHA256, "v9 promoter source")
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
        "upstreamNavigationSha256": NAVIGATION_SHA256,
        "sourcePins": str(SOURCE_PINS),
        "sourcePinsSha256": SOURCE_PINS_SHA256,
        "promoterSha256": PROMOTER_SHA256,
        "basePromoterSha256": BASE_PROMOTER_SHA256,
        "baseVerifierSha256": BASE_VERIFIER_SHA256,
        "baseReceipt": str(BASE_DIR / "promotion-receipt.json"),
        "baseReceiptSha256": BASE_RECEIPT_SHA256,
        "frozenWheel": str(abi131.WHEEL),
        "frozenWheelSha256": abi131.WHEEL_SHA256,
        "historicalSyntheticErrorTestIds": list(HISTORICAL_SYNTHETIC_ERROR_IDS),
    }
    require(
        all(receipt.get(key) == value for key, value in expected_fields.items()),
        "v9 receipt source, target or historical ID differs",
    )
    require(base_receipt.get("successors") is not None, "v8 receipt has no successors")
    require(
        isinstance(receipt.get("bases"), dict) and set(receipt["bases"]) == set(MODES),
        "v9 base lanes differ",
    )
    require(
        isinstance(receipt.get("reports"), dict)
        and set(receipt["reports"]) == {"candidate"},
        "v9 report lane differs",
    )
    require(
        isinstance(receipt.get("successors"), dict)
        and set(receipt["successors"]) == set(MODES),
        "v9 successor lanes differ",
    )
    expected_report = {
        "provenance": str(PROVENANCE),
        "provenanceSha256": PROVENANCE_SHA256,
        "result": str(RESULT),
        "resultSha256": REPORT_SHA256,
        "caseReportsSha256": CASE_REPORTS_SHA256,
        "isolatedCaseIdsSha256": ISOLATED_IDS_SHA256,
        "observedOutcomes": EXPECTED_REPORT_COUNTS,
    }
    require(
        receipt["reports"]["candidate"] == expected_report, "v9 receipt report differs"
    )
    for mode in MODES:
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
            f"{mode} v9 receipt base differs",
        )
        output_ledger = output_dir / output_name(mode)
        raw = output_ledger.read_bytes()
        successor_lines, _ = jsonl(raw, f"{mode} v9 ledger")
        counts = check_reconciliation(
            base_lines[mode], successor_lines, inventory, cases, mode
        )
        summary = json_object(
            output_ledger.with_suffix(".summary.json").read_bytes(),
            f"{mode} v9 summary",
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
            f"{mode} v9 summary differs",
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
        NavigationRecheckValidationError,
        prior.Network50ValidationError,
        abi131.FullNavigationV7ValidationError,
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
