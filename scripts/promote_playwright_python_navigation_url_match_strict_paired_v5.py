"""Publish a bounded, source-bound three-case URL-matched navigation successor.

It replaces exactly three failed candidate rows only when all three official
cases pass; every other candidate row and the entire oracle ledger retain v4
bytes. The new report is focused, not a fresh full-file execution.
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

import promote_playwright_python_navigation_history_api_strict_paired_v4 as prior
from verify_playwright_python_official_provenance import (
    ProvenanceValidationError,
    verify_provenance,
)

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / ".work/official-conformance"
OUTCOMES = WORK / "outcomes"
BASE_DIR = OUTCOMES / "python-navigation-history-api-chromium-strict-paired-v4"
OUTPUT_DIR = OUTCOMES / "python-navigation-url-match-chromium-strict-paired-v5"

BUILD_DIR = WORK / "builds/python-navigation-url-match-abi130-v2-oct20.srSIiv"
CANDIDATE_PACKAGE = (
    BUILD_DIR / "candidate-abi130-v2/lib/python3.12/site-packages/pwright"
)
REPORT = BUILD_DIR / "results/navigation-url-match-candidate-abi130-v2.xml"
IDENTITY = BUILD_DIR / "results/abi130-v2-identity.json"
WHEEL = BUILD_DIR / "wheel/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl"
REPORT_SHA256 = "d0e3f4d643d9093c40f5bc6627f36cd35b00dc5f7cf5edf5dcee2572f9316dfe"
PROVENANCE_SHA256 = "16be690a4210ba3c1b4dd6cb669d5ed50c3a9500643eb1783b3fb7323137be22"
IDENTITY_SHA256 = "aeb78b8ff2a6acd554b210491f8b73e9e27cae47faa29ddd128106b38ae7f2f7"
WHEEL_SHA256 = "40889db525656b5410a994c81e79530f873531c28ac0e8c9052e8603a87efb78"
PACKAGE_SHA256 = "c66d04b01ca060ccbaf31f2aed127b51aebe34834d8b8b0246391f5fb489446e"
ARTIFACT_SHA256 = "ef567004a447ee148e988961b04868a8d1063f885c816d10845a10e5c6c36df8"
EVIDENCE_PINNED = True

BASE_PROMOTER_SHA256 = (
    "895e9eb44ed33dcabf04d97415c653be058afa7448bdbb5e2ed522577d1485a4"
)
BASE_VERIFIER_SHA256 = (
    "d67da041dd15cfab5dd7ad351f23d515b671b06a7b16b74fbf3ad9c088f105fd"
)
BASE_RECEIPT_SHA256 = "541e82e5805207de17299295b9e9dfe3ca4c04c20771f36981b1db5e2ef0c4b6"
BASES = {
    "candidate": (
        "1274adf9a571c73c425484e2a934f3101a9b9a52bebb0e61ffa641bc67dfba31",
        "28d1da5e92374497026a9670b58bdff0decea430e068424245ddce7456ed3768",
    ),
    "oracle": (
        "b209bf22f7777283798842abfd2a9774e4bb8f1659ae1b67e6015a3dc5a22479",
        "0bf6e764521ee3235ad0e19f64dcab364a618122d88492b826f605f76ef80b70",
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
TARGET_INDEXES = (2881, 2914, 2917)
TARGET_IDS = (
    "tests/async/test_navigation.py::test_wait_for_nav_should_respect_timeout[chromium]",
    "tests/async/test_navigation.py::test_wait_for_nav_should_work_with_url_match[chromium]",
    "tests/async/test_navigation.py::test_wait_for_nav_should_work_with_url_match_for_same_document_navigations[chromium]",
)
ERROR_INDEX = 2683
ERROR_ID = "tests/async/test_navigation.py::test_frame_goto_should_continue_after_client_redirect[chromium]"
RECORDS = 6_526
GENERATED = frozenset({"attempts", "durationSeconds", "error", "outcome", "resultFile"})
RECEIPT_KIND = (
    "playwright-python-navigation-url-match-chromium-strict-paired-promotion-v5"
)

PromotionError = prior.PromotionError
require = prior.require
sealed = prior.sealed
sha256 = prior.sha256
json_object = prior.json_object
jsonl = prior.jsonl


def base_name(mode: str) -> str:
    return f"playwright-python-{mode}-navigation-history-api-chromium-strict-paired-v4.jsonl"


def output_name(mode: str) -> str:
    return (
        f"playwright-python-{mode}-navigation-url-match-chromium-strict-paired-v5.jsonl"
    )


def _evidence_pinned() -> None:
    require(EVIDENCE_PINNED, "ABI130 official evidence has not been pinned")
    for label, digest in (
        ("candidate JUnit", REPORT_SHA256),
        ("candidate provenance", PROVENANCE_SHA256),
        ("ABI130 identity", IDENTITY_SHA256),
        ("frozen ABI130 wheel", WHEEL_SHA256),
        ("executed package", PACKAGE_SHA256),
        ("executed artifact", ARTIFACT_SHA256),
    ):
        require(
            len(digest) == 64
            and all(character in "0123456789abcdef" for character in digest),
            f"{label} lacks a SHA-256 pin",
        )


def wheel_package_identity() -> None:
    """Bind the immutable ABI130 wheel to the package and binary executed."""
    _evidence_pinned()
    sealed(WHEEL, WHEEL_SHA256, "frozen ABI130 wheel")
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
    require(b"NATIVE_ABI_VERSION = 130" in members["_abi.py"], "wheel ABI is not 130")


def source_inputs() -> None:
    _evidence_pinned()
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


def identity() -> dict[str, Any]:
    value = json_object(
        sealed(IDENTITY, IDENTITY_SHA256, "ABI130 build identity"),
        "ABI130 build identity",
    )
    installed = value.get("installedIdentity")
    run = value.get("officialRun")
    require(
        value.get("schema") == 1
        and value.get("kind")
        == "pwright-python-navigation-url-match-abi130-v2-source-bound-identity"
        and value.get("wheelPath") == str(WHEEL)
        and value.get("wheelSha256") == WHEEL_SHA256
        and isinstance(installed, dict)
        and installed.get("abi") == 130
        and installed.get("nativeModule") == str(CANDIDATE_PACKAGE / "_native.abi3.so")
        and "navigation-url-match" in installed.get("selectedFeatures", [])
        and value.get("nativeArtifactSha256") == ARTIFACT_SHA256
        and value.get("installedPackageSha256") == PACKAGE_SHA256
        and isinstance(run, dict)
        and run.get("resultPath") == str(REPORT)
        and run.get("resultSha256") == REPORT_SHA256
        and run.get("provenancePath") == str(REPORT.with_suffix(".provenance.json"))
        and run.get("exitCode") == 0
        and run.get("selections") == list(TARGET_IDS)
        and run.get("upstreamCommit") == UPSTREAM_COMMIT
        and run.get("manifestSha256") == MANIFEST_SHA256
        and (
            run.get("tests"),
            run.get("failures"),
            run.get("errors"),
            run.get("skipped"),
        )
        == (3, 0, 0, 0),
        "ABI130 source-bound identity differs",
    )
    return value


def base() -> dict[str, tuple[list[bytes], list[dict[str, Any]]]]:
    base_promoter = (
        ROOT
        / "scripts/promote_playwright_python_navigation_history_api_strict_paired_v4.py"
    )
    base_verifier = (
        ROOT
        / "scripts/verify_playwright_python_navigation_history_api_strict_paired_v4.py"
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
    require(
        all(
            inventory[index].get("id") == test_id
            and inventory[index].get("sourceClassification") == "public-api-source"
            for index, test_id in zip(TARGET_INDEXES, TARGET_IDS)
        )
        and inventory[ERROR_INDEX].get("id") == ERROR_ID,
        "target or synthetic-error inventory identities differ",
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
        all(
            originals["candidate"][1][index].get("outcome") == "failed"
            for index in TARGET_INDEXES
        )
        and originals["candidate"][1][ERROR_INDEX].get("outcome") == "error",
        "base target or synthetic-error outcomes differ",
    )
    return originals


def report() -> tuple[dict[str, Any], dict[str, float]]:
    _evidence_pinned()
    identity()
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
        and len(payload.get("selections", [])) == 3
        and set(payload["selections"]) == set(TARGET_IDS)
        and payload.get("manifest") == str(MANIFEST)
        and payload.get("manifestSha256") == MANIFEST_SHA256
        and payload.get("inventory") is None
        and payload.get("inventorySha256") is None
        and payload.get("isolateCases") is False
        and payload.get("isolatedCaseCount") == 0
        and payload.get("isolatedCaseIdsSha256") is None
        and payload.get("caseReports") is None
        and payload.get("caseReportsSha256") is None
        and payload.get("result") == str(REPORT)
        and payload.get("resultSha256") == REPORT_SHA256
        and payload.get("pwrightPackageSha256") == PACKAGE_SHA256
        and payload.get("pwrightArtifactSha256") == ARTIFACT_SHA256
        and payload.get("pwrightPackage") == str(CANDIDATE_PACKAGE)
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
    require(len(cases) == 3, "candidate JUnit does not contain three cases")
    mapped: dict[str, float] = {}
    for case in cases:
        require(
            case.get("classname") == "tests.async.test_navigation",
            "candidate JUnit class differs",
        )
        name = case.get("name")
        test_id = f"tests/async/test_navigation.py::{name}"
        require(
            test_id in TARGET_IDS and test_id not in mapped,
            "candidate JUnit IDs differ",
        )
        require(
            not any(
                case.find(tag) is not None for tag in ("failure", "error", "skipped")
            ),
            f"candidate focused case did not pass: {test_id}",
        )
        try:
            duration = float(case.get("time", ""))
        except ValueError as error:
            raise PromotionError(f"candidate duration is invalid: {test_id}") from error
        require(
            math.isfinite(duration) and duration >= 0,
            f"candidate duration is invalid: {test_id}",
        )
        mapped[test_id] = duration
    require(set(mapped) == set(TARGET_IDS), "candidate JUnit target set differs")
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
        "kind": "playwright-python-navigation-url-match-candidate-v5",
        "upstreamCommit": UPSTREAM_COMMIT,
        "upstreamNavigationSha256": UPSTREAM_NAVIGATION_SHA256,
        "officialManifestSha256": MANIFEST_SHA256,
        "profiledInventorySha256": INVENTORY_SHA256,
        "targetTestIds": list(TARGET_IDS),
        "targetZeroBasedIndexes": list(TARGET_INDEXES),
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
    durations: dict[str, float],
    output: Path,
) -> tuple[bytes, bytes, dict[str, Any]]:
    lines, rows = original
    updated = list(lines)
    if mode == "candidate":
        for index, test_id in zip(TARGET_INDEXES, TARGET_IDS):
            replacement = {
                key: value for key, value in rows[index].items() if key not in GENERATED
            }
            replacement.update(
                attempts=1,
                durationSeconds=durations[test_id],
                outcome="passed",
                resultFile=str(REPORT),
            )
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
            sum(before != after for before, after in zip(lines, updated)) == 3,
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
        {"error": 4, "failed": 105, "not-run": 4764, "passed": 1616, "skipped": 37}
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
    payload, durations = report()
    content: dict[str, bytes] = {}
    summaries: dict[str, dict[str, Any]] = {}
    for mode in ("candidate", "oracle"):
        name = output_name(mode)
        ledger, summary_raw, summary = successor(
            mode, originals[mode], durations, output_dir / name
        )
        content[name] = ledger
        content[Path(name).with_suffix(".summary.json").name] = summary_raw
        summaries[mode] = summary
    pins = source_pins(payload)
    content["source-pins.json"] = pins
    receipt = {
        "schema": 2,
        "kind": RECEIPT_KIND,
        "scope": "tests/async/test_navigation.py-chromium-url-match-3",
        "publicApiParityEvidence": False,
        "fullCorpusExecution": False,
        "completeFileExecution": False,
        "fixtureAdapted": True,
        "launchMode": "cdp-adapter",
        "chromeVersionObserved": "155.0.8059.85",
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "recordsPerLane": RECORDS,
        "targetZeroBasedIndexes": list(TARGET_INDEXES),
        "targetTestIds": list(TARGET_IDS),
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
                "observedOutcomes": {"passed": 3},
                "isolateCases": False,
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
    prior.prior.prior.prior.prior.prior.prior._check_output(output_dir)


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
        prior.prior.prior.prior.prior.prior.prior._fsync_directory(staging)
        _check_output(output_dir)
        require(
            prepare(output_dir) == (content, receipt),
            "strict evidence changed during staging",
        )
        prior.prior.prior.prior.prior.prior.prior._publish_exclusive(
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
