"""Publish the source-bound ABI130 full-file Python navigation successor.

All 80 Chromium candidate rows come from one official full-file report. The
other 6,446 candidate rows and every oracle row retain their exact v5 bytes.
The report's synthetic error remains an error at its new case ID.
"""

from __future__ import annotations

import argparse
import json
import math
import os
import shutil
import subprocess
import tempfile
import xml.etree.ElementTree as ET
import zipfile
from collections import Counter
from pathlib import Path
from typing import Any

import promote_playwright_python_navigation_url_match_strict_paired_v5 as prior
from verify_playwright_python_official_provenance import (
    ProvenanceValidationError,
    verify_provenance,
)

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / ".work/official-conformance"
OUTCOMES = WORK / "outcomes"
BASE_DIR = OUTCOMES / "python-navigation-url-match-chromium-strict-paired-v5"
OUTPUT_DIR = OUTCOMES / "python-navigation-full-chromium-strict-paired-v6"
BUILD_DIR = WORK / "builds/python-navigation-url-match-abi130-v2-oct20.srSIiv"
REPORT = BUILD_DIR / "results/navigation-candidate-full80-abi130-v2.xml"
REPORT_SHA256 = "58c4769fbe08476f961ed5b17ef1e24734fe36847d716ffa5be02134de974164"
PROVENANCE_SHA256 = "72e7ea0f8b21427cad4b38bc834a245dfc97501e79cdd9564e80677cffb0be31"
CASE_REPORTS_SHA256 = "3bc05e94ea64fbb3d455d9bab8dec091e08b504210c2e77ea2bf7e9b268f354e"
ISOLATED_CASE_IDS_SHA256 = (
    "4b9f37f3a3857ddf59e9b5766255f84dfe83d3d5af76285208e176088f656696"
)
BASE_PROMOTER_SHA256 = (
    "694bbe6f368ce549049b0d0e1fcab33dc060222b049e81923a37645ae76499fb"
)
BASE_VERIFIER_SHA256 = (
    "356e8258c25aee4758cd5c34ffcf5c9d38ef09735ccbb0c508249aa325c251ad"
)
BASE_RECEIPT_SHA256 = "3c2d07e5d788b5ede07145e69ec78a6848045b3145a5772e61db55cf667145e6"
BASES = {
    "candidate": (
        "b985acb23344b379c1e3615dd040f0da821916f52d782c2d4f0b9ac477f2e661",
        "5bb2acd38c4b7dbcc7c5cb5a66314cc68d7d3a6aea4fd0ba1589cf5925b24f79",
    ),
    "oracle": (
        "b209bf22f7777283798842abfd2a9774e4bb8f1659ae1b67e6015a3dc5a22479",
        "9aab8096d2d6902e0bb2c7c86cafd44f9b8d2d703e480b5499fb9165ec2e1c81",
    ),
}
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-python")
UPSTREAM_COMMIT = "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
UPSTREAM_NAVIGATION_SHA256 = (
    "f04af8cbdee3dffb2c578d61d7685f02e5186ca565906323d71b6f92f96cd144"
)
MANIFEST = ROOT / "packages/pwright-python/tests/official/manifest.json"
MANIFEST_SHA256 = "6d4f86fee517e300319cfa87014a54302980bdf8cf538fa21c1daff4219ea724"
INVENTORY = WORK / "playwright-python-v1.58.0-profiled.jsonl"
INVENTORY_SHA256 = "3dfb11b6ad7eb4736166ee2c4f7fa32cda78063324ad8314ce12f3e2d49b9d41"
WHEEL = BUILD_DIR / "wheel/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl"
WHEEL_SHA256 = "40889db525656b5410a994c81e79530f873531c28ac0e8c9052e8603a87efb78"
IDENTITY = BUILD_DIR / "results/abi130-v2-identity.json"
IDENTITY_SHA256 = "aeb78b8ff2a6acd554b210491f8b73e9e27cae47faa29ddd128106b38ae7f2f7"
PACKAGE_SHA256 = "c66d04b01ca060ccbaf31f2aed127b51aebe34834d8b8b0246391f5fb489446e"
ARTIFACT_SHA256 = "ef567004a447ee148e988961b04868a8d1063f885c816d10845a10e5c6c36df8"
RECORDS = 6_526
TARGET_INDEXES = tuple(2680 + 3 * ordinal for ordinal in range(80))
ERROR_INDEX = 2890
ERROR_ID = "tests/async/test_navigation.py::test_wait_for_nav_should_work_when_subframe_issues_window_stop[chromium]"
ERROR_RAW_SHA256 = "b02a6a718c82c6806367fcc6ac8cf98717b8a5c5388b863b381e8758413b66b3"
GENERATED = frozenset({"attempts", "durationSeconds", "error", "outcome", "resultFile"})
RECEIPT_KIND = "playwright-python-navigation-full-chromium-strict-paired-promotion-v6"

