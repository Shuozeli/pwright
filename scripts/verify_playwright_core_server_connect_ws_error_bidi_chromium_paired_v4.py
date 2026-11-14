#!/usr/bin/env python3
"""Independently verify the six-row Bidi WS-error Core successor.

This fixture-adapted implementation-internal diagnostic is not public-API or
browser parity evidence. The verifier does not import the promoter.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from collections import Counter
from collections.abc import Iterable, Mapping
from itertools import zip_longest
from pathlib import Path
from typing import Any

import run_playwright_core_server_connect_ws_error_bidi_chromium_strict_v4 as capture
from verify_playwright_core_server_connect_version_mismatch_paired_v3 import (
    verify_existing as verify_predecessor_bundle,
)
from verify_playwright_core_server_connect_ws_error_bidi_chromium_strict_v4 import (
    verify_pair,
)

ROOT = Path(__file__).resolve().parents[1]
OUTCOMES = ROOT / ".work/official-conformance/outcomes"
PREDECESSOR = (
    OUTCOMES / "core-server-connect-version-mismatch-url-owner-strict-paired-v3"
)
SUCCESSOR = OUTCOMES / "core-server-connect-ws-error-bidi-chromium-strict-paired-v4"
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
PROMOTER = (
    ROOT
    / "scripts/promote_playwright_core_server_connect_ws_error_bidi_chromium_strict_v4.py"
)
MUTATION_TEST = (
    ROOT
    / "scripts/test_verify_playwright_core_server_connect_ws_error_bidi_chromium_paired_v4.py"
)
COUNT = 48_477
LANES = ("candidate", "oracle")
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
PREDECESSOR_RECEIPT_SHA256 = (
    "5e6180a6b2e0a90d764276af84b933b14b6117cc4dd799a0f925622ea347e1b8"
)
PREDECESSOR_LEDGER_SHA256 = {
    "candidate": "e8485c418287054c78c2b1331d2cd258103d556fb0aca2bf988a5ac738f17e04",
    "oracle": "6994207de60e01ad2e36dc296fe20922e39ab2483d574d02b77d42e18bb2fa3c",
}
PREDECESSOR_SUMMARY_SHA256 = {
    "candidate": "7302bec7437611f8d590443bb23fe3ebdb6dd87128d57f51d899548f67d6c55d",
    "oracle": "0a2e1eb879d1d78cd759fcbb7f19a024b5600489d428b45368a2a69c9e39fe87",
}
STRICT_SHA256 = {
    "candidate": "64c5d948cba49d1973ee353eef49328a28838861e86c4cee0c911e34a9a07f55",
    "oracle": "d896dd6a35598140a845e78096aa686f6b3c6922aee851074a9b9f8d3a485b20",
}
SNAPSHOT_MANIFEST_SHA256 = (
    "a6367b883ec96f48d247a6c685c22ce7725df5a2315a968b26e9c27d8a3fd755"
)
CANDIDATE_SOURCE_SHA256 = (
    "a2bb47a7c22f88b1c8cbaff72fb107de0a015eec85f6a4101d94e0cbddbfca5a"
)
STRICT_VERIFIER_SHA256 = (
    "81ae905d120bdfa277cae229d5ee378ad4f67541fc3acd6df56bbc526a2692f9"
)
SCOPE = "official-browser-free-server-connect-ws-error-bidi-chromium-internal-diagnostic-green-v4"
CLASSIFICATION_REASONS = [
    "private-client-access",
    "runtime-source-import:packages/playwright-core/lib/server/utils/network",
    "runtime-source-import:packages/playwright-core/lib/server/utils/userAgent",
]
TARGETS = {
    5348: (
        "ee6df4b3a2217bd356a7-b8cf56582c005adbb27a",
        "bidi-chrome-canary-library",
        "run-server",
    ),
    5349: (
        "ee6df4b3a2217bd356a7-f3aa7e41c539ce16e04a",
        "bidi-chrome-canary-library",
        "launchServer",
    ),
    5350: (
        "ee6df4b3a2217bd356a7-3020514ab5e796a1d392",
        "bidi-chrome-stable-library",
        "launchServer",
    ),
    5351: (
        "ee6df4b3a2217bd356a7-d09b02e8063437028bbd",
        "bidi-chrome-stable-library",
        "run-server",
    ),
    5352: (
        "ee6df4b3a2217bd356a7-2fde122253a3920eead1",
        "bidi-chromium-library",
        "run-server",
    ),
    5353: (
        "ee6df4b3a2217bd356a7-5be15441c4b3627bd25a",
        "bidi-chromium-library",
        "launchServer",
    ),
}
EXPECTED_FILES = {
    "candidate.jsonl",
    "candidate.summary.json",
    "oracle.jsonl",
    "oracle.summary.json",
    "promotion-receipt.json",
}
RECEIPT_FIELDS = {
    "schema",
    "scope",
    "promotionKind",
    "fixtureAdapted",
    "publicApiParityEvidence",
    "browserExecutionEvidence",
    "sourceClassificationPreserved",
    "diagnosticOnly",
    "candidateRuntimeAttested",
    "selectedOfficialTestRequiresBrowser",
    "selectedOfficialTestStartsRemoteServer",
    "upstreamAssertionsModified",
    "captureSidecarPromotedFlagAtCapture",
    "promoterSourceSha256",
    "strictVerifierSha256",
    "pairedVerifierSha256",
    "promotionMutationTestSha256",
    "harnessInputs",
    "upstreamInputs",
    "priorBundle",
    "priorBundleReceiptSha256",
    "inventory",
    "inventorySha256",
    "recordsPerLane",
    "promotedRowsPerLane",
    "targetZeroBasedIndexes",
    "targetSpecIds",
    "lanes",
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sealed(path: Path, expected: str, label: str) -> None:
    require(path.is_file() and not path.is_symlink(), f"{label} missing or linked")
    require(digest(path.read_bytes()) == expected, f"{label} SHA-256 drifted")


def target_source(spec_id: str, project: str, kind: str) -> dict[str, Any]:
    return {
        "classificationReasons": CLASSIFICATION_REASONS,
        "column": 5,
        "expectedStatus": "passed",
        "file": "library/browsertype-connect.spec.ts",
        "line": 122,
        "projectId": project,
        "projectName": project,
        "role": "official-entrypoint",
        "sourceClassification": "implementation-internal",
        "sourceFile": "tests/library/browsertype-connect.spec.ts",
        "specId": spec_id,
        "suite": "bidi",
        "titlePath": [
            "library/browsertype-connect.spec.ts",
            kind,
            "should print ws error",
        ],
    }


def verify_successor_lines(
    inventory_lines: Iterable[bytes],
    predecessor_lines: Iterable[bytes],
    successor_lines: Iterable[bytes],
    *,
    lane: str,
    results_by_id: Mapping[str, str],
    prior_outcomes: Mapping[str, int],
    prior_scopes: Mapping[str, int],
    expected_count: int = COUNT,
    targets: Mapping[int, tuple[str, str, str]] = TARGETS,
) -> tuple[dict[str, int], dict[str, int], list[dict[str, Any]]]:
    """Require exactly the six target row deltas and raw equality elsewhere."""
    require(lane in LANES, "unknown lane")
    require(len(set(targets)) == 6, "target count differs")
    require(
        set(results_by_id) == {item[0] for item in targets.values()},
        "official result ID set differs",
    )
    before: Counter[str] = Counter()
    after: Counter[str] = Counter()
    before_scopes: Counter[str] = Counter()
    after_scopes: Counter[str] = Counter()
    changed: list[dict[str, Any]] = []
    count = 0
    for index, (inventory_raw, old_raw, new_raw) in enumerate(
        zip_longest(inventory_lines, predecessor_lines, successor_lines)
    ):
        require(
            inventory_raw is not None and old_raw is not None and new_raw is not None,
            f"{lane} row count differs at {index}",
        )
        require(
            all(raw.endswith(b"\n") for raw in (inventory_raw, old_raw, new_raw)),
            f"{lane} unterminated row {index}",
        )
        source = json.loads(inventory_raw)
        old = json.loads(old_raw)
        require(
            isinstance(source, dict)
            and isinstance(old, dict)
            and all(old.get(key) == value for key, value in source.items())
            and old.get("outcome") in {"expected", "unexpected", "skipped", "not-run"}
            and type(old.get("attempts")) is int
            and old["attempts"] >= 0,
            f"{lane} predecessor identity/outcome differs at {index}",
        )
        before[old["outcome"]] += 1
        if "evidenceScope" in old:
            require(
                isinstance(old["evidenceScope"], str), f"{lane} old scope malformed"
            )
            before_scopes[old["evidenceScope"]] += 1
        if index in targets:
            spec_id, project, kind = targets[index]
            require(
                source == target_source(spec_id, project, kind)
                and old == {**source, "attempts": 0, "outcome": "not-run"},
                f"{lane} target identity/classification differs at {index}",
            )
            expected = {
                **old,
                "attempts": 1,
                "candidateRuntimeRequired": lane == "candidate",
                "evidenceScope": SCOPE,
                "outcome": "expected",
                "resultFile": results_by_id[spec_id],
            }
            canonical = (
                json.dumps(expected, sort_keys=True, separators=(",", ":")) + "\n"
            ).encode()
            require(
                new_raw == canonical and new_raw != old_raw,
                f"{lane} target row {index} is not the exact allowed delta",
            )
            changed.append(
                {
                    "index": index,
                    "specId": spec_id,
                    "oldRowSha256": digest(old_raw),
                    "newRowSha256": digest(new_raw),
                }
            )
            after["expected"] += 1
            after_scopes[SCOPE] += 1
        else:
            require(old_raw == new_raw, f"{lane} non-target row {index} changed")
            require(
                old.get("specId") not in {entry[0] for entry in targets.values()},
                f"{lane} target ID appears at wrong row {index}",
            )
            after[old["outcome"]] += 1
            if "evidenceScope" in old:
                after_scopes[old["evidenceScope"]] += 1
        count += 1
    require(
        count == expected_count
        and [item["index"] for item in changed] == sorted(targets),
        f"{lane} target positions or total row count differ",
    )
    require(
        dict(before) == dict(prior_outcomes)
        and dict(before_scopes) == dict(prior_scopes),
        f"{lane} predecessor counts differ",
    )
    outcomes = before.copy()
    outcomes["not-run"] -= 6
    outcomes["expected"] += 6
    scopes = before_scopes.copy()
    scopes[SCOPE] += 6
    require(
        before["not-run"] >= 6 and after == outcomes and after_scopes == scopes,
        f"{lane} successor outcome/scope arithmetic differs",
    )
    return dict(sorted(after.items())), dict(sorted(after_scopes.items())), changed


def verify_inputs() -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    sealed(INVENTORY, INVENTORY_SHA256, "Core inventory")
    sealed(
        PREDECESSOR / "promotion-receipt.json",
        PREDECESSOR_RECEIPT_SHA256,
        "predecessor receipt",
    )
    predecessor_receipt = verify_predecessor_bundle()
    require(
        predecessor_receipt["recordsPerLane"] == COUNT
        and predecessor_receipt["publicApiParityEvidence"] is False,
        "predecessor identity differs",
    )
    for lane in LANES:
        sealed(
            PREDECESSOR / f"{lane}.jsonl",
            PREDECESSOR_LEDGER_SHA256[lane],
            f"{lane} predecessor ledger",
        )
        sealed(
            PREDECESSOR / f"{lane}.summary.json",
            PREDECESSOR_SUMMARY_SHA256[lane],
            f"{lane} predecessor summary",
        )
    strict_paths = {
        lane: capture.RESULT_PARENT / f"{lane}-v4.strict.json" for lane in LANES
    }
    for lane in LANES:
        sealed(strict_paths[lane], STRICT_SHA256[lane], f"{lane} strict sidecar")
    oracle, candidate = verify_pair(strict_paths["oracle"], strict_paths["candidate"])
    strict = {"candidate": candidate, "oracle": oracle}
    sealed(
        capture.SNAPSHOT_MANIFEST,
        SNAPSHOT_MANIFEST_SHA256,
        "candidate snapshot manifest",
    )
    sealed(
        capture.SOURCE_PACKAGE / "native" / capture.PROFILE / "pwright.node",
        capture.ARTIFACT_SHA256,
        "candidate addon",
    )
    sealed(
        ROOT
        / "scripts/verify_playwright_core_server_connect_ws_error_bidi_chromium_strict_v4.py",
        STRICT_VERIFIER_SHA256,
        "strict evidence verifier",
    )
    require(
        candidate["candidateSourceSha256"] == CANDIDATE_SOURCE_SHA256
        and candidate["candidateSnapshotManifestSha256"] == SNAPSHOT_MANIFEST_SHA256
        and candidate["candidateArtifactSha256"] == capture.ARTIFACT_SHA256
        and oracle["candidateArtifactSha256"] is None,
        "candidate/oracle runtime provenance differs",
    )
    expected_ids = {entry[0] for entry in TARGETS.values()}
    for lane in LANES:
        records = [case for run in strict[lane]["runs"] for case in run["selected"]]
        require(
            len(records) == 6
            and {case["specId"] for case in records} == expected_ids
            and all(
                case["status"] == "expected"
                and case["errors"] == []
                and len(case["attempts"]) == 1
                and case["attempts"][0]["retry"] == 0
                and case["attempts"][0]["status"] == "passed"
                for case in records
            )
            and all(run["exitCode"] == 0 for run in strict[lane]["runs"]),
            f"{lane} official results do not contain six clean passes",
        )
    return predecessor_receipt, strict


def expected_header(strict: Mapping[str, dict[str, Any]]) -> dict[str, Any]:
    require(
        strict["candidate"]["inputs"] == strict["oracle"]["inputs"],
        "candidate/oracle harness differs",
    )
    return {
        "schema": 1,
        "scope": SCOPE,
        "promotionKind": "implementation-internal-diagnostic",
        "fixtureAdapted": True,
        "publicApiParityEvidence": False,
        "browserExecutionEvidence": False,
        "sourceClassificationPreserved": "implementation-internal",
        "diagnosticOnly": True,
        "candidateRuntimeAttested": True,
        "selectedOfficialTestRequiresBrowser": False,
        "selectedOfficialTestStartsRemoteServer": False,
        "upstreamAssertionsModified": False,
        "captureSidecarPromotedFlagAtCapture": False,
        "promoterSourceSha256": digest(PROMOTER.read_bytes()),
        "strictVerifierSha256": STRICT_VERIFIER_SHA256,
        "pairedVerifierSha256": digest(Path(__file__).read_bytes()),
        "promotionMutationTestSha256": digest(MUTATION_TEST.read_bytes()),
        "harnessInputs": strict["candidate"]["inputs"],
        "upstreamInputs": capture.UPSTREAM_INPUTS,
        "priorBundle": str(PREDECESSOR),
        "priorBundleReceiptSha256": PREDECESSOR_RECEIPT_SHA256,
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "recordsPerLane": COUNT,
        "promotedRowsPerLane": 6,
        "targetZeroBasedIndexes": list(TARGETS),
        "targetSpecIds": [entry[0] for entry in TARGETS.values()],
    }


def verify_payloads(
    files: Mapping[str, bytes],
    *,
    checked_inputs: tuple[dict[str, Any], dict[str, dict[str, Any]]] | None = None,
) -> dict[str, Any]:
    """Verify a planned or published bundle without using promoter transforms."""
    require(set(files) == EXPECTED_FILES, "successor file set differs")
    predecessor_receipt, strict = checked_inputs or verify_inputs()
    receipt = json.loads(files["promotion-receipt.json"])
    require(
        isinstance(receipt, dict) and set(receipt) == RECEIPT_FIELDS,
        "successor receipt field set differs",
    )
    require(
        {key: value for key, value in receipt.items() if key != "lanes"}
        == expected_header(strict)
        and isinstance(receipt["lanes"], dict)
        and set(receipt["lanes"]) == set(LANES),
        "successor receipt header, no-parity flags, or lane set differs",
    )
    with INVENTORY.open("rb") as source:
        inventory_lines = source.readlines()
    for lane in LANES:
        old_bytes = (PREDECESSOR / f"{lane}.jsonl").read_bytes()
        new_bytes = files[f"{lane}.jsonl"]
        old_summary = json.loads((PREDECESSOR / f"{lane}.summary.json").read_bytes())
        result_by_id = {
            case["specId"]: run["result"]
            for run in strict[lane]["runs"]
            for case in run["selected"]
        }
        outcomes, scopes, row_hashes = verify_successor_lines(
            inventory_lines,
            old_bytes.splitlines(keepends=True),
            new_bytes.splitlines(keepends=True),
            lane=lane,
            results_by_id=result_by_id,
            prior_outcomes=old_summary["outcomes"],
            prior_scopes=old_summary["explicitEvidenceScopes"],
        )
        summary_bytes = files[f"{lane}.summary.json"]
        summary = json.loads(summary_bytes)
        ledger_path = SUCCESSOR / f"{lane}.jsonl"
        require(
            summary
            == {
                "schema": 1,
                "inventory": str(INVENTORY),
                "records": COUNT,
                "outcomes": outcomes,
                "explicitEvidenceScopes": scopes,
                "ledgerSha256": digest(new_bytes),
                "ledger": str(ledger_path),
            },
            f"{lane} summary differs from independently counted rows",
        )
        expected_runs = [
            {
                "project": run["project"],
                "result": run["result"],
                "resultSha256": run["resultSha256"],
                "rawProvenance": str(capture.paths(lane, run["project"])["provenance"]),
                "rawProvenanceSha256": run["provenanceSha256"],
                "guardReceipt": str(capture.paths(lane, run["project"])["receipt"]),
                "guardReceiptSha256": run["guardReceiptSha256"],
            }
            for run in strict[lane]["runs"]
        ]
        expected_lane = {
            "base": str(PREDECESSOR / f"{lane}.jsonl"),
            "baseSha256": PREDECESSOR_LEDGER_SHA256[lane],
            "baseSummarySha256": PREDECESSOR_SUMMARY_SHA256[lane],
            "strictSidecar": str(capture.RESULT_PARENT / f"{lane}-v4.strict.json"),
            "strictSidecarSha256": STRICT_SHA256[lane],
            "officialRuns": expected_runs,
            "candidateArtifactSha256": strict[lane]["candidateArtifactSha256"],
            "candidateSnapshotManifestSha256": strict[lane][
                "candidateSnapshotManifestSha256"
            ],
            "candidateSourceSha256": strict[lane]["candidateSourceSha256"],
            "ledger": str(ledger_path),
            "ledgerSha256": digest(new_bytes),
            "summarySha256": digest(summary_bytes),
            "oldOutcome": "not-run",
            "newOutcome": "expected",
            "promotedRows": 6,
            "changedRows": row_hashes,
            "outcomes": outcomes,
        }
        require(
            receipt["lanes"][lane] == expected_lane,
            f"{lane} receipt does not match verified source/result/row hashes",
        )
        require(
            predecessor_receipt["lanes"][lane]["ledgerSha256"]
            == PREDECESSOR_LEDGER_SHA256[lane],
            f"{lane} predecessor receipt ledger pin differs",
        )
    return receipt


def verify_existing() -> dict[str, Any]:
    if sys.flags.optimize:
        raise RuntimeError("paired Core verification requires assertions enabled")
    require(
        SUCCESSOR.is_dir() and not SUCCESSOR.is_symlink(),
        "immutable successor directory missing or linked",
    )
    require(
        {entry.name for entry in SUCCESSOR.iterdir()} == EXPECTED_FILES,
        "successor file set differs",
    )
    for name in EXPECTED_FILES:
        path = SUCCESSOR / name
        require(
            path.is_file() and not path.is_symlink(),
            f"successor {name} linked or missing",
        )
    return verify_payloads(
        {name: (SUCCESSOR / name).read_bytes() for name in EXPECTED_FILES}
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    try:
        receipt = verify_existing()
    except (
        AssertionError,
        OSError,
        ValueError,
        KeyError,
        TypeError,
        json.JSONDecodeError,
    ) as error:
        parser.exit(2, f"Bidi WS-error paired successor invalid: {error}\n")
    print(
        json.dumps(
            {
                "scope": receipt["scope"],
                "recordsPerLane": receipt["recordsPerLane"],
                "outcomes": {
                    lane: receipt["lanes"][lane]["outcomes"] for lane in LANES
                },
                "ledgerSha256": {
                    lane: receipt["lanes"][lane]["ledgerSha256"] for lane in LANES
                },
            },
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
