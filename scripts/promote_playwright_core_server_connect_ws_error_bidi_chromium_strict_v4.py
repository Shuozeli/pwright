#!/usr/bin/env python3
"""Promote six passed Bidi WS-error diagnostics into an immutable Core pair."""

from __future__ import annotations

import argparse
import ctypes
import hashlib
import json
import os
import shutil
import sys
import tempfile
from collections import Counter
from itertools import zip_longest
from pathlib import Path
from typing import Any

import run_playwright_core_server_connect_ws_error_bidi_chromium_strict_v4 as capture
from verify_playwright_core_server_connect_ws_error_bidi_chromium_strict_v4 import (
    verify_pair,
)
from verify_playwright_core_server_connect_version_mismatch_paired_v3 import (
    verify_existing as verify_predecessor,
)

ROOT = capture.ROOT
OUTCOMES = ROOT / ".work/official-conformance/outcomes"
BASE = OUTCOMES / "core-server-connect-version-mismatch-url-owner-strict-paired-v3"
OUTPUT = OUTCOMES / "core-server-connect-ws-error-bidi-chromium-strict-paired-v4"
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
BASE_RECEIPT_SHA256 = "5e6180a6b2e0a90d764276af84b933b14b6117cc4dd799a0f925622ea347e1b8"
BASE_LEDGER_SHA256 = {
    "candidate": "e8485c418287054c78c2b1331d2cd258103d556fb0aca2bf988a5ac738f17e04",
    "oracle": "6994207de60e01ad2e36dc296fe20922e39ab2483d574d02b77d42e18bb2fa3c",
}
BASE_SUMMARY_SHA256 = {
    "candidate": "7302bec7437611f8d590443bb23fe3ebdb6dd87128d57f51d899548f67d6c55d",
    "oracle": "0a2e1eb879d1d78cd759fcbb7f19a024b5600489d428b45368a2a69c9e39fe87",
}
STRICT_SHA256 = {
    "candidate": "64c5d948cba49d1973ee353eef49328a28838861e86c4cee0c911e34a9a07f55",
    "oracle": "d896dd6a35598140a845e78096aa686f6b3c6922aee851074a9b9f8d3a485b20",
}
SNAPSHOT_MANIFEST_SHA256 = (
    "a6367b883ec96f48d247a6c685c22ce7725df5a2315a968b26e9c27d8a3fd755"
)
CANDIDATE_SOURCE_SHA256 = (
    "a2bb47a7c22f88b1c8cbaff72fb107de0a015eec85f6a4101d94e0cbddbfca5a"
)
COUNT = 48_477
TARGETS = {
    5_348: (
        "ee6df4b3a2217bd356a7-b8cf56582c005adbb27a",
        "bidi-chrome-canary-library",
        "run-server",
    ),
    5_349: (
        "ee6df4b3a2217bd356a7-f3aa7e41c539ce16e04a",
        "bidi-chrome-canary-library",
        "launchServer",
    ),
    5_350: (
        "ee6df4b3a2217bd356a7-3020514ab5e796a1d392",
        "bidi-chrome-stable-library",
        "launchServer",
    ),
    5_351: (
        "ee6df4b3a2217bd356a7-d09b02e8063437028bbd",
        "bidi-chrome-stable-library",
        "run-server",
    ),
    5_352: (
        "ee6df4b3a2217bd356a7-2fde122253a3920eead1",
        "bidi-chromium-library",
        "run-server",
    ),
    5_353: (
        "ee6df4b3a2217bd356a7-5be15441c4b3627bd25a",
        "bidi-chromium-library",
        "launchServer",
    ),
}
CLASSIFICATION_REASONS = [
    "private-client-access",
    "runtime-source-import:packages/playwright-core/lib/server/utils/network",
    "runtime-source-import:packages/playwright-core/lib/server/utils/userAgent",
]
SCOPE = "official-browser-free-server-connect-ws-error-bidi-chromium-internal-diagnostic-green-v4"
HARNESS_PINS = {
    "scripts/official_candidate_package_snapshot_v1.py": "20af90af081554d2102f34ca337e7ff1bff438488dfe50f3db0cf8e7d7363fbe",
    "scripts/official_result_destination.py": "61cd172cc58af3fecfe09b9d44da5ae67038ab660f7ed8a140b1ddf1784c04ab",
    "scripts/run_playwright_core_server_connect_ws_error_bidi_chromium_strict_v4.py": "0a0267ae46a8438d0843308bf99d000588121be30e300260a369449f3e481ec1",
    "scripts/run_playwright_core_server_connect_ws_error_strict_v2.py": "ec298baf952a80822d78694d15ddd4df657ec39c08bab467ff0924300985f822",
    "scripts/run_playwright_official_conformance.py": "21f1df10e77485bf29b67b31e548a1bc5126d50dc47cdeb115a9cf6798704a7b",
    "scripts/test_verify_playwright_core_server_connect_ws_error_bidi_chromium_strict_v4.py": "b46767cc8e1d485541e55c35f2b4aa09c4ae4c4bf43ca7efaea489ff22f200d6",
    "scripts/verify_playwright_core_server_connect_ws_error_bidi_chromium_strict_v4.py": "81ae905d120bdfa277cae229d5ee378ad4f67541fc3acd6df56bbc526a2692f9",
    "tests/official/playwright-core-browser-type-owner-v2-preload.cjs": "b7dc4e3be3aa4c07ac069bc07adfa5a2e65456efc6fc438e49a84a907ffc06ce",
    "tests/official/playwright-core-candidate-preload.cjs": "d17be590ff9334b0d99dd16d0078e4b434a39eae21568b799314ababc6bbf924",
    "tests/official/playwright-core-server-connect-ws-error-bidi-chromium-strict-v4-manifest.json": "e0d701c1d4842cf40f4a1a9f2f6676384fa08e5d5223c741c866d0aeca92fc62",
    "tests/official/playwright-core-server-connect-ws-error-strict-v2-preload.cjs": "8961683cd908e302ec6a2319c2562e61972930986c70071db8fec2059b02ae71",
    "tests/official/playwright-core-server-runtime.cjs": "58278e6c976da47074a1eb8d682916ebc3d0a41975fe89fe9270d1a664c396fd",
}
PAIRED_VERIFIER = (
    ROOT
    / "scripts/verify_playwright_core_server_connect_ws_error_bidi_chromium_paired_v4.py"
)
PROMOTION_TEST = (
    ROOT
    / "scripts/test_verify_playwright_core_server_connect_ws_error_bidi_chromium_paired_v4.py"
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sealed(path: Path, expected: str) -> None:
    require(
        path.is_file() and not path.is_symlink(), f"missing or linked evidence: {path}"
    )
    require(
        len(expected) == 64 and digest(path.read_bytes()) == expected,
        f"SHA-256 drift: {path}",
    )


def check_predecessor() -> dict[str, Any]:
    sealed(INVENTORY, INVENTORY_SHA256)
    sealed(BASE / "promotion-receipt.json", BASE_RECEIPT_SHA256)
    previous = verify_predecessor()
    require(
        previous["schema"] == 1
        and previous["scope"]
        == "official-browser-free-server-version-mismatch-internal-diagnostic-green-v3"
        and previous["promotionKind"] == "implementation-internal-diagnostic"
        and previous["fixtureAdapted"] is True
        and previous["publicApiParityEvidence"] is False
        and previous["inventory"] == str(INVENTORY)
        and previous["inventorySha256"] == INVENTORY_SHA256
        and previous["recordsPerLane"] == COUNT
        and set(previous["lanes"]) == {"candidate", "oracle"},
        "predecessor identity drifted",
    )
    for lane in ("candidate", "oracle"):
        ledger = BASE / f"{lane}.jsonl"
        summary = BASE / f"{lane}.summary.json"
        sealed(ledger, BASE_LEDGER_SHA256[lane])
        sealed(summary, BASE_SUMMARY_SHA256[lane])
        prior_lane = previous["lanes"][lane]
        report = json.loads(summary.read_text())
        require(
            prior_lane["ledger"] == str(ledger)
            and prior_lane["ledgerSha256"] == BASE_LEDGER_SHA256[lane]
            and prior_lane["summarySha256"] == BASE_SUMMARY_SHA256[lane]
            and prior_lane["outcomes"] == report["outcomes"]
            and report["schema"] == 1
            and report["records"] == COUNT
            and report["inventory"] == str(INVENTORY)
            and report["ledger"] == str(ledger)
            and report["ledgerSha256"] == BASE_LEDGER_SHA256[lane],
            f"{lane} predecessor lane drifted",
        )
    return previous


def check_harness() -> None:
    capture.check_browserless_source_and_manifest()
    require(
        {str(path.relative_to(ROOT)) for path in capture.INPUT_FILES}
        == set(HARNESS_PINS),
        "captured harness input set drifted",
    )
    for relative, expected in HARNESS_PINS.items():
        sealed(ROOT / relative, expected)
    sealed(
        capture.SOURCE_PACKAGE / "native" / capture.PROFILE / "pwright.node",
        capture.ARTIFACT_SHA256,
    )
    sealed(capture.SNAPSHOT_MANIFEST, SNAPSHOT_MANIFEST_SHA256)
    require(
        PAIRED_VERIFIER.is_file()
        and not PAIRED_VERIFIER.is_symlink()
        and PROMOTION_TEST.is_file()
        and not PROMOTION_TEST.is_symlink(),
        "independent verifier or mutation controls are missing",
    )


def official_outcomes() -> dict[str, dict[str, Any]]:
    strict_paths = {
        lane: capture.RESULT_PARENT / f"{lane}-v4.strict.json"
        for lane in ("candidate", "oracle")
    }
    for lane, path in strict_paths.items():
        sealed(path, STRICT_SHA256[lane])
    oracle, candidate = verify_pair(strict_paths["oracle"], strict_paths["candidate"])
    strict_by_lane = {"candidate": candidate, "oracle": oracle}
    target_ids = {entry[0] for entry in TARGETS.values()}
    for lane, strict in strict_by_lane.items():
        require(
            strict["schema"] == 1
            and strict["mode"] == lane
            and strict["scope"] == capture.SCOPE
            and strict["sourceClassification"] == "implementation-internal"
            and strict["fixtureAdapted"] is True
            and strict["publicApiParityEvidence"] is False
            and strict["promotedToCoreLedger"] is False
            and strict["upstreamCommit"] == capture.COMMIT
            and strict["upstreamInputs"] == capture.UPSTREAM_INPUTS
            and strict["inputs"] == HARNESS_PINS
            and len(strict["runs"]) == 3,
            f"{lane} strict evidence identity drifted",
        )
        if lane == "candidate":
            require(
                strict["candidateProfile"] == capture.PROFILE
                and strict["candidateArtifactSha256"] == capture.ARTIFACT_SHA256
                and strict["candidateSnapshot"] == str(capture.SNAPSHOT_PACKAGE)
                and strict["candidateSnapshotManifestSha256"]
                == SNAPSHOT_MANIFEST_SHA256
                and strict["candidateSourceSha256"] == CANDIDATE_SOURCE_SHA256,
                "candidate package provenance drifted",
            )
        else:
            require(
                all(
                    strict[key] is None
                    for key in (
                        "candidateProfile",
                        "candidateArtifactSha256",
                        "candidateSnapshot",
                        "candidateSnapshotManifestSha256",
                        "candidateSourceSha256",
                    )
                ),
                "oracle evidence unexpectedly references candidate runtime",
            )
        seen: set[str] = set()
        for run in strict["runs"]:
            project = run["project"]
            require(
                project in capture.PROJECTS
                and run["exitCode"] == 0
                and len(run["selected"]) == 2,
                f"{lane} run failed or selection drifted",
            )
            for case in run["selected"]:
                spec_id = case["specId"]
                require(
                    spec_id in capture.PROJECTS[project]["specs"]
                    and spec_id not in seen,
                    f"{lane} duplicate or wrong official ID",
                )
                seen.add(spec_id)
                require(
                    case["project"] == project
                    and case["kind"] == capture.PROJECTS[project]["specs"][spec_id]
                    and case["line"] == 122
                    and case["title"] == "should print ws error"
                    and case["status"] == "expected"
                    and case["errors"] == []
                    and len(case["attempts"]) == 1
                    and case["attempts"][0]["retry"] == 0
                    and case["attempts"][0]["status"] == "passed",
                    f"{lane} official assertion did not pass once: {spec_id}",
                )
        require(seen == target_ids, f"{lane} selected ID set drifted")
    return strict_by_lane


def transform_lines(
    inventory_lines: list[bytes],
    baseline_lines: list[bytes],
    *,
    lane: str,
    result_by_spec: dict[str, str],
    prior_outcomes: dict[str, int],
    prior_scopes: dict[str, int],
) -> tuple[bytes, dict[str, int], dict[str, int], list[dict[str, Any]]]:
    require(lane in {"candidate", "oracle"}, "unknown ledger lane")
    before: Counter[str] = Counter()
    after: Counter[str] = Counter()
    before_scopes: Counter[str] = Counter()
    after_scopes: Counter[str] = Counter()
    output = bytearray()
    changed: list[dict[str, Any]] = []
    rows = 0
    for index, (source_line, line) in enumerate(
        zip_longest(inventory_lines, baseline_lines)
    ):
        require(
            source_line is not None and line is not None,
            f"{lane} row count drifted at {index}",
        )
        require(
            source_line.endswith(b"\n") and line.endswith(b"\n"),
            f"{lane} unterminated row {index}",
        )
        inventory_row = json.loads(source_line)
        record = json.loads(line)
        require(
            isinstance(inventory_row, dict)
            and isinstance(record, dict)
            and all(record.get(key) == value for key, value in inventory_row.items()),
            f"{lane} inventory identity/classification drift at row {index}",
        )
        before[record["outcome"]] += 1
        if "evidenceScope" in record:
            before_scopes[record["evidenceScope"]] += 1
        if index in TARGETS:
            spec_id, project, kind = TARGETS[index]
            require(
                record == {**inventory_row, "attempts": 0, "outcome": "not-run"}
                and spec_id in result_by_spec
                and record["specId"] == spec_id
                and record["projectId"] == record["projectName"] == project
                and record["sourceFile"] == capture.SOURCE
                and record["file"] == "library/browsertype-connect.spec.ts"
                and record["line"] == 122
                and record["column"] == 5
                and record["titlePath"]
                == [record["file"], kind, "should print ws error"]
                and record["expectedStatus"] == "passed"
                and record["role"] == "official-entrypoint"
                and record["suite"] == "bidi"
                and record["sourceClassification"] == "implementation-internal"
                and record["classificationReasons"] == CLASSIFICATION_REASONS,
                f"{lane} target identity or classification drift at row {index}",
            )
            promoted = {
                **record,
                "attempts": 1,
                "candidateRuntimeRequired": lane == "candidate",
                "evidenceScope": SCOPE,
                "outcome": "expected",
                "resultFile": result_by_spec[spec_id],
            }
            require(
                all(promoted[key] == value for key, value in inventory_row.items()),
                f"{lane} target inventory metadata changed",
            )
            new_line = (
                json.dumps(promoted, sort_keys=True, separators=(",", ":")) + "\n"
            ).encode()
            changed.append(
                {
                    "index": index,
                    "specId": spec_id,
                    "oldRowSha256": digest(line),
                    "newRowSha256": digest(new_line),
                }
            )
            output.extend(new_line)
            after["expected"] += 1
            after_scopes[SCOPE] += 1
        else:
            output.extend(line)
            after[record["outcome"]] += 1
            if "evidenceScope" in record:
                after_scopes[record["evidenceScope"]] += 1
        rows += 1
    require(rows == COUNT, f"{lane} row count drifted")
    require(
        [item["index"] for item in changed] == sorted(TARGETS),
        f"{lane} target count/position drifted",
    )
    require(
        dict(before) == prior_outcomes and dict(before_scopes) == prior_scopes,
        f"{lane} predecessor summary differs from ledger",
    )
    expected_outcomes = before.copy()
    expected_outcomes["not-run"] -= len(TARGETS)
    expected_outcomes["expected"] += len(TARGETS)
    expected_scopes = before_scopes.copy()
    expected_scopes[SCOPE] += len(TARGETS)
    require(
        after == expected_outcomes and after_scopes == expected_scopes,
        f"{lane} promoted outcome/scope counts drifted",
    )
    return (
        bytes(output),
        dict(sorted(after.items())),
        dict(sorted(after_scopes.items())),
        changed,
    )


def prepare_lane(
    lane: str, strict: dict[str, Any]
) -> tuple[bytes, bytes, dict[str, Any]]:
    baseline = BASE / f"{lane}.jsonl"
    previous_summary = json.loads((BASE / f"{lane}.summary.json").read_text())
    result_by_spec = {
        case["specId"]: run["result"]
        for run in strict["runs"]
        for case in run["selected"]
    }
    with INVENTORY.open("rb") as inventory, baseline.open("rb") as source:
        ledger, outcomes, scopes, changed = transform_lines(
            inventory.readlines(),
            source.readlines(),
            lane=lane,
            result_by_spec=result_by_spec,
            prior_outcomes=previous_summary["outcomes"],
            prior_scopes=previous_summary["explicitEvidenceScopes"],
        )
    destination = OUTPUT / f"{lane}.jsonl"
    summary = {
        "schema": 1,
        "inventory": str(INVENTORY),
        "records": COUNT,
        "outcomes": outcomes,
        "explicitEvidenceScopes": scopes,
        "ledgerSha256": digest(ledger),
        "ledger": str(destination),
    }
    summary_bytes = (json.dumps(summary, indent=2) + "\n").encode()
    evidence = {
        "base": str(baseline),
        "baseSha256": BASE_LEDGER_SHA256[lane],
        "baseSummarySha256": BASE_SUMMARY_SHA256[lane],
        "strictSidecar": str(capture.RESULT_PARENT / f"{lane}-v4.strict.json"),
        "strictSidecarSha256": STRICT_SHA256[lane],
        "officialRuns": [
            {
                "project": run["project"],
                "result": run["result"],
                "resultSha256": run["resultSha256"],
                "rawProvenance": str(capture.paths(lane, run["project"])["provenance"]),
                "rawProvenanceSha256": run["provenanceSha256"],
                "guardReceipt": str(capture.paths(lane, run["project"])["receipt"]),
                "guardReceiptSha256": run["guardReceiptSha256"],
            }
            for run in strict["runs"]
        ],
        "candidateArtifactSha256": strict["candidateArtifactSha256"],
        "candidateSnapshotManifestSha256": strict["candidateSnapshotManifestSha256"],
        "candidateSourceSha256": strict["candidateSourceSha256"],
        "ledger": str(destination),
        "ledgerSha256": summary["ledgerSha256"],
        "summarySha256": digest(summary_bytes),
        "oldOutcome": "not-run",
        "newOutcome": "expected",
        "promotedRows": len(TARGETS),
        "changedRows": changed,
        "outcomes": outcomes,
    }
    return ledger, summary_bytes, evidence


def prepare_pair() -> dict[str, bytes]:
    check_predecessor()
    check_harness()
    strict_by_lane = official_outcomes()
    prepared = {
        lane: prepare_lane(lane, strict_by_lane[lane])
        for lane in ("candidate", "oracle")
    }
    receipt = {
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
        "selectedOfficialTestStartsRemoteServer": False,
        "upstreamAssertionsModified": False,
        "captureSidecarPromotedFlagAtCapture": False,
        "promoterSourceSha256": digest(Path(__file__).read_bytes()),
        "strictVerifierSha256": HARNESS_PINS[
            "scripts/verify_playwright_core_server_connect_ws_error_bidi_chromium_strict_v4.py"
        ],
        "pairedVerifierSha256": digest(PAIRED_VERIFIER.read_bytes()),
        "promotionMutationTestSha256": digest(PROMOTION_TEST.read_bytes()),
        "harnessInputs": HARNESS_PINS,
        "upstreamInputs": capture.UPSTREAM_INPUTS,
        "priorBundle": str(BASE),
        "priorBundleReceiptSha256": BASE_RECEIPT_SHA256,
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "recordsPerLane": COUNT,
        "promotedRowsPerLane": len(TARGETS),
        "targetZeroBasedIndexes": sorted(TARGETS),
        "targetSpecIds": [TARGETS[index][0] for index in sorted(TARGETS)],
        "lanes": {lane: item[2] for lane, item in prepared.items()},
    }
    files = {f"{lane}.jsonl": item[0] for lane, item in prepared.items()}
    files.update({f"{lane}.summary.json": item[1] for lane, item in prepared.items()})
    files["promotion-receipt.json"] = (
        json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    ).encode()
    return files


def verify_existing() -> dict[str, Any]:
    expected = prepare_pair()
    require(OUTPUT.is_dir() and not OUTPUT.is_symlink(), "immutable output missing")
    require(
        {path.name for path in OUTPUT.iterdir()} == set(expected),
        "immutable output has missing/extra files",
    )
    for name, payload in expected.items():
        path = OUTPUT / name
        require(
            path.is_file() and not path.is_symlink() and path.read_bytes() == payload,
            f"immutable output drifted: {name}",
        )
    return json.loads(expected["promotion-receipt.json"])


def _publish_exclusive(staging: Path, destination: Path) -> None:
    libc = ctypes.CDLL(None, use_errno=True)
    if sys.platform == "darwin":
        rename = libc.renameatx_np
        flag, at_fdcwd = 0x4, -2
    elif sys.platform.startswith("linux"):
        rename = libc.renameat2
        flag, at_fdcwd = 0x1, -100
    else:
        raise ValueError("atomic no-replace directory publication is unavailable")
    rename.argtypes = [
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_uint,
    ]
    rename.restype = ctypes.c_int
    if (
        rename(at_fdcwd, os.fsencode(staging), at_fdcwd, os.fsencode(destination), flag)
        != 0
    ):
        raise ValueError(
            f"atomic no-replace publication failed: {os.strerror(ctypes.get_errno())}"
        )


def publish(files: dict[str, bytes]) -> None:
    require(
        not OUTPUT.exists() and not OUTPUT.is_symlink(),
        "immutable output already exists",
    )
    OUTCOMES.mkdir(parents=True, exist_ok=True)
    temporary = Path(
        tempfile.mkdtemp(prefix=".core-server-connect-ws-bidi-v4-", dir=OUTCOMES)
    )
    try:
        for name, payload in files.items():
            with (temporary / name).open("xb") as stream:
                stream.write(payload)
                stream.flush()
                os.fsync(stream.fileno())
        directory_fd = os.open(temporary, os.O_RDONLY)
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)
        _publish_exclusive(temporary, OUTPUT)
        parent_fd = os.open(OUTCOMES, os.O_RDONLY)
        try:
            os.fsync(parent_fd)
        finally:
            os.close(parent_fd)
    finally:
        if temporary.exists():
            shutil.rmtree(temporary)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--plan", action="store_true")
    action.add_argument("--publish", action="store_true")
    action.add_argument("--verify-existing", action="store_true")
    args = parser.parse_args()
    try:
        if args.verify_existing:
            receipt = verify_existing()
            print(
                json.dumps(
                    {
                        "scope": receipt["scope"],
                        "lanes": receipt["lanes"],
                        "receiptSha256": digest(
                            (OUTPUT / "promotion-receipt.json").read_bytes()
                        ),
                    },
                    indent=2,
                )
            )
        else:
            files = prepare_pair()
            hashes = {name: digest(payload) for name, payload in files.items()}
            if args.publish:
                require(
                    files == prepare_pair(), "paired inputs changed before publication"
                )
                publish(files)
                verify_existing()
            print(json.dumps({"output": str(OUTPUT), "sha256": hashes}, indent=2))
    except (
        AssertionError,
        OSError,
        ValueError,
        KeyError,
        TypeError,
        json.JSONDecodeError,
    ) as error:
        parser.error(f"strict v4 Bidi WS-error promotion rejected: {error}")


if __name__ == "__main__":
    main()
