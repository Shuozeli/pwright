"""Publish only five source-bound Chromium Frame navigation row improvements.

The predecessor is the immutable 80-case navigation v1 pair. This focused,
fixture-adapted CDP rerun is neither a full-file rerun nor public API parity.
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

import promote_playwright_python_navigation_full_strict_paired_v1 as prior
from verify_playwright_python_official_provenance import (
    ProvenanceValidationError,
    verify_provenance,
)

ROOT = Path(__file__).resolve().parents[1]
OUTCOMES = ROOT / ".work/official-conformance/outcomes"
BASE_DIR = OUTCOMES / "python-navigation-full-chromium-strict-paired-v1"
OUTPUT_DIR = OUTCOMES / "python-navigation-frame-chromium-strict-paired-v2"
BUILD_DIR = (
    ROOT
    / ".work/official-conformance/builds/python-frame-navigation-postfix-oct20.6zcD6U"
)
REPORT = BUILD_DIR / "results/frame-navigation-candidate-frozen-v3-v1.xml"
REPORT_SHA256 = "ba396965142d2eaf3446d90604651bbcd8345793c0e96b69b8951347fb17e0f2"
PROVENANCE_SHA256 = "9b44b8f0ad4d869334b9cc6ebb5149fae8ae9b1e8506b3392d78ad63f6a681f3"
CASE_REPORTS_SHA256 = "df6fb82df06bc0c1ed3e43e1992641b90267c70f392cfe1053fac538c37427db"
ISOLATED_CASE_IDS_SHA256 = (
    "f0115e2aa61824cf677ca4a4c940ee772e119bb16cec138c3a28018dae8578b5"
)
WHEEL = (
    BUILD_DIR / "wheel-abi128-v3/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl"
)
WHEEL_SHA256 = "b3bf0fab066e6513db8b5d4ba9bfa555b18c726097689c4a5418ed6c675a5d0f"
PACKAGE_SHA256 = "506fb374098c906de43f2bb940ea53aae3be3390de48746b9dc87c18a7f1ff24"
ARTIFACT_SHA256 = "9a659567aaed311d3a947199944fbcda6724ab4d6c349e2534181cbab5241124"
BASE_PROMOTER_SHA256 = (
    "27e5ce34093865e6b4d2c127a11e5733fcb351bd26a50285ef0f08c76c72d384"
)
BASE_VERIFIER_SHA256 = (
    "ca173a347232b9f02a1cc1df8735461f889c24919375a1492803780e80f0f650"
)
BASE_RECEIPT_SHA256 = "6998ef7e55f2e24e879f515eb1e1f533bc4e427f67a68b911dc0a6701428f5e1"
BASES = {
    "candidate": (
        "03704b471a45711121894424f7024cbf21ba49e91a59b0d8c13d7845a4f38e71",
        "cc1183e3d3a74d49e301d526127429673b4115245a8a4cccf2bb9a2b63f8e1e9",
    ),
    "oracle": (
        "b209bf22f7777283798842abfd2a9774e4bb8f1659ae1b67e6015a3dc5a22479",
        "649634a74d79ab2b0019db2d7056ceef8d9f72f96c5451dc77f26ed770f3d011",
    ),
}
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-python")
UPSTREAM_COMMIT = "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
UPSTREAM_NAVIGATION_SHA256 = (
    "f04af8cbdee3dffb2c578d61d7685f02e5186ca565906323d71b6f92f96cd144"
)
MANIFEST = ROOT / "packages/pwright-python/tests/official/manifest.json"
MANIFEST_SHA256 = "6d4f86fee517e300319cfa87014a54302980bdf8cf538fa21c1daff4219ea724"
INVENTORY = ROOT / ".work/official-conformance/playwright-python-v1.58.0-profiled.jsonl"
INVENTORY_SHA256 = "3dfb11b6ad7eb4736166ee2c4f7fa32cda78063324ad8314ce12f3e2d49b9d41"
RECORDS = 6_526
TARGET_INDEXES = (2686, 2689, 2692, 2695, 2698)
TARGET_IDS = (
    "tests/async/test_navigation.py::test_frame_goto_should_navigate_subframes[chromium]",
    "tests/async/test_navigation.py::test_frame_goto_should_reject_when_frame_detaches[chromium]",
    "tests/async/test_navigation.py::test_frame_wait_for_load_state_should_work[chromium]",
    "tests/async/test_navigation.py::test_frame_wait_for_nav_should_fail_when_frame_detaches[chromium]",
    "tests/async/test_navigation.py::test_frame_wait_for_nav_should_work[chromium]",
)
GENERATED = frozenset({"attempts", "durationSeconds", "error", "outcome", "resultFile"})
RECEIPT_KIND = "playwright-python-navigation-frame-chromium-strict-paired-promotion-v2"

PromotionError = prior.PromotionError
require = prior.require
sealed = prior.sealed
sha256 = prior.sha256
json_object = prior.json_object
jsonl = prior.jsonl


def base_name(mode: str) -> str:
    return f"playwright-python-{mode}-navigation-full-chromium-strict-paired-v1.jsonl"


def output_name(mode: str) -> str:
    return f"playwright-python-{mode}-navigation-frame-chromium-strict-paired-v2.jsonl"


def wheel_package_identity() -> None:
    """Bind the installed package digest to the frozen wheel's actual bytes."""
    sealed(WHEEL, WHEEL_SHA256, "frozen ABI128-v3 wheel")
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
    require(b"NATIVE_ABI_VERSION = 128" in members["_abi.py"], "wheel ABI is not 128")


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


