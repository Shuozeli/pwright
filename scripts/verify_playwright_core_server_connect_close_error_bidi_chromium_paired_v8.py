#!/usr/bin/env python3
"""Independently replay the six-row Bidi custom-Close Core successor.

The promoter is never imported. Every non-target raw line remains identical to
the sealed 48,477-row v7 predecessor, and no parity credit is inferred.
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

import run_playwright_core_server_connect_close_error_bidi_chromium_strict_v6 as capture
from verify_playwright_core_server_connect_close_error_bidi_chromium_strict_v6 import (
    verify_pair,
    verify_strict,
)
from verify_playwright_core_server_connect_http_401_bidi_chromium_paired_v7 import (
    verify_existing as verify_predecessor_bundle,
)

ROOT = Path(__file__).resolve().parents[1]
OUTCOMES = ROOT / ".work/official-conformance/outcomes"
PREDECESSOR = OUTCOMES / "core-server-connect-http-401-bidi-chromium-strict-paired-v7"
SUCCESSOR = OUTCOMES / "core-server-connect-close-error-bidi-chromium-strict-paired-v8"
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
PROMOTER = (
    ROOT / "scripts/promote_playwright_core_server_connect_close_error_bidi_chromium_strict_v8.py"
)
MUTATION_TEST = (
    ROOT
    / "scripts/test_verify_playwright_core_server_connect_close_error_bidi_chromium_paired_v8.py"
)
COUNT = 48_477
LANES = ("candidate", "oracle")
SCOPE = (
    "official-browser-free-server-connect-close-error-bidi-chromium-"
    "fixture-host-adapted-internal-diagnostic-green-v8"
)
PREDECESSOR_SCOPE = (
    "official-browser-free-server-connect-http-401-bidi-chromium-"
    "fixture-host-adapted-internal-diagnostic-green-v7"
)
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
PREDECESSOR_RECEIPT_SHA256 = "ef2b8a1d16fd3758c784071e50777df0b35935e04b9c1a3eaa8bf3a60a339f9a"
PREDECESSOR_VERIFIER_SHA256 = "c9c7972fb96fab7d03e17ee614984357c101770531bcd30fdd34bc28a8128cc4"
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
SNAPSHOT_MANIFEST_SHA256 = "bb901e4bac312364d6f1af762053e63e8a3dccf3fc7e1785c5d7c2c7d3524c5b"
CANDIDATE_SOURCE_SHA256 = "1bf7e4d67e226a1666af2abff31db4960781a641bf28d3b392d4c0834c0c95b4"
TARGETS = {
    5360: (
        "ee6df4b3a2217bd356a7-511a6ea814b661f15f2e",
        "bidi-chrome-canary-library",
        "launchServer",
    ),
    5361: (
        "ee6df4b3a2217bd356a7-82f4b2b4a00d4a3c199b",
        "bidi-chrome-canary-library",
        "run-server",
    ),
    5362: (
        "ee6df4b3a2217bd356a7-2b2c19e6a3745a17907d",
        "bidi-chrome-stable-library",
        "launchServer",
    ),
    5363: (
        "ee6df4b3a2217bd356a7-f0142f43a589fa23adf3",
        "bidi-chrome-stable-library",
        "run-server",
    ),
    5364: (
        "ee6df4b3a2217bd356a7-5977d0c5bef040f1bf03",
        "bidi-chromium-library",
        "launchServer",
    ),
    5365: (
        "ee6df4b3a2217bd356a7-9c774e5f960e4e3f4a21",
        "bidi-chromium-library",
        "run-server",
    ),
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
        "line": 130,
        "projectId": project,
        "projectName": project,
        "role": "official-entrypoint",
        "sourceClassification": "implementation-internal",
        "sourceFile": capture.SOURCE,
        "specId": spec_id,
        "suite": "bidi",
        "titlePath": [
            "library/browsertype-connect.spec.ts",
            kind,
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
    targets: Mapping[int, tuple[str, str, str]] = TARGETS,
) -> tuple[dict[str, int], dict[str, int], list[dict[str, Any]]]:
    """Require six canonical deltas and identical bytes for all other lines."""
    require(lane in LANES, "unknown lane")
    require(len(targets) == 6, "target count differs")
    ids = {entry[0] for entry in targets.values()}
    require(len(ids) == 6 and set(results_by_id) == ids, "target/result IDs differ")
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
            promoted = {
                **old,
                "attempts": 1,
                "candidateRuntimeRequired": lane == "candidate",
                "evidenceScope": SCOPE,
                "outcome": "expected",
                "resultFile": results_by_id[spec_id],
            }
            canonical = (
                json.dumps(promoted, sort_keys=True, separators=(",", ":")) + "\n"
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
            require(old.get("specId") not in ids, "target ID at wrong row")
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


def require_green_strict(lane: str, sidecar: dict[str, Any], expected_ids: set[str]) -> None:
    require(
        sidecar["sourceClassification"] == "official-derived-fixture-host-adapted-internal"
        and sidecar["fixtureAdaptation"] == capture.FIXTURE_ADAPTATION
        and sidecar["unadaptedOfficialPass"] is False
        and sidecar["publicApiParityEvidence"] is False
        and sidecar["browserExecutionEvidence"] is False
        and sidecar["bidiProtocolParityEvidence"] is False
        and sidecar["currentLivePackageEvidence"] is False
        and sidecar["promotedToCoreLedger"] is False
        and sidecar["perSpecTransportAttribution"]
        == "ordered-project-receipts-only-not-proven-per-spec"
        and [run["project"] for run in sidecar["runs"]] == list(capture.PROJECTS),
        f"{lane} strict Close envelope differs",
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
        f"{lane} strict Close result is RED or incomplete",
    )


def verify_inputs() -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    """Validate sealed v7, sealed v6 capture, and frozen v5 candidate."""
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
        / "verify_playwright_core_server_connect_http_401_bidi_chromium_paired_v7.py",
        PREDECESSOR_VERIFIER_SHA256,
        "predecessor verifier",
    )
    predecessor = verify_predecessor_bundle()
    require(
        predecessor["scope"] == PREDECESSOR_SCOPE
        and predecessor["recordsPerLane"] == COUNT
        and predecessor["sourceClassificationPreserved"] == "implementation-internal"
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
        ROOT
        / "scripts"
        / "verify_playwright_core_server_connect_close_error_bidi_chromium_strict_v6.py",
        STRICT_VERIFIER_SHA256,
        "strict evidence verifier",
    )
    strict = {lane: verify_strict(strict_paths[lane]) for lane in LANES}
    oracle, candidate = verify_pair(strict_paths["oracle"], strict_paths["candidate"])
    require(
        oracle == strict["oracle"] and candidate == strict["candidate"],
        "strict pair disagrees with sidecars",
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
    require(
        candidate["candidateSourceSha256"] == CANDIDATE_SOURCE_SHA256
        and candidate["candidateSnapshotManifestSha256"] == SNAPSHOT_MANIFEST_SHA256
        and candidate["candidateArtifactSha256"] == capture.ARTIFACT_SHA256
        and candidate["candidateSnapshot"] == str(capture.SNAPSHOT_PACKAGE)
        and oracle["candidateArtifactSha256"] is None,
        "frozen candidate/oracle provenance differs",
    )
    expected_ids = {entry[0] for entry in TARGETS.values()}
    require(
        strict["candidate"]["inputs"] == strict["oracle"]["inputs"],
        "oracle/candidate harness inputs differ",
    )
    for lane in LANES:
        sidecar = strict[lane]
        require_green_strict(lane, sidecar, expected_ids)
        for run in sidecar["runs"]:
            receipts = capture.paths(lane, run["project"])["receipt"]
            require(
                receipts.is_file() and not receipts.is_symlink(),
                f"{lane} {run['project']} receipt missing or linked",
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
    """Verify planned or published bytes without calling the promoter."""
    require(set(files) == EXPECTED_FILES, "successor file set differs")
    predecessor, strict = checked_inputs or verify_inputs()
    receipt = json.loads(files["promotion-receipt.json"])
    verify_receipt_header(receipt, strict)
    require(
        files["promotion-receipt.json"]
        == (json.dumps(receipt, indent=2, sort_keys=True) + "\n").encode(),
        "successor receipt bytes are not canonical",
    )
    with INVENTORY.open("rb") as stream:
        inventory_lines = stream.readlines()
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
        expected_summary = {
            "schema": 1,
            "inventory": str(INVENTORY),
            "records": COUNT,
            "outcomes": outcomes,
            "explicitEvidenceScopes": scopes,
            "ledgerSha256": digest(new_bytes),
            "ledger": str(SUCCESSOR / f"{lane}.jsonl"),
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
            "ledger": str(SUCCESSOR / f"{lane}.jsonl"),
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
            f"{lane} receipt does not match evidence and row hashes",
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
        parser.exit(2, f"Bidi Close paired successor invalid: {error}\n")
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
