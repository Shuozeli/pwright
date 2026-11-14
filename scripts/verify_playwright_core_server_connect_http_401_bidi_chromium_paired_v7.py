#!/usr/bin/env python3
"""Independently replay the six-row Bidi HTTP-401 Core diagnostic successor.

This module never imports the promoter. Every non-target row must remain byte
identical to the preceding 48,477-row ledger, and all parity credit stays false.
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

import run_playwright_core_server_connect_http_401_bidi_chromium_strict_v6 as capture
from verify_playwright_core_server_connect_http_401_bidi_chromium_strict_v6 import (
    verify_pair,
    verify_strict,
)
from verify_playwright_core_server_connect_timeout_redirect_bidi_chromium_paired_v6 import (
    verify_existing as verify_predecessor_bundle,
)

ROOT = Path(__file__).resolve().parents[1]
OUTCOMES = ROOT / ".work/official-conformance/outcomes"
PREDECESSOR = OUTCOMES / "core-server-connect-timeout-redirect-bidi-chromium-strict-paired-v6"
SUCCESSOR = OUTCOMES / "core-server-connect-http-401-bidi-chromium-strict-paired-v7"
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
PROMOTER = (
    ROOT / "scripts/promote_playwright_core_server_connect_http_401_bidi_chromium_strict_v7.py"
)
MUTATION_TEST = (
    ROOT / "scripts/test_verify_playwright_core_server_connect_http_401_bidi_chromium_paired_v7.py"
)
COUNT = 48_477
LANES = ("candidate", "oracle")
SCOPE = (
    "official-browser-free-server-connect-http-401-bidi-chromium-"
    "fixture-host-adapted-internal-diagnostic-green-v7"
)
PREDECESSOR_SCOPE = (
    "official-browser-free-server-connect-timeout-redirect-"
    "bidi-chromium-internal-diagnostic-green-v6"
)
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
PREDECESSOR_RECEIPT_SHA256 = "43b8f240c16d7c5517fcb16c90f77828c0818552be5e19e368a900356ca266ba"
PREDECESSOR_VERIFIER_SHA256 = "a56e126cf4367bb0e1afeac2e4022cf4776bcb45699bb600e5e6ba48e65db5f6"
PREDECESSOR_LEDGER_SHA256 = {
    "candidate": "7231a32556a6ff6bfe0158b6e3e36f4e5adb921d1e82e7c45d162b8da1e9a5a0",
    "oracle": "edd416263e439f9ede38a8927b343c73890e20de3460e91dade9fff20dd3f1cf",
}
PREDECESSOR_SUMMARY_SHA256 = {
    "candidate": "6a2ef2df0a4b83e4f02d057e9e57826f41ec5b78f93d00ac1dbafa0ca78cba60",
    "oracle": "cf0d0e073330bae4174feae3a1b4c9fd813d6fce2175229c2812901b3aa16a59",
}
STRICT_SHA256 = {
    "candidate": "cd105946ad04c588cc28bd82c6a7b1cd1639ec01b8a095beeec82bfa0ee67c6b",
    "oracle": "52dd2b7283a82cedec91d128b6b1be9f2816f75177a8baf1ad4d515dc15d73f3",
}
STRICT_VERIFIER_SHA256 = "c877fa770934af42e5e7c0c64d30c62f3265120511ea796669bba58e595f8060"
SNAPSHOT_MANIFEST_SHA256 = "ea5b9a1a2437575a82e5c85ae4cdcee1c1e2238a5e6c8cadd7d93e0a0550037b"
CANDIDATE_SOURCE_SHA256 = "1bf7e4d67e226a1666af2abff31db4960781a641bf28d3b392d4c0834c0c95b4"
TARGETS = {
    5336: (
        "ee6df4b3a2217bd356a7-ee9b136cffd3c3d2a689",
        "bidi-chrome-canary-library",
        "launchServer",
    ),
    5337: ("ee6df4b3a2217bd356a7-f9d592efdaee806ceade", "bidi-chrome-canary-library", "run-server"),
    5338: ("ee6df4b3a2217bd356a7-54234fd4c0513393a2e5", "bidi-chrome-stable-library", "run-server"),
    5339: (
        "ee6df4b3a2217bd356a7-fcc2e9a2f05c9afd5838",
        "bidi-chrome-stable-library",
        "launchServer",
    ),
    5340: ("ee6df4b3a2217bd356a7-c029c9945e2dad716fff", "bidi-chromium-library", "run-server"),
    5341: ("ee6df4b3a2217bd356a7-e4ef96db466ddcd9b9c7", "bidi-chromium-library", "launchServer"),
}
CLASSIFICATION_REASONS = [
    "private-client-access",
    "runtime-source-import:packages/playwright-core/lib/server/utils/network",
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
    "fixtureAdaptation",
    "fixtureAdapted",
    "receiptAttribution",
    "publicApiParityEvidence",
    "browserExecutionEvidence",
    "bidiProjectNamesOnly",
    "bidiProtocolParityEvidence",
    "currentLivePackageEvidence",
    "sourceClassificationPreserved",
    "diagnosticOnly",
    "frozenCandidateSnapshotAttested",
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
        "line": 116,
        "projectId": project,
        "projectName": project,
        "role": "official-entrypoint",
        "sourceClassification": "implementation-internal",
        "sourceFile": capture.SOURCE,
        "specId": spec_id,
        "suite": "bidi",
        "titlePath": ["library/browsertype-connect.spec.ts", kind, "should print HTTP error"],
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
    """Require six canonical row deltas and 48,471 byte-identical lines."""
    require(lane in LANES, "unknown lane")
    require(len(targets) == 6, "target count differs")
    target_ids = {entry[0] for entry in targets.values()}
    require(len(target_ids) == 6, "target IDs duplicate")
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
            require(old.get("specId") not in target_ids, "target ID at wrong row")
            after[old["outcome"]] += 1
            if "evidenceScope" in old:
                after_scopes[old["evidenceScope"]] += 1
        count += 1
    require(
        count == expected_count and [item["index"] for item in changed] == sorted(targets),
        f"{lane} target positions or total row count differ",
    )
    expected_outcomes = before.copy()
    expected_outcomes["not-run"] -= 6
    expected_outcomes["expected"] += 6
    expected_scopes = before_scopes.copy()
    expected_scopes[SCOPE] += 6
    require(
        dict(before) == dict(prior_outcomes) and dict(before_scopes) == dict(prior_scopes),
        f"{lane} predecessor counts differ",
    )
    require(
        before["not-run"] >= 6 and after == expected_outcomes and after_scopes == expected_scopes,
        f"{lane} successor arithmetic differs",
    )
    return dict(sorted(after.items())), dict(sorted(after_scopes.items())), changed


def verify_inputs() -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    """Replay the immutable predecessor and both strict evidence sidecars."""
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
        / "scripts"
        / "verify_playwright_core_server_connect_timeout_redirect_bidi_chromium_paired_v6.py",
        PREDECESSOR_VERIFIER_SHA256,
        "predecessor verifier",
    )
    predecessor = verify_predecessor_bundle()
    require(
        predecessor["scope"] == PREDECESSOR_SCOPE
        and predecessor["recordsPerLane"] == COUNT
        and predecessor["publicApiParityEvidence"] is False
        and predecessor["browserExecutionEvidence"] is False
        and predecessor["bidiProtocolParityEvidence"] is False
        and predecessor["currentLivePackageEvidence"] is False,
        "predecessor identity/no-parity flags differ",
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
    strict_paths = {lane: capture.RESULT_PARENT / f"{lane}-v6.strict.json" for lane in LANES}
    for lane in LANES:
        sealed(strict_paths[lane], STRICT_SHA256[lane], f"{lane} strict sidecar")
    sealed(
        ROOT / "scripts/verify_playwright_core_server_connect_http_401_bidi_chromium_strict_v6.py",
        STRICT_VERIFIER_SHA256,
        "strict evidence verifier",
    )
    strict = {lane: verify_strict(strict_paths[lane]) for lane in LANES}
    verified_pair = verify_pair(strict_paths["oracle"], strict_paths["candidate"])
    require(
        verified_pair["candidateStatuses"] == ["expected"] * 6
        and verified_pair["publicApiParityEvidence"] is False
        and verified_pair["browserExecutionEvidence"] is False
        and verified_pair["bidiProtocolParityEvidence"] is False
        and verified_pair["currentLivePackageEvidence"] is False,
        "paired diagnostic outcome/no-parity flags differ",
    )
    sealed(
        capture.SNAPSHOT_MANIFEST,
        SNAPSHOT_MANIFEST_SHA256,
        "frozen candidate snapshot manifest",
    )
    sealed(
        capture.SNAPSHOT_PACKAGE / "native" / capture.PROFILE / "pwright.node",
        capture.ARTIFACT_SHA256,
        "frozen candidate addon",
    )
    candidate = strict["candidate"]
    oracle = strict["oracle"]
    require(
        candidate["candidateSourceSha256"] == CANDIDATE_SOURCE_SHA256
        and candidate["candidateSnapshotManifestSha256"] == SNAPSHOT_MANIFEST_SHA256
        and candidate["candidateArtifactSha256"] == capture.ARTIFACT_SHA256
        and candidate["candidateSnapshot"] == str(capture.SNAPSHOT_PACKAGE)
        and oracle["candidateArtifactSha256"] is None,
        "frozen candidate/oracle runtime provenance differs",
    )
    expected_ids = {entry[0] for entry in TARGETS.values()}
    for lane in LANES:
        sidecar = strict[lane]
        require(
            sidecar["sourceClassification"] == capture.CLASSIFICATION
            and sidecar["fixtureAdapted"] is True
            and sidecar["fixtureAdaptation"] == capture.FIXTURE_ADAPTATION
            and sidecar["receiptAttribution"] == "worker-count-only"
            and sidecar["publicApiParityEvidence"] is False
            and sidecar["browserExecutionEvidence"] is False
            and sidecar["bidiProjectNamesOnly"] is True
            and sidecar["bidiProtocolParityEvidence"] is False
            and sidecar["currentLivePackageEvidence"] is False
            and sidecar["promotedToCoreLedger"] is False
            and [run["project"] for run in sidecar["runs"]] == list(capture.PROJECTS),
            f"{lane} strict HTTP-401 envelope differs",
        )
        cases = [case for run in sidecar["runs"] for case in run["selected"]]
        require(
            len(cases) == 6
            and {case["specId"] for case in cases} == expected_ids
            and all(run["exitCode"] == 0 and len(run["selected"]) == 2 for run in sidecar["runs"])
            and all(
                case["status"] == "expected"
                and case["errors"] == []
                and len(case["attempts"]) == 1
                and case["attempts"][0]["retry"] == 0
                and case["attempts"][0]["status"] == "passed"
                for case in cases
            ),
            f"{lane} strict HTTP-401 result differs",
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
        "fixtureAdaptation": capture.FIXTURE_ADAPTATION,
        "fixtureAdapted": True,
        "receiptAttribution": "worker-count-only",
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


def verify_receipt_header(receipt: dict[str, Any], strict: Mapping[str, dict[str, Any]]) -> None:
    """Require the exact internal/fixture-adapted and no-parity envelope."""
    require(
        isinstance(receipt, dict) and set(receipt) == RECEIPT_FIELDS,
        "successor receipt field set differs",
    )
    require(
        {key: value for key, value in receipt.items() if key != "lanes"} == expected_header(strict)
        and isinstance(receipt["lanes"], dict)
        and set(receipt["lanes"]) == set(LANES),
        "successor receipt header or no-parity flags differ",
    )


def verify_payloads(
    files: Mapping[str, bytes],
    *,
    checked_inputs: tuple[dict[str, Any], dict[str, dict[str, Any]]] | None = None,
) -> dict[str, Any]:
    """Verify planned or published bytes without invoking a promoter."""
    require(set(files) == EXPECTED_FILES, "successor file set differs")
    predecessor, strict = checked_inputs or verify_inputs()
    receipt = json.loads(files["promotion-receipt.json"])
    verify_receipt_header(receipt, strict)
    require(
        files["promotion-receipt.json"]
        == (json.dumps(receipt, indent=2, sort_keys=True) + "\n").encode(),
        "successor receipt bytes are not canonical",
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
        expected_summary = {
            "schema": 1,
            "inventory": str(INVENTORY),
            "records": COUNT,
            "outcomes": outcomes,
            "explicitEvidenceScopes": scopes,
            "ledgerSha256": digest(new_bytes),
            "ledger": str(ledger_path),
        }
        require(
            summary == expected_summary
            and summary_bytes == (json.dumps(expected_summary, indent=2) + "\n").encode(),
            f"{lane} summary differs from independently counted canonical rows",
        )
        expected_runs = []
        for run in strict[lane]["runs"]:
            paths = capture.paths(lane, run["project"])
            expected_runs.append(
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
        expected_lane = {
            "base": str(PREDECESSOR / f"{lane}.jsonl"),
            "baseSha256": PREDECESSOR_LEDGER_SHA256[lane],
            "baseSummarySha256": PREDECESSOR_SUMMARY_SHA256[lane],
            "strictSidecar": str(capture.RESULT_PARENT / f"{lane}-v6.strict.json"),
            "strictSidecarSha256": STRICT_SHA256[lane],
            "officialRuns": expected_runs,
            "candidateArtifactSha256": strict[lane]["candidateArtifactSha256"],
            "candidateSnapshotManifestSha256": strict[lane]["candidateSnapshotManifestSha256"],
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
            f"{lane} receipt does not match verified evidence and row hashes",
        )
        require(
            predecessor["lanes"][lane]["ledgerSha256"] == PREDECESSOR_LEDGER_SHA256[lane],
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
    return verify_payloads({name: (SUCCESSOR / name).read_bytes() for name in EXPECTED_FILES})


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
        parser.exit(2, f"Bidi HTTP-401 paired successor invalid: {error}\n")
    print(
        json.dumps(
            {
                "scope": receipt["scope"],
                "recordsPerLane": receipt["recordsPerLane"],
                "outcomes": {lane: receipt["lanes"][lane]["outcomes"] for lane in LANES},
                "ledgerSha256": {lane: receipt["lanes"][lane]["ledgerSha256"] for lane in LANES},
            },
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
