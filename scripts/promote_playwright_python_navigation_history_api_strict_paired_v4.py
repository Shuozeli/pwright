"""Publish the source-bound ABI129 history-API navigation successor.

One candidate row is projected from a focused official run. The other 6,525
candidate rows, including the v3 synthetic error, and all 6,526 oracle rows
retain their exact v3 bytes. This does not claim a fresh full-file execution.
"""

from __future__ import annotations

import argparse
import hashlib
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

import promote_playwright_python_navigation_full_strict_paired_v3 as prior
from verify_playwright_python_official_provenance import (
    ProvenanceValidationError,
    verify_provenance,
)

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / ".work/official-conformance"
OUTCOMES = WORK / "outcomes"
BASE_DIR = OUTCOMES / "python-navigation-full-chromium-strict-paired-v3"
OUTPUT_DIR = OUTCOMES / "python-navigation-history-api-chromium-strict-paired-v4"
BUILD_DIR = WORK / "builds/python-history-forward-abi129-v6.aWkEM6"
REPORT = BUILD_DIR / "results/history-api-candidate-abi129-v6.xml"
REPORT_SHA256 = "4895ff3b36514245834b9a882219fa73143ecfb01a9831550437ff1b3302eb80"
PROVENANCE_SHA256 = "3ec57bf19ba02eb326e3e823dcb08963df1460bb2d611f6084a7026f2b5c654f"
CASE_REPORTS_SHA256 = "efaeea1a1bb7bbffa2b04aebca50c71cc66b6243e9420539b9fb608f51430bdb"
ISOLATED_CASE_IDS_SHA256 = (
    "d95313963641267c0d28dde8ce144bae0cc3f41ff74d9429b9f711c2d2a1d8da"
)
WHEEL = (
    BUILD_DIR / "wheel-repaired/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl"
)
WHEEL_SHA256 = "07cc6f949c765746f76f6bb4b1567e3a1bee74a9f57e75e0f50d666c5745b37c"
PACKAGE_SHA256 = "8f58621d0e98bcc3d88a102bd5a7846eabb920dc359ef02ab5dd58327d8abbf7"
ARTIFACT_SHA256 = "2ec3c59d61d5f6affd604f2ad0d92d9902d77d7530738481ddae9115a885e9ff"
BASE_PROMOTER_SHA256 = (
    "c36107a1f7ced0645a33a1c52c8817d1bbd29e0d75693c6ce2fbdf0d641f04cd"
)
BASE_VERIFIER_SHA256 = (
    "42d51cfad1a4f14c803d171e6f4a0afeb1813787248be929170dd48ee629caed"
)
BASE_RECEIPT_SHA256 = "2255d24bdeee2fc9b9a9d4c12c742d1e65c55793a8562c8b65cf978eb6346f1b"
BASES = {
    "candidate": (
        "2be32b693fc328194efcd0b69b412bfbff8e10d1d54e21aafd125be4e0177bf8",
        "e4e102c0cc462ad7ad8b367ba3c266fdf9a5c132fb155dbdd901d5473e731d28",
    ),
    "oracle": (
        "b209bf22f7777283798842abfd2a9774e4bb8f1659ae1b67e6015a3dc5a22479",
        "9a860500dc4e26b77a768175826d440bbfa77b2e3bb7bc5c85c57771604d442b",
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
TARGET_INDEX = 2704
TARGET_ID = "tests/async/test_navigation.py::test_go_back_should_work_with_history_api[chromium]"
ERROR_INDEX = 2683
ERROR_ID = "tests/async/test_navigation.py::test_frame_goto_should_continue_after_client_redirect[chromium]"
RECORDS = 6_526
GENERATED = frozenset({"attempts", "durationSeconds", "error", "outcome", "resultFile"})
RECEIPT_KIND = (
    "playwright-python-navigation-history-api-chromium-strict-paired-promotion-v4"
)

PromotionError = prior.PromotionError
require = prior.require
sealed = prior.sealed
sha256 = prior.sha256
json_object = prior.json_object
jsonl = prior.jsonl


def base_name(mode: str) -> str:
    return f"playwright-python-{mode}-navigation-full-chromium-strict-paired-v3.jsonl"


def output_name(mode: str) -> str:
    return f"playwright-python-{mode}-navigation-history-api-chromium-strict-paired-v4.jsonl"


def wheel_package_identity() -> None:
    """Bind the repaired wheel bytes to the package and native binary executed."""
    sealed(WHEEL, WHEEL_SHA256, "frozen ABI129-v6 repaired wheel")
    digest = hashlib.sha256()
    members: dict[str, bytes] = {}
    with zipfile.ZipFile(WHEEL) as archive:
        for info in archive.infolist():
            if not info.filename.startswith("pwright/") or info.is_dir():
                continue
            relative = info.filename.removeprefix("pwright/")
            require(
                relative
                and not relative.startswith("/")
                and ".." not in Path(relative).parts
                and relative not in members,
                "wheel package members are ambiguous",
            )
            if "__pycache__" in Path(relative).parts or Path(relative).suffix == ".pyc":
                continue
            members[relative] = archive.read(info)
    require(
        "_native.abi3.so" in members and "_abi.py" in members,
        "wheel package is incomplete",
    )
    for relative, raw in sorted(members.items()):
        name = relative.encode()
        digest.update(len(name).to_bytes(8, "big"))
        digest.update(name)
        digest.update(len(raw).to_bytes(8, "big"))
        digest.update(raw)
    require(
        digest.hexdigest() == PACKAGE_SHA256,
        "wheel package differs from executed package",
    )
    require(
        sha256(members["_native.abi3.so"]) == ARTIFACT_SHA256,
        "wheel native artifact differs from executed artifact",
    )
    require(b"NATIVE_ABI_VERSION = 129" in members["_abi.py"], "wheel ABI is not 129")


def source_inputs() -> None:
    sealed(MANIFEST, MANIFEST_SHA256, "official manifest")
    sealed(INVENTORY, INVENTORY_SHA256, "profiled inventory")
    sealed(
        UPSTREAM / "tests/async/test_navigation.py",
        UPSTREAM_NAVIGATION_SHA256,
        "upstream navigation source",
    )
    require(
        subprocess.run(
            ["git", "-C", str(UPSTREAM), "rev-parse", "HEAD"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        == UPSTREAM_COMMIT,
        "upstream checkout commit differs",
    )
    require(
        not subprocess.run(
            ["git", "-C", str(UPSTREAM), "status", "--porcelain"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip(),
        "upstream checkout is dirty",
    )
    wheel_package_identity()


def base() -> dict[str, tuple[list[bytes], list[dict[str, Any]]]]:
    base_promoter = (
        ROOT / "scripts/promote_playwright_python_navigation_full_strict_paired_v3.py"
    )
    base_verifier = (
        ROOT / "scripts/verify_playwright_python_navigation_full_strict_paired_v3.py"
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
        and receipt.get("completeFileExecution") is True,
        "base receipt identity differs",
    )
    _, inventory = jsonl(sealed(INVENTORY, INVENTORY_SHA256, "inventory"), "inventory")
    require(len(inventory) == RECORDS, "inventory row count differs")
    require(
        inventory[TARGET_INDEX].get("id") == TARGET_ID
        and inventory[TARGET_INDEX].get("sourceClassification") == "public-api-source"
        and inventory[ERROR_INDEX].get("id") == ERROR_ID,
        "target or synthetic-error inventory identity differs",
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
    require(
        originals["candidate"][1][TARGET_INDEX].get("outcome") == "failed"
        and originals["candidate"][1][ERROR_INDEX].get("outcome") == "error",
        "base target or synthetic-error outcome differs",
    )
    return originals


def report() -> tuple[dict[str, Any], float]:
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
        and payload.get("selections") == [TARGET_ID]
        and payload.get("manifest") == str(MANIFEST)
        and payload.get("manifestSha256") == MANIFEST_SHA256
        and payload.get("inventory") == str(INVENTORY)
        and payload.get("inventorySha256") == INVENTORY_SHA256
        and payload.get("isolateCases") is True
        and payload.get("isolatedCaseCount") == 1
        and payload.get("isolatedCaseIdsSha256") == ISOLATED_CASE_IDS_SHA256
        and payload.get("caseReports") == str(Path(f"{REPORT}.cases"))
        and payload.get("caseReportsSha256") == CASE_REPORTS_SHA256
        and payload.get("result") == str(REPORT)
        and payload.get("resultSha256") == REPORT_SHA256
        and payload.get("pwrightPackageSha256") == PACKAGE_SHA256
        and payload.get("pwrightArtifactSha256") == ARTIFACT_SHA256
        and payload.get("pwrightPackage")
        == str(BUILD_DIR / "candidate-abi129-v6/lib/python3.12/site-packages/pwright")
        and payload.get("fixtureBind") == "100.78.97.63"
        and payload.get("fixtureHost") == "macbookpro-yuacx.tail8f3b66.ts.net"
        and payload.get("exitCode") == 0
        and payload.get("bootstrapOnly") is False
        and payload.get("collectOnly") is False
        and payload.get("manifestCases") is False,
        "candidate focused run identity differs",
    )
    raw = sealed(REPORT, REPORT_SHA256, "candidate focused JUnit")
    root = ET.fromstring(raw)
    cases = list(root.iter("testcase"))
    require(len(cases) == 1, "candidate JUnit does not contain one case")
    case = cases[0]
    require(
        case.get("classname") == "tests.async.test_navigation"
        and case.get("name") == TARGET_ID.split("::", 1)[1],
        "candidate JUnit target ID differs",
    )
    require(
        not any(case.find(tag) is not None for tag in ("failure", "error", "skipped")),
        "candidate focused case is not passed",
    )
    try:
        duration = float(case.get("time", ""))
    except ValueError as error:
        raise PromotionError("candidate duration is invalid") from error
    require(math.isfinite(duration) and duration >= 0, "candidate duration is invalid")
    require(
        REPORT.read_bytes() == raw
        and sidecar.read_bytes()
        == sealed(sidecar, PROVENANCE_SHA256, "candidate provenance"),
        "candidate evidence changed during reading",
    )
    return payload, duration


def source_pins(payload: dict[str, Any]) -> bytes:
    pins = {
        "schema": 1,
        "kind": "playwright-python-navigation-history-api-candidate-v4",
        "upstreamCommit": UPSTREAM_COMMIT,
        "upstreamNavigationSha256": UPSTREAM_NAVIGATION_SHA256,
        "officialManifestSha256": MANIFEST_SHA256,
        "profiledInventorySha256": INVENTORY_SHA256,
        "targetTestId": TARGET_ID,
        "targetZeroBasedIndex": TARGET_INDEX,
        "installedNativeAbi": 129,
        "frozenWheel": str(WHEEL),
        "frozenWheelSha256": WHEEL_SHA256,
        "wheelName": WHEEL.name,
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
    duration: float,
    output: Path,
) -> tuple[bytes, bytes, dict[str, Any]]:
    lines, rows = original
    updated = list(lines)
    if mode == "candidate":
        replacement = {
            key: value
            for key, value in rows[TARGET_INDEX].items()
            if key not in GENERATED
        }
        replacement.update(
            attempts=1,
            durationSeconds=duration,
            outcome="passed",
            resultFile=str(REPORT),
        )
        updated[TARGET_INDEX] = (
            json.dumps(replacement, sort_keys=True, separators=(",", ":")) + "\n"
        ).encode()
        require(
            all(
                updated[index] == line
                for index, line in enumerate(lines)
                if index != TARGET_INDEX
            ),
            "candidate off-target row bytes changed",
        )
        require(
            sum(before != after for before, after in zip(lines, updated)) == 1,
            "candidate changed-row count differs",
        )
        require(
            json_object(updated[ERROR_INDEX], "preserved synthetic error").get(
                "outcome"
            )
            == "error",
            "base synthetic error was reclassified",
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
        {"error": 4, "failed": 108, "not-run": 4764, "passed": 1613, "skipped": 37}
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
    originals = base()
    payload, duration = report()
    content: dict[str, bytes] = {}
    summaries: dict[str, dict[str, Any]] = {}
    for mode in ("candidate", "oracle"):
        name = output_name(mode)
        ledger, summary_raw, summary = successor(
            mode, originals[mode], duration, output_dir / name
        )
        content[name] = ledger
        content[Path(name).with_suffix(".summary.json").name] = summary_raw
        summaries[mode] = summary
    pins = source_pins(payload)
    content["source-pins.json"] = pins
    receipt = {
        "schema": 2,
        "kind": RECEIPT_KIND,
        "scope": "tests/async/test_navigation.py-chromium-history-api-1",
        "publicApiParityEvidence": False,
        "fullCorpusExecution": False,
        "completeFileExecution": False,
        "fixtureAdapted": True,
        "launchMode": "cdp-adapter",
        "chromeVersionObserved": "155.0.8059.85",
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "recordsPerLane": RECORDS,
        "targetZeroBasedIndexes": [TARGET_INDEX],
        "targetTestIds": [TARGET_ID],
        "manifest": str(MANIFEST),
        "manifestSha256": MANIFEST_SHA256,
        "upstream": str(UPSTREAM),
        "upstreamCommit": UPSTREAM_COMMIT,
        "upstreamNavigationSha256": UPSTREAM_NAVIGATION_SHA256,
        "sourcePins": str(output_dir / "source-pins.json"),
        "sourcePinsSha256": sha256(pins),
        "frozenWheel": str(WHEEL),
        "frozenWheelSha256": WHEEL_SHA256,
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
                "observedOutcomes": {"passed": 1},
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
    prior.prior.prior.prior.prior.prior._check_output(output_dir)


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
        prior.prior.prior.prior.prior.prior._fsync_directory(staging)
        _check_output(output_dir)
        require(
            prepare(output_dir) == (content, receipt),
            "strict evidence changed during staging",
        )
        prior.prior.prior.prior.prior.prior._publish_exclusive(staging, output_dir)
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