PromotionError = prior.PromotionError
require = prior.require
sealed = prior.sealed
sha256 = prior.sha256
json_object = prior.json_object
jsonl = prior.jsonl


def base_name(mode: str) -> str:
    return (
        f"playwright-python-{mode}-navigation-url-match-chromium-strict-paired-v5.jsonl"
    )


def output_name(mode: str) -> str:
    return f"playwright-python-{mode}-navigation-full-chromium-strict-paired-v6.jsonl"


def source_inputs() -> None:
    require(
        prior.WHEEL == WHEEL
        and prior.WHEEL_SHA256 == WHEEL_SHA256
        and prior.IDENTITY == IDENTITY
        and prior.IDENTITY_SHA256 == IDENTITY_SHA256
        and prior.PACKAGE_SHA256 == PACKAGE_SHA256
        and prior.ARTIFACT_SHA256 == ARTIFACT_SHA256,
        "ABI130 base package pins differ",
    )
    prior.source_inputs()
    prior.identity()
    sealed(MANIFEST, MANIFEST_SHA256, "official manifest")
    sealed(INVENTORY, INVENTORY_SHA256, "profiled inventory")
    sealed(
        UPSTREAM / "tests/async/test_navigation.py",
        UPSTREAM_NAVIGATION_SHA256,
        "upstream navigation source",
    )


