"""Publish the source-bound Python Chromium network50 strict paired v8 ledger.

Both candidate and Oracle replace exactly 50 official rows. Each lane's other
6,476 raw rows are kept byte-for-byte from v7; every RED remains RED.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import tempfile
from pathlib import Path
from typing import Any

import promote_playwright_python_dummy_node_strict_pair as atomic
import verify_playwright_python_network50_strict_paired_v8 as verify

OUTPUT_DIR = verify.OUTPUT_DIR


class PromotionError(ValueError):
    """The frozen paired network evidence cannot be promoted."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise PromotionError(message)


def successor(
    mode: str,
    base_lines: list[bytes],
    inventory: list[dict[str, Any]],
    cases: dict[str, dict[str, object]],
    output: Path,
) -> tuple[bytes, bytes, dict[str, Any]]:
    require(mode in verify.MODES, "unknown ledger lane")
    updated = list(base_lines)
    for index in verify.TARGET_INDEXES:
        source = inventory[index]
        case = cases[source["id"]]
        replacement: dict[str, object] = {
            **source,
            "attempts": 1,
            "durationSeconds": case["durationSeconds"],
            "outcome": case["outcome"],
            "resultFile": str(verify.REPORTS[mode]),
        }
        if "error" in case:
            replacement["error"] = case["error"]
        updated[index] = (
            json.dumps(replacement, sort_keys=True, separators=(",", ":")) + "\n"
        ).encode()
    counts = verify.check_reconciliation(base_lines, updated, inventory, cases, mode)
    ledger = b"".join(updated)
    summary = {
        "schema": 1,
        "inventory": str(verify.INVENTORY),
        "records": verify.RECORDS,
        "outcomes": dict(sorted(counts.items())),
        "ledgerSha256": verify.sha256(ledger),
        "ledger": str(output),
    }
    return ledger, (json.dumps(summary, indent=2) + "\n").encode(), summary


def prepare(output_dir: Path) -> tuple[dict[str, bytes], dict[str, Any]]:
    require(
        output_dir == OUTPUT_DIR and output_dir.is_absolute(), "successor path differs"
    )
    _, inventory, base_lines, provenance, cases, comparison = verify.evidence()
    content: dict[str, bytes] = {}
    summaries: dict[str, dict[str, Any]] = {}
    for mode in verify.MODES:
        name = verify.output_name(mode)
        ledger, summary_raw, summary = successor(
            mode, base_lines[mode], inventory, cases[mode], output_dir / name
        )
        content[name] = ledger
        content[Path(name).with_suffix(".summary.json").name] = summary_raw
        summaries[mode] = summary
    pins = (
        json.dumps(verify.source_pins_expected(provenance), indent=2, sort_keys=True)
        + "\n"
    ).encode()
    content["source-pins.json"] = pins
    receipt = {
        "schema": 2,
        "kind": verify.KIND,
        "scope": verify.SCOPE,
        "publicApiParityEvidence": False,
        "fullCorpusExecution": False,
        "completeFileExecution": False,
        "completeManifestExecution": True,
        "fixtureAdapted": True,
        "launchMode": "cdp-adapter",
        "sharedFailedCases": len(comparison["shared"]),
        "candidateOnlyFailedCases": len(comparison["candidateOnly"]),
        "oracleOnlyFailedCases": len(comparison["oracleOnly"]),
        "crossLaneComparison": comparison,
        "inventory": str(verify.INVENTORY),
        "inventorySha256": verify.INVENTORY_SHA256,
        "recordsPerLane": verify.RECORDS,
        "targetZeroBasedIndexes": list(verify.TARGET_INDEXES),
        "targetTestIds": [inventory[index]["id"] for index in verify.TARGET_INDEXES],
        "manifest": str(verify.MANIFEST),
        "manifestSha256": verify.MANIFEST_SHA256,
        "upstream": str(verify.UPSTREAM),
        "upstreamCommit": verify.COMMIT,
        "upstreamNetworkSha256": verify.NETWORK_SHA256,
        "sourcePins": str(output_dir / "source-pins.json"),
        "sourcePinsSha256": verify.sha256(pins),
        "promoterSha256": verify.sha256(Path(__file__).read_bytes()),
        "basePromoterSha256": verify.BASE_PROMOTER_SHA256,
        "baseVerifierSha256": verify.BASE_VERIFIER_SHA256,
        "baseReceipt": str(verify.BASE_DIR / "promotion-receipt.json"),
        "baseReceiptSha256": verify.BASE_RECEIPT_SHA256,
        "candidateFrozenWheel": str(verify.abi130.WHEEL),
        "candidateFrozenWheelSha256": verify.abi130.WHEEL_SHA256,
        "bases": {
            mode: {
                "ledger": str(verify.BASE_DIR / verify.base_name(mode)),
                "ledgerSha256": verify.BASE_HASHES[mode][0],
                "summary": str(
                    (verify.BASE_DIR / verify.base_name(mode)).with_suffix(
                        ".summary.json"
                    )
                ),
                "summarySha256": verify.BASE_HASHES[mode][1],
            }
            for mode in verify.MODES
        },
        "reports": {
            mode: {
                "provenance": str(verify.REPORTS[mode].with_suffix(".provenance.json")),
                "provenanceSha256": verify.PROVENANCE_SHA256[mode],
                "result": str(verify.REPORTS[mode]),
                "resultSha256": verify.REPORT_SHA256[mode],
                "caseReportsSha256": verify.CASE_REPORTS_SHA256[mode],
                "isolatedCaseIdsSha256": verify.ISOLATED_IDS_SHA256,
                "observedOutcomes": verify.EXPECTED_REPORT_COUNTS[mode],
            }
            for mode in verify.MODES
        },
        "successors": summaries,
    }
    content["promotion-receipt.json"] = (
        json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    ).encode()
    return content, receipt


