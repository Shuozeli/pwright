"""Publish the source-bound, full-file Chromium navigation Python pair.

Only the 80 pinned ``tests/async/test_navigation.py`` Chromium rows may change.
This is fixture-adapted CDP evidence, not a full-corpus or API-parity claim.
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
from collections import Counter
from pathlib import Path
from typing import Any

import promote_playwright_python_upstream_skip_callable_fixtures_strict_v2 as prior
from verify_playwright_python_official_provenance import (
    ProvenanceValidationError,
    verify_provenance,
)

ROOT = Path(__file__).resolve().parents[1]
OUTCOMES = ROOT / ".work/official-conformance/outcomes"
BASE_DIR = OUTCOMES / "python-upstream-skip-callable-fixtures-strict-paired-v2"
OUTPUT_DIR = OUTCOMES / "python-navigation-full-chromium-strict-paired-v1"
BUILD_DIR = (
    ROOT
    / ".work/official-conformance/builds/python-navigation-full-abi127-oct20.bB5H3w"
)
RESULT_DIR = BUILD_DIR / "results"
SOURCE_PINS = BUILD_DIR / "source-pins.json"
SOURCE_PINS_SHA256 = "cd5f3360d8b25a5546162600ed71bb63bcc73c997e1bbb4f71e0a91bf2a063a3"
WHEEL = BUILD_DIR / "frozen/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl"
WHEEL_SHA256 = "05142f1065ec5e3de608da0ebad697d8888131f480099b7bb12a3472b2b5eeb4"
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-python")
UPSTREAM_COMMIT = "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
UPSTREAM_NAVIGATION_SHA256 = (
    "f04af8cbdee3dffb2c578d61d7685f02e5186ca565906323d71b6f92f96cd144"
)
MANIFEST = ROOT / "packages/pwright-python/tests/official/manifest.json"
MANIFEST_SHA256 = "6d4f86fee517e300319cfa87014a54302980bdf8cf538fa21c1daff4219ea724"
INVENTORY = ROOT / ".work/official-conformance/playwright-python-v1.58.0-profiled.jsonl"
INVENTORY_SHA256 = "3dfb11b6ad7eb4736166ee2c4f7fa32cda78063324ad8314ce12f3e2d49b9d41"
BASE_PROMOTER_SHA256 = (
    "891543d38877631763806b68a4449d0e7689e7269f3473ebfe27d2ecd7401bd1"
)
BASE_RECEIPT_SHA256 = "1b8ce858fa3ab643fb10e8f12142c12b32e3164b8719e2c627d2f6e4a66dbb4a"
BASES = {
    "candidate": (
        "bc150cd75730d0c0a0fa8fab6fc0743d7fa81209304474e917dc70e13194a81d",
        "a36899c92dc5aa9b1ad9b19b26a61155bc5fa81b17f700c032e2ce8247093cc9",
    ),
    "oracle": (
        "3b4c8e43d1a0398e1e2ec6bfdde636a5ce3be0e5d7c3fe8e110016a721bb6934",
        "8057b5670e705c208f28b71d318c6452fe1cd9d00e2e3dc9022dc6c0772be27e",
    ),
}
REPORT_PINS = {
    "candidate": {
        "provenance": "b54a1408045342de9249a80b8dc0e77c51bb5261048854f67387f610f15c637e",
        "result": "0a6bb9528dbf49ec7f761e37e8aaaf4813cc2c82385f9c8dc3e7081267be8591",
        "caseReports": "f40f4cfaa98b555b4645e04a90e85f294577bc7163c1bcbab821dd7ff1833c04",
        "outcomes": {"failed": 23, "passed": 57},
    },
    "oracle": {
        "provenance": "9d2cb134fcd79ab56142b8f8cb63d5b5d88aac43c7564877039fdf706fd308da",
        "result": "3d0bc87ff1615ff4d6c2488e5e07107df1d5d6d4da3945312f175edcbd3114e0",
        "caseReports": "68c219ce12e9875a2a5267980b25864ad242d809096e5dc55ddc134c27e717cd",
        "outcomes": {"failed": 3, "passed": 77},
    },
}
RECORDS = 6_526
MODES = ("candidate", "oracle")
TARGET_INDEXES = tuple(2680 + 3 * ordinal for ordinal in range(80))
GENERATED = frozenset({"attempts", "durationSeconds", "error", "outcome", "resultFile"})
RECEIPT_KIND = "playwright-python-navigation-full-chromium-strict-paired-promotion-v1"
ISOLATED_CASE_IDS_SHA256 = (
    "4b9f37f3a3857ddf59e9b5766255f84dfe83d3d5af76285208e176088f656696"
)

PromotionError = prior.PromotionError
require = prior.require
sha256 = prior.sha256
sealed = prior.sealed
json_object = prior.json_object
jsonl = prior.jsonl


def base_name(mode: str) -> str:
    return f"playwright-python-{mode}-upstream-skip-callable-fixtures-strict-paired-v2.jsonl"


def output_name(mode: str) -> str:
    return f"playwright-python-{mode}-navigation-full-chromium-strict-paired-v1.jsonl"


def report_path(mode: str) -> Path:
    return RESULT_DIR / f"navigation-{mode}-full-v1.xml"


def source_inputs() -> dict[str, Any]:
    """Recheck the frozen build and upstream source identities."""
    pins = json_object(
        sealed(SOURCE_PINS, SOURCE_PINS_SHA256, "source pins"), "source pins"
    )
    require(
        pins.get("kind") == "playwright-python-navigation-full-file-candidate"
        and pins.get("upstreamCommit") == UPSTREAM_COMMIT
        and pins.get("upstreamNavigationSha256") == UPSTREAM_NAVIGATION_SHA256
        and pins.get("officialManifestSha256") == MANIFEST_SHA256
        and pins.get("profiledInventorySha256") == INVENTORY_SHA256
        and pins.get("chromiumExpandedCases") == 80
        and pins.get("installedNativeAbi") == 127
        and pins.get("installedSelectedFeatures") == 109
        and pins.get("wheelName") == WHEEL.name
        and pins.get("frozenWheelSha256") == WHEEL_SHA256,
        "frozen source pins differ",
    )
    sealed(WHEEL, WHEEL_SHA256, "frozen ABI127 wheel")
    sealed(MANIFEST, MANIFEST_SHA256, "official manifest")
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
    return pins


def base() -> tuple[dict[str, tuple[list[bytes], list[dict[str, Any]]]], list[str]]:
    """Prove the full predecessor chain and preserve its off-target bytes."""
    base_promoter = (
        ROOT
        / "scripts/promote_playwright_python_upstream_skip_callable_fixtures_strict_v2.py"
    )
    require(
        Path(prior.__file__).resolve() == base_promoter,
        "predecessor module path differs",
    )
    sealed(base_promoter, BASE_PROMOTER_SHA256, "base promoter")
    predecessor = prior.verify_bundle(BASE_DIR)
    receipt = json_object(
        sealed(
            BASE_DIR / "promotion-receipt.json", BASE_RECEIPT_SHA256, "base receipt"
        ),
        "base receipt",
    )
    require(predecessor == receipt, "predecessor receipt differs after verification")
    require(
        receipt.get("kind") == prior.RECEIPT_KIND
        and receipt.get("inventory") == str(INVENTORY)
        and receipt.get("inventorySha256") == INVENTORY_SHA256
        and receipt.get("recordsPerLane") == RECORDS
        and receipt.get("commit") == UPSTREAM_COMMIT
        and receipt.get("promoterSha256") == BASE_PROMOTER_SHA256
        and receipt.get("publicApiParityEvidence") is False,
        "base receipt identity differs",
    )
    _, inventory = jsonl(sealed(INVENTORY, INVENTORY_SHA256, "inventory"), "inventory")
    ids = [row.get("id") for row in inventory]
    require(len(ids) == RECORDS and len(set(ids)) == RECORDS, "inventory IDs differ")
    targets = [ids[index] for index in TARGET_INDEXES]
    require(
        len(TARGET_INDEXES) == len(set(TARGET_INDEXES)) == 80
        and len(set(targets)) == 80
        and all(
            isinstance(test_id, str)
            and test_id.startswith("tests/async/test_navigation.py::")
            and test_id.endswith("[chromium]")
            for test_id in targets
        )
        and all(
            inventory[index].get("sourceClassification") == "public-api-source"
            for index in TARGET_INDEXES
        ),
        "80 navigation targets differ from source-pinned inventory",
    )
    # This file has one expanded row per browser; the 80 Chromium rows are the full selection.
    require(
        sum(
            isinstance(test_id, str)
            and test_id.startswith("tests/async/test_navigation.py::")
            and test_id.endswith("[chromium]")
            for test_id in ids
        )
        == 80,
        "inventory contains other Chromium navigation rows",
    )
    originals: dict[str, tuple[list[bytes], list[dict[str, Any]]]] = {}
    for mode in MODES:
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
        require(
            summary
            == {
                "schema": 1,
                "inventory": str(INVENTORY),
                "records": RECORDS,
                "outcomes": dict(
                    sorted(Counter(row.get("outcome") for row in rows).items())
                ),
                "ledgerSha256": ledger_sha,
                "ledger": str(ledger_path),
            },
            f"{mode} base count or identity differs",
        )
        for index, (row, source) in enumerate(zip(rows, inventory)):
            require(
                row.get("id") == ids[index], f"{mode} base row order differs at {index}"
            )
            require(
                {key: value for key, value in row.items() if key not in GENERATED}
                == source,
                f"{mode} base source metadata differs at {index}",
            )
        old = Counter(rows[index]["outcome"] for index in TARGET_INDEXES)
        require(
            old
            == (
                {"not-run": 78, "passed": 2} if mode == "candidate" else {"not-run": 80}
            ),
            f"{mode} prior navigation outcomes differ",
        )
        if mode == "candidate":
            require(
                {
                    index
                    for index in TARGET_INDEXES
                    if rows[index]["outcome"] == "passed"
                }
                == {2710, 2713},
                "candidate prior pass identities differ",
            )
        originals[mode] = lines, rows
    return originals, targets


def _case_id(case: ET.Element) -> str:
    classname = case.get("classname")
    name = case.get("name")
    require(
        isinstance(classname, str)
        and classname == "tests.async.test_navigation"
        and isinstance(name, str)
        and name.startswith("test_")
        and name.endswith("[chromium]"),
        "JUnit case identity differs",
    )
    return f"tests/async/test_navigation.py::{name}"


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
    require(
        node.tag == "failure", "navigation full-file evidence contains error or skip"
    )
    message = (node.get("message") or node.text or "").splitlines()
    require(bool(message) and bool(message[0]), "failure lacks a diagnostic")
    return "failed", message[0]


def reports(targets: list[str]) -> dict[str, dict[str, Any]]:
    """Verify pinned full-file raw reports and extract all 80 actual outcomes."""
    verified: dict[str, dict[str, Any]] = {}
    for mode in MODES:
        path = report_path(mode)
        sidecar = path.with_suffix(".provenance.json")
        pins = REPORT_PINS[mode]
        payload = json_object(
            sealed(sidecar, pins["provenance"], f"{mode} provenance"),
            f"{mode} provenance",
        )
        actual = verify_provenance(payload, provenance_path=sidecar)
        require(
            actual == payload
            and sidecar.read_bytes()
            == sealed(sidecar, pins["provenance"], f"{mode} provenance"),
            f"{mode} provenance changed",
        )
        require(
            payload.get("schema") == 5
            and payload.get("mode") == mode
            and payload.get("commit") == UPSTREAM_COMMIT
            and payload.get("manifest") == str(MANIFEST)
            and payload.get("manifestSha256") == MANIFEST_SHA256
            and payload.get("inventory") == str(INVENTORY)
            and payload.get("inventorySha256") == INVENTORY_SHA256
            and payload.get("launchMode") == "cdp-adapter"
            and payload.get("scope") == "selected"
            and payload.get("browsers") == ["chromium"]
            and payload.get("selections") == ["tests/async/test_navigation.py"]
            and payload.get("manifestCases") is False
            and payload.get("bootstrapOnly") is False
            and payload.get("collectOnly") is False
            and payload.get("isolateCases") is True
            and payload.get("isolatedCaseCount") == 80
            and payload.get("isolatedCaseIdsSha256") == ISOLATED_CASE_IDS_SHA256
            and payload.get("caseTimeout") == 45.0
            and payload.get("fixtureBind") == "100.78.97.63"
            and payload.get("fixtureHost") == "macbookpro-yuacx.tail8f3b66.ts.net"
            and payload.get("exitCode") == 1
            and payload.get("result") == str(path)
            and payload.get("resultSha256") == pins["result"]
            and payload.get("caseReports") == str(Path(f"{path}.cases"))
            and payload.get("caseReportsSha256") == pins["caseReports"],
            f"{mode} full-file source-bound run identity differs",
        )
        raw = sealed(path, pins["result"], f"{mode} JUnit")
        root = ET.fromstring(raw)
        cases = list(root.iter("testcase"))
        require(len(cases) == 80, f"{mode} JUnit must contain exactly 80 cases")
        mapped: dict[str, dict[str, Any]] = {}
        counts: Counter[str] = Counter()
        for case in cases:
            test_id = _case_id(case)
            require(test_id not in mapped, f"{mode} JUnit duplicates {test_id}")
            try:
                duration = float(case.get("time", ""))
            except ValueError as error:
                raise PromotionError(
                    f"{mode} JUnit duration is invalid: {test_id}"
                ) from error
            require(
                math.isfinite(duration) and duration >= 0,
                f"{mode} JUnit duration is invalid: {test_id}",
            )
            outcome, error = _outcome(case)
            counts[outcome] += 1
            mapped[test_id] = {
                "outcome": outcome,
                "durationSeconds": duration,
                "error": error,
            }
        require(
            set(mapped) == set(targets),
            f"{mode} JUnit does not cover exact 80 target IDs",
        )
        require(dict(counts) == pins["outcomes"], f"{mode} JUnit outcome counts differ")
        require(path.read_bytes() == raw, f"{mode} JUnit changed during parsing")
        verified[mode] = {
            "payload": payload,
            "cases": mapped,
            "counts": dict(sorted(counts.items())),
        }
    require(
        verified["candidate"]["payload"]["pwrightArtifactSha256"]
        == "f4afce01a1a0efca9851200444ca015f373e5986d16920ac3bfbdbd9534021b9"
        and verified["oracle"]["payload"]["pwrightArtifactSha256"] is None,
        "candidate/oracle runtime identity differs",
    )
    return verified


def successor(
    mode: str,
    original: tuple[list[bytes], list[dict[str, Any]]],
    targets: list[str],
    report: dict[str, Any],
    output: Path,
) -> tuple[bytes, bytes, dict[str, Any]]:
    lines, rows = original
    require(
        mode in MODES and len(lines) == len(rows) == RECORDS,
        "successor baseline differs",
    )
    updated = list(lines)
    for index, test_id in zip(TARGET_INDEXES, targets):
        require(rows[index]["id"] == test_id, f"{mode} target moved at {index}")
        result = report["cases"][test_id]
        replacement = {
            key: value for key, value in rows[index].items() if key not in GENERATED
        }
        replacement.update(
            attempts=1,
            durationSeconds=result["durationSeconds"],
            outcome=result["outcome"],
            resultFile=str(report_path(mode)),
        )
        if result["error"] is not None:
            replacement["error"] = result["error"]
        updated[index] = (
            json.dumps(replacement, sort_keys=True, separators=(",", ":")) + "\n"
        ).encode()
    target_set = set(TARGET_INDEXES)
    require(
        all(
            updated[index] == line
            for index, line in enumerate(lines)
            if index not in target_set
        ),
        f"one of the other 6,446 {mode} row bytes changed",
    )
    ledger = b"".join(updated)
    resulting_rows = [
        json_object(line, f"{mode} successor row {index}")
        for index, line in enumerate(updated)
    ]
    summary = {
        "schema": 1,
        "inventory": str(INVENTORY),
        "records": RECORDS,
        "outcomes": dict(
            sorted(Counter(row["outcome"] for row in resulting_rows).items())
        ),
        "ledgerSha256": sha256(ledger),
        "ledger": str(output),
    }
    return ledger, (json.dumps(summary, indent=2) + "\n").encode(), summary


def prepare(output_dir: Path) -> tuple[dict[str, bytes], dict[str, Any]]:
    require(
        output_dir == OUTPUT_DIR and output_dir.is_absolute(), "successor path differs"
    )
    source_inputs()
    originals, targets = base()
    evidence = reports(targets)
    content: dict[str, bytes] = {}
    summaries: dict[str, dict[str, Any]] = {}
    for mode in MODES:
        name = output_name(mode)
        ledger, summary_bytes, summary = successor(
            mode, originals[mode], targets, evidence[mode], output_dir / name
        )
        content[name] = ledger
        content[Path(name).with_suffix(".summary.json").name] = summary_bytes
        summaries[mode] = summary
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
        "sourcePins": str(SOURCE_PINS),
        "sourcePinsSha256": SOURCE_PINS_SHA256,
        "frozenWheel": str(WHEEL),
        "frozenWheelSha256": WHEEL_SHA256,
        "promoterSha256": sha256(Path(__file__).read_bytes()),
        "basePromoterSha256": BASE_PROMOTER_SHA256,
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
            for mode in MODES
        },
        "reports": {
            mode: {
                "provenance": str(report_path(mode).with_suffix(".provenance.json")),
                "provenanceSha256": REPORT_PINS[mode]["provenance"],
                "result": str(report_path(mode)),
                "resultSha256": REPORT_PINS[mode]["result"],
                "observedOutcomes": evidence[mode]["counts"],
                "isolatedCaseIdsSha256": ISOLATED_CASE_IDS_SHA256,
                "caseReportsSha256": REPORT_PINS[mode]["caseReports"],
            }
            for mode in MODES
        },
        "successors": summaries,
    }
    content["promotion-receipt.json"] = (
        json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    ).encode()
    return content, receipt


def _check_output(output_dir: Path) -> None:
    require(output_dir == OUTPUT_DIR, "successor path differs")
    prior.prior.prior._check_output(output_dir)


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
        prior.prior.prior._fsync_directory(staging)
        _check_output(output_dir)
        require(
            prepare(output_dir) == (content, receipt),
            "strict evidence changed during staging",
        )
        prior.prior.prior._publish_exclusive(staging, output_dir)
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
