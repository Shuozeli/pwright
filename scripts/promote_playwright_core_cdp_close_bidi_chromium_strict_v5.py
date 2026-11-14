#!/usr/bin/env python3
"""Plan or atomically publish three Bidi-config CDP Close diagnostic rows."""

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

import run_playwright_core_cdp_close_bidi_chromium_strict_v4 as capture
import verify_playwright_core_cdp_close_bidi_chromium_paired_v5 as independent

ROOT = Path(__file__).resolve().parents[1]
OUTCOMES = ROOT / ".work/official-conformance/outcomes"
PREDECESSOR = OUTCOMES / "core-server-connect-ws-error-bidi-chromium-strict-paired-v4"
OUTPUT = OUTCOMES / "core-cdp-close-bidi-chromium-strict-paired-v5"
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
COUNT = 48_477
SCOPE = "official-browser-free-cdp-close-bidi-chromium-internal-diagnostic-green-v5"
TARGETS = {
    6527: (
        "8d42d2a6326b0b57ff3d-bd81835b26c6ab884876",
        "bidi-chrome-canary-library",
    ),
    6528: (
        "8d42d2a6326b0b57ff3d-abec76758cb8ccb82b80",
        "bidi-chrome-stable-library",
    ),
    6529: (
        "8d42d2a6326b0b57ff3d-da88fd98989a23696b8f",
        "bidi-chromium-library",
    ),
}
PREDECESSOR_RECEIPT_SHA256 = (
    "f7c72c003d4306ade21f077b8b424a60a21d265a72512c5b8ac249c21886d2c6"
)
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
PREDECESSOR_LEDGER_SHA256 = {
    "candidate": "c19aa1c129d20e25424bfbd3620255f0c6d4b16b0ea38a7284aca4161a9384cc",
    "oracle": "0a95ae35c25da64b74d5c6bc949c39dd1d033f6dca1d586d9cbe8994754978f5",
}
PREDECESSOR_SUMMARY_SHA256 = {
    "candidate": "9aad34b91d9e739c26563a9878657cc1df587ec3be806c45535a9230744cfba0",
    "oracle": "735bc01f5382124fa070208c2f2354a02dc4c6ec09fb7a5e2b4fb61bce0c7105",
}
STRICT_SHA256 = {
    "candidate": "08043d6464e21092671f5c55e923be4cd3d7fdf199c08a910532789282b4e048",
    "oracle": "5ebba48bb916863875e2c594322fdd7fa1af1b4b5688bff1abdbda3d08c87cab",
}
STRICT_VERIFIER_SHA256 = (
    "7b8dea4ce695d2b392ff445df486b512e84d20b7f684f65fc49d8f15178ebf06"
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def transform_lane(
    lane: str, strict: dict[str, Any]
) -> tuple[bytes, bytes, dict[str, Any]]:
    """Leave every non-target ledger line at its original byte representation."""
    require(lane in ("candidate", "oracle"), "unknown lane")
    baseline = PREDECESSOR / f"{lane}.jsonl"
    old_summary = json.loads((PREDECESSOR / f"{lane}.summary.json").read_bytes())
    result_by_id = {
        case["specId"]: run["result"]
        for run in strict["runs"]
        for case in run["selected"]
    }
    require(
        set(result_by_id) == {entry[0] for entry in TARGETS.values()},
        "strict selected IDs differ",
    )
    old_lines = baseline.read_bytes().splitlines(keepends=True)
    require(len(old_lines) == COUNT, "predecessor row count differs")
    changed: list[dict[str, Any]] = []
    output = bytearray()
    for index, old_raw in enumerate(old_lines):
        if index not in TARGETS:
            output.extend(old_raw)
            continue
        spec_id, project = TARGETS[index]
        old = json.loads(old_raw)
        require(
            old["specId"] == spec_id
            and old["projectName"] == old["projectId"] == project
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
        new_raw = (
            json.dumps(promoted, sort_keys=True, separators=(",", ":")) + "\n"
        ).encode()
        changed.append(
            {
                "index": index,
                "specId": spec_id,
                "oldRowSha256": digest(old_raw),
                "newRowSha256": digest(new_raw),
            }
        )
        output.extend(new_raw)
    require(len(changed) == 3, "target count differs")
    ledger = bytes(output)
    outcomes = dict(old_summary["outcomes"])
    outcomes["not-run"] -= 3
    outcomes["expected"] += 3
    scopes = dict(old_summary["explicitEvidenceScopes"])
    scopes[SCOPE] = 3
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
        "strictSidecar": str(capture.RESULT_PARENT / f"{lane}-v4.strict.json"),
        "strictSidecarSha256": STRICT_SHA256[lane],
        "officialRuns": [
            {
                "project": run["project"],
                "result": run["result"],
                "resultSha256": run["resultSha256"],
                "rawProvenance": str(capture.paths(lane, run["project"])["provenance"]),
                "rawProvenanceSha256": run["provenanceSha256"],
                "guardReceipt": str(capture.paths(lane, run["project"])["receipt"]),
                "guardReceiptSha256": run["guardReceiptSha256"],
            }
            for run in strict["runs"]
        ],
        "candidateArtifactSha256": strict["candidateArtifactSha256"],
        "candidateSnapshotManifestSha256": strict["candidateSnapshotManifestSha256"],
        "candidateSourceSha256": strict["candidateSourceSha256"],
        "ledger": str(destination),
        "ledgerSha256": summary["ledgerSha256"],
        "summarySha256": digest(summary_bytes),
        "oldOutcome": "not-run",
        "newOutcome": "expected",
        "promotedRows": 3,
        "changedRows": changed,
        "outcomes": summary["outcomes"],
    }
    return ledger, summary_bytes, evidence


def prepare_pair() -> dict[str, bytes]:
    predecessor, strict = independent.verify_inputs()
    require(
        predecessor["scope"]
        == (
            "official-browser-free-server-connect-ws-error-bidi-chromium-"
            "internal-diagnostic-green-v4"
        ),
        "predecessor scope differs",
    )
    prepared = {lane: transform_lane(lane, strict[lane]) for lane in strict}
    receipt = {
        "schema": 1,
        "scope": SCOPE,
        "promotionKind": "implementation-internal-diagnostic",
        "protocolUnderTest": "CDP-over-WebSocket",
        "bidiProjectNamesOnly": True,
        "bidiProtocolParityEvidence": False,
        "fixtureAdapted": True,
        "publicApiParityEvidence": False,
        "browserExecutionEvidence": False,
        "currentLivePackageEvidence": False,
        "sourceClassificationPreserved": "implementation-internal",
        "diagnosticOnly": True,
        "frozenCandidateSnapshotAttested": True,
        "selectedOfficialTestRequiresBrowser": False,
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
        "promotedRowsPerLane": 3,
        "targetZeroBasedIndexes": list(TARGETS),
        "targetSpecIds": [entry[0] for entry in TARGETS.values()],
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
        rename(at_fdcwd, os.fsencode(staging), at_fdcwd, os.fsencode(destination), flag)
        != 0
    ):
        raise ValueError(
            f"atomic no-replace publication failed: {os.strerror(ctypes.get_errno())}"
        )


def publish(files: dict[str, bytes]) -> None:
    require(not OUTPUT.exists() and not OUTPUT.is_symlink(), "output already exists")
    OUTCOMES.mkdir(parents=True, exist_ok=True)
    temporary = Path(tempfile.mkdtemp(prefix=".core-cdp-close-bidi-v5-", dir=OUTCOMES))
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
        parser.error(f"strict v5 Bidi CDP Close promotion rejected: {error}")


if __name__ == "__main__":
    main()
