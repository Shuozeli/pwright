"""Independently replay the immutable 32-case Core select-option ledger pair.

The historical v10 pair and the two source-bound official reports are sealed
inputs. This verifier deliberately does not import the v11 publisher.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / ".work/official-conformance"
V10 = WORK / "outcomes/core-dummy-node-bidi-strict-paired-v10"
V11 = WORK / "outcomes/core-page-select-option32-strict-paired-v11"
INVENTORY = WORK / "playwright-core-v1.58.0.jsonl"
MANIFEST = (
    ROOT
    / "tests/official/playwright-core-page-select-option-public-chromium-v1-manifest.json"
)
SOURCE = Path(
    "/Users/cyuan/projects/thirdparty/playwright-v1.58.0/tests/page/page-select-option.spec.ts"
)
PROMOTER = ROOT / "scripts/promote_playwright_core_select_option32_paired_v11.py"
COUNT = 48_477
TARGET_COUNT = 32
SUITE = "library"
PROJECT = "chromium-page"
FILE = "page/page-select-option.spec.ts"
CAPTURE_SUITE = "library-page-select-option-public-chromium"
SCOPE = "official-browser-backed-page-select-option-full-file-paired-v11"
RED_SPEC_ID = "8349af1b8919c79147f0-6ce19407ff351e5f91fa"
RED_ERROR = "Error: locator.selectOption: Backend: CDP error -32000: Cannot find context with specified id"
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
MANIFEST_SHA256 = "acc5e1b0ce2d6b3eff4c14a8b7ad650c76091ae457e293039311394231aead99"
SOURCE_SHA256 = "0dc2dd41232ee2ccf7a05e3ce302a50319a736efdc08fb8ec9dd971e2788dbbb"
SPEC_IDS_SHA256 = "ef9f3c7706dc6ef7ce0f07890227de4374a8a5231f8455dc4e61302d8ea41c0a"
V10_SHA256 = {
    "promotion-receipt.json": "2ae0478daf066539db4c9904e344dc7c8b102ff65eda8ab9587f2b591f8903e2",
    "candidate.jsonl": "136192ef2e648d9abe0a023f952321923f5dc159c9f6a3240ba13118b5aa8f20",
    "candidate.summary.json": "efb8f70b96374c6fb2d954387b42b58e55b8ac20ace727adfacb86eb3667286d",
    "oracle.jsonl": "23899b55d159bfca184dca7812122c0ac979083dbd1526a9c55d70d54ad182fb",
    "oracle.summary.json": "2a7aa5391561b8fba7d48946a83882a2be41ad4da81e0bf4f57787df88cf327b",
}
REPORT_SHA256 = {
    "candidate": "78eda032d312bb7baa5e81953b823f1b64c50c4572eadd6610ffea6b85dc341d",
    "oracle": "6ef7eb977b6058cee2792a8d3d5c2b9f21f9c243c01de1c8dfe84758f2851ef8",
}
PROVENANCE_SHA256 = {
    "candidate": "f850fc26d937e5ffd369ec552b931713429410dce7896adb33864127eb0665ee",
    "oracle": "8e66c5e87ae9bdf704b5cc3ef83ae7aaec0d4e5634d1b6e5b99b9694995451c0",
}
CANDIDATE_PROFILE = "official-conformance-page-select-option-timeout-progress"
CANDIDATE_ARTIFACT_SHA256 = (
    "ca8ba88e684409d190c184e84079edc2f111ed97a0f992c347d3c3cdee543e74"
)
CANDIDATE_SOURCE_SHA256 = (
    "fefa89f15b0dd2c75b8485d20dcbb181866330a5099595c3405537f16cc3674e"
)
LANES = ("candidate", "oracle")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical_line(record: dict[str, Any]) -> bytes:
    return (json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n").encode()


def sealed(path: Path, expected: str, label: str) -> bytes:
    require(path.is_file() and not path.is_symlink(), f"{label} is absent or a symlink")
    data = path.read_bytes()
    require(digest(data) == expected, f"{label} SHA-256 changed")
    return data


def report_path(lane: str) -> Path:
    require(lane in LANES, "unknown lane")
    return WORK / f"results/core-select-option32-{lane}-20261020-v1.json"


def provenance_path(lane: str) -> Path:
    return report_path(lane).with_suffix(".provenance.json")


def _object(data: bytes, label: str) -> dict[str, Any]:
    value = json.loads(data)
    require(isinstance(value, dict), f"{label} is not an object")
    return value


def _collect_specs(report: dict[str, Any]) -> list[dict[str, Any]]:
    suites = report.get("suites")
    require(isinstance(suites, list), "report has no suites")
    specs: list[dict[str, Any]] = []

    def visit(suite: dict[str, Any]) -> None:
        for spec in suite.get("specs", []):
            require(isinstance(spec, dict), "malformed report spec")
            specs.append(spec)
        for child in suite.get("suites", []):
            require(isinstance(child, dict), "malformed report child suite")
            visit(child)

    for suite in suites:
        require(isinstance(suite, dict), "malformed report suite")
        visit(suite)
    return specs


def _report_rows(
    lane: str, data: bytes, target_ids: set[str]
) -> dict[str, dict[str, Any]]:
    report = _object(data, f"{lane} report")
    specs = _collect_specs(report)
    require(len(specs) == TARGET_COUNT, f"{lane} report is not the full 32-case file")
    rows: dict[str, dict[str, Any]] = {}
    for spec in specs:
        tests = spec.get("tests")
        require(
            spec.get("file") == FILE
            and isinstance(tests, list)
            and len(tests) == 1
            and isinstance(tests[0], dict),
            f"{lane} report has a wrong file or test shape",
        )
        spec_id = spec.get("id")
        test = tests[0]
        attempts = test.get("results")
        require(
            isinstance(spec_id, str)
            and spec_id in target_ids
            and spec_id not in rows
            and test.get("projectId") == PROJECT
            and test.get("expectedStatus") == "passed"
            and isinstance(attempts, list)
            and len(attempts) == 1
            and isinstance(attempts[0], dict),
            f"{lane} report has a missing, duplicate, or unexecuted invocation",
        )
        status = test.get("status")
        attempt_status = attempts[0].get("status")
        require(
            (status, attempt_status)
            in {("expected", "passed"), ("unexpected", "failed")},
            f"{lane} report status and attempt disagree for {spec_id}",
        )
        error = attempts[0].get("error") or {}
        message = error.get("message", "") if isinstance(error, dict) else ""
        first_error = (
            message.splitlines()[0] if isinstance(message, str) and message else ""
        )
        require(
            (status == "expected" and not first_error)
            or (
                lane == "candidate"
                and spec_id == RED_SPEC_ID
                and first_error == RED_ERROR
            ),
            f"{lane} report has an unpinned error for {spec_id}",
        )
        rows[spec_id] = {"outcome": status, "attempts": 1, "error": first_error}
    require(set(rows) == target_ids, f"{lane} report does not cover exact target IDs")
    counts = Counter(row["outcome"] for row in rows.values())
    require(
        counts
        == (
            {"expected": 31, "unexpected": 1}
            if lane == "candidate"
            else {"expected": 32}
        ),
        f"{lane} report changed historical RED/GREEN counts",
    )
    return rows


def _provenance(lane: str, data: bytes) -> dict[str, Any]:
    value = _object(data, f"{lane} provenance")
    require(
        value.get("schema") == 2
        and value.get("mode") == lane
        and value.get("suite") == CAPTURE_SUITE
        and value.get("project") == PROJECT
        and value.get("commit") == "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
        and value.get("manifest") == str(MANIFEST)
        and value.get("manifestSha256") == MANIFEST_SHA256
        and value.get("result") == str(report_path(lane))
        and value.get("resultSha256") == REPORT_SHA256[lane]
        and value.get("endpoint") == "http://127.0.0.1:9222"
        and value.get("filters") == ["tests/page/page-select-option.spec.ts"]
        and value.get("workers") == 1
        and value.get("grep") is None
        and value.get("headed") is False
        and value.get("adapter")
        == (
            "test-mode-playwright-core"
            if lane == "candidate"
            else "test-mode-playwright-core-cdp"
        )
        and value.get("exitCode") == (1 if lane == "candidate" else 0),
        f"{lane} provenance identity or source pin differs",
    )
    if lane == "candidate":
        require(
            value.get("candidateProfile") == CANDIDATE_PROFILE
            and value.get("candidateArtifactSha256") == CANDIDATE_ARTIFACT_SHA256
            and value.get("candidateSourceSha256") == CANDIDATE_SOURCE_SHA256,
            "candidate runtime source or artifact differs",
        )
    else:
        require(
            value.get("candidateProfile") is None
            and value.get("candidateArtifactSha256") is None,
            "oracle provenance contains a candidate runtime",
        )
    return value


def verify_inputs() -> dict[str, Any]:
    inventory_data = sealed(INVENTORY, INVENTORY_SHA256, "Core inventory")
    inventory_lines = inventory_data.splitlines(keepends=True)
    require(len(inventory_lines) == COUNT, "Core inventory row count changed")
    manifest = _object(
        sealed(MANIFEST, MANIFEST_SHA256, "full-file manifest"), "manifest"
    )
    require(
        manifest.get("commit") == "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
        and len(manifest.get("suites", [])) == 1
        and manifest["suites"][0].get("id") == CAPTURE_SUITE
        and manifest["suites"][0].get("expectedTestsPerProject") == TARGET_COUNT
        and manifest["suites"][0].get("expectedSpecIdsSha256") == SPEC_IDS_SHA256
        and manifest["suites"][0].get("upstreamTestSourceSha256") == SOURCE_SHA256,
        "full-file manifest selection differs",
    )
    sealed(SOURCE, SOURCE_SHA256, "upstream select-option source")
    prior_receipt = _object(
        sealed(
            V10 / "promotion-receipt.json",
            V10_SHA256["promotion-receipt.json"],
            "v10 receipt",
        ),
        "v10 receipt",
    )
    require(
        prior_receipt.get("recordsPerLane") == COUNT, "v10 receipt row count differs"
    )
    targets: dict[int, dict[str, Any]] = {}
    for index, raw in enumerate(inventory_lines):
        row = json.loads(raw)
        if (
            row.get("suite") == SUITE
            and row.get("projectId") == PROJECT
            and row.get("file") == FILE
        ):
            require(index not in targets, "duplicate target index")
            targets[index] = row
    require(len(targets) == TARGET_COUNT, "inventory target count differs")
    ids = [row["specId"] for row in targets.values()]
    require(len(set(ids)) == TARGET_COUNT, "inventory target IDs duplicate")
    ids_digest = digest("".join(f"{spec_id}\n" for spec_id in sorted(ids)).encode())
    require(ids_digest == SPEC_IDS_SHA256, "inventory target IDs changed")
    old_lines: dict[str, list[bytes]] = {}
    summaries: dict[str, dict[str, Any]] = {}
    reports: dict[str, dict[str, dict[str, Any]]] = {}
    for lane in LANES:
        old = sealed(
            V10 / f"{lane}.jsonl", V10_SHA256[f"{lane}.jsonl"], f"v10 {lane} ledger"
        )
        old_lines[lane] = old.splitlines(keepends=True)
        require(len(old_lines[lane]) == COUNT, f"v10 {lane} count differs")
        summary = _object(
            sealed(
                V10 / f"{lane}.summary.json",
                V10_SHA256[f"{lane}.summary.json"],
                f"v10 {lane} summary",
            ),
            f"v10 {lane} summary",
        )
        require(
            summary.get("ledgerSha256") == V10_SHA256[f"{lane}.jsonl"]
            and summary.get("records") == COUNT
            and prior_receipt["lanes"][lane]["ledgerSha256"]
            == V10_SHA256[f"{lane}.jsonl"]
            and prior_receipt["lanes"][lane]["summarySha256"]
            == V10_SHA256[f"{lane}.summary.json"],
            f"v10 {lane} chain differs",
        )
        summaries[lane] = summary
        for index, source_row in targets.items():
            old_row = json.loads(old_lines[lane][index])
            require(
                old_row == {**source_row, "attempts": 0, "outcome": "not-run"},
                f"v10 {lane} target at {index} was previously run",
            )
        provenance = sealed(
            provenance_path(lane), PROVENANCE_SHA256[lane], f"{lane} provenance"
        )
        _provenance(lane, provenance)
        report = sealed(
            report_path(lane), REPORT_SHA256[lane], f"{lane} official report"
        )
        reports[lane] = _report_rows(lane, report, set(ids))
    return {
        "targets": targets,
        "oldLines": old_lines,
        "summaries": summaries,
        "reports": reports,
    }


def expected_row(
    old: dict[str, Any], report: dict[str, Any], lane: str
) -> dict[str, Any]:
    new = {
        **old,
        "attempts": 1,
        "outcome": report["outcome"],
        "resultFile": str(report_path(lane)),
        "evidenceScope": SCOPE,
        "fixtureAdapted": True,
        "browserExecutionEvidence": True,
        "publicApiParityEvidence": False,
    }
    if report["error"]:
        new["error"] = report["error"]
    return new


def verify_payloads(
    files: dict[str, bytes], inputs: dict[str, Any] | None = None
) -> dict[str, Any]:
    require(
        set(files)
        == {
            "candidate.jsonl",
            "candidate.summary.json",
            "oracle.jsonl",
            "oracle.summary.json",
            "promotion-receipt.json",
        },
        "v11 bundle files differ",
    )
    checked = inputs or verify_inputs()
    receipt = _object(files["promotion-receipt.json"], "v11 receipt")
    require(
        set(receipt)
        == {
            "schema",
            "scope",
            "promotionKind",
            "priorBundle",
            "priorBundleReceiptSha256",
            "inventory",
            "inventorySha256",
            "manifest",
            "manifestSha256",
            "upstreamSourceSha256",
            "recordsPerLane",
            "changedRowsPerLane",
            "unchangedRawRowsPerLane",
            "targetZeroBasedIndexes",
            "fixtureAdapted",
            "browserExecutionEvidence",
            "publicApiParityEvidence",
            "upstreamAssertionsModified",
            "candidateExpectedRows",
            "candidateUnexpectedRows",
            "oracleExpectedRows",
            "candidateRedSpecId",
            "candidateRedError",
            "candidateProfile",
            "candidateArtifactSha256",
            "candidateSourceSha256",
            "promoterSourceSha256",
            "verifierSourceSha256",
            "lanes",
        },
        "v11 receipt field set differs",
    )
    require(
        receipt.get("schema") == 1
        and receipt.get("scope") == SCOPE
        and receipt.get("promotionKind")
        == "browser-backed-public-api-full-file-outcomes"
        and receipt.get("priorBundle") == str(V10)
        and receipt.get("priorBundleReceiptSha256")
        == V10_SHA256["promotion-receipt.json"]
        and receipt.get("inventory") == str(INVENTORY)
        and receipt.get("inventorySha256") == INVENTORY_SHA256
        and receipt.get("manifest") == str(MANIFEST)
        and receipt.get("manifestSha256") == MANIFEST_SHA256
        and receipt.get("upstreamSourceSha256") == SOURCE_SHA256
        and receipt.get("recordsPerLane") == COUNT
        and receipt.get("changedRowsPerLane") == TARGET_COUNT
        and receipt.get("unchangedRawRowsPerLane") == COUNT - TARGET_COUNT
        and receipt.get("targetZeroBasedIndexes") == list(checked["targets"])
        and receipt.get("fixtureAdapted") is True
        and receipt.get("browserExecutionEvidence") is True
        and receipt.get("publicApiParityEvidence") is False
        and receipt.get("upstreamAssertionsModified") is False
        and receipt.get("candidateExpectedRows") == 31
        and receipt.get("candidateUnexpectedRows") == 1
        and receipt.get("oracleExpectedRows") == 32
        and receipt.get("candidateRedSpecId") == RED_SPEC_ID
        and receipt.get("candidateRedError") == RED_ERROR
        and receipt.get("candidateProfile") == CANDIDATE_PROFILE
        and receipt.get("candidateArtifactSha256") == CANDIDATE_ARTIFACT_SHA256
        and receipt.get("candidateSourceSha256") == CANDIDATE_SOURCE_SHA256
        and receipt.get("promoterSourceSha256") == digest(PROMOTER.read_bytes())
        and receipt.get("verifierSourceSha256") == digest(Path(__file__).read_bytes()),
        "v11 receipt header or source pin differs",
    )
    require(set(receipt.get("lanes", {})) == set(LANES), "v11 receipt lane set differs")
    for lane in LANES:
        old_lines: list[bytes] = checked["oldLines"][lane]
        new_lines = files[f"{lane}.jsonl"].splitlines(keepends=True)
        require(len(new_lines) == COUNT, f"v11 {lane} row count differs")
        changed: list[dict[str, Any]] = []
        outcomes: Counter[str] = Counter()
        scopes: Counter[str] = Counter()
        for index, (old_raw, new_raw) in enumerate(
            zip(old_lines, new_lines, strict=True)
        ):
            if index in checked["targets"]:
                old = json.loads(old_raw)
                spec_id = old["specId"]
                report = checked["reports"][lane][spec_id]
                require(
                    new_raw == canonical_line(expected_row(old, report, lane)),
                    f"v11 {lane} target row {index} differs",
                )
                changed.append(
                    {
                        "index": index,
                        "specId": spec_id,
                        "oldRowSha256": digest(old_raw),
                        "newRowSha256": digest(new_raw),
                        "oldOutcome": "not-run",
                        "newOutcome": report["outcome"],
                    }
                )
            else:
                require(
                    old_raw == new_raw, f"v11 {lane} non-target raw row {index} changed"
                )
            row = json.loads(new_raw)
            outcomes[row["outcome"]] += 1
            if "evidenceScope" in row:
                scopes[row["evidenceScope"]] += 1
        require(
            len(changed) == TARGET_COUNT and scopes[SCOPE] == TARGET_COUNT,
            f"v11 {lane} target or scope count differs",
        )
        ledger_sha = digest(files[f"{lane}.jsonl"])
        summary = _object(files[f"{lane}.summary.json"], f"v11 {lane} summary")
        expected_summary = {
            "schema": 1,
            "inventory": str(INVENTORY),
            "records": COUNT,
            "outcomes": dict(sorted(outcomes.items())),
            "explicitEvidenceScopes": dict(sorted(scopes.items())),
            "ledgerSha256": ledger_sha,
            "ledger": str(V11 / f"{lane}.jsonl"),
        }
        require(summary == expected_summary, f"v11 {lane} summary differs")
        lane_receipt = receipt["lanes"][lane]
        require(
            lane_receipt
            == {
                "base": str(V10 / f"{lane}.jsonl"),
                "baseSha256": V10_SHA256[f"{lane}.jsonl"],
                "baseSummarySha256": V10_SHA256[f"{lane}.summary.json"],
                "ledger": str(V11 / f"{lane}.jsonl"),
                "ledgerSha256": ledger_sha,
                "summarySha256": digest(files[f"{lane}.summary.json"]),
                "officialResult": str(report_path(lane)),
                "officialResultSha256": REPORT_SHA256[lane],
                "provenance": str(provenance_path(lane)),
                "provenanceSha256": PROVENANCE_SHA256[lane],
                "changedRows": changed,
                "promotedRows": TARGET_COUNT,
                "unchangedRawRows": COUNT - TARGET_COUNT,
                "outcomes": expected_summary["outcomes"],
            },
            f"v11 {lane} receipt differs",
        )
    return receipt


def verify_existing() -> dict[str, Any]:
    require(V11.is_dir() and not V11.is_symlink(), "v11 bundle is absent or a symlink")
    names = {path.name for path in V11.iterdir()}
    expected = {
        "candidate.jsonl",
        "candidate.summary.json",
        "oracle.jsonl",
        "oracle.summary.json",
        "promotion-receipt.json",
    }
    require(names == expected, "v11 published file set differs")
    require(
        all(
            (V11 / name).is_file() and not (V11 / name).is_symlink()
            for name in expected
        ),
        "v11 published file is absent, not regular, or a symlink",
    )
    files = {name: (V11 / name).read_bytes() for name in expected}
    return verify_payloads(files)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify-existing", action="store_true", required=True)
    parser.parse_args()
    try:
        receipt = verify_existing()
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
        parser.exit(2, f"strict paired Core v11 verification rejected: {error}\n")
    print(
        json.dumps(
            {"verified": True, "scope": receipt["scope"], "destination": str(V11)},
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
