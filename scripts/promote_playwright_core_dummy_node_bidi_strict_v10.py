"""Plan or exclusively publish six Core BiDi dummy-node official rerun rows.

The v9 ledger (including its historical candidate RED) and twelve fully
verified v3 sidecars are immutable inputs. --plan is read-only. --publish is
the only publishing action and cannot replace an existing successor.
"""

from __future__ import annotations

import argparse
import ctypes
import hashlib
import json
import os
import shutil
import sys
import tempfile
from collections import Counter
from pathlib import Path
from typing import Any

import verify_playwright_core_dummy_node_bidi_paired_v10 as independent

V9 = independent.V9
V10 = independent.V10
OUTCOMES = independent.OUTCOMES
INVENTORY = independent.INVENTORY
LANES = independent.LANES
TARGETS = independent.TARGETS
COUNT = independent.COUNT
SCOPE = independent.SCOPE


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical_line(record: dict[str, Any]) -> bytes:
    return (json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n").encode()


def transform_lane(
    lane: str,
    summary: dict[str, Any],
    prior: dict[str, Any],
    evidence: dict[tuple[str, str], dict[str, Any]],
) -> tuple[bytes, bytes, dict[str, Any]]:
    """Copy all 48,471 other raw lines and render six strict rerun rows."""
    require(lane in LANES, "unknown lane")
    old_lines = independent.sealed(
        V9 / f"{lane}.jsonl",
        independent.V9_LEDGER_SHA256[lane],
        f"v9 {lane} ledger during plan",
    ).splitlines(keepends=True)
    require(len(old_lines) == COUNT, f"v9 {lane} count differs")
    old_outcomes: Counter[str] = Counter()
    old_scopes: Counter[str] = Counter()
    changed: list[dict[str, Any]] = []
    output = bytearray()
    for index, old_raw in enumerate(old_lines):
        old = json.loads(old_raw)
        old_outcomes[old["outcome"]] += 1
        if "evidenceScope" in old:
            old_scopes[old["evidenceScope"]] += 1
        if index not in TARGETS:
            output.extend(old_raw)
            continue
        project, engine, spec_id = TARGETS[index]
        strict = evidence[(lane, project)]
        previous_result = prior["strictEvidence"][project]["officialResult"]
        old_status = "unexpected" if lane == "candidate" else "expected"
        require(
            old
            == {
                **independent.target_source(project, spec_id),
                "attempts": 1,
                "evidenceScope": independent.V9_SCOPE,
                "outcome": old_status,
                "publicApiParityEvidence": False,
                "resultFile": previous_result,
            }
            and strict["mode"] == lane
            and strict["project"] == project
            and strict["engine"] == engine
            and strict["index"] == index
            and strict["specId"] == spec_id
            and strict["testStatus"] == "expected",
            f"v9 {lane} target or v3 sidecar differs at {index}",
        )
        promoted = {
            **old,
            "attempts": 2,
            "evidenceScope": SCOPE,
            "outcome": "expected",
            "resultFile": strict["result"],
        }
        new_raw = canonical_line(promoted)
        changed.append(
            {
                "index": index,
                "specId": spec_id,
                "oldRowSha256": digest(old_raw),
                "newRowSha256": digest(new_raw),
                "oldOutcome": old_status,
                "newOutcome": "expected",
            }
        )
        output.extend(new_raw)
    require(
        len(changed) == 6
        and dict(old_outcomes) == summary["outcomes"]
        and dict(old_scopes) == summary["explicitEvidenceScopes"]
        and old_scopes[independent.V9_SCOPE] == 6
        and old_scopes[SCOPE] == 0,
        f"v9 {lane} summary or target count differs",
    )
    outcomes = old_outcomes.copy()
    outcomes["unexpected" if lane == "candidate" else "expected"] -= 6
    outcomes["expected"] += 6
    scopes = old_scopes.copy()
    scopes[independent.V9_SCOPE] -= 6
    scopes[SCOPE] += 6
    ledger = bytes(output)
    summary_object = {
        "schema": 1,
        "inventory": str(INVENTORY),
        "records": COUNT,
        "outcomes": dict(sorted(outcomes.items())),
        "explicitEvidenceScopes": dict(
            sorted((key, count) for key, count in scopes.items() if count)
        ),
        "ledgerSha256": digest(ledger),
        "ledger": str(V10 / f"{lane}.jsonl"),
    }
    summary_bytes = (json.dumps(summary_object, indent=2) + "\n").encode()
    strict_evidence = {}
    for project, _, _ in TARGETS.values():
        strict = evidence[(lane, project)]
        strict_evidence[project] = {
            "strictSidecar": str(independent.sidecar_path(lane, project)),
            "strictSidecarSha256": independent.SIDECAR_SHA256[(lane, project)],
            "runId": strict["runId"],
            "officialResult": strict["result"],
            "officialResultSha256": strict["resultSha256"],
            "launchReceiptSha256": strict["launchReceiptSha256"],
            "profileReceiptSha256": strict["profileReceiptSha256"],
            "processReceiptSha256": strict["processReceiptSha256"],
            "logSha256": strict["logSha256"],
            "testStatus": strict["testStatus"],
            "attemptStatus": strict["attemptStatus"],
            "dummyProcessExitObserved": strict["dummyProcessExitObserved"],
            "candidateSourceSha256": strict["candidateSourceSha256"],
            "candidateArtifactSha256": strict["candidateArtifactSha256"],
        }
    lane_receipt = {
        "base": str(V9 / f"{lane}.jsonl"),
        "baseSha256": independent.V9_LEDGER_SHA256[lane],
        "baseSummarySha256": independent.V9_SUMMARY_SHA256[lane],
        "ledger": str(V10 / f"{lane}.jsonl"),
        "ledgerSha256": digest(ledger),
        "summarySha256": digest(summary_bytes),
        "oldOutcomeByProject": {
            project: "unexpected" if lane == "candidate" else "expected"
            for project, _, _ in TARGETS.values()
        },
        "newOutcomesByProject": {
            project: evidence[(lane, project)]["testStatus"]
            for project, _, _ in TARGETS.values()
        },
        "changedRows": changed,
        "promotedRows": 6,
        "unchangedRawRows": COUNT - 6,
        "outcomes": summary_object["outcomes"],
        "priorStrictEvidence": prior["strictEvidence"],
        "strictEvidence": strict_evidence,
    }
    return ledger, summary_bytes, lane_receipt


def prepare_pair(
    checked_inputs: tuple[
        dict[str, Any], dict[str, dict[str, Any]], dict[tuple[str, str], dict[str, Any]]
    ]
    | None = None,
) -> dict[str, bytes]:
    """Build and independently verify five unpublished successor files."""
    inputs = checked_inputs or independent.verify_inputs()
    prior_receipt, summaries, evidence = inputs
    prepared = {
        lane: transform_lane(
            lane, summaries[lane], prior_receipt["lanes"][lane], evidence
        )
        for lane in LANES
    }
    receipt = {
        "schema": 1,
        "scope": SCOPE,
        "promotionKind": "public-api-candidate-official-rerun-outcomes",
        "sourceClassificationPreserved": "public-api-candidate",
        "publicApiParityEvidence": False,
        "browserExecutionEvidence": False,
        "bidiProtocolParityEvidence": False,
        "fixtureAdapted": True,
        "upstreamAssertionsModified": False,
        "selectedExecutable": "pinned-exit-1-node-asset",
        "promoterSourceSha256": digest(Path(__file__).read_bytes()),
        "pairedVerifierSha256": digest(Path(independent.__file__).read_bytes()),
        "strictCaptureSha256": independent.STRICT_CAPTURE_SHA256,
        "strictVerifierSha256": independent.STRICT_VERIFIER_SHA256,
        "strictManifestSha256": independent.MANIFEST_SHA256,
        "priorVerifierSha256": independent.V9_VERIFIER_SHA256,
        "priorBundle": str(V9),
        "priorBundleReceiptSha256": independent.V9_RECEIPT_SHA256,
        "inventory": str(INVENTORY),
        "inventorySha256": independent.INVENTORY_SHA256,
        "recordsPerLane": COUNT,
        "changedRowsPerLane": 6,
        "unchangedRawRowsPerLane": COUNT - 6,
        "targetZeroBasedIndexes": list(TARGETS),
        "targetSpecIds": [case[2] for case in TARGETS.values()],
        "upstreamCommit": independent.COMMIT,
        "selectedUpstreamSpecSha256": independent.SOURCE_SHA256,
        "candidateGreenRows": 6,
        "candidateRedRows": 0,
        "oracleGreenRows": 6,
        "lanes": {lane: item[2] for lane, item in prepared.items()},
    }
    files = {f"{lane}.jsonl": item[0] for lane, item in prepared.items()}
    files.update({f"{lane}.summary.json": item[1] for lane, item in prepared.items()})
    files["promotion-receipt.json"] = (
        json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    ).encode()
    independent.verify_payloads(files, checked_inputs=inputs)
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
    require(
        not V10.exists() and not V10.is_symlink(),
        "immutable v10 successor already exists",
    )
    OUTCOMES.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix=".core-dummy-bidi-v10-", dir=OUTCOMES))
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
        _publish_exclusive(staging, V10)
        parent_fd = os.open(OUTCOMES, os.O_RDONLY)
        try:
            os.fsync(parent_fd)
        finally:
            os.close(parent_fd)
    finally:
        if staging.exists():
            shutil.rmtree(staging)


