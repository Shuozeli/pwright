"""Publish the source-bound six-case Python sync network v10 paired ledger.

This publication archives only the observed candidate Python package/native
artifact, its adjacent runtime dylibs, and the small official shim/runtime.
The first candidate run remains pinned as infrastructure-invalid evidence.
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
import verify_playwright_python_sync_network6_strict_paired_v10 as verify

OUTPUT_DIR = verify.OUTPUT_DIR


class PromotionError(ValueError):
    """The six-case evidence cannot be atomically promoted."""


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
            "resultFile": str(verify.REPORTS[f"{mode}Qualified"]),
        }
        if "error" in case:
            replacement["error"] = case["error"]
        updated[index] = (
            json.dumps(replacement, sort_keys=True, separators=(",", ":")) + "\n"
        ).encode()
    counts = verify.check_reconciliation(
        base_lines, updated, inventory, cases, mode
    )
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
    require(output_dir == OUTPUT_DIR and output_dir.is_absolute(), "successor path differs")
    require(
        verify.archive_fingerprint() == verify.ARCHIVE_FINGERPRINT,
        "live candidate archive sources differ from the sealed file list",
    )
    inventory, base_lines = verify._base()
    payloads, cases = verify._reports()
    classification = verify.paired_classification(cases)
    content: dict[str, bytes] = {}
    summaries: dict[str, dict[str, Any]] = {}
    for mode in verify.MODES:
        name = verify.output_name(mode)
        ledger, summary_raw, summary = successor(
            mode,
            base_lines[mode],
            inventory,
            cases[f"{mode}Qualified"],
            output_dir / name,
        )
        content[name] = ledger
        content[Path(name).with_suffix(".summary.json").name] = summary_raw
        summaries[mode] = summary
    pins = (
        json.dumps(verify.source_pins_expected(payloads), indent=2, sort_keys=True)
        + "\n"
    ).encode()
    content["source-pins.json"] = pins
    receipt = verify.receipt_expected(summaries, classification, verify.sha256(pins))
    content["promotion-receipt.json"] = (
        json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    ).encode()
    return content, receipt


def _archive(staging: Path) -> None:
    candidate_package = verify.ROOT / "packages/pwright-python/python/pwright"
    candidate_libraries = verify.ROOT / "packages/pwright-python/python/shuozeli_pwright.dylibs"
    official_harness = verify.ROOT / "packages/pwright-python/tests/official"
    archive = staging / "archive"
    archive.mkdir(mode=0o755)
    shutil.copytree(candidate_package, archive / "pwright", symlinks=True)
    shutil.copytree(candidate_libraries, archive / "shuozeli_pwright.dylibs", symlinks=True)
    (archive / "harness").mkdir(mode=0o755)
    for name in ("shim", "runtime"):
        shutil.copytree(official_harness / name, archive / "harness" / name, symlinks=True)
    require(
        verify.archive_fingerprint(archive) == verify.ARCHIVE_FINGERPRINT,
        "copied archive file list differs from the sealed source list",
    )
    payloads, _ = verify._reports(archive)
    require(
        payloads["candidateQualified"]["pwrightPackageSha256"]
        == "ae49b06f267c68194edfe268a3f1ca19debbd9bb8895286acd9871f789dfa482",
        "archived candidate package differs",
    )
    for path in sorted(archive.rglob("*"), reverse=True):
        if path.is_file():
            require(not path.is_symlink(), "archive contains a symlink")
            with path.open("rb") as stream:
                os.fsync(stream.fileno())
        elif path.is_dir():
            require(not path.is_symlink(), "archive contains a symlink directory")
            atomic._fsync_directory(path)
    atomic._fsync_directory(archive)


def publish(output_dir: Path) -> dict[str, Any]:
    require(output_dir == OUTPUT_DIR, "successor path differs")
    atomic._check_output(output_dir)
    content, receipt = prepare(output_dir)
    require(
        verify.SOURCE_PINS_SHA256 == verify.sha256(content["source-pins.json"]),
        "v10 source pins are not sealed",
    )
    require(
        verify.PROMOTER_SHA256 == verify.sha256(Path(__file__).read_bytes()),
        "v10 publisher source is not sealed",
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
        _archive(staging)
        atomic._fsync_directory(staging)
        atomic._check_output(output_dir)
        require(prepare(output_dir) == (content, receipt), "evidence changed during staging")
        atomic._publish_exclusive(staging, output_dir)
    finally:
        if staging.exists():
            shutil.rmtree(staging)
    return receipt


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify-existing", action="store_true")
    args = parser.parse_args()
    try:
        receipt = (
            verify.verify_bundle(OUTPUT_DIR)
            if args.verify_existing
            else publish(OUTPUT_DIR)
        )
    except (
        OSError,
        PromotionError,
        verify.SyncNetwork6ValidationError,
        verify.prior.NavigationRecheckValidationError,
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
