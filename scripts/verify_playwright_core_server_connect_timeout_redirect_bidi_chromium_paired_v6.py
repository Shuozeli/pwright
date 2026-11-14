"""Independently verify the five-row Bidi Core timeout/redirect successor.

The official bodies are fixture-adapted implementation-internal diagnostics.
This verifier never treats the result as browser, BiDi, current-package, or
public-API parity evidence, and it does not import the promoter.
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

import run_playwright_core_server_connect_timeout_redirect_bidi_chromium_strict_v1 as capture
from verify_playwright_core_cdp_close_bidi_chromium_paired_v5 import (
    verify_existing as verify_predecessor_bundle,
)
from verify_playwright_core_server_connect_timeout_redirect_bidi_chromium_strict_v1 import (
    verify_pair,
)

ROOT = Path(__file__).resolve().parents[1]
OUTCOMES = ROOT / ".work/official-conformance/outcomes"
PREDECESSOR = OUTCOMES / "core-cdp-close-bidi-chromium-strict-paired-v5"
SUCCESSOR = OUTCOMES / "core-server-connect-timeout-redirect-bidi-chromium-strict-paired-v6"
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
PROMOTER = (
    ROOT
    / "scripts/promote_playwright_core_server_connect_timeout_redirect_bidi_chromium_strict_v6.py"
)
MUTATION_TEST = (
    ROOT
    / "scripts"
    / "test_verify_playwright_core_server_connect_timeout_redirect_bidi_chromium_paired_v6.py"
)
COUNT = 48_477
LANES = ("candidate", "oracle")
SCOPE = (
    "official-browser-free-server-connect-timeout-redirect-"
    "bidi-chromium-internal-diagnostic-green-v6"
)
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
PREDECESSOR_RECEIPT_SHA256 = "13d7df7ccf58fb1838c5f523471a77fadd3b7ad92330d086f13c8d3ac64df068"
PREDECESSOR_VERIFIER_SHA256 = "51f53dc68c41e2546e444f22994667b6cf409d886827e957cb838c22e6244b77"
PREDECESSOR_LEDGER_SHA256 = {
    "candidate": "0358d4210f90ab8cc941bcec7ac5711cee196eca195e08f26460f775eb9bbc25",
    "oracle": "f82cda1e9aab54936a0343ffd2d4d13c5da90a4aa0922c0c0cd69453dd26b835",
}
PREDECESSOR_SUMMARY_SHA256 = {
    "candidate": "9abad396b758382e21980f845dffb61b4a8e589befc552917a5902ea8259c611",
    "oracle": "fce4940d9dda54184ca827aea2488bf9a3956d5cedda31c34de7bea3b80f74c9",
}
STRICT_SHA256 = {
    "candidate": "7da683bf33205b1b6aa52f2749b4084d340cab9b2bd2fd24531a260c1cb6a35b",
    "oracle": "9a385742a0894f12c585a7212d72d27ae1ebd67d19dcf65cf161dfa2c48fa4b1",
}
STRICT_VERIFIER_SHA256 = "4fa9b9d1cc356554df9046bb34af7c89e3b88cc0b560e1f9d30e4013a8205ad3"
SNAPSHOT_MANIFEST_SHA256 = "a79c22bc1d160cc46379806820ee10d06ad4762aee3c037522a798cb129599b8"
CANDIDATE_SOURCE_SHA256 = "bddad5646a880efc80a94d7bf4b975e4eb06691562ac6602a8df29c3cf97afe2"
TARGETS = {
    5436: (
        "ee6df4b3a2217bd356a7-5817a5ed954cfa532ace",
        221,
        5,
        "launchServer",
        "should timeout in socket while connecting",
    ),
    5437: (
        "ee6df4b3a2217bd356a7-5b07f2a5e757a16ed468",
        221,
        5,
        "run-server",
        "should timeout in socket while connecting",
    ),
    5448: (
        "ee6df4b3a2217bd356a7-30b6e0f369ffebc5d854",
        228,
        5,
        "launchServer",
        "should timeout in connect while connecting",
    ),
    5449: (
        "ee6df4b3a2217bd356a7-50b361500ef280984f89",
        228,
        5,
        "run-server",
        "should timeout in connect while connecting",
    ),
    5956: (
        "ee6df4b3a2217bd356a7-27eeb64a07b3673ac59b",
        1056,
        1,
        "standalone",
        "should timeout after redirect when connecting over http",
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


def target_source(spec_id: str, line: int, column: int, kind: str, title: str) -> dict[str, Any]:
    title_path = ["library/browsertype-connect.spec.ts"]
    if kind != "standalone":
        title_path.append(kind)
    title_path.append(title)
    return {
        "classificationReasons": CLASSIFICATION_REASONS,
        "column": column,
        "expectedStatus": "passed",
        "file": "library/browsertype-connect.spec.ts",
        "line": line,
        "projectId": capture.PROJECT,
        "projectName": capture.PROJECT,
        "role": "official-entrypoint",
        "sourceClassification": "implementation-internal",
        "sourceFile": capture.SOURCE,
        "specId": spec_id,
        "suite": "bidi",
        "titlePath": title_path,
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
    targets: Mapping[int, tuple[str, int, int, str, str]] = TARGETS,
) -> tuple[dict[str, int], dict[str, int], list[dict[str, Any]]]:
    """Allow exactly five canonical row changes and no other byte changes."""
    require(lane in LANES, "unknown lane")
    require(len(targets) == 5, "target count differs")
    target_ids = {entry[0] for entry in targets.values()}
    require(len(target_ids) == 5, "target IDs duplicate")
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
            spec_id, line, column, kind, title = targets[index]
            require(
                source == target_source(spec_id, line, column, kind, title)
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
    expected_outcomes["not-run"] -= 5
    expected_outcomes["expected"] += 5
    expected_scopes = before_scopes.copy()
    expected_scopes[SCOPE] += 5
    require(
        dict(before) == dict(prior_outcomes) and dict(before_scopes) == dict(prior_scopes),
        f"{lane} predecessor counts differ",
    )
    require(
        before["not-run"] >= 5 and after == expected_outcomes and after_scopes == expected_scopes,
        f"{lane} successor arithmetic differs",
    )
    return dict(sorted(after.items())), dict(sorted(after_scopes.items())), changed


def verify_inputs() -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    """Replay the immutable predecessor and raw strict pair before promotion."""
    if sys.flags.optimize:
        raise RuntimeError("paired Core verification requires assertions enabled")
    sealed(INVENTORY, INVENTORY_SHA256, "Core inventory")
    sealed(
        PREDECESSOR / "promotion-receipt.json",
        PREDECESSOR_RECEIPT_SHA256,
        "predecessor receipt",
    )
    sealed(
        ROOT / "scripts/verify_playwright_core_cdp_close_bidi_chromium_paired_v5.py",
        PREDECESSOR_VERIFIER_SHA256,
        "predecessor verifier",
    )
    predecessor = verify_predecessor_bundle()
    require(
        predecessor["scope"]
        == "official-browser-free-cdp-close-bidi-chromium-internal-diagnostic-green-v5"
        and predecessor["recordsPerLane"] == COUNT
        and predecessor["publicApiParityEvidence"] is False
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
    strict_paths = {lane: capture.paths(lane)["strict"] for lane in LANES}
    for lane in LANES:
        sealed(strict_paths[lane], STRICT_SHA256[lane], f"{lane} strict sidecar")
    sealed(
        ROOT
        / "scripts"
        / "verify_playwright_core_server_connect_timeout_redirect_bidi_chromium_strict_v1.py",
        STRICT_VERIFIER_SHA256,
        "strict evidence verifier",
    )
    oracle, candidate = verify_pair(strict_paths["oracle"], strict_paths["candidate"])
    strict = {"candidate": candidate, "oracle": oracle}
    sealed(
        capture.SNAPSHOT_MANIFEST,
        SNAPSHOT_MANIFEST_SHA256,
        "frozen candidate snapshot manifest",
    )
    sealed(
        capture.SNAPSHOT / "native" / capture.PROFILE / "pwright.node",
        capture.ARTIFACT_SHA256,
        "frozen candidate addon",
    )
    require(
        candidate["candidateSourceSha256"] == CANDIDATE_SOURCE_SHA256
        and candidate["candidateSnapshotManifestSha256"] == SNAPSHOT_MANIFEST_SHA256
        and candidate["candidateArtifactSha256"] == capture.ARTIFACT_SHA256
        and candidate["candidateSnapshot"] == str(capture.SNAPSHOT)
        and oracle["candidateArtifactSha256"] is None,
        "frozen candidate/oracle runtime provenance differs",
    )
    expected_ids = {entry[0] for entry in TARGETS.values()}
    for lane in LANES:
        sidecar = strict[lane]
        run = sidecar["run"]
        cases = run["selected"]
        require(
            sidecar["sourceClassification"] == "implementation-internal"
            and sidecar["fixtureAdapted"] is True
            and sidecar["fixtureAdaptation"] == capture.FIXTURE_ADAPTATION
            and sidecar["publicApiParityEvidence"] is False
            and sidecar["realBrowserServer"] is False
            and sidecar["promotedToCoreLedger"] is False
            and run["project"] == capture.PROJECT
            and run["suite"] == capture.SUITE
            and run["exitCode"] == 0
            and len(cases) == 5
            and {case["specId"] for case in cases} == expected_ids
            and all(
                case["status"] == "expected"
                and case["errors"] == []
                and len(case["attempts"]) == 1
                and case["attempts"][0]["retry"] == 0
                and case["attempts"][0]["status"] == "passed"
                for case in cases
            ),
            f"{lane} strict timeout/redirect evidence differs",
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
        "promotedRowsPerLane": 5,
        "targetZeroBasedIndexes": list(TARGETS),
        "targetSpecIds": [entry[0] for entry in TARGETS.values()],
    }


def verify_receipt_header(receipt: dict[str, Any], strict: Mapping[str, dict[str, Any]]) -> None:
    """Require the exact diagnostic/no-parity receipt envelope."""
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
        run = strict[lane]["run"]
        result_by_id = {case["specId"]: run["result"] for case in run["selected"]}
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
        paths = capture.paths(lane)
        expected_runs = [
            {
                "project": run["project"],
                "result": run["result"],
                "resultSha256": run["resultSha256"],
                "rawProvenance": str(paths["provenance"]),
                "rawProvenanceSha256": run["provenanceSha256"],
                "guardReceipt": str(paths["receipt"]),
                "guardReceiptSha256": run["guardReceiptSha256"],
            }
        ]
        expected_lane = {
            "base": str(PREDECESSOR / f"{lane}.jsonl"),
            "baseSha256": PREDECESSOR_LEDGER_SHA256[lane],
            "baseSummarySha256": PREDECESSOR_SUMMARY_SHA256[lane],
            "strictSidecar": str(paths["strict"]),
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
            "promotedRows": 5,
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
        parser.exit(2, f"Bidi timeout/redirect paired successor invalid: {error}\n")
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