def base() -> tuple[dict[str, tuple[list[bytes], list[dict[str, Any]]]], list[str]]:
    base_promoter = (
        ROOT
        / "scripts/promote_playwright_python_navigation_url_match_strict_paired_v5.py"
    )
    base_verifier = (
        ROOT
        / "scripts/verify_playwright_python_navigation_url_match_strict_paired_v5.py"
    )
    require(
        Path(prior.__file__).resolve() == base_promoter, "base promoter module differs"
    )
    sealed(base_promoter, BASE_PROMOTER_SHA256, "base promoter")
    sealed(base_verifier, BASE_VERIFIER_SHA256, "base verifier")
    receipt = json_object(
        sealed(
            BASE_DIR / "promotion-receipt.json", BASE_RECEIPT_SHA256, "base receipt"
        ),
        "base receipt",
    )
    require(
        prior.verify_bundle(BASE_DIR) == receipt, "base receipt verification differs"
    )
    require(
        receipt.get("kind") == prior.RECEIPT_KIND
        and receipt.get("recordsPerLane") == RECORDS
        and receipt.get("inventorySha256") == INVENTORY_SHA256
        and receipt.get("manifestSha256") == MANIFEST_SHA256
        and receipt.get("upstreamCommit") == UPSTREAM_COMMIT
        and receipt.get("upstreamNavigationSha256") == UPSTREAM_NAVIGATION_SHA256
        and receipt.get("publicApiParityEvidence") is False
        and receipt.get("completeFileExecution") is False,
        "base receipt identity differs",
    )
    _, inventory = jsonl(sealed(INVENTORY, INVENTORY_SHA256, "inventory"), "inventory")
    require(len(inventory) == RECORDS, "inventory row count differs")
    targets = [inventory[index].get("id") for index in TARGET_INDEXES]
    require(
        len(set(targets)) == 80
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
        and inventory[ERROR_INDEX].get("id") == ERROR_ID,
        "80 full-file target identities differ",
    )
    originals: dict[str, tuple[list[bytes], list[dict[str, Any]]]] = {}
    for mode in ("candidate", "oracle"):
        ledger_path = BASE_DIR / base_name(mode)
        ledger_sha, summary_sha = BASES[mode]
        lines, rows = jsonl(
            sealed(ledger_path, ledger_sha, f"{mode} base ledger"),
            f"{mode} base ledger",
        )
        summary = json_object(
            sealed(
                ledger_path.with_suffix(".summary.json"),
                summary_sha,
                f"{mode} base summary",
            ),
            f"{mode} base summary",
        )
        require(
            receipt.get("successors", {}).get(mode) == summary,
            f"{mode} base summary differs",
        )
        require(len(lines) == len(rows) == RECORDS, f"{mode} base record count differs")
        for index, row in enumerate(rows):
            require(
                row.get("id") == inventory[index].get("id"),
                f"{mode} row order differs at {index}",
            )
            require(
                {key: value for key, value in row.items() if key not in GENERATED}
                == inventory[index],
                f"{mode} source metadata differs at {index}",
            )
        originals[mode] = lines, rows
    return originals, targets


def _outcome(case: ET.Element) -> tuple[str, str | None]:
    terminals = [
        node
        for tag in ("failure", "error", "skipped")
        if (node := case.find(tag)) is not None
    ]
    require(len(terminals) <= 1, "JUnit case has conflicting terminal statuses")
    if not terminals:
        return "passed", None
    node = terminals[0]
    require(node.tag != "skipped", "full-file evidence contains a skipped case")
    message = (node.get("message") or node.text or "").splitlines()
    require(
        bool(message) and bool(message[0]), "failed or error case lacks a diagnostic"
    )
    return ("failed" if node.tag == "failure" else "error"), message[0]


