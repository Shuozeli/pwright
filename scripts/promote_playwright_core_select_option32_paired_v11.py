"""Plan or atomically publish the strict 32-case Core select-option successor.

The v10 pair and the two pinned official browser-backed reports are immutable
inputs. The v11 pair remains historically RED: one candidate result failed.
"""

from __future__ import annotations

import argparse
import ctypes
import json
import os
import shutil
import sys
import tempfile
from collections import Counter
from pathlib import Path
from typing import Any

import verify_playwright_core_select_option32_paired_v11 as independent


def transform_lane(
    lane: str, inputs: dict[str, Any]
) -> tuple[bytes, bytes, dict[str, Any]]:
    independent.require(lane in independent.LANES, "unknown lane")
    old_lines: list[bytes] = inputs["oldLines"][lane]
    targets: dict[int, dict[str, Any]] = inputs["targets"]
    report: dict[str, dict[str, Any]] = inputs["reports"][lane]
    old_outcomes: Counter[str] = Counter()
    old_scopes: Counter[str] = Counter()
    new_outcomes: Counter[str] = Counter()
    new_scopes: Counter[str] = Counter()
    changes: list[dict[str, Any]] = []
    output = bytearray()
    for index, old_raw in enumerate(old_lines):
        old = json.loads(old_raw)
        old_outcomes[old["outcome"]] += 1
        if "evidenceScope" in old:
            old_scopes[old["evidenceScope"]] += 1
        if index in targets:
            spec_id = old["specId"]
            new = independent.expected_row(old, report[spec_id], lane)
            new_raw = independent.canonical_line(new)
            changes.append(
                {
                    "index": index,
                    "specId": spec_id,
                    "oldRowSha256": independent.digest(old_raw),
                    "newRowSha256": independent.digest(new_raw),
                    "oldOutcome": "not-run",
                    "newOutcome": report[spec_id]["outcome"],
                }
            )
        else:
            new_raw = old_raw
            new = old
        output.extend(new_raw)
        new_outcomes[new["outcome"]] += 1
        if "evidenceScope" in new:
            new_scopes[new["evidenceScope"]] += 1
    independent.require(
        len(changes) == independent.TARGET_COUNT
        and dict(old_outcomes) == inputs["summaries"][lane]["outcomes"]
        and dict(old_scopes) == inputs["summaries"][lane]["explicitEvidenceScopes"],
        f"v10 {lane} summary or target count differs",
    )
    ledger = bytes(output)
    summary = {
        "schema": 1,
        "inventory": str(independent.INVENTORY),
        "records": independent.COUNT,
        "outcomes": dict(sorted(new_outcomes.items())),
        "explicitEvidenceScopes": dict(sorted(new_scopes.items())),
        "ledgerSha256": independent.digest(ledger),
        "ledger": str(independent.V11 / f"{lane}.jsonl"),
    }
    summary_bytes = (json.dumps(summary, indent=2) + "\n").encode()
    lane_receipt = {
        "base": str(independent.V10 / f"{lane}.jsonl"),
        "baseSha256": independent.V10_SHA256[f"{lane}.jsonl"],
        "baseSummarySha256": independent.V10_SHA256[f"{lane}.summary.json"],
        "ledger": str(independent.V11 / f"{lane}.jsonl"),
        "ledgerSha256": independent.digest(ledger),
        "summarySha256": independent.digest(summary_bytes),
        "officialResult": str(independent.report_path(lane)),
        "officialResultSha256": independent.REPORT_SHA256[lane],
        "provenance": str(independent.provenance_path(lane)),
        "provenanceSha256": independent.PROVENANCE_SHA256[lane],
        "changedRows": changes,
        "promotedRows": independent.TARGET_COUNT,
        "unchangedRawRows": independent.COUNT - independent.TARGET_COUNT,
        "outcomes": summary["outcomes"],
    }
    return ledger, summary_bytes, lane_receipt


