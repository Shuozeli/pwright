"""Publish the source-bound ABI128 full-file Python navigation successor.

Exactly 80 Chromium candidate rows are projected from one official full-file
report. The other 6,446 candidate rows and all 6,526 oracle rows retain their
v2 bytes. The synthetic report error remains an error, not a focused-run pass.
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

import promote_playwright_python_navigation_frame_strict_paired_v2 as prior
from verify_playwright_python_official_provenance import (
    ProvenanceValidationError,
    verify_provenance,
)

ROOT = Path(__file__).resolve().parents[1]
OUTCOMES = ROOT / ".work/official-conformance/outcomes"
BASE_DIR = OUTCOMES / "python-navigation-frame-chromium-strict-paired-v2"
OUTPUT_DIR = OUTCOMES / "python-navigation-full-chromium-strict-paired-v3"
BUILD_DIR = (
    ROOT
    / ".work/official-conformance/builds/python-frame-navigation-postfix-oct20.6zcD6U"
)
REPORT = BUILD_DIR / "results/navigation-candidate-full-abi128-v5.xml"
REPORT_SHA256 = "8082a55316d549b02a5c21fc1311224b5e6eda6476978e65a94f4d5d78e4d193"
PROVENANCE_SHA256 = "9098f4ee457aabacae159351b42c6abc60cad40d254dd4380088aa7b1a9c90b6"
CASE_REPORTS_SHA256 = "b996b2202b06122b6eae4c70d0c1ba7a862e437886e350d4e1f537d2b6c3d440"
ISOLATED_CASE_IDS_SHA256 = (
    "4b9f37f3a3857ddf59e9b5766255f84dfe83d3d5af76285208e176088f656696"
)
WHEEL = (
    BUILD_DIR / "wheel-abi128-v5/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl"
)
WHEEL_SHA256 = "fb342855ca70473c59a84729cce4b44eafea0cb3b3b506929831e88385bce108"
PACKAGE_SHA256 = "0070c191a57572e533940f56fb41761a51926a80f694cd0017c6cd5f6b672513"
ARTIFACT_SHA256 = "9a659567aaed311d3a947199944fbcda6724ab4d6c349e2534181cbab5241124"
BASE_PROMOTER_SHA256 = (
    "a350549a3a40e80660610af0ce80e02bbc83840abdad5c00575f3a7b9764db91"
)
BASE_VERIFIER_SHA256 = (
    "a1c5e547f8f599a05c3023c3c4f02e605f2853b34c1a2efcdcfcdfe2a34826c9"
)
BASE_RECEIPT_SHA256 = "a35ba350fdd40a0e91251980fc98cc7d0f461f009268c616fe54cdf052660139"
BASES = {
    "candidate": (
        "792944338f6092a3ca538102ad65b5545e9320a615e1ef3adf89b7e5b434f002",
        "b1f084795f1e818cba17db3a5bb37fa0422f776acae855834359fcfb93d47b21",
    ),
    "oracle": (
        "b209bf22f7777283798842abfd2a9774e4bb8f1659ae1b67e6015a3dc5a22479",
        "954069e28e225b0be6493e2a32d84b69823432946e3990571850844a354477a7",
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
TARGET_INDEXES = tuple(2680 + 3 * ordinal for ordinal in range(80))
GENERATED = frozenset({"attempts", "durationSeconds", "error", "outcome", "resultFile"})
RECEIPT_KIND = "playwright-python-navigation-full-chromium-strict-paired-promotion-v3"

PromotionError = prior.PromotionError
require = prior.require
sealed = prior.sealed
sha256 = prior.sha256
json_object = prior.json_object
jsonl = prior.jsonl


def base_name(mode: str) -> str:
    return f"playwright-python-{mode}-navigation-frame-chromium-strict-paired-v2.jsonl"


def output_name(mode: str) -> str:
    return f"playwright-python-{mode}-navigation-full-chromium-strict-paired-v3.jsonl"


def wheel_package_identity() -> None:
    """Match the entire frozen wheel package to the recorded executed package."""
    sealed(WHEEL, WHEEL_SHA256, "frozen ABI128-v5 wheel")
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
    base_promoter = (
        ROOT / "scripts/promote_playwright_python_navigation_frame_strict_paired_v2.py"
    )
    base_verifier = (
        ROOT / "scripts/verify_playwright_python_navigation_frame_strict_paired_v2.py"
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
        ),
        "80 full-file target identities differ",
    )
    require(
        sum(
            isinstance(row.get("id"), str)
            and row["id"].startswith("tests/async/test_navigation.py::")
            and row["id"].endswith("[chromium]")
            for row in inventory
        )
        == 80,
        "inventory includes unselected Chromium navigation rows",
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
        and payload.get("pwrightPackage")
        == str(
            BUILD_DIR
            / "candidate-abi128-v5-full-nav/lib/python3.12/site-packages/pwright"
        )
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
        test_id = prior.prior._case_id(case)
        require(
            test_id in targets and test_id not in mapped, "candidate JUnit IDs differ"
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
        dict(counts) == {"passed": 72, "failed": 7, "error": 1},
        "candidate full-file outcome counts differ",
    )
    require(
        mapped[
            "tests/async/test_navigation.py::test_frame_goto_should_continue_after_client_redirect[chromium]"
        ]["outcome"]
        == "error",
        "synthetic error case was reclassified",
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
        "kind": "playwright-python-navigation-full-file-candidate-v3",
        "upstreamCommit": UPSTREAM_COMMIT,
        "upstreamNavigationSha256": UPSTREAM_NAVIGATION_SHA256,
        "officialManifestSha256": MANIFEST_SHA256,
        "profiledInventorySha256": INVENTORY_SHA256,
        "chromiumExpandedCases": 80,
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
        {"error": 4, "failed": 109, "not-run": 4764, "passed": 1612, "skipped": 37}
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
                "observedOutcomes": {"passed": 72, "failed": 7, "error": 1},
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
    prior.prior.prior.prior.prior._check_output(output_dir)


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
        prior.prior.prior.prior.prior._fsync_directory(staging)
        _check_output(output_dir)
        require(
            prepare(output_dir) == (content, receipt),
            "strict evidence changed during staging",
        )
        prior.prior.prior.prior.prior._publish_exclusive(staging, output_dir)
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
