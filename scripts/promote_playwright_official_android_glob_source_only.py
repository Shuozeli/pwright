"""Promote one verified upstream-private Android glob row per strict Core lane.

This source-only evidence never grants Pwright runtime attestation or public
API parity. Every other inventory row is preserved byte-for-byte.
"""

from __future__ import annotations

import hashlib
import json
import os
from collections import Counter
from pathlib import Path
from typing import Any

from verify_playwright_official_android_glob_source_only import verify

_ROOT = Path(__file__).parents[1]
_OUTCOMES = _ROOT / ".work/official-conformance/outcomes"
_RESULTS = _ROOT / ".work/official-conformance/results"
_EXPECTED_RECORDS = 48_477
_SPEC_ID = "c569f641042bf346bcbd-ff0d174e7981f09583b5"
_LANES = {
    "candidate": {
        "base": _OUTCOMES / "playwright-core-candidate-fit-to-width-source-only-20261011-v1.jsonl",
        "baseSha256": "017fc15553e0358f1f6495cfb614274e886451001419d91f17d53f853a24d271",
        "report": _RESULTS / "playwright-core-candidate-core464-private-android-glob-source-only-v1.json",
        "reportSha256": "e6ef72fbcddc802607568046f73b08f062bcfa8a17823a8e6c601fec433de6ce",
        "provenanceSha256": "2a14c45b0a3a39e77e5c6b41422e743a8ff073cc5e348b6708f151d7fa56d7a1",
        "receiptSha256": "2621afa0df5f332f4718ff80c44392dd12efa493d6bc02af082d701f2ef4fda9",
        "successor": _OUTCOMES / "playwright-core-candidate-android-glob-source-only-20261011-v1.jsonl",
    },
    "oracle": {
        "base": _OUTCOMES / "playwright-core-oracle-fit-to-width-source-only-20261011-v1.jsonl",
        "baseSha256": "c08dd5c73b61b12d9961bd642eff1097c98e0f4da61c1cdc0a017df860a4708e",
        "report": _RESULTS / "playwright-core-oracle-core464-private-android-glob-source-only-v1.json",
        "reportSha256": "956db2412f19afcc65a546af2d6560d9d582239e3b0be35b1dac71fc008aba8d",
        "provenanceSha256": "279d00209815a24661ae655d2e2056dba53e5016a043e507df56e4f287d968c3",
        "receiptSha256": "7fa4b3ce7ffb65adc8da34e8026cb5df6187580919f84747584a98e66c2ec8d0",
        "successor": _OUTCOMES / "playwright-core-oracle-android-glob-source-only-20261011-v1.jsonl",
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
             f"{lane} Android glob source-only evidence is missing or a symlink")
    _require(_sha256(report.read_bytes()) == config["reportSha256"]
             and _sha256(provenance.read_bytes()) == config["provenanceSha256"]
             and _sha256(receipt.read_bytes()) == config["receiptSha256"],
             f"{lane} Android glob evidence bytes changed")
    checked = verify(provenance)
    _require(checked == {
        "scope": "upstream-private-source-only",
        "classification": "implementation-internal",
        "suite": "core464-private-android-glob",
        "mode": lane,
        "resultSha256": config["reportSha256"],
        "passed": 1,
        "guardedWorkerProcesses": 1,
        "candidateRuntimeRequired": False,
        "candidateRuntimeAttested": False,
        "publicApiParityEvidence": False,
    }, f"{lane} source-only receipt does not match one exact pass")


def _validate_target(record: dict[str, Any]) -> None:
    _require(record == {
        "attempts": 0,
        "classificationReasons": ["runtime-source-import:packages/playwright-core/lib/utils/isomorphic/urlMatch"],
        "column": 3,
        "expectedStatus": "passed",
        "file": "page/interception.spec.ts",
        "line": 73,
        "outcome": "not-run",
        "projectId": "android-page",
        "projectName": "android-page",
        "role": "official-entrypoint",
        "sourceClassification": "implementation-internal",
        "sourceFile": "tests/page/interception.spec.ts",
        "specId": _SPEC_ID,
        "suite": "android",
        "titlePath": ["page/interception.spec.ts", "should work with glob"],
    }, "Android glob target is not the pinned unrun upstream-private invocation")


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

    output_lines: list[bytes] = []
    seen: set[tuple[str, str, str]] = set()
    found = 0
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
        if key == ("android", _SPEC_ID, "android-page"):
            _require(found == 0, f"{lane} Android glob target repeats")
            _validate_target(record)
            found += 1
            record = {**record,
                      "attempts": 1,
                      "outcome": "expected",
                      "resultFile": str(config["report"]),
                      "evidenceScope": "upstream-private-source-only",
                      "candidateRuntimeRequired": False}
            output_lines.append((json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n").encode())
        else:
            output_lines.append(original)
        output_outcomes[record["outcome"]] += 1
        if "evidenceScope" in record:
            output_scopes[record["evidenceScope"]] += 1
    _require(len(seen) == _EXPECTED_RECORDS and found == 1,
             f"{lane} strict base lost its complete inventory or Android glob ID")
    _require(dict(base_outcomes) == base_summary["outcomes"]
             and dict(base_scopes) == base_summary["explicitEvidenceScopes"],
             f"{lane} base summary counts differ from the actual rows")
    expected_outcomes = base_outcomes.copy()
    expected_outcomes["not-run"] -= 1
    expected_outcomes["expected"] += 1
    expected_scopes = base_scopes.copy()
    expected_scopes["upstream-private-source-only"] += 1
    _require(output_outcomes == expected_outcomes and output_scopes == expected_scopes,
             f"{lane} successor changed more than its one source-only row")
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
