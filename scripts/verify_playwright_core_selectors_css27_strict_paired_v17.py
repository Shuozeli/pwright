"""Independently verify the immutable selectors-css27 Core paired successor.

The verifier reads the two unchanged official reports, their source-bound
provenance, the pinned inventory, and the sealed dialog v16 predecessor. It
never imports the v17 publisher or reruns browser tests.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib
import json
import subprocess
from pathlib import Path
from typing import Any

import verify_playwright_official_provenance as generic_provenance

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / ".work/official-conformance"
V16 = WORK / "outcomes/core-page-dialog10-strict-paired-v16"
V17 = WORK / "outcomes/core-selectors-css27-strict-paired-v17"
INVENTORY = WORK / "playwright-core-v1.58.0.jsonl"
MANIFEST = (
    ROOT
    / "tests/official/playwright-core-selectors-css-public-chromium-v1-manifest.json"
)
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-v1.58.0")
SOURCE = UPSTREAM / "tests/page/selectors-css.spec.ts"
PROMOTER = ROOT / "scripts/promote_playwright_core_selectors_css27_strict_paired_v17.py"
PREDECESSOR_VERIFIER = (
    ROOT / "scripts/verify_playwright_core_page_dialog10_strict_paired_v16.py"
)
PROVENANCE_VERIFIER = ROOT / "scripts/verify_playwright_official_provenance.py"
PACKAGE = WORK / "builds/core-dialog10-evaluation-clean-20261021-v1/candidate"
ARTIFACT = PACKAGE / "native/official-conformance-page-check/pwright.node"
COUNT = 48_477
TARGET_COUNT = 27
FILE = "page/selectors-css.spec.ts"
PROJECT = "chromium-page"
SUITE = "library-selectors-css-public-chromium"
SCOPE = "official-browser-backed-selectors-css-full-file-paired-v17"
PRIOR_SCOPE = "official-browser-backed-page-dialog-full-file-paired-v16"
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
MANIFEST_SHA256 = "2d1a9faa57877dbcb385ce6fb9540d61bc24947ddea11cf54f0e154f1b59d723"
SOURCE_SHA256 = "f0a096c63febc7b2486dc15d3c26aca4f24f2beaa4f5ad1dc96b6eb5a1ba68f5"
SPEC_IDS_SHA256 = "72b5dc160dfd74f0ff5a8f218dab4eff7a0cd22270e116d64b6101f5e7e6ba39"
PROVENANCE_VERIFIER_SHA256 = (
    "9b1c6c4edee469c3c7a56866444fb5ad1bafe31443164f497c562a8eda1e0672"
)
CANDIDATE_PROFILE = "official-conformance-page-check"
CANDIDATE_SOURCE_SHA256 = (
    "d886564b6d7b99457fea41ff4716b3f7d42388aeb75aca4fde2b075c4d89f43c"
)
CANDIDATE_ARTIFACT_SHA256 = (
    "9be17a4945465b2355f9a1ee3f3ed8996ce6a5cf0b5596535ac1b56dc2e56a93"
)
PACKAGE_MANIFEST_SHA256 = (
    "40b301caa57ef941fd2293bd1becfdbb0cf513d5634edb43883fb251af1fb458"
)
PREDECESSOR_VERIFIER_SHA256 = (
    "7b7608a0a9ec9f1695e5037e83aea93216c42714c1d0a264f509f00a5c4c9f18"
)
V16_SHA256 = {
    "candidate.jsonl": "856678769a96218bb2c3d3954b551bd6c6684f8a2e8a2119a695caf43222d3b2",
    "candidate.summary.json": "4dc1ad76bb986aac44e56c02e8fd41564084930049f998ff792465a658e66652",
    "oracle.jsonl": "f1b1f4f3bd9aa733cd26047b4e23eaab56003173667a3ca3facdf7080bea6125",
    "oracle.summary.json": "8f974aea8a0103e8dd8514e6b185088bc234f51d88cd32862bb538a572f3872b",
    "promotion-receipt.json": "60277dcb60601de2f890c8f3ced2d8475363d0aa9c1fcffab65c90d7e5df5c1f",
}
LANES = ("candidate", "oracle")
REPORT_SHA256 = {
    "candidate": "f4943108bec4ccc69c53be00b3b848cce647bc3e481d3ed292978862fcb3239a",
    "oracle": "3d28631fffd70007eb1cb1050a073d1b75fa30a0c5884cecc51a1570a88a87ac",
}
PROVENANCE_SHA256 = {
    "candidate": "c0957cab9ba2e6ec7c323c81d0e540bd90a76023af85dd83cbed2fb93a305a55",
    "oracle": "5a1ade74b9fcffa7059ebc2c048c650f8775fed629a8f87e7452f69b15a8c551",
}
REPORT_OUTCOMES = {
    "candidate": {"expected": 14, "unexpected": 13},
    "oracle": {"expected": 27, "unexpected": 0},
}
BUNDLE_NAMES = {
    "candidate.jsonl",
    "candidate.summary.json",
    "oracle.jsonl",
    "oracle.summary.json",
    "promotion-receipt.json",
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical_line(row: dict[str, Any]) -> bytes:
    return (json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n").encode()


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


def report_path(lane: str) -> Path:
    require(lane in LANES, "unknown lane")
    return WORK / f"results/core-selectors-css27-{lane}-20261021-v1.json"


def provenance_path(lane: str) -> Path:
    return report_path(lane).with_suffix(".provenance.json")


def _source_ids(selected: dict[int, dict[str, Any]]) -> set[str]:
    require(len(selected) == TARGET_COUNT, "inventory has wrong selectors-css count")
    ids: set[str] = set()
    for index, row in selected.items():
        spec_id = row.get("specId")
        require(
            isinstance(spec_id, str)
            and bool(spec_id)
            and spec_id not in ids
            and row.get("sourceFile") == "tests/page/selectors-css.spec.ts"
            and row.get("sourceClassification") == "public-api-candidate"
            and row.get("expectedStatus") == "passed"
            and isinstance(row.get("line"), int)
            and row["line"] > 0,
            f"inventory selectors-css identity differs at {index}",
        )
        ids.add(spec_id)
    ids_hash = digest("".join(f"{spec_id}\n" for spec_id in sorted(ids)).encode())
    require(ids_hash == SPEC_IDS_SHA256, "inventory selectors-css ID digest differs")
    return ids


def verify_provenance(lane: str, data: bytes) -> dict[str, Any]:
    value = object_from(data, f"{lane} provenance")
    expected_command = [
        "node",
        str(UPSTREAM / "packages/playwright/cli.js"),
        "test",
        "--config=tests/library/playwright.config.ts",
        "--reporter=line,json",
        "--workers=1",
        "--project=chromium-page",
        "tests/page/selectors-css.spec.ts",
    ]
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
        and value.get("filters") == ["tests/page/selectors-css.spec.ts"]
        and value.get("workers") == 1
        and value.get("grep") is None
        and value.get("headed") is False
        and value.get("exitCode") == (1 if lane == "candidate" else 0)
        and value.get("command") == expected_command
        and value.get("adapter")
        == (
            "test-mode-playwright-core"
            if lane == "candidate"
            else "test-mode-playwright-core-cdp"
        ),
        f"{lane} provenance identity differs",
    )
    if lane == "candidate":
        require(
            value.get("candidate") == str(PACKAGE)
            and value.get("candidateProfile") == CANDIDATE_PROFILE
            and value.get("candidateSourceSha256") == CANDIDATE_SOURCE_SHA256
            and value.get("candidateArtifact") == str(ARTIFACT)
            and value.get("candidateArtifactSha256") == CANDIDATE_ARTIFACT_SHA256,
            "candidate runtime pin differs",
        )
    else:
        require(
            all(
                value.get(key) is None
                for key in (
                    "candidate",
                    "candidateProfile",
                    "candidateSourceSha256",
                    "candidateArtifact",
                    "candidateArtifactSha256",
                )
            ),
            "oracle provenance contains candidate runtime",
        )
    # This generic verifier is itself sealed before use. It independently
    # checks the source, runner, preload, artifact, result and exact 27-ID set.
    generic_provenance.verify_provenance(value, provenance_path=provenance_path(lane))
    return value


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
            isinstance(children, list) and isinstance(entries, list), "malformed suite"
        )
        for spec in entries:
            require(isinstance(spec, dict), "malformed report spec")
            specs.append(spec)
        for child in children:
            visit(child)

    for suite in suites:
        visit(suite)
    require(
        len(specs) == TARGET_COUNT, f"{lane} report is not complete selectors-css file"
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
            f"{lane} wrong, missing or duplicate selector invocation",
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
            f"{lane} unexecuted or malformed selector invocation {spec_id}",
        )
        attempt = attempts[0]
        if test["status"] == "expected":
            require(
                attempt.get("status") == "passed"
                and not attempt.get("error")
                and not attempt.get("errors"),
                f"{lane} GREEN selector invocation is not a clean pass {spec_id}",
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
                f"{lane} RED selector invocation lacks terminal error {spec_id}",
            )
            rows[spec_id] = {
                "outcome": "unexpected",
                "attempts": 1,
                "error": error["message"],
            }
    require(set(rows) == target_ids, f"{lane} selectors-css ID set differs")
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
    outcomes = dict(old_summary["outcomes"])
    scopes = dict(old_summary["explicitEvidenceScopes"])
    require(
        SCOPE not in scopes and outcomes["not-run"] >= TARGET_COUNT,
        "v17 already promoted",
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
        "ledger": str(V17 / f"{lane}.jsonl"),
    }


def verify_inputs() -> dict[str, Any]:
    require(
        len(V16_SHA256) == len(BUNDLE_NAMES) and set(V16_SHA256) == BUNDLE_NAMES,
        "v16 predecessor hashes not sealed",
    )
    sealed(PREDECESSOR_VERIFIER, PREDECESSOR_VERIFIER_SHA256, "v16 verifier")
    predecessor = importlib.import_module(
        "verify_playwright_core_page_dialog10_strict_paired_v16"
    )
    predecessor.verify_existing()
    sealed(
        PROVENANCE_VERIFIER, PROVENANCE_VERIFIER_SHA256, "generic provenance verifier"
    )
    inventory_lines = sealed(INVENTORY, INVENTORY_SHA256, "Core inventory").splitlines(
        keepends=True
    )
    require(len(inventory_lines) == COUNT, "Core inventory row count differs")
    manifest = object_from(
        sealed(MANIFEST, MANIFEST_SHA256, "selectors manifest"), "manifest"
    )
    suite = manifest.get("suites")
    require(
        manifest.get("schema") == 1
        and manifest.get("repository") == "https://github.com/microsoft/playwright.git"
        and manifest.get("tag") == "v1.58.0"
        and manifest.get("commit") == COMMIT
        and isinstance(suite, list)
        and len(suite) == 1
        and suite[0]
        == {
            "id": SUITE,
            "role": "browser-backed-official-file",
            "runner": "packages/playwright/cli.js",
            "config": "tests/library/playwright.config.ts",
            "args": ["tests/page/selectors-css.spec.ts"],
            "candidateAdapter": "test-mode-playwright-core",
            "oracleAdapter": "test-mode-playwright-core-cdp",
            "project": PROJECT,
            "requiresBrowser": True,
            "expectedTestsPerProject": TARGET_COUNT,
            "expectedFiles": 1,
            "upstreamTestSourceSha256": SOURCE_SHA256,
            "expectedSpecIdsSha256": SPEC_IDS_SHA256,
        },
        "selectors manifest identity differs",
    )
    sealed(SOURCE, SOURCE_SHA256, "upstream selectors-css source")
    revision = subprocess.run(
        ["git", "-C", str(UPSTREAM), "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    require(revision == COMMIT, "upstream checkout revision differs")
    tracked = subprocess.run(
        ["git", "-C", str(UPSTREAM), "status", "--porcelain", "--untracked-files=no"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    require(not tracked, "upstream checkout has tracked changes")
    sealed(
        PACKAGE / "package.json", PACKAGE_MANIFEST_SHA256, "candidate package manifest"
    )
    sealed(ARTIFACT, CANDIDATE_ARTIFACT_SHA256, "candidate native artifact")
    selected: dict[int, dict[str, Any]] = {}
    for index, raw in enumerate(inventory_lines):
        row = json.loads(raw)
        if (
            row.get("suite") == "library"
            and row.get("projectId") == PROJECT
            and row.get("file") == FILE
        ):
            selected[index] = row
    target_ids = _source_ids(selected)
    old_files = {
        name: sealed(V16 / name, sha, f"v16 {name}") for name, sha in V16_SHA256.items()
    }
    old_receipt = object_from(old_files["promotion-receipt.json"], "v16 receipt")
    require(
        old_receipt.get("scope") == PRIOR_SCOPE
        and old_receipt.get("recordsPerLane") == COUNT
        and old_receipt.get("fullCorpusExecution") is False
        and old_receipt.get("publicApiParityEvidence") is False,
        "v16 predecessor scope differs",
    )
    old_lines: dict[str, list[bytes]] = {}
    old_summaries: dict[str, dict[str, Any]] = {}
    replacements: dict[str, dict[int, dict[str, Any]]] = {}
    reports: dict[str, dict[str, dict[str, Any]]] = {}
    for lane in LANES:
        lines = old_files[f"{lane}.jsonl"].splitlines(keepends=True)
        require(len(lines) == COUNT, f"v16 {lane} row count differs")
        old_lines[lane] = lines
        summary = object_from(old_files[f"{lane}.summary.json"], f"v16 {lane} summary")
        require(
            summary.get("schema") == 1
            and summary.get("records") == COUNT
            and summary.get("ledgerSha256") == V16_SHA256[f"{lane}.jsonl"]
            and old_receipt["lanes"][lane]["ledgerSha256"]
            == V16_SHA256[f"{lane}.jsonl"]
            and old_receipt["lanes"][lane]["summarySha256"]
            == V16_SHA256[f"{lane}.summary.json"]
            and old_receipt["lanes"][lane]["outcomes"] == summary["outcomes"]
            and sum(summary["outcomes"].values()) == COUNT,
            f"v16 {lane} summary or receipt chain differs",
        )
        old_summaries[lane] = summary
        verify_provenance(
            lane,
            sealed(
                provenance_path(lane), PROVENANCE_SHA256[lane], f"{lane} provenance"
            ),
        )
        report = sealed(report_path(lane), REPORT_SHA256[lane], f"{lane} report")
        reports[lane] = report_rows(lane, report, target_ids)
        replacements[lane] = {}
        for index, inventory_row in selected.items():
            old = object_from(lines[index], f"v16 {lane} selector target {index}")
            require(
                old == inventory_row | {"attempts": 0, "outcome": "not-run"},
                f"v16 {lane} target {index} is not untested inventory row",
            )
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
    require(
        all(row["outcome"] == "expected" for row in reports["oracle"].values()),
        "selectors Oracle report is not 27/27",
    )
    return {
        "oldLines": old_lines,
        "oldSummaries": old_summaries,
        "oldReceipt": old_receipt,
        "selected": selected,
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
    summary = object_from(files[f"{lane}.summary.json"], f"v17 {lane} summary")
    return {
        "base": str(V16 / f"{lane}.jsonl"),
        "baseSha256": V16_SHA256[f"{lane}.jsonl"],
        "baseSummarySha256": V16_SHA256[f"{lane}.summary.json"],
        "ledger": str(V17 / f"{lane}.jsonl"),
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
    require(PROMOTER.is_file() and not PROMOTER.is_symlink(), "v17 publisher absent")
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
    return {
        "schema": 1,
        "scope": SCOPE,
        "promotionKind": "browser-backed-public-api-full-file-paired-red-status",
        "priorBundle": str(V16),
        "priorBundleReceiptSha256": V16_SHA256["promotion-receipt.json"],
        "priorScope": checked["oldReceipt"]["scope"],
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "manifest": str(MANIFEST),
        "manifestSha256": MANIFEST_SHA256,
        "upstreamSourceSha256": SOURCE_SHA256,
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
        "candidateProfile": CANDIDATE_PROFILE,
        "candidateArtifactSha256": CANDIDATE_ARTIFACT_SHA256,
        "candidateSourceSha256": CANDIDATE_SOURCE_SHA256,
        "candidatePackage": str(PACKAGE),
        "candidatePackageManifest": str(PACKAGE / "package.json"),
        "candidatePackageManifestSha256": PACKAGE_MANIFEST_SHA256,
        "promoterSourceSha256": digest(PROMOTER.read_bytes()),
        "verifierSourceSha256": digest(Path(__file__).read_bytes()),
        "lanes": {lane: expected_lane_receipt(lane, files, checked) for lane in LANES},
    }


def verify_payloads(
    files: dict[str, bytes], checked: dict[str, Any] | None = None
) -> dict[str, Any]:
    require(set(files) == BUNDLE_NAMES, "v17 bundle files differ")
    inputs = checked if checked is not None else verify_inputs()
    for lane in LANES:
        old_lines = inputs["oldLines"][lane]
        new_lines = files[f"{lane}.jsonl"].splitlines(keepends=True)
        require(len(new_lines) == COUNT, f"v17 {lane} row count differs")
        replacements = inputs["replacementRows"][lane]
        for index, (old_raw, new_raw) in enumerate(
            zip(old_lines, new_lines, strict=True)
        ):
            if index in replacements:
                require(
                    new_raw == canonical_line(replacements[index]),
                    f"v17 {lane} target row {index} differs",
                )
            else:
                require(
                    new_raw == old_raw, f"v17 {lane} off-target raw row {index} changed"
                )
        summary = object_from(files[f"{lane}.summary.json"], f"v17 {lane} summary")
        require(
            summary
            == expected_summary(
                lane, inputs["oldSummaries"][lane], files[f"{lane}.jsonl"]
            ),
            f"v17 {lane} summary differs",
        )
    receipt = object_from(files["promotion-receipt.json"], "v17 receipt")
    require(receipt == expected_receipt(files, inputs), "v17 receipt differs")
    return receipt


def verify_existing() -> dict[str, Any]:
    require(V17.is_dir() and not V17.is_symlink(), "v17 bundle absent or symlink")
    require(
        {path.name for path in V17.iterdir()} == BUNDLE_NAMES, "v17 file set differs"
    )
    require(
        all(
            (V17 / name).is_file() and not (V17 / name).is_symlink()
            for name in BUNDLE_NAMES
        ),
        "v17 file absent or symlink",
    )
    return verify_payloads({name: (V17 / name).read_bytes() for name in BUNDLE_NAMES})


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
            2, f"strict Core selectors-css v17 verification rejected: {error}\n"
        )
    print(
        json.dumps(
            {"verified": True, "scope": receipt["scope"], "destination": str(V17)},
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
