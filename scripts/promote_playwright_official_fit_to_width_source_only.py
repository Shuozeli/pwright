"""Promote exactly two verified upstream-private fit-to-width rows per Core lane.

The candidate and oracle receipts run upstream reporter code without loading
Pwright. This promotion never grants public API parity or candidate runtime
attestation.
"""

from __future__ import annotations

import hashlib
import json
import os
from collections import Counter
from pathlib import Path
from typing import Any

from verify_playwright_official_fit_to_width_source_only import verify

_ROOT = Path(__file__).parents[1]
_OUTCOMES = _ROOT / ".work/official-conformance/outcomes"
_RESULTS = _ROOT / ".work/official-conformance/results"
_EXPECTED_RECORDS = 48_477
_TARGETS = {
    "43ecc55b07560c6b6ffc-c2e63de005eea391e61b": (20, "chinese characters"),
    "43ecc55b07560c6b6ffc-9aca4b4e10409bacd54b": (25, "surrogate pairs"),
}
_LANES = {
    "candidate": {
        "base": _OUTCOMES / "playwright-core-candidate-elementhandle-misc10-20261011-v1.jsonl",
        "baseSha256": "5a9a29a3713f0047a2aa179f4da0c1a768ba77331dd721d8bbdc431dc0b34acb",
        "report": _RESULTS / "playwright-core-candidate-core463-private-fit-to-width-source-only-v2.json",
        "reportSha256": "1dd7da296164a639b7a20df6d216d8f659d4dbe9d04f43973eb77642ad30a723",
        "successor": _OUTCOMES / "playwright-core-candidate-fit-to-width-source-only-20261011-v1.jsonl",
    },
    "oracle": {
        "base": _OUTCOMES / "playwright-core-oracle-elementhandle-misc10-20261011-v1.jsonl",
        "baseSha256": "069bb83ac37346ce4000485d5bc16d7832d3712cecb070746c91e33402420dfc",
        "report": _RESULTS / "playwright-core-oracle-core463-private-fit-to-width-source-only-v2.json",
        "reportSha256": "0c6a0a73b0b5d489b2fe0eae277f2837d37a5aa0c490b2aea62c4a9691545685",
        "successor": _OUTCOMES / "playwright-core-oracle-fit-to-width-source-only-20261011-v1.jsonl",
    },
}


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def _sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _verify_receipt(lane: str, config: dict[str, Any]) -> None:
    report = config["report"]
    provenance = report.with_suffix(".provenance.json")
    receipt = report.with_suffix(".guard-receipt.jsonl")
    _require(all(path.is_file() and not path.is_symlink() for path in (report, provenance, receipt)),
             f"{lane} fit-to-width source-only evidence is missing or a symlink")
    checked = verify(provenance)
    _require(checked == {
        "scope": "upstream-private-source-only",
        "classification": "product-infrastructure",
        "suite": "core463-private-fit-to-width",
        "mode": lane,
        "resultSha256": config["reportSha256"],
        "passed": 2,
        "guardedWorkerProcesses": 1,
        "candidateRuntimeAttested": False,
        "publicApiParityEvidence": False,
    }, f"{lane} source-only receipt does not match its two exact passes")
    _require(_sha256(report.read_bytes()) == config["reportSha256"],
             f"{lane} result changed after source-only verification")


def _validate_target(record: dict[str, Any]) -> None:
    spec_id = record["specId"]
    line, title = _TARGETS[spec_id]
    _require(record == {
        "attempts": 0,
        "classificationReasons": [],
        "column": 5,
        "expectedStatus": "passed",
        "file": "fit-to-width.spec.ts",
        "line": line,
        "outcome": "not-run",
        "projectId": "playwright-test",
        "projectName": "playwright-test",
        "role": "official-entrypoint",
        "sourceClassification": "product-infrastructure",
        "sourceFile": "tests/playwright-test/fit-to-width.spec.ts",
        "specId": spec_id,
        "suite": "playwright-test",
        "titlePath": ["fit-to-width.spec.ts", title],
    }, f"fit-to-width target {spec_id} is not an unrun upstream-private inventory entry")


