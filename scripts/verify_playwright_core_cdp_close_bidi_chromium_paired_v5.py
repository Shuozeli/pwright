#!/usr/bin/env python3
"""Independently verify the three-row, browserless Bidi CDP Close successor."""

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

import run_playwright_core_cdp_close_bidi_chromium_strict_v4 as capture
from verify_playwright_core_cdp_close_bidi_chromium_strict_v4 import verify_pair
from verify_playwright_core_server_connect_ws_error_bidi_chromium_paired_v4 import (
    verify_existing as verify_predecessor_bundle,
)

ROOT = Path(__file__).resolve().parents[1]
OUTCOMES = ROOT / ".work/official-conformance/outcomes"
PREDECESSOR = OUTCOMES / "core-server-connect-ws-error-bidi-chromium-strict-paired-v4"
SUCCESSOR = OUTCOMES / "core-cdp-close-bidi-chromium-strict-paired-v5"
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
PROMOTER = ROOT / "scripts/promote_playwright_core_cdp_close_bidi_chromium_strict_v5.py"
MUTATION_TEST = (
    ROOT / "scripts/test_verify_playwright_core_cdp_close_bidi_chromium_paired_v5.py"
)
COUNT = 48_477
LANES = ("candidate", "oracle")
SCOPE = "official-browser-free-cdp-close-bidi-chromium-internal-diagnostic-green-v5"
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
PREDECESSOR_RECEIPT_SHA256 = (
    "f7c72c003d4306ade21f077b8b424a60a21d265a72512c5b8ac249c21886d2c6"
)
PREDECESSOR_LEDGER_SHA256 = {
    "candidate": "c19aa1c129d20e25424bfbd3620255f0c6d4b16b0ea38a7284aca4161a9384cc",
    "oracle": "0a95ae35c25da64b74d5c6bc949c39dd1d033f6dca1d586d9cbe8994754978f5",
}
PREDECESSOR_SUMMARY_SHA256 = {
    "candidate": "9aad34b91d9e739c26563a9878657cc1df587ec3be806c45535a9230744cfba0",
    "oracle": "735bc01f5382124fa070208c2f2354a02dc4c6ec09fb7a5e2b4fb61bce0c7105",
}
PREDECESSOR_VERIFIER_SHA256 = (
    "1ada4342300efc0cc9697a227efbd9b8d98fcf0347142774f04b5b7f11e14fa2"
)
STRICT_SHA256 = {
    "candidate": "08043d6464e21092671f5c55e923be4cd3d7fdf199c08a910532789282b4e048",
    "oracle": "5ebba48bb916863875e2c594322fdd7fa1af1b4b5688bff1abdbda3d08c87cab",
}
STRICT_VERIFIER_SHA256 = (
    "7b8dea4ce695d2b392ff445df486b512e84d20b7f684f65fc49d8f15178ebf06"
)
SNAPSHOT_MANIFEST_SHA256 = (
    "c90f2f9d9b7904610abfeb3a37ee720e988b37748bedf8849cf36c7658534115"
)
CANDIDATE_SOURCE_SHA256 = (
    "c4d7f9022e10ef17445bab1edacf01931d526a69d5e8415a0bf770140164dfe7"
)
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
CLASSIFICATION_REASONS = [
    "private-client-access",
    "runtime-source-import:packages/playwright-core/lib/server/utils/userAgent",
]
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
    "protocolUnderTest",
    "bidiProjectNamesOnly",
    "bidiProtocolParityEvidence",
    "fixtureAdapted",
    "publicApiParityEvidence",
    "browserExecutionEvidence",
    "currentLivePackageEvidence",
    "sourceClassificationPreserved",
    "diagnosticOnly",
    "frozenCandidateSnapshotAttested",
    "selectedOfficialTestRequiresBrowser",
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


