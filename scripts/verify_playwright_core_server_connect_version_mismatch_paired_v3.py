#!/usr/bin/env python3
"""Verify the one-row Core version-mismatch successor independently of its promoter.

This is a fixture-adapted, implementation-internal diagnostic. A recorded
official assertion pass is not browser execution or public-API parity evidence.
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

from run_playwright_core_runserver_url_owner_v3 import verify_envelope

ROOT = Path(__file__).resolve().parents[1]
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
PREDECESSOR = (
    ROOT
    / ".work/official-conformance/outcomes"
    / "core-server-connect-ws-error-other-engines-strict-paired-v3"
)
EVIDENCE = (
    ROOT
    / ".work/official-conformance/results"
    / "core-server-connect-version-mismatch-url-owner-v3"
    / "strict-pair-20261020-v3"
)
SUCCESSOR = (
    ROOT
    / ".work/official-conformance/outcomes"
    / "core-server-connect-version-mismatch-url-owner-strict-paired-v3"
)
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
PREDECESSOR_RECEIPT_SHA256 = (
    "59c7cbb11ad7284ab8aedadb8c850cd4552ffaa0b035b70430c8b4e2e4ef0ef7"
)
PREDECESSOR_LEDGER_SHA256 = {
    "candidate": "e7d5924d9face0aaac68f5bf1db35516cb6d687871aa34c55361b961e69aee2d",
    "oracle": "3db929fbe661c35aee7f05c9ea4ae3353a0fc4dd249550a1ea43f918f1de8865",
}
PREDECESSOR_SUMMARY_SHA256 = {
    "candidate": "296b32931c67822a2b205db6b81b4200ff4f416fa57b5aade22e4a981a6ff474",
    "oracle": "3c6fd4b568c23776f76790156403ecac47503b31e2811139edb4c72919a2e4ae",
}
ENVELOPE_SHA256 = {
    "candidate": "11ea37b4233f799f73f02ff6e7a0836f0635aadd7dd7af8b42ed9f3b7f998f8b",
    "oracle": "54641e1250d83dcd657f9657b56549ecba80ffcfdf458d01863ead6f11ace230",
}
SUCCESSOR_LEDGER_SHA256 = {
    "candidate": "e8485c418287054c78c2b1331d2cd258103d556fb0aca2bf988a5ac738f17e04",
    "oracle": "6994207de60e01ad2e36dc296fe20922e39ab2483d574d02b77d42e18bb2fa3c",
}
LANES = ("candidate", "oracle")
COUNT = 48_477
TARGET_INDEX = 31_532
TARGET_ID = "ee6df4b3a2217bd356a7-d60bdc61fb009792457d"
SCOPE = "official-browser-free-server-version-mismatch-internal-diagnostic-green-v3"
SOURCE = {
    "classificationReasons": [
        "private-client-access",
        "runtime-source-import:packages/playwright-core/lib/server/utils/network",
        "runtime-source-import:packages/playwright-core/lib/server/utils/userAgent",
    ],
    "column": 1,
    "expectedStatus": "passed",
    "file": "library/browsertype-connect.spec.ts",
    "line": 1046,
    "projectId": "chromium-library",
    "projectName": "chromium-library",
    "role": "official-entrypoint",
    "sourceClassification": "implementation-internal",
    "sourceFile": "tests/library/browsertype-connect.spec.ts",
    "specId": TARGET_ID,
    "suite": "library",
    "titlePath": [
        "library/browsertype-connect.spec.ts",
        "should refuse connecting when versions do not match",
    ],
}
OUTCOMES = frozenset({"expected", "unexpected", "skipped", "not-run"})
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


def sha_file(file: Path) -> str:
    digest = hashlib.sha256()
    with file.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def sealed(file: Path, expected_sha256: str, label: str) -> None:
    require(file.is_file() and not file.is_symlink(), f"missing or linked {label}")
    require(sha_file(file) == expected_sha256, f"{label} SHA-256 drifted")


def object_json(raw: bytes, label: str) -> dict[str, Any]:
    value = json.loads(raw)
    require(isinstance(value, dict), f"{label} is not a JSON object")
    return value


def selected_official_report(report: dict[str, Any]) -> None:
    """Require exactly one clean attempt at the unchanged official assertion."""
    require(report.get("errors") == [], "official report has global errors")
    stats = report.get("stats")
    require(
        isinstance(stats, dict)
        and {
            key: stats.get(key)
            for key in ("expected", "unexpected", "flaky", "skipped")
        }
        == {"expected": 1, "unexpected": 0, "flaky": 0, "skipped": 0},
        "official report totals are not one clean expected result",
    )
    selected: list[dict[str, Any]] = []

    def visit(suite: object) -> None:
        require(isinstance(suite, dict), "official suite is malformed")
        specs = suite.get("specs", [])
        children = suite.get("suites", [])
        require(
            isinstance(specs, list) and isinstance(children, list),
            "official suite children are malformed",
        )
        for spec in specs:
            require(isinstance(spec, dict), "official spec is malformed")
            selected.append(spec)
        for child in children:
            visit(child)

    suites = report.get("suites")
    require(isinstance(suites, list), "official report suites are malformed")
    for suite in suites:
        visit(suite)
    require(len(selected) == 1, "official report does not contain one spec")
    spec = selected[0]
    tests = spec.get("tests")
    require(
        isinstance(tests, list) and len(tests) == 1, "official spec is not one test"
    )
    test = tests[0]
    require(isinstance(test, dict), "official test is malformed")
    attempts = test.get("results")
    require(
        isinstance(attempts, list) and len(attempts) == 1, "official test was retried"
    )
    attempt = attempts[0]
    require(isinstance(attempt, dict), "official attempt is malformed")
    require(
        spec.get("id") == TARGET_ID
        and spec.get("title") == SOURCE["titlePath"][1]
        and spec.get("line") == 1046
        and spec.get("column") == 1
        and spec.get("ok") is True
        and test.get("projectId") == "chromium-library"
        and test.get("expectedStatus") == "passed"
        and test.get("status") == "expected"
        and attempt.get("retry") == 0
        and attempt.get("status") == "passed"
        and attempt.get("errors") == [],
        "official assertion identity or outcome differs",
    )


def verify_evidence() -> dict[str, dict[str, Any]]:
    envelopes: dict[str, dict[str, Any]] = {}
    for lane in LANES:
        envelope_file = EVIDENCE / f"{lane}.runserver-url-owner-v3-attestation.json"
        sealed(envelope_file, ENVELOPE_SHA256[lane], f"{lane} v3 envelope")
        envelope = verify_envelope(envelope_file)
        require(
            envelope.get("mode") == lane
            and envelope.get("specId") == TARGET_ID
            and envelope.get("status") == "expected"
            and envelope.get("runnerExitCode") == 0,
            f"{lane} envelope does not attest an official pass",
        )
        result = Path(envelope["result"])
        sealed(result, envelope["resultSha256"], f"{lane} official report")
        selected_official_report(object_json(result.read_bytes(), f"{lane} report"))
        envelopes[lane] = envelope
    require(
        envelopes["candidate"]["manifestSha256"]
        == envelopes["oracle"]["manifestSha256"]
        and envelopes["candidate"]["sourceFileSha256"]
        == envelopes["oracle"]["sourceFileSha256"]
        and envelopes["candidate"]["harnessInputs"]
        == envelopes["oracle"]["harnessInputs"],
        "candidate/oracle official source or harness differs",
    )
    return envelopes


def verify_predecessor() -> dict[str, dict[str, Any]]:
    sealed(INVENTORY, INVENTORY_SHA256, "Core inventory")
    receipt_file = PREDECESSOR / "promotion-receipt.json"
    sealed(receipt_file, PREDECESSOR_RECEIPT_SHA256, "predecessor receipt")
    receipt = object_json(receipt_file.read_bytes(), "predecessor receipt")
    require(
        receipt.get("schema") == 1
        and receipt.get("inventory") == str(INVENTORY)
        and receipt.get("inventorySha256") == INVENTORY_SHA256
        and receipt.get("recordsPerLane") == COUNT
        and receipt.get("publicApiParityEvidence") is False
        and set(receipt.get("lanes", {})) == set(LANES),
        "predecessor identity differs",
    )
    summaries: dict[str, dict[str, Any]] = {}
    for lane in LANES:
        ledger = PREDECESSOR / f"{lane}.jsonl"
        summary_file = PREDECESSOR / f"{lane}.summary.json"
        sealed(ledger, PREDECESSOR_LEDGER_SHA256[lane], f"{lane} predecessor ledger")
        sealed(
            summary_file,
            PREDECESSOR_SUMMARY_SHA256[lane],
            f"{lane} predecessor summary",
        )
        summary = object_json(summary_file.read_bytes(), f"{lane} predecessor summary")
        lane_receipt = receipt["lanes"][lane]
        require(
            summary.get("schema") == 1
            and summary.get("inventory") == str(INVENTORY)
            and summary.get("records") == COUNT
            and summary.get("ledger") == str(ledger)
            and summary.get("ledgerSha256") == PREDECESSOR_LEDGER_SHA256[lane]
            and lane_receipt.get("ledger") == str(ledger)
            and lane_receipt.get("ledgerSha256") == PREDECESSOR_LEDGER_SHA256[lane]
            and lane_receipt.get("summarySha256") == PREDECESSOR_SUMMARY_SHA256[lane]
            and lane_receipt.get("outcomes") == summary.get("outcomes"),
            f"{lane} predecessor summary or receipt differs",
        )
        summaries[lane] = summary
    return summaries


def verify_successor_lines(
    inventory_lines: Iterable[bytes],
    predecessor_lines: Iterable[bytes],
    successor_lines: Iterable[bytes],
    *,
    lane: str,
    result_file: str,
    prior_outcomes: Mapping[str, int],
    prior_scopes: Mapping[str, int],
    expected_count: int = COUNT,
    target_index: int = TARGET_INDEX,
) -> tuple[dict[str, int], dict[str, int], dict[str, str]]:
    """Reject any non-target byte change and any extra target field."""
    require(lane in LANES, "unknown ledger lane")
    before: Counter[str] = Counter()
    after: Counter[str] = Counter()
    before_scopes: Counter[str] = Counter()
    after_scopes: Counter[str] = Counter()
    found = False
    count = 0
    row_hashes: dict[str, str] = {}
    for index, (source_raw, prior_raw, successor_raw) in enumerate(
        zip_longest(inventory_lines, predecessor_lines, successor_lines)
    ):
        require(
            source_raw is not None
            and prior_raw is not None
            and successor_raw is not None,
            f"{lane} row length differs at {index}",
        )
        require(
            source_raw.endswith(b"\n")
            and prior_raw.endswith(b"\n")
            and successor_raw.endswith(b"\n"),
            f"{lane} row {index} is unterminated",
        )
        source = object_json(source_raw, f"inventory row {index}")
        prior = object_json(prior_raw, f"{lane} predecessor row {index}")
        require(
            all(prior.get(key) == value for key, value in source.items())
            and prior.get("outcome") in OUTCOMES
            and type(prior.get("attempts")) is int
            and prior["attempts"] >= 0,
            f"{lane} predecessor source or outcome differs at {index}",
        )
        before[prior["outcome"]] += 1
        if "evidenceScope" in prior:
            require(
                isinstance(prior["evidenceScope"], str),
                f"{lane} predecessor scope malformed at {index}",
            )
            before_scopes[prior["evidenceScope"]] += 1
        if index == target_index:
            found = True
            require(
                source == SOURCE
                and prior == {**SOURCE, "attempts": 0, "outcome": "not-run"},
                f"{lane} target predecessor identity differs at {index}",
            )
            expected = {
                **prior,
                "attempts": 1,
                "candidateRuntimeRequired": lane == "candidate",
                "evidenceScope": SCOPE,
                "outcome": "expected",
                "resultFile": result_file,
            }
            expected_raw = (
                json.dumps(expected, sort_keys=True, separators=(",", ":")) + "\n"
            ).encode()
            require(
                successor_raw == expected_raw and successor_raw != prior_raw,
                f"{lane} target row {index} differs from exact allowed delta",
            )
            row_hashes = {
                "oldRowSha256": hashlib.sha256(prior_raw).hexdigest(),
                "newRowSha256": hashlib.sha256(successor_raw).hexdigest(),
            }
            after["expected"] += 1
            after_scopes[SCOPE] += 1
        else:
            require(
                prior_raw == successor_raw,
                f"{lane} non-target row {index} changed raw bytes",
            )
            require(
                prior.get("specId") != TARGET_ID,
                f"{lane} target ID appears at wrong row {index}",
            )
            after[prior["outcome"]] += 1
            if "evidenceScope" in prior:
                after_scopes[prior["evidenceScope"]] += 1
        count += 1
    require(count == expected_count and found, f"{lane} total rows or target differ")
    require(
        dict(before) == dict(prior_outcomes)
        and dict(before_scopes) == dict(prior_scopes),
        f"{lane} predecessor counts differ from sealed summary",
    )
    expected_outcomes = before.copy()
    expected_outcomes["not-run"] -= 1
    expected_outcomes["expected"] += 1
    expected_scopes = before_scopes.copy()
    expected_scopes[SCOPE] += 1
    require(
        before["not-run"] >= 1
        and after == expected_outcomes
        and after_scopes == expected_scopes,
        f"{lane} successor count arithmetic differs",
    )
    return (
        dict(sorted(after.items())),
        dict(sorted(after_scopes.items())),
        row_hashes,
    )


def relative_harness_inputs(envelope: Mapping[str, Any]) -> dict[str, str]:
    entries = envelope.get("harnessInputs")
    require(
        isinstance(entries, list) and bool(entries), "v3 harness input list missing"
    )
    result: dict[str, str] = {}
    for entry in entries:
        require(
            isinstance(entry, dict) and set(entry) == {"path", "sha256"},
            "v3 harness input entry malformed",
        )
        absolute = Path(entry["path"])
        require(absolute.is_absolute(), "v3 harness input path is not absolute")
        relative = absolute.relative_to(ROOT).as_posix()
        require(relative not in result, "v3 harness input repeats a path")
        checksum = entry["sha256"]
        require(
            isinstance(checksum, str)
            and len(checksum) == 64
            and all(char in "0123456789abcdef" for char in checksum),
            "v3 harness input SHA-256 malformed",
        )
        result[relative] = checksum
    return dict(sorted(result.items()))


def verify_receipt_header(
    receipt: dict[str, Any],
    envelopes: Mapping[str, dict[str, Any]],
    *,
    promoter_sha256: str,
    verifier_sha256: str,
) -> None:
    require(set(receipt) == RECEIPT_FIELDS, "successor receipt field set differs")
    harness = relative_harness_inputs(envelopes["candidate"])
    require(
        harness == relative_harness_inputs(envelopes["oracle"]),
        "candidate/oracle harness input maps differ",
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
        "selectedOfficialTestStartsRemoteServer": True,
        "upstreamAssertionsModified": False,
        "captureSidecarPromotedFlagAtCapture": False,
        "promoterSourceSha256": promoter_sha256,
        "pairedVerifierSha256": verifier_sha256,
        "harnessInputs": harness,
        "priorBundle": str(PREDECESSOR),
        "priorBundleReceiptSha256": PREDECESSOR_RECEIPT_SHA256,
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "recordsPerLane": COUNT,
        "promotedRowsPerLane": 1,
        "targetZeroBasedIndexes": [TARGET_INDEX],
        "targetSpecIds": [TARGET_ID],
    }
    require(
        {key: value for key, value in receipt.items() if key != "lanes"} == expected,
        "successor receipt identity or zero-parity flags drifted",
    )
    require(
        isinstance(receipt["lanes"], dict) and set(receipt["lanes"]) == set(LANES),
        "successor receipt lane set differs",
    )


def verify_lane_receipt(
    lane: str,
    lane_receipt: dict[str, Any],
    envelope: dict[str, Any],
    *,
    ledger_sha256: str,
    summary_sha256: str,
    row_hashes: Mapping[str, str],
    outcomes: Mapping[str, int],
) -> None:
    require(envelope.get("mode") == lane, f"{lane} evidence lane differs")
    candidate = lane == "candidate"
    expected = {
        "base": str(PREDECESSOR / f"{lane}.jsonl"),
        "baseSha256": PREDECESSOR_LEDGER_SHA256[lane],
        "baseSummarySha256": PREDECESSOR_SUMMARY_SHA256[lane],
        "strictEnvelope": str(
            EVIDENCE / f"{lane}.runserver-url-owner-v3-attestation.json"
        ),
        "strictEnvelopeSha256": ENVELOPE_SHA256[lane],
        "rawProvenance": envelope["rawProvenance"],
        "rawProvenanceSha256": envelope["rawProvenanceSha256"],
        "ownerReceipt": envelope["ownerReceipt"],
        "ownerReceiptSha256": envelope["ownerReceiptSha256"],
        "officialResult": envelope["result"],
        "officialResultSha256": envelope["resultSha256"],
        "candidateArtifactSha256": (
            envelope["snapshot"]["addonSha256"] if candidate else None
        ),
        "ledger": str(SUCCESSOR / f"{lane}.jsonl"),
        "ledgerSha256": ledger_sha256,
        "summarySha256": summary_sha256,
        "oldOutcome": "not-run",
        "newOutcome": "expected",
        "promotedRows": 1,
        "oldRowSha256": row_hashes["oldRowSha256"],
        "newRowSha256": row_hashes["newRowSha256"],
        "outcomes": dict(outcomes),
    }
    require(
        lane_receipt == expected,
        f"{lane} receipt does not match independently verified evidence",
    )


def verify_existing() -> dict[str, Any]:
    if sys.flags.optimize:
        raise RuntimeError(
            "paired Core verification requires Python assertions enabled"
        )
    require(
        SUCCESSOR.is_dir() and not SUCCESSOR.is_symlink(),
        "immutable successor directory missing or linked",
    )
    expected_names = {
        "candidate.jsonl",
        "candidate.summary.json",
        "oracle.jsonl",
        "oracle.summary.json",
        "promotion-receipt.json",
    }
    require(
        {entry.name for entry in SUCCESSOR.iterdir()} == expected_names,
        "successor file set differs",
    )
    for name in expected_names:
        require(
            (SUCCESSOR / name).is_file() and not (SUCCESSOR / name).is_symlink(),
            f"successor {name} missing or linked",
        )
    prior = verify_predecessor()
    envelopes = verify_evidence()
    receipt = object_json(
        (SUCCESSOR / "promotion-receipt.json").read_bytes(), "successor receipt"
    )
    promoter = (
        ROOT
        / "scripts/promote_playwright_core_server_connect_version_mismatch_paired_v3.py"
    )
    require(promoter.is_file() and not promoter.is_symlink(), "paired promoter missing")
    verify_receipt_header(
        receipt,
        envelopes,
        promoter_sha256=sha_file(promoter),
        verifier_sha256=sha_file(Path(__file__).resolve()),
    )
    for lane in LANES:
        ledger = SUCCESSOR / f"{lane}.jsonl"
        summary_file = SUCCESSOR / f"{lane}.summary.json"
        with (
            INVENTORY.open("rb") as inventory_stream,
            (PREDECESSOR / f"{lane}.jsonl").open("rb") as prior_stream,
            ledger.open("rb") as successor_stream,
        ):
            outcomes, scopes, row_hashes = verify_successor_lines(
                inventory_stream,
                prior_stream,
                successor_stream,
                lane=lane,
                result_file=envelopes[lane]["result"],
                prior_outcomes=prior[lane]["outcomes"],
                prior_scopes=prior[lane]["explicitEvidenceScopes"],
            )
        sealed(ledger, SUCCESSOR_LEDGER_SHA256[lane], f"{lane} successor ledger")
        summary = object_json(summary_file.read_bytes(), f"{lane} successor summary")
        require(
            summary
            == {
                "schema": 1,
                "inventory": str(INVENTORY),
                "records": COUNT,
                "outcomes": outcomes,
                "explicitEvidenceScopes": scopes,
                "ledgerSha256": SUCCESSOR_LEDGER_SHA256[lane],
                "ledger": str(ledger),
            },
            f"{lane} successor summary differs from exact row accounting",
        )
        lane_receipt = receipt["lanes"][lane]
        require(isinstance(lane_receipt, dict), f"{lane} receipt is malformed")
        verify_lane_receipt(
            lane,
            lane_receipt,
            envelopes[lane],
            ledger_sha256=SUCCESSOR_LEDGER_SHA256[lane],
            summary_sha256=sha_file(summary_file),
            row_hashes=row_hashes,
            outcomes=outcomes,
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
        parser.exit(2, f"Core version-mismatch paired successor invalid: {error}\n")
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
