#!/usr/bin/env python3
"""Read-only, byte-preserving hypothetical four-row Core ledger promotion."""

from __future__ import annotations

import hashlib
import json
import sys
from collections import Counter
from pathlib import Path
from typing import Any

import run_playwright_core_server_connect_ws_error_other_engines_strict_v3 as capture
from verify_playwright_core_server_connect_ws_error_other_engines_strict_v3 import (
    verify_pair,
)

PREDECESSOR = (
    capture.ROOT
    / ".work/official-conformance/outcomes/core-dummy-node-other-engines-strict-paired-v1"
)
PREDECESSOR_HASHES = {
    "candidate": "d0ad535d17c595414bca06f7f81c33515cf92b12d28fb025f915e9482e7fa8fb",
    "oracle": "c176a3e55da3556ff17c263b71044079ba5c467ca5925464ca03f514b8e41b56",
}
PREDECESSOR_RECEIPT_SHA256 = (
    "27131a3e31ff27d3a7dd676f97581c8fcdf09e19a76647d3ffc83d689edf15c5"
)
EXPECTED_LINES = {
    "ee6df4b3a2217bd356a7-7ff2dc18d1e08d985682": 31235,
    "ee6df4b3a2217bd356a7-95a86c47485eb83c0016": 31236,
    "ee6df4b3a2217bd356a7-184cfa77707c96da434b": 31237,
    "ee6df4b3a2217bd356a7-6d5f8a4e2638a54cc171": 31238,
}
EVIDENCE_SCOPE = "official-browser-free-server-connect-ws-error-internal-diagnostic-green-v3-other-engines"


def digest(path: Path) -> str:
    result = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            result.update(chunk)
    return result.hexdigest()


def virtual_diff(mode: str, strict: dict[str, Any]) -> dict[str, Any]:
    predecessor = PREDECESSOR / f"{mode}.jsonl"
    if digest(predecessor) != PREDECESSOR_HASHES[mode]:
        raise ValueError("canonical predecessor ledger digest changed")
    selected = {
        item["specId"]: (run, item)
        for run in strict["runs"]
        for item in run["selected"]
    }
    if set(selected) != set(EXPECTED_LINES) or any(
        item["status"] != "expected" for _, item in selected.values()
    ):
        raise ValueError("strict pair is not an exact four-case GREEN slice")
    old_hash = hashlib.sha256()
    virtual_hash = hashlib.sha256()
    before: Counter[str] = Counter()
    after: Counter[str] = Counter()
    changed: list[dict[str, Any]] = []
    rows = 0
    with predecessor.open("rb") as stream:
        for rows, raw in enumerate(stream, start=1):
            old_hash.update(raw)
            if not raw.endswith(b"\n"):
                raise ValueError("ledger row lacks a newline")
            record = json.loads(raw)
            spec_id = record["specId"]
            before[record["outcome"]] += 1
            if spec_id not in selected:
                virtual_hash.update(raw)
                after[record["outcome"]] += 1
                continue
            if rows != EXPECTED_LINES[spec_id]:
                raise ValueError("target ledger row moved")
            if (
                record["outcome"] != "not-run"
                or record["attempts"] != 0
                or record["suite"] != "library"
                or record["sourceFile"] != capture.SOURCE
                or record["line"] != 122
                or record["sourceClassification"] != "implementation-internal"
                or record["expectedStatus"] != "passed"
                or record["projectId"] != selected[spec_id][0]["project"]
            ):
                raise ValueError("target predecessor row has changed semantics")
            run, _ = selected[spec_id]
            proposal = dict(record)
            proposal.update(
                {
                    "attempts": 1,
                    "candidateRuntimeRequired": mode == "candidate",
                    "evidenceScope": EVIDENCE_SCOPE,
                    "outcome": "expected",
                    "resultFile": run["result"],
                }
            )
            new_raw = (
                json.dumps(proposal, sort_keys=True, separators=(",", ":")) + "\n"
            ).encode()
            if new_raw == raw:
                raise ValueError("target virtual row did not change")
            virtual_hash.update(new_raw)
            after["expected"] += 1
            changed.append(
                {
                    "line": rows,
                    "specId": spec_id,
                    "project": record["projectId"],
                    "beforeSha256": hashlib.sha256(raw).hexdigest(),
                    "afterSha256": hashlib.sha256(new_raw).hexdigest(),
                    "beforeOutcome": "not-run",
                    "afterOutcome": "expected",
                }
            )
    if (
        rows != 48477
        or len(changed) != 4
        or {row["specId"] for row in changed} != set(EXPECTED_LINES)
    ):
        raise ValueError("virtual ledger does not change exactly four target rows")
    if old_hash.hexdigest() != PREDECESSOR_HASHES[mode]:
        raise ValueError("predecessor streaming digest mismatch")
    if (
        after["expected"] != before["expected"] + 4
        or after["not-run"] != before["not-run"] - 4
    ):
        raise ValueError("virtual outcome arithmetic differs")
    if any(
        after[key] != value
        for key, value in before.items()
        if key not in {"expected", "not-run"}
    ):
        raise ValueError("other outcomes changed")
    return {
        "mode": mode,
        "predecessorSha256": old_hash.hexdigest(),
        "virtualSuccessorSha256": virtual_hash.hexdigest(),
        "rows": rows,
        "byteChangedRows": changed,
        "beforeOutcomes": dict(sorted(before.items())),
        "virtualOutcomes": dict(sorted(after.items())),
        "materialized": False,
    }


def main() -> None:
    if sys.flags.optimize:
        raise RuntimeError("virtual Core audit requires Python assertions enabled")
    receipt = PREDECESSOR / "promotion-receipt.json"
    if digest(receipt) != PREDECESSOR_RECEIPT_SHA256:
        raise ValueError("canonical predecessor receipt changed")
    oracle, candidate = verify_pair(
        capture.RESULT_PARENT / "oracle-v3.strict.json",
        capture.RESULT_PARENT / "candidate-v3.strict.json",
    )
    print(
        json.dumps(
            {
                "scope": "read-only hypothetical successor, no ledger files written",
                "predecessorReceiptSha256": PREDECESSOR_RECEIPT_SHA256,
                "oracle": virtual_diff("oracle", oracle),
                "candidate": virtual_diff("candidate", candidate),
            },
            indent=2,
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
