"""Independently verify the one-case Python Chromium history-API v4 successor.

The sealed v3 full-file pair is the base. Only the candidate history-API row
may change from failed to passed using a source-bound ABI129-v6 focused run.
The other 6,525 candidate rows (including the synthetic report error) and all
6,526 oracle rows must retain their exact v3 bytes. This script never imports
the v4 publisher.
"""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import math
import subprocess
import xml.etree.ElementTree as ET
import zipfile
from collections import Counter
from pathlib import Path
from typing import Any, NoReturn

import verify_playwright_python_navigation_full_strict_paired_v3 as prior
import verify_playwright_python_official_provenance as official

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / ".work/official-conformance"
BUILD = WORK / "builds/python-history-forward-abi129-v6.aWkEM6"
BASE_DIR = WORK / "outcomes/python-navigation-full-chromium-strict-paired-v3"
OUTPUT_DIR = WORK / "outcomes/python-navigation-history-api-chromium-strict-paired-v4"
INVENTORY = WORK / "playwright-python-v1.58.0-profiled.jsonl"
MANIFEST = ROOT / "packages/pwright-python/tests/official/manifest.json"
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-python")
NAVIGATION_SOURCE = UPSTREAM / "tests/async/test_navigation.py"
SOURCE_PINS = OUTPUT_DIR / "source-pins.json"
RESULT = BUILD / "results/history-api-candidate-abi129-v6.xml"
PROVENANCE = RESULT.with_suffix(".provenance.json")
CANDIDATE_VENV = BUILD / "candidate-abi129-v6"
CANDIDATE_PACKAGE = CANDIDATE_VENV / "lib/python3.12/site-packages/pwright"
CANDIDATE_ARTIFACT = CANDIDATE_PACKAGE / "_native.abi3.so"
PYTHON_EXECUTABLE = CANDIDATE_VENV / "bin/python"
WHEEL = BUILD / "wheel-repaired/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl"
BASE_VERIFIER = (
    ROOT / "scripts/verify_playwright_python_navigation_full_strict_paired_v3.py"
)
BASE_PROMOTER = (
    ROOT / "scripts/promote_playwright_python_navigation_full_strict_paired_v3.py"
)
OFFICIAL_VERIFIER = ROOT / "scripts/verify_playwright_python_official_provenance.py"
PROMOTER = (
    ROOT
    / "scripts/promote_playwright_python_navigation_history_api_strict_paired_v4.py"
)

MODES = ("candidate", "oracle")
RECORDS = 6_526
TARGET_INDEX = 2704
TARGET_ID = "tests/async/test_navigation.py::test_go_back_should_work_with_history_api[chromium]"
GENERATED = frozenset({"attempts", "durationSeconds", "error", "outcome", "resultFile"})
KIND = "playwright-python-navigation-history-api-chromium-strict-paired-promotion-v4"
SCOPE = "tests/async/test_navigation.py-chromium-history-api-1"
COMMIT = "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
INVENTORY_SHA256 = "3dfb11b6ad7eb4736166ee2c4f7fa32cda78063324ad8314ce12f3e2d49b9d41"
MANIFEST_SHA256 = "6d4f86fee517e300319cfa87014a54302980bdf8cf538fa21c1daff4219ea724"
NAVIGATION_SHA256 = "f04af8cbdee3dffb2c578d61d7685f02e5186ca565906323d71b6f92f96cd144"
BASE_VERIFIER_SHA256 = (
    "42d51cfad1a4f14c803d171e6f4a0afeb1813787248be929170dd48ee629caed"
)
BASE_PROMOTER_SHA256 = (
    "c36107a1f7ced0645a33a1c52c8817d1bbd29e0d75693c6ce2fbdf0d641f04cd"
)
OFFICIAL_VERIFIER_SHA256 = (
    "cb451d9977ff574efb0778b842ff8662a95471fc2d1ca5dff450bef39c6e50d7"
)
BASE_RECEIPT_SHA256 = "2255d24bdeee2fc9b9a9d4c12c742d1e65c55793a8562c8b65cf978eb6346f1b"
BASE_HASHES = {
    "candidate": (
        "2be32b693fc328194efcd0b69b412bfbff8e10d1d54e21aafd125be4e0177bf8",
        "e4e102c0cc462ad7ad8b367ba3c266fdf9a5c132fb155dbdd901d5473e731d28",
    ),
    "oracle": (
        "b209bf22f7777283798842abfd2a9774e4bb8f1659ae1b67e6015a3dc5a22479",
        "9a860500dc4e26b77a768175826d440bbfa77b2e3bb7bc5c85c57771604d442b",
    ),
}
WHEEL_SHA256 = "07cc6f949c765746f76f6bb4b1567e3a1bee74a9f57e75e0f50d666c5745b37c"
RESULT_SHA256 = "4895ff3b36514245834b9a882219fa73143ecfb01a9831550437ff1b3302eb80"
PROVENANCE_SHA256 = "3ec57bf19ba02eb326e3e823dcb08963df1460bb2d611f6084a7026f2b5c654f"
CASE_REPORTS_SHA256 = "efaeea1a1bb7bbffa2b04aebca50c71cc66b6243e9420539b9fb608f51430bdb"
ISOLATED_IDS_SHA256 = "d95313963641267c0d28dde8ce144bae0cc3f41ff74d9429b9f711c2d2a1d8da"
PWRIGHT_ARTIFACT_SHA256 = (
    "2ec3c59d61d5f6affd604f2ad0d92d9902d77d7530738481ddae9115a885e9ff"
)
PWRIGHT_PACKAGE_SHA256 = (
    "8f58621d0e98bcc3d88a102bd5a7846eabb920dc359ef02ab5dd58327d8abbf7"
)
SOURCE_PINS_SHA256 = "da763aa45e2e1d44bca10b3491b89c4e90a16ba1e512a12a77f44b2f30b70d94"


