"""Independently verify the ABI131 full-80 Python navigation v7 successor.

The sealed v6 pair is the base. Exactly 80 candidate Chromium navigation rows
are reconstructed from one source-bound ABI131 report. Every other candidate
row and every oracle row retain their exact v6 bytes. This verifier does not
import the v7 publisher.
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

import verify_playwright_python_navigation_full_strict_paired_v6 as prior
import verify_playwright_python_official_provenance as official

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / ".work/official-conformance"
BUILD = WORK / "builds/python-frame-events-abi131-oct20.6SC15h"
BASE_DIR = WORK / "outcomes/python-navigation-full-chromium-strict-paired-v6"
OUTPUT_DIR = WORK / "outcomes/python-navigation-full-chromium-strict-paired-v7"
RESULT = BUILD / "results/navigation-candidate-full80-abi131.xml"
PROVENANCE = RESULT.with_suffix(".provenance.json")
FOCUSED_RESULT = BUILD / "results/frame-events-candidate-focused-abi131.xml"
FOCUSED_PROVENANCE = FOCUSED_RESULT.with_suffix(".provenance.json")
WHEEL = BUILD / "wheel/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl"
CANDIDATE_VENV = BUILD / "candidate-abi131"
PYTHON_EXECUTABLE = CANDIDATE_VENV / "bin/python"
CANDIDATE_PACKAGE = CANDIDATE_VENV / "lib/python3.12/site-packages/pwright"
CANDIDATE_ARTIFACT = CANDIDATE_PACKAGE / "_native.abi3.so"
INVENTORY = prior.INVENTORY
MANIFEST = prior.MANIFEST
UPSTREAM = prior.UPSTREAM
NAVIGATION_SOURCE = prior.NAVIGATION_SOURCE
SOURCE_PINS = OUTPUT_DIR / "source-pins.json"
BASE_VERIFIER = (
    ROOT / "scripts/verify_playwright_python_navigation_full_strict_paired_v6.py"
)
BASE_PROMOTER = (
    ROOT / "scripts/promote_playwright_python_navigation_full_strict_paired_v6.py"
)
OFFICIAL_VERIFIER = ROOT / "scripts/verify_playwright_python_official_provenance.py"
PROMOTER = (
    ROOT / "scripts/promote_playwright_python_navigation_full_strict_paired_v7.py"
)

MODES = ("candidate", "oracle")
RECORDS = 6_526
TARGET_INDEXES = tuple(range(2680, 2918, 3))
GENERATED = frozenset({"attempts", "durationSeconds", "error", "outcome", "resultFile"})
KIND = "playwright-python-navigation-full-chromium-strict-paired-promotion-v7"
SCOPE = "tests/async/test_navigation.py-chromium-full-file-80"
COMMIT = "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
INVENTORY_SHA256 = "3dfb11b6ad7eb4736166ee2c4f7fa32cda78063324ad8314ce12f3e2d49b9d41"
MANIFEST_SHA256 = "6d4f86fee517e300319cfa87014a54302980bdf8cf538fa21c1daff4219ea724"
NAVIGATION_SHA256 = "f04af8cbdee3dffb2c578d61d7685f02e5186ca565906323d71b6f92f96cd144"
BASE_VERIFIER_SHA256 = (
    "273914f9f461db9468fed8c8bf982a3ea76ebc7a7a1aec732ce263903a1ab2d8"
)
BASE_PROMOTER_SHA256 = (
    "1fd8223ee374c04dd52c7fc018ab5928b3c8c3416e3ef4b27519c234f05cea45"
)
OFFICIAL_VERIFIER_SHA256 = (
    "cb451d9977ff574efb0778b842ff8662a95471fc2d1ca5dff450bef39c6e50d7"
)
BASE_RECEIPT_SHA256 = "928da6d927abd7cf0667b63e88a89eb9fab260fe15ff0c50c04db358a295cb33"
BASE_HASHES = {
    "candidate": (
        "f2441051460ef9ecef5653f5bc45481754860e87b9a0e4d0273d89d0bfd10996",
        "8966505e4a4a15277bde620b6a866a0dc4896c660c971c69e9ad28ba69bc81ad",
    ),
    "oracle": (
        "b209bf22f7777283798842abfd2a9774e4bb8f1659ae1b67e6015a3dc5a22479",
        "05655d61a0c491e14eafe8cea3c5f4f0fe34707429c3ceee29208bc62e087091",
    ),
}
WHEEL_SHA256 = "950d4ac52f654e2c9584a85e76d8f83816f8ca2403a33393394baae078e23e00"
PWRIGHT_ARTIFACT_SHA256 = (
    "e536e6d604f0cd47faa16000819c644c52574c84af0080769c04674bc39de2fb"
)
PWRIGHT_PACKAGE_SHA256 = (
    "294e0a59fa399a848036d5e5cbab6025e2a95c2da3c174b943a5bbd56f62260a"
)
PLAYWRIGHT_PACKAGE_SHA256 = (
    "e3a7333b66af39f60b8fc68ea8ff177604a914ac2380a99f17ad596451c886e9"
)
ISOLATED_IDS_SHA256 = "4b9f37f3a3857ddf59e9b5766255f84dfe83d3d5af76285208e176088f656696"

RESULT_SHA256 = "af61500645795505ea3e96abade1607f7d5d16b6c67aa85a6149c8af68b7a0e3"
PROVENANCE_SHA256 = "1e2485f171757407b8aec08a331750c6cd1f35e215e4603655aeac71dc11de3a"
CASE_REPORTS_SHA256 = "41239bb5ba3f124b661bf772a41835bcd43bfb71feae2b31589a138a13439d54"
FOCUSED_RESULT_SHA256 = (
    "4c73343d1674ed27f82d8c11b2a2632dfc1f0973b67fa8e6517016d1915f7d68"
)
FOCUSED_PROVENANCE_SHA256 = (
    "b3e093adbb08eba31e00d2fb87cd30cb88721534a03195c982b13219931aa5ed"
)
FOCUSED_TARGET_ID = prior.ERROR_ID
SOURCE_PINS_SHA256 = "bbc10c067c28598e6a6dc2c2bf7c510df515bda3b5c42928b36897e4041aaef5"
PROMOTER_SHA256 = "f8db0cfa62eb36f0ccc4355be4c785392233214d255004ce49092372eb461b80"
EXPECTED_REPORT_COUNTS = {"passed": 76, "failed": 2, "error": 2}
EXPECTED_SUCCESSOR_COUNTS = {
    "candidate": {
        "error": 5,
        "failed": 104,
        "not-run": 4764,
        "passed": 1616,
        "skipped": 37,
    },
    "oracle": {"failed": 7, "not-run": 5096, "passed": 1386, "skipped": 37},
}


class FullNavigationV7ValidationError(ValueError):
    """An immutable source, report, or successor differs from its contract."""


def fail(message: str) -> NoReturn:
    raise FullNavigationV7ValidationError(message)


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
        raise FullNavigationV7ValidationError(f"{label} is invalid JSON") from error
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
    return f"playwright-python-{mode}-navigation-full-chromium-strict-paired-v6.jsonl"


def output_name(mode: str) -> str:
    require(mode in MODES, "unknown lane")
    return f"playwright-python-{mode}-navigation-full-chromium-strict-paired-v7.jsonl"


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
        ("fullFileRegression", True),
        ("focusedGreenDoesNotImplyFullGreen", True),
    ):
        require(receipt.get(field) is expected, f"receipt {field} differs")
    require(receipt.get("launchMode") == "cdp-adapter", "receipt launch mode differs")
    require(
        receipt.get("chromeVersionObserved") == "155.0.8059.85",
        "receipt Chrome version differs",
    )


def _base() -> tuple[dict[str, Any], list[dict[str, Any]], dict[str, list[bytes]]]:
    require(
        Path(prior.__file__).resolve() == BASE_VERIFIER, "v6 verifier module differs"
    )
    sealed(BASE_VERIFIER, BASE_VERIFIER_SHA256, "v6 verifier source")
    sealed(BASE_PROMOTER, BASE_PROMOTER_SHA256, "v6 promoter source")
    base_receipt = json_object(
        sealed(BASE_DIR / "promotion-receipt.json", BASE_RECEIPT_SHA256, "v6 receipt"),
        "v6 receipt",
    )
    require(
        prior.verify_bundle(BASE_DIR) == base_receipt, "v6 chain verification differs"
    )
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
        == 80
        and targets == base_receipt.get("targetTestIds")
        and list(TARGET_INDEXES) == base_receipt.get("targetZeroBasedIndexes"),
        "80 official Chromium navigation targets differ",
    )
    originals: dict[str, list[bytes]] = {}
    for mode in MODES:
        ledger = BASE_DIR / base_name(mode)
        ledger_sha, summary_sha = BASE_HASHES[mode]
        lines, rows = jsonl(
            sealed(ledger, ledger_sha, f"{mode} v6 ledger"), f"{mode} v6 ledger"
        )
        summary = json_object(
            sealed(
                ledger.with_suffix(".summary.json"), summary_sha, f"{mode} v6 summary"
            ),
            f"{mode} v6 summary",
        )
        require(
            base_receipt["successors"].get(mode) == summary,
            f"{mode} v6 summary differs",
        )
        require(
            all(
                row.get("id") == inventory[index].get("id")
                for index, row in enumerate(rows)
            ),
            f"{mode} row order differs",
        )
        originals[mode] = lines
    return base_receipt, inventory, originals


def check_report_metadata(payload: dict[str, Any]) -> None:
    expected = {
        "schema": 5,
        "mode": "candidate",
        "commit": COMMIT,
        "manifest": str(MANIFEST),
        "manifestSha256": MANIFEST_SHA256,
        "upstream": str(UPSTREAM),
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "scope": "selected",
        "endpoint": "http://127.0.0.1:9222",
        "fixtureBind": "100.78.97.63",
        "fixtureHost": "macbookpro-yuacx.tail8f3b66.ts.net",
        "result": str(RESULT),
        "resultSha256": RESULT_SHA256,
        "caseReports": str(Path(f"{RESULT}.cases")),
        "caseReportsSha256": CASE_REPORTS_SHA256,
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
        "manifestCases": False,
        "secureHttpFixture": False,
        "pythonExecutable": str(PYTHON_EXECUTABLE),
        "pwrightPackage": str(CANDIDATE_PACKAGE),
        "pwrightArtifact": str(CANDIDATE_ARTIFACT),
        "pwrightArtifactSha256": PWRIGHT_ARTIFACT_SHA256,
        "pwrightPackageSha256": PWRIGHT_PACKAGE_SHA256,
        "playwrightPackageSha256": PLAYWRIGHT_PACKAGE_SHA256,
    }
    require(
        all(payload.get(key) == value for key, value in expected.items()),
        "full-80 provenance identity or execution scope differs",
    )
    require(
        type(payload.get("exitCode")) is int and payload["exitCode"] == 1,
        "full-80 exit code differs",
    )


def _junit_cases(raw: bytes, target_ids: list[str]) -> dict[str, dict[str, object]]:
    try:
        tree = ET.fromstring(raw)
    except ET.ParseError as error:
        raise FullNavigationV7ValidationError("full-80 JUnit is invalid XML") from error
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
            raise FullNavigationV7ValidationError(
                "full-80 JUnit duration is invalid"
            ) from error
        require(
            math.isfinite(duration) and duration >= 0,
            "full-80 JUnit duration is invalid",
        )
        terminals = [
            node
            for tag in ("failure", "error", "skipped")
            for node in case.findall(tag)
        ]
        require(
            len(terminals) <= 1 and (not terminals or terminals[0].tag != "skipped"),
            "full-80 JUnit status is ambiguous",
        )
        outcome = (
            "passed"
            if not terminals
            else "failed"
            if terminals[0].tag == "failure"
            else "error"
        )
        row: dict[str, object] = {"outcome": outcome, "durationSeconds": duration}
        if terminals:
            message = (
                terminals[0].get("message") or terminals[0].text or ""
            ).splitlines()
            require(
                bool(message and message[0]), "full-80 failure/error message missing"
            )
            row["error"] = message[0]
        observed[test_id] = row
    require(
        list(observed) == target_ids, "full-80 JUnit IDs/order differ from inventory"
    )
    require(
        dict(Counter(row["outcome"] for row in observed.values()))
        == EXPECTED_REPORT_COUNTS,
        "full-80 JUnit outcome counts differ",
    )
    return observed


def _report(
    target_ids: list[str],
) -> tuple[dict[str, Any], dict[str, dict[str, object]]]:
    require(
        Path(official.__file__).resolve() == OFFICIAL_VERIFIER,
        "official verifier module differs",
    )
    sealed(OFFICIAL_VERIFIER, OFFICIAL_VERIFIER_SHA256, "official provenance verifier")
    payload = json_object(
        sealed(PROVENANCE, PROVENANCE_SHA256, "full-80 provenance"),
        "full-80 provenance",
    )
    raw = sealed(RESULT, RESULT_SHA256, "full-80 JUnit")
    try:
        official.verify_provenance(payload, provenance_path=PROVENANCE)
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        raise FullNavigationV7ValidationError(
            f"full-80 official provenance failed: {error}"
        ) from error
    check_report_metadata(payload)
    return payload, _junit_cases(raw, target_ids)


def _verify_focused() -> None:
    payload = json_object(
        sealed(FOCUSED_PROVENANCE, FOCUSED_PROVENANCE_SHA256, "focused provenance"),
        "focused provenance",
    )
    raw = sealed(FOCUSED_RESULT, FOCUSED_RESULT_SHA256, "focused JUnit")
    try:
        official.verify_provenance(payload, provenance_path=FOCUSED_PROVENANCE)
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        raise FullNavigationV7ValidationError(
            f"focused official provenance failed: {error}"
        ) from error
    require(
        payload.get("mode") == "candidate"
        and payload.get("commit") == COMMIT
        and payload.get("manifestSha256") == MANIFEST_SHA256
        and payload.get("inventorySha256") == INVENTORY_SHA256
        and payload.get("result") == str(FOCUSED_RESULT)
        and payload.get("resultSha256") == FOCUSED_RESULT_SHA256
        and payload.get("selections") == [FOCUSED_TARGET_ID]
        and payload.get("pwrightArtifactSha256") == PWRIGHT_ARTIFACT_SHA256
        and payload.get("pwrightPackageSha256") == PWRIGHT_PACKAGE_SHA256
        and payload.get("exitCode") == 0,
        "focused ABI131 source-bound identity differs",
    )
    cases = list(ET.fromstring(raw).iter("testcase"))
    require(
        len(cases) == 1
        and cases[0].get("classname") == "tests.async.test_navigation"
        and f"tests/async/test_navigation.py::{cases[0].get('name')}"
        == FOCUSED_TARGET_ID
        and not any(
            cases[0].find(tag) is not None for tag in ("failure", "error", "skipped")
        ),
        "focused ABI131 target did not pass exactly once",
    )


def _verify_wheel(provenance: dict[str, Any]) -> None:
    wheel_raw = sealed(WHEEL, WHEEL_SHA256, "frozen ABI131 wheel")
    require(
        CANDIDATE_PACKAGE.is_dir() and not CANDIDATE_PACKAGE.is_symlink(),
        "installed ABI131 package missing or linked",
    )
    try:
        with zipfile.ZipFile(io.BytesIO(wheel_raw)) as archive:
            files = [
                item
                for item in archive.infolist()
                if item.filename.startswith("pwright/") and not item.is_dir()
            ]
            require(
                bool(files) and len(files) == len({item.filename for item in files}),
                "wheel package members missing or duplicated",
            )
            digest = hashlib.sha256()
            for item in sorted(files, key=lambda value: value.filename):
                relative = Path(item.filename.removeprefix("pwright/"))
                require(
                    relative.parts
                    and not relative.is_absolute()
                    and all(part not in {"..", "."} for part in relative.parts),
                    "wheel package path is unsafe",
                )
                raw = archive.read(item)
                installed = CANDIDATE_PACKAGE / relative
                require(
                    installed.is_file()
                    and not installed.is_symlink()
                    and installed.read_bytes() == raw,
                    f"installed ABI131 package differs: {item.filename}",
                )
                name = str(relative).encode()
                digest.update(len(name).to_bytes(8, "big"))
                digest.update(name)
                digest.update(len(raw).to_bytes(8, "big"))
                digest.update(raw)
            native = archive.read("pwright/_native.abi3.so")
            require(
                b"NATIVE_ABI_VERSION = 131" in archive.read("pwright/_abi.py"),
                "wheel ABI is not 131",
            )
    except (OSError, KeyError, zipfile.BadZipFile) as error:
        raise FullNavigationV7ValidationError(
            "frozen ABI131 wheel is invalid"
        ) from error
    require(digest.hexdigest() == PWRIGHT_PACKAGE_SHA256, "wheel package SHA differs")
    require(sha256(native) == PWRIGHT_ARTIFACT_SHA256, "wheel native artifact differs")
    require(
        provenance.get("pwrightPackageSha256") == digest.hexdigest()
        and provenance.get("pwrightArtifactSha256") == sha256(native),
        "report/wheel package or artifact differs",
    )


def source_pins_expected(provenance: dict[str, Any]) -> dict[str, Any]:
    for field in ("harnessInputs", "toolingSources", "pwrightRuntimeLibraries"):
        require(
            isinstance(provenance.get(field), list) and bool(provenance[field]),
            f"full-80 report {field} is missing",
        )
    return {
        "schema": 1,
        "kind": "playwright-python-navigation-full-file-candidate-v7",
        "upstreamCommit": COMMIT,
        "upstreamNavigationSha256": NAVIGATION_SHA256,
        "officialManifestSha256": MANIFEST_SHA256,
        "profiledInventorySha256": INVENTORY_SHA256,
        "chromiumExpandedCases": 80,
        "installedNativeAbi": 131,
        "frozenWheel": str(WHEEL),
        "frozenWheelSha256": WHEEL_SHA256,
        "wheelName": WHEEL.name,
        "pwrightPackageSha256": PWRIGHT_PACKAGE_SHA256,
        "pwrightArtifactSha256": PWRIGHT_ARTIFACT_SHA256,
        "playwrightPackageSha256": PLAYWRIGHT_PACKAGE_SHA256,
        "focusedResult": str(FOCUSED_RESULT),
        "focusedResultSha256": FOCUSED_RESULT_SHA256,
        "focusedProvenance": str(FOCUSED_PROVENANCE),
        "focusedProvenanceSha256": FOCUSED_PROVENANCE_SHA256,
        "focusedTargetId": FOCUSED_TARGET_ID,
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
    require(mode in MODES, "unknown lane")
    require(
        len(base_lines) == len(successor_lines) == len(inventory) == RECORDS,
        "ledger row count differs",
    )
    require(
        set(cases) == {inventory[index].get("id") for index in TARGET_INDEXES},
        "report target ID set differs",
    )
    targets = set(TARGET_INDEXES)
    changed = 0
    counts: Counter[str] = Counter()
    for index, (before_raw, after_raw, source) in enumerate(
        zip(base_lines, successor_lines, inventory)
    ):
        before = json_object(before_raw, f"{mode} v6 row {index}")
        after = json_object(after_raw, f"{mode} v7 row {index}")
        require(
            before.get("id") == after.get("id") == source.get("id"),
            f"{mode} row ID moved at {index}",
        )
        require(
            {key: value for key, value in before.items() if key not in GENERATED}
            == source,
            f"{mode} v6 metadata differs at {index}",
        )
        require(
            {key: value for key, value in after.items() if key not in GENERATED}
            == source,
            f"{mode} v7 metadata differs at {index}",
        )
        if mode == "candidate" and index in targets:
            test_id = source["id"]
            require(test_id in cases, f"candidate target missing at {index}")
            require(
                before.get("attempts") == 1
                and before.get("outcome") in {"passed", "failed", "error"},
                f"candidate prior target status differs at {index}",
            )
            expected: dict[str, object] = {
                **source,
                "attempts": 1,
                "durationSeconds": cases[test_id]["durationSeconds"],
                "outcome": cases[test_id]["outcome"],
                "resultFile": str(RESULT),
            }
            if "error" in cases[test_id]:
                expected["error"] = cases[test_id]["error"]
            canonical = (
                json.dumps(expected, sort_keys=True, separators=(",", ":")) + "\n"
            ).encode()
            require(
                after == expected and after_raw == canonical,
                f"candidate target row differs at {index}",
            )
            require(
                after_raw != before_raw, f"candidate target was not replaced at {index}"
            )
        else:
            require(
                after_raw == before_raw, f"{mode} raw off-target row changed at {index}"
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
        dict(counts) == EXPECTED_SUCCESSOR_COUNTS.get(mode),
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
    _verify_focused()
    require(
        cases[FOCUSED_TARGET_ID]["outcome"] == "error",
        "focused GREEN did not reproduce as full-file RED",
    )
    require(
        base_receipt["reports"]["candidate"]["observedOutcomes"]
        == {"passed": 77, "failed": 2, "error": 1}
        and EXPECTED_REPORT_COUNTS == {"passed": 76, "failed": 2, "error": 2},
        "observed ABI131 full-file regression differs",
    )
    _verify_wheel(provenance)
    sealed(MANIFEST, MANIFEST_SHA256, "official manifest")
    sealed(NAVIGATION_SOURCE, NAVIGATION_SHA256, "upstream navigation source")
    return base_receipt, inventory, base_lines, provenance, cases


def verify_bundle(output_dir: Path = OUTPUT_DIR) -> dict[str, Any]:
    require(
        output_dir == OUTPUT_DIR
        and output_dir.is_dir()
        and not output_dir.is_symlink(),
        "v7 successor directory missing or moved",
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
        "v7 successor file set differs",
    )
    require(
        all(
            (output_dir / name).is_file() and not (output_dir / name).is_symlink()
            for name in expected_files
        ),
        "v7 successor file missing or linked",
    )
    receipt = json_object(
        (output_dir / "promotion-receipt.json").read_bytes(), "v7 receipt"
    )
    check_receipt_scope(receipt)
    base_receipt, inventory, base_lines, provenance, cases = evidence()
    target_ids = [inventory[index]["id"] for index in TARGET_INDEXES]
    pins = json_object(
        sealed(SOURCE_PINS, SOURCE_PINS_SHA256, "v7 source pins"), "v7 source pins"
    )
    require(pins == source_pins_expected(provenance), "v7 source-pin identity differs")
    sealed(PROMOTER, PROMOTER_SHA256, "v7 promoter source")
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
        "frozenWheel": str(WHEEL),
        "frozenWheelSha256": WHEEL_SHA256,
        "promoterSha256": PROMOTER_SHA256,
        "basePromoterSha256": BASE_PROMOTER_SHA256,
        "baseVerifierSha256": BASE_VERIFIER_SHA256,
        "baseReceipt": str(BASE_DIR / "promotion-receipt.json"),
        "baseReceiptSha256": BASE_RECEIPT_SHA256,
    }
    require(
        all(receipt.get(key) == value for key, value in expected_fields.items()),
        "v7 receipt source or target identity differs",
    )
    require(base_receipt.get("successors") is not None, "v6 receipt has no successors")
    require(
        isinstance(receipt.get("bases"), dict) and set(receipt["bases"]) == set(MODES),
        "v7 base lanes differ",
    )
    require(
        isinstance(receipt.get("reports"), dict)
        and set(receipt["reports"]) == {"candidate"},
        "v7 report lanes differ",
    )
    require(
        isinstance(receipt.get("successors"), dict)
        and set(receipt["successors"]) == set(MODES),
        "v7 successor lanes differ",
    )
    expected_report = {
        "provenance": str(PROVENANCE),
        "provenanceSha256": PROVENANCE_SHA256,
        "result": str(RESULT),
        "resultSha256": RESULT_SHA256,
        "observedOutcomes": EXPECTED_REPORT_COUNTS,
        "isolatedCaseIdsSha256": ISOLATED_IDS_SHA256,
        "caseReportsSha256": CASE_REPORTS_SHA256,
    }
    require(
        receipt["reports"]["candidate"] == expected_report,
        "v7 candidate receipt report differs",
    )
    require(
        receipt.get("focusedEvidence")
        == {
            "result": str(FOCUSED_RESULT),
            "resultSha256": FOCUSED_RESULT_SHA256,
            "provenance": str(FOCUSED_PROVENANCE),
            "provenanceSha256": FOCUSED_PROVENANCE_SHA256,
            "targetTestId": FOCUSED_TARGET_ID,
            "observedOutcome": "passed",
        },
        "v7 focused/full-file contrast receipt differs",
    )
    for mode in MODES:
        base_ledger = BASE_DIR / base_name(mode)
        base_sha, base_summary_sha = BASE_HASHES[mode]
        expected_base = {
            "ledger": str(base_ledger),
            "ledgerSha256": base_sha,
            "summary": str(base_ledger.with_suffix(".summary.json")),
            "summarySha256": base_summary_sha,
        }
        require(
            receipt["bases"][mode] == expected_base, f"{mode} v7 receipt base differs"
        )
        output_ledger = output_dir / output_name(mode)
        successor_raw = output_ledger.read_bytes()
        successor_lines, _ = jsonl(successor_raw, f"{mode} v7 ledger")
        counts = check_reconciliation(
            base_lines[mode], successor_lines, inventory, cases, mode
        )
        summary = json_object(
            output_ledger.with_suffix(".summary.json").read_bytes(),
            f"{mode} v7 summary",
        )
        expected_summary = {
            "schema": 1,
            "inventory": str(INVENTORY),
            "records": RECORDS,
            "outcomes": dict(sorted(counts.items())),
            "ledgerSha256": sha256(successor_raw),
            "ledger": str(output_ledger),
        }
        require(summary == expected_summary, f"{mode} v7 summary differs")
        require(
            receipt["successors"][mode] == summary, f"{mode} v7 receipt summary differs"
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
        FullNavigationV7ValidationError,
        prior.FullNavigationValidationError,
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
