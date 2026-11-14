"""Independently verify the full-80 Python Chromium navigation v6 successor.

The immutable v5 pair is the base. All 80 candidate navigation rows are
reconstructed from one source-bound ABI130-v2 report, including the single
synthetic report error; 6,446 other candidate rows and all 6,526 oracle rows
must keep their exact v5 bytes. This verifier never imports the v6 publisher.
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

import verify_playwright_python_navigation_url_match_strict_paired_v5 as prior
import verify_playwright_python_official_provenance as official

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / ".work/official-conformance"
BUILD = WORK / "builds/python-navigation-url-match-abi130-v2-oct20.srSIiv"
BASE_DIR = WORK / "outcomes/python-navigation-url-match-chromium-strict-paired-v5"
OUTPUT_DIR = WORK / "outcomes/python-navigation-full-chromium-strict-paired-v6"
INVENTORY = WORK / "playwright-python-v1.58.0-profiled.jsonl"
MANIFEST = ROOT / "packages/pwright-python/tests/official/manifest.json"
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-python")
NAVIGATION_SOURCE = UPSTREAM / "tests/async/test_navigation.py"
SOURCE_PINS = OUTPUT_DIR / "source-pins.json"
RESULT = BUILD / "results/navigation-candidate-full80-abi130-v2.xml"
PROVENANCE = RESULT.with_suffix(".provenance.json")
IDENTITY = BUILD / "results/abi130-v2-identity.json"
CANDIDATE_VENV = BUILD / "candidate-abi130-v2"
CANDIDATE_PACKAGE = CANDIDATE_VENV / "lib/python3.12/site-packages/pwright"
CANDIDATE_ARTIFACT = CANDIDATE_PACKAGE / "_native.abi3.so"
PYTHON_EXECUTABLE = CANDIDATE_VENV / "bin/python"
WHEEL = BUILD / "wheel/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl"
BASE_VERIFIER = (
    ROOT / "scripts/verify_playwright_python_navigation_url_match_strict_paired_v5.py"
)
BASE_PROMOTER = (
    ROOT / "scripts/promote_playwright_python_navigation_url_match_strict_paired_v5.py"
)
OFFICIAL_VERIFIER = ROOT / "scripts/verify_playwright_python_official_provenance.py"
PROMOTER = (
    ROOT / "scripts/promote_playwright_python_navigation_full_strict_paired_v6.py"
)

MODES = ("candidate", "oracle")
RECORDS = 6_526
TARGET_INDEXES = tuple(range(2680, 2918, 3))
ERROR_ID = "tests/async/test_navigation.py::test_wait_for_nav_should_work_when_subframe_issues_window_stop[chromium]"
ERROR_INDEX = 2890
ERROR_RAW_SHA256 = "b02a6a718c82c6806367fcc6ac8cf98717b8a5c5388b863b381e8758413b66b3"
PRIOR_ERROR_INDEX = 2683
GENERATED = frozenset({"attempts", "durationSeconds", "error", "outcome", "resultFile"})
KIND = "playwright-python-navigation-full-chromium-strict-paired-promotion-v6"
SCOPE = "tests/async/test_navigation.py-chromium-full-file-80"
COMMIT = "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
INVENTORY_SHA256 = "3dfb11b6ad7eb4736166ee2c4f7fa32cda78063324ad8314ce12f3e2d49b9d41"
MANIFEST_SHA256 = "6d4f86fee517e300319cfa87014a54302980bdf8cf538fa21c1daff4219ea724"
NAVIGATION_SHA256 = "f04af8cbdee3dffb2c578d61d7685f02e5186ca565906323d71b6f92f96cd144"
BASE_VERIFIER_SHA256 = (
    "356e8258c25aee4758cd5c34ffcf5c9d38ef09735ccbb0c508249aa325c251ad"
)
BASE_PROMOTER_SHA256 = (
    "694bbe6f368ce549049b0d0e1fcab33dc060222b049e81923a37645ae76499fb"
)
OFFICIAL_VERIFIER_SHA256 = (
    "cb451d9977ff574efb0778b842ff8662a95471fc2d1ca5dff450bef39c6e50d7"
)
BASE_RECEIPT_SHA256 = "3c2d07e5d788b5ede07145e69ec78a6848045b3145a5772e61db55cf667145e6"
BASE_HASHES = {
    "candidate": (
        "b985acb23344b379c1e3615dd040f0da821916f52d782c2d4f0b9ac477f2e661",
        "5bb2acd38c4b7dbcc7c5cb5a66314cc68d7d3a6aea4fd0ba1589cf5925b24f79",
    ),
    "oracle": (
        "b209bf22f7777283798842abfd2a9774e4bb8f1659ae1b67e6015a3dc5a22479",
        "9aab8096d2d6902e0bb2c7c86cafd44f9b8d2d703e480b5499fb9165ec2e1c81",
    ),
}
WHEEL_SHA256 = "40889db525656b5410a994c81e79530f873531c28ac0e8c9052e8603a87efb78"
RESULT_SHA256 = "58c4769fbe08476f961ed5b17ef1e24734fe36847d716ffa5be02134de974164"
PROVENANCE_SHA256 = "72e7ea0f8b21427cad4b38bc834a245dfc97501e79cdd9564e80677cffb0be31"
CASE_REPORTS_SHA256 = "3bc05e94ea64fbb3d455d9bab8dec091e08b504210c2e77ea2bf7e9b268f354e"
ISOLATED_IDS_SHA256 = "4b9f37f3a3857ddf59e9b5766255f84dfe83d3d5af76285208e176088f656696"
PWRIGHT_ARTIFACT_SHA256 = (
    "ef567004a447ee148e988961b04868a8d1063f885c816d10845a10e5c6c36df8"
)
PWRIGHT_PACKAGE_SHA256 = (
    "c66d04b01ca060ccbaf31f2aed127b51aebe34834d8b8b0246391f5fb489446e"
)
PLAYWRIGHT_PACKAGE_SHA256 = (
    "e3a7333b66af39f60b8fc68ea8ff177604a914ac2380a99f17ad596451c886e9"
)
IDENTITY_SHA256 = "aeb78b8ff2a6acd554b210491f8b73e9e27cae47faa29ddd128106b38ae7f2f7"
PROMOTER_SHA256 = "1fd8223ee374c04dd52c7fc018ab5928b3c8c3416e3ef4b27519c234f05cea45"
SOURCE_PINS_SHA256 = "1313c1a4deb806b3897ace706080ad56abb8b212dbbb8f0bf78932d7a5ec6f84"


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
    return (
        f"playwright-python-{mode}-navigation-url-match-chromium-strict-paired-v5.jsonl"
    )


def output_name(mode: str) -> str:
    require(mode in MODES, "unknown lane")
    return f"playwright-python-{mode}-navigation-full-chromium-strict-paired-v6.jsonl"


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
                before.get("outcome") in {"passed", "failed", "error"}
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
        == {"passed": 77, "failed": 2, "error": 1},
        "full-80 JUnit outcome counts differ",
    )
    require(
        observed[ERROR_ID]["outcome"] == "error", "synthetic error was reclassified"
    )
    require(
        observed[target_ids[1]]["outcome"] == "passed",
        "v5 synthetic error was not reclassified by the new report",
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
        Path(prior.__file__).resolve() == BASE_VERIFIER, "v5 verifier module differs"
    )
    sealed(BASE_VERIFIER, BASE_VERIFIER_SHA256, "v5 verifier source")
    sealed(BASE_PROMOTER, BASE_PROMOTER_SHA256, "v5 promoter source")
    base_receipt = json_object(
        sealed(BASE_DIR / "promotion-receipt.json", BASE_RECEIPT_SHA256, "v5 receipt"),
        "v5 receipt",
    )
    require(
        prior.verify_bundle(BASE_DIR) == base_receipt, "v5 chain verification differs"
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
        and target_ids[70] == ERROR_ID,
        "full-80 inventory target IDs moved",
    )
    require(base_receipt.get("successors") is not None, "v5 receipt has no successors")
    base_lines: dict[str, list[bytes]] = {}
    for mode in MODES:
        ledger = BASE_DIR / base_name(mode)
        ledger_sha, summary_sha = BASE_HASHES[mode]
        lines, rows = jsonl(
            sealed(ledger, ledger_sha, f"{mode} v5 ledger"), f"{mode} v5 ledger"
        )
        summary = json_object(
            sealed(
                ledger.with_suffix(".summary.json"), summary_sha, f"{mode} v5 summary"
            ),
            f"{mode} v5 summary",
        )
        require(
            base_receipt["successors"].get(mode) == summary,
            f"{mode} v5 receipt summary differs",
        )
        require(
            summary.get("ledgerSha256") == ledger_sha
            and summary.get("records") == RECORDS,
            f"{mode} v5 summary differs",
        )
        statuses = Counter(rows[index].get("outcome") for index in TARGET_INDEXES)
        expected = (
            {"passed": 76, "failed": 3, "error": 1}
            if mode == "candidate"
            else {"passed": 77, "failed": 3}
        )
        require(statuses == expected, f"{mode} v5 full-80 status slice differs")
        if mode == "candidate":
            require(
                rows[PRIOR_ERROR_INDEX].get("outcome") == "error",
                "v5 original synthetic error differs",
            )
            require(
                rows[ERROR_INDEX].get("outcome") == "failed",
                "v5 future synthetic-error case differs",
            )
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
        and payload.get("pwrightPackageSha256") == PWRIGHT_PACKAGE_SHA256
        and payload.get("playwrightPackageSha256") == PLAYWRIGHT_PACKAGE_SHA256,
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
    wheel_raw = sealed(WHEEL, WHEEL_SHA256, "frozen ABI130-v2 wheel")
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
            require(
                b"NATIVE_ABI_VERSION = 130" in archive.read("pwright/_abi.py"),
                "wheel ABI is not 130",
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
    """Require the exact ABI130-v2 wheel/report identity and 80-case scope."""
    require(type(source_pins.get("schema")) is int, "v6 source-pin schema type differs")
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
        "kind": "playwright-python-navigation-full-file-candidate-v6",
        "upstreamCommit": COMMIT,
        "upstreamNavigationSha256": NAVIGATION_SHA256,
        "officialManifestSha256": MANIFEST_SHA256,
        "profiledInventorySha256": INVENTORY_SHA256,
        "chromiumExpandedCases": 80,
        "frozenWheel": str(WHEEL),
        "frozenWheelSha256": WHEEL_SHA256,
        "wheelName": WHEEL.name,
        "abi130Identity": str(IDENTITY),
        "abi130IdentitySha256": IDENTITY_SHA256,
        "installedNativeAbi": 130,
        "pwrightArtifactSha256": PWRIGHT_ARTIFACT_SHA256,
        "pwrightPackageSha256": PWRIGHT_PACKAGE_SHA256,
        "playwrightPackageSha256": package_sha,
        "harnessInputs": provenance["harnessInputs"],
        "toolingSources": provenance["toolingSources"],
        "pwrightRuntimeLibraries": provenance["pwrightRuntimeLibraries"],
    }
    require(source_pins == expected, "v6 source-pin identity differs")


def verify_bundle(output_dir: Path = OUTPUT_DIR) -> dict[str, Any]:
    """Verify one fixed v6 successor and all immutable v5/source-bound inputs."""
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
    identity = json_object(
        sealed(IDENTITY, IDENTITY_SHA256, "ABI130 identity"), "ABI130 identity"
    )
    try:
        prior.check_identity(identity)
    except prior.UrlMatchValidationError as error:
        raise FullNavigationValidationError(
            f"ABI130 identity failed: {error}"
        ) from error
    sealed(MANIFEST, MANIFEST_SHA256, "official manifest")
    sealed(NAVIGATION_SOURCE, NAVIGATION_SHA256, "upstream navigation source")
    source_pins = json_object(
        sealed(SOURCE_PINS, SOURCE_PINS_SHA256, "v6 source pins"), "v6 source pins"
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
        and receipt.get("abi130Identity") == str(IDENTITY)
        and receipt.get("abi130IdentitySha256") == IDENTITY_SHA256
        and receipt.get("baseReceipt") == str(BASE_DIR / "promotion-receipt.json")
        and receipt.get("baseReceiptSha256") == BASE_RECEIPT_SHA256
        and receipt.get("basePromoterSha256") == BASE_PROMOTER_SHA256
        and receipt.get("baseVerifierSha256") == BASE_VERIFIER_SHA256
        and receipt.get("targetZeroBasedIndexes") == list(TARGET_INDEXES)
        and receipt.get("targetTestIds") == target_ids,
        "promotion receipt source or target identity differs",
    )
    require(base_receipt.get("successors") is not None, "v5 receipt has no successors")
    require(receipt.get("promoterSha256") == PROMOTER_SHA256, "v6 promoter SHA differs")
    sealed(PROMOTER, PROMOTER_SHA256, "v6 promoter source")
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
            "observedOutcomes": {"passed": 77, "failed": 2, "error": 1},
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
            {"error": 4, "failed": 104, "not-run": 4764, "passed": 1617, "skipped": 37}
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