class HistoryApiValidationError(ValueError):
    """An immutable source, report, or successor differs from its contract."""


def fail(message: str) -> NoReturn:
    raise HistoryApiValidationError(message)


def require(condition: bool, message: str) -> None:
    if not condition:
        fail(message)


def sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def sealed(path: Path, expected_sha: str, label: str) -> bytes:
    require(
        len(expected_sha) == 64
        and all(character in "0123456789abcdef" for character in expected_sha),
        f"{label} has an invalid SHA-256 pin",
    )
    require(path.is_file() and not path.is_symlink(), f"{label} missing or linked")
    raw = path.read_bytes()
    require(sha256(raw) == expected_sha, f"{label} SHA-256 differs")
    return raw


def json_object(raw: bytes, label: str) -> dict[str, Any]:
    try:
        value = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise HistoryApiValidationError(f"{label} is invalid JSON") from error
    require(isinstance(value, dict), f"{label} is not an object")
    return value


def jsonl(
    raw: bytes, label: str, expected_rows: int = RECORDS
) -> tuple[list[bytes], list[dict[str, Any]]]:
    require(raw.endswith(b"\n"), f"{label} is not newline terminated")
    lines = raw.splitlines(keepends=True)
    require(len(lines) == expected_rows, f"{label} row count differs")
    return lines, [
        json_object(line, f"{label} row {index}") for index, line in enumerate(lines)
    ]


def base_name(mode: str) -> str:
    require(mode in MODES, "unknown lane")
    return f"playwright-python-{mode}-navigation-full-chromium-strict-paired-v3.jsonl"


def output_name(mode: str) -> str:
    require(mode in MODES, "unknown lane")
    return f"playwright-python-{mode}-navigation-history-api-chromium-strict-paired-v4.jsonl"


def check_receipt_scope(receipt: dict[str, object]) -> None:
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
        ("fixtureAdapted", True),
    ):
        require(receipt.get(field) is expected, f"receipt {field} differs")
    require(receipt.get("launchMode") == "cdp-adapter", "receipt launch mode differs")
    require(
        receipt.get("chromeVersionObserved") == "155.0.8059.85",
        "receipt Chrome version differs",
    )


def _metadata(row: dict[str, Any]) -> dict[str, Any]:
    return {key: value for key, value in row.items() if key not in GENERATED}