def report(targets: list[str]) -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    sidecar = REPORT.with_suffix(".provenance.json")
    payload = json_object(
        sealed(sidecar, PROVENANCE_SHA256, "candidate provenance"),
        "candidate provenance",
    )
    require(
        verify_provenance(payload, provenance_path=sidecar) == payload,
        "candidate provenance did not verify",
    )
    require(
        payload.get("schema") == 5
        and payload.get("mode") == "candidate"
        and payload.get("commit") == UPSTREAM_COMMIT
        and payload.get("launchMode") == "cdp-adapter"
        and payload.get("endpoint") == "http://127.0.0.1:9222"
        and payload.get("scope") == "selected"
        and payload.get("browsers") == ["chromium"]
        and payload.get("selections") == ["tests/async/test_navigation.py"]
        and payload.get("manifest") == str(MANIFEST)
        and payload.get("manifestSha256") == MANIFEST_SHA256
        and payload.get("inventory") == str(INVENTORY)
        and payload.get("inventorySha256") == INVENTORY_SHA256
        and payload.get("isolateCases") is True
        and payload.get("isolatedCaseCount") == 80
        and payload.get("isolatedCaseIdsSha256") == ISOLATED_CASE_IDS_SHA256
        and payload.get("caseReports") == str(Path(f"{REPORT}.cases"))
        and payload.get("caseReportsSha256") == CASE_REPORTS_SHA256
        and payload.get("result") == str(REPORT)
        and payload.get("resultSha256") == REPORT_SHA256
        and payload.get("pwrightPackageSha256") == PACKAGE_SHA256
        and payload.get("pwrightArtifactSha256") == ARTIFACT_SHA256
        and payload.get("pwrightPackage") == str(prior.CANDIDATE_PACKAGE)
        and payload.get("fixtureBind") == "100.78.97.63"
        and payload.get("fixtureHost") == "macbookpro-yuacx.tail8f3b66.ts.net"
        and payload.get("exitCode") == 1
        and payload.get("bootstrapOnly") is False
        and payload.get("collectOnly") is False
        and payload.get("manifestCases") is False,
        "candidate full-file run identity differs",
    )
    raw = sealed(REPORT, REPORT_SHA256, "candidate full-file JUnit")
    root = ET.fromstring(raw)
    cases = list(root.iter("testcase"))
    require(len(cases) == 80, "candidate JUnit does not contain 80 cases")
    mapped: dict[str, dict[str, Any]] = {}
    counts: Counter[str] = Counter()
    for case in cases:
        test_id = f"tests/async/test_navigation.py::{case.get('name')}"
        require(
            case.get("classname") == "tests.async.test_navigation"
            and test_id in targets
            and test_id not in mapped,
            "candidate JUnit IDs differ",
        )
        try:
            duration = float(case.get("time", ""))
        except ValueError as error:
            raise PromotionError(f"candidate duration is invalid: {test_id}") from error
        require(
            math.isfinite(duration) and duration >= 0,
            f"candidate duration is invalid: {test_id}",
        )
        outcome, error = _outcome(case)
        counts[outcome] += 1
        mapped[test_id] = {
            "outcome": outcome,
            "durationSeconds": duration,
            "error": error,
        }
    require(set(mapped) == set(targets), "candidate JUnit target set differs")
    require(
        dict(counts) == {"passed": 77, "failed": 2, "error": 1},
        "candidate full-file outcome counts differ",
    )
    require(
        mapped[ERROR_ID]["outcome"] == "error"
        and mapped[
            "tests/async/test_navigation.py::test_frame_goto_should_continue_after_client_redirect[chromium]"
        ]["outcome"]
        == "passed"
        and ERROR_RAW_SHA256 in (mapped[ERROR_ID]["error"] or ""),
        "synthetic error evidence was reclassified or substituted",
    )
    require(
        REPORT.read_bytes() == raw
        and sidecar.read_bytes()
        == sealed(sidecar, PROVENANCE_SHA256, "candidate provenance"),
        "candidate evidence changed during reading",
    )
    return payload, mapped


def source_pins(payload: dict[str, Any]) -> bytes:
    pins = {
        "schema": 1,
        "kind": "playwright-python-navigation-full-file-candidate-v6",
        "upstreamCommit": UPSTREAM_COMMIT,
        "upstreamNavigationSha256": UPSTREAM_NAVIGATION_SHA256,
        "officialManifestSha256": MANIFEST_SHA256,
        "profiledInventorySha256": INVENTORY_SHA256,
        "chromiumExpandedCases": 80,
        "installedNativeAbi": 130,
        "frozenWheel": str(WHEEL),
        "frozenWheelSha256": WHEEL_SHA256,
        "wheelName": WHEEL.name,
        "abi130Identity": str(IDENTITY),
        "abi130IdentitySha256": IDENTITY_SHA256,
        "pwrightPackageSha256": payload["pwrightPackageSha256"],
        "pwrightArtifactSha256": payload["pwrightArtifactSha256"],
        "playwrightPackageSha256": payload["playwrightPackageSha256"],
        "harnessInputs": payload["harnessInputs"],
        "toolingSources": payload["toolingSources"],
        "pwrightRuntimeLibraries": payload["pwrightRuntimeLibraries"],
    }
    return (json.dumps(pins, indent=2, sort_keys=True) + "\n").encode()


