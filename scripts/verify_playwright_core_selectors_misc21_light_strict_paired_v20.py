"""Independently verify the immutable selectors-misc21 candidate-only v20 pair.

The v19 baseline, unchanged Oracle bytes, light candidate snapshot, and new
complete official 21-case report remain independently sealed evidence. This
module never imports the v20 publisher or reruns browser tests.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path
from typing import Any

import verify_playwright_core_frame_frame_element6_strict_paired_v19 as predecessor
import verify_playwright_core_selectors_misc21_strict_paired_v18 as misc_source
import verify_playwright_official_provenance as provenance_verifier

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / ".work/official-conformance"
V19 = predecessor.V19
V20 = WORK / "outcomes/core-selectors-misc21-light-strict-paired-v20"
PROMOTER = (
    ROOT / "scripts/promote_playwright_core_selectors_misc21_light_strict_paired_v20.py"
)
PREDECESSOR_VERIFIER = (
    ROOT / "scripts/verify_playwright_core_frame_frame_element6_strict_paired_v19.py"
)
PROVENANCE_VERIFIER = ROOT / "scripts/verify_playwright_official_provenance.py"
REPORT = WORK / "results/core-selectors-misc21-light-candidate-20261021-v1.json"
PROVENANCE = REPORT.with_suffix(".provenance.json")
PACKAGE = WORK / "builds/core-selectors-misc21-light-v1/candidate"
PROFILE = "official-conformance-page-check-light-v1"
ARTIFACT = PACKAGE / f"native/{PROFILE}/pwright.node"
COUNT = predecessor.COUNT
TARGET_COUNT = misc_source.TARGET_COUNT
RED_ID = "c55872d27f1ea9b3c80f-5ef98f2c2d45634754d1"
RED_INDEX = 41_868
LANES = predecessor.LANES
BUNDLE_NAMES = predecessor.BUNDLE_NAMES
SCOPE = "official-browser-backed-selectors-misc-light-full-file-candidate-only-v20"
PREDECESSOR_VERIFIER_SHA256 = (
    "022f5a335e4500e0b1598c658af448a480a742a66b6f6369d5f40c3b6dcc592b"
)
PROVENANCE_VERIFIER_SHA256 = (
    "9b1c6c4edee469c3c7a56866444fb5ad1bafe31443164f497c562a8eda1e0672"
)
V19_SHA256 = {
    "candidate.jsonl": "b39bc7410104e425d8389709c265593755b5d87bbf6708fff0e90b2fdc1ce46d",
    "candidate.summary.json": "8569c03f71d3abd9861a135200fa501f0ee273de522afd67e8144391aa7a0108",
    "oracle.jsonl": "e9b921670232979db5d0fb3b6dfb045aad5d747c5f63f1385dfe372a9155c2a6",
    "oracle.summary.json": "921186214ade8d718a6b2d4211db02cbdd3a15b602254a86c8ce3b51aa378a0b",
    "promotion-receipt.json": "9303f3e34a31f15b92f9f834b734392c88b8589c5628a3a118ba379d3d92ab4b",
}
REPORT_SHA256 = "779513e68b3bcbf5278370e8722b02d823286683a0bdf036a2a031a9ca7f288c"
PROVENANCE_SHA256 = "66dab5f1f49304709ae029f0a279703da5af369b250f282fe4b9157e29594a50"
ARTIFACT_SHA256 = "39e5c0d109ec446dc5b3b0e680546e6c2d13c7be6426e5dcb5412f2ddf0f464b"
CANDIDATE_SOURCE_SHA256 = (
    "5110769ab37a1ae033aecdf859ae3c3422292f268a9fab03abf8191797d5bfa2"
)
PACKAGE_MANIFEST_SHA256 = (
    "40b301caa57ef941fd2293bd1becfdbb0cf513d5634edb43883fb251af1fb458"
)


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


def report_rows(data: bytes, target_ids: set[str]) -> dict[str, dict[str, Any]]:
    report = object_from(data, "new candidate official report")
    stats = report.get("stats")
    require(
        isinstance(stats, dict)
        and all(
            stats.get(key) == value
            for key, value in {
                "expected": 11,
                "unexpected": 10,
                "skipped": 0,
                "flaky": 0,
            }.items()
        ),
        "new candidate report is not 11/21 with ten RED",
    )
    require(report.get("errors") == [], "new candidate report has top-level errors")
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
    require(len(specs) == TARGET_COUNT, "report is not complete 21-case file")
    rows: dict[str, dict[str, Any]] = {}
    for spec in specs:
        spec_id, tests = spec.get("id"), spec.get("tests")
        require(
            spec.get("file") == misc_source.FILE
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
            test.get("projectId") == misc_source.PROJECT
            and test.get("expectedStatus") == "passed"
            and test.get("status") in {"expected", "unexpected"}
            and isinstance(attempts, list)
            and len(attempts) == 1
            and isinstance(attempts[0], dict)
            and attempts[0].get("retry") == 0,
            f"report invocation is unexecuted or malformed for {spec_id}",
        )
        attempt = attempts[0]
        if test["status"] == "expected":
            require(
                attempt.get("status") == "passed"
                and not attempt.get("error")
                and not attempt.get("errors"),
                f"GREEN invocation is not a clean pass for {spec_id}",
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
                f"RED invocation lacks terminal error for {spec_id}",
            )
            rows[spec_id] = {
                "outcome": "unexpected",
                "attempts": 1,
                "error": error["message"],
            }
    require(set(rows) == target_ids, "report target IDs differ")
    require(
        sum(row["outcome"] == "expected" for row in rows.values()) == 11
        and sum(row["outcome"] == "unexpected" for row in rows.values()) == 10,
        "report statuses contradict totals",
    )
    return rows


def verify_new_provenance(data: bytes) -> dict[str, Any]:
    payload = object_from(data, "new candidate provenance")
    require(
        payload.get("schema") == 2
        and payload.get("mode") == "candidate"
        and payload.get("suite") == misc_source.SUITE
        and payload.get("suiteRole") == "browser-backed-official-file"
        and payload.get("adapter") == "test-mode-playwright-core"
        and payload.get("project") == misc_source.PROJECT
        and payload.get("repository") == "https://github.com/microsoft/playwright.git"
        and payload.get("tag") == "v1.58.0"
        and payload.get("commit") == misc_source.COMMIT
        and payload.get("upstream") == str(misc_source.UPSTREAM)
        and payload.get("manifest") == str(misc_source.MANIFEST)
        and payload.get("manifestSha256") == misc_source.MANIFEST_SHA256
        and payload.get("endpoint") == "http://127.0.0.1:9222"
        and payload.get("fixtureHost") == "macbookpro-yuacx.tail8f3b66.ts.net"
        and payload.get("fixtureBind") == "100.78.97.63"
        and payload.get("filters") == ["tests/page/selectors-misc.spec.ts"]
        and payload.get("workers") == 1
        and payload.get("grep") is None
        and payload.get("headed") is False
        and payload.get("exitCode") == 1
        and payload.get("result") == str(REPORT)
        and payload.get("resultSha256") == REPORT_SHA256
        and payload.get("candidate") == str(PACKAGE)
        and payload.get("candidateProfile") == PROFILE
        and payload.get("candidateArtifact") == str(ARTIFACT)
        and payload.get("candidateArtifactSha256") == ARTIFACT_SHA256
        and payload.get("candidateSourceSha256") == CANDIDATE_SOURCE_SHA256
        and payload.get("command")
        == [
            "node",
            str(misc_source.UPSTREAM / "packages/playwright/cli.js"),
            "test",
            "--config=tests/library/playwright.config.ts",
            "--reporter=line,json",
            "--workers=1",
            "--project=chromium-page",
            "tests/page/selectors-misc.spec.ts",
        ],
        "new candidate provenance identity or runtime pin differs",
    )
    require(
        PACKAGE.is_dir() and not PACKAGE.is_symlink(),
        "clean package missing or symlink",
    )
    require(
        all(not item.is_symlink() for item in PACKAGE.rglob("*")),
        "clean package contains a symlink",
    )
    sealed(PACKAGE / "package.json", PACKAGE_MANIFEST_SHA256, "clean package manifest")
    sealed(ARTIFACT, ARTIFACT_SHA256, "clean native artifact")
    provenance_verifier.verify_provenance(payload, provenance_path=PROVENANCE)
    return payload


def expected_red_row(old: dict[str, Any]) -> dict[str, Any]:
    require(
        old.get("specId") == RED_ID
        and old.get("attempts") == 1
        and old.get("outcome") == "unexpected"
        and old.get("evidenceScope") == misc_source.SCOPE
        and old.get("resultFile") == str(misc_source.report_path("candidate"))
        and isinstance(old.get("error"), str)
        and "id:light=target" in old["error"],
        "historical v19 candidate RED row differs",
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
        scopes.get(misc_source.SCOPE) == TARGET_COUNT and SCOPE not in scopes,
        "v19 candidate evidence scope differs",
    )
    outcomes["expected"] += 1
    outcomes["unexpected"] -= 1
    scopes[misc_source.SCOPE] -= 1
    scopes[SCOPE] = 1
    require(sum(outcomes.values()) == COUNT, "candidate summary arithmetic differs")
    return {
        "schema": 1,
        "inventory": str(misc_source.INVENTORY),
        "records": COUNT,
        "outcomes": dict(sorted(outcomes.items())),
        "explicitEvidenceScopes": dict(sorted(scopes.items())),
        "ledgerSha256": digest(ledger),
        "ledger": str(V20 / "candidate.jsonl"),
    }


def verify_inputs() -> dict[str, Any]:
    require(set(V19_SHA256) == BUNDLE_NAMES, "v19 predecessor hashes are incomplete")
    sealed(
        PREDECESSOR_VERIFIER, PREDECESSOR_VERIFIER_SHA256, "v19 independent verifier"
    )
    sealed(PROVENANCE_VERIFIER, PROVENANCE_VERIFIER_SHA256, "Core provenance verifier")
    old_receipt = predecessor.verify_existing()
    old_files = {
        name: sealed(V19 / name, sha, f"v19 {name}") for name, sha in V19_SHA256.items()
    }
    require(
        old_receipt.get("scope") == predecessor.SCOPE
        and old_receipt.get("recordsPerLane") == COUNT
        and old_receipt.get("candidateExpectedRows") == 0
        and old_receipt.get("candidateUnexpectedRows") == 6
        and old_receipt.get("oracleExpectedRows") == 6
        and old_receipt.get("oracleUnexpectedRows") == 0
        and old_receipt.get("fullCorpusExecution") is False
        and old_receipt.get("publicApiParityEvidence") is False,
        "v19 predecessor receipt differs",
    )
    inventory_lines = sealed(
        misc_source.INVENTORY, misc_source.INVENTORY_SHA256, "Core inventory"
    ).splitlines(keepends=True)
    require(len(inventory_lines) == COUNT, "Core inventory row count differs")
    sealed(misc_source.MANIFEST, misc_source.MANIFEST_SHA256, "selectors-misc manifest")
    sealed(
        misc_source.SOURCE, misc_source.SOURCE_SHA256, "upstream selectors-misc source"
    )
    revision = subprocess.run(
        ["git", "-C", str(misc_source.UPSTREAM), "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    require(revision == misc_source.COMMIT, "upstream revision differs")
    tracked = subprocess.run(
        [
            "git",
            "-C",
            str(misc_source.UPSTREAM),
            "status",
            "--porcelain",
            "--untracked-files=no",
        ],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    require(not tracked, "upstream checkout has tracked changes")
    selected: dict[int, dict[str, Any]] = {}
    for index, raw in enumerate(inventory_lines):
        row = object_from(raw, f"inventory row {index}")
        if (
            row.get("suite") == "library"
            and row.get("projectId") == misc_source.PROJECT
            and row.get("file") == misc_source.FILE
        ):
            selected[index] = row
    target_ids = misc_source._source_ids(selected)
    require(
        selected.get(RED_INDEX, {}).get("specId") == RED_ID
        and selected[RED_INDEX].get("line") == 20,
        "source-pinned historical RED index differs",
    )
    old_lines = {
        lane: old_files[f"{lane}.jsonl"].splitlines(keepends=True) for lane in LANES
    }
    require(
        all(len(lines) == COUNT for lines in old_lines.values()),
        "v19 ledger row count differs",
    )
    old_summaries = {
        lane: object_from(old_files[f"{lane}.summary.json"], f"v19 {lane} summary")
        for lane in LANES
    }
    old_reports = {
        lane: misc_source.report_rows(
            lane,
            sealed(
                misc_source.report_path(lane),
                misc_source.REPORT_SHA256[lane],
                f"v18 {lane} selectors-misc report",
            ),
            target_ids,
        )
        for lane in LANES
    }
    require(
        all(row["outcome"] == "expected" for row in old_reports["oracle"].values()),
        "historical selectors-misc Oracle is not 21/21",
    )
    for lane in LANES:
        summary = old_summaries[lane]
        require(
            summary.get("schema") == 1
            and summary.get("records") == COUNT
            and summary.get("ledgerSha256") == V19_SHA256[f"{lane}.jsonl"]
            and old_receipt["lanes"][lane]["ledgerSha256"]
            == V19_SHA256[f"{lane}.jsonl"]
            and old_receipt["lanes"][lane]["summarySha256"]
            == V19_SHA256[f"{lane}.summary.json"]
            and sum(summary["outcomes"].values()) == COUNT
            and summary["explicitEvidenceScopes"].get(misc_source.SCOPE)
            == TARGET_COUNT,
            f"v19 {lane} summary or chain differs",
        )
        for index, inventory_row in selected.items():
            old = object_from(
                old_lines[lane][index], f"v19 {lane} selector row {index}"
            )
            historical = old_reports[lane][inventory_row["specId"]]
            require(
                old.get("specId") == inventory_row["specId"]
                and old.get("outcome") == historical["outcome"]
                and old.get("attempts") == 1
                and old.get("evidenceScope") == misc_source.SCOPE
                and old.get("resultFile") == str(misc_source.report_path(lane))
                and old.get("fixtureAdapted") is True
                and old.get("browserExecutionEvidence") is True
                and old.get("publicApiParityEvidence") is False,
                f"v19 {lane} selector row {index} differs",
            )
            if "error" in historical:
                require(
                    old.get("error") == historical["error"],
                    f"v19 {lane} selector error differs",
                )
            else:
                require("error" not in old, f"v19 {lane} clean selector has error")
    expected_red_row(object_from(old_lines["candidate"][RED_INDEX], "v19 RED row"))
    report = report_rows(
        sealed(REPORT, REPORT_SHA256, "new candidate official report"), target_ids
    )
    provenance = sealed(PROVENANCE, PROVENANCE_SHA256, "new candidate provenance")
    verify_new_provenance(provenance)
    require(
        report[RED_ID] == {"outcome": "expected", "attempts": 1}, "RED did not pass"
    )
    improvements: list[str] = []
    for spec_id in sorted(target_ids):
        before = old_reports["candidate"][spec_id]["outcome"]
        after = report[spec_id]["outcome"]
        require(
            not (before == "expected" and after != "expected"),
            f"candidate regression in {spec_id}",
        )
        if before != after:
            improvements.append(spec_id)
    require(
        improvements == [RED_ID],
        "candidate changed IDs differ from the single RED improvement",
    )
    return {
        "oldFiles": old_files,
        "oldLines": old_lines,
        "oldSummaries": old_summaries,
        "oldReceipt": old_receipt,
        "report": report,
        "selected": selected,
        "oldReports": old_reports,
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
    summary = object_from(files[f"{lane}.summary.json"], f"v20 {lane} summary")
    official_result = REPORT if lane == "candidate" else misc_source.report_path(lane)
    provenance = (
        PROVENANCE if lane == "candidate" else misc_source.provenance_path(lane)
    )
    return {
        "base": str(V19 / f"{lane}.jsonl"),
        "baseSha256": V19_SHA256[f"{lane}.jsonl"],
        "baseSummarySha256": V19_SHA256[f"{lane}.summary.json"],
        "ledger": str(V20 / f"{lane}.jsonl"),
        "ledgerSha256": digest(files[f"{lane}.jsonl"]),
        "summarySha256": digest(files[f"{lane}.summary.json"]),
        "summaryReferencesHistoricalV19Ledger": lane == "oracle",
        "officialResult": str(official_result),
        "officialResultSha256": REPORT_SHA256
        if lane == "candidate"
        else misc_source.REPORT_SHA256[lane],
        "provenance": str(provenance),
        "provenanceSha256": PROVENANCE_SHA256
        if lane == "candidate"
        else misc_source.PROVENANCE_SHA256[lane],
        "changedRows": changed,
        "promotedRows": len(changed),
        "unchangedRawRows": COUNT - len(changed),
        "expectedRows": 11 if lane == "candidate" else TARGET_COUNT,
        "unexpectedRows": 10 if lane == "candidate" else 0,
        "outcomes": summary["outcomes"],
    }


def expected_receipt(
    files: dict[str, bytes], checked: dict[str, Any]
) -> dict[str, Any]:
    require(
        PROMOTER.is_file() and not PROMOTER.is_symlink(),
        "publisher source absent or symlink",
    )
    candidate_red = sorted(
        spec_id
        for spec_id, result in checked["report"].items()
        if result["outcome"] == "unexpected"
    )
    require(len(candidate_red) == 10, "candidate RED comparison count differs")
    return {
        "schema": 1,
        "scope": SCOPE,
        "promotionKind": "browser-backed-public-api-full-file-candidate-only-red-to-green",
        "priorBundle": str(V19),
        "priorBundleReceiptSha256": V19_SHA256["promotion-receipt.json"],
        "inventory": str(misc_source.INVENTORY),
        "inventorySha256": misc_source.INVENTORY_SHA256,
        "manifest": str(misc_source.MANIFEST),
        "manifestSha256": misc_source.MANIFEST_SHA256,
        "upstreamSourceSha256": misc_source.SOURCE_SHA256,
        "targetSpecIdsSha256": misc_source.SPEC_IDS_SHA256,
        "recordsPerLane": COUNT,
        "targetZeroBasedIndexes": sorted(checked["selected"]),
        "changedRowsPerLane": {"candidate": 1, "oracle": 0},
        "unchangedRawRowsPerLane": {"candidate": COUNT - 1, "oracle": COUNT},
        "candidateOnlyRerun": True,
        "oracleJsonlAndSummaryByteIdenticalToV19": True,
        "fixtureAdapted": True,
        "browserExecutionEvidence": True,
        "completeFileExecution": True,
        "fullCorpusExecution": False,
        "publicApiParityEvidence": False,
        "upstreamAssertionsModified": False,
        "candidateExpectedRows": 11,
        "candidateUnexpectedRows": 10,
        "oracleExpectedRows": 21,
        "oracleUnexpectedRows": 0,
        "historicalRedSpecId": RED_ID,
        "historicalRedRowSha256": digest(checked["oldLines"]["candidate"][RED_INDEX]),
        "newlyExpectedSpecIds": [RED_ID],
        "regressedSpecIds": [],
        "redComparison": {
            "candidateOnlySpecIds": candidate_red,
            "bothRedSpecIds": [],
            "oracleOnlySpecIds": [],
            "candidateOnlyCount": 10,
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
    require(set(files) == BUNDLE_NAMES, "v20 bundle files differ")
    inputs = checked if checked is not None else verify_inputs()
    for lane in LANES:
        old_lines = inputs["oldLines"][lane]
        new_lines = files[f"{lane}.jsonl"].splitlines(keepends=True)
        require(len(new_lines) == COUNT, f"v20 {lane} row count differs")
        for index, (old_raw, new_raw) in enumerate(
            zip(old_lines, new_lines, strict=True)
        ):
            if lane == "candidate" and index == RED_INDEX:
                expected = canonical_line(
                    expected_red_row(object_from(old_raw, "v19 RED row"))
                )
                require(new_raw == expected, "v20 candidate RED target row differs")
            else:
                require(
                    new_raw == old_raw, f"v20 {lane} off-target raw row {index} changed"
                )
    candidate_summary = object_from(
        files["candidate.summary.json"], "v20 candidate summary"
    )
    require(
        candidate_summary
        == expected_candidate_summary(
            inputs["oldSummaries"]["candidate"], files["candidate.jsonl"]
        ),
        "v20 candidate summary differs",
    )
    require(
        files["oracle.summary.json"] == inputs["oldFiles"]["oracle.summary.json"],
        "v20 Oracle summary bytes changed",
    )
    require(
        files["oracle.jsonl"] == inputs["oldFiles"]["oracle.jsonl"],
        "v20 Oracle ledger bytes changed",
    )
    receipt = object_from(files["promotion-receipt.json"], "v20 receipt")
    require(receipt == expected_receipt(files, inputs), "v20 receipt differs")
    return receipt


def verify_existing() -> dict[str, Any]:
    require(V20.is_dir() and not V20.is_symlink(), "v20 bundle is absent or symlink")
    require(
        {path.name for path in V20.iterdir()} == BUNDLE_NAMES, "v20 file set differs"
    )
    require(
        all(
            (V20 / name).is_file() and not (V20 / name).is_symlink()
            for name in BUNDLE_NAMES
        ),
        "v20 file absent or symlink",
    )
    return verify_payloads({name: (V20 / name).read_bytes() for name in BUNDLE_NAMES})


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
        json.JSONDecodeError,
        subprocess.CalledProcessError,
    ) as error:
        parser.exit(
            2, f"strict paired Core selectors-misc v20 verification rejected: {error}\n"
        )
    print(
        json.dumps(
            {"verified": True, "scope": receipt["scope"], "destination": str(V20)},
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
