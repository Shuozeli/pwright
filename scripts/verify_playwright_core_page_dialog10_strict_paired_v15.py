"""Independently verify the immutable full-file Core dialog v15 paired ledger.

The official reports, provenance, manifest, source, and v14 predecessor are
read-only evidence. This verifier never imports the v15 publisher.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path
from typing import Any

import verify_playwright_core_page_history16_strict_paired_v14 as predecessor

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / ".work/official-conformance"
V14 = WORK / "outcomes/core-page-history16-strict-paired-v14"
V15 = WORK / "outcomes/core-page-dialog10-strict-paired-v15"
INVENTORY = WORK / "playwright-core-v1.58.0.jsonl"
MANIFEST = (
    ROOT / "tests/official/playwright-core-page-dialog-public-chromium-v1-manifest.json"
)
SOURCE = Path(
    "/Users/cyuan/projects/thirdparty/playwright-v1.58.0/tests/page/page-dialog.spec.ts"
)
UPSTREAM = SOURCE.parents[2]
PROMOTER = ROOT / "scripts/promote_playwright_core_page_dialog10_strict_paired_v15.py"
PREDECESSOR_VERIFIER = (
    ROOT / "scripts/verify_playwright_core_page_history16_strict_paired_v14.py"
)
COUNT = 48_477
TARGET_COUNT = 10
FILE = "page/page-dialog.spec.ts"
PROJECT = "chromium-page"
SUITE = "library-page-dialog-public-chromium"
SCOPE = "official-browser-backed-page-dialog-full-file-paired-v15"
PRIOR_SCOPE = "official-browser-backed-page-history-full-file-paired-v14"
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
MANIFEST_SHA256 = "93807be69aa93c7287cafe207756b5386b65edda2c1ca18711d8f6041dbe8bd6"
SOURCE_SHA256 = "b758da2196bf60d560528fc77eec6dfef23e191de9211fb0e6570909a61e0d7f"
SPEC_IDS_SHA256 = "bc8293399b2b2f323a1473312971d19cf9adc2920cac2dfd9c1e98e4ba68cb50"
PREDECESSOR_VERIFIER_SHA256 = (
    "1bfa866b893925f0e11704c067de9ce5364ce0dbe4b15e4fa7616d8945959e15"
)
V14_SHA256 = {
    "candidate.jsonl": "6f9b461048ab0fd4c278eded09ae646e5d926c34490d02da1c57f9cd76e1ff87",
    "candidate.summary.json": "41cca07f0d102751177c61918fafeb9ded9e06c91bed1b69847f97beb482f1dd",
    "oracle.jsonl": "65c757c90cec3f30ab90d144e9bc9635c058e85d59117b73592685bf6be59534",
    "oracle.summary.json": "f921da2b6eb3d9aca799568a5a9bef61ce84a559de40151834864e01d1008d5f",
    "promotion-receipt.json": "07ce72583a7011139e9aeebc94c2be8c3573e29c619a4ed4107557ed44125b9e",
}
LANES = ("candidate", "oracle")
REPORT_SHA256 = {
    "candidate": "efc6831c54e44cfd1aea25a844f68327fd77f40d3fba4601e273993e2526083b",
    "oracle": "ee6957fbe23f27725c3158eba1cc016932001e8dc244352968ddddf7c02e05f6",
}
PROVENANCE_SHA256 = {
    "candidate": "0f6cacf24db4917e434bae7b0536e54be6914dfafaa30053b248c31ba39d9a00",
    "oracle": "b2a73df40f69fbf37b042b935a9acdb4dcaa313a10b8cebe83b17b0b7bbb17a4",
}
REPORT_OUTCOMES = {
    "candidate": {"expected": 9, "unexpected": 1},
    "oracle": {"expected": 10, "unexpected": 0},
}
TARGETS = (
    (38_595, "4234be2f84dbe1e7b3b6-0b499d2688cbc5d2e5dd", 20),
    (38_598, "4234be2f84dbe1e7b3b6-ec008d483744cb48339d", 30),
    (38_601, "4234be2f84dbe1e7b3b6-8afcd88777ba8e40af67", 43),
    (38_604, "4234be2f84dbe1e7b3b6-7d769c817525eaae6525", 51),
    (38_607, "4234be2f84dbe1e7b3b6-dcf281d3e5e7d8d823f8", 59),
    (38_610, "4234be2f84dbe1e7b3b6-bc3329a3363d6fae8c10", 67),
    (38_613, "4234be2f84dbe1e7b3b6-2bd38bfcbfa1ab44efcc", 75),
    (38_616, "4234be2f84dbe1e7b3b6-7f429de1d812f294dd7c", 90),
    (38_619, "4234be2f84dbe1e7b3b6-86b9cbd0315be32715a9", 105),
    (38_622, "4234be2f84dbe1e7b3b6-2e609ae90079fb25b1cd", 112),
)
RED_ID = "4234be2f84dbe1e7b3b6-bc3329a3363d6fae8c10"
BUNDLE_NAMES = {
    "candidate.jsonl",
    "candidate.summary.json",
    "oracle.jsonl",
    "oracle.summary.json",
    "promotion-receipt.json",
}
TOOLING_SOURCES = (
    (
        "scripts/official_result_destination.py",
        "61cd172cc58af3fecfe09b9d44da5ae67038ab660f7ed8a140b1ddf1784c04ab",
    ),
    (
        "scripts/run_playwright_official_conformance.py",
        "f1e4282361b650daaf0e5eae406e22b099c2a4f9dac796b4a6d594d3543dad29",
    ),
)
HARNESS_SOURCES = {
    "candidate": (
        (
            "tests/official/playwright-core-server-runtime.cjs",
            "58278e6c976da47074a1eb8d682916ebc3d0a41975fe89fe9270d1a664c396fd",
        ),
        (
            "tests/official/playwright-core-candidate-preload.cjs",
            "d17be590ff9334b0d99dd16d0078e4b434a39eae21568b799314ababc6bbf924",
        ),
    ),
    "oracle": (
        (
            "tests/official/playwright-core-server-runtime.cjs",
            "58278e6c976da47074a1eb8d682916ebc3d0a41975fe89fe9270d1a664c396fd",
        ),
        (
            "tests/official/playwright-core-oracle-preload.cjs",
            "ac947bd025a32c148e7b9e650f94eba11846b56c4bdd546dc3630b8edf28d43b",
        ),
    ),
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical_line(row: dict[str, Any]) -> bytes:
    return (json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n").encode()


def sealed(path: Path, expected: str, label: str) -> bytes:
    require(path.is_file() and not path.is_symlink(), f"{label} absent or symlink")
    data = path.read_bytes()
    require(digest(data) == expected, f"{label} SHA-256 changed")
    return data


def object_from(data: bytes, label: str) -> dict[str, Any]:
    value = json.loads(data)
    require(isinstance(value, dict), f"{label} is not an object")
    return value


def report_path(lane: str) -> Path:
    require(lane in LANES, "unknown lane")
    return WORK / f"results/core-page-dialog10-{lane}-20261021-v1.json"


def provenance_path(lane: str) -> Path:
    return report_path(lane).with_suffix(".provenance.json")


def verify_source_records(
    raw: Any,
    records: tuple[tuple[str, str], ...],
    label: str,
    *,
    absolute: bool = False,
) -> None:
    expected = [
        {"path": str(ROOT / path) if absolute else path, "sha256": sha}
        for path, sha in records
    ]
    require(raw == expected, f"{label} source pins differ")
    for relative, sha in records:
        sealed(ROOT / relative, sha, f"{label} source {relative}")


def verify_provenance(lane: str, data: bytes) -> None:
    value = object_from(data, f"{lane} provenance")
    require(
        value.get("schema") == 2
        and value.get("mode") == lane
        and value.get("suite") == SUITE
        and value.get("suiteRole") == "browser-backed-official-file"
        and value.get("project") == PROJECT
        and value.get("repository") == "https://github.com/microsoft/playwright.git"
        and value.get("tag") == "v1.58.0"
        and value.get("commit") == COMMIT
        and value.get("upstream") == str(UPSTREAM)
        and value.get("manifest") == str(MANIFEST)
        and value.get("manifestSha256") == MANIFEST_SHA256
        and value.get("result") == str(report_path(lane))
        and value.get("resultSha256") == REPORT_SHA256[lane]
        and value.get("endpoint") == "http://127.0.0.1:9222"
        and value.get("fixtureHost") == "macbookpro-yuacx.tail8f3b66.ts.net"
        and value.get("fixtureBind") == "100.78.97.63"
        and value.get("filters") == ["tests/page/page-dialog.spec.ts"]
        and value.get("workers") == 1
        and value.get("grep") is None
        and value.get("headed") is False
        and value.get("exitCode") == (1 if lane == "candidate" else 0)
        and value.get("adapter")
        == (
            "test-mode-playwright-core"
            if lane == "candidate"
            else "test-mode-playwright-core-cdp"
        )
        and value.get("command")
        == [
            "node",
            str(UPSTREAM / "packages/playwright/cli.js"),
            "test",
            "--config=tests/library/playwright.config.ts",
            "--reporter=line,json",
            "--workers=1",
            "--project=chromium-page",
            "tests/page/page-dialog.spec.ts",
        ],
        f"{lane} provenance identity or source pin differs",
    )
    verify_source_records(
        value.get("toolingSources"), TOOLING_SOURCES, f"{lane} tooling"
    )
    verify_source_records(
        value.get("harnessInputs"),
        HARNESS_SOURCES[lane],
        f"{lane} harness",
        absolute=True,
    )
    if lane == "candidate":
        require(
            value.get("candidate") == str(predecessor.PACKAGE)
            and value.get("candidateArtifact") == str(predecessor.ARTIFACT)
            and value.get("candidateProfile") == predecessor.CANDIDATE_PROFILE
            and value.get("candidateArtifactSha256")
            == predecessor.CANDIDATE_ARTIFACT_SHA256
            and value.get("candidateSourceSha256")
            == predecessor.CANDIDATE_SOURCE_SHA256,
            "candidate provenance runtime pin differs",
        )
    else:
        require(
            all(
                value.get(key) is None
                for key in (
                    "candidate",
                    "candidateArtifact",
                    "candidateProfile",
                    "candidateArtifactSha256",
                    "candidateSourceSha256",
                )
            ),
            "oracle provenance contains candidate runtime",
        )


def report_rows(
    lane: str, data: bytes, target_ids: set[str]
) -> dict[str, dict[str, Any]]:
    report = object_from(data, f"{lane} report")
    stats = report.get("stats")
    require(
        isinstance(stats, dict)
        and all(
            stats.get(key) == value
            for key, value in {
                **REPORT_OUTCOMES[lane],
                "skipped": 0,
                "flaky": 0,
            }.items()
        ),
        f"{lane} report counts differ",
    )
    suites = report.get("suites")
    require(
        isinstance(suites, list) and len(suites) == 1,
        f"{lane} report suite count differs",
    )
    specs: list[dict[str, Any]] = []

    def visit(suite: dict[str, Any]) -> None:
        require(isinstance(suite, dict), "malformed report suite")
        children, entries = suite.get("suites", []), suite.get("specs", [])
        require(
            isinstance(children, list) and isinstance(entries, list),
            "malformed report suite children",
        )
        for spec in entries:
            require(isinstance(spec, dict), "malformed report spec")
            specs.append(spec)
        for child in children:
            visit(child)

    for suite in suites:
        visit(suite)
    require(
        len(specs) == TARGET_COUNT, f"{lane} report is not the complete 10-case file"
    )
    rows: dict[str, dict[str, Any]] = {}
    for spec in specs:
        spec_id, tests = spec.get("id"), spec.get("tests")
        require(
            spec.get("file") == FILE
            and isinstance(spec_id, str)
            and spec_id in target_ids
            and spec_id not in rows
            and isinstance(tests, list)
            and len(tests) == 1
            and isinstance(tests[0], dict),
            f"{lane} report has wrong, missing, or duplicate invocation",
        )
        test = tests[0]
        attempts = test.get("results")
        require(
            test.get("projectId") == PROJECT
            and test.get("expectedStatus") == "passed"
            and test.get("status") in {"expected", "unexpected"}
            and isinstance(attempts, list)
            and len(attempts) == 1
            and isinstance(attempts[0], dict)
            and attempts[0].get("retry") == 0,
            f"{lane} report has an unexecuted or malformed invocation for {spec_id}",
        )
        attempt = attempts[0]
        if test["status"] == "expected":
            require(
                attempt.get("status") == "passed"
                and not attempt.get("error")
                and not attempt.get("errors"),
                f"{lane} report GREEN invocation is not a clean pass for {spec_id}",
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
                f"{lane} report RED invocation lacks terminal failure for {spec_id}",
            )
            rows[spec_id] = {
                "outcome": "unexpected",
                "attempts": 1,
                "error": error["message"],
            }
    require(set(rows) == target_ids, f"{lane} report does not cover exact target IDs")
    require(
        {
            outcome: sum(row["outcome"] == outcome for row in rows.values())
            for outcome in ("expected", "unexpected")
        }
        == REPORT_OUTCOMES[lane],
        f"{lane} report statuses contradict totals",
    )
    return rows


def expected_summary(
    lane: str, old_summary: dict[str, Any], ledger_bytes: bytes
) -> dict[str, Any]:
    require(lane in LANES, "unknown lane")
    outcomes = dict(old_summary["outcomes"])
    scopes = dict(old_summary["explicitEvidenceScopes"])
    require(
        SCOPE not in scopes and outcomes["not-run"] >= TARGET_COUNT,
        "v15 already promoted",
    )
    outcomes["not-run"] -= TARGET_COUNT
    for outcome, count in REPORT_OUTCOMES[lane].items():
        outcomes[outcome] += count
    scopes[SCOPE] = TARGET_COUNT
    return {
        "schema": 1,
        "inventory": str(INVENTORY),
        "records": COUNT,
        "outcomes": dict(sorted(outcomes.items())),
        "explicitEvidenceScopes": dict(sorted(scopes.items())),
        "ledgerSha256": digest(ledger_bytes),
        "ledger": str(V15 / f"{lane}.jsonl"),
    }


def verify_inputs() -> dict[str, Any]:
    sealed(
        PREDECESSOR_VERIFIER, PREDECESSOR_VERIFIER_SHA256, "v14 independent verifier"
    )
    predecessor.verify_existing()
    inventory_lines = sealed(INVENTORY, INVENTORY_SHA256, "Core inventory").splitlines(
        keepends=True
    )
    require(len(inventory_lines) == COUNT, "Core inventory row count differs")
    manifest = object_from(
        sealed(MANIFEST, MANIFEST_SHA256, "dialog manifest"), "dialog manifest"
    )
    suite = manifest.get("suites")
    require(
        manifest.get("schema") == 1
        and manifest.get("repository") == "https://github.com/microsoft/playwright.git"
        and manifest.get("tag") == "v1.58.0"
        and manifest.get("commit") == COMMIT
        and isinstance(suite, list)
        and len(suite) == 1
        and suite[0].get("id") == SUITE
        and suite[0].get("project") == PROJECT
        and suite[0].get("expectedTestsPerProject") == TARGET_COUNT
        and suite[0].get("expectedFiles") == 1
        and suite[0].get("expectedSpecIdsSha256") == SPEC_IDS_SHA256
        and suite[0].get("upstreamTestSourceSha256") == SOURCE_SHA256,
        "dialog manifest selection differs",
    )
    sealed(SOURCE, SOURCE_SHA256, "upstream page-dialog source")
    revision = subprocess.run(
        ["git", "-C", str(UPSTREAM), "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    require(revision == COMMIT, "upstream checkout revision differs")
    require(
        digest(
            "".join(
                f"{spec_id}\n"
                for spec_id in sorted(spec_id for _, spec_id, _ in TARGETS)
            ).encode()
        )
        == SPEC_IDS_SHA256,
        "literal target IDs do not match source-pinned digest",
    )
    selected: dict[int, dict[str, Any]] = {}
    for index, raw in enumerate(inventory_lines):
        row = json.loads(raw)
        if (
            row.get("suite") == "library"
            and row.get("projectId") == PROJECT
            and row.get("file") == FILE
        ):
            selected[index] = row
    require(
        len(selected) == TARGET_COUNT
        and set(selected) == {index for index, _, _ in TARGETS},
        "inventory target indexes differ",
    )
    for index, spec_id, line in TARGETS:
        require(
            selected[index].get("specId") == spec_id
            and selected[index].get("line") == line
            and selected[index].get("sourceClassification") == "public-api-candidate"
            and selected[index].get("expectedStatus") == "passed",
            f"inventory dialog source identity differs at {index}",
        )
    old_files = {
        name: sealed(V14 / name, sha, f"v14 {name}") for name, sha in V14_SHA256.items()
    }
    old_receipt = object_from(old_files["promotion-receipt.json"], "v14 receipt")
    require(
        old_receipt.get("scope") == PRIOR_SCOPE
        and old_receipt.get("recordsPerLane") == COUNT
        and old_receipt.get("fullCorpusExecution") is False
        and old_receipt.get("publicApiParityEvidence") is False,
        "v14 predecessor scope differs",
    )
    old_lines: dict[str, list[bytes]] = {}
    old_summaries: dict[str, dict[str, Any]] = {}
    replacements: dict[str, dict[int, dict[str, Any]]] = {}
    reports: dict[str, dict[str, dict[str, Any]]] = {}
    target_ids = {spec_id for _, spec_id, _ in TARGETS}
    for lane in LANES:
        lines = old_files[f"{lane}.jsonl"].splitlines(keepends=True)
        require(len(lines) == COUNT, f"v14 {lane} row count differs")
        old_lines[lane] = lines
        summary = object_from(old_files[f"{lane}.summary.json"], f"v14 {lane} summary")
        require(
            summary.get("schema") == 1
            and summary.get("records") == COUNT
            and summary.get("ledgerSha256") == V14_SHA256[f"{lane}.jsonl"]
            and old_receipt["lanes"][lane]["ledgerSha256"]
            == V14_SHA256[f"{lane}.jsonl"]
            and old_receipt["lanes"][lane]["summarySha256"]
            == V14_SHA256[f"{lane}.summary.json"]
            and sum(summary["outcomes"].values()) == COUNT,
            f"v14 {lane} summary or receipt chain differs",
        )
        old_summaries[lane] = summary
        verify_provenance(
            lane,
            sealed(
                provenance_path(lane), PROVENANCE_SHA256[lane], f"{lane} provenance"
            ),
        )
        report = sealed(
            report_path(lane), REPORT_SHA256[lane], f"{lane} official report"
        )
        reports[lane] = report_rows(lane, report, target_ids)
        replacements[lane] = {}
        for index, spec_id, _ in TARGETS:
            old = object_from(lines[index], f"v14 {lane} target {index}")
            require(
                old == selected[index] | {"attempts": 0, "outcome": "not-run"},
                f"v14 {lane} target {index} is not untested inventory row",
            )
            new = dict(old)
            new.update(reports[lane][spec_id])
            new.update(
                {
                    "resultFile": str(report_path(lane)),
                    "evidenceScope": SCOPE,
                    "fixtureAdapted": True,
                    "browserExecutionEvidence": True,
                    "publicApiParityEvidence": False,
                }
            )
            replacements[lane][index] = new
    require(
        {
            spec_id
            for spec_id, row in reports["candidate"].items()
            if row["outcome"] == "unexpected"
        }
        == {RED_ID}
        and all(row["outcome"] == "expected" for row in reports["oracle"].values())
        and "timeout" in reports["candidate"][RED_ID]["error"].lower(),
        "dialog candidate-only RED intersection or timeout differs",
    )
    return {
        "oldLines": old_lines,
        "oldSummaries": old_summaries,
        "oldReceipt": old_receipt,
        "replacementRows": replacements,
        "reports": reports,
    }


def expected_lane_receipt(
    lane: str, files: dict[str, bytes], checked: dict[str, Any]
) -> dict[str, Any]:
    new_lines = files[f"{lane}.jsonl"].splitlines(keepends=True)
    changed = [
        {
            "index": index,
            "specId": spec_id,
            "oldRowSha256": digest(checked["oldLines"][lane][index]),
            "newRowSha256": digest(new_lines[index]),
            "oldOutcome": "not-run",
            "newOutcome": checked["replacementRows"][lane][index]["outcome"],
            "oldAttempts": 0,
            "newAttempts": 1,
        }
        for index, spec_id, _ in TARGETS
    ]
    summary = object_from(files[f"{lane}.summary.json"], f"v15 {lane} summary")
    return {
        "base": str(V14 / f"{lane}.jsonl"),
        "baseSha256": V14_SHA256[f"{lane}.jsonl"],
        "baseSummarySha256": V14_SHA256[f"{lane}.summary.json"],
        "ledger": str(V15 / f"{lane}.jsonl"),
        "ledgerSha256": digest(files[f"{lane}.jsonl"]),
        "summarySha256": digest(files[f"{lane}.summary.json"]),
        "officialResult": str(report_path(lane)),
        "officialResultSha256": REPORT_SHA256[lane],
        "provenance": str(provenance_path(lane)),
        "provenanceSha256": PROVENANCE_SHA256[lane],
        "changedRows": changed,
        "promotedRows": TARGET_COUNT,
        "unchangedRawRows": COUNT - TARGET_COUNT,
        "expectedRows": REPORT_OUTCOMES[lane]["expected"],
        "unexpectedRows": REPORT_OUTCOMES[lane]["unexpected"],
        "outcomes": summary["outcomes"],
    }


def expected_receipt(
    files: dict[str, bytes], checked: dict[str, Any]
) -> dict[str, Any]:
    require(
        PROMOTER.is_file() and not PROMOTER.is_symlink(),
        "publisher source absent or symlink",
    )
    return {
        "schema": 1,
        "scope": SCOPE,
        "promotionKind": "browser-backed-public-api-full-file-paired-red-status",
        "priorBundle": str(V14),
        "priorBundleReceiptSha256": V14_SHA256["promotion-receipt.json"],
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "manifest": str(MANIFEST),
        "manifestSha256": MANIFEST_SHA256,
        "upstreamSourceSha256": SOURCE_SHA256,
        "targetSpecIdsSha256": SPEC_IDS_SHA256,
        "recordsPerLane": COUNT,
        "targetZeroBasedIndexes": [index for index, _, _ in TARGETS],
        "changedRowsPerLane": {lane: TARGET_COUNT for lane in LANES},
        "unchangedRawRowsPerLane": {lane: COUNT - TARGET_COUNT for lane in LANES},
        "fixtureAdapted": True,
        "browserExecutionEvidence": True,
        "completeFileExecution": True,
        "fullCorpusExecution": False,
        "chromeVersionObserved": "155.0.8059.85",
        "launchMode": "cdp-adapter",
        "publicApiParityEvidence": False,
        "upstreamAssertionsModified": False,
        "candidateExpectedRows": 9,
        "candidateUnexpectedRows": 1,
        "oracleExpectedRows": 10,
        "oracleUnexpectedRows": 0,
        "redComparison": {
            "candidateOnlySpecIds": [RED_ID],
            "bothRedSpecIds": [],
            "oracleOnlySpecIds": [],
            "candidateOnlyCount": 1,
            "bothRedCount": 0,
            "oracleOnlyCount": 0,
        },
        "candidateProfile": predecessor.CANDIDATE_PROFILE,
        "candidateArtifactSha256": predecessor.CANDIDATE_ARTIFACT_SHA256,
        "candidateSourceSha256": predecessor.CANDIDATE_SOURCE_SHA256,
        "candidatePackage": str(predecessor.PACKAGE),
        "candidatePackageManifest": str(predecessor.PACKAGE_MANIFEST),
        "candidatePackageManifestSha256": predecessor.PACKAGE_MANIFEST_SHA256,
        "promoterSourceSha256": digest(PROMOTER.read_bytes()),
        "verifierSourceSha256": digest(Path(__file__).read_bytes()),
        "lanes": {lane: expected_lane_receipt(lane, files, checked) for lane in LANES},
    }


def verify_payloads(
    files: dict[str, bytes], checked: dict[str, Any] | None = None
) -> dict[str, Any]:
    require(set(files) == BUNDLE_NAMES, "v15 bundle files differ")
    inputs = checked if checked is not None else verify_inputs()
    for lane in LANES:
        old_lines = inputs["oldLines"][lane]
        new_lines = files[f"{lane}.jsonl"].splitlines(keepends=True)
        require(len(new_lines) == COUNT, f"v15 {lane} row count differs")
        replacements = inputs["replacementRows"][lane]
        for index, (old_raw, new_raw) in enumerate(
            zip(old_lines, new_lines, strict=True)
        ):
            if index in replacements:
                require(
                    new_raw == canonical_line(replacements[index]),
                    f"v15 {lane} target row {index} differs",
                )
            else:
                require(
                    new_raw == old_raw, f"v15 {lane} off-target raw row {index} changed"
                )
        summary = object_from(files[f"{lane}.summary.json"], f"v15 {lane} summary")
        require(
            summary
            == expected_summary(
                lane, inputs["oldSummaries"][lane], files[f"{lane}.jsonl"]
            ),
            f"v15 {lane} summary differs",
        )
    receipt = object_from(files["promotion-receipt.json"], "v15 receipt")
    require(receipt == expected_receipt(files, inputs), "v15 receipt differs")
    return receipt


def verify_existing() -> dict[str, Any]:
    require(V15.is_dir() and not V15.is_symlink(), "v15 bundle is absent or symlink")
    require(
        {path.name for path in V15.iterdir()} == BUNDLE_NAMES,
        "v15 published file set differs",
    )
    require(
        all(
            (V15 / name).is_file() and not (V15 / name).is_symlink()
            for name in BUNDLE_NAMES
        ),
        "v15 published file is absent or symlink",
    )
    return verify_payloads({name: (V15 / name).read_bytes() for name in BUNDLE_NAMES})


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
        subprocess.CalledProcessError,
        json.JSONDecodeError,
    ) as error:
        parser.exit(
            2, f"strict paired Core dialog v15 verification rejected: {error}\n"
        )
    print(
        json.dumps(
            {"verified": True, "scope": receipt["scope"], "destination": str(V15)},
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
