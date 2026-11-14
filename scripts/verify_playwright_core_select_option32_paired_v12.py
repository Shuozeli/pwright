"""Independently replay the immutable Core select-option RED-to-GREEN pair.

The v11 bundle, frozen v3 candidate package, and official reports are sealed
inputs. This verifier does not import the v12 publisher or reinterpret v11 RED.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / ".work/official-conformance"
V11 = WORK / "outcomes/core-page-select-option32-strict-paired-v11"
V12 = WORK / "outcomes/core-page-select-option32-strict-paired-v12"
INVENTORY = WORK / "playwright-core-v1.58.0.jsonl"
MANIFEST = (
    ROOT
    / "tests/official/playwright-core-page-select-option-public-chromium-v1-manifest.json"
)
SOURCE = Path(
    "/Users/cyuan/projects/thirdparty/playwright-v1.58.0/tests/page/page-select-option.spec.ts"
)
PACKAGE = WORK / "builds/core-select-option32-post-binding-20261020-v3/candidate"
PACKAGE_MANIFEST = PACKAGE.parent / "candidate-manifest.json"
PROMOTER = ROOT / "scripts/promote_playwright_core_select_option32_paired_v12.py"
COUNT = 48_477
TARGET_COUNT = 32
FILE = "page/page-select-option.spec.ts"
PROJECT = "chromium-page"
CAPTURE_SUITE = "library-page-select-option-public-chromium"
SCOPE_V11 = "official-browser-backed-page-select-option-full-file-paired-v11"
SCOPE_V12 = "official-browser-backed-page-select-option-full-file-paired-v12"
RED_SPEC_ID = "8349af1b8919c79147f0-6ce19407ff351e5f91fa"
RED_ERROR = "Error: locator.selectOption: Backend: CDP error -32000: Cannot find context with specified id"
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
MANIFEST_SHA256 = "acc5e1b0ce2d6b3eff4c14a8b7ad650c76091ae457e293039311394231aead99"
SOURCE_SHA256 = "0dc2dd41232ee2ccf7a05e3ce302a50319a736efdc08fb8ec9dd971e2788dbbb"
SPEC_IDS_SHA256 = "ef9f3c7706dc6ef7ce0f07890227de4374a8a5231f8455dc4e61302d8ea41c0a"
V11_SHA256 = {
    "candidate.jsonl": "5aeb97be0654e3391470f61576d4e915f9f52d53faf4120f8bb2506c7df102ba",
    "candidate.summary.json": "397dbe3bda35977237380ac87008e45840dda5cbca33a58c7c67bd102f059730",
    "oracle.jsonl": "1baef074b29f90a49caa8ce4ae8620499359f03a0017fc0a98a4d1106c271303",
    "oracle.summary.json": "217faff735cd9e7c77ffa534b2395cb37cd62198528c384ffc25b056d3d8e9f1",
    "promotion-receipt.json": "990021bd0736ec4bcc4c94337c1ac158482cd1b6ef514e14dde98093655041d0",
}
REPORT_SHA256 = {
    "candidate": "829485d5e7267456f2dc4642325e37de13ab34fdc634cbad07a1069c9e1a57ae",
    "oracle": "6ef7eb977b6058cee2792a8d3d5c2b9f21f9c243c01de1c8dfe84758f2851ef8",
}
PROVENANCE_SHA256 = {
    "candidate": "35f98e3c011b43cc3e4f1912a75994b7e1b9b9424e5ce2b1e01cd63e95d81702",
    "oracle": "8e66c5e87ae9bdf704b5cc3ef83ae7aaec0d4e5634d1b6e5b99b9694995451c0",
}
PACKAGE_MANIFEST_SHA256 = (
    "6a27255dab8f2c54adcdd3c7a74cf42561297d443dffd17ce7e5e49178eedb82"
)
CANDIDATE_PROFILE = "official-conformance-page-select-option-navigation-commit"
CANDIDATE_ARTIFACT_SHA256 = (
    "ee5ae9896176dfec7fb5c653e2497a5a519e36ae9cf20cdde9fde1356d3cc8bb"
)
CANDIDATE_SOURCE_SHA256 = (
    "8f7871adeb009eed6bdd097f6317e8f16c8df84116b04e23ac652faa3ac0367f"
)
ARTIFACT = PACKAGE / f"native/{CANDIDATE_PROFILE}/pwright.node"
LANES = ("candidate", "oracle")
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
    version = "v3" if lane == "candidate" else "v1"
    return WORK / f"results/core-select-option32-{lane}-20261020-{version}.json"


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
        "frozen candidate package is absent or a symlink",
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
            path.is_file() and not path.is_symlink(),
            f"candidate package file is absent or a symlink: {relative}",
        )
        require(
            path.stat().st_size == item["size"],
            f"candidate package file size changed: {relative}",
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
) -> dict[str, dict[str, Any]]:
    report = object_from(data, f"{lane} official report")
    stats = report.get("stats")
    require(
        isinstance(stats, dict)
        and all(
            stats.get(key) == value
            for key, value in {
                "expected": 32,
                "unexpected": 0,
                "skipped": 0,
                "flaky": 0,
            }.items()
        ),
        f"{lane} report is not 32/32 GREEN",
    )
    specs: list[dict[str, Any]] = []

    def visit(suite: dict[str, Any]) -> None:
        require(isinstance(suite, dict), "malformed official report suite")
        for spec in suite.get("specs", []):
            require(isinstance(spec, dict), "malformed official report spec")
            specs.append(spec)
        for child in suite.get("suites", []):
            visit(child)

    suites = report.get("suites")
    require(
        isinstance(suites, list) and len(suites) == 1,
        f"{lane} report suite count differs",
    )
    for suite in suites:
        visit(suite)
    require(len(specs) == TARGET_COUNT, f"{lane} report is not the full 32-case file")
    rows: dict[str, dict[str, Any]] = {}
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
            and not attempts[0].get("error"),
            f"{lane} report has an unexecuted or failing invocation for {spec_id}",
        )
        rows[spec_id] = {"outcome": "expected", "attempts": 1}
    require(set(rows) == target_ids, f"{lane} report does not cover exact target IDs")
    return rows


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
        and value.get("manifest") == str(MANIFEST)
        and value.get("manifestSha256") == MANIFEST_SHA256
        and value.get("result") == str(report_path(lane))
        and value.get("resultSha256") == REPORT_SHA256[lane]
        and value.get("endpoint") == "http://127.0.0.1:9222"
        and value.get("filters") == ["tests/page/page-select-option.spec.ts"]
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
            "/Users/cyuan/projects/thirdparty/playwright-v1.58.0/packages/playwright/cli.js",
            "test",
            "--config=tests/library/playwright.config.ts",
            "--reporter=line,json",
            "--workers=1",
            "--project=chromium-page",
            "tests/page/page-select-option.spec.ts",
        ],
        f"{lane} provenance identity or source pin differs",
    )
    if lane == "candidate":
        require(
            value.get("candidate") == str(PACKAGE)
            and value.get("candidateArtifact") == str(ARTIFACT)
            and value.get("candidateProfile") == CANDIDATE_PROFILE
            and value.get("candidateArtifactSha256") == CANDIDATE_ARTIFACT_SHA256
            and value.get("candidateSourceSha256") == CANDIDATE_SOURCE_SHA256
            and value.get("fixtureHost") == "macbookpro-yuacx.tail8f3b66.ts.net"
            and value.get("fixtureBind") == "100.78.97.63",
            "candidate provenance package or fixture pin differs",
        )
    else:
        require(
            value.get("candidateProfile") is None
            and value.get("candidateArtifactSha256") is None,
            "oracle provenance contains a candidate runtime",
        )


def expected_red_row(old: dict[str, Any]) -> dict[str, Any]:
    require(old.get("specId") == RED_SPEC_ID, "wrong historical RED row")
    new = {**old}
    new.pop("error", None)
    new.update(
        {
            "attempts": 2,
            "outcome": "expected",
            "resultFile": str(report_path("candidate")),
            "evidenceScope": SCOPE_V12,
            "fixtureAdapted": True,
            "browserExecutionEvidence": True,
            "publicApiParityEvidence": False,
        }
    )
    return new


def expected_summary(lane: str, old: dict[str, Any], ledger: bytes) -> dict[str, Any]:
    outcomes = dict(old["outcomes"])
    scopes = dict(old["explicitEvidenceScopes"])
    if lane == "candidate":
        outcomes["expected"] += 1
        outcomes["unexpected"] -= 1
        scopes[SCOPE_V11] -= 1
        scopes[SCOPE_V12] = 1
    return {
        "schema": 1,
        "inventory": str(INVENTORY),
        "records": COUNT,
        "outcomes": dict(sorted(outcomes.items())),
        "explicitEvidenceScopes": dict(sorted(scopes.items())),
        "ledgerSha256": digest(ledger),
        "ledger": str(V12 / f"{lane}.jsonl"),
    }


def verify_inputs() -> dict[str, Any]:
    inventory_lines = sealed(INVENTORY, INVENTORY_SHA256, "Core inventory").splitlines(
        keepends=True
    )
    require(len(inventory_lines) == COUNT, "Core inventory row count changed")
    manifest = object_from(
        sealed(MANIFEST, MANIFEST_SHA256, "full-file manifest"), "full-file manifest"
    )
    require(
        manifest.get("commit") == COMMIT
        and len(manifest.get("suites", [])) == 1
        and manifest["suites"][0].get("id") == CAPTURE_SUITE
        and manifest["suites"][0].get("expectedTestsPerProject") == TARGET_COUNT
        and manifest["suites"][0].get("expectedSpecIdsSha256") == SPEC_IDS_SHA256
        and manifest["suites"][0].get("upstreamTestSourceSha256") == SOURCE_SHA256,
        "full-file manifest selection differs",
    )
    sealed(SOURCE, SOURCE_SHA256, "upstream select-option source")
    old_files = {
        name: sealed(V11 / name, sha, f"v11 {name}") for name, sha in V11_SHA256.items()
    }
    old_receipt = object_from(old_files["promotion-receipt.json"], "v11 receipt")
    require(
        old_receipt.get("scope") == SCOPE_V11
        and old_receipt.get("recordsPerLane") == COUNT
        and old_receipt.get("candidateExpectedRows") == 31
        and old_receipt.get("candidateUnexpectedRows") == 1
        and old_receipt.get("oracleExpectedRows") == 32
        and old_receipt.get("fixtureAdapted") is True
        and old_receipt.get("browserExecutionEvidence") is True
        and old_receipt.get("publicApiParityEvidence") is False,
        "v11 historical receipt differs",
    )
    targets: dict[int, dict[str, Any]] = {}
    for index, raw in enumerate(inventory_lines):
        row = json.loads(raw)
        if (
            row.get("suite") == "library"
            and row.get("projectId") == PROJECT
            and row.get("file") == FILE
        ):
            targets[index] = row
    require(len(targets) == TARGET_COUNT, "inventory target count differs")
    target_ids = {row["specId"] for row in targets.values()}
    require(
        len(target_ids) == TARGET_COUNT and RED_SPEC_ID in target_ids,
        "inventory target IDs differ",
    )
    require(
        digest("".join(f"{spec_id}\n" for spec_id in sorted(target_ids)).encode())
        == SPEC_IDS_SHA256,
        "inventory target IDs changed",
    )
    require(
        old_receipt.get("targetZeroBasedIndexes") == list(targets),
        "v11 target indexes differ",
    )
    old_lines: dict[str, list[bytes]] = {}
    old_summaries: dict[str, dict[str, Any]] = {}
    for lane in LANES:
        lines = old_files[f"{lane}.jsonl"].splitlines(keepends=True)
        require(len(lines) == COUNT, f"v11 {lane} row count differs")
        old_lines[lane] = lines
        summary = object_from(old_files[f"{lane}.summary.json"], f"v11 {lane} summary")
        require(
            summary.get("records") == COUNT
            and summary.get("ledgerSha256") == V11_SHA256[f"{lane}.jsonl"]
            and old_receipt["lanes"][lane]["ledgerSha256"]
            == V11_SHA256[f"{lane}.jsonl"]
            and old_receipt["lanes"][lane]["summarySha256"]
            == V11_SHA256[f"{lane}.summary.json"],
            f"v11 {lane} summary chain differs",
        )
        old_summaries[lane] = summary
        for index, source_row in targets.items():
            row = json.loads(lines[index])
            expected = (
                "unexpected"
                if lane == "candidate" and row["specId"] == RED_SPEC_ID
                else "expected"
            )
            require(
                row.get("specId") == source_row["specId"]
                and row.get("attempts") == 1
                and row.get("outcome") == expected
                and row.get("evidenceScope") == SCOPE_V11
                and row.get("browserExecutionEvidence") is True
                and row.get("publicApiParityEvidence") is False
                and (
                    row.get("error") == RED_ERROR
                    if expected == "unexpected"
                    else "error" not in row
                ),
                f"v11 {lane} target row {index} differs",
            )
    red_index = next(
        index for index, row in targets.items() if row["specId"] == RED_SPEC_ID
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
    summary = object_from(files[f"{lane}.summary.json"], f"v12 {lane} summary")
    return {
        "base": str(V11 / f"{lane}.jsonl"),
        "baseSha256": V11_SHA256[f"{lane}.jsonl"],
        "baseSummarySha256": V11_SHA256[f"{lane}.summary.json"],
        "ledger": str(V12 / f"{lane}.jsonl"),
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
        "scope": SCOPE_V12,
        "promotionKind": "browser-backed-public-api-full-file-red-to-green-outcome",
        "priorBundle": str(V11),
        "priorBundleReceiptSha256": V11_SHA256["promotion-receipt.json"],
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
        "publicApiParityEvidence": False,
        "upstreamAssertionsModified": False,
        "candidateExpectedRows": 32,
        "candidateUnexpectedRows": 0,
        "oracleExpectedRows": 32,
        "historicalRedSpecId": RED_SPEC_ID,
        "historicalRedError": RED_ERROR,
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
    require(set(files) == BUNDLE_NAMES, "v12 bundle files differ")
    checked = inputs or verify_inputs()
    for lane in LANES:
        old_lines = checked["oldLines"][lane]
        new_lines = files[f"{lane}.jsonl"].splitlines(keepends=True)
        require(len(new_lines) == COUNT, f"v12 {lane} row count differs")
        for index, (old_raw, new_raw) in enumerate(
            zip(old_lines, new_lines, strict=True)
        ):
            if lane == "candidate" and index == checked["redIndex"]:
                require(
                    new_raw == canonical_line(expected_red_row(json.loads(old_raw))),
                    "v12 candidate historical RED target row differs",
                )
            else:
                require(
                    new_raw == old_raw, f"v12 {lane} non-target raw row {index} changed"
                )
        summary = object_from(files[f"{lane}.summary.json"], f"v12 {lane} summary")
        require(
            summary
            == expected_summary(
                lane, checked["oldSummaries"][lane], files[f"{lane}.jsonl"]
            ),
            f"v12 {lane} summary differs",
        )
    receipt = object_from(files["promotion-receipt.json"], "v12 receipt")
    require(receipt == expected_receipt(files, checked), "v12 receipt differs")
    return receipt


def verify_existing() -> dict[str, Any]:
    require(V12.is_dir() and not V12.is_symlink(), "v12 bundle is absent or a symlink")
    require(
        {path.name for path in V12.iterdir()} == BUNDLE_NAMES,
        "v12 published file set differs",
    )
    require(
        all(
            (V12 / name).is_file() and not (V12 / name).is_symlink()
            for name in BUNDLE_NAMES
        ),
        "v12 published file is absent, not regular, or a symlink",
    )
    return verify_payloads({name: (V12 / name).read_bytes() for name in BUNDLE_NAMES})


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
        parser.exit(2, f"strict paired Core v12 verification rejected: {error}\n")
    print(
        json.dumps(
            {"verified": True, "scope": receipt["scope"], "destination": str(V12)},
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