def check_reconciliation(
    base_lines: list[bytes],
    successor_lines: list[bytes],
    inventory_rows: list[dict[str, object]],
    target_index: int,
    target_id: str,
    duration: float,
    result_file: str,
    mode: str,
) -> Counter[str]:
    """Rebuild the one candidate target and retain exact other/oracle bytes."""
    require(mode in MODES, "ledger mode differs")
    require(
        len(base_lines) == len(successor_lines) == len(inventory_rows),
        f"{mode} ledger length differs",
    )
    require(0 <= target_index < len(inventory_rows), "target index is out of bounds")
    require(
        inventory_rows[target_index].get("id") == target_id,
        "target ID moved in inventory",
    )
    ids = [row.get("id") for row in inventory_rows]
    require(
        all(isinstance(test_id, str) for test_id in ids) and len(ids) == len(set(ids)),
        "inventory IDs differ",
    )
    require(
        type(duration) is float and math.isfinite(duration) and duration >= 0,
        "focused report duration differs",
    )
    counts: Counter[str] = Counter()
    for index, (before_raw, after_raw, source) in enumerate(
        zip(base_lines, successor_lines, inventory_rows)
    ):
        before = json_object(before_raw, f"{mode} base row {index}")
        after = json_object(after_raw, f"{mode} successor row {index}")
        require(
            before.get("id") == ids[index] and after.get("id") == ids[index],
            f"{mode} ID moved at {index}",
        )
        require(
            _metadata(before) == source and _metadata(after) == source,
            f"{mode} metadata changed at {index}",
        )
        if mode == "oracle" or index != target_index:
            require(after_raw == before_raw, f"{mode} raw row changed at {index}")
        else:
            require(
                source.get("sourceClassification") == "public-api-source",
                "target source classification differs",
            )
            require(
                before.get("outcome") == "failed" and before.get("attempts") == 1,
                "candidate base target status differs",
            )
            require(
                isinstance(before.get("error"), str) and bool(before["error"]),
                "candidate base target failure missing",
            )
            expected: dict[str, object] = {
                **source,
                "attempts": 1,
                "durationSeconds": duration,
                "outcome": "passed",
                "resultFile": result_file,
            }
            require(after == expected, "candidate promoted target differs")
            canonical = (
                json.dumps(expected, sort_keys=True, separators=(",", ":")) + "\n"
            ).encode()
            require(after_raw == canonical, "candidate promoted target bytes differ")
            require(after_raw != before_raw, "candidate target row did not change")
        status = after.get("outcome")
        require(isinstance(status, str), f"{mode} outcome malformed at {index}")
        counts[status] += 1
    return counts


def _junit_duration(raw: bytes, target_id: str) -> float:
    try:
        tree = ET.fromstring(raw)
    except ET.ParseError as error:
        raise HistoryApiValidationError(
            "focused history-API JUnit is invalid XML"
        ) from error
    cases = list(tree.iter("testcase"))
    require(len(cases) == 1, "focused history-API JUnit does not have exactly one case")
    case = cases[0]
    require(
        case.get("classname") == "tests.async.test_navigation",
        "focused JUnit case is outside pinned source",
    )
    require(
        f"tests/async/test_navigation.py::{case.get('name')}" == target_id,
        "focused JUnit target ID differs",
    )
    require(
        not any(case.findall(tag) for tag in ("failure", "error", "skipped")),
        "focused JUnit case did not pass",
    )
    try:
        duration = float(case.get("time", ""))
    except ValueError as error:
        raise HistoryApiValidationError("focused JUnit duration is invalid") from error
    require(
        math.isfinite(duration) and duration >= 0, "focused JUnit duration is invalid"
    )
    return duration


def _verify_base() -> tuple[
    dict[str, Any], list[dict[str, Any]], dict[str, list[bytes]]
]:
    require(
        Path(prior.__file__).resolve() == BASE_VERIFIER, "v3 verifier module differs"
    )
    sealed(BASE_VERIFIER, BASE_VERIFIER_SHA256, "v3 verifier source")
    sealed(BASE_PROMOTER, BASE_PROMOTER_SHA256, "v3 promoter source")
    base_receipt = json_object(
        sealed(BASE_DIR / "promotion-receipt.json", BASE_RECEIPT_SHA256, "v3 receipt"),
        "v3 receipt",
    )
    require(
        prior.verify_bundle(BASE_DIR) == base_receipt, "v3 chain verification differs"
    )
    _, inventory = jsonl(
        sealed(INVENTORY, INVENTORY_SHA256, "profiled inventory"), "inventory"
    )
    require(
        inventory[TARGET_INDEX].get("id") == TARGET_ID, "v3 target inventory ID differs"
    )
    require(
        inventory[TARGET_INDEX].get("sourceClassification") == "public-api-source",
        "v3 target source differs",
    )
    require(base_receipt.get("successors") is not None, "v3 receipt has no successors")
    base_lines: dict[str, list[bytes]] = {}
    for mode in MODES:
        ledger = BASE_DIR / base_name(mode)
        ledger_sha, summary_sha = BASE_HASHES[mode]
        lines, rows = jsonl(
            sealed(ledger, ledger_sha, f"{mode} v3 ledger"), f"{mode} v3 ledger"
        )
        summary = json_object(
            sealed(
                ledger.with_suffix(".summary.json"), summary_sha, f"{mode} v3 summary"
            ),
            f"{mode} v3 summary",
        )
        require(
            base_receipt["successors"].get(mode) == summary,
            f"{mode} v3 receipt summary differs",
        )
        require(
            summary.get("ledgerSha256") == ledger_sha
            and summary.get("records") == RECORDS,
            f"{mode} v3 summary differs",
        )
        expected = "failed" if mode == "candidate" else "passed"
        require(
            rows[TARGET_INDEX].get("outcome") == expected,
            f"{mode} v3 target status differs",
        )
        base_lines[mode] = lines
    return base_receipt, inventory, base_lines