def _prepare_successor(lane: str, config: dict[str, Any]) -> tuple[bytes, bytes, dict[str, Any]]:
    base = config["base"]
    _require(base.is_file() and not base.is_symlink(), f"{lane} current strict base is missing or a symlink")
    baseline = base.read_bytes()
    _require(_sha256(baseline) == config["baseSha256"], f"{lane} current strict base SHA-256 changed")
    base_summary_file = base.with_suffix(".summary.json")
    _require(base_summary_file.is_file() and not base_summary_file.is_symlink(), f"{lane} base summary is missing")
    base_summary = json.loads(base_summary_file.read_bytes())
    _require(base_summary.get("schema") == 1 and base_summary.get("records") == _EXPECTED_RECORDS
             and base_summary.get("ledgerSha256") == config["baseSha256"]
             and base_summary.get("ledger") == str(base), f"{lane} base summary does not match the strict ledger")

    result = str(config["report"])
    output_lines: list[bytes] = []
    seen: set[tuple[str, str, str]] = set()
    found: set[str] = set()
    base_outcomes: Counter[str] = Counter()
    output_outcomes: Counter[str] = Counter()
    base_scopes: Counter[str] = Counter()
    output_scopes: Counter[str] = Counter()
    for original in baseline.splitlines(keepends=True):
        _require(original.endswith(b"\n"), f"{lane} base contains an unterminated JSONL row")
        record = json.loads(original)
        _require(isinstance(record, dict), f"{lane} base contains a malformed row")
        key = (record["suite"], record["specId"], record["projectId"])
        _require(key not in seen, f"{lane} base repeats an invocation")
        seen.add(key)
        base_outcomes[record["outcome"]] += 1
        if "evidenceScope" in record:
            base_scopes[record["evidenceScope"]] += 1
        if record["sourceFile"] == "tests/playwright-test/fit-to-width.spec.ts":
            _require(record["specId"] in _TARGETS and record["specId"] not in found,
                     f"{lane} fit-to-width file contains an unpinned invocation")
            _validate_target(record)
            found.add(record["specId"])
            record = {**record,
                      "attempts": 1,
                      "outcome": "expected",
                      "resultFile": result,
                      "evidenceScope": "upstream-private-source-only",
                      "candidateRuntimeRequired": False}
            output_lines.append((json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n").encode())
        else:
            output_lines.append(original)
        output_outcomes[record["outcome"]] += 1
        if "evidenceScope" in record:
            output_scopes[record["evidenceScope"]] += 1
    _require(len(seen) == _EXPECTED_RECORDS and found == set(_TARGETS),
             f"{lane} strict base lost its complete inventory or two exact fit-to-width IDs")
    _require(dict(base_outcomes) == base_summary["outcomes"]
             and dict(base_scopes) == base_summary["explicitEvidenceScopes"],
             f"{lane} base summary counts differ from the actual rows")
    expected_outcomes = base_outcomes.copy()
    expected_outcomes["not-run"] -= 2
    expected_outcomes["expected"] += 2
    expected_scopes = base_scopes.copy()
    expected_scopes["upstream-private-source-only"] += 2
    _require(output_outcomes == expected_outcomes and output_scopes == expected_scopes,
             f"{lane} successor changed more than its two source-only rows")
    successor = b"".join(output_lines)
    output = config["successor"]
    summary = {
        "schema": 1,
        "inventory": base_summary["inventory"],
        "records": _EXPECTED_RECORDS,
        "outcomes": dict(sorted(output_outcomes.items())),
        "explicitEvidenceScopes": dict(sorted(output_scopes.items())),
        "ledgerSha256": _sha256(successor),
        "ledger": str(output),
    }
    summary_bytes = (json.dumps(summary, indent=2) + "\n").encode()
    return successor, summary_bytes, summary


def main() -> None:
    prepared: dict[str, tuple[bytes, bytes, dict[str, Any]]] = {}
    for lane, config in _LANES.items():
        _verify_receipt(lane, config)
        prepared[lane] = _prepare_successor(lane, config)
        for output in (config["successor"], config["successor"].with_suffix(".summary.json")):
            _require(not output.exists() and not output.is_symlink(), f"immutable successor already exists: {output}")
    for lane, config in _LANES.items():
        ledger_bytes, summary_bytes, summary = prepared[lane]
        for output, data in ((config["successor"], ledger_bytes),
                             (config["successor"].with_suffix(".summary.json"), summary_bytes)):
            with output.open("xb") as destination:
                destination.write(data)
                destination.flush()
                os.fsync(destination.fileno())
        print(json.dumps({"mode": lane, "ledger": str(config["successor"]),
                          "ledgerSha256": summary["ledgerSha256"], "outcomes": summary["outcomes"],
                          "explicitEvidenceScopes": summary["explicitEvidenceScopes"]}, sort_keys=True))


if __name__ == "__main__":
    main()
