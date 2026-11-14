"""Independently verify the immutable Core dialog candidate-only v16 successor.

The v15 RED, unchanged Oracle bytes, clean candidate snapshot, and new complete
official candidate report remain independently sealed evidence. This module
never imports the v16 publisher.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

import verify_playwright_core_page_dialog10_strict_paired_v15 as predecessor
import verify_playwright_official_provenance as provenance_verifier


ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / ".work/official-conformance"
V15 = predecessor.V15
V16 = WORK / "outcomes/core-page-dialog10-strict-paired-v16"
PROMOTER = ROOT / "scripts/promote_playwright_core_page_dialog10_strict_paired_v16.py"
PREDECESSOR_VERIFIER = (
    ROOT / "scripts/verify_playwright_core_page_dialog10_strict_paired_v15.py"
)
PROVENANCE_VERIFIER = ROOT / "scripts/verify_playwright_official_provenance.py"
REPORT = WORK / "results/core-page-dialog10-candidate-sync-clean-20261021-v1.json"
PROVENANCE = REPORT.with_suffix(".provenance.json")
PACKAGE = WORK / "builds/core-dialog10-evaluation-clean-20261021-v1/candidate"
PROFILE = "official-conformance-page-check"
ARTIFACT = PACKAGE / f"native/{PROFILE}/pwright.node"
COUNT = predecessor.COUNT
TARGET_COUNT = predecessor.TARGET_COUNT
RED_ID = predecessor.RED_ID
RED_INDEX = 38_610
LANES = predecessor.LANES
BUNDLE_NAMES = predecessor.BUNDLE_NAMES
SCOPE = "official-browser-backed-page-dialog-full-file-paired-v16"
PREDECESSOR_VERIFIER_SHA256 = (
    "c00ef5d7432de553031558660a11acd31b1fa2eef0dd7b1c9b25090d7aad3995"
)
PROVENANCE_VERIFIER_SHA256 = (
    "9b1c6c4edee469c3c7a56866444fb5ad1bafe31443164f497c562a8eda1e0672"
)
V15_SHA256 = {
    "candidate.jsonl": "50cc75b00dff2e0d89569084b27ac84cc1899942e23c8ac7c9287d79acf08916",
    "candidate.summary.json": "5a85e65023cad40b2f611d137975f12774730bf93ed8c7c473f91c34fd3c173e",
    "oracle.jsonl": "f1b1f4f3bd9aa733cd26047b4e23eaab56003173667a3ca3facdf7080bea6125",
    "oracle.summary.json": "8f974aea8a0103e8dd8514e6b185088bc234f51d88cd32862bb538a572f3872b",
    "promotion-receipt.json": "b44fc60ac36d0db4b646b319bf2a430f6240511185aebcc74154857dac1d8110",
}
REPORT_SHA256 = "0b08a1542fbbf980987ca8e196db833986ec6220c5da4115fa9a9f9dd0db607a"
PROVENANCE_SHA256 = "a0e49e48d937dbb44f1baab32db43c2fcb46e339a8c048535b7d949147e425e5"
ARTIFACT_SHA256 = "9be17a4945465b2355f9a1ee3f3ed8996ce6a5cf0b5596535ac1b56dc2e56a93"
SOURCE_SHA256 = predecessor.predecessor.CANDIDATE_SOURCE_SHA256


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sealed(path: Path, sha256: str, label: str) -> bytes:
    require(path.is_file() and not path.is_symlink(), f"{label} absent or symlink")
    data = path.read_bytes()
    require(digest(data) == sha256, f"{label} SHA-256 changed")
    return data


def object_from(data: bytes, label: str) -> dict[str, Any]:
    value = json.loads(data)
    require(isinstance(value, dict), f"{label} is not an object")
    return value


def canonical_line(row: dict[str, Any]) -> bytes:
    return (json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n").encode()


def report_rows(data: bytes) -> dict[str, dict[str, Any]]:
    report = object_from(data, "new candidate official report")
    stats = report.get("stats")
    require(
        isinstance(stats, dict)
        and all(
            stats.get(key) == value
            for key, value in {"expected": 10, "unexpected": 0, "skipped": 0, "flaky": 0}.items()
        ),
        "new candidate report is not 10/10 GREEN",
    )
    suites = report.get("suites")
    require(isinstance(suites, list) and len(suites) == 1, "report suite count differs")
    specs: list[dict[str, Any]] = []

    def visit(suite: dict[str, Any]) -> None:
        require(isinstance(suite, dict), "report suite malformed")
        children, entries = suite.get("suites", []), suite.get("specs", [])
        require(
            isinstance(children, list) and isinstance(entries, list),
            "report suite children malformed",
        )
        for spec in entries:
            require(isinstance(spec, dict), "report spec malformed")
            specs.append(spec)
        for child in children:
            visit(child)

    for suite in suites:
        visit(suite)
    require(len(specs) == TARGET_COUNT, "report is not complete 10-case file")
    target_ids = {spec_id for _, spec_id, _ in predecessor.TARGETS}
    rows: dict[str, dict[str, Any]] = {}
    for spec in specs:
        spec_id, tests = spec.get("id"), spec.get("tests")
        require(
            spec.get("file") == predecessor.FILE
            and isinstance(spec_id, str)
            and spec_id in target_ids
            and spec_id not in rows
            and isinstance(tests, list)
            and len(tests) == 1
            and isinstance(tests[0], dict),
            "report has wrong, missing, or duplicate invocation",
        )
        test = tests[0]
        attempts = test.get("results")
        require(
            test.get("projectId") == predecessor.PROJECT
            and test.get("expectedStatus") == "passed"
            and test.get("status") == "expected"
            and isinstance(attempts, list)
            and len(attempts) == 1
            and isinstance(attempts[0], dict)
            and attempts[0].get("status") == "passed"
            and attempts[0].get("retry") == 0
            and not attempts[0].get("error")
            and not attempts[0].get("errors"),
            f"report invocation is not a clean pass for {spec_id}",
        )
        rows[spec_id] = {"outcome": "expected", "attempts": 1}
    require(set(rows) == target_ids, "report target IDs differ")
    return rows


def verify_new_provenance(data: bytes) -> dict[str, Any]:
    payload = object_from(data, "new candidate provenance")
    require(
        payload.get("schema") == 2
        and payload.get("mode") == "candidate"
        and payload.get("suite") == predecessor.SUITE
        and payload.get("suiteRole") == "browser-backed-official-file"
        and payload.get("adapter") == "test-mode-playwright-core"
        and payload.get("project") == predecessor.PROJECT
        and payload.get("commit") == predecessor.COMMIT
        and payload.get("manifest") == str(predecessor.MANIFEST)
        and payload.get("manifestSha256") == predecessor.MANIFEST_SHA256
        and payload.get("endpoint") == "http://127.0.0.1:9222"
        and payload.get("fixtureHost") == "macbookpro-yuacx.tail8f3b66.ts.net"
        and payload.get("fixtureBind") == "100.78.97.63"
        and payload.get("filters") == ["tests/page/page-dialog.spec.ts"]
        and payload.get("workers") == 1
        and payload.get("grep") is None
        and payload.get("headed") is False
        and payload.get("exitCode") == 0
        and payload.get("result") == str(REPORT)
        and payload.get("resultSha256") == REPORT_SHA256
        and payload.get("candidate") == str(PACKAGE)
        and payload.get("candidateProfile") == PROFILE
        and payload.get("candidateArtifact") == str(ARTIFACT)
        and payload.get("candidateArtifactSha256") == ARTIFACT_SHA256
        and payload.get("candidateSourceSha256") == SOURCE_SHA256,
        "new candidate provenance identity or runtime pin differs",
    )
    require(PACKAGE.is_dir() and not PACKAGE.is_symlink(), "clean package missing or symlink")
    require(
        all(not item.is_symlink() for item in PACKAGE.rglob("*")),
        "clean package contains a symlink",
    )
    sealed(ARTIFACT, ARTIFACT_SHA256, "clean native artifact")
    provenance_verifier.verify_provenance(payload, provenance_path=PROVENANCE)
    return payload


def expected_red_row(old: dict[str, Any]) -> dict[str, Any]:
    require(
        old.get("specId") == RED_ID
        and old.get("attempts") == 1
        and old.get("outcome") == "unexpected"
        and old.get("evidenceScope") == predecessor.SCOPE
        and isinstance(old.get("error"), str)
        and "timeout" in old["error"].lower(),
        "historical v15 candidate RED row differs",
    )
    row = dict(old)
    row.pop("error")
    row.update(
        {
            "attempts": 2,
            "outcome": "expected",
            "resultFile": str(REPORT),
            "evidenceScope": SCOPE,
            "fixtureAdapted": True,
            "browserExecutionEvidence": True,
            "publicApiParityEvidence": False,
        }
    )
    return row


def expected_candidate_summary(old: dict[str, Any], ledger: bytes) -> dict[str, Any]:
    outcomes = dict(old["outcomes"])
    scopes = dict(old["explicitEvidenceScopes"])
    require(
        scopes.get(predecessor.SCOPE) == TARGET_COUNT and SCOPE not in scopes,
        "v15 candidate evidence scope differs",
    )
    outcomes["expected"] += 1
    outcomes["unexpected"] -= 1
    scopes[predecessor.SCOPE] -= 1
    scopes[SCOPE] = 1
    require(sum(outcomes.values()) == COUNT, "candidate summary arithmetic differs")
    return {
        "schema": 1,
        "inventory": str(predecessor.INVENTORY),
        "records": COUNT,
        "outcomes": dict(sorted(outcomes.items())),
        "explicitEvidenceScopes": dict(sorted(scopes.items())),
        "ledgerSha256": digest(ledger),
        "ledger": str(V16 / "candidate.jsonl"),
    }


def verify_inputs() -> dict[str, Any]:
    sealed(PREDECESSOR_VERIFIER, PREDECESSOR_VERIFIER_SHA256, "v15 independent verifier")
    sealed(PROVENANCE_VERIFIER, PROVENANCE_VERIFIER_SHA256, "Core provenance verifier")
    old_receipt = predecessor.verify_existing()
    old_files = {
        name: sealed(V15 / name, sha, f"v15 {name}") for name, sha in V15_SHA256.items()
    }
    require(
        old_receipt.get("scope") == predecessor.SCOPE
        and old_receipt.get("candidateExpectedRows") == 9
        and old_receipt.get("candidateUnexpectedRows") == 1
        and old_receipt.get("oracleExpectedRows") == 10
        and old_receipt.get("oracleUnexpectedRows") == 0
        and old_receipt.get("redComparison", {}).get("candidateOnlySpecIds") == [RED_ID]
        and old_receipt.get("fullCorpusExecution") is False
        and old_receipt.get("publicApiParityEvidence") is False,
        "v15 historical RED receipt differs",
    )
    require(
        (RED_INDEX, RED_ID, 67) in predecessor.TARGETS,
        "source-pinned historical RED index differs",
    )
    old_lines = {
        lane: old_files[f"{lane}.jsonl"].splitlines(keepends=True) for lane in LANES
    }
    require(
        all(len(lines) == COUNT for lines in old_lines.values()),
        "v15 ledger row count differs",
    )
    old_summaries = {
        lane: object_from(old_files[f"{lane}.summary.json"], f"v15 {lane} summary")
        for lane in LANES
    }
    expected_red_row(object_from(old_lines["candidate"][RED_INDEX], "v15 RED row"))
    report = report_rows(sealed(REPORT, REPORT_SHA256, "new candidate official report"))
    provenance = sealed(PROVENANCE, PROVENANCE_SHA256, "new candidate provenance")
    verify_new_provenance(provenance)
    require(report[RED_ID] == {"outcome": "expected", "attempts": 1}, "RED did not pass")
    return {
        "oldFiles": old_files,
        "oldLines": old_lines,
        "oldSummaries": old_summaries,
        "oldReceipt": old_receipt,
        "report": report,
    }


def expected_lane_receipt(
    lane: str, files: dict[str, bytes], checked: dict[str, Any]
) -> dict[str, Any]:
    require(lane in LANES, "unknown lane")
    changed: list[dict[str, Any]] = []
    if lane == "candidate":
        old_raw = checked["oldLines"][lane][RED_INDEX]
        new_raw = files[f"{lane}.jsonl"].splitlines(keepends=True)[RED_INDEX]
        changed = [
            {
                "index": RED_INDEX,
                "specId": RED_ID,
                "oldRowSha256": digest(old_raw),
                "newRowSha256": digest(new_raw),
                "oldOutcome": "unexpected",
                "newOutcome": "expected",
                "oldAttempts": 1,
                "newAttempts": 2,
            }
        ]
    summary = object_from(files[f"{lane}.summary.json"], f"v16 {lane} summary")
    official_result = REPORT if lane == "candidate" else predecessor.report_path(lane)
    provenance = PROVENANCE if lane == "candidate" else predecessor.provenance_path(lane)
    return {
        "base": str(V15 / f"{lane}.jsonl"),
        "baseSha256": V15_SHA256[f"{lane}.jsonl"],
        "baseSummarySha256": V15_SHA256[f"{lane}.summary.json"],
        "ledger": str(V16 / f"{lane}.jsonl"),
        "ledgerSha256": digest(files[f"{lane}.jsonl"]),
        "summarySha256": digest(files[f"{lane}.summary.json"]),
        "summaryReferencesHistoricalV15Ledger": lane == "oracle",
        "officialResult": str(official_result),
        "officialResultSha256": REPORT_SHA256 if lane == "candidate" else predecessor.REPORT_SHA256[lane],
        "provenance": str(provenance),
        "provenanceSha256": PROVENANCE_SHA256 if lane == "candidate" else predecessor.PROVENANCE_SHA256[lane],
        "changedRows": changed,
        "promotedRows": len(changed),
        "unchangedRawRows": COUNT - len(changed),
        "expectedRows": TARGET_COUNT,
        "unexpectedRows": 0,
        "outcomes": summary["outcomes"],
    }


def expected_receipt(files: dict[str, bytes], checked: dict[str, Any]) -> dict[str, Any]:
    require(PROMOTER.is_file() and not PROMOTER.is_symlink(), "publisher source absent or symlink")
    return {
        "schema": 1,
        "scope": SCOPE,
        "promotionKind": "browser-backed-public-api-full-file-candidate-only-red-to-green",
        "priorBundle": str(V15),
        "priorBundleReceiptSha256": V15_SHA256["promotion-receipt.json"],
        "inventory": str(predecessor.INVENTORY),
        "inventorySha256": predecessor.INVENTORY_SHA256,
        "manifest": str(predecessor.MANIFEST),
        "manifestSha256": predecessor.MANIFEST_SHA256,
        "upstreamSourceSha256": predecessor.SOURCE_SHA256,
        "targetSpecIdsSha256": predecessor.SPEC_IDS_SHA256,
        "recordsPerLane": COUNT,
        "targetZeroBasedIndexes": [index for index, _, _ in predecessor.TARGETS],
        "changedRowsPerLane": {"candidate": 1, "oracle": 0},
        "unchangedRawRowsPerLane": {"candidate": COUNT - 1, "oracle": COUNT},
        "candidateOnlyRerun": True,
        "oracleJsonlAndSummaryByteIdenticalToV15": True,
        "fixtureAdapted": True,
        "browserExecutionEvidence": True,
        "completeFileExecution": True,
        "fullCorpusExecution": False,
        "publicApiParityEvidence": False,
        "upstreamAssertionsModified": False,
        "candidateExpectedRows": 10,
        "candidateUnexpectedRows": 0,
        "oracleExpectedRows": 10,
        "oracleUnexpectedRows": 0,
        "historicalRedSpecId": RED_ID,
        "historicalRedRowSha256": digest(checked["oldLines"]["candidate"][RED_INDEX]),
        "redComparison": {
            "candidateOnlySpecIds": [],
            "bothRedSpecIds": [],
            "oracleOnlySpecIds": [],
            "candidateOnlyCount": 0,
            "bothRedCount": 0,
            "oracleOnlyCount": 0,
        },
        "candidateProfile": PROFILE,
        "candidateArtifactSha256": ARTIFACT_SHA256,
        "candidateSourceSha256": SOURCE_SHA256,
        "candidatePackage": str(PACKAGE),
        "promoterSourceSha256": digest(PROMOTER.read_bytes()),
        "verifierSourceSha256": digest(Path(__file__).read_bytes()),
        "lanes": {lane: expected_lane_receipt(lane, files, checked) for lane in LANES},
    }


def verify_payloads(
    files: dict[str, bytes], checked: dict[str, Any] | None = None
) -> dict[str, Any]:
    require(set(files) == BUNDLE_NAMES, "v16 bundle files differ")
    inputs = checked if checked is not None else verify_inputs()
    for lane in LANES:
        old_lines = inputs["oldLines"][lane]
        new_lines = files[f"{lane}.jsonl"].splitlines(keepends=True)
        require(len(new_lines) == COUNT, f"v16 {lane} row count differs")
        for index, (old_raw, new_raw) in enumerate(zip(old_lines, new_lines, strict=True)):
            if lane == "candidate" and index == RED_INDEX:
                expected = canonical_line(expected_red_row(object_from(old_raw, "v15 RED row")))
                require(new_raw == expected, "v16 candidate RED target row differs")
            else:
                require(new_raw == old_raw, f"v16 {lane} off-target raw row {index} changed")
    candidate_summary = object_from(files["candidate.summary.json"], "v16 candidate summary")
    require(
        candidate_summary
        == expected_candidate_summary(
            inputs["oldSummaries"]["candidate"], files["candidate.jsonl"]
        ),
        "v16 candidate summary differs",
    )
    require(
        files["oracle.summary.json"] == inputs["oldFiles"]["oracle.summary.json"],
        "v16 Oracle summary bytes changed",
    )
    require(
        files["oracle.jsonl"] == inputs["oldFiles"]["oracle.jsonl"],
        "v16 Oracle ledger bytes changed",
    )
    receipt = object_from(files["promotion-receipt.json"], "v16 receipt")
    require(receipt == expected_receipt(files, inputs), "v16 receipt differs")
    return receipt


def verify_existing() -> dict[str, Any]:
    require(V16.is_dir() and not V16.is_symlink(), "v16 bundle is absent or symlink")
    require({path.name for path in V16.iterdir()} == BUNDLE_NAMES, "v16 file set differs")
    require(
        all((V16 / name).is_file() and not (V16 / name).is_symlink() for name in BUNDLE_NAMES),
        "v16 file absent or symlink",
    )
    return verify_payloads({name: (V16 / name).read_bytes() for name in BUNDLE_NAMES})


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify-existing", action="store_true", required=True)
    parser.parse_args()
    try:
        receipt = verify_existing()
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
        parser.exit(2, f"strict paired Core dialog v16 verification rejected: {error}\n")
    print(json.dumps({"verified": True, "scope": receipt["scope"], "destination": str(V16)}, sort_keys=True))


if __name__ == "__main__":
    main()
