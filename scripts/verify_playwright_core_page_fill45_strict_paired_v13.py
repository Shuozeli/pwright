"""Independently replay the immutable 45-case Core Page.fill RED-to-GREEN pair.

The v12 ledger, frozen v2 candidate package, full-file manifest, and two
source-bound official reports are sealed inputs. Only the historical candidate
RED row may change; this verifier never imports the v13 publisher.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / ".work/official-conformance"
V12 = WORK / "outcomes/core-page-select-option32-strict-paired-v12"
V13 = WORK / "outcomes/core-page-fill45-strict-paired-v13"
INVENTORY = WORK / "playwright-core-v1.58.0.jsonl"
MANIFEST = (
    ROOT / "tests/official/playwright-core-page-fill-public-chromium-v1-manifest.json"
)
SOURCE = Path(
    "/Users/cyuan/projects/thirdparty/playwright-v1.58.0/tests/page/page-fill.spec.ts"
)
UPSTREAM = SOURCE.parents[2]
PACKAGE = WORK / "builds/core-fill45-post-binding-20261020-v2/candidate"
PACKAGE_MANIFEST = PACKAGE.parent / "candidate-manifest.json"
PROMOTER = ROOT / "scripts/promote_playwright_core_page_fill45_strict_paired_v13.py"
COUNT = 48_477
TARGET_COUNT = 45
FILE = "page/page-fill.spec.ts"
PROJECT = "chromium-page"
CAPTURE_SUITE = "library-page-fill-public-chromium"
SCOPE_V12 = "official-browser-backed-page-select-option-full-file-paired-v12"
SCOPE_V13 = "official-browser-backed-page-fill-full-file-paired-v13"
RED_SPEC_ID = "93a63d1471fbabaebc58-fc74ff33620dd81e92d1"
RED_INDEX = 39_564
RED_SOURCE_LINE = 388
RED_ERROR = (
    "Error: page.fill: Backend: CDP error -32000: Cannot find context with specified id"
)
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
MANIFEST_SHA256 = "bcc762635dfd7f61a26e078484917b7fb6eb0e4c81030e2c7f71ecaa6bd53c09"
SOURCE_SHA256 = "e2dbc71d151ad9635359ee6d1808f71acd6d70109212a4fe4c0265f3aeffad77"
SPEC_IDS_SHA256 = "b75daac089d56a8845a13f81cc217f0fd97bd8cebed9e6bab0f5ed3a44a7d21d"
V12_SHA256 = {
    "candidate.jsonl": "9dcc8795144898c3dc4f1624777855279045e9e9c90f8ca49d1b5dae43030d05",
    "candidate.summary.json": "588238dc037d16aafa55ad8280938079fe0c2ea80c53b169152c93b6cb301ae7",
    "oracle.jsonl": "1baef074b29f90a49caa8ce4ae8620499359f03a0017fc0a98a4d1106c271303",
    "oracle.summary.json": "773b95ab3593fc3d0cf0d7c823bad400bd0c110908ae90be5ba5102b5e1e0b7b",
    "promotion-receipt.json": "40c861162bbec094b8729927366000ac3b3026586ce98d16e48dcedb1e6c2dc3",
}
REPORT_SHA256 = {
    "candidate": "946371fe9389e0568392444a9b7543cd29e49faecbcd85a683c6b0a18e88d56a",
    "oracle": "f9af32ce78d1c82bea4d8375d501fc512dd7cbfd6734b1fc775506729540b3ac",
}
PROVENANCE_SHA256 = {
    "candidate": "20047fc9e1cfd9a62bd86619248ade501e908033041cc260531910b3d05e80fc",
    "oracle": "25e5156abdb19ad5214e40d2db5cd59799ff2aefdf2347f3bfe48e827fdb8f6c",
}
PACKAGE_MANIFEST_SHA256 = (
    "c1501af04093b8d6c197d190915eb6ab039b3ae7c3ffee394f8fcb371f92f037"
)
CANDIDATE_PROFILE = "official-conformance-page-fill-navigation-commit"
CANDIDATE_ARTIFACT_SHA256 = (
    "12c0d93f9e66798023f7380a64fa835e9f5eae6887575d9864f50a74abb97636"
)
CANDIDATE_SOURCE_SHA256 = (
    "8f7871adeb009eed6bdd097f6317e8f16c8df84116b04e23ac652faa3ac0367f"
)
ARTIFACT = PACKAGE / f"native/{CANDIDATE_PROFILE}/pwright.node"
CHROME_VERSION_OBSERVED = "155.0.8059.85"
LANES = ("candidate", "oracle")
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
        "21f1df10e77485bf29b67b31e548a1bc5126d50dc47cdeb115a9cf6798704a7b",
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
    require(path.is_file() and not path.is_symlink(), f"{label} is absent or a symlink")
    data = path.read_bytes()
    require(digest(data) == expected, f"{label} SHA-256 changed")
    return data


def object_from(data: bytes, label: str) -> dict[str, Any]:
    value = json.loads(data)
    require(isinstance(value, dict), f"{label} is not an object")
    return value


def report_path(lane: str) -> Path:
    require(lane in LANES, "unknown lane")
    return WORK / f"results/core-fill45-{lane}-20261020-v2.json"


def provenance_path(lane: str) -> Path:
    return report_path(lane).with_suffix(".provenance.json")


def verify_package() -> None:
    manifest = object_from(
        sealed(PACKAGE_MANIFEST, PACKAGE_MANIFEST_SHA256, "frozen candidate manifest"),
        "frozen candidate manifest",
    )
    require(
        set(manifest)
        == {
            "schema",
            "profile",
            "candidateSourceSha256",
            "artifactSha256",
            "sourcePackage",
            "snapshotPackage",
            "files",
        }
        and manifest["schema"] == 1
        and manifest["profile"] == CANDIDATE_PROFILE
        and manifest["candidateSourceSha256"] == CANDIDATE_SOURCE_SHA256
        and manifest["artifactSha256"] == CANDIDATE_ARTIFACT_SHA256
        and manifest["sourcePackage"] == str(ROOT / "packages/pwright")
        and manifest["snapshotPackage"] == str(PACKAGE)
        and isinstance(manifest["files"], list)
        and len(manifest["files"]) == 68,
        "frozen candidate manifest identity differs",
    )
    require(
        PACKAGE.is_dir() and not PACKAGE.is_symlink(),
        "frozen package is absent or a symlink",
    )
    listed: set[str] = set()
    for item in manifest["files"]:
        require(
            isinstance(item, dict) and set(item) == {"path", "sha256", "size"},
            "malformed candidate manifest file",
        )
        relative = Path(item["path"])
        require(
            not relative.is_absolute()
            and relative.parts
            and all(part not in {".", ".."} for part in relative.parts)
            and relative.as_posix() not in listed,
            "candidate manifest path is unsafe or duplicate",
        )
        listed.add(relative.as_posix())
        path = PACKAGE / relative
        require(
            path.is_file()
            and not path.is_symlink()
            and path.stat().st_size == item["size"],
            f"candidate package file is absent, changed, or a symlink: {relative}",
        )
        sealed(path, item["sha256"], f"candidate package file {relative}")
    actual: set[str] = set()
    for path in PACKAGE.rglob("*"):
        require(not path.is_symlink(), f"candidate package symlink: {path}")
        if path.is_file():
            actual.add(path.relative_to(PACKAGE).as_posix())
        else:
            require(path.is_dir(), f"candidate package special file: {path}")
    require(actual == listed, "candidate package file set differs")
    require(
        next(
            item["sha256"]
            for item in manifest["files"]
            if item["path"] == ARTIFACT.relative_to(PACKAGE).as_posix()
        )
        == CANDIDATE_ARTIFACT_SHA256,
        "candidate native artifact pin differs",
    )


def report_rows(
    lane: str, data: bytes, target_ids: set[str]
) -> dict[str, dict[str, int | str]]:
    report = object_from(data, f"{lane} official report")
    stats = report.get("stats")
    require(
        isinstance(stats, dict)
        and all(
            stats.get(key) == value
            for key, value in {
                "expected": TARGET_COUNT,
                "unexpected": 0,
                "skipped": 0,
                "flaky": 0,
            }.items()
        ),
        f"{lane} report is not 45/45 GREEN",
    )
    specs: list[dict[str, Any]] = []

    def visit(suite: dict[str, Any]) -> None:
        require(isinstance(suite, dict), "malformed official report suite")
        children = suite.get("suites", [])
        entries = suite.get("specs", [])
        require(
            isinstance(children, list) and isinstance(entries, list),
            "malformed report suite children",
        )
        for spec in entries:
            require(isinstance(spec, dict), "malformed official report spec")
            specs.append(spec)
        for child in children:
            visit(child)

    suites = report.get("suites")
    require(
        isinstance(suites, list) and len(suites) == 1,
        f"{lane} report suite count differs",
    )
    for suite in suites:
        visit(suite)
    require(len(specs) == TARGET_COUNT, f"{lane} report is not the full 45-case file")
    rows: dict[str, dict[str, int | str]] = {}
    for spec in specs:
        spec_id = spec.get("id")
        tests = spec.get("tests")
        require(
            spec.get("file") == FILE
            and isinstance(spec_id, str)
            and spec_id in target_ids
            and spec_id not in rows
            and isinstance(tests, list)
            and len(tests) == 1
            and isinstance(tests[0], dict),
            f"{lane} report has a wrong, missing, or duplicate invocation",
        )
        test = tests[0]
        attempts = test.get("results")
        require(
            test.get("projectId") == PROJECT
            and test.get("expectedStatus") == "passed"
            and test.get("status") == "expected"
            and isinstance(attempts, list)
            and len(attempts) == 1
            and isinstance(attempts[0], dict)
            and attempts[0].get("status") == "passed"
            and attempts[0].get("retry") == 0
            and not attempts[0].get("error")
            and not attempts[0].get("errors"),
            f"{lane} report has an unexecuted or failing invocation for {spec_id}",
        )
        rows[spec_id] = {"outcome": "expected", "attempts": 1}
    require(set(rows) == target_ids, f"{lane} report does not cover exact target IDs")
    return rows


def _verify_recorded_sources(
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
        and value.get("suite") == CAPTURE_SUITE
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
        and value.get("filters") == ["tests/page/page-fill.spec.ts"]
        and value.get("workers") == 1
        and value.get("grep") is None
        and value.get("headed") is False
        and value.get("exitCode") == 0
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
            "tests/page/page-fill.spec.ts",
        ],
        f"{lane} provenance identity or source pin differs",
    )
    _verify_recorded_sources(
        value.get("toolingSources"), TOOLING_SOURCES, f"{lane} tooling"
    )
    _verify_recorded_sources(
        value.get("harnessInputs"),
        HARNESS_SOURCES[lane],
        f"{lane} harness",
        absolute=True,
    )
    if lane == "candidate":
        require(
            value.get("candidate") == str(PACKAGE)
            and value.get("candidateArtifact") == str(ARTIFACT)
            and value.get("candidateProfile") == CANDIDATE_PROFILE
            and value.get("candidateArtifactSha256") == CANDIDATE_ARTIFACT_SHA256
            and value.get("candidateSourceSha256") == CANDIDATE_SOURCE_SHA256,
            "candidate provenance package pin differs",
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
            "oracle provenance contains a candidate runtime",
        )


def expected_red_row(old: dict[str, Any]) -> dict[str, Any]:
    require(
        old.get("specId") == RED_SPEC_ID
        and old.get("line") == RED_SOURCE_LINE
        and old.get("outcome") == "unexpected"
        and old.get("attempts") == 1
        and old.get("error") == RED_ERROR,
        "wrong historical Page.fill RED row",
    )
    new = {**old}
    new.pop("error")
    new.update(
        {
            "attempts": 2,
            "outcome": "expected",
            "resultFile": str(report_path("candidate")),
            "evidenceScope": SCOPE_V13,
            "fixtureAdapted": True,
            "browserExecutionEvidence": True,
            "publicApiParityEvidence": False,
        }
    )
    return new


def expected_summary(lane: str, old: dict[str, Any], ledger: bytes) -> dict[str, Any]:
    require(lane in LANES, "unknown lane")
    outcomes = dict(old["outcomes"])
    scopes = dict(old["explicitEvidenceScopes"])
    if lane == "candidate":
        require(
            SCOPE_V13 not in scopes and outcomes["unexpected"] >= 1,
            "v13 already promoted",
        )
        outcomes["expected"] += 1
        outcomes["unexpected"] -= 1
        scopes[SCOPE_V13] = 1
    return {
        "schema": 1,
        "inventory": str(INVENTORY),
        "records": COUNT,
        "outcomes": dict(sorted(outcomes.items())),
        "explicitEvidenceScopes": dict(sorted(scopes.items())),
        "ledgerSha256": digest(ledger),
        "ledger": str(V13 / f"{lane}.jsonl"),
    }


def verify_prior_chain() -> None:
    """Check every predecessor receipt link and both sealed lane payloads."""
    cursor = V12
    expected_receipt_sha = V12_SHA256["promotion-receipt.json"]
    seen: set[Path] = set()
    while True:
        require(
            cursor.is_dir()
            and not cursor.is_symlink()
            and cursor not in seen
            and cursor.is_relative_to(WORK / "outcomes"),
            "prior bundle chain is missing, cyclic, or unsafe",
        )
        seen.add(cursor)
        receipt_path = cursor / "promotion-receipt.json"
        receipt = object_from(
            sealed(receipt_path, expected_receipt_sha, "prior receipt"),
            "prior receipt",
        )
        require(receipt.get("recordsPerLane") == COUNT, "prior chain row count differs")
        for lane in LANES:
            entry = receipt["lanes"][lane]
            ledger_path = cursor / f"{lane}.jsonl"
            summary_path = cursor / f"{lane}.summary.json"
            ledger = sealed(ledger_path, entry["ledgerSha256"], f"prior {lane} ledger")
            summary = object_from(
                sealed(summary_path, entry["summarySha256"], f"prior {lane} summary"),
                f"prior {lane} summary",
            )
            require(
                entry["ledger"] == str(ledger_path)
                and summary.get("ledgerSha256") == digest(ledger)
                and summary.get("records") == COUNT
                and sum(summary["outcomes"].values()) == COUNT,
                f"prior {lane} lane chain differs",
            )
        prior = receipt.get("priorBundle")
        if prior is None:
            break
        predecessor = Path(prior)
        predecessor_receipt = predecessor / "promotion-receipt.json"
        expected_receipt_sha = receipt["priorBundleReceiptSha256"]
        previous = object_from(
            sealed(predecessor_receipt, expected_receipt_sha, "predecessor receipt"),
            "predecessor receipt",
        )
        for lane in LANES:
            current_entry = receipt["lanes"][lane]
            old_entry = previous["lanes"][lane]
            if "base" in current_entry:
                require(
                    current_entry["base"] == str(predecessor / f"{lane}.jsonl"),
                    "prior base path differs",
                )
            if "baseSha256" in current_entry:
                require(
                    current_entry["baseSha256"] == old_entry["ledgerSha256"],
                    "prior base SHA-256 differs",
                )
            if "baseSummarySha256" in current_entry:
                require(
                    current_entry["baseSummarySha256"] == old_entry["summarySha256"],
                    "prior base summary SHA-256 differs",
                )
        cursor = predecessor
    require(len(seen) == 22, "prior bundle chain length differs")


def verify_inputs() -> dict[str, Any]:
    inventory_lines = sealed(INVENTORY, INVENTORY_SHA256, "Core inventory").splitlines(
        keepends=True
    )
    require(len(inventory_lines) == COUNT, "Core inventory row count changed")
    manifest = object_from(
        sealed(MANIFEST, MANIFEST_SHA256, "Page.fill manifest"), "Page.fill manifest"
    )
    require(
        manifest.get("commit") == COMMIT
        and manifest.get("repository") == "https://github.com/microsoft/playwright.git"
        and manifest.get("tag") == "v1.58.0"
        and len(manifest.get("suites", [])) == 1
        and manifest["suites"][0].get("id") == CAPTURE_SUITE
        and manifest["suites"][0].get("expectedTestsPerProject") == TARGET_COUNT
        and manifest["suites"][0].get("expectedSpecIdsSha256") == SPEC_IDS_SHA256
        and manifest["suites"][0].get("upstreamTestSourceSha256") == SOURCE_SHA256,
        "Page.fill manifest selection differs",
    )
    sealed(SOURCE, SOURCE_SHA256, "upstream Page.fill source")
    old_files = {
        name: sealed(V12 / name, sha, f"v12 {name}") for name, sha in V12_SHA256.items()
    }
    old_receipt = object_from(old_files["promotion-receipt.json"], "v12 receipt")
    require(
        old_receipt.get("scope") == SCOPE_V12
        and old_receipt.get("recordsPerLane") == COUNT
        and old_receipt.get("candidateExpectedRows") == 32
        and old_receipt.get("candidateUnexpectedRows") == 0
        and old_receipt.get("oracleExpectedRows") == 32
        and old_receipt.get("fixtureAdapted") is True
        and old_receipt.get("browserExecutionEvidence") is True
        and old_receipt.get("publicApiParityEvidence") is False,
        "v12 historical receipt differs",
    )
    verify_prior_chain()
    targets: dict[int, dict[str, Any]] = {}
    for index, raw in enumerate(inventory_lines):
        row = json.loads(raw)
        if (
            row.get("suite") == "library"
            and row.get("projectId") == PROJECT
            and row.get("file") == FILE
        ):
            targets[index] = row
    require(len(targets) == TARGET_COUNT, "inventory Page.fill target count differs")
    target_ids = {row["specId"] for row in targets.values()}
    require(
        len(target_ids) == TARGET_COUNT
        and RED_SPEC_ID in target_ids
        and digest("".join(f"{spec_id}\n" for spec_id in sorted(target_ids)).encode())
        == SPEC_IDS_SHA256,
        "inventory Page.fill IDs changed",
    )
    red_index = next(
        index for index, row in targets.items() if row["specId"] == RED_SPEC_ID
    )
    require(
        red_index == RED_INDEX and targets[red_index]["line"] == RED_SOURCE_LINE,
        "historical RED index or source line differs",
    )
    old_lines: dict[str, list[bytes]] = {}
    old_summaries: dict[str, dict[str, Any]] = {}
    for lane in LANES:
        lines = old_files[f"{lane}.jsonl"].splitlines(keepends=True)
        require(len(lines) == COUNT, f"v12 {lane} row count differs")
        old_lines[lane] = lines
        summary = object_from(old_files[f"{lane}.summary.json"], f"v12 {lane} summary")
        require(
            summary.get("records") == COUNT
            and summary.get("ledgerSha256") == V12_SHA256[f"{lane}.jsonl"]
            and old_receipt["lanes"][lane]["ledgerSha256"]
            == V12_SHA256[f"{lane}.jsonl"]
            and old_receipt["lanes"][lane]["summarySha256"]
            == V12_SHA256[f"{lane}.summary.json"],
            f"v12 {lane} summary chain differs",
        )
        old_summaries[lane] = summary
        for index, source_row in targets.items():
            row = json.loads(lines[index])
            expected = (
                "unexpected"
                if lane == "candidate" and index == red_index
                else "expected"
            )
            require(
                row.get("specId") == source_row["specId"]
                and row.get("line") == source_row["line"]
                and row.get("attempts") == 1
                and row.get("outcome") == expected
                and (
                    row.get("error") == RED_ERROR
                    if expected == "unexpected"
                    else "error" not in row
                ),
                f"v12 {lane} Page.fill row {index} differs",
            )
    require(
        old_summaries["candidate"]["outcomes"]
        == {
            "expected": 4613,
            "not-run": 43788,
            "skipped": 21,
            "unexpected": 55,
        }
        and old_summaries["oracle"]["outcomes"]
        == {
            "expected": 4362,
            "not-run": 44087,
            "skipped": 20,
            "unexpected": 8,
        },
        "v12 lane totals differ",
    )
    verify_package()
    for lane in LANES:
        verify_provenance(
            lane,
            sealed(
                provenance_path(lane), PROVENANCE_SHA256[lane], f"{lane} provenance"
            ),
        )
        report_rows(
            lane,
            sealed(report_path(lane), REPORT_SHA256[lane], f"{lane} official report"),
            target_ids,
        )
    return {
        "oldLines": old_lines,
        "oldSummaries": old_summaries,
        "oldReceipt": old_receipt,
        "targets": targets,
        "redIndex": red_index,
    }


def expected_lane_receipt(
    lane: str, files: dict[str, bytes], inputs: dict[str, Any]
) -> dict[str, Any]:
    changed: list[dict[str, Any]] = []
    if lane == "candidate":
        index = inputs["redIndex"]
        old_raw = inputs["oldLines"][lane][index]
        new_raw = files[f"{lane}.jsonl"].splitlines(keepends=True)[index]
        changed = [
            {
                "index": index,
                "specId": RED_SPEC_ID,
                "oldRowSha256": digest(old_raw),
                "newRowSha256": digest(new_raw),
                "oldOutcome": "unexpected",
                "newOutcome": "expected",
                "oldAttempts": 1,
                "newAttempts": 2,
            }
        ]
    summary = object_from(files[f"{lane}.summary.json"], f"v13 {lane} summary")
    return {
        "base": str(V12 / f"{lane}.jsonl"),
        "baseSha256": V12_SHA256[f"{lane}.jsonl"],
        "baseSummarySha256": V12_SHA256[f"{lane}.summary.json"],
        "ledger": str(V13 / f"{lane}.jsonl"),
        "ledgerSha256": digest(files[f"{lane}.jsonl"]),
        "summarySha256": digest(files[f"{lane}.summary.json"]),
        "officialResult": str(report_path(lane)),
        "officialResultSha256": REPORT_SHA256[lane],
        "provenance": str(provenance_path(lane)),
        "provenanceSha256": PROVENANCE_SHA256[lane],
        "changedRows": changed,
        "promotedRows": len(changed),
        "unchangedRawRows": COUNT - len(changed),
        "outcomes": summary["outcomes"],
    }


def expected_receipt(files: dict[str, bytes], inputs: dict[str, Any]) -> dict[str, Any]:
    return {
        "schema": 1,
        "scope": SCOPE_V13,
        "promotionKind": "browser-backed-public-api-full-file-red-to-green-outcome",
        "priorBundle": str(V12),
        "priorBundleReceiptSha256": V12_SHA256["promotion-receipt.json"],
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "manifest": str(MANIFEST),
        "manifestSha256": MANIFEST_SHA256,
        "upstreamSourceSha256": SOURCE_SHA256,
        "recordsPerLane": COUNT,
        "targetZeroBasedIndexes": list(inputs["targets"]),
        "changedRowsPerLane": {"candidate": 1, "oracle": 0},
        "unchangedRawRowsPerLane": {"candidate": COUNT - 1, "oracle": COUNT},
        "fixtureAdapted": True,
        "browserExecutionEvidence": True,
        "completeFileExecution": True,
        "fullCorpusExecution": False,
        "chromeVersionObserved": CHROME_VERSION_OBSERVED,
        "launchMode": "cdp-adapter",
        "publicApiParityEvidence": False,
        "upstreamAssertionsModified": False,
        "candidateExpectedRows": TARGET_COUNT,
        "candidateUnexpectedRows": 0,
        "oracleExpectedRows": TARGET_COUNT,
        "historicalRedSpecId": RED_SPEC_ID,
        "historicalRedError": RED_ERROR,
        "historicalRedSourceLine": RED_SOURCE_LINE,
        "candidateProfile": CANDIDATE_PROFILE,
        "candidateArtifactSha256": CANDIDATE_ARTIFACT_SHA256,
        "candidateSourceSha256": CANDIDATE_SOURCE_SHA256,
        "candidatePackage": str(PACKAGE),
        "candidatePackageManifest": str(PACKAGE_MANIFEST),
        "candidatePackageManifestSha256": PACKAGE_MANIFEST_SHA256,
        "promoterSourceSha256": digest(PROMOTER.read_bytes()),
        "verifierSourceSha256": digest(Path(__file__).read_bytes()),
        "lanes": {lane: expected_lane_receipt(lane, files, inputs) for lane in LANES},
    }


def verify_payloads(
    files: dict[str, bytes], inputs: dict[str, Any] | None = None
) -> dict[str, Any]:
    require(set(files) == BUNDLE_NAMES, "v13 bundle files differ")
    checked = inputs or verify_inputs()
    for lane in LANES:
        old_lines = checked["oldLines"][lane]
        new_lines = files[f"{lane}.jsonl"].splitlines(keepends=True)
        require(len(new_lines) == COUNT, f"v13 {lane} row count differs")
        for index, (old_raw, new_raw) in enumerate(
            zip(old_lines, new_lines, strict=True)
        ):
            if lane == "candidate" and index == checked["redIndex"]:
                require(
                    new_raw == canonical_line(expected_red_row(json.loads(old_raw))),
                    "v13 candidate historical Page.fill RED row differs",
                )
            else:
                require(
                    new_raw == old_raw, f"v13 {lane} non-target raw row {index} changed"
                )
        summary = object_from(files[f"{lane}.summary.json"], f"v13 {lane} summary")
        require(
            summary
            == expected_summary(
                lane, checked["oldSummaries"][lane], files[f"{lane}.jsonl"]
            ),
            f"v13 {lane} summary differs",
        )
    receipt = object_from(files["promotion-receipt.json"], "v13 receipt")
    require(receipt == expected_receipt(files, checked), "v13 receipt differs")
    return receipt


def verify_existing() -> dict[str, Any]:
    require(V13.is_dir() and not V13.is_symlink(), "v13 bundle is absent or a symlink")
    require(
        {path.name for path in V13.iterdir()} == BUNDLE_NAMES,
        "v13 published file set differs",
    )
    require(
        all(
            (V13 / name).is_file() and not (V13 / name).is_symlink()
            for name in BUNDLE_NAMES
        ),
        "v13 published file is absent, not regular, or a symlink",
    )
    return verify_payloads({name: (V13 / name).read_bytes() for name in BUNDLE_NAMES})


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
    ) as error:
        parser.exit(
            2, f"strict paired Core Page.fill v13 verification rejected: {error}\n"
        )
    print(
        json.dumps(
            {"verified": True, "scope": receipt["scope"], "destination": str(V13)},
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