def check_report_metadata(payload: dict[str, Any]) -> None:
    """Reject a report outside the exact frozen one-case Chromium run."""
    require(
        payload.get("schema") == 5
        and payload.get("mode") == "candidate"
        and payload.get("commit") == COMMIT
        and payload.get("manifest") == str(MANIFEST)
        and payload.get("manifestSha256") == MANIFEST_SHA256
        and payload.get("upstream") == str(UPSTREAM)
        and payload.get("inventory") == str(INVENTORY)
        and payload.get("inventorySha256") == INVENTORY_SHA256
        and payload.get("scope") == "selected"
        and payload.get("endpoint") == "http://127.0.0.1:9222"
        and payload.get("fixtureBind") == "100.78.97.63"
        and payload.get("fixtureHost") == "macbookpro-yuacx.tail8f3b66.ts.net"
        and payload.get("result") == str(RESULT)
        and payload.get("resultSha256") == RESULT_SHA256
        and payload.get("caseReports") == str(Path(f"{RESULT}.cases"))
        and payload.get("caseReportsSha256") == CASE_REPORTS_SHA256
        and payload.get("caseTimeout") == 45.0
        and payload.get("isolatedCaseIdsSha256") == ISOLATED_IDS_SHA256
        and payload.get("isolatedCaseCount") == 1
        and payload.get("selections") == [TARGET_ID]
        and payload.get("browsers") == ["chromium"]
        and payload.get("launchMode") == "cdp-adapter"
        and payload.get("isolateCases") is True
        and payload.get("resumeIsolated") is False
        and payload.get("collectOnly") is False
        and payload.get("bootstrapOnly") is False
        and payload.get("manifestCases") is False
        and payload.get("secureHttpFixture") is False
        and payload.get("exitCode") == 0
        and payload.get("pythonExecutable") == str(PYTHON_EXECUTABLE)
        and payload.get("pwrightPackage") == str(CANDIDATE_PACKAGE)
        and payload.get("pwrightArtifact") == str(CANDIDATE_ARTIFACT)
        and payload.get("pwrightArtifactSha256") == PWRIGHT_ARTIFACT_SHA256
        and payload.get("pwrightPackageSha256") == PWRIGHT_PACKAGE_SHA256,
        "focused history-API provenance identity or scope differs",
    )


def _verify_report() -> tuple[dict[str, Any], float]:
    require(
        Path(official.__file__).resolve() == OFFICIAL_VERIFIER,
        "official verifier module differs",
    )
    sealed(OFFICIAL_VERIFIER, OFFICIAL_VERIFIER_SHA256, "official verifier source")
    payload = json_object(
        sealed(PROVENANCE, PROVENANCE_SHA256, "focused provenance"),
        "focused provenance",
    )
    result_raw = sealed(RESULT, RESULT_SHA256, "focused JUnit")
    try:
        official.verify_provenance(payload, provenance_path=PROVENANCE)
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        raise HistoryApiValidationError(
            f"focused official provenance failed: {error}"
        ) from error
    check_report_metadata(payload)
    return payload, _junit_duration(result_raw, TARGET_ID)


