#!/usr/bin/env python3
"""Plan or atomically publish five browserless Bidi-connect diagnostic rows."""

from __future__ import annotations

import argparse
import ctypes
import hashlib
import json
import os
import shutil
import sys
import tempfile
from pathlib import Path
from typing import Any

import run_playwright_core_server_connect_timeout_redirect_bidi_chromium_strict_v1 as capture
import verify_playwright_core_server_connect_timeout_redirect_bidi_chromium_paired_v6 as independent

ROOT = Path(__file__).resolve().parents[1]
OUTCOMES = ROOT / ".work/official-conformance/outcomes"
PREDECESSOR = OUTCOMES / "core-cdp-close-bidi-chromium-strict-paired-v5"
OUTPUT = OUTCOMES / "core-server-connect-timeout-redirect-bidi-chromium-strict-paired-v6"
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
COUNT = 48_477
SCOPE = (
    "official-browser-free-server-connect-timeout-redirect-"
    "bidi-chromium-internal-diagnostic-green-v6"
)
TARGETS = {
    5436: "ee6df4b3a2217bd356a7-5817a5ed954cfa532ace",
    5437: "ee6df4b3a2217bd356a7-5b07f2a5e757a16ed468",
    5448: "ee6df4b3a2217bd356a7-30b6e0f369ffebc5d854",
    5449: "ee6df4b3a2217bd356a7-50b361500ef280984f89",
    5956: "ee6df4b3a2217bd356a7-27eeb64a07b3673ac59b",
}
PREDECESSOR_RECEIPT_SHA256 = "13d7df7ccf58fb1838c5f523471a77fadd3b7ad92330d086f13c8d3ac64df068"
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
PREDECESSOR_LEDGER_SHA256 = {
    "candidate": "0358d4210f90ab8cc941bcec7ac5711cee196eca195e08f26460f775eb9bbc25",
    "oracle": "f82cda1e9aab54936a0343ffd2d4d13c5da90a4aa0922c0c0cd69453dd26b835",
}
PREDECESSOR_SUMMARY_SHA256 = {
    "candidate": "9abad396b758382e21980f845dffb61b4a8e589befc552917a5902ea8259c611",
    "oracle": "fce4940d9dda54184ca827aea2488bf9a3956d5cedda31c34de7bea3b80f74c9",
}
STRICT_SHA256 = {
    "candidate": "7da683bf33205b1b6aa52f2749b4084d340cab9b2bd2fd24531a260c1cb6a35b",
    "oracle": "9a385742a0894f12c585a7212d72d27ae1ebd67d19dcf65cf161dfa2c48fa4b1",
}
STRICT_VERIFIER_SHA256 = "4fa9b9d1cc356554df9046bb34af7c89e3b88cc0b560e1f9d30e4013a8205ad3"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def transform_lane(lane: str, strict: dict[str, Any]) -> tuple[bytes, bytes, dict[str, Any]]:
    """Rewrite only five specified rows, retaining all other raw line bytes."""
    require(lane in ("candidate", "oracle"), "unknown lane")
    baseline = PREDECESSOR / f"{lane}.jsonl"
    old_summary = json.loads((PREDECESSOR / f"{lane}.summary.json").read_bytes())
    run = strict["run"]
    result_by_id = {case["specId"]: run["result"] for case in run["selected"]}
    require(set(result_by_id) == set(TARGETS.values()), "strict selected IDs differ")
    old_lines = baseline.read_bytes().splitlines(keepends=True)
    require(len(old_lines) == COUNT, "predecessor row count differs")
    changed: list[dict[str, Any]] = []
    output = bytearray()
    for index, old_raw in enumerate(old_lines):
        if index not in TARGETS:
            output.extend(old_raw)
            continue
        spec_id = TARGETS[index]
        old = json.loads(old_raw)
        require(
            old["specId"] == spec_id
            and old["projectName"] == old["projectId"] == capture.PROJECT
            and old["attempts"] == 0
            and old["outcome"] == "not-run"
            and old["sourceClassification"] == "implementation-internal",
            f"{lane} target identity differs at {index}",
        )
        promoted = {
            **old,
            "attempts": 1,
            "candidateRuntimeRequired": lane == "candidate",
            "evidenceScope": SCOPE,
            "outcome": "expected",
            "resultFile": result_by_id[spec_id],
        }
        new_raw = (json.dumps(promoted, sort_keys=True, separators=(",", ":")) + "\n").encode()
        changed.append(
            {
                "index": index,
                "specId": spec_id,
                "oldRowSha256": digest(old_raw),
                "newRowSha256": digest(new_raw),
            }
        )
        output.extend(new_raw)
    require(len(changed) == len(TARGETS), "target count differs")
    ledger = bytes(output)
    outcomes = dict(old_summary["outcomes"])
    outcomes["not-run"] -= len(TARGETS)
    outcomes["expected"] += len(TARGETS)
    scopes = dict(old_summary["explicitEvidenceScopes"])
    require(SCOPE not in scopes, "v6 scope already exists")
    scopes[SCOPE] = len(TARGETS)
    destination = OUTPUT / f"{lane}.jsonl"
    summary = {
        "schema": 1,
        "inventory": str(INVENTORY),
        "records": COUNT,
        "outcomes": dict(sorted(outcomes.items())),
        "explicitEvidenceScopes": dict(sorted(scopes.items())),
        "ledgerSha256": digest(ledger),
        "ledger": str(destination),
    }
    summary_bytes = (json.dumps(summary, indent=2) + "\n").encode()
    evidence = {
        "base": str(baseline),
        "baseSha256": PREDECESSOR_LEDGER_SHA256[lane],
        "baseSummarySha256": PREDECESSOR_SUMMARY_SHA256[lane],
        "strictSidecar": str(capture.paths(lane)["strict"]),
        "strictSidecarSha256": STRICT_SHA256[lane],
        "officialRuns": [
            {
                "project": run["project"],
                "result": run["result"],
                "resultSha256": run["resultSha256"],
                "rawProvenance": str(capture.paths(lane)["provenance"]),
                "rawProvenanceSha256": run["provenanceSha256"],
                "guardReceipt": str(capture.paths(lane)["receipt"]),
                "guardReceiptSha256": run["guardReceiptSha256"],
            }
        ],
        "candidateArtifactSha256": strict["candidateArtifactSha256"],
        "candidateSnapshotManifestSha256": strict["candidateSnapshotManifestSha256"],
        "candidateSourceSha256": strict["candidateSourceSha256"],
        "ledger": str(destination),
        "ledgerSha256": summary["ledgerSha256"],
        "summarySha256": digest(summary_bytes),
        "oldOutcome": "not-run",
        "newOutcome": "expected",
        "promotedRows": len(TARGETS),
        "changedRows": changed,
        "outcomes": summary["outcomes"],
    }
    return ledger, summary_bytes, evidence