def prepare_pair(inputs: dict[str, Any] | None = None) -> dict[str, bytes]:
    checked = inputs or independent.verify_inputs()
    prepared = {lane: transform_lane(lane, checked) for lane in independent.LANES}
    receipt = {
        "schema": 1,
        "scope": independent.SCOPE,
        "promotionKind": "browser-backed-public-api-full-file-outcomes",
        "priorBundle": str(independent.V10),
        "priorBundleReceiptSha256": independent.V10_SHA256["promotion-receipt.json"],
        "inventory": str(independent.INVENTORY),
        "inventorySha256": independent.INVENTORY_SHA256,
        "manifest": str(independent.MANIFEST),
        "manifestSha256": independent.MANIFEST_SHA256,
        "upstreamSourceSha256": independent.SOURCE_SHA256,
        "recordsPerLane": independent.COUNT,
        "changedRowsPerLane": independent.TARGET_COUNT,
        "unchangedRawRowsPerLane": independent.COUNT - independent.TARGET_COUNT,
        "targetZeroBasedIndexes": list(checked["targets"]),
        "fixtureAdapted": True,
        "browserExecutionEvidence": True,
        "publicApiParityEvidence": False,
        "upstreamAssertionsModified": False,
        "candidateExpectedRows": 31,
        "candidateUnexpectedRows": 1,
        "oracleExpectedRows": 32,
        "candidateRedSpecId": independent.RED_SPEC_ID,
        "candidateRedError": independent.RED_ERROR,
        "candidateProfile": independent.CANDIDATE_PROFILE,
        "candidateArtifactSha256": independent.CANDIDATE_ARTIFACT_SHA256,
        "candidateSourceSha256": independent.CANDIDATE_SOURCE_SHA256,
        "promoterSourceSha256": independent.digest(Path(__file__).read_bytes()),
        "verifierSourceSha256": independent.digest(
            Path(independent.__file__).read_bytes()
        ),
        "lanes": {lane: prepared[lane][2] for lane in independent.LANES},
    }
    files = {f"{lane}.jsonl": prepared[lane][0] for lane in independent.LANES}
    files.update(
        {f"{lane}.summary.json": prepared[lane][1] for lane in independent.LANES}
    )
    files["promotion-receipt.json"] = (
        json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    ).encode()
    independent.verify_payloads(files, inputs=checked)
    return files


def _publish_exclusive(staging: Path, destination: Path) -> None:
    libc = ctypes.CDLL(None, use_errno=True)
    if sys.platform == "darwin":
        rename = libc.renameatx_np
        flag, at_fdcwd = 0x4, -2
    elif sys.platform.startswith("linux"):
        rename = libc.renameat2
        flag, at_fdcwd = 0x1, -100
    else:
        raise ValueError("atomic no-replace directory publication is unavailable")
    rename.argtypes = [
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_uint,
    ]
    rename.restype = ctypes.c_int
    if (
        rename(at_fdcwd, os.fsencode(staging), at_fdcwd, os.fsencode(destination), flag)
        != 0
    ):
        raise ValueError(
            f"atomic no-replace publication failed: {os.strerror(ctypes.get_errno())}"
        )


def publish(files: dict[str, bytes]) -> None:
    independent.require(
        not independent.V11.exists() and not independent.V11.is_symlink(),
        "immutable v11 successor already exists",
    )
    independent.verify_payloads(files)
    independent.V11.parent.mkdir(parents=True, exist_ok=True)
    staging = Path(
        tempfile.mkdtemp(
            prefix=".core-select-option32-v11-", dir=independent.V11.parent
        )
    )
    try:
        for name, payload in files.items():
            with (staging / name).open("xb") as stream:
                stream.write(payload)
                stream.flush()
                os.fsync(stream.fileno())
        directory_fd = os.open(staging, os.O_RDONLY)
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)
        _publish_exclusive(staging, independent.V11)
        parent_fd = os.open(independent.V11.parent, os.O_RDONLY)
        try:
            os.fsync(parent_fd)
        finally:
            os.close(parent_fd)
    finally:
        if staging.exists():
            shutil.rmtree(staging)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--plan", action="store_true")
    action.add_argument("--publish", action="store_true")
    action.add_argument("--verify-existing", action="store_true")
    args = parser.parse_args()
    try:
        if args.verify_existing:
            receipt = independent.verify_existing()
            result = {
                "published": True,
                "scope": receipt["scope"],
                "destination": str(independent.V11),
            }
        else:
            independent.require(
                not independent.V11.exists() and not independent.V11.is_symlink(),
                "immutable v11 successor already exists",
            )
            files = prepare_pair()
            result = {
                "published": False,
                "scope": independent.SCOPE,
                "destination": str(independent.V11),
                "sha256": {
                    name: independent.digest(payload)
                    for name, payload in sorted(files.items())
                },
                "outcomes": {
                    lane: json.loads(files[f"{lane}.summary.json"])["outcomes"]
                    for lane in independent.LANES
                },
            }
            if args.publish:
                independent.require(
                    files == prepare_pair(), "evidence changed before publication"
                )
                publish(files)
                independent.verify_existing()
                result["published"] = True
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
        parser.exit(2, f"strict paired Core v11 promotion rejected: {error}\n")
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
