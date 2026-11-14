"""Independently verify the immutable frameElement candidate-only v21 pair.

The candidate's first complete rerun failed during frameset navigation. Three
later complete reruns passed. All four source-bound runs are mandatory evidence;
the v21 ledger promotes the latest run without claiming stability or parity.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path
from typing import Any

import verify_playwright_core_frame_frame_element6_strict_paired_v19 as frame_source
import verify_playwright_core_selectors_misc21_light_strict_paired_v20 as predecessor
import verify_playwright_official_provenance as provenance_verifier

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / ".work/official-conformance"
V20 = predecessor.V20
V21 = WORK / "outcomes/core-frame-frame-element6-rust-owner-strict-paired-v21"
PROMOTER = (
    ROOT
    / "scripts/promote_playwright_core_frame_frame_element6_rust_owner_strict_paired_v21.py"
)
PREDECESSOR_VERIFIER = (
    ROOT / "scripts/verify_playwright_core_selectors_misc21_light_strict_paired_v20.py"
)
PROVENANCE_VERIFIER = ROOT / "scripts/verify_playwright_official_provenance.py"
PACKAGE = WORK / "builds/core-frame-frame-element6-rust-owner-v1/candidate"
PROFILE = "official-conformance-page-frame-element"
ARTIFACT = PACKAGE / f"native/{PROFILE}/pwright.node"
COUNT = frame_source.COUNT
TARGET_COUNT = frame_source.TARGET_COUNT
LANES = frame_source.LANES
BUNDLE_NAMES = frame_source.BUNDLE_NAMES
SCOPE = "official-browser-backed-frame-frame-element-rust-owner-full-file-candidate-only-v21"
FRAMESET_ID = "320fa7c1239dab0b2b33-ad52ed21af9213d68b5b"
FIRST_FAILURE = "Error: page.goto: Target page, context or browser has been closed"
PREDECESSOR_VERIFIER_SHA256 = (
    "0fb545b51d337550dc4b51452bc1b462c097bdbd0570c1a198d6b22bceebed6a"
)
PROVENANCE_VERIFIER_SHA256 = (
    "9b1c6c4edee469c3c7a56866444fb5ad1bafe31443164f497c562a8eda1e0672"
)
V20_SHA256 = {
    "candidate.jsonl": "2453c74e80e5de5321e1d5b6a1d42cd6c1992cc7714ff76d0bb834848ecc3c8f",
    "candidate.summary.json": "11f21a83f8b984aac8f20464172e368b9021edec41182b5a1373d8296944398f",
    "oracle.jsonl": "e9b921670232979db5d0fb3b6dfb045aad5d747c5f63f1385dfe372a9155c2a6",
    "oracle.summary.json": "921186214ade8d718a6b2d4211db02cbdd3a15b602254a86c8ce3b51aa378a0b",
    "promotion-receipt.json": "2889851a3db6106bb726018ade7b8f2f96e72852496d40123ea825a80f3e5bd9",
}
REPORT_SHA256 = {
    1: "96048cd84832c56a9b456f5c250234ee4b9389e27f589615f7b9346fe7e786ff",
    2: "178ffd01bf2ab0750de4145998a284defb4c6e906fe1afcb5a2f873d21386bec",
    3: "25e21e881b8ca078bc8b0e0721aed3067a28d437d21f48f646fa4615a47466e7",
    4: "8c04d2aa0f11063d9260ea336f66511f9b809574ea1e9788c2a356f52e87f2b4",
}
PROVENANCE_SHA256 = {
    1: "9dc53510a5f5e691a3381a1dcb391be8bc9381b324112e4259d1bc0eec770d3d",
    2: "43da6c861a0e6f7da7a88a848197905dca0ad63d3e47ce68ea59979553a62d15",
    3: "db080a7304806b1977dd6260d026bc82d7e581a75aa1b3145e5bfc20d0093e13",
    4: "0c3e758aab72c7d2d2853c4385d7cf6b0b2ada1cf17594e843918c9b01e68494",
}
PACKAGE_MANIFEST_SHA256 = (
    "40b301caa57ef941fd2293bd1becfdbb0cf513d5634edb43883fb251af1fb458"
)
ARTIFACT_SHA256 = "440ec056a5cfceeacbf851c698ee24c33c3db063ad92262c215d996a568db6a0"
CANDIDATE_SOURCE_SHA256 = (
    "150ba50a46ca5690923ce7897be1c1cf5391f9cbcf94ee184a6f675ed6524567"
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sealed(path: Path, expected: str, label: str) -> bytes:
    require(
        len(expected) == 64 and path.is_file() and not path.is_symlink(),
        f"{label} absent, symlink, or unpinned",
    )
    data = path.read_bytes()
    require(digest(data) == expected, f"{label} SHA-256 changed")
    return data


def object_from(data: bytes, label: str) -> dict[str, Any]:
    value = json.loads(data)
    require(isinstance(value, dict), f"{label} is not an object")
    return value


def canonical_line(row: dict[str, Any]) -> bytes:
    return (json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n").encode()


def report_path(run: int) -> Path:
    require(run in REPORT_SHA256, "unknown run")
    return (
        WORK
        / f"results/core-frame-frame-element6-rust-owner-candidate-20261021-v{run}.json"
    )


def provenance_path(run: int) -> Path:
    return report_path(run).with_suffix(".provenance.json")


def report_rows(
    run: int, data: bytes, target_ids: set[str]
) -> dict[str, dict[str, Any]]:
    require(run in REPORT_SHA256, "unknown run")
    report = object_from(data, f"candidate v{run} report")
    stats = report.get("stats")
    expected, unexpected = (5, 1) if run == 1 else (6, 0)
    require(
        isinstance(stats, dict)
        and all(
            stats.get(key) == value
            for key, value in {
                "expected": expected,
                "unexpected": unexpected,
                "skipped": 0,
                "flaky": 0,
            }.items()
        )
        and report.get("errors") == [],
        f"candidate v{run} report totals or top-level errors differ",
    )
    suites = report.get("suites")
    require(
        isinstance(suites, list) and len(suites) == 1,
        f"candidate v{run} suite count differs",
    )
    specs: list[dict[str, Any]] = []

    def visit(raw: object) -> None:
        require(isinstance(raw, dict), "malformed report suite")
        children, entries = raw.get("suites", []), raw.get("specs", [])
        require(
            isinstance(children, list) and isinstance(entries, list),
            "malformed report suite",
        )
        for spec in entries:
            require(isinstance(spec, dict), "malformed report spec")
            specs.append(spec)
        for child in children:
            visit(child)

    for suite in suites:
        visit(suite)
    require(
        len(specs) == TARGET_COUNT,
        f"candidate v{run} is not the complete six-case file",
    )
    rows: dict[str, dict[str, Any]] = {}
    for spec in specs:
        spec_id, tests = spec.get("id"), spec.get("tests")
        require(
            spec.get("file") == frame_source.FILE
            and isinstance(spec_id, str)
            and spec_id in target_ids
            and spec_id not in rows
            and isinstance(tests, list)
            and len(tests) == 1
            and isinstance(tests[0], dict),
            f"candidate v{run} wrong, missing, or duplicate invocation",
        )
        test = tests[0]
        attempts = test.get("results")
        require(
            test.get("projectId") == frame_source.PROJECT
            and test.get("expectedStatus") == "passed"
            and test.get("status") in {"expected", "unexpected"}
            and isinstance(attempts, list)
            and len(attempts) == 1
            and isinstance(attempts[0], dict)
            and attempts[0].get("retry") == 0,
            f"candidate v{run} unexecuted or retried invocation {spec_id}",
        )
        attempt = attempts[0]
        if test["status"] == "expected":
            require(
                attempt.get("status") == "passed"
                and not attempt.get("error")
                and not attempt.get("errors"),
                f"candidate v{run} GREEN is not a clean pass {spec_id}",
            )
            rows[spec_id] = {"outcome": "expected", "attempts": 1}
        else:
            error = attempt.get("error")
            require(
                attempt.get("status") in {"failed", "timedOut"}
                and isinstance(error, dict)
                and isinstance(error.get("message"), str)
                and bool(error["message"])
                and isinstance(attempt.get("errors"), list)
                and bool(attempt["errors"]),
                f"candidate v{run} RED lacks terminal error {spec_id}",
            )
            rows[spec_id] = {
                "outcome": "unexpected",
                "attempts": 1,
                "error": error["message"],
            }
    require(set(rows) == target_ids, f"candidate v{run} ID set differs")
    require(
        sum(row["outcome"] == "expected" for row in rows.values()) == expected
        and sum(row["outcome"] == "unexpected" for row in rows.values()) == unexpected,
        f"candidate v{run} per-ID outcomes contradict totals",
    )
    require(
        (
            rows[FRAMESET_ID]
            == {"outcome": "unexpected", "attempts": 1, "error": FIRST_FAILURE}
        )
        if run == 1
        else all(
            row == {"outcome": "expected", "attempts": 1} for row in rows.values()
        ),
        f"candidate v{run} frameset failure or subsequent clean history differs",
    )
    if run == 1:
        require(
            all(
                row == {"outcome": "expected", "attempts": 1}
                for spec_id, row in rows.items()
                if spec_id != FRAMESET_ID
            ),
            "candidate v1 has another RED",
        )
    return rows


def verify_run_provenance(run: int, data: bytes) -> dict[str, Any]:
    payload = object_from(data, f"candidate v{run} provenance")
    require(
        payload.get("schema") == 2
        and payload.get("mode") == "candidate"
        and payload.get("suite") == frame_source.SUITE
        and payload.get("suiteRole") == "browser-backed-official-file"
        and payload.get("adapter") == "test-mode-playwright-core"
        and payload.get("project") == frame_source.PROJECT
        and payload.get("repository") == "https://github.com/microsoft/playwright.git"
        and payload.get("tag") == "v1.58.0"
        and payload.get("commit") == frame_source.COMMIT
        and payload.get("upstream") == str(frame_source.UPSTREAM)
        and payload.get("manifest") == str(frame_source.MANIFEST)
        and payload.get("manifestSha256") == frame_source.MANIFEST_SHA256
        and payload.get("endpoint") == "http://127.0.0.1:9222"
        and payload.get("fixtureHost") == "macbookpro-yuacx.tail8f3b66.ts.net"
        and payload.get("fixtureBind") == "100.78.97.63"
        and payload.get("filters") == ["tests/page/frame-frame-element.spec.ts"]
        and payload.get("workers") == 1
        and payload.get("grep") is None
        and payload.get("headed") is False
        and payload.get("exitCode") == (1 if run == 1 else 0)
        and payload.get("result") == str(report_path(run))
        and payload.get("resultSha256") == REPORT_SHA256[run]
        and payload.get("candidate") == str(PACKAGE)
        and payload.get("candidateProfile") == PROFILE
        and payload.get("candidateArtifact") == str(ARTIFACT)
        and payload.get("candidateArtifactSha256") == ARTIFACT_SHA256
        and payload.get("candidateSourceSha256") == CANDIDATE_SOURCE_SHA256
        and payload.get("command")
        == [
            "node",
            str(frame_source.UPSTREAM / "packages/playwright/cli.js"),
            "test",
            "--config=tests/library/playwright.config.ts",
            "--reporter=line,json",
            "--workers=1",
            "--project=chromium-page",
            "tests/page/frame-frame-element.spec.ts",
        ],
        f"candidate v{run} provenance identity, runtime, or exit differs",
    )
    provenance_verifier.verify_provenance(payload, provenance_path=provenance_path(run))
    return payload


def expected_candidate_row(old: dict[str, Any], spec_id: str) -> dict[str, Any]:
    require(
        old.get("specId") == spec_id
        and old.get("attempts") == 1
        and old.get("outcome") == "unexpected"
        and old.get("evidenceScope") == frame_source.SCOPE
        and old.get("resultFile") == str(frame_source.report_path("candidate"))
        and isinstance(old.get("error"), str)
        and bool(old["error"]),
        f"historical v20 candidate frame RED row differs: {spec_id}",
    )
    row = dict(old)
    row.pop("error")
    row.update(
        {
            "attempts": 5,
            "outcome": "expected",
            "resultFile": str(report_path(4)),
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
        scopes.get(frame_source.SCOPE) == TARGET_COUNT and SCOPE not in scopes,
        "v20 candidate frame scope differs",
    )
    outcomes["expected"] += TARGET_COUNT
    outcomes["unexpected"] -= TARGET_COUNT
    scopes.pop(frame_source.SCOPE)
    scopes[SCOPE] = TARGET_COUNT
    require(sum(outcomes.values()) == COUNT, "candidate summary arithmetic differs")
    return {
        "schema": 1,
        "inventory": str(frame_source.INVENTORY),
        "records": COUNT,
        "outcomes": dict(sorted(outcomes.items())),
        "explicitEvidenceScopes": dict(sorted(scopes.items())),
        "ledgerSha256": digest(ledger),
        "ledger": str(V21 / "candidate.jsonl"),
    }


def verify_inputs() -> dict[str, Any]:
    require(set(V20_SHA256) == BUNDLE_NAMES, "v20 hashes are incomplete")
    sealed(
        PREDECESSOR_VERIFIER, PREDECESSOR_VERIFIER_SHA256, "v20 independent verifier"
    )
    sealed(PROVENANCE_VERIFIER, PROVENANCE_VERIFIER_SHA256, "Core provenance verifier")
    old_receipt = predecessor.verify_existing()
    old_files = {
        name: sealed(V20 / name, sha, f"v20 {name}") for name, sha in V20_SHA256.items()
    }
    require(
        old_receipt.get("scope") == predecessor.SCOPE
        and old_receipt.get("recordsPerLane") == COUNT
        and old_receipt.get("fullCorpusExecution") is False
        and old_receipt.get("publicApiParityEvidence") is False,
        "v20 predecessor receipt differs",
    )
    inventory_lines = sealed(
        frame_source.INVENTORY, frame_source.INVENTORY_SHA256, "Core inventory"
    ).splitlines(keepends=True)
    require(len(inventory_lines) == COUNT, "Core inventory count differs")
    sealed(frame_source.MANIFEST, frame_source.MANIFEST_SHA256, "frame manifest")
    source = sealed(
        frame_source.SOURCE, frame_source.SOURCE_SHA256, "upstream frame source"
    )
    source_text = source.decode()
    frameset_start = source_text.index("it('should work with frameset'")
    frameset_end = source_text.index("it('should throw when detached'", frameset_start)
    frameset_body = source_text[frameset_start:frameset_end]
    require(
        frameset_body.index("await page.goto(")
        < frameset_body.index("await frame.frameElement()"),
        "frameset source does not establish pre-frameElement navigation",
    )
    revision = subprocess.run(
        ["git", "-C", str(frame_source.UPSTREAM), "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    require(revision == frame_source.COMMIT, "upstream revision differs")
    tracked = subprocess.run(
        [
            "git",
            "-C",
            str(frame_source.UPSTREAM),
            "status",
            "--porcelain",
            "--untracked-files=no",
        ],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    require(not tracked, "upstream checkout has tracked changes")
    require(
        PACKAGE.is_dir() and not PACKAGE.is_symlink(),
        "frozen candidate snapshot missing or symlink",
    )
    require(
        all(not path.is_symlink() for path in PACKAGE.rglob("*")),
        "frozen candidate snapshot has symlink",
    )
    sealed(PACKAGE / "package.json", PACKAGE_MANIFEST_SHA256, "frozen package manifest")
    sealed(ARTIFACT, ARTIFACT_SHA256, "frozen native addon")
    selected: dict[int, dict[str, Any]] = {}
    for index, raw in enumerate(inventory_lines):
        row = object_from(raw, f"inventory row {index}")
        if (
            row.get("suite") == "library"
            and row.get("projectId") == frame_source.PROJECT
            and row.get("file") == frame_source.FILE
        ):
            selected[index] = row
    target_ids = frame_source._source_ids(selected)
    require(
        len(selected) == TARGET_COUNT
        and FRAMESET_ID in target_ids
        and next(row for row in selected.values() if row["specId"] == FRAMESET_ID)[
            "line"
        ]
        == 43,
        "source-pinned frameset identity differs",
    )
    old_lines = {
        lane: old_files[f"{lane}.jsonl"].splitlines(keepends=True) for lane in LANES
    }
    require(
        all(len(lines) == COUNT for lines in old_lines.values()),
        "v20 ledger row count differs",
    )
    old_summaries = {
        lane: object_from(old_files[f"{lane}.summary.json"], f"v20 {lane} summary")
        for lane in LANES
    }
    historical = {
        lane: frame_source.report_rows(
            lane,
            sealed(
                frame_source.report_path(lane),
                frame_source.REPORT_SHA256[lane],
                f"v19 {lane} official report",
            ),
            target_ids,
        )
        for lane in LANES
    }
    require(
        all(row["outcome"] == "unexpected" for row in historical["candidate"].values())
        and all(row["outcome"] == "expected" for row in historical["oracle"].values()),
        "historical frame candidate 0/6 or Oracle 6/6 differs",
    )
    for lane in LANES:
        summary = old_summaries[lane]
        require(
            summary.get("schema") == 1
            and summary.get("records") == COUNT
            and summary.get("ledgerSha256") == V20_SHA256[f"{lane}.jsonl"]
            and summary["explicitEvidenceScopes"].get(frame_source.SCOPE)
            == TARGET_COUNT
            and sum(summary["outcomes"].values()) == COUNT,
            f"v20 {lane} summary differs",
        )
        for index, inventory_row in selected.items():
            old = object_from(old_lines[lane][index], f"v20 {lane} frame row {index}")
            expected = historical[lane][inventory_row["specId"]]
            require(
                old.get("specId") == inventory_row["specId"]
                and old.get("outcome") == expected["outcome"]
                and old.get("attempts") == 1
                and old.get("evidenceScope") == frame_source.SCOPE
                and old.get("resultFile") == str(frame_source.report_path(lane))
                and old.get("fixtureAdapted") is True
                and old.get("browserExecutionEvidence") is True
                and old.get("publicApiParityEvidence") is False,
                f"v20 {lane} frame row {index} differs",
            )
            if "error" in expected:
                require(
                    old.get("error") == expected["error"],
                    f"v20 {lane} frame error differs",
                )
            else:
                require("error" not in old, f"v20 {lane} frame clean row has error")
    runs: dict[int, dict[str, dict[str, Any]]] = {}
    for run, report_sha256 in REPORT_SHA256.items():
        report_data = sealed(
            report_path(run), report_sha256, f"candidate v{run} official report"
        )
        provenance_data = sealed(
            provenance_path(run), PROVENANCE_SHA256[run], f"candidate v{run} provenance"
        )
        verify_run_provenance(run, provenance_data)
        runs[run] = report_rows(run, report_data, target_ids)
    require(
        runs[1][FRAMESET_ID]["error"] == FIRST_FAILURE
        and all(
            runs[run][spec_id] == {"outcome": "expected", "attempts": 1}
            for run in (2, 3, 4)
            for spec_id in target_ids
        ),
        "four-run per-ID history differs",
    )
    return {
        "oldFiles": old_files,
        "oldLines": old_lines,
        "oldSummaries": old_summaries,
        "oldReceipt": old_receipt,
        "selected": selected,
        "historical": historical,
        "runs": runs,
    }


def expected_lane_receipt(
    lane: str, files: dict[str, bytes], checked: dict[str, Any]
) -> dict[str, Any]:
    require(lane in LANES, "unknown lane")
    new_lines = files[f"{lane}.jsonl"].splitlines(keepends=True)
    changed = (
        [
            {
                "index": index,
                "specId": checked["selected"][index]["specId"],
                "oldRowSha256": digest(checked["oldLines"][lane][index]),
                "newRowSha256": digest(new_lines[index]),
                "oldOutcome": "unexpected",
                "newOutcome": "expected",
                "oldAttempts": 1,
                "newAttempts": 5,
            }
            for index in sorted(checked["selected"])
        ]
        if lane == "candidate"
        else []
    )
    summary = object_from(files[f"{lane}.summary.json"], f"v21 {lane} summary")
    official_result = (
        report_path(4) if lane == "candidate" else frame_source.report_path(lane)
    )
    provenance = (
        provenance_path(4)
        if lane == "candidate"
        else frame_source.provenance_path(lane)
    )
    return {
        "base": str(V20 / f"{lane}.jsonl"),
        "baseSha256": V20_SHA256[f"{lane}.jsonl"],
        "baseSummarySha256": V20_SHA256[f"{lane}.summary.json"],
        "ledger": str(V21 / f"{lane}.jsonl"),
        "ledgerSha256": digest(files[f"{lane}.jsonl"]),
        "summarySha256": digest(files[f"{lane}.summary.json"]),
        "summaryReferencesHistoricalV20Ledger": lane == "oracle",
        "officialResult": str(official_result),
        "officialResultSha256": REPORT_SHA256[4]
        if lane == "candidate"
        else frame_source.REPORT_SHA256[lane],
        "provenance": str(provenance),
        "provenanceSha256": PROVENANCE_SHA256[4]
        if lane == "candidate"
        else frame_source.PROVENANCE_SHA256[lane],
        "changedRows": changed,
        "promotedRows": len(changed),
        "unchangedRawRows": COUNT - len(changed),
        "expectedRows": TARGET_COUNT,
        "unexpectedRows": 0,
        "outcomes": summary["outcomes"],
    }


def expected_receipt(
    files: dict[str, bytes], checked: dict[str, Any]
) -> dict[str, Any]:
    require(
        PROMOTER.is_file() and not PROMOTER.is_symlink(),
        "publisher source absent or symlink",
    )
    run_evidence = [
        {
            "run": run,
            "officialResult": str(report_path(run)),
            "officialResultSha256": REPORT_SHA256[run],
            "provenance": str(provenance_path(run)),
            "provenanceSha256": PROVENANCE_SHA256[run],
            "expectedRows": 5 if run == 1 else 6,
            "unexpectedRows": 1 if run == 1 else 0,
            "perSpecOutcomes": {
                spec_id: checked["runs"][run][spec_id]["outcome"]
                for spec_id in sorted(checked["runs"][run])
            },
        }
        for run in REPORT_SHA256
    ]
    return {
        "schema": 1,
        "scope": SCOPE,
        "promotionKind": "browser-backed-public-api-full-file-candidate-only-repeated-red-to-green-with-observed-instability",
        "priorBundle": str(V20),
        "priorBundleReceiptSha256": V20_SHA256["promotion-receipt.json"],
        "inventory": str(frame_source.INVENTORY),
        "inventorySha256": frame_source.INVENTORY_SHA256,
        "manifest": str(frame_source.MANIFEST),
        "manifestSha256": frame_source.MANIFEST_SHA256,
        "upstreamSourceSha256": frame_source.SOURCE_SHA256,
        "targetSpecIdsSha256": frame_source.SPEC_IDS_SHA256,
        "recordsPerLane": COUNT,
        "targetZeroBasedIndexes": sorted(checked["selected"]),
        "changedRowsPerLane": {"candidate": TARGET_COUNT, "oracle": 0},
        "unchangedRawRowsPerLane": {"candidate": COUNT - TARGET_COUNT, "oracle": COUNT},
        "candidateOnlyReruns": 4,
        "ledgerAttemptsPerPromotedSpec": 5,
        "latestPromotedRun": 4,
        "runEvidence": run_evidence,
        "unstableSpecIds": [FRAMESET_ID],
        "observedIntermittentNavigationFailure": {
            "specId": FRAMESET_ID,
            "title": "should work with frameset",
            "run": 1,
            "officialResult": str(report_path(1)),
            "officialResultSha256": REPORT_SHA256[1],
            "provenance": str(provenance_path(1)),
            "provenanceSha256": PROVENANCE_SHA256[1],
            "error": FIRST_FAILURE,
            "stage": "page.goto",
            "beforeFrameElement": True,
            "rootCauseEstablished": False,
        },
        "oracleJsonlAndSummaryByteIdenticalToV20": True,
        "fixtureAdapted": True,
        "browserExecutionEvidence": True,
        "completeFileExecution": True,
        "fullCorpusExecution": False,
        "publicApiParityEvidence": False,
        "upstreamAssertionsModified": False,
        "candidateExpectedRows": 6,
        "candidateUnexpectedRows": 0,
        "oracleExpectedRows": 6,
        "oracleUnexpectedRows": 0,
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
        "candidateSourceSha256": CANDIDATE_SOURCE_SHA256,
        "candidatePackageManifestSha256": PACKAGE_MANIFEST_SHA256,
        "candidatePackage": str(PACKAGE),
        "promoterSourceSha256": digest(PROMOTER.read_bytes()),
        "verifierSourceSha256": digest(Path(__file__).read_bytes()),
        "lanes": {lane: expected_lane_receipt(lane, files, checked) for lane in LANES},
    }


def verify_payloads(
    files: dict[str, bytes], checked: dict[str, Any] | None = None
) -> dict[str, Any]:
    require(set(files) == BUNDLE_NAMES, "v21 bundle files differ")
    inputs = checked if checked is not None else verify_inputs()
    targets = set(inputs["selected"])
    for lane in LANES:
        old_lines = inputs["oldLines"][lane]
        new_lines = files[f"{lane}.jsonl"].splitlines(keepends=True)
        require(len(new_lines) == COUNT, f"v21 {lane} row count differs")
        for index, (old_raw, new_raw) in enumerate(
            zip(old_lines, new_lines, strict=True)
        ):
            if lane == "candidate" and index in targets:
                spec_id = inputs["selected"][index]["specId"]
                expected = canonical_line(
                    expected_candidate_row(
                        object_from(old_raw, "v20 RED frame row"), spec_id
                    )
                )
                require(
                    new_raw == expected,
                    f"v21 candidate frame target row {index} differs",
                )
            else:
                require(
                    new_raw == old_raw, f"v21 {lane} off-target raw row {index} changed"
                )
    candidate_summary = object_from(
        files["candidate.summary.json"], "v21 candidate summary"
    )
    require(
        candidate_summary
        == expected_candidate_summary(
            inputs["oldSummaries"]["candidate"], files["candidate.jsonl"]
        ),
        "v21 candidate summary differs",
    )
    require(
        files["oracle.summary.json"] == inputs["oldFiles"]["oracle.summary.json"],
        "v21 Oracle summary bytes changed",
    )
    require(
        files["oracle.jsonl"] == inputs["oldFiles"]["oracle.jsonl"],
        "v21 Oracle ledger bytes changed",
    )
    receipt = object_from(files["promotion-receipt.json"], "v21 receipt")
    require(receipt == expected_receipt(files, inputs), "v21 receipt differs")
    return receipt


def verify_existing() -> dict[str, Any]:
    require(V21.is_dir() and not V21.is_symlink(), "v21 bundle is absent or symlink")
    require(
        {path.name for path in V21.iterdir()} == BUNDLE_NAMES, "v21 file set differs"
    )
    require(
        all(
            (V21 / name).is_file() and not (V21 / name).is_symlink()
            for name in BUNDLE_NAMES
        ),
        "v21 file absent or symlink",
    )
    return verify_payloads({name: (V21 / name).read_bytes() for name in BUNDLE_NAMES})


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify-existing", action="store_true", required=True)
    parser.parse_args()
    try:
        receipt = verify_existing()
    except (
        OSError,
        ValueError,
        KeyError,
        TypeError,
        StopIteration,
        json.JSONDecodeError,
        subprocess.CalledProcessError,
    ) as error:
        parser.exit(
            2, f"strict paired Core frameElement v21 verification rejected: {error}\n"
        )
    print(
        json.dumps(
            {"verified": True, "scope": receipt["scope"], "destination": str(V21)},
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