def prepare_pair() -> dict[str, bytes]:
    predecessor, strict = independent.verify_inputs()
    require(
        predecessor["scope"]
        == "official-browser-free-cdp-close-bidi-chromium-internal-diagnostic-green-v5",
        "predecessor scope differs",
    )
    prepared = {lane: transform_lane(lane, strict[lane]) for lane in strict}
    receipt = {
        "schema": 1,
        "scope": SCOPE,
        "promotionKind": "implementation-internal-diagnostic",
        "fixtureAdaptation": capture.FIXTURE_ADAPTATION,
        "fixtureAdapted": True,
        "publicApiParityEvidence": False,
        "browserExecutionEvidence": False,
        "bidiProjectNamesOnly": True,
        "bidiProtocolParityEvidence": False,
        "currentLivePackageEvidence": False,
        "sourceClassificationPreserved": "implementation-internal",
        "diagnosticOnly": True,
        "frozenCandidateSnapshotAttested": True,
        "selectedOfficialTestRequiresBrowser": False,
        "selectedOfficialTestStartsRemoteServer": False,
        "upstreamAssertionsModified": False,
        "captureSidecarPromotedFlagAtCapture": False,
        "promoterSourceSha256": digest(Path(__file__).read_bytes()),
        "strictVerifierSha256": STRICT_VERIFIER_SHA256,
        "pairedVerifierSha256": digest(Path(independent.__file__).read_bytes()),
        "promotionMutationTestSha256": digest(independent.MUTATION_TEST.read_bytes()),
        "harnessInputs": strict["candidate"]["inputs"],
        "upstreamInputs": capture.UPSTREAM_INPUTS,
        "priorBundle": str(PREDECESSOR),
        "priorBundleReceiptSha256": PREDECESSOR_RECEIPT_SHA256,
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "recordsPerLane": COUNT,
        "promotedRowsPerLane": len(TARGETS),
        "targetZeroBasedIndexes": list(TARGETS),
        "targetSpecIds": list(TARGETS.values()),
        "lanes": {lane: item[2] for lane, item in prepared.items()},
    }
    files = {f"{lane}.jsonl": item[0] for lane, item in prepared.items()}
    files.update({f"{lane}.summary.json": item[1] for lane, item in prepared.items()})
    files["promotion-receipt.json"] = (
        json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    ).encode()
    independent.verify_payloads(files, checked_inputs=(predecessor, strict))
    return files


def verify_existing() -> dict[str, Any]:
    return independent.verify_existing()


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
    if rename(at_fdcwd, os.fsencode(staging), at_fdcwd, os.fsencode(destination), flag) != 0:
        raise ValueError(f"atomic no-replace publication failed: {os.strerror(ctypes.get_errno())}")


def publish(files: dict[str, bytes]) -> None:
    require(not OUTPUT.exists() and not OUTPUT.is_symlink(), "output already exists")
    OUTCOMES.mkdir(parents=True, exist_ok=True)
    temporary = Path(tempfile.mkdtemp(prefix=".core-connect-bidi-v6-", dir=OUTCOMES))
    try:
        for name, payload in files.items():
            with (temporary / name).open("xb") as stream:
                stream.write(payload)
                stream.flush()
                os.fsync(stream.fileno())
        directory_fd = os.open(temporary, os.O_RDONLY)
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)
        _publish_exclusive(temporary, OUTPUT)
        parent_fd = os.open(OUTCOMES, os.O_RDONLY)
        try:
            os.fsync(parent_fd)
        finally:
            os.close(parent_fd)
    finally:
        if temporary.exists():
            shutil.rmtree(temporary)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--plan", action="store_true")
    action.add_argument("--publish", action="store_true")
    action.add_argument("--verify-existing", action="store_true")
    args = parser.parse_args()
    try:
        if args.verify_existing:
            receipt = verify_existing()
            print(json.dumps({"scope": receipt["scope"], "output": str(OUTPUT)}))
        else:
            files = prepare_pair()
            hashes = {name: digest(payload) for name, payload in files.items()}
            if args.publish:
                require(files == prepare_pair(), "paired inputs changed before publish")
                publish(files)
                verify_existing()
            print(json.dumps({"output": str(OUTPUT), "sha256": hashes}, indent=2))
    except (AssertionError, OSError, ValueError, KeyError, TypeError) as error:
        parser.error(f"strict v6 Bidi connect promotion rejected: {error}")


if __name__ == "__main__":
    main()
