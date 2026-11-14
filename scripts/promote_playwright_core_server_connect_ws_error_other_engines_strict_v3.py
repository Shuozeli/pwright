#!/usr/bin/env python3
"""Seal four browser-free Firefox/WebKit WS-error assertions in a Core ledger pair.

This is implementation-internal diagnostic evidence. The in-memory server
fixture adaptation and these assertions do not prove public API or browser
execution parity.
"""

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
from collections.abc import Iterable
from itertools import zip_longest
from pathlib import Path
from typing import Any

import run_playwright_core_server_connect_ws_error_other_engines_strict_v3 as capture
from verify_playwright_core_server_connect_ws_error_other_engines_strict_v3 import (
    verify_pair,
)

ROOT = capture.ROOT
OUTCOMES = ROOT / ".work/official-conformance/outcomes"
BASE = OUTCOMES / "core-dummy-node-other-engines-strict-paired-v1"
OUTPUT = OUTCOMES / "core-server-connect-ws-error-other-engines-strict-paired-v3"
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
BASE_PROMOTER_SHA256 = (
    "47f390de3fb55db8b9c669c90d347ab421505632c00c88a362cd0084ccdec249"
)
BASE_RECEIPT_SHA256 = "27131a3e31ff27d3a7dd676f97581c8fcdf09e19a76647d3ffc83d689edf15c5"
BASE_SHA256 = {
    "candidate": "d0ad535d17c595414bca06f7f81c33515cf92b12d28fb025f915e9482e7fa8fb",
    "oracle": "c176a3e55da3556ff17c263b71044079ba5c467ca5925464ca03f514b8e41b56",
}
BASE_SUMMARY_SHA256 = {
    "candidate": "c62f53b66968f11b3278ddb8c8e20a3aa3a6d0077cad04eaeec040c0d3f18277",
    "oracle": "ab690f080fb2b962c6d67c4083ccb7d5446cac1e15b22d5ae1cb591ac53b348b",
}
STRICT_SHA256 = {
    "candidate": "dbea2e573720908777738c9fc97f893659d144b59c37bd47c5368f3f57347241",
    "oracle": "6235e35e8ac6bbe31b3fa71c5f9f0cac947ef085979c00dcb5197defca10f332",
}
SNAPSHOT_MANIFEST_SHA256 = (
    "64d2c47cac438316ab86ed1bef009eef097fd71a9352c313d9ef42a00a53228b"
)
CANDIDATE_SOURCE_SHA256 = (
    "a2bb47a7c22f88b1c8cbaff72fb107de0a015eec85f6a4101d94e0cbddbfca5a"
)
HARNESS_PINS = {
    "scripts/official_candidate_package_snapshot_v1.py": "20af90af081554d2102f34ca337e7ff1bff438488dfe50f3db0cf8e7d7363fbe",
    "scripts/official_result_destination.py": "61cd172cc58af3fecfe09b9d44da5ae67038ab660f7ed8a140b1ddf1784c04ab",
    "scripts/run_playwright_core_server_connect_ws_error_other_engines_strict_v3.py": "1943cb5d93799315b6f79659c5a34831297d7f047c0f3dcb758be431f7b960a2",
    "scripts/run_playwright_core_server_connect_ws_error_strict_v2.py": "ec298baf952a80822d78694d15ddd4df657ec39c08bab467ff0924300985f822",
    "scripts/run_playwright_official_conformance.py": "21f1df10e77485bf29b67b31e548a1bc5126d50dc47cdeb115a9cf6798704a7b",
    "scripts/test_verify_playwright_core_server_connect_ws_error_other_engines_strict_v3.py": "7cbfd30b7c71e42d8af363f5aa9295e5e8b8fa559dd9f2f51538ff2d24b34068",
    "scripts/verify_playwright_core_server_connect_ws_error_other_engines_strict_v3.py": "b3b6628f5b513ca9c6e19619e93cb2fe071963a7c2a311d6e0245624b5efa368",
    "tests/official/playwright-core-browser-type-owner-v2-preload.cjs": "b7dc4e3be3aa4c07ac069bc07adfa5a2e65456efc6fc438e49a84a907ffc06ce",
    "tests/official/playwright-core-candidate-preload.cjs": "d17be590ff9334b0d99dd16d0078e4b434a39eae21568b799314ababc6bbf924",
    "tests/official/playwright-core-server-connect-ws-error-other-engines-strict-v3-manifest.json": "4546b88c76cdd35390a1be4815fbc1449e466d07e73d5daeb63f843aa2854d18",
    "tests/official/playwright-core-server-connect-ws-error-strict-v2-preload.cjs": "8961683cd908e302ec6a2319c2562e61972930986c70071db8fec2059b02ae71",
    "tests/official/playwright-core-server-runtime.cjs": "58278e6c976da47074a1eb8d682916ebc3d0a41975fe89fe9270d1a664c396fd",
}
COUNT = 48_477
TARGETS = {
    31_234: (
        "ee6df4b3a2217bd356a7-7ff2dc18d1e08d985682",
        "firefox-library",
        "launchServer",
    ),
    31_235: (
        "ee6df4b3a2217bd356a7-95a86c47485eb83c0016",
        "firefox-library",
        "run-server",
    ),
    31_236: (
        "ee6df4b3a2217bd356a7-184cfa77707c96da434b",
        "webkit-library",
        "run-server",
    ),
    31_237: (
        "ee6df4b3a2217bd356a7-6d5f8a4e2638a54cc171",
        "webkit-library",
        "launchServer",
    ),
}
CLASSIFICATION_REASONS = [
    "private-client-access",
    "runtime-source-import:packages/playwright-core/lib/server/utils/network",
    "runtime-source-import:packages/playwright-core/lib/server/utils/userAgent",
]
SCOPE = "official-browser-free-server-connect-ws-error-internal-diagnostic-green-v3-other-engines"
SUCCESSOR_SHA256 = {
    "candidate": "e7d5924d9face0aaac68f5bf1db35516cb6d687871aa34c55361b961e69aee2d",
    "oracle": "3db929fbe661c35aee7f05c9ea4ae3353a0fc4dd249550a1ea43f918f1de8865",
}
PAIRED_VERIFIER = (
    ROOT
    / "scripts/verify_playwright_core_server_connect_ws_error_other_engines_paired_v3.py"
)
PAIRED_VERIFIER_SHA256 = (
    "4db6eda42c8b4700eb62a97e3fb3ab7ca046e4d1d4c9540269cf203f9959f971"
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256(path: Path) -> str:
    return digest(path.read_bytes())


def sealed(path: Path, expected: str) -> None:
    require(
        len(expected) == 64 and all(c in "0123456789abcdef" for c in expected),
        f"invalid SHA-256 pin: {path}",
    )
    require(
        path.is_file() and not path.is_symlink(), f"missing or linked evidence: {path}"
    )
    require(sha256(path) == expected, f"SHA-256 drift: {path}")


def check_prior_bundle() -> dict[str, Any]:
    sealed(INVENTORY, INVENTORY_SHA256)
    sealed(
        ROOT / "scripts/promote_playwright_core_dummy_node_other_engines_strict_v1.py",
        BASE_PROMOTER_SHA256,
    )
    receipt_path = BASE / "promotion-receipt.json"
    sealed(receipt_path, BASE_RECEIPT_SHA256)
    prior = json.loads(receipt_path.read_text())
    require(
        prior["schema"] == 1
        and prior["scope"]
        == "official-browser-free-browsertype-dummy-node-other-engines-strict-red-v1"
        and prior["promotionKind"] == "public-api-official-red"
        and prior["fixtureAdapted"] is True
        and prior["publicApiParityEvidence"] is False
        and prior["promoterSourceSha256"] == BASE_PROMOTER_SHA256
        and prior["inventory"] == str(INVENTORY)
        and prior["inventorySha256"] == INVENTORY_SHA256
        and prior["recordsPerLane"] == COUNT
        and set(prior["lanes"]) == {"candidate", "oracle"},
        "predecessor receipt identity drifted",
    )
    for lane in ("candidate", "oracle"):
        ledger = BASE / f"{lane}.jsonl"
        summary = BASE / f"{lane}.summary.json"
        sealed(ledger, BASE_SHA256[lane])
        sealed(summary, BASE_SUMMARY_SHA256[lane])
        lane_receipt = prior["lanes"][lane]
        summary_data = json.loads(summary.read_text())
        require(
            lane_receipt["ledger"] == str(ledger)
            and lane_receipt["ledgerSha256"] == BASE_SHA256[lane]
            and lane_receipt["summarySha256"] == BASE_SUMMARY_SHA256[lane]
            and lane_receipt["outcomes"] == summary_data["outcomes"]
            and summary_data["schema"] == 1
            and summary_data["inventory"] == str(INVENTORY)
            and summary_data["records"] == COUNT
            and summary_data["ledger"] == str(ledger)
            and summary_data["ledgerSha256"] == BASE_SHA256[lane],
            f"{lane} predecessor ledger or summary drifted",
        )
    return prior


def check_harness() -> None:
    capture.check_browserless_source_and_manifest()
    require(
        {str(path.relative_to(ROOT)) for path in capture.INPUT_FILES}
        == set(HARNESS_PINS),
        "strict v3 input set drifted",
    )
    for relative, expected in HARNESS_PINS.items():
        sealed(ROOT / relative, expected)
    sealed(PAIRED_VERIFIER, PAIRED_VERIFIER_SHA256)
    sealed(
        capture.SOURCE_PACKAGE / "native" / capture.PROFILE / "pwright.node",
        capture.ARTIFACT_SHA256,
    )
    sealed(capture.SNAPSHOT_MANIFEST, SNAPSHOT_MANIFEST_SHA256)


def official_outcomes() -> dict[str, dict[str, Any]]:
    strict_paths = {
        lane: capture.RESULT_PARENT / f"{lane}-v3.strict.json"
        for lane in ("candidate", "oracle")
    }
    for lane, strict_path in strict_paths.items():
        sealed(strict_path, STRICT_SHA256[lane])
    oracle, candidate = verify_pair(strict_paths["oracle"], strict_paths["candidate"])
    strict_by_lane = {"candidate": candidate, "oracle": oracle}
    expected_ids = {item[0] for item in TARGETS.values()}
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
            and len(strict["runs"]) == len(capture.PROJECTS),
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
                project in capture.PROJECTS and run["exitCode"] == 0,
                f"{lane} run failed",
            )
            require(len(run["selected"]) == 2, f"{lane} selected count drifted")
            for case in run["selected"]:
                spec_id = case["specId"]
                require(spec_id not in seen, f"{lane} duplicate official ID")
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
        require(seen == expected_ids, f"{lane} selected ID set drifted")
    return strict_by_lane