def base() -> tuple[dict[str, tuple[list[bytes], list[dict[str, Any]]]], list[str]]:
    """Verify the entire immutable predecessor and exact source-inventory slice."""
    base_promoter = (
        ROOT / "scripts/promote_playwright_python_navigation_full_strict_paired_v1.py"
    )
    base_verifier = (
        ROOT / "scripts/verify_playwright_python_navigation_full_strict_paired_v1.py"
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
    targets = [inventory[index].get("id") for index in TARGET_INDEXES]
    require(
        targets == list(TARGET_IDS)
        and all(
            inventory[index].get("sourceClassification") == "public-api-source"
            for index in TARGET_INDEXES
        ),
        "target IDs or source classifications differ",
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
        require(
            [rows[index].get("outcome") for index in TARGET_INDEXES]
            == ["failed" if mode == "candidate" else "passed"] * len(TARGET_INDEXES),
            f"{mode} target base outcomes differ",
        )
        originals[mode] = lines, rows
    return originals, targets


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
        and payload.get("selections")
        == [targets[0], targets[1], targets[2], targets[4], targets[3]]
        and payload.get("manifest") == str(MANIFEST)
        and payload.get("manifestSha256") == MANIFEST_SHA256
        and payload.get("inventory") == str(INVENTORY)
        and payload.get("inventorySha256") == INVENTORY_SHA256
        and payload.get("isolateCases") is True
        and payload.get("isolatedCaseCount") == 5
        and payload.get("isolatedCaseIdsSha256") == ISOLATED_CASE_IDS_SHA256
        and payload.get("caseReports") == str(Path(f"{REPORT}.cases"))
        and payload.get("caseReportsSha256") == CASE_REPORTS_SHA256
        and payload.get("result") == str(REPORT)
        and payload.get("resultSha256") == REPORT_SHA256
        and payload.get("pwrightPackageSha256") == PACKAGE_SHA256
        and payload.get("pwrightArtifactSha256") == ARTIFACT_SHA256
        and payload.get("pwrightPackage")
        == str(
            BUILD_DIR / "candidate-frame-v3-frozen/lib/python3.12/site-packages/pwright"
        )
        and payload.get("fixtureBind") == "100.78.97.63"
        and payload.get("fixtureHost") == "macbookpro-yuacx.tail8f3b66.ts.net"
        and payload.get("exitCode") == 0
        and payload.get("bootstrapOnly") is False
        and payload.get("collectOnly") is False
        and payload.get("manifestCases") is False,
        "candidate focused run identity differs",
    )
    raw = sealed(REPORT, REPORT_SHA256, "candidate JUnit")
    root = ET.fromstring(raw)
    cases = list(root.iter("testcase"))
    require(len(cases) == 5, "candidate JUnit does not contain five cases")
    mapped: dict[str, dict[str, Any]] = {}
    for case in cases:
        test_id = prior._case_id(case)
        require(
            test_id in TARGET_IDS and test_id not in mapped,
            "candidate JUnit IDs differ",
        )
        require(
            all(case.find(tag) is None for tag in ("failure", "error", "skipped")),
            f"candidate JUnit is not green: {test_id}",
        )
        try:
            duration = float(case.get("time", ""))
        except ValueError as error:
            raise PromotionError(f"candidate duration is invalid: {test_id}") from error
        require(
            math.isfinite(duration) and duration >= 0,
            f"candidate duration is invalid: {test_id}",
        )
        mapped[test_id] = {"outcome": "passed", "durationSeconds": duration}
    require(set(mapped) == set(targets), "candidate JUnit target set differs")
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
        "kind": "playwright-python-navigation-frame-five-candidate",
        "upstreamCommit": UPSTREAM_COMMIT,
        "upstreamNavigationSha256": UPSTREAM_NAVIGATION_SHA256,
        "officialManifestSha256": MANIFEST_SHA256,
        "profiledInventorySha256": INVENTORY_SHA256,
        "installedNativeAbi": 128,
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
    cases: dict[str, dict[str, Any]],
    output: Path,
) -> tuple[bytes, bytes, dict[str, Any]]:
    lines, rows = original
    updated = list(lines)
    if mode == "candidate":
        for index, test_id in zip(TARGET_INDEXES, TARGET_IDS):
            require(rows[index]["id"] == test_id, "target source row moved")
            replacement = {
                key: value for key, value in rows[index].items() if key not in GENERATED
            }
            replacement.update(
                attempts=1,
                durationSeconds=cases[test_id]["durationSeconds"],
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
            sum(before != after for before, after in zip(lines, updated)) == 5,
            "candidate changed-row count differs",
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
        {"error": 3, "failed": 120, "not-run": 4764, "passed": 1602, "skipped": 37}
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
            mode, originals[mode], cases, output_dir / name
        )
        content[name] = ledger
        content[Path(name).with_suffix(".summary.json").name] = summary_raw
        summaries[mode] = summary
    pins = source_pins(payload)
    content["source-pins.json"] = pins
    receipt = {
        "schema": 2,
        "kind": RECEIPT_KIND,
        "scope": "tests/async/test_navigation.py-chromium-frame-five",
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
                "observedOutcomes": {"passed": 5},
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
    prior.prior.prior.prior._check_output(output_dir)


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
        prior.prior.prior.prior._fsync_directory(staging)
        _check_output(output_dir)
        require(
            prepare(output_dir) == (content, receipt),
            "strict evidence changed during staging",
        )
        prior.prior.prior.prior._publish_exclusive(staging, output_dir)
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
