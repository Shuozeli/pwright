#!/usr/bin/env python3
"""Independently verify the immutable four-case Core WS-error ledger successor.

These Firefox/WebKit rows are implementation-internal diagnostics. A passing
official assertion here is not evidence of public API or browser parity.
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

import run_playwright_core_server_connect_ws_error_other_engines_strict_v3 as capture
from verify_playwright_core_server_connect_ws_error_other_engines_strict_v3 import (
    verify_pair,
)

ROOT = Path(__file__).resolve().parents[1]
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
PREDECESSOR = (
    ROOT
    / ".work/official-conformance/outcomes/core-dummy-node-other-engines-strict-paired-v1"
)
OUTPUT = (
    ROOT
    / ".work/official-conformance/outcomes/core-server-connect-ws-error-other-engines-strict-paired-v3"
)
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
PREDECESSOR_RECEIPT_SHA256 = (
    "27131a3e31ff27d3a7dd676f97581c8fcdf09e19a76647d3ffc83d689edf15c5"
)
PREDECESSOR_LEDGER_SHA256 = {
    "candidate": "d0ad535d17c595414bca06f7f81c33515cf92b12d28fb025f915e9482e7fa8fb",
    "oracle": "c176a3e55da3556ff17c263b71044079ba5c467ca5925464ca03f514b8e41b56",
}
PREDECESSOR_SUMMARY_SHA256 = {
    "candidate": "c62f53b66968f11b3278ddb8c8e20a3aa3a6d0077cad04eaeec040c0d3f18277",
    "oracle": "ab690f080fb2b962c6d67c4083ccb7d5446cac1e15b22d5ae1cb591ac53b348b",
}
STRICT_SHA256 = {
    "candidate": "dbea2e573720908777738c9fc97f893659d144b59c37bd47c5368f3f57347241",
    "oracle": "6235e35e8ac6bbe31b3fa71c5f9f0cac947ef085979c00dcb5197defca10f332",
}
COUNT = 48_477
LANES = ("candidate", "oracle")
TARGETS = {
    31_234: (
        "ee6df4b3a2217bd356a7-7ff2dc18d1e08d985682",
        "firefox-library",
        "launchServer",
    ),
    31_235: (
        "ee6df4b3a2217bd356a7-95a86c47485eb83c0016",
        "firefox-library",
        "run-server",
    ),
    31_236: (
        "ee6df4b3a2217bd356a7-184cfa77707c96da434b",
        "webkit-library",
        "run-server",
    ),
    31_237: (
        "ee6df4b3a2217bd356a7-6d5f8a4e2638a54cc171",
        "webkit-library",
        "launchServer",
    ),
}
CLASSIFICATION_REASONS = [
    "private-client-access",
    "runtime-source-import:packages/playwright-core/lib/server/utils/network",
    "runtime-source-import:packages/playwright-core/lib/server/utils/userAgent",
]
SCOPE = "official-browser-free-server-connect-ws-error-internal-diagnostic-green-v3-other-engines"
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
    "harnessInputs",
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


def sha_file(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def regular(path: Path, label: str) -> None:
    require(path.is_file() and not path.is_symlink(), f"missing or linked {label}")


def sealed(path: Path, expected: str, label: str) -> None:
    require(
        len(expected) == 64 and all(char in "0123456789abcdef" for char in expected),
        f"invalid {label} SHA-256 pin",
    )
    regular(path, label)
    require(sha_file(path) == expected, f"{label} SHA-256 drifted")


def object_json(raw: bytes, label: str) -> dict[str, Any]:
    value = json.loads(raw)
    require(isinstance(value, dict), f"{label} is not a JSON object")
    return value


def selected_results(strict: dict[str, Any]) -> dict[str, str]:
    results: dict[str, str] = {}
    for run in strict["runs"]:
        require(
            run["project"] in capture.PROJECTS
            and run["suite"] == capture.PROJECTS[run["project"]]["suite"],
            "strict project identity drifted",
        )
        for item in run["selected"]:
            spec_id = item["specId"]
            require(spec_id not in results, "strict evidence repeats a spec ID")
            require(
                item["project"] == run["project"]
                and item["kind"] == capture.PROJECTS[run["project"]]["specs"][spec_id]
                and item["line"] == 122
                and item["title"] == "should print ws error"
                and item["status"] == "expected"
                and item["errors"] == []
                and item["attempts"]
                == [{"retry": 0, "status": "passed", "workerIndex": 0}],
                "strict evidence is not a single clean official pass",
            )
            results[spec_id] = run["result"]
    require(
        set(results) == {spec_id for spec_id, _, _ in TARGETS.values()},
        "strict evidence did not select exactly the four target IDs",
    )
    return results


def target_source(
    source: dict[str, Any],
    prior: dict[str, Any],
    index: int,
    target: tuple[str, str, str],
) -> None:
    spec_id, project, kind = target
    source_file = "tests/library/browsertype-connect.spec.ts"
    file = "library/browsertype-connect.spec.ts"
    require(
        source.get("specId") == spec_id
        and source.get("projectId") == source.get("projectName") == project
        and source.get("sourceFile") == source_file
        and source.get("file") == file
        and source.get("line") == 122
        and source.get("column") == 5
        and source.get("titlePath") == [file, kind, "should print ws error"]
        and source.get("sourceClassification") == "implementation-internal"
        and source.get("classificationReasons") == CLASSIFICATION_REASONS
        and source.get("role") == "official-entrypoint"
        and source.get("suite") == "library"
        and source.get("expectedStatus") == "passed"
        and prior == {**source, "attempts": 0, "outcome": "not-run"},
        f"target predecessor row {index} is not the exact unrun internal case",
    )


def verify_successor_lines(
    inventory_lines: Iterable[bytes],
    predecessor_lines: Iterable[bytes],
    successor_lines: Iterable[bytes],
    *,
    lane: str,
    result_by_spec: Mapping[str, str],
    prior_outcomes: Mapping[str, int],
    prior_scopes: Mapping[str, int],
    expected_count: int = COUNT,
    targets: Mapping[int, tuple[str, str, str]] = TARGETS,
) -> tuple[dict[str, int], dict[str, int]]:
    """Check an exact four-row raw-byte delta without trusting the promoter."""
    require(lane in LANES, "unknown ledger lane")
    require(
        set(result_by_spec) == {spec_id for spec_id, _, _ in targets.values()},
        "strict result IDs differ from ledger targets",
    )
    before: Counter[str] = Counter()
    after: Counter[str] = Counter()
    before_scopes: Counter[str] = Counter()
    after_scopes: Counter[str] = Counter()
    seen: set[int] = set()
    rows = 0
    for index, (inventory_raw, prior_raw, successor_raw) in enumerate(
        zip_longest(inventory_lines, predecessor_lines, successor_lines)
    ):
        require(
            inventory_raw is not None
            and prior_raw is not None
            and successor_raw is not None,
            f"{lane} row count differs at {index}",
        )
        require(
            inventory_raw.endswith(b"\n")
            and prior_raw.endswith(b"\n")
            and successor_raw.endswith(b"\n"),
            f"{lane} row {index} is unterminated",
        )
        source = object_json(inventory_raw, f"inventory row {index}")
        prior = object_json(prior_raw, f"predecessor row {index}")
        require(
            all(prior.get(key) == value for key, value in source.items())
            and prior.get("outcome") in {"expected", "unexpected", "skipped", "not-run"}
            and type(prior.get("attempts")) is int
            and prior["attempts"] >= 0,
            f"{lane} predecessor metadata/outcome drift at row {index}",
        )
        before[prior["outcome"]] += 1
        if "evidenceScope" in prior:
            require(
                isinstance(prior["evidenceScope"], str),
                f"{lane} invalid predecessor evidence scope at row {index}",
            )
            before_scopes[prior["evidenceScope"]] += 1
        if index in targets:
            seen.add(index)
            target = targets[index]
            target_source(source, prior, index, target)
            spec_id = target[0]
            expected = {
                **prior,
                "attempts": 1,
                "candidateRuntimeRequired": lane == "candidate",
                "evidenceScope": SCOPE,
                "outcome": "expected",
                "resultFile": result_by_spec[spec_id],
            }
            encoded = (
                json.dumps(expected, sort_keys=True, separators=(",", ":")) + "\n"
            ).encode()
            require(
                successor_raw == encoded,
                f"{lane} target row {index} differs from the exact allowed delta",
            )
            require(
                successor_raw != prior_raw,
                f"{lane} target row {index} did not change",
            )
            after["expected"] += 1
            after_scopes[SCOPE] += 1
        else:
            require(
                prior_raw == successor_raw,
                f"{lane} non-target row {index} changed raw bytes",
            )
            require(
                prior["specId"] not in result_by_spec,
                f"{lane} target spec ID appeared at wrong row {index}",
            )
            after[prior["outcome"]] += 1
            if "evidenceScope" in prior:
                after_scopes[prior["evidenceScope"]] += 1
        rows += 1
    require(
        rows == expected_count and seen == set(targets), "target row set/count drifted"
    )
    require(
        dict(before) == dict(prior_outcomes)
        and dict(before_scopes) == dict(prior_scopes),
        f"{lane} predecessor summary differs from ledger contents",
    )
    expected_outcomes = before.copy()
    expected_outcomes["not-run"] -= len(targets)
    expected_outcomes["expected"] += len(targets)
    expected_scopes = before_scopes.copy()
    expected_scopes[SCOPE] += len(targets)
    require(
        before["not-run"] >= len(targets)
        and after == expected_outcomes
        and after_scopes == expected_scopes,
        f"{lane} successor outcome/scope arithmetic differs",
    )
    return dict(sorted(after.items())), dict(sorted(after_scopes.items()))


def verify_predecessor() -> dict[str, dict[str, Any]]:
    sealed(INVENTORY, INVENTORY_SHA256, "Core inventory")
    receipt_path = PREDECESSOR / "promotion-receipt.json"
    sealed(receipt_path, PREDECESSOR_RECEIPT_SHA256, "predecessor receipt")
    receipt = object_json(receipt_path.read_bytes(), "predecessor receipt")
    require(
        receipt.get("schema") == 1
        and receipt.get("scope")
        == "official-browser-free-browsertype-dummy-node-other-engines-strict-red-v1"
        and receipt.get("inventory") == str(INVENTORY)
        and receipt.get("inventorySha256") == INVENTORY_SHA256
        and receipt.get("recordsPerLane") == COUNT
        and receipt.get("targetZeroBasedIndexes") == [31_614, 31_615]
        and receipt.get("fixtureAdapted") is True
        and receipt.get("publicApiParityEvidence") is False
        and set(receipt.get("lanes", {})) == set(LANES),
        "predecessor chain identity drifted",
    )
    summaries: dict[str, dict[str, Any]] = {}
    for lane in LANES:
        ledger = PREDECESSOR / f"{lane}.jsonl"
        summary_path = PREDECESSOR / f"{lane}.summary.json"
        sealed(ledger, PREDECESSOR_LEDGER_SHA256[lane], f"{lane} predecessor ledger")
        sealed(
            summary_path,
            PREDECESSOR_SUMMARY_SHA256[lane],
            f"{lane} predecessor summary",
        )
        summary = object_json(summary_path.read_bytes(), f"{lane} predecessor summary")
        lane_receipt = receipt["lanes"][lane]
        require(
            set(summary)
            == {
                "schema",
                "inventory",
                "records",
                "outcomes",
                "explicitEvidenceScopes",
                "ledgerSha256",
                "ledger",
            }
            and summary["schema"] == 1
            and summary["inventory"] == str(INVENTORY)
            and summary["records"] == COUNT
            and summary["ledger"] == str(ledger)
            and summary["ledgerSha256"] == PREDECESSOR_LEDGER_SHA256[lane]
            and lane_receipt["ledger"] == str(ledger)
            and lane_receipt["ledgerSha256"] == PREDECESSOR_LEDGER_SHA256[lane]
            and lane_receipt["summarySha256"] == PREDECESSOR_SUMMARY_SHA256[lane]
            and lane_receipt["outcomes"] == summary["outcomes"],
            f"{lane} predecessor summary/receipt drifted",
        )
        summaries[lane] = summary
    return summaries


def verify_receipt_header(
    receipt: dict[str, Any],
    strict: Mapping[str, dict[str, Any]],
    *,
    promoter_sha: str,
    paired_verifier_sha: str,
    strict_verifier_sha: str,
) -> None:
    require(set(receipt) == RECEIPT_FIELDS, "successor receipt field set differs")
    require(
        strict["candidate"]["inputs"] == strict["oracle"]["inputs"],
        "strict pair harness inputs differ",
    )
    expected = {
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
        "promoterSourceSha256": promoter_sha,
        "strictVerifierSha256": strict_verifier_sha,
        "pairedVerifierSha256": paired_verifier_sha,
        "harnessInputs": strict["candidate"]["inputs"],
        "priorBundle": str(PREDECESSOR),
        "priorBundleReceiptSha256": PREDECESSOR_RECEIPT_SHA256,
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "recordsPerLane": COUNT,
        "promotedRowsPerLane": len(TARGETS),
        "targetZeroBasedIndexes": sorted(TARGETS),
        "targetSpecIds": [TARGETS[index][0] for index in sorted(TARGETS)],
    }
    require(
        {key: value for key, value in receipt.items() if key != "lanes"} == expected,
        "successor receipt identity/parity flags drifted",
    )
    require(
        isinstance(receipt["lanes"], dict) and set(receipt["lanes"]) == set(LANES),
        "successor receipt lane set differs",
    )


def verify_lane_receipt(
    lane: str,
    receipt: dict[str, Any],
    strict: dict[str, Any],
    ledger_sha: str,
    summary_sha: str,
    outcomes: Mapping[str, int],
) -> None:
    runs = strict["runs"]
    expected = {
        "base": str(PREDECESSOR / f"{lane}.jsonl"),
        "baseSha256": PREDECESSOR_LEDGER_SHA256[lane],
        "strictSidecar": str(capture.RESULT_PARENT / f"{lane}-v3.strict.json"),
        "strictSidecarSha256": STRICT_SHA256[lane],
        "officialResults": [
            {
                "project": run["project"],
                "result": run["result"],
                "resultSha256": run["resultSha256"],
            }
            for run in runs
        ],
        "candidateArtifactSha256": strict["candidateArtifactSha256"],
        "candidateSnapshotManifestSha256": strict["candidateSnapshotManifestSha256"],
        "candidateSourceSha256": strict["candidateSourceSha256"],
        "ledger": str(OUTPUT / f"{lane}.jsonl"),
        "ledgerSha256": ledger_sha,
        "summarySha256": summary_sha,
        "oldOutcome": "not-run",
        "newOutcome": "expected",
        "promotedRows": len(TARGETS),
        "outcomes": dict(outcomes),
    }
    require(
        receipt == expected,
        f"{lane} successor receipt differs from independently verified evidence",
    )


def verify_existing() -> dict[str, Any]:
    if sys.flags.optimize:
        raise RuntimeError(
            "paired Core verification requires Python assertions enabled"
        )
    require(OUTPUT.is_dir() and not OUTPUT.is_symlink(), "immutable successor missing")
    expected_names = {
        "candidate.jsonl",
        "candidate.summary.json",
        "oracle.jsonl",
        "oracle.summary.json",
        "promotion-receipt.json",
    }
    require(
        {path.name for path in OUTPUT.iterdir()} == expected_names,
        "successor contains missing or extra files",
    )
    for name in expected_names:
        regular(OUTPUT / name, f"successor {name}")
    previous = verify_predecessor()
    strict_paths = {
        lane: capture.RESULT_PARENT / f"{lane}-v3.strict.json" for lane in LANES
    }
    for lane, path in strict_paths.items():
        sealed(path, STRICT_SHA256[lane], f"{lane} strict sidecar")
    oracle, candidate = verify_pair(strict_paths["oracle"], strict_paths["candidate"])
    strict = {"candidate": candidate, "oracle": oracle}
    results = {lane: selected_results(value) for lane, value in strict.items()}
    receipt_path = OUTPUT / "promotion-receipt.json"
    receipt = object_json(receipt_path.read_bytes(), "successor receipt")
    promoter = (
        ROOT
        / "scripts/promote_playwright_core_server_connect_ws_error_other_engines_strict_v3.py"
    )
    strict_verifier = (
        ROOT
        / "scripts/verify_playwright_core_server_connect_ws_error_other_engines_strict_v3.py"
    )
    regular(promoter, "v3 paired promoter")
    regular(strict_verifier, "v3 strict verifier")
    verify_receipt_header(
        receipt,
        strict,
        promoter_sha=sha_file(promoter),
        paired_verifier_sha=sha_file(Path(__file__).resolve()),
        strict_verifier_sha=sha_file(strict_verifier),
    )
    for lane in LANES:
        ledger = OUTPUT / f"{lane}.jsonl"
        summary_path = OUTPUT / f"{lane}.summary.json"
        with (
            INVENTORY.open("rb") as inventory_stream,
            (PREDECESSOR / f"{lane}.jsonl").open("rb") as predecessor_stream,
            ledger.open("rb") as successor_stream,
        ):
            outcomes, scopes = verify_successor_lines(
                inventory_stream,
                predecessor_stream,
                successor_stream,
                lane=lane,
                result_by_spec=results[lane],
                prior_outcomes=previous[lane]["outcomes"],
                prior_scopes=previous[lane]["explicitEvidenceScopes"],
            )
        ledger_sha = sha_file(ledger)
        summary = object_json(summary_path.read_bytes(), f"{lane} successor summary")
        require(
            summary
            == {
                "schema": 1,
                "inventory": str(INVENTORY),
                "records": COUNT,
                "outcomes": outcomes,
                "explicitEvidenceScopes": scopes,
                "ledgerSha256": ledger_sha,
                "ledger": str(ledger),
            },
            f"{lane} successor summary differs from exact row counts",
        )
        verify_lane_receipt(
            lane,
            receipt["lanes"][lane],
            strict[lane],
            ledger_sha,
            sha_file(summary_path),
            outcomes,
        )
    return receipt


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    try:
        receipt = verify_existing()
    except (
        OSError,
        ValueError,
        KeyError,
        TypeError,
        IndexError,
        AssertionError,
    ) as error:
        parser.exit(2, f"strict paired Core WS-error successor invalid: {error}\n")
    print(
        json.dumps(
            {"scope": receipt["scope"], "lanes": receipt["lanes"]},
            indent=2,
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