def successor(
    mode: str,
    original: tuple[list[bytes], list[dict[str, Any]]],
    targets: list[str],
    cases: dict[str, dict[str, Any]],
    output: Path,
) -> tuple[bytes, bytes, dict[str, Any]]:
    lines, rows = original
    updated = list(lines)
    if mode == "candidate":
        for index, test_id in zip(TARGET_INDEXES, targets):
            require(rows[index]["id"] == test_id, "target source row moved")
            result = cases[test_id]
            replacement = {
                key: value for key, value in rows[index].items() if key not in GENERATED
            }
            replacement.update(
                attempts=1,
                durationSeconds=result["durationSeconds"],
                outcome=result["outcome"],
                resultFile=str(REPORT),
            )
            if result["error"] is not None:
                replacement["error"] = result["error"]
            updated[index] = (
                json.dumps(replacement, sort_keys=True, separators=(",", ":")) + "\n"
            ).encode()
        require(
            all(
                updated[index] == line
                for index, line in enumerate(lines)
                if index not in TARGET_INDEXES
            ),
            "candidate off-target row bytes changed",
        )
        require(
            sum(before != after for before, after in zip(lines, updated)) == 80,
            "candidate changed-row count differs",
        )
        require(
            json_object(updated[ERROR_INDEX], "new synthetic error").get("outcome")
            == "error",
            "report synthetic error was reclassified",
        )
    else:
        require(mode == "oracle", "unknown ledger mode")
        require(updated == lines, "oracle raw rows changed")
    ledger = b"".join(updated)
    outcomes = Counter(
        json_object(line, f"{mode} successor row")["outcome"] for line in updated
    )
    summary = {
        "schema": 1,
        "inventory": str(INVENTORY),
        "records": RECORDS,
        "outcomes": dict(sorted(outcomes.items())),
        "ledgerSha256": sha256(ledger),
        "ledger": str(output),
    }
    expected = (
        {"error": 4, "failed": 104, "not-run": 4764, "passed": 1617, "skipped": 37}
        if mode == "candidate"
        else {"failed": 7, "not-run": 5096, "passed": 1386, "skipped": 37}
    )
    require(summary["outcomes"] == expected, f"{mode} successor outcomes differ")
    return ledger, (json.dumps(summary, indent=2) + "\n").encode(), summary


def prepare(output_dir: Path) -> tuple[dict[str, bytes], dict[str, Any]]:
    require(
        output_dir == OUTPUT_DIR and output_dir.is_absolute(), "successor path differs"
    )
    source_inputs()
    originals, targets = base()
    payload, cases = report(targets)
    content: dict[str, bytes] = {}
    summaries: dict[str, dict[str, Any]] = {}
    for mode in ("candidate", "oracle"):
        name = output_name(mode)
        ledger, summary_raw, summary = successor(
            mode, originals[mode], targets, cases, output_dir / name
        )
        content[name] = ledger
        content[Path(name).with_suffix(".summary.json").name] = summary_raw
        summaries[mode] = summary
    pins = source_pins(payload)
    content["source-pins.json"] = pins
    receipt = {
        "schema": 2,
        "kind": RECEIPT_KIND,
        "scope": "tests/async/test_navigation.py-chromium-full-file-80",
        "publicApiParityEvidence": False,
        "fullCorpusExecution": False,
        "completeFileExecution": True,
        "fixtureAdapted": True,
        "launchMode": "cdp-adapter",
        "chromeVersionObserved": "155.0.8059.85",
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "recordsPerLane": RECORDS,
        "targetZeroBasedIndexes": list(TARGET_INDEXES),
        "targetTestIds": targets,
        "manifest": str(MANIFEST),
        "manifestSha256": MANIFEST_SHA256,
        "upstream": str(UPSTREAM),
        "upstreamCommit": UPSTREAM_COMMIT,
        "upstreamNavigationSha256": UPSTREAM_NAVIGATION_SHA256,
        "sourcePins": str(output_dir / "source-pins.json"),
        "sourcePinsSha256": sha256(pins),
        "frozenWheel": str(WHEEL),
        "frozenWheelSha256": WHEEL_SHA256,
        "abi130Identity": str(IDENTITY),
        "abi130IdentitySha256": IDENTITY_SHA256,
        "promoterSha256": sha256(Path(__file__).read_bytes()),
        "basePromoterSha256": BASE_PROMOTER_SHA256,
        "baseVerifierSha256": BASE_VERIFIER_SHA256,
        "baseReceipt": str(BASE_DIR / "promotion-receipt.json"),
        "baseReceiptSha256": BASE_RECEIPT_SHA256,
        "bases": {
            mode: {
                "ledger": str(BASE_DIR / base_name(mode)),
                "ledgerSha256": BASES[mode][0],
                "summary": str(
                    (BASE_DIR / base_name(mode)).with_suffix(".summary.json")
                ),
                "summarySha256": BASES[mode][1],
            }
            for mode in ("candidate", "oracle")
        },
        "reports": {
            "candidate": {
                "provenance": str(REPORT.with_suffix(".provenance.json")),
                "provenanceSha256": PROVENANCE_SHA256,
                "result": str(REPORT),
                "resultSha256": REPORT_SHA256,
                "observedOutcomes": {"passed": 77, "failed": 2, "error": 1},
                "isolatedCaseIdsSha256": ISOLATED_CASE_IDS_SHA256,
                "caseReportsSha256": CASE_REPORTS_SHA256,
            }
        },
        "successors": summaries,
    }
    content["promotion-receipt.json"] = (
        json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    ).encode()
    return content, receipt