def transform_lines(
    inventory_lines: Iterable[bytes],
    baseline_lines: Iterable[bytes],
    *,
    lane: str,
    result_by_spec: dict[str, str],
    prior_outcomes: dict[str, int],
    prior_scopes: dict[str, int],
) -> tuple[bytes, dict[str, int], dict[str, int]]:
    require(lane in {"candidate", "oracle"}, "unknown ledger lane")
    before: Counter[str] = Counter()
    after: Counter[str] = Counter()
    before_scopes: Counter[str] = Counter()
    after_scopes: Counter[str] = Counter()
    output = bytearray()
    seen: set[int] = set()
    rows = 0
    for index, (source_line, line) in enumerate(
        zip_longest(inventory_lines, baseline_lines)
    ):
        require(
            source_line is not None and line is not None, f"{lane} row count drifted"
        )
        require(
            source_line.endswith(b"\n") and line.endswith(b"\n"),
            f"{lane} unterminated row {index}",
        )
        inventory_row = json.loads(source_line)
        record = json.loads(line)
        require(
            isinstance(inventory_row, dict) and isinstance(record, dict),
            f"{lane} non-object row {index}",
        )
        require(
            all(record.get(key) == value for key, value in inventory_row.items()),
            f"{lane} inventory identity/classification drift at row {index}",
        )
        before[record["outcome"]] += 1
        if "evidenceScope" in record:
            before_scopes[record["evidenceScope"]] += 1
        if index in TARGETS:
            spec_id, project, kind = TARGETS[index]
            require(
                index not in seen and spec_id in result_by_spec,
                f"{lane} target missing",
            )
            seen.add(index)
            require(
                record == {**inventory_row, "attempts": 0, "outcome": "not-run"}
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
                and record["suite"] == "library"
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
                "promotion changed target inventory metadata",
            )
            output.extend(
                (
                    json.dumps(promoted, sort_keys=True, separators=(",", ":")) + "\n"
                ).encode()
            )
            after["expected"] += 1
            after_scopes[SCOPE] += 1
        else:
            output.extend(line)
            after[record["outcome"]] += 1
            if "evidenceScope" in record:
                after_scopes[record["evidenceScope"]] += 1
        rows += 1
    require(
        rows == COUNT and seen == set(TARGETS), f"{lane} target count/position drifted"
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
    result = bytes(output)
    require(
        digest(result) == SUCCESSOR_SHA256[lane],
        f"{lane} virtual successor hash drifted",
    )
    return result, dict(sorted(after.items())), dict(sorted(after_scopes.items()))


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
        ledger, outcomes, scopes = transform_lines(
            inventory,
            source,
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
        "baseSha256": BASE_SHA256[lane],
        "strictSidecar": str(capture.RESULT_PARENT / f"{lane}-v3.strict.json"),
        "strictSidecarSha256": STRICT_SHA256[lane],
        "officialResults": [
            {
                "project": run["project"],
                "result": run["result"],
                "resultSha256": run["resultSha256"],
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
        "outcomes": outcomes,
    }
    return ledger, summary_bytes, evidence


def prepare_pair() -> dict[str, bytes]:
    check_prior_bundle()
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
        "promoterSourceSha256": sha256(Path(__file__).resolve()),
        "strictVerifierSha256": HARNESS_PINS[
            "scripts/verify_playwright_core_server_connect_ws_error_other_engines_strict_v3.py"
        ],
        "pairedVerifierSha256": PAIRED_VERIFIER_SHA256,
        "harnessInputs": HARNESS_PINS,
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
        tempfile.mkdtemp(
            prefix=".core-server-connect-ws-other-engines-v3-", dir=OUTCOMES
        )
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
    parser.add_argument("--verify-existing", action="store_true")
    args = parser.parse_args()
    try:
        if args.verify_existing:
            receipt = verify_existing()
        else:
            files = prepare_pair()
            require(files == prepare_pair(), "paired inputs changed before publication")
            publish(files)
            receipt = verify_existing()
    except (
        AssertionError,
        OSError,
        ValueError,
        KeyError,
        TypeError,
        json.JSONDecodeError,
    ) as error:
        parser.error(f"strict v3 paired Core WS-error promotion rejected: {error}")
    print(json.dumps({"scope": receipt["scope"], "lanes": receipt["lanes"]}, indent=2))


if __name__ == "__main__":
    main()
