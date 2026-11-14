"""Independently verify the five-case Python Frame navigation successor.

The full-file v1 pair is an immutable input. Five candidate rows are rebuilt
from one source-bound official report; all other candidate rows and every
oracle row must retain their exact v1 bytes. This module never imports the v2
publisher or launches a browser.
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

import verify_playwright_python_navigation_full_strict_paired_v1 as prior
import verify_playwright_python_official_provenance as official

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / ".work/official-conformance"
BUILD = WORK / "builds/python-frame-navigation-postfix-oct20.6zcD6U"
BASE_DIR = WORK / "outcomes/python-navigation-full-chromium-strict-paired-v1"
OUTPUT_DIR = WORK / "outcomes/python-navigation-frame-chromium-strict-paired-v2"
INVENTORY = WORK / "playwright-python-v1.58.0-profiled.jsonl"
MANIFEST = ROOT / "packages/pwright-python/tests/official/manifest.json"
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-python")
NAVIGATION_SOURCE = UPSTREAM / "tests/async/test_navigation.py"
SOURCE_PINS = OUTPUT_DIR / "source-pins.json"
RESULT = BUILD / "results/frame-navigation-candidate-frozen-v3-v1.xml"
PROVENANCE = RESULT.with_suffix(".provenance.json")
CANDIDATE_VENV = BUILD / "candidate-frame-v3-frozen"
CANDIDATE_PACKAGE = CANDIDATE_VENV / "lib/python3.12/site-packages/pwright"
CANDIDATE_ARTIFACT = CANDIDATE_PACKAGE / "_native.abi3.so"
PYTHON_EXECUTABLE = CANDIDATE_VENV / "bin/python"
WHEEL = BUILD / "wheel-abi128-v3/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl"
BASE_VERIFIER = (
    ROOT / "scripts/verify_playwright_python_navigation_full_strict_paired_v1.py"
)
OFFICIAL_VERIFIER = ROOT / "scripts/verify_playwright_python_official_provenance.py"
PROMOTER = (
    ROOT / "scripts/promote_playwright_python_navigation_frame_strict_paired_v2.py"
)

MODES = ("candidate", "oracle")
RECORDS = 6_526
TARGET_INDEXES = (2686, 2689, 2692, 2695, 2698)
TARGET_IDS = (
    "tests/async/test_navigation.py::test_frame_goto_should_navigate_subframes[chromium]",
    "tests/async/test_navigation.py::test_frame_goto_should_reject_when_frame_detaches[chromium]",
    "tests/async/test_navigation.py::test_frame_wait_for_load_state_should_work[chromium]",
    "tests/async/test_navigation.py::test_frame_wait_for_nav_should_fail_when_frame_detaches[chromium]",
    "tests/async/test_navigation.py::test_frame_wait_for_nav_should_work[chromium]",
)
REPORT_SELECTIONS = (
    TARGET_IDS[0],
    TARGET_IDS[1],
    TARGET_IDS[2],
    TARGET_IDS[4],
    TARGET_IDS[3],
)
GENERATED = frozenset({"attempts", "durationSeconds", "error", "outcome", "resultFile"})
KIND = "playwright-python-navigation-frame-chromium-strict-paired-promotion-v2"
SCOPE = "tests/async/test_navigation.py-chromium-frame-five"
COMMIT = "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
INVENTORY_SHA256 = "3dfb11b6ad7eb4736166ee2c4f7fa32cda78063324ad8314ce12f3e2d49b9d41"
MANIFEST_SHA256 = "6d4f86fee517e300319cfa87014a54302980bdf8cf538fa21c1daff4219ea724"
NAVIGATION_SHA256 = "f04af8cbdee3dffb2c578d61d7685f02e5186ca565906323d71b6f92f96cd144"
BASE_VERIFIER_SHA256 = (
    "ca173a347232b9f02a1cc1df8735461f889c24919375a1492803780e80f0f650"
)
OFFICIAL_VERIFIER_SHA256 = (
    "cb451d9977ff574efb0778b842ff8662a95471fc2d1ca5dff450bef39c6e50d7"
)
BASE_RECEIPT_SHA256 = "6998ef7e55f2e24e879f515eb1e1f533bc4e427f67a68b911dc0a6701428f5e1"
BASE_PROMOTER_SHA256 = (
    "27e5ce34093865e6b4d2c127a11e5733fcb351bd26a50285ef0f08c76c72d384"
)
BASE_HASHES = {
    "candidate": (
        "03704b471a45711121894424f7024cbf21ba49e91a59b0d8c13d7845a4f38e71",
        "cc1183e3d3a74d49e301d526127429673b4115245a8a4cccf2bb9a2b63f8e1e9",
    ),
    "oracle": (
        "b209bf22f7777283798842abfd2a9774e4bb8f1659ae1b67e6015a3dc5a22479",
        "649634a74d79ab2b0019db2d7056ceef8d9f72f96c5451dc77f26ed770f3d011",
    ),
}
WHEEL_SHA256 = "b3bf0fab066e6513db8b5d4ba9bfa555b18c726097689c4a5418ed6c675a5d0f"
RESULT_SHA256 = "ba396965142d2eaf3446d90604651bbcd8345793c0e96b69b8951347fb17e0f2"
PROVENANCE_SHA256 = "9b44b8f0ad4d869334b9cc6ebb5149fae8ae9b1e8506b3392d78ad63f6a681f3"
CASE_REPORTS_SHA256 = "df6fb82df06bc0c1ed3e43e1992641b90267c70f392cfe1053fac538c37427db"
ISOLATED_IDS_SHA256 = "f0115e2aa61824cf677ca4a4c940ee772e119bb16cec138c3a28018dae8578b5"
PWRIGHT_ARTIFACT_SHA256 = (
    "9a659567aaed311d3a947199944fbcda6724ab4d6c349e2534181cbab5241124"
)
PWRIGHT_PACKAGE_SHA256 = (
    "506fb374098c906de43f2bb940ea53aae3be3390de48746b9dc87c18a7f1ff24"
)
SOURCE_PINS_SHA256 = "c08f36d30793a19a822d24226a57443db3d1add1b369e33a64fe6e7829d3b639"


class FrameNavigationValidationError(ValueError):
    """An immutable source, report, or successor differs from its contract."""


def fail(message: str) -> NoReturn:
    raise FrameNavigationValidationError(message)


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
        raise FrameNavigationValidationError(f"{label} is invalid JSON") from error
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
    return f"playwright-python-{mode}-navigation-full-chromium-strict-paired-v1.jsonl"


def output_name(mode: str) -> str:
    require(mode in MODES, "unknown lane")
    return f"playwright-python-{mode}-navigation-frame-chromium-strict-paired-v2.jsonl"


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
    target_indexes: list[int],
    expected_cases: dict[str, dict[str, object]],
    result_file: str,
    mode: str,
) -> Counter[str]:
    """Rebuild five candidate rows and preserve exact off-target/oracle bytes.

    The synthetic arguments make this kernel independently mutation-testable.
    The fixed inventory, target IDs, and source-bound report are checked by
    ``verify_bundle`` before calling it.
    """
    require(mode in MODES, "ledger mode differs")
    require(
        len(base_lines) == len(successor_lines) == len(inventory_rows),
        f"{mode} ledger length differs",
    )
    require(
        len(target_indexes) == len(set(target_indexes))
        and all(0 <= index < len(inventory_rows) for index in target_indexes),
        "target indexes are missing, duplicated or out of bounds",
    )
    ids = [row.get("id") for row in inventory_rows]
    require(
        all(isinstance(test_id, str) for test_id in ids), "inventory ID is malformed"
    )
    require(len(set(ids)) == len(ids), "inventory has duplicate IDs")
    target_ids = [ids[index] for index in target_indexes]
    require(set(expected_cases) == set(target_ids), "report target ID set differs")
    target_set = set(target_indexes)
    counts: Counter[str] = Counter()
    for index, (before_raw, after_raw, source) in enumerate(
        zip(base_lines, successor_lines, inventory_rows)
    ):
        before = json_object(before_raw, f"{mode} base row {index}")
        after = json_object(after_raw, f"{mode} successor row {index}")
        require(before.get("id") == ids[index], f"{mode} base ID moved at {index}")
        require(_metadata(before) == source, f"{mode} base metadata differs at {index}")
        require(after.get("id") == ids[index], f"{mode} successor ID moved at {index}")
        require(
            _metadata(after) == source, f"{mode} successor metadata differs at {index}"
        )
        if mode == "oracle" or index not in target_set:
            require(after_raw == before_raw, f"{mode} raw row changed at {index}")
        else:
            require(
                source.get("sourceClassification") == "public-api-source",
                "candidate target source classification differs",
            )
            require(
                before.get("outcome") == "failed" and before.get("attempts") == 1,
                f"candidate prior target status differs at {index}",
            )
            case = expected_cases[ids[index]]
            require(
                case.get("outcome") == "passed",
                f"candidate report outcome differs at {index}",
            )
            duration = case.get("durationSeconds")
            require(
                type(duration) is float and math.isfinite(duration) and duration >= 0,
                f"candidate report duration differs at {index}",
            )
            expected: dict[str, object] = {
                **source,
                "attempts": 1,
                "durationSeconds": duration,
                "outcome": "passed",
                "resultFile": result_file,
            }
            require(after == expected, f"candidate promoted row differs at {index}")
            canonical = (
                json.dumps(expected, sort_keys=True, separators=(",", ":")) + "\n"
            ).encode()
            require(
                after_raw == canonical,
                f"candidate promoted row bytes differ at {index}",
            )
        status = after.get("outcome")
        require(isinstance(status, str), f"{mode} row outcome malformed at {index}")
        counts[status] += 1
    return counts


def _junit_cases(raw: bytes) -> dict[str, dict[str, object]]:
    try:
        tree = ET.fromstring(raw)
    except ET.ParseError as error:
        raise FrameNavigationValidationError(
            "candidate result is invalid XML"
        ) from error
    cases = list(tree.iter("testcase"))
    require(
        len(cases) == len(TARGET_IDS), "focused JUnit does not have exactly five cases"
    )
    observed: dict[str, dict[str, object]] = {}
    for case in cases:
        require(
            case.get("classname") == "tests.async.test_navigation",
            "focused JUnit case is outside pinned source",
        )
        name = case.get("name")
        require(
            isinstance(name, str) and name.endswith("[chromium]"),
            "focused JUnit case name differs",
        )
        test_id = f"tests/async/test_navigation.py::{name}"
        require(test_id not in observed, f"focused JUnit duplicate ID {test_id}")
        try:
            duration = float(case.get("time", ""))
        except ValueError as error:
            raise FrameNavigationValidationError(
                "focused JUnit duration is invalid"
            ) from error
        require(
            math.isfinite(duration) and duration >= 0,
            "focused JUnit duration is invalid",
        )
        require(
            not case.findall("failure")
            and not case.findall("error")
            and not case.findall("skipped"),
            f"focused JUnit case did not pass: {test_id}",
        )
        observed[test_id] = {"outcome": "passed", "durationSeconds": duration}
    require(tuple(observed) == TARGET_IDS, "focused JUnit IDs/order differ")
    return observed


def _verify_base() -> tuple[
    dict[str, Any], list[dict[str, Any]], dict[str, list[bytes]]
]:
    require(
        Path(prior.__file__).resolve() == BASE_VERIFIER, "v1 verifier module differs"
    )
    sealed(BASE_VERIFIER, BASE_VERIFIER_SHA256, "v1 verifier source")
    base_receipt = json_object(
        sealed(BASE_DIR / "promotion-receipt.json", BASE_RECEIPT_SHA256, "v1 receipt"),
        "v1 receipt",
    )
    require(
        prior.verify_bundle(BASE_DIR) == base_receipt, "v1 chain verification differs"
    )
    _, inventory = jsonl(
        sealed(INVENTORY, INVENTORY_SHA256, "profiled inventory"), "inventory"
    )
    require(
        tuple(inventory[index].get("id") for index in TARGET_INDEXES) == TARGET_IDS,
        "five inventory target IDs moved",
    )
    base_lines: dict[str, list[bytes]] = {}
    for mode in MODES:
        ledger = BASE_DIR / base_name(mode)
        ledger_sha, summary_sha = BASE_HASHES[mode]
        lines, rows = jsonl(
            sealed(ledger, ledger_sha, f"{mode} v1 ledger"), f"{mode} v1 ledger"
        )
        summary = json_object(
            sealed(
                ledger.with_suffix(".summary.json"), summary_sha, f"{mode} v1 summary"
            ),
            f"{mode} v1 summary",
        )
        require(
            base_receipt.get("successors", {}).get(mode) == summary,
            f"{mode} v1 receipt summary differs",
        )
        require(
            summary.get("ledgerSha256") == ledger_sha
            and summary.get("records") == RECORDS,
            f"{mode} v1 summary differs",
        )
        if mode == "candidate":
            require(
                all(rows[index].get("outcome") == "failed" for index in TARGET_INDEXES),
                "five v1 candidate failures moved",
            )
        else:
            require(
                all(rows[index].get("outcome") == "passed" for index in TARGET_INDEXES),
                "five v1 oracle passes moved",
            )
        base_lines[mode] = lines
    return base_receipt, inventory, base_lines


def check_report_metadata(payload: dict[str, Any]) -> None:
    """Reject a report outside the exact frozen five-case capture scope."""
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
        and payload.get("isolatedCaseCount") == 5
        and payload.get("selections") == list(REPORT_SELECTIONS)
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
        "focused provenance identity or execution scope differs",
    )


def _verify_report() -> tuple[dict[str, Any], dict[str, dict[str, object]]]:
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
        raise FrameNavigationValidationError(
            f"focused official provenance failed: {error}"
        ) from error
    check_report_metadata(payload)
    return payload, _junit_cases(result_raw)


def _verify_wheel(provenance: dict[str, Any]) -> None:
    wheel_raw = sealed(WHEEL, WHEEL_SHA256, "frozen ABI128 wheel")
    # The source-bound report records the installed package and native artifact.
    # Match every wheel-supplied facade file to that captured installed package.
    package = provenance.get("pwrightPackage")
    require(isinstance(package, str), "report installed package path missing")
    package_path = Path(package)
    require(
        package_path.is_dir() and not package_path.is_symlink(),
        "report installed package missing or linked",
    )
    try:
        with zipfile.ZipFile(io.BytesIO(wheel_raw)) as archive:
            native = archive.read("pwright/_native.abi3.so")
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
            for item in files:
                relative = Path(item.filename.removeprefix("pwright/"))
                require(
                    relative.parts
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
    except (OSError, KeyError, zipfile.BadZipFile) as error:
        raise FrameNavigationValidationError(
            "frozen wheel has no valid native artifact"
        ) from error
    require(bool(wheel_raw), "frozen wheel is empty")
    require(sha256(native) == PWRIGHT_ARTIFACT_SHA256, "wheel/native artifact differs")
    require(
        provenance.get("pwrightArtifactSha256") == sha256(native),
        "report/wheel native artifact differs",
    )


def check_source_pins(source_pins: dict[str, Any], provenance: dict[str, Any]) -> None:
    """Require the exact wheel/report identity, not self-attested metadata."""
    require(type(source_pins.get("schema")) is int, "v2 source-pin schema type differs")
    for field in (
        "harnessInputs",
        "toolingSources",
        "pwrightRuntimeLibraries",
    ):
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
        "kind": "playwright-python-navigation-frame-five-candidate",
        "upstreamCommit": COMMIT,
        "upstreamNavigationSha256": NAVIGATION_SHA256,
        "officialManifestSha256": MANIFEST_SHA256,
        "profiledInventorySha256": INVENTORY_SHA256,
        "frozenWheel": str(WHEEL),
        "frozenWheelSha256": WHEEL_SHA256,
        "wheelName": WHEEL.name,
        "installedNativeAbi": 128,
        "pwrightArtifactSha256": PWRIGHT_ARTIFACT_SHA256,
        "pwrightPackageSha256": PWRIGHT_PACKAGE_SHA256,
        "playwrightPackageSha256": package_sha,
        "harnessInputs": provenance["harnessInputs"],
        "toolingSources": provenance["toolingSources"],
        "pwrightRuntimeLibraries": provenance["pwrightRuntimeLibraries"],
    }
    require(source_pins == expected, "v2 source-pin identity differs")


def verify_bundle(output_dir: Path = OUTPUT_DIR) -> dict[str, Any]:
    """Verify one fixed successor and its immutable v1/source-bound inputs."""
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
    provenance, cases = _verify_report()
    _verify_wheel(provenance)
    sealed(MANIFEST, MANIFEST_SHA256, "official manifest")
    sealed(NAVIGATION_SOURCE, NAVIGATION_SHA256, "upstream navigation source")
    source_pins = json_object(
        sealed(SOURCE_PINS, SOURCE_PINS_SHA256, "v2 source pins"), "v2 source pins"
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
        and receipt.get("targetZeroBasedIndexes") == list(TARGET_INDEXES)
        and receipt.get("targetTestIds") == list(TARGET_IDS),
        "promotion receipt source or target identity differs",
    )
    require(base_receipt.get("successors") is not None, "v1 receipt has no successors")
    promoter_sha = receipt.get("promoterSha256")
    require(isinstance(promoter_sha, str), "promoter source SHA missing")
    sealed(PROMOTER, promoter_sha, "v2 promoter source")
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
            "observedOutcomes": {"passed": 5},
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
            list(TARGET_INDEXES),
            cases,
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
        FrameNavigationValidationError,
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
