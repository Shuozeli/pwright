"""Independently verify the full-80 Python Chromium navigation v3 successor.

The immutable v2 pair is the base. All 80 candidate navigation rows are
reconstructed from one source-bound ABI128-v5 report, including the single
synthetic report error; 6,446 other candidate rows and all 6,526 oracle rows
must keep their exact v2 bytes. This verifier never imports the v3 publisher.
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

import verify_playwright_python_navigation_frame_strict_paired_v2 as prior
import verify_playwright_python_official_provenance as official

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / ".work/official-conformance"
BUILD = WORK / "builds/python-frame-navigation-postfix-oct20.6zcD6U"
BASE_DIR = WORK / "outcomes/python-navigation-frame-chromium-strict-paired-v2"
OUTPUT_DIR = WORK / "outcomes/python-navigation-full-chromium-strict-paired-v3"
INVENTORY = WORK / "playwright-python-v1.58.0-profiled.jsonl"
MANIFEST = ROOT / "packages/pwright-python/tests/official/manifest.json"
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-python")
NAVIGATION_SOURCE = UPSTREAM / "tests/async/test_navigation.py"
SOURCE_PINS = OUTPUT_DIR / "source-pins.json"
RESULT = BUILD / "results/navigation-candidate-full-abi128-v5.xml"
PROVENANCE = RESULT.with_suffix(".provenance.json")
CANDIDATE_VENV = BUILD / "candidate-abi128-v5-full-nav"
CANDIDATE_PACKAGE = CANDIDATE_VENV / "lib/python3.12/site-packages/pwright"
CANDIDATE_ARTIFACT = CANDIDATE_PACKAGE / "_native.abi3.so"
PYTHON_EXECUTABLE = CANDIDATE_VENV / "bin/python"
WHEEL = BUILD / "wheel-abi128-v5/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl"
BASE_VERIFIER = (
    ROOT / "scripts/verify_playwright_python_navigation_frame_strict_paired_v2.py"
)
BASE_PROMOTER = (
    ROOT / "scripts/promote_playwright_python_navigation_frame_strict_paired_v2.py"
)
OFFICIAL_VERIFIER = ROOT / "scripts/verify_playwright_python_official_provenance.py"
PROMOTER = (
    ROOT / "scripts/promote_playwright_python_navigation_full_strict_paired_v3.py"
)

MODES = ("candidate", "oracle")
RECORDS = 6_526
TARGET_INDEXES = tuple(range(2680, 2918, 3))
ERROR_ID = "tests/async/test_navigation.py::test_frame_goto_should_continue_after_client_redirect[chromium]"
ERROR_INDEX = 2683
ERROR_RAW_SHA256 = "d79a773ce3dc827914b0008c91fb6575c125bc0a0205b205ff9f500e8bc699c6"
GENERATED = frozenset({"attempts", "durationSeconds", "error", "outcome", "resultFile"})
KIND = "playwright-python-navigation-full-chromium-strict-paired-promotion-v3"
SCOPE = "tests/async/test_navigation.py-chromium-full-file-80"
COMMIT = "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
INVENTORY_SHA256 = "3dfb11b6ad7eb4736166ee2c4f7fa32cda78063324ad8314ce12f3e2d49b9d41"
MANIFEST_SHA256 = "6d4f86fee517e300319cfa87014a54302980bdf8cf538fa21c1daff4219ea724"
NAVIGATION_SHA256 = "f04af8cbdee3dffb2c578d61d7685f02e5186ca565906323d71b6f92f96cd144"
BASE_VERIFIER_SHA256 = (
    "a1c5e547f8f599a05c3023c3c4f02e605f2853b34c1a2efcdcfcdfe2a34826c9"
)
BASE_PROMOTER_SHA256 = (
    "a350549a3a40e80660610af0ce80e02bbc83840abdad5c00575f3a7b9764db91"
)
OFFICIAL_VERIFIER_SHA256 = (
    "cb451d9977ff574efb0778b842ff8662a95471fc2d1ca5dff450bef39c6e50d7"
)
BASE_RECEIPT_SHA256 = "a35ba350fdd40a0e91251980fc98cc7d0f461f009268c616fe54cdf052660139"
BASE_HASHES = {
    "candidate": (
        "792944338f6092a3ca538102ad65b5545e9320a615e1ef3adf89b7e5b434f002",
        "b1f084795f1e818cba17db3a5bb37fa0422f776acae855834359fcfb93d47b21",
    ),
    "oracle": (
        "b209bf22f7777283798842abfd2a9774e4bb8f1659ae1b67e6015a3dc5a22479",
        "954069e28e225b0be6493e2a32d84b69823432946e3990571850844a354477a7",
    ),
}
WHEEL_SHA256 = "fb342855ca70473c59a84729cce4b44eafea0cb3b3b506929831e88385bce108"
RESULT_SHA256 = "8082a55316d549b02a5c21fc1311224b5e6eda6476978e65a94f4d5d78e4d193"
PROVENANCE_SHA256 = "9098f4ee457aabacae159351b42c6abc60cad40d254dd4380088aa7b1a9c90b6"
CASE_REPORTS_SHA256 = "b996b2202b06122b6eae4c70d0c1ba7a862e437886e350d4e1f537d2b6c3d440"
ISOLATED_IDS_SHA256 = "4b9f37f3a3857ddf59e9b5766255f84dfe83d3d5af76285208e176088f656696"
PWRIGHT_ARTIFACT_SHA256 = (
    "9a659567aaed311d3a947199944fbcda6724ab4d6c349e2534181cbab5241124"
)
PWRIGHT_PACKAGE_SHA256 = (
    "0070c191a57572e533940f56fb41761a51926a80f694cd0017c6cd5f6b672513"
)
SOURCE_PINS_SHA256 = "28dd1eb8e6d3539b5b716297a7ceb4e0aeb969877c05ff5c482634b700568226"


class FullNavigationValidationError(ValueError):
    """An immutable source, report, or successor differs from its contract."""


def fail(message: str) -> NoReturn:
    raise FullNavigationValidationError(message)


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
        raise FullNavigationValidationError(f"{label} is invalid JSON") from error
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
    return f"playwright-python-{mode}-navigation-frame-chromium-strict-paired-v2.jsonl"


def output_name(mode: str) -> str:
    require(mode in MODES, "unknown lane")
    return f"playwright-python-{mode}-navigation-full-chromium-strict-paired-v3.jsonl"


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
        ("completeFileExecution", True),
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
    """Rebuild candidate targets and retain exact off-target/oracle raw bytes."""
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
                before.get("outcome") in {"passed", "failed"}
                and before.get("attempts") == 1,
                f"candidate prior target status differs at {index}",
            )
            case = expected_cases[ids[index]]
            outcome = case.get("outcome")
            duration = case.get("durationSeconds")
            require(
                outcome in {"passed", "failed", "error"},
                f"candidate report status differs at {index}",
            )
            require(
                type(duration) is float and math.isfinite(duration) and duration >= 0,
                f"candidate report duration differs at {index}",
            )
            expected: dict[str, object] = {
                **source,
                "attempts": 1,
                "durationSeconds": duration,
                "outcome": outcome,
                "resultFile": result_file,
            }
            if outcome != "passed":
                message = case.get("error")
                require(
                    isinstance(message, str) and bool(message),
                    f"candidate report error missing at {index}",
                )
                expected["error"] = message
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


def _junit_cases(raw: bytes, target_ids: list[str]) -> dict[str, dict[str, object]]:
    try:
        tree = ET.fromstring(raw)
    except ET.ParseError as error:
        raise FullNavigationValidationError("full-80 JUnit is invalid XML") from error
    cases = list(tree.iter("testcase"))
    require(len(cases) == 80, "full-80 JUnit does not have exactly 80 cases")
    observed: dict[str, dict[str, object]] = {}
    for case in cases:
        require(
            case.get("classname") == "tests.async.test_navigation",
            "full-80 JUnit case is outside pinned source",
        )
        name = case.get("name")
        require(
            isinstance(name, str) and name.endswith("[chromium]"),
            "full-80 JUnit case name differs",
        )
        test_id = f"tests/async/test_navigation.py::{name}"
        require(test_id not in observed, f"full-80 JUnit duplicate ID {test_id}")
        try:
            duration = float(case.get("time", ""))
        except ValueError as error:
            raise FullNavigationValidationError(
                "full-80 JUnit duration is invalid"
            ) from error
        require(
            math.isfinite(duration) and duration >= 0,
            "full-80 JUnit duration is invalid",
        )
        failures = case.findall("failure")
        errors = case.findall("error")
        skips = case.findall("skipped")
        require(
            len(failures) + len(errors) + len(skips) <= 1 and not skips,
            "full-80 JUnit status is ambiguous",
        )
        status = "error" if errors else "failed" if failures else "passed"
        row: dict[str, object] = {"outcome": status, "durationSeconds": duration}
        if status != "passed":
            node = (errors or failures)[0]
            message = node.get("message") or node.text or ""
            first_line = message.splitlines()
            require(
                bool(first_line and first_line[0]),
                "full-80 failure/error message missing",
            )
            row["error"] = first_line[0]
        observed[test_id] = row
    require(
        list(observed) == target_ids, "full-80 JUnit IDs/order differ from inventory"
    )
    require(
        Counter(case["outcome"] for case in observed.values())
        == {"passed": 72, "failed": 7, "error": 1},
        "full-80 JUnit outcome counts differ",
    )
    require(
        observed[ERROR_ID]["outcome"] == "error", "synthetic error was reclassified"
    )
    diagnostic = observed[ERROR_ID].get("error")
    require(
        isinstance(diagnostic, str)
        and diagnostic.startswith("isolated pytest report was invalid after exit 1:")
        and ERROR_RAW_SHA256 in diagnostic,
        "synthetic isolated-report error differs",
    )
    return observed


def _verify_base() -> tuple[
    dict[str, Any], list[dict[str, Any]], dict[str, list[bytes]]
]:
    require(
        Path(prior.__file__).resolve() == BASE_VERIFIER, "v2 verifier module differs"
    )
    sealed(BASE_VERIFIER, BASE_VERIFIER_SHA256, "v2 verifier source")
    sealed(BASE_PROMOTER, BASE_PROMOTER_SHA256, "v2 promoter source")
    base_receipt = json_object(
        sealed(BASE_DIR / "promotion-receipt.json", BASE_RECEIPT_SHA256, "v2 receipt"),
        "v2 receipt",
    )
    require(
        prior.verify_bundle(BASE_DIR) == base_receipt, "v2 chain verification differs"
    )
    _, inventory = jsonl(
        sealed(INVENTORY, INVENTORY_SHA256, "profiled inventory"), "inventory"
    )
    target_ids = [inventory[index].get("id") for index in TARGET_INDEXES]
    require(
        len(target_ids) == 80
        and all(
            isinstance(test_id, str)
            and test_id.startswith("tests/async/test_navigation.py::")
            and test_id.endswith("[chromium]")
            for test_id in target_ids
        )
        and target_ids[1] == ERROR_ID,
        "full-80 inventory target IDs moved",
    )
    require(base_receipt.get("successors") is not None, "v2 receipt has no successors")
    base_lines: dict[str, list[bytes]] = {}
    for mode in MODES:
        ledger = BASE_DIR / base_name(mode)
        ledger_sha, summary_sha = BASE_HASHES[mode]
        lines, rows = jsonl(
            sealed(ledger, ledger_sha, f"{mode} v2 ledger"), f"{mode} v2 ledger"
        )
        summary = json_object(
            sealed(
                ledger.with_suffix(".summary.json"), summary_sha, f"{mode} v2 summary"
            ),
            f"{mode} v2 summary",
        )
        require(
            base_receipt["successors"].get(mode) == summary,
            f"{mode} v2 receipt summary differs",
        )
        require(
            summary.get("ledgerSha256") == ledger_sha
            and summary.get("records") == RECORDS,
            f"{mode} v2 summary differs",
        )
        statuses = Counter(rows[index].get("outcome") for index in TARGET_INDEXES)
        expected = (
            {"passed": 62, "failed": 18}
            if mode == "candidate"
            else {"passed": 77, "failed": 3}
        )
        require(statuses == expected, f"{mode} v2 full-80 status slice differs")
        base_lines[mode] = lines
    return base_receipt, inventory, base_lines


def check_report_metadata(payload: dict[str, Any]) -> None:
    """Reject a report outside the exact frozen full-file Chromium run."""
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
        and payload.get("isolatedCaseCount") == 80
        and payload.get("selections") == ["tests/async/test_navigation.py"]
        and payload.get("browsers") == ["chromium"]
        and payload.get("launchMode") == "cdp-adapter"
        and payload.get("isolateCases") is True
        and payload.get("resumeIsolated") is False
        and payload.get("collectOnly") is False
        and payload.get("bootstrapOnly") is False
        and payload.get("manifestCases") is False
        and payload.get("secureHttpFixture") is False
        and payload.get("exitCode") == 1
        and payload.get("pythonExecutable") == str(PYTHON_EXECUTABLE)
        and payload.get("pwrightPackage") == str(CANDIDATE_PACKAGE)
        and payload.get("pwrightArtifact") == str(CANDIDATE_ARTIFACT)
        and payload.get("pwrightArtifactSha256") == PWRIGHT_ARTIFACT_SHA256
        and payload.get("pwrightPackageSha256") == PWRIGHT_PACKAGE_SHA256,
        "full-80 provenance identity or execution scope differs",
    )


def _verify_report(
    target_ids: list[str],
) -> tuple[dict[str, Any], dict[str, dict[str, object]]]:
    require(
        Path(official.__file__).resolve() == OFFICIAL_VERIFIER,
        "official verifier module differs",
    )
    sealed(OFFICIAL_VERIFIER, OFFICIAL_VERIFIER_SHA256, "official verifier source")
    payload = json_object(
        sealed(PROVENANCE, PROVENANCE_SHA256, "full-80 provenance"),
        "full-80 provenance",
    )
    result_raw = sealed(RESULT, RESULT_SHA256, "full-80 JUnit")
    try:
        official.verify_provenance(payload, provenance_path=PROVENANCE)
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        raise FullNavigationValidationError(
            f"full-80 official provenance failed: {error}"
        ) from error
    check_report_metadata(payload)
    return payload, _junit_cases(result_raw, target_ids)


def _verify_wheel(provenance: dict[str, Any]) -> None:
    wheel_raw = sealed(WHEEL, WHEEL_SHA256, "frozen ABI128-v5 wheel")
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
            digest = hashlib.sha256()
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
            require(
                b"NATIVE_ABI_VERSION = 128" in archive.read("pwright/_abi.py"),
                "wheel ABI is not 128",
            )
    except (OSError, KeyError, zipfile.BadZipFile) as error:
        raise FullNavigationValidationError(
            "frozen wheel has no valid native artifact"
        ) from error
    require(sha256(native) == PWRIGHT_ARTIFACT_SHA256, "wheel/native artifact differs")
    require(
        provenance.get("pwrightArtifactSha256") == sha256(native),
        "report/wheel native artifact differs",
    )


def check_source_pins(source_pins: dict[str, Any], provenance: dict[str, Any]) -> None:
    """Require the exact ABI128-v5 wheel/report identity and 80-case scope."""
    require(type(source_pins.get("schema")) is int, "v3 source-pin schema type differs")
    for field in ("harnessInputs", "toolingSources", "pwrightRuntimeLibraries"):
        require(
            isinstance(provenance.get(field), list) and bool(provenance[field]),
            f"full-80 report {field} is missing",
        )
    package_sha = provenance.get("playwrightPackageSha256")
    require(
        isinstance(package_sha, str)
        and len(package_sha) == 64
        and all(character in "0123456789abcdef" for character in package_sha),
        "full-80 report Playwright package SHA is invalid",
    )
    expected = {
        "schema": 1,
        "kind": "playwright-python-navigation-full-file-candidate-v3",
        "upstreamCommit": COMMIT,
        "upstreamNavigationSha256": NAVIGATION_SHA256,
        "officialManifestSha256": MANIFEST_SHA256,
        "profiledInventorySha256": INVENTORY_SHA256,
        "chromiumExpandedCases": 80,
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
    require(source_pins == expected, "v3 source-pin identity differs")


def verify_bundle(output_dir: Path = OUTPUT_DIR) -> dict[str, Any]:
    """Verify one fixed v3 successor and all immutable v2/source-bound inputs."""
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
    target_ids = [inventory[index]["id"] for index in TARGET_INDEXES]
    provenance, cases = _verify_report(target_ids)
    _verify_wheel(provenance)
    sealed(MANIFEST, MANIFEST_SHA256, "official manifest")
    sealed(NAVIGATION_SOURCE, NAVIGATION_SHA256, "upstream navigation source")
    source_pins = json_object(
        sealed(SOURCE_PINS, SOURCE_PINS_SHA256, "v3 source pins"), "v3 source pins"
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
        and receipt.get("targetTestIds") == target_ids,
        "promotion receipt source or target identity differs",
    )
    require(base_receipt.get("successors") is not None, "v2 receipt has no successors")
    promoter_sha = receipt.get("promoterSha256")
    require(isinstance(promoter_sha, str), "promoter source SHA missing")
    sealed(PROMOTER, promoter_sha, "v3 promoter source")
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
            "observedOutcomes": {"passed": 72, "failed": 7, "error": 1},
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
        expected_counts = (
            {"error": 4, "failed": 109, "not-run": 4764, "passed": 1612, "skipped": 37}
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
        FullNavigationValidationError,
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