def plan_summary(files: dict[str, bytes]) -> dict[str, Any]:
    receipt = json.loads(files["promotion-receipt.json"])
    return {
        "scope": receipt["scope"],
        "destination": str(V10),
        "published": False,
        "candidateGreenRows": receipt["candidateGreenRows"],
        "candidateRedRows": receipt["candidateRedRows"],
        "oracleGreenRows": receipt["oracleGreenRows"],
        "unchangedRawRowsPerLane": receipt["unchangedRawRowsPerLane"],
        "outcomes": {lane: receipt["lanes"][lane]["outcomes"] for lane in LANES},
        "sha256": {name: digest(payload) for name, payload in sorted(files.items())},
        "bytes": {name: len(payload) for name, payload in sorted(files.items())},
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument(
        "--plan", action="store_true", help="verify all evidence and print draft hashes"
    )
    action.add_argument(
        "--verify-existing", action="store_true", help="replay a published successor"
    )
    action.add_argument(
        "--publish", action="store_true", help="atomically publish the verified pair"
    )
    args = parser.parse_args()
    try:
        if args.verify_existing:
            receipt = independent.verify_existing()
            result = {
                "scope": receipt["scope"],
                "destination": str(V10),
                "published": True,
                "outcomes": {
                    lane: receipt["lanes"][lane]["outcomes"] for lane in LANES
                },
            }
        else:
            require(
                not V10.exists() and not V10.is_symlink(),
                "immutable v10 successor already exists",
            )
            files = prepare_pair()
            result = plan_summary(files)
            if args.publish:
                require(files == prepare_pair(), "evidence changed before publication")
                publish(files)
                independent.verify_existing()
                result["published"] = True
    except (
        AssertionError,
        OSError,
        ValueError,
        KeyError,
        TypeError,
        json.JSONDecodeError,
    ) as error:
        parser.exit(2, f"strict paired Core v10 promotion rejected: {error}\n")
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