def publish(output_dir: Path) -> dict[str, Any]:
    require(output_dir == OUTPUT_DIR, "successor path differs")
    atomic._check_output(output_dir)
    content, receipt = prepare(output_dir)
    require(
        verify.SOURCE_PINS_SHA256 == verify.sha256(content["source-pins.json"]),
        "v8 source pins are not sealed",
    )
    require(
        verify.PROMOTER_SHA256 == verify.sha256(Path(__file__).read_bytes()),
        "v8 publisher source is not sealed",
    )
    staging = Path(
        tempfile.mkdtemp(prefix=f".{output_dir.name}.staging-", dir=output_dir.parent)
    )
    try:
        for name, raw in content.items():
            with (staging / name).open("xb") as stream:
                stream.write(raw)
                stream.flush()
                os.fsync(stream.fileno())
        atomic._fsync_directory(staging)
        atomic._check_output(output_dir)
        require(
            prepare(output_dir) == (content, receipt), "evidence changed during staging"
        )
        atomic._publish_exclusive(staging, output_dir)
    finally:
        if staging.exists():
            shutil.rmtree(staging)
    return receipt


def verify_existing(output_dir: Path) -> dict[str, Any]:
    receipt = verify.verify_bundle(output_dir)
    content, prepared = prepare(output_dir)
    require(receipt == prepared, "v8 receipt differs from publisher replay")
    require(
        {path.name for path in output_dir.iterdir()} == set(content),
        "v8 file set differs",
    )
    for name, raw in content.items():
        require(
            (output_dir / name).read_bytes() == raw,
            f"v8 published bytes differ: {name}",
        )
    return receipt


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify-existing", action="store_true")
    args = parser.parse_args()
    try:
        receipt = (
            verify_existing(OUTPUT_DIR) if args.verify_existing else publish(OUTPUT_DIR)
        )
    except (
        OSError,
        PromotionError,
        verify.Network50ValidationError,
        verify.prior.FullNavigationV7ValidationError,
        atomic.PromotionError,
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