def _verify_wheel(provenance: dict[str, Any]) -> None:
    wheel_raw = sealed(WHEEL, WHEEL_SHA256, "frozen repaired ABI129-v6 wheel")
    package = provenance.get("pwrightPackage")
    require(isinstance(package, str), "report installed package path missing")
    package_path = Path(package)
    require(
        package_path.is_dir() and not package_path.is_symlink(),
        "report installed package missing or linked",
    )
    try:
        with zipfile.ZipFile(io.BytesIO(wheel_raw)) as archive:
            files = [
                item
                for item in archive.infolist()
                if item.filename.startswith("pwright/") and not item.is_dir()
            ]
            require(bool(files), "frozen wheel has no Python package")
            names = [item.filename for item in files]
            require(
                len(names) == len(set(names)),
                "frozen wheel has duplicate Python package files",
            )
            digest = hashlib.sha256()
            for item in files:
                relative = Path(item.filename.removeprefix("pwright/"))
                require(
                    relative.parts
                    and not relative.is_absolute()
                    and all(part not in {"..", "."} for part in relative.parts),
                    "frozen wheel has unsafe package path",
                )
                installed = package_path / relative
                require(
                    installed.is_file() and not installed.is_symlink(),
                    "installed package file missing or linked",
                )
                require(
                    sha256(archive.read(item)) == sha256(installed.read_bytes()),
                    f"wheel/installed package file differs: {item.filename}",
                )
            for item in sorted(files, key=lambda value: value.filename):
                relative = item.filename.removeprefix("pwright/")
                name = relative.encode()
                raw = archive.read(item)
                digest.update(len(name).to_bytes(8, "big"))
                digest.update(name)
                digest.update(len(raw).to_bytes(8, "big"))
                digest.update(raw)
            require(
                digest.hexdigest() == PWRIGHT_PACKAGE_SHA256,
                "wheel package SHA differs",
            )
            native = archive.read("pwright/_native.abi3.so")
            require(
                b"NATIVE_ABI_VERSION = 129" in archive.read("pwright/_abi.py"),
                "wheel ABI is not 129",
            )
    except (OSError, KeyError, zipfile.BadZipFile) as error:
        raise HistoryApiValidationError(
            "frozen wheel has no valid native artifact"
        ) from error
    require(sha256(native) == PWRIGHT_ARTIFACT_SHA256, "wheel/native artifact differs")
    require(
        provenance.get("pwrightArtifactSha256") == sha256(native),
        "report/wheel native artifact differs",
    )


def check_source_pins(source_pins: dict[str, Any], provenance: dict[str, Any]) -> None:
    """Require the exact ABI129-v6 wheel/report identity and one-case scope."""
    require(type(source_pins.get("schema")) is int, "v4 source-pin schema type differs")
    for field in ("harnessInputs", "toolingSources", "pwrightRuntimeLibraries"):
        require(
            isinstance(provenance.get(field), list) and bool(provenance[field]),
            f"focused report {field} is missing",
        )
    package_sha = provenance.get("playwrightPackageSha256")
    require(
        isinstance(package_sha, str)
        and len(package_sha) == 64
        and all(character in "0123456789abcdef" for character in package_sha),
        "focused report Playwright package SHA is invalid",
    )
    expected = {
        "schema": 1,
        "kind": "playwright-python-navigation-history-api-candidate-v4",
        "upstreamCommit": COMMIT,
        "upstreamNavigationSha256": NAVIGATION_SHA256,
        "officialManifestSha256": MANIFEST_SHA256,
        "profiledInventorySha256": INVENTORY_SHA256,
        "targetTestId": TARGET_ID,
        "targetZeroBasedIndex": TARGET_INDEX,
        "frozenWheel": str(WHEEL),
        "frozenWheelSha256": WHEEL_SHA256,
        "wheelName": WHEEL.name,
        "installedNativeAbi": 129,
        "pwrightArtifactSha256": PWRIGHT_ARTIFACT_SHA256,
        "pwrightPackageSha256": PWRIGHT_PACKAGE_SHA256,
        "playwrightPackageSha256": package_sha,
        "harnessInputs": provenance["harnessInputs"],
        "toolingSources": provenance["toolingSources"],
        "pwrightRuntimeLibraries": provenance["pwrightRuntimeLibraries"],
    }
    require(source_pins == expected, "v4 source-pin identity differs")