def _check_output(output_dir: Path) -> None:
    require(output_dir == OUTPUT_DIR, "successor path differs")
    prior.prior.prior.prior.prior.prior.prior.prior._check_output(output_dir)


def publish(output_dir: Path) -> dict[str, Any]:
    _check_output(output_dir)
    content, receipt = prepare(output_dir)
    staging = Path(
        tempfile.mkdtemp(prefix=f".{output_dir.name}.staging-", dir=output_dir.parent)
    )
    try:
        for name, raw in content.items():
            with (staging / name).open("xb") as stream:
                stream.write(raw)
                stream.flush()
                os.fsync(stream.fileno())
        prior.prior.prior.prior.prior.prior.prior.prior._fsync_directory(staging)
        _check_output(output_dir)
        require(
            prepare(output_dir) == (content, receipt),
            "strict evidence changed during staging",
        )
        prior.prior.prior.prior.prior.prior.prior.prior._publish_exclusive(
            staging, output_dir
        )
    finally:
        if staging.exists():
            shutil.rmtree(staging)
    return receipt


def verify_bundle(output_dir: Path) -> dict[str, Any]:
    require(
        output_dir == OUTPUT_DIR
        and output_dir.is_dir()
        and not output_dir.is_symlink(),
        "successor bundle is missing or moved",
    )
    content, receipt = prepare(output_dir)
    require(
        {path.name for path in output_dir.iterdir()} == set(content),
        "successor file set differs",
    )
    for name, expected in content.items():
        path = output_dir / name
        require(
            path.is_file() and not path.is_symlink(),
            f"successor file missing or linked: {name}",
        )
        require(
            path.read_bytes() == expected, f"successor file content differs: {name}"
        )
    return receipt


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify-existing", action="store_true")
    args = parser.parse_args()
    try:
        receipt = (
            verify_bundle(OUTPUT_DIR) if args.verify_existing else publish(OUTPUT_DIR)
        )
    except (
        OSError,
        PromotionError,
        ProvenanceValidationError,
        subprocess.CalledProcessError,
        subprocess.TimeoutExpired,
        ET.ParseError,
        zipfile.BadZipFile,
        KeyError,
        TypeError,
        AttributeError,
        IndexError,
    ) as error:
        parser.error(str(error))
    print(
        json.dumps(
            {
                "verified" if args.verify_existing else "published": str(OUTPUT_DIR),
                "successors": receipt["successors"],
            },
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