def target_source(spec_id: str, project: str) -> dict[str, Any]:
    return {
        "classificationReasons": CLASSIFICATION_REASONS,
        "column": 5,
        "expectedStatus": "passed",
        "file": "library/chromium/connect-over-cdp.spec.ts",
        "line": 572,
        "projectId": project,
        "projectName": project,
        "role": "official-entrypoint",
        "sourceClassification": "implementation-internal",
        "sourceFile": capture.SOURCE,
        "specId": spec_id,
        "suite": "bidi",
        "titlePath": [
            "library/chromium/connect-over-cdp.spec.ts",
            "should print custom ws close error",
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
    targets: Mapping[int, tuple[str, str]] = TARGETS,
) -> tuple[dict[str, int], dict[str, int], list[dict[str, Any]]]:
    """Check exact target deltas and raw equality for every other line."""
    require(lane in LANES, "unknown lane")
    require(len(targets) == 3, "target count differs")
    target_ids = {entry[0] for entry in targets.values()}
    require(len(target_ids) == 3, "target IDs duplicate")
    require(set(results_by_id) == target_ids, "official result ID set differs")
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
            require(isinstance(old["evidenceScope"], str), "old scope malformed")
            before_scopes[old["evidenceScope"]] += 1
        if index in targets:
            spec_id, project = targets[index]
            require(
                source == target_source(spec_id, project)
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
            require(old.get("specId") not in target_ids, "target ID at wrong row")
            after[old["outcome"]] += 1
            if "evidenceScope" in old:
                after_scopes[old["evidenceScope"]] += 1
        count += 1
    require(
        count == expected_count
        and [item["index"] for item in changed] == sorted(targets),
        f"{lane} target positions or total row count differ",
    )
    expected_outcomes = before.copy()
    expected_outcomes["not-run"] -= 3
    expected_outcomes["expected"] += 3
    expected_scopes = before_scopes.copy()
    expected_scopes[SCOPE] += 3
    require(
        dict(before) == dict(prior_outcomes)
        and dict(before_scopes) == dict(prior_scopes),
        f"{lane} predecessor counts differ",
    )
    require(
        before["not-run"] >= 3
        and after == expected_outcomes
        and after_scopes == expected_scopes,
        f"{lane} successor arithmetic differs",
    )
    return dict(sorted(after.items())), dict(sorted(after_scopes.items())), changed


def verify_inputs() -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    if sys.flags.optimize:
        raise RuntimeError("paired Core verification requires assertions enabled")
    sealed(INVENTORY, INVENTORY_SHA256, "Core inventory")
    sealed(
        PREDECESSOR / "promotion-receipt.json",
        PREDECESSOR_RECEIPT_SHA256,
        "predecessor receipt",
    )
    sealed(
        ROOT
        / "scripts/verify_playwright_core_server_connect_ws_error_bidi_chromium_paired_v4.py",
        PREDECESSOR_VERIFIER_SHA256,
        "predecessor verifier",
    )
    predecessor = verify_predecessor_bundle()
    require(
        predecessor["recordsPerLane"] == COUNT
        and predecessor["publicApiParityEvidence"] is False,
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
    sealed(
        ROOT / "scripts/verify_playwright_core_cdp_close_bidi_chromium_strict_v4.py",
        STRICT_VERIFIER_SHA256,
        "strict evidence verifier",
    )
    oracle, candidate = verify_pair(strict_paths["oracle"], strict_paths["candidate"])
    strict = {"candidate": candidate, "oracle": oracle}
    sealed(
        capture.FROZEN_MANIFEST,
        SNAPSHOT_MANIFEST_SHA256,
        "frozen candidate snapshot manifest",
    )
    sealed(
        capture.FROZEN_SNAPSHOT / "native" / capture.PROFILE / "pwright.node",
        capture.ARTIFACT_SHA256,
        "frozen candidate addon",
    )
    require(
        candidate["candidateSourceSha256"] == CANDIDATE_SOURCE_SHA256
        and candidate["candidateSnapshotManifestSha256"] == SNAPSHOT_MANIFEST_SHA256
        and candidate["candidateArtifactSha256"] == capture.ARTIFACT_SHA256
        and candidate["candidateSnapshot"] == str(capture.FROZEN_SNAPSHOT)
        and oracle["candidateArtifactSha256"] is None,
        "frozen candidate/oracle provenance differs",
    )
    expected_ids = {entry[0] for entry in TARGETS.values()}
    for lane in LANES:
        sidecar = strict[lane]
        records = [case for run in sidecar["runs"] for case in run["selected"]]
        require(
            sidecar["protocolUnderTest"] == "CDP-over-WebSocket"
            and sidecar["bidiProjectNamesOnly"] is True
            and sidecar["fixtureAdapted"] is True
            and sidecar["publicApiParityEvidence"] is False
            and sidecar["currentLivePackageParityAttested"] is False
            and sidecar["promotedToCoreLedger"] is False
            and len(records) == 3
            and {case["specId"] for case in records} == expected_ids
            and all(
                case["status"] == "expected"
                and case["errors"] == []
                and len(case["attempts"]) == 1
                and case["attempts"][0]["retry"] == 0
                and case["attempts"][0]["status"] == "passed"
                for case in records
            )
            and all(run["exitCode"] == 0 for run in sidecar["runs"]),
            f"{lane} strict CDP Close evidence differs",
        )
    return predecessor, strict


def expected_header(strict: Mapping[str, dict[str, Any]]) -> dict[str, Any]:
    require(
        strict["candidate"]["inputs"] == strict["oracle"]["inputs"],
        "candidate/oracle harness differs",
    )
    return {
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
        "promotedRowsPerLane": 3,
        "targetZeroBasedIndexes": list(TARGETS),
        "targetSpecIds": [entry[0] for entry in TARGETS.values()],
    }


def verify_payloads(
    files: Mapping[str, bytes],
    *,
    checked_inputs: tuple[dict[str, Any], dict[str, dict[str, Any]]] | None = None,
) -> dict[str, Any]:
    """Verify planned or published bytes without importing the promoter."""
    require(set(files) == EXPECTED_FILES, "successor file set differs")
    predecessor, strict = checked_inputs or verify_inputs()
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
        "successor receipt header or no-parity flags differ",
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
            "promotedRows": 3,
            "changedRows": row_hashes,
            "outcomes": outcomes,
        }
        require(
            receipt["lanes"][lane] == expected_lane,
            f"{lane} receipt does not match verified evidence and row hashes",
        )
        require(
            predecessor["lanes"][lane]["ledgerSha256"]
            == PREDECESSOR_LEDGER_SHA256[lane],
            f"{lane} predecessor receipt ledger pin differs",
        )
    return receipt


def verify_existing() -> dict[str, Any]:
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
        require(path.is_file() and not path.is_symlink(), f"{name} linked or missing")
    return verify_payloads(
        {name: (SUCCESSOR / name).read_bytes() for name in EXPECTED_FILES}
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    try:
        receipt = verify_existing()
    except (AssertionError, OSError, ValueError, KeyError, TypeError) as error:
        parser.exit(2, f"Bidi CDP Close paired successor invalid: {error}\n")
    print(
        json.dumps(
            {
                "scope": receipt["scope"],
                "recordsPerLane": receipt["recordsPerLane"],
                "ledgerSha256": {
                    lane: receipt["lanes"][lane]["ledgerSha256"] for lane in LANES
                },
            },
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