def verify_bundle(output_dir: Path = OUTPUT_DIR) -> dict[str, Any]:
    """Verify one fixed v4 successor and all immutable v3/source-bound inputs."""
    require(
        output_dir == OUTPUT_DIR
        and output_dir.is_dir()
        and not output_dir.is_symlink(),
        "successor directory missing or moved",
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
        "successor file set differs",
    )
    require(
        all(
            (output_dir / name).is_file() and not (output_dir / name).is_symlink()
            for name in expected_files
        ),
        "successor contains a missing or linked file",
    )
    receipt = json_object(
        (output_dir / "promotion-receipt.json").read_bytes(), "promotion receipt"
    )
    check_receipt_scope(receipt)
    base_receipt, inventory, base_lines = _verify_base()
    provenance, duration = _verify_report()
    _verify_wheel(provenance)
    sealed(MANIFEST, MANIFEST_SHA256, "official manifest")
    sealed(NAVIGATION_SOURCE, NAVIGATION_SHA256, "upstream navigation source")
    source_pins = json_object(
        sealed(SOURCE_PINS, SOURCE_PINS_SHA256, "v4 source pins"), "v4 source pins"
    )
    check_source_pins(source_pins, provenance)
    require(
        receipt.get("inventory") == str(INVENTORY)
        and receipt.get("inventorySha256") == INVENTORY_SHA256
        and receipt.get("recordsPerLane") == RECORDS
        and receipt.get("upstreamCommit") == COMMIT
        and receipt.get("manifest") == str(MANIFEST)
        and receipt.get("manifestSha256") == MANIFEST_SHA256
        and receipt.get("upstream") == str(UPSTREAM)
        and receipt.get("upstreamNavigationSha256") == NAVIGATION_SHA256
        and receipt.get("sourcePins") == str(SOURCE_PINS)
        and receipt.get("sourcePinsSha256") == SOURCE_PINS_SHA256
        and receipt.get("frozenWheel") == str(WHEEL)
        and receipt.get("frozenWheelSha256") == WHEEL_SHA256
        and receipt.get("baseReceipt") == str(BASE_DIR / "promotion-receipt.json")
        and receipt.get("baseReceiptSha256") == BASE_RECEIPT_SHA256
        and receipt.get("basePromoterSha256") == BASE_PROMOTER_SHA256
        and receipt.get("baseVerifierSha256") == BASE_VERIFIER_SHA256
        and receipt.get("targetZeroBasedIndexes") == [TARGET_INDEX]
        and receipt.get("targetTestIds") == [TARGET_ID],
        "promotion receipt source or target identity differs",
    )
    require(base_receipt.get("successors") is not None, "v3 receipt has no successors")
    promoter_sha = receipt.get("promoterSha256")
    require(isinstance(promoter_sha, str), "promoter source SHA missing")
    sealed(PROMOTER, promoter_sha, "v4 promoter source")
    require(isinstance(receipt.get("bases"), dict), "receipt bases missing")
    require(isinstance(receipt.get("reports"), dict), "receipt reports missing")
    require(isinstance(receipt.get("successors"), dict), "receipt successors missing")
    require(set(receipt["bases"]) == set(MODES), "receipt base lanes differ")
    require(
        set(receipt["reports"]) == {"candidate"},
        "receipt reports must be candidate-only",
    )
    require(set(receipt["successors"]) == set(MODES), "receipt successor lanes differ")
    require(
        receipt["reports"]["candidate"]
        == {
            "provenance": str(PROVENANCE),
            "provenanceSha256": PROVENANCE_SHA256,
            "result": str(RESULT),
            "resultSha256": RESULT_SHA256,
            "observedOutcomes": {"passed": 1},
            "isolatedCaseIdsSha256": ISOLATED_IDS_SHA256,
            "caseReportsSha256": CASE_REPORTS_SHA256,
        },
        "candidate receipt report differs",
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
            f"{mode} receipt base differs",
        )
        output_ledger = output_dir / output_name(mode)
        successor_raw = output_ledger.read_bytes()
        successor_lines, _ = jsonl(successor_raw, f"{mode} successor ledger")
        counts = check_reconciliation(
            base_lines[mode],
            successor_lines,
            inventory,
            TARGET_INDEX,
            TARGET_ID,
            duration,
            str(RESULT),
            mode,
        )
        summary = json_object(
            output_ledger.with_suffix(".summary.json").read_bytes(),
            f"{mode} successor summary",
        )
        expected_summary = {
            "schema": 1,
            "inventory": str(INVENTORY),
            "records": RECORDS,
            "outcomes": dict(sorted(counts.items())),
            "ledgerSha256": sha256(successor_raw),
            "ledger": str(output_ledger),
        }
        require(summary == expected_summary, f"{mode} successor summary differs")
        require(
            receipt["successors"][mode] == summary, f"{mode} receipt successor differs"
        )
        expected_counts = (
            {"error": 4, "failed": 108, "not-run": 4764, "passed": 1613, "skipped": 37}
            if mode == "candidate"
            else {"failed": 7, "not-run": 5096, "passed": 1386, "skipped": 37}
        )
        require(dict(counts) == expected_counts, f"{mode} successor aggregate differs")
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
        HistoryApiValidationError,
        ET.ParseError,
        zipfile.BadZipFile,
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
