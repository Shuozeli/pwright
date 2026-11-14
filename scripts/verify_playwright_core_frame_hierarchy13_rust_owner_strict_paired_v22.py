"""Independently verify the immutable Core frame-hierarchy 13-case v22 pair.

This checks sealed full-file candidate and Oracle runs, the v21 predecessor,
source and fixture inputs, every promoted row, every untouched raw row, and
the receipt. Five candidate REDs are evidence, not a parity claim.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path
from typing import Any

import verify_playwright_core_frame_frame_element6_rust_owner_strict_paired_v21 as predecessor
import verify_playwright_official_provenance as provenance_verifier

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / ".work/official-conformance"
V21 = predecessor.V21
V22 = WORK / "outcomes/core-frame-hierarchy13-rust-owner-strict-paired-v22"
INVENTORY = WORK / "playwright-core-v1.58.0.jsonl"
MANIFEST = (
    ROOT
    / "tests/official/playwright-core-frame-hierarchy-public-chromium-v1-manifest.json"
)
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-v1.58.0")
SOURCE = UPSTREAM / "tests/page/frame-hierarchy.spec.ts"
PROMOTER = (
    ROOT
    / "scripts/promote_playwright_core_frame_hierarchy13_rust_owner_strict_paired_v22.py"
)
PREDECESSOR_VERIFIER = (
    ROOT
    / "scripts/verify_playwright_core_frame_frame_element6_rust_owner_strict_paired_v21.py"
)
PROVENANCE_VERIFIER = ROOT / "scripts/verify_playwright_official_provenance.py"
PACKAGE = WORK / "builds/core-frame-frame-element6-rust-owner-v1/candidate"
PROFILE = "official-conformance-page-frame-element"
ARTIFACT = PACKAGE / f"native/{PROFILE}/pwright.node"
COUNT = 48_477
TARGET_COUNT = 13
FILE = "page/frame-hierarchy.spec.ts"
SOURCE_FILE = "tests/page/frame-hierarchy.spec.ts"
PROJECT = "chromium-page"
SUITE = "library-frame-hierarchy-public-chromium"
SCOPE = "official-browser-backed-frame-hierarchy-full-file-paired-rust-owner-v22"
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
MANIFEST_SHA256 = "e0cc5a5b2568a56472bab4e6b0f26314a68d01d10197e494673641f95f73afa7"
SOURCE_SHA256 = "3bd104830e4cb8f17ed674b5818141b26101d0a2b09be793de974b416f726a7f"
SPEC_IDS_SHA256 = "7eadee938cc59a04bfc7febb07cea12564972441f5d3742f61e055e062cce1fa"
PREDECESSOR_VERIFIER_SHA256 = (
    "5c2d7c2362466935d33f5eb973b9173a13adc7de2a0afdaf85c98515f13d50ff"
)
PROVENANCE_VERIFIER_SHA256 = (
    "9b1c6c4edee469c3c7a56866444fb5ad1bafe31443164f497c562a8eda1e0672"
)
PACKAGE_MANIFEST_SHA256 = (
    "40b301caa57ef941fd2293bd1becfdbb0cf513d5634edb43883fb251af1fb458"
)
ARTIFACT_SHA256 = "440ec056a5cfceeacbf851c698ee24c33c3db063ad92262c215d996a568db6a0"
CANDIDATE_SOURCE_SHA256 = (
    "150ba50a46ca5690923ce7897be1c1cf5391f9cbcf94ee184a6f675ed6524567"
)
V21_SHA256 = {
    "candidate.jsonl": "642e4fb2d25b999ed8f44902a81be67dd2f162b73e0efc86e93e52e5ab72be73",
    "candidate.summary.json": "511abe306b42f4e4ed5af3f1fe99b1c40505fd97d597e0e88fab3f0fb1605340",
    "oracle.jsonl": "e9b921670232979db5d0fb3b6dfb045aad5d747c5f63f1385dfe372a9155c2a6",
    "oracle.summary.json": "921186214ade8d718a6b2d4211db02cbdd3a15b602254a86c8ce3b51aa378a0b",
    "promotion-receipt.json": "3f717d16e9cdca75e601ecdcea8185c82df9c42a6c4beebb94cb67ea76f5324d",
}
REPORT_SHA256 = {
    "candidate": "68e3ef530a241e78be140ecaa1e0afa02e69695a6d1f022b4795cee00db56fe5",
    "oracle": "be0516317e00d638e23154cb8e5bdc9916b80178b0043dc42d90c119697454ff",
}
PROVENANCE_SHA256 = {
    "candidate": "99b0a78772b5fcfcfb9e3e041fbfa8d044fa87959d52f157ef296adc4c16e10a",
    "oracle": "30a1d7048f5c0f12e0be70e0af63c381c9aa0c9e8edbebff68463e9fa098fa87",
}
REPORT_OUTCOMES = {
    "candidate": {"expected": 8, "unexpected": 5},
    "oracle": {"expected": 13, "unexpected": 0},
}
EXPECTED_RED_IDS = {
    "97193df5d3dc5207ad52-4671b258e95cb887a303",
    "97193df5d3dc5207ad52-c0eac18c86514cabfa91",
    "97193df5d3dc5207ad52-3fe6c068950339dcc208",
    "97193df5d3dc5207ad52-4e2bbdc06d6b8688f700",
    "97193df5d3dc5207ad52-ccd164d67f3cab736f4e",
}
FIXTURE_INPUT_SHA256 = {
    "tests/page/pageTest.ts": "ba3c28fbfff95bf482fc9647fdd4bc468997575cddcd60507ba0661e64b97b3d",
    "tests/config/utils.ts": "bc5818dc4ec7035ed3c22598f7bf9e8b1bd187de22014574e26f0d1146a38657",
    "tests/config/browserTest.ts": "0ef5440e164d262e7c3d01523448b5d3c4d50b99060604c20f168f04419a13d5",
    "tests/config/testserver/index.ts": "4330e0b94c88fdba07401efd3f4cd30a229bb48da037d8c880cd79ba0bae7307",
    "tests/library/playwright.config.ts": "338eb705bb4a17322dfba39d1c6770613bf8cf62cfec5c43bf7cece3e5db8146",
    "tests/assets/empty.html": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "tests/assets/frames/nested-frames.html": "d9b5820d68d7425551b0c5b7b3d08f717c9fa46d8465a4b42208f57769ef8bdf",
    "tests/assets/frames/frame.html": "ab70ea3dc4c8129241745a7884ee2b321225f341846c475e2540813a2ea8950b",
    "tests/assets/frames/two-frames.html": "14cbb3de85b237d85b5870805500fa9f91a15cef80c25a65eb05706ca60e4962",
    "tests/assets/frames/frameset.html": "7e1a6e0d67f83a5533e0630c386aef2830ba402d3d2ffc6fe96809217a5e98e1",
    "tests/assets/frames/one-frame.html": "1426a921774b42ff8aa66b06e10ba1500bc0371f5f44f61ad0f4642b94626efe",
    "tests/assets/frames/style.css": "5c78f332a9044b12a3bf3818abd921968834e766bbdf2268b6369a94b1c4a835",
    "tests/assets/frames/script.js": "577ca60854a02937e3be20a0f332174293023325711b2af7df664812bb923f87",
    "tests/assets/shadow.html": "6c545e1581e1395f5ecc44cdd1b6792b5559c640a020383e814cf1d66ea44dc5",
}
LANES = ("candidate", "oracle")
BUNDLE_NAMES = frozenset(V21_SHA256)


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


def report_path(lane: str) -> Path:
    require(lane in LANES, "unknown lane")
    name = "rust-owner-candidate" if lane == "candidate" else "oracle"
    return WORK / f"results/core-frame-hierarchy13-{name}-20261021-v1.json"


def provenance_path(lane: str) -> Path:
    return report_path(lane).with_suffix(".provenance.json")


def _source_ids(selected: dict[int, dict[str, Any]]) -> set[str]:
    require(len(selected) == TARGET_COUNT, "inventory has wrong hierarchy count")
    ids: set[str] = set()
    for index, row in selected.items():
        spec_id = row.get("specId")
        require(
            isinstance(spec_id, str)
            and spec_id not in ids
            and row.get("sourceFile") == SOURCE_FILE
            and row.get("sourceClassification") == "public-api-candidate"
            and row.get("classificationReasons") == []
            and row.get("expectedStatus") == "passed"
            and isinstance(row.get("line"), int)
            and row["line"] > 0,
            f"inventory hierarchy identity differs at {index}",
        )
        ids.add(spec_id)
    require(
        digest("".join(f"{spec_id}\n" for spec_id in sorted(ids)).encode())
        == SPEC_IDS_SHA256,
        "inventory hierarchy ID digest differs",
    )
    return ids


def report_rows(
    lane: str, data: bytes, target_ids: set[str]
) -> dict[str, dict[str, Any]]:
    require(lane in LANES, "unknown lane")
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
        )
        and report.get("errors") == [],
        f"{lane} report totals or top-level errors differ",
    )
    suites = report.get("suites")
    require(
        isinstance(suites, list) and len(suites) == 1, f"{lane} suite count differs"
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
        len(specs) == TARGET_COUNT, f"{lane} is not the complete thirteen-case file"
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
            f"{lane} missing or duplicate hierarchy invocation",
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
            f"{lane} unexecuted or retried invocation {spec_id}",
        )
        attempt = attempts[0]
        if test["status"] == "expected":
            require(
                attempt.get("status") == "passed"
                and not attempt.get("error")
                and not attempt.get("errors"),
                f"{lane} GREEN is not clean {spec_id}",
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
                f"{lane} RED lacks terminal error {spec_id}",
            )
            rows[spec_id] = {
                "outcome": "unexpected",
                "attempts": 1,
                "error": error["message"],
            }
    require(set(rows) == target_ids, f"{lane} hierarchy ID set differs")
    require(
        {
            outcome: sum(row["outcome"] == outcome for row in rows.values())
            for outcome in ("expected", "unexpected")
        }
        == REPORT_OUTCOMES[lane],
        f"{lane} per-ID outcomes contradict totals",
    )
    require(
        {spec_id for spec_id, row in rows.items() if row["outcome"] == "unexpected"}
        == (EXPECTED_RED_IDS if lane == "candidate" else set()),
        f"{lane} RED ID set differs",
    )
    return rows


def verify_provenance(lane: str, data: bytes) -> dict[str, Any]:
    require(lane in LANES, "unknown lane")
    value = object_from(data, f"{lane} provenance")
    expected_command = [
        "node",
        str(UPSTREAM / "packages/playwright/cli.js"),
        "test",
        "--config=tests/library/playwright.config.ts",
        "--reporter=line,json",
        "--workers=1",
        "--project=chromium-page",
        SOURCE_FILE,
    ]
    require(
        value.get("schema") == 2
        and value.get("mode") == lane
        and value.get("suite") == SUITE
        and value.get("suiteRole") == "browser-backed-official-file"
        and value.get("adapter")
        == (
            "test-mode-playwright-core"
            if lane == "candidate"
            else "test-mode-playwright-core-cdp"
        )
        and value.get("project") == PROJECT
        and value.get("repository") == "https://github.com/microsoft/playwright.git"
        and value.get("tag") == "v1.58.0"
        and value.get("commit") == COMMIT
        and value.get("upstream") == str(UPSTREAM)
        and value.get("manifest") == str(MANIFEST)
        and value.get("manifestSha256") == MANIFEST_SHA256
        and value.get("endpoint") == "http://127.0.0.1:9222"
        and value.get("fixtureHost") == "macbookpro-yuacx.tail8f3b66.ts.net"
        and value.get("fixtureBind") == "100.78.97.63"
        and value.get("filters") == [SOURCE_FILE]
        and value.get("workers") == 1
        and value.get("grep") is None
        and value.get("headed") is False
        and value.get("exitCode") == (1 if lane == "candidate" else 0)
        and value.get("result") == str(report_path(lane))
        and value.get("resultSha256") == REPORT_SHA256[lane]
        and value.get("command") == expected_command,
        f"{lane} provenance identity or runtime differs",
    )
    if lane == "candidate":
        require(
            value.get("candidate") == str(PACKAGE)
            and value.get("candidateProfile") == PROFILE
            and value.get("candidateArtifact") == str(ARTIFACT)
            and value.get("candidateArtifactSha256") == ARTIFACT_SHA256
            and value.get("candidateSourceSha256") == CANDIDATE_SOURCE_SHA256,
            "candidate frozen runtime differs",
        )
    else:
        require(
            all(
                value.get(key) is None
                for key in (
                    "candidate",
                    "candidateProfile",
                    "candidateArtifact",
                    "candidateArtifactSha256",
                    "candidateSourceSha256",
                )
            ),
            "Oracle provenance contains candidate runtime",
        )
    provenance_verifier.verify_provenance(value, provenance_path=provenance_path(lane))
    return value


def expected_summary(lane: str, old: dict[str, Any], ledger: bytes) -> dict[str, Any]:
    require(lane in LANES, "unknown lane")
    outcomes = dict(old["outcomes"])
    scopes = dict(old["explicitEvidenceScopes"])
    require(
        SCOPE not in scopes and outcomes["not-run"] >= TARGET_COUNT,
        "v22 already promoted",
    )
    outcomes["not-run"] -= TARGET_COUNT
    for outcome, count in REPORT_OUTCOMES[lane].items():
        outcomes[outcome] += count
    scopes[SCOPE] = TARGET_COUNT
    require(sum(outcomes.values()) == COUNT, "summary arithmetic differs")
    return {
        "schema": 1,
        "inventory": str(INVENTORY),
        "records": COUNT,
        "outcomes": dict(sorted(outcomes.items())),
        "explicitEvidenceScopes": dict(sorted(scopes.items())),
        "ledgerSha256": digest(ledger),
        "ledger": str(V22 / f"{lane}.jsonl"),
    }


def verify_inputs() -> dict[str, Any]:
    require(set(V21_SHA256) == BUNDLE_NAMES, "v21 hashes are incomplete")
    sealed(
        PREDECESSOR_VERIFIER, PREDECESSOR_VERIFIER_SHA256, "v21 independent verifier"
    )
    sealed(PROVENANCE_VERIFIER, PROVENANCE_VERIFIER_SHA256, "Core provenance verifier")
    old_receipt = predecessor.verify_existing()
    old_files = {
        name: sealed(V21 / name, sha, f"v21 {name}") for name, sha in V21_SHA256.items()
    }
    require(
        old_receipt.get("scope") == predecessor.SCOPE
        and old_receipt.get("recordsPerLane") == COUNT
        and old_receipt.get("fullCorpusExecution") is False
        and old_receipt.get("publicApiParityEvidence") is False
        and old_receipt.get("unstableSpecIds") == [predecessor.FRAMESET_ID],
        "v21 predecessor receipt differs",
    )
    inventory_lines = sealed(INVENTORY, INVENTORY_SHA256, "Core inventory").splitlines(
        keepends=True
    )
    require(len(inventory_lines) == COUNT, "Core inventory count differs")
    manifest = object_from(
        sealed(MANIFEST, MANIFEST_SHA256, "hierarchy manifest"), "hierarchy manifest"
    )
    require(
        manifest
        == {
            "schema": 1,
            "repository": "https://github.com/microsoft/playwright.git",
            "tag": "v1.58.0",
            "commit": COMMIT,
            "suites": [
                {
                    "id": SUITE,
                    "role": "browser-backed-official-file",
                    "runner": "packages/playwright/cli.js",
                    "config": "tests/library/playwright.config.ts",
                    "args": [SOURCE_FILE],
                    "candidateAdapter": "test-mode-playwright-core",
                    "oracleAdapter": "test-mode-playwright-core-cdp",
                    "project": PROJECT,
                    "requiresBrowser": True,
                    "expectedTestsPerProject": TARGET_COUNT,
                    "expectedFiles": 1,
                    "upstreamTestSourceSha256": SOURCE_SHA256,
                    "expectedSpecIdsSha256": SPEC_IDS_SHA256,
                }
            ],
        },
        "hierarchy manifest identity differs",
    )
    sealed(SOURCE, SOURCE_SHA256, "upstream hierarchy source")
    for relative, sha in FIXTURE_INPUT_SHA256.items():
        sealed(UPSTREAM / relative, sha, f"upstream fixture {relative}")
    revision = subprocess.run(
        ["git", "-C", str(UPSTREAM), "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    require(revision == COMMIT, "upstream revision differs")
    tracked = subprocess.run(
        ["git", "-C", str(UPSTREAM), "status", "--porcelain", "--untracked-files=no"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    require(not tracked, "upstream checkout has tracked changes")
    require(
        PACKAGE.is_dir()
        and not PACKAGE.is_symlink()
        and all(not path.is_symlink() for path in PACKAGE.rglob("*")),
        "frozen candidate snapshot missing or symlink",
    )
    sealed(PACKAGE / "package.json", PACKAGE_MANIFEST_SHA256, "frozen package manifest")
    sealed(ARTIFACT, ARTIFACT_SHA256, "frozen native addon")
    selected: dict[int, dict[str, Any]] = {}
    for index, raw in enumerate(inventory_lines):
        row = object_from(raw, f"inventory row {index}")
        if (
            row.get("suite") == "library"
            and row.get("projectId") == PROJECT
            and row.get("file") == FILE
        ):
            selected[index] = row
    target_ids = _source_ids(selected)
    old_lines = {
        lane: old_files[f"{lane}.jsonl"].splitlines(keepends=True) for lane in LANES
    }
    old_summaries = {
        lane: object_from(old_files[f"{lane}.summary.json"], f"v21 {lane} summary")
        for lane in LANES
    }
    for lane in LANES:
        require(len(old_lines[lane]) == COUNT, f"v21 {lane} ledger count differs")
        summary = old_summaries[lane]
        require(
            summary.get("schema") == 1
            and summary.get("records") == COUNT
            and summary.get("ledgerSha256") == V21_SHA256[f"{lane}.jsonl"]
            and old_receipt["lanes"][lane]["ledgerSha256"]
            == V21_SHA256[f"{lane}.jsonl"]
            and old_receipt["lanes"][lane]["summarySha256"]
            == V21_SHA256[f"{lane}.summary.json"]
            and sum(summary["outcomes"].values()) == COUNT,
            f"v21 {lane} summary chain differs",
        )
        for index, inventory_row in selected.items():
            old = object_from(old_lines[lane][index], f"v21 {lane} target row {index}")
            require(
                old == inventory_row | {"attempts": 0, "outcome": "not-run"},
                f"v21 {lane} target {index} is not an untested inventory row",
            )
    reports: dict[str, dict[str, dict[str, Any]]] = {}
    replacements: dict[str, dict[int, dict[str, Any]]] = {}
    for lane in LANES:
        verify_provenance(
            lane,
            sealed(
                provenance_path(lane), PROVENANCE_SHA256[lane], f"{lane} provenance"
            ),
        )
        reports[lane] = report_rows(
            lane,
            sealed(report_path(lane), REPORT_SHA256[lane], f"{lane} report"),
            target_ids,
        )
        replacements[lane] = {}
        for index, inventory_row in selected.items():
            old = object_from(old_lines[lane][index], f"v21 {lane} target {index}")
            new = dict(old)
            new.update(reports[lane][inventory_row["specId"]])
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
    return {
        "oldFiles": old_files,
        "oldLines": old_lines,
        "oldSummaries": old_summaries,
        "oldReceipt": old_receipt,
        "selected": selected,
        "reports": reports,
        "replacementRows": replacements,
    }


def expected_lane_receipt(
    lane: str, files: dict[str, bytes], checked: dict[str, Any]
) -> dict[str, Any]:
    new_lines = files[f"{lane}.jsonl"].splitlines(keepends=True)
    changed = [
        {
            "index": index,
            "specId": checked["selected"][index]["specId"],
            "oldRowSha256": digest(checked["oldLines"][lane][index]),
            "newRowSha256": digest(new_lines[index]),
            "oldOutcome": "not-run",
            "newOutcome": checked["replacementRows"][lane][index]["outcome"],
            "oldAttempts": 0,
            "newAttempts": 1,
        }
        for index in sorted(checked["selected"])
    ]
    summary = object_from(files[f"{lane}.summary.json"], f"v22 {lane} summary")
    return {
        "base": str(V21 / f"{lane}.jsonl"),
        "baseSha256": V21_SHA256[f"{lane}.jsonl"],
        "baseSummarySha256": V21_SHA256[f"{lane}.summary.json"],
        "ledger": str(V22 / f"{lane}.jsonl"),
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
        "v22 publisher absent or symlink",
    )
    candidate_red = {
        spec_id
        for spec_id, row in checked["reports"]["candidate"].items()
        if row["outcome"] == "unexpected"
    }
    oracle_red = {
        spec_id
        for spec_id, row in checked["reports"]["oracle"].items()
        if row["outcome"] == "unexpected"
    }
    require(
        candidate_red == EXPECTED_RED_IDS and not oracle_red,
        "five candidate REDs or Oracle baseline differ",
    )
    return {
        "schema": 1,
        "scope": SCOPE,
        "promotionKind": "browser-backed-public-api-full-file-paired-red-status",
        "priorBundle": str(V21),
        "priorBundleReceiptSha256": V21_SHA256["promotion-receipt.json"],
        "priorScope": checked["oldReceipt"]["scope"],
        "inheritedUnstableSpecIds": checked["oldReceipt"]["unstableSpecIds"],
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "manifest": str(MANIFEST),
        "manifestSha256": MANIFEST_SHA256,
        "upstreamSourceSha256": SOURCE_SHA256,
        "upstreamFixtureInputs": [
            {"path": relative, "sha256": sha}
            for relative, sha in sorted(FIXTURE_INPUT_SHA256.items())
        ],
        "targetSpecIdsSha256": SPEC_IDS_SHA256,
        "recordsPerLane": COUNT,
        "targetZeroBasedIndexes": sorted(checked["selected"]),
        "changedRowsPerLane": {lane: TARGET_COUNT for lane in LANES},
        "unchangedRawRowsPerLane": {lane: COUNT - TARGET_COUNT for lane in LANES},
        "fixtureAdapted": True,
        "browserExecutionEvidence": True,
        "completeFileExecution": True,
        "fullCorpusExecution": False,
        "launchMode": "cdp-adapter",
        "publicApiParityEvidence": False,
        "upstreamAssertionsModified": False,
        "candidateExpectedRows": REPORT_OUTCOMES["candidate"]["expected"],
        "candidateUnexpectedRows": REPORT_OUTCOMES["candidate"]["unexpected"],
        "oracleExpectedRows": REPORT_OUTCOMES["oracle"]["expected"],
        "oracleUnexpectedRows": REPORT_OUTCOMES["oracle"]["unexpected"],
        "redComparison": {
            "candidateOnlySpecIds": sorted(candidate_red - oracle_red),
            "bothRedSpecIds": sorted(candidate_red & oracle_red),
            "oracleOnlySpecIds": sorted(oracle_red - candidate_red),
            "candidateOnlyCount": len(candidate_red - oracle_red),
            "bothRedCount": len(candidate_red & oracle_red),
            "oracleOnlyCount": len(oracle_red - candidate_red),
        },
        "candidateProfile": PROFILE,
        "candidateArtifactSha256": ARTIFACT_SHA256,
        "candidateSourceSha256": CANDIDATE_SOURCE_SHA256,
        "candidatePackage": str(PACKAGE),
        "candidatePackageManifestSha256": PACKAGE_MANIFEST_SHA256,
        "promoterSourceSha256": digest(PROMOTER.read_bytes()),
        "verifierSourceSha256": digest(Path(__file__).read_bytes()),
        "lanes": {lane: expected_lane_receipt(lane, files, checked) for lane in LANES},
    }


def verify_payloads(
    files: dict[str, bytes], checked: dict[str, Any] | None = None
) -> dict[str, Any]:
    require(set(files) == BUNDLE_NAMES, "v22 bundle files differ")
    inputs = checked if checked is not None else verify_inputs()
    for lane in LANES:
        old_lines = inputs["oldLines"][lane]
        new_lines = files[f"{lane}.jsonl"].splitlines(keepends=True)
        require(len(new_lines) == COUNT, f"v22 {lane} row count differs")
        replacements = inputs["replacementRows"][lane]
        for index, (old_raw, new_raw) in enumerate(
            zip(old_lines, new_lines, strict=True)
        ):
            require(
                new_raw
                == (
                    canonical_line(replacements[index])
                    if index in replacements
                    else old_raw
                ),
                f"v22 {lane} {'target' if index in replacements else 'off-target raw'} row {index} differs",
            )
        require(
            object_from(files[f"{lane}.summary.json"], f"v22 {lane} summary")
            == expected_summary(
                lane, inputs["oldSummaries"][lane], files[f"{lane}.jsonl"]
            ),
            f"v22 {lane} summary differs",
        )
    receipt = object_from(files["promotion-receipt.json"], "v22 receipt")
    require(receipt == expected_receipt(files, inputs), "v22 receipt differs")
    return receipt


def verify_existing() -> dict[str, Any]:
    require(V22.is_dir() and not V22.is_symlink(), "v22 bundle is absent or symlink")
    require(
        {path.name for path in V22.iterdir()} == BUNDLE_NAMES, "v22 file set differs"
    )
    require(
        all(
            (V22 / name).is_file() and not (V22 / name).is_symlink()
            for name in BUNDLE_NAMES
        ),
        "v22 file absent or symlink",
    )
    return verify_payloads({name: (V22 / name).read_bytes() for name in BUNDLE_NAMES})


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
            2,
            f"strict paired Core frame-hierarchy v22 verification rejected: {error}\n",
        )
    print(
        json.dumps(
            {"verified": True, "scope": receipt["scope"], "destination": str(V22)},
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
