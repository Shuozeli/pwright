#!/usr/bin/env python3
"""Plan or atomically publish six browserless Bidi custom-Close diagnostic rows."""

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

import run_playwright_core_server_connect_close_error_bidi_chromium_strict_v6 as capture
import verify_playwright_core_server_connect_close_error_bidi_chromium_paired_v8 as independent

ROOT = Path(__file__).resolve().parents[1]
OUTCOMES = ROOT / ".work/official-conformance/outcomes"
PREDECESSOR = OUTCOMES / "core-server-connect-http-401-bidi-chromium-strict-paired-v7"
OUTPUT = OUTCOMES / "core-server-connect-close-error-bidi-chromium-strict-paired-v8"
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
COUNT = 48_477
SCOPE = (
    "official-browser-free-server-connect-close-error-bidi-chromium-"
    "fixture-host-adapted-internal-diagnostic-green-v8"
)
PREDECESSOR_SCOPE = (
    "official-browser-free-server-connect-http-401-bidi-chromium-"
    "fixture-host-adapted-internal-diagnostic-green-v7"
)
TARGETS = {
    5360: "ee6df4b3a2217bd356a7-511a6ea814b661f15f2e",
    5361: "ee6df4b3a2217bd356a7-82f4b2b4a00d4a3c199b",
    5362: "ee6df4b3a2217bd356a7-2b2c19e6a3745a17907d",
    5363: "ee6df4b3a2217bd356a7-f0142f43a589fa23adf3",
    5364: "ee6df4b3a2217bd356a7-5977d0c5bef040f1bf03",
    5365: "ee6df4b3a2217bd356a7-9c774e5f960e4e3f4a21",
}
PREDECESSOR_RECEIPT_SHA256 = "ef2b8a1d16fd3758c784071e50777df0b35935e04b9c1a3eaa8bf3a60a339f9a"
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
PREDECESSOR_LEDGER_SHA256 = {
    "candidate": "6f8abbddeb28c1666afd6b7a28eaccfad2f4b611834fb39a2d9e09a125c4a2ab",
    "oracle": "d3cf5ba7b73d00fcc0e2d38ced08c482c1542a5927b8ea3b298a23fc58e50db1",
}
PREDECESSOR_SUMMARY_SHA256 = {
    "candidate": "a66b8aa2a2e723d097652b68c2e17c7fbe557219fc5e81b1eafe5b92b188c182",
    "oracle": "90a10cd49d2b675d4f086701f97fd24f6e9f26528e8cbc7c4518da1725478fb9",
}
STRICT_SHA256 = {
    "candidate": "1ace68f24f338a88430905101c5a10c1cd485b6e1338cb8fae0e4d34ffa1686c",
    "oracle": "5dc032eb40790c071b9bf29a30fc3fb6871334e01a279f2fd44a4e5db8c4dfbf",
}
STRICT_VERIFIER_SHA256 = "96fbf79b353d3f1d2f0a8bba49a837e0a86cb247f537e7fc24f2a615ea74191b"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def transform_lane(lane: str, strict: dict[str, Any]) -> tuple[bytes, bytes, dict[str, Any]]:
    """Replace only the six target lines, preserving every other raw byte."""
    require(lane in independent.LANES, "unknown lane")
    baseline = PREDECESSOR / f"{lane}.jsonl"
    old_summary = json.loads((PREDECESSOR / f"{lane}.summary.json").read_bytes())
    result_by_id = {
        case["specId"]: run["result"] for run in strict["runs"] for case in run["selected"]
    }
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
            and old["projectName"] == old["projectId"]
            and old["attempts"] == 0
            and old["outcome"] == "not-run"
            and old["sourceClassification"] == "implementation-internal"
            and old["line"] == 130
            and old["file"] == "library/browsertype-connect.spec.ts",
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
    require(SCOPE not in scopes, "v8 scope already exists")
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
    official_runs = []
    for run in strict["runs"]:
        paths = capture.paths(lane, run["project"])
        official_runs.append(
            {
                "project": run["project"],
                "result": run["result"],
                "resultSha256": run["resultSha256"],
                "rawProvenance": str(paths["provenance"]),
                "rawProvenanceSha256": run["provenanceSha256"],
                "guardReceipt": str(paths["receipt"]),
                "guardReceiptSha256": run["guardReceiptSha256"],
            }
        )
    evidence = {
        "base": str(baseline),
        "baseSha256": PREDECESSOR_LEDGER_SHA256[lane],
        "baseSummarySha256": PREDECESSOR_SUMMARY_SHA256[lane],
        "strictSidecar": str(capture.RESULT_PARENT / f"{lane}-v6.strict.json"),
        "strictSidecarSha256": STRICT_SHA256[lane],
        "officialRuns": official_runs,
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
    require(predecessor["scope"] == PREDECESSOR_SCOPE, "predecessor scope differs")
    prepared = {lane: transform_lane(lane, strict[lane]) for lane in independent.LANES}
    receipt = {
        "schema": 1,
        "scope": SCOPE,
        "promotionKind": "implementation-internal-diagnostic",
        "fixtureAdaptation": capture.FIXTURE_ADAPTATION,
        "fixtureAdapted": True,
        "receiptAttribution": "ordered-project-receipts-only-not-proven-per-spec",
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
    if (
        rename(
            at_fdcwd,
            os.fsencode(staging),
            at_fdcwd,
            os.fsencode(destination),
            flag,
        )
        != 0
    ):
        raise ValueError(f"atomic no-replace publication failed: {os.strerror(ctypes.get_errno())}")


def publish(files: dict[str, bytes]) -> None:
    require(not OUTPUT.exists() and not OUTPUT.is_symlink(), "output already exists")
    OUTCOMES.mkdir(parents=True, exist_ok=True)
    temporary = Path(tempfile.mkdtemp(prefix=".core-close-bidi-v8-", dir=OUTCOMES))
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
                require(
                    os.environ.get("PWRIGHT_OFFICIAL_V8_PUBLISH_APPROVED") == "1",
                    "v8 publication requires explicit approval for this run",
                )
                require(files == prepare_pair(), "paired inputs changed before publish")
                publish(files)
                verify_existing()
            print(json.dumps({"output": str(OUTPUT), "sha256": hashes}, indent=2))
    except (AssertionError, OSError, ValueError, KeyError, TypeError) as error:
        parser.error(f"strict v8 Bidi Close promotion rejected: {error}")


if __name__ == "__main__":
    main()
