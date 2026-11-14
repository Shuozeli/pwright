#!/usr/bin/env python3
"""Read-only proposed Core ledger delta for the sealed v3 version-error pair."""

from __future__ import annotations

import hashlib
import json
import sys
from collections import Counter
from pathlib import Path
from typing import Any

from run_playwright_core_runserver_url_owner_v3 import verify_envelope

ROOT = Path(__file__).resolve().parents[1]
PREDECESSOR = (
    ROOT
    / ".work/official-conformance/outcomes"
    / "core-server-connect-ws-error-other-engines-strict-paired-v3"
)
PREDECESSOR_RECEIPT_SHA256 = (
    "59c7cbb11ad7284ab8aedadb8c850cd4552ffaa0b035b70430c8b4e2e4ef0ef7"
)
PREDECESSOR_SHA256 = {
    "candidate": "e7d5924d9face0aaac68f5bf1db35516cb6d687871aa34c55361b961e69aee2d",
    "oracle": "3db929fbe661c35aee7f05c9ea4ae3353a0fc4dd249550a1ea43f918f1de8865",
}
RESULTS = (
    ROOT
    / ".work/official-conformance/results"
    / "core-server-connect-version-mismatch-url-owner-v3/strict-pair-20261020-v3"
)
ENVELOPE_SHA256 = {
    "candidate": "11ea37b4233f799f73f02ff6e7a0836f0635aadd7dd7af8b42ed9f3b7f998f8b",
    "oracle": "54641e1250d83dcd657f9657b56549ecba80ffcfdf458d01863ead6f11ace230",
}
TARGET_LINE = 31_533
TARGET_ID = "ee6df4b3a2217bd356a7-d60bdc61fb009792457d"
SCOPE = "official-browser-free-server-version-mismatch-internal-diagnostic-green-v3"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def selected(mode: str) -> dict[str, Any]:
    path = RESULTS / f"{mode}.runserver-url-owner-v3-attestation.json"
    if path.is_symlink() or sha256(path) != ENVELOPE_SHA256[mode]:
        raise ValueError(f"{mode} sealed v3 attestation drifted")
    envelope = verify_envelope(path)
    if (
        envelope["mode"] != mode
        or envelope["specId"] != TARGET_ID
        or envelope["status"] != "expected"
        or envelope["runnerExitCode"] != 0
    ):
        raise ValueError(f"{mode} official assertion did not pass exactly once")
    report = json.loads(Path(envelope["result"]).read_text())
    specs: list[dict[str, Any]] = []

    def visit(suite: dict[str, Any]) -> None:
        specs.extend(suite.get("specs", []))
        for child in suite.get("suites", []):
            visit(child)

    for suite in report["suites"]:
        visit(suite)
    if len(specs) != 1 or len(specs[0].get("tests", [])) != 1:
        raise ValueError(f"{mode} official report did not select one test")
    spec = specs[0]
    test = spec["tests"][0]
    if (
        spec["id"] != TARGET_ID
        or spec["line"] != 1046
        or spec["ok"] is not True
        or test["projectId"] != "chromium-library"
        or test["expectedStatus"] != "passed"
        or test["status"] != "expected"
        or len(test["results"]) != 1
        or test["results"][0]["retry"] != 0
        or test["results"][0]["status"] != "passed"
        or test["results"][0]["errors"] != []
    ):
        raise ValueError(f"{mode} official attempt was not one clean pass")
    return envelope


def audit(mode: str, envelope: dict[str, Any]) -> dict[str, Any]:
    predecessor = PREDECESSOR / f"{mode}.jsonl"
    if predecessor.is_symlink() or sha256(predecessor) != PREDECESSOR_SHA256[mode]:
        raise ValueError(f"{mode} canonical predecessor drifted")
    before: Counter[str] = Counter()
    after: Counter[str] = Counter()
    virtual = hashlib.sha256()
    changed: dict[str, Any] | None = None
    count = 0
    with predecessor.open("rb") as stream:
        for count, raw in enumerate(stream, start=1):
            if not raw.endswith(b"\n"):
                raise ValueError(f"{mode} unterminated ledger row {count}")
            row = json.loads(raw)
            before[row["outcome"]] += 1
            if count != TARGET_LINE:
                if row["specId"] == TARGET_ID:
                    raise ValueError(f"{mode} target moved")
                virtual.update(raw)
                after[row["outcome"]] += 1
                continue
            if (
                row["specId"] != TARGET_ID
                or row["outcome"] != "not-run"
                or row["attempts"] != 0
                or row["suite"] != "library"
                or row["projectId"] != "chromium-library"
                or row["sourceClassification"] != "implementation-internal"
                or row["line"] != 1046
                or row["expectedStatus"] != "passed"
            ):
                raise ValueError(f"{mode} target predecessor identity drifted")
            new_row = {
                **row,
                "attempts": 1,
                "candidateRuntimeRequired": mode == "candidate",
                "evidenceScope": SCOPE,
                "outcome": "expected",
                "resultFile": envelope["result"],
            }
            new_raw = (
                json.dumps(new_row, sort_keys=True, separators=(",", ":")) + "\n"
            ).encode()
            virtual.update(new_raw)
            after["expected"] += 1
            changed = {
                "line": count,
                "specId": TARGET_ID,
                "oldRowSha256": hashlib.sha256(raw).hexdigest(),
                "newRowSha256": hashlib.sha256(new_raw).hexdigest(),
            }
    if (
        count != 48_477
        or changed is None
        or after["expected"] != before["expected"] + 1
        or after["not-run"] != before["not-run"] - 1
        or any(
            after[key] != value
            for key, value in before.items()
            if key not in {"expected", "not-run"}
        )
    ):
        raise ValueError(f"{mode} virtual delta is not exactly one outcome")
    return {
        "mode": mode,
        "predecessorSha256": PREDECESSOR_SHA256[mode],
        "virtualSuccessorSha256": virtual.hexdigest(),
        "rows": count,
        "changed": changed,
        "before": dict(sorted(before.items())),
        "after": dict(sorted(after.items())),
        "materialized": False,
    }


def main() -> None:
    if sys.flags.optimize:
        raise RuntimeError("virtual Core audit requires Python assertions enabled")
    if sha256(PREDECESSOR / "promotion-receipt.json") != PREDECESSOR_RECEIPT_SHA256:
        raise ValueError("canonical predecessor receipt drifted")
    print(
        json.dumps(
            {
                "candidate": audit("candidate", selected("candidate")),
                "oracle": audit("oracle", selected("oracle")),
            },
            indent=2,
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
