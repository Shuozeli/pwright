"""Independently verify the three-case Python Chromium URL-match v5 successor.

The sealed v4 pair is the base. Only three candidate URL-matched navigation
rows may change from failed to passed using a source-bound ABI130 report.
The other 6,523 candidate rows (including the synthetic report error) and all
6,526 oracle rows must retain their exact v4 bytes. This script never imports
the v5 publisher. Every new evidence file and publisher source is hash-pinned.
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

import verify_playwright_python_navigation_history_api_strict_paired_v4 as prior
import verify_playwright_python_official_provenance as official

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / ".work/official-conformance"
BUILD = WORK / "builds/python-navigation-url-match-abi130-v2-oct20.srSIiv"
BASE_DIR = WORK / "outcomes/python-navigation-history-api-chromium-strict-paired-v4"
OUTPUT_DIR = WORK / "outcomes/python-navigation-url-match-chromium-strict-paired-v5"
INVENTORY = WORK / "playwright-python-v1.58.0-profiled.jsonl"
MANIFEST = ROOT / "packages/pwright-python/tests/official/manifest.json"
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-python")
NAVIGATION_SOURCE = UPSTREAM / "tests/async/test_navigation.py"
SOURCE_PINS = OUTPUT_DIR / "source-pins.json"
RESULT = BUILD / "results/navigation-url-match-candidate-abi130-v2.xml"
PROVENANCE = RESULT.with_suffix(".provenance.json")
IDENTITY = BUILD / "results/abi130-v2-identity.json"
CANDIDATE_VENV = BUILD / "candidate-abi130-v2"
CANDIDATE_PACKAGE = CANDIDATE_VENV / "lib/python3.12/site-packages/pwright"
CANDIDATE_ARTIFACT = CANDIDATE_PACKAGE / "_native.abi3.so"
PYTHON_EXECUTABLE = CANDIDATE_VENV / "bin/python"
WHEEL = BUILD / "wheel/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl"
BASE_VERIFIER = (
    ROOT / "scripts/verify_playwright_python_navigation_history_api_strict_paired_v4.py"
)
BASE_PROMOTER = (
    ROOT
    / "scripts/promote_playwright_python_navigation_history_api_strict_paired_v4.py"
)
OFFICIAL_VERIFIER = ROOT / "scripts/verify_playwright_python_official_provenance.py"
PROMOTER = (
    ROOT / "scripts/promote_playwright_python_navigation_url_match_strict_paired_v5.py"
)

MODES = ("candidate", "oracle")
RECORDS = 6_526
TARGET_INDEXES = (2881, 2914, 2917)
TARGET_IDS = (
    "tests/async/test_navigation.py::test_wait_for_nav_should_respect_timeout[chromium]",
    "tests/async/test_navigation.py::test_wait_for_nav_should_work_with_url_match[chromium]",
    "tests/async/test_navigation.py::test_wait_for_nav_should_work_with_url_match_for_same_document_navigations[chromium]",
)
SYNTHETIC_ERROR_INDEX = 2683
GENERATED = frozenset({"attempts", "durationSeconds", "error", "outcome", "resultFile"})
KIND = "playwright-python-navigation-url-match-chromium-strict-paired-promotion-v5"
SCOPE = "tests/async/test_navigation.py-chromium-url-match-3"
COMMIT = "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
INVENTORY_SHA256 = "3dfb11b6ad7eb4736166ee2c4f7fa32cda78063324ad8314ce12f3e2d49b9d41"
MANIFEST_SHA256 = "6d4f86fee517e300319cfa87014a54302980bdf8cf538fa21c1daff4219ea724"
NAVIGATION_SHA256 = "f04af8cbdee3dffb2c578d61d7685f02e5186ca565906323d71b6f92f96cd144"
BASE_VERIFIER_SHA256 = (
    "d67da041dd15cfab5dd7ad351f23d515b671b06a7b16b74fbf3ad9c088f105fd"
)
BASE_PROMOTER_SHA256 = (
    "895e9eb44ed33dcabf04d97415c653be058afa7448bdbb5e2ed522577d1485a4"
)
OFFICIAL_VERIFIER_SHA256 = (
    "cb451d9977ff574efb0778b842ff8662a95471fc2d1ca5dff450bef39c6e50d7"
)
BASE_RECEIPT_SHA256 = "541e82e5805207de17299295b9e9dfe3ca4c04c20771f36981b1db5e2ef0c4b6"
BASE_HASHES = {
    "candidate": (
        "1274adf9a571c73c425484e2a934f3101a9b9a52bebb0e61ffa641bc67dfba31",
        "28d1da5e92374497026a9670b58bdff0decea430e068424245ddce7456ed3768",
    ),
    "oracle": (
        "b209bf22f7777283798842abfd2a9774e4bb8f1659ae1b67e6015a3dc5a22479",
        "0bf6e764521ee3235ad0e19f64dcab364a618122d88492b826f605f76ef80b70",
    ),
}
WHEEL_SHA256 = "40889db525656b5410a994c81e79530f873531c28ac0e8c9052e8603a87efb78"
RESULT_SHA256 = "d0e3f4d643d9093c40f5bc6627f36cd35b00dc5f7cf5edf5dcee2572f9316dfe"
PROVENANCE_SHA256 = "16be690a4210ba3c1b4dd6cb669d5ed50c3a9500643eb1783b3fb7323137be22"
IDENTITY_SHA256 = "aeb78b8ff2a6acd554b210491f8b73e9e27cae47faa29ddd128106b38ae7f2f7"
PWRIGHT_ARTIFACT_SHA256 = (
    "ef567004a447ee148e988961b04868a8d1063f885c816d10845a10e5c6c36df8"
)
PWRIGHT_PACKAGE_SHA256 = (
    "c66d04b01ca060ccbaf31f2aed127b51aebe34834d8b8b0246391f5fb489446e"
)
PLAYWRIGHT_PACKAGE_SHA256 = (
    "e3a7333b66af39f60b8fc68ea8ff177604a914ac2380a99f17ad596451c886e9"
)
PROMOTER_SHA256 = "694bbe6f368ce549049b0d0e1fcab33dc060222b049e81923a37645ae76499fb"
SOURCE_PINS_SHA256 = "a614d23baf672974b98b3b29723420beb9452b7112bccffca1e8a414d7fc0004"


class UrlMatchValidationError(ValueError):
    """An immutable source, report, or successor differs from its contract."""


def fail(message: str) -> NoReturn:
    raise UrlMatchValidationError(message)


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
        raise UrlMatchValidationError(f"{label} is invalid JSON") from error
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
    return f"playwright-python-{mode}-navigation-history-api-chromium-strict-paired-v4.jsonl"


def output_name(mode: str) -> str:
    require(mode in MODES, "unknown lane")
    return (
        f"playwright-python-{mode}-navigation-url-match-chromium-strict-paired-v5.jsonl"
    )


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
    target_indexes: tuple[int, ...],
    target_ids: tuple[str, ...],
    durations: dict[str, float],
    result_file: str,
    mode: str,
) -> Counter[str]:
    """Rebuild exactly three candidate rows and retain all other raw bytes."""
    require(mode in MODES, "ledger mode differs")
    require(
        len(base_lines) == len(successor_lines) == len(inventory_rows),
        f"{mode} ledger length differs",
    )
    require(
        len(target_indexes) == len(target_ids) == 3
        and len(set(target_indexes)) == 3
        and len(set(target_ids)) == 3,
        "target count or identity differs",
    )
    require(
        all(0 <= index < len(inventory_rows) for index in target_indexes),
        "target index is out of bounds",
    )
    ids = [row.get("id") for row in inventory_rows]
    require(
        all(isinstance(test_id, str) for test_id in ids) and len(ids) == len(set(ids)),
        "inventory IDs differ",
    )
    require(
        tuple(ids[index] for index in target_indexes) == target_ids,
        "target IDs moved in inventory",
    )
    require(set(durations) == set(target_ids), "focused JUnit target set differs")
    require(
        all(
            type(value) is float and math.isfinite(value) and value >= 0
            for value in durations.values()
        ),
        "focused report duration differs",
    )
    target_by_index = dict(zip(target_indexes, target_ids))
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
        if mode == "oracle" or index not in target_by_index:
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
                "durationSeconds": durations[target_by_index[index]],
                "outcome": "passed",
                "resultFile": result_file,
            }
            require(after == expected, f"candidate promoted target differs at {index}")
            canonical = (
                json.dumps(expected, sort_keys=True, separators=(",", ":")) + "\n"
            ).encode()
            require(
                after_raw == canonical,
                f"candidate promoted target bytes differ at {index}",
            )
            require(
                after_raw != before_raw,
                f"candidate target row did not change at {index}",
            )
        status = after.get("outcome")
        require(isinstance(status, str), f"{mode} outcome malformed at {index}")
        counts[status] += 1
    return counts


def _junit_durations(raw: bytes, target_ids: tuple[str, ...]) -> dict[str, float]:
    try:
        tree = ET.fromstring(raw)
    except ET.ParseError as error:
        raise UrlMatchValidationError(
            "focused URL-match JUnit is invalid XML"
        ) from error
    cases = list(tree.iter("testcase"))
    require(
        len(cases) == 3, "focused URL-match JUnit does not have exactly three cases"
    )
    observed: dict[str, float] = {}
    for case in cases:
        require(
            case.get("classname") == "tests.async.test_navigation",
            "focused JUnit case is outside pinned source",
        )
        name = case.get("name")
        require(isinstance(name, str), "focused JUnit case name missing")
        test_id = f"tests/async/test_navigation.py::{name}"
        require(
            test_id in target_ids and test_id not in observed,
            "focused JUnit target ID differs or repeats",
        )
        require(
            not any(case.findall(tag) for tag in ("failure", "error", "skipped")),
            "focused JUnit case did not pass",
        )
        try:
            duration = float(case.get("time", ""))
        except ValueError as error:
            raise UrlMatchValidationError(
                "focused JUnit duration is invalid"
            ) from error
        require(
            math.isfinite(duration) and duration >= 0,
            "focused JUnit duration is invalid",
        )
        observed[test_id] = duration
    require(tuple(observed) == target_ids, "focused JUnit case order differs")
    return observed


def _verify_base() -> tuple[
    dict[str, Any], list[dict[str, Any]], dict[str, list[bytes]]
]:
    require(
        Path(prior.__file__).resolve() == BASE_VERIFIER, "v4 verifier module differs"
    )
    sealed(BASE_VERIFIER, BASE_VERIFIER_SHA256, "v4 verifier source")
    sealed(BASE_PROMOTER, BASE_PROMOTER_SHA256, "v4 promoter source")
    base_receipt = json_object(
        sealed(BASE_DIR / "promotion-receipt.json", BASE_RECEIPT_SHA256, "v4 receipt"),
        "v4 receipt",
    )
    require(
        prior.verify_bundle(BASE_DIR) == base_receipt, "v4 chain verification differs"
    )
    _, inventory = jsonl(
        sealed(INVENTORY, INVENTORY_SHA256, "profiled inventory"), "inventory"
    )
    require(
        tuple(inventory[index].get("id") for index in TARGET_INDEXES) == TARGET_IDS,
        "v4 target inventory IDs differ",
    )
    require(
        all(
            inventory[index].get("sourceClassification") == "public-api-source"
            for index in TARGET_INDEXES
        ),
        "v4 target source differs",
    )
    require(
        SYNTHETIC_ERROR_INDEX not in TARGET_INDEXES,
        "synthetic error entered target set",
    )
    require(base_receipt.get("successors") is not None, "v4 receipt has no successors")
    base_lines: dict[str, list[bytes]] = {}
    for mode in MODES:
        ledger = BASE_DIR / base_name(mode)
        ledger_sha, summary_sha = BASE_HASHES[mode]
        lines, rows = jsonl(
            sealed(ledger, ledger_sha, f"{mode} v4 ledger"), f"{mode} v4 ledger"
        )
        summary = json_object(
            sealed(
                ledger.with_suffix(".summary.json"), summary_sha, f"{mode} v4 summary"
            ),
            f"{mode} v4 summary",
        )
        require(
            base_receipt["successors"].get(mode) == summary,
            f"{mode} v4 receipt summary differs",
        )
        require(
            summary.get("ledgerSha256") == ledger_sha
            and summary.get("records") == RECORDS,
            f"{mode} v4 summary differs",
        )
        expected = "failed" if mode == "candidate" else "passed"
        require(
            all(rows[index].get("outcome") == expected for index in TARGET_INDEXES),
            f"{mode} v4 target statuses differ",
        )
        if mode == "candidate":
            require(
                rows[SYNTHETIC_ERROR_INDEX].get("outcome") == "error",
                "v4 synthetic report error was reclassified",
            )
        base_lines[mode] = lines
    return base_receipt, inventory, base_lines


def check_report_metadata(payload: dict[str, Any]) -> None:
    """Reject a report outside the exact frozen three-case Chromium run."""
    require(
        payload.get("schema") == 5
        and payload.get("mode") == "candidate"
        and payload.get("commit") == COMMIT
        and payload.get("manifest") == str(MANIFEST)
        and payload.get("manifestSha256") == MANIFEST_SHA256
        and payload.get("upstream") == str(UPSTREAM)
        and payload.get("inventory") is None
        and payload.get("inventorySha256") is None
        and payload.get("scope") == "selected"
        and payload.get("endpoint") == "http://127.0.0.1:9222"
        and payload.get("fixtureBind") == "100.78.97.63"
        and payload.get("fixtureHost") == "macbookpro-yuacx.tail8f3b66.ts.net"
        and payload.get("result") == str(RESULT)
        and payload.get("resultSha256") == RESULT_SHA256
        and payload.get("caseReports") is None
        and payload.get("caseReportsSha256") is None
        and payload.get("caseTimeout") == 45.0
        and payload.get("isolatedCaseIdsSha256") is None
        and payload.get("isolatedCaseCount") == 0
        and payload.get("selections") == list(TARGET_IDS)
        and payload.get("browsers") == ["chromium"]
        and payload.get("launchMode") == "cdp-adapter"
        and payload.get("isolateCases") is False
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
        and payload.get("pwrightPackageSha256") == PWRIGHT_PACKAGE_SHA256
        and payload.get("playwrightPackageSha256") == PLAYWRIGHT_PACKAGE_SHA256,
        "focused URL-match provenance identity or scope differs",
    )


def _verify_report() -> tuple[dict[str, Any], dict[str, float]]:
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
        raise UrlMatchValidationError(
            f"focused official provenance failed: {error}"
        ) from error
    check_report_metadata(payload)
    return payload, _junit_durations(result_raw, TARGET_IDS)


def _verify_wheel(provenance: dict[str, Any]) -> None:
    wheel_raw = sealed(WHEEL, WHEEL_SHA256, "frozen ABI130 wheel")
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
                b"NATIVE_ABI_VERSION = 130" in archive.read("pwright/_abi.py"),
                "wheel ABI is not 130",
            )
    except (OSError, KeyError, zipfile.BadZipFile) as error:
        raise UrlMatchValidationError(
            "frozen wheel has no valid native artifact"
        ) from error
    require(sha256(native) == PWRIGHT_ARTIFACT_SHA256, "wheel/native artifact differs")
    require(
        provenance.get("pwrightArtifactSha256") == sha256(native),
        "report/wheel native artifact differs",
    )


def check_identity(identity: dict[str, Any]) -> None:
    """Bind the ABI130 identity record to the frozen wheel and official run."""
    installed = identity.get("installedIdentity")
    run = identity.get("officialRun")
    proof = identity.get("defaultOffProof")
    require(
        type(identity.get("schema")) is int
        and identity["schema"] == 1
        and identity.get("kind")
        == "pwright-python-navigation-url-match-abi130-v2-source-bound-identity"
        and identity.get("wheelPath") == str(WHEEL)
        and identity.get("wheelSha256") == WHEEL_SHA256
        and isinstance(installed, dict)
        and installed.get("abi") == 130
        and installed.get("nativeModule") == str(CANDIDATE_ARTIFACT)
        and installed.get("packageVersion") == "0.1.0"
        and isinstance(installed.get("selectedFeatures"), list)
        and "navigation-url-match" in installed["selectedFeatures"]
        and identity.get("nativeArtifactSha256") == PWRIGHT_ARTIFACT_SHA256
        and identity.get("installedPackageSha256") == PWRIGHT_PACKAGE_SHA256
        and isinstance(run, dict)
        and run.get("resultPath") == str(RESULT)
        and run.get("resultSha256") == RESULT_SHA256
        and run.get("provenancePath") == str(PROVENANCE)
        and run.get("exitCode") == 0
        and run.get("selections") == list(TARGET_IDS)
        and run.get("upstreamCommit") == COMMIT
        and run.get("upstreamTag") == "v1.58.0"
        and run.get("manifestSha256") == MANIFEST_SHA256
        and (
            run.get("tests"),
            run.get("failures"),
            run.get("errors"),
            run.get("skipped"),
        )
        == (3, 0, 0, 0)
        and isinstance(proof, dict)
        and proof.get("addonDefaultFeatures") == []
        and proof.get("releaseCheckPassed") is True
        and proof.get("urlMatchAddedPackagesVsFrameNavigation") == [],
        "ABI130 source-bound identity differs",
    )


def check_source_pins(source_pins: dict[str, Any], provenance: dict[str, Any]) -> None:
    """Require the exact ABI130 wheel/report identity and three-case scope."""
    require(type(source_pins.get("schema")) is int, "v5 source-pin schema type differs")
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
        "kind": "playwright-python-navigation-url-match-candidate-v5",
        "upstreamCommit": COMMIT,
        "upstreamNavigationSha256": NAVIGATION_SHA256,
        "officialManifestSha256": MANIFEST_SHA256,
        "profiledInventorySha256": INVENTORY_SHA256,
        "targetTestIds": list(TARGET_IDS),
        "targetZeroBasedIndexes": list(TARGET_INDEXES),
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
    require(source_pins == expected, "v5 source-pin identity differs")


def verify_bundle(output_dir: Path = OUTPUT_DIR) -> dict[str, Any]:
    """Verify one fixed v5 successor and all immutable v4/source-bound inputs."""
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
    provenance, durations = _verify_report()
    _verify_wheel(provenance)
    identity = json_object(
        sealed(IDENTITY, IDENTITY_SHA256, "ABI130 identity"), "ABI130 identity"
    )
    check_identity(identity)
    sealed(MANIFEST, MANIFEST_SHA256, "official manifest")
    sealed(NAVIGATION_SOURCE, NAVIGATION_SHA256, "upstream navigation source")
    source_pins = json_object(
        sealed(SOURCE_PINS, SOURCE_PINS_SHA256, "v5 source pins"), "v5 source pins"
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
        and receipt.get("targetTestIds") == list(TARGET_IDS),
        "promotion receipt source or target identity differs",
    )
    require(base_receipt.get("successors") is not None, "v4 receipt has no successors")
    require(receipt.get("promoterSha256") == PROMOTER_SHA256, "v5 promoter SHA differs")
    sealed(PROMOTER, PROMOTER_SHA256, "v5 promoter source")
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
            "observedOutcomes": {"passed": 3},
            "isolateCases": False,
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
            TARGET_INDEXES,
            TARGET_IDS,
            durations,
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
            {"error": 4, "failed": 105, "not-run": 4764, "passed": 1616, "skipped": 37}
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
        UrlMatchValidationError,
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
