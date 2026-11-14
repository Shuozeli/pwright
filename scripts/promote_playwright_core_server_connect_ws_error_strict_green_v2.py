"""Seal the two passing Core Server-connect diagnostics as an immutable pair.

This is implementation-internal, browser-free diagnostic evidence. It does not
establish public API, BrowserServer, or browser-execution parity.
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

import promote_playwright_core_server_connect_ws_error_strict_red_v1 as previous
import run_playwright_core_server_connect_ws_error_strict_v2 as capture
from verify_playwright_core_server_connect_ws_error_strict_v2 import verify_strict

ROOT = capture.ROOT
OUTCOMES = ROOT / ".work/official-conformance/outcomes"
BASE = OUTCOMES / "core-server-connect-ws-error-strict-red-paired-v1"
OUTPUT = OUTCOMES / "core-server-connect-ws-error-strict-green-paired-v2"
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
BASE_PROMOTER_SHA256 = (
    "ef5c49bfd4d3e9e83317a0741f192fe99cee2c381f17249041c21add5dad5761"
)
BASE_RECEIPT_SHA256 = "705d4e894978e35a89ea816ae8d27641d79e4c97dc6fb3d5775577999e43ceb3"
BASE_SHA256 = {
    "candidate": "a4d5c52c183445d8901e158883e275351412bdc8a8cac15734fb1647086fe309",
    "oracle": "d9eb047d9f52247fbcb76828143409d6043f91b48bdf61595c3734946ebde85b",
}
BASE_SUMMARY_SHA256 = {
    "candidate": "b5046050b758bf0330b29712e4d6f10d81297bdabaaa0b16ee0d6469cb87eb07",
    "oracle": "3478b02ef36fdd991338c39457cf610d6091909095e5ff4fa530fb6b5ab959ce",
}
STRICT_SHA256 = {
    "candidate": "912be1ceb5781a982ec22e8e78ff97b95ecfd1311e0282a49636e41ca7bb15bc",
    "oracle": "219d60a35e0edc8b3296fab8746934580b128c1ac23d8018c786b4ccf153fa50",
}
SNAPSHOT_MANIFEST_SHA256 = (
    "5bf1198089c2ec4f6d32486185f805cc4d8600601594ca76750e0c5f674a4793"
)
CANDIDATE_SOURCE_SHA256 = (
    "c5a3a7012d06c130c8efdb06d16ef1b2dc93855c74086f3bfc38fddd11e780f3"
)
HARNESS_PINS = {
    "scripts/official_candidate_package_snapshot_v1.py": "20af90af081554d2102f34ca337e7ff1bff438488dfe50f3db0cf8e7d7363fbe",
    "scripts/official_result_destination.py": "61cd172cc58af3fecfe09b9d44da5ae67038ab660f7ed8a140b1ddf1784c04ab",
    "scripts/run_playwright_core_server_connect_ws_error_strict_v2.py": "ec298baf952a80822d78694d15ddd4df657ec39c08bab467ff0924300985f822",
    "scripts/run_playwright_official_conformance.py": "21f1df10e77485bf29b67b31e548a1bc5126d50dc47cdeb115a9cf6798704a7b",
    "scripts/verify_playwright_core_server_connect_ws_error_strict_v2.py": "33db91c160ce60b837a548a271206ef4be5f3cc790f8aba13a8a59e02ff95294",
    "tests/official/playwright-core-browser-type-owner-v2-preload.cjs": "b7dc4e3be3aa4c07ac069bc07adfa5a2e65456efc6fc438e49a84a907ffc06ce",
    "tests/official/playwright-core-candidate-preload.cjs": "d17be590ff9334b0d99dd16d0078e4b434a39eae21568b799314ababc6bbf924",
    "tests/official/playwright-core-server-connect-ws-error-strict-v2-manifest.json": "c4d034b0a5e6a22dfb1ee5f4140cd7400cd6334905e47dc88e28fb4300f5c811",
    "tests/official/playwright-core-server-connect-ws-error-strict-v2-preload.cjs": "8961683cd908e302ec6a2319c2562e61972930986c70071db8fec2059b02ae71",
    "tests/official/playwright-core-server-runtime.cjs": "58278e6c976da47074a1eb8d682916ebc3d0a41975fe89fe9270d1a664c396fd",
}
COUNT = 48_477
TARGETS = {
    31_232: ("ee6df4b3a2217bd356a7-4cd655b8b4dbe5ced07a", "run-server"),
    31_233: ("ee6df4b3a2217bd356a7-ebb1741f4d1b656436cc", "launchServer"),
}
CLASSIFICATION_REASONS = previous.CLASSIFICATION_REASONS
SCOPE = "official-browser-free-server-connect-ws-error-internal-diagnostic-green-v2"


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
    sealed(Path(previous.__file__).resolve(), BASE_PROMOTER_SHA256)
    receipt_path = BASE / "promotion-receipt.json"
    sealed(receipt_path, BASE_RECEIPT_SHA256)
    prior = json.loads(receipt_path.read_text())
    require(
        prior["schema"] == 1
        and prior["scope"] == previous.SCOPE
        and prior["promoterSourceSha256"] == BASE_PROMOTER_SHA256
        and prior["inventory"] == str(INVENTORY)
        and prior["inventorySha256"] == INVENTORY_SHA256
        and prior["recordsPerLane"] == COUNT
        and prior["targetZeroBasedIndexes"] == sorted(TARGETS)
        and prior["targetSpecIds"] == [TARGETS[index][0] for index in sorted(TARGETS)]
        and prior["sourceClassificationPreserved"] == "implementation-internal"
        and prior["candidateRuntimeAttested"] is True
        and prior["diagnosticOnly"] is True
        and prior["publicApiParityEvidence"] is False
        and prior["browserServerParityEvidence"] is False
        and prior["browserExecutionEvidence"] is False
        and prior["captureSidecarPromotedFlagAtCapture"] is False,
        "RED predecessor receipt identity drifted",
    )
    require(set(prior["lanes"]) == {"candidate", "oracle"}, "RED lane set drifted")
    for lane in ("candidate", "oracle"):
        ledger = BASE / f"{lane}.jsonl"
        summary = BASE / f"{lane}.summary.json"
        sealed(ledger, BASE_SHA256[lane])
        sealed(summary, BASE_SUMMARY_SHA256[lane])
        lane_receipt = prior["lanes"][lane]
        data = json.loads(summary.read_text())
        require(
            lane_receipt["ledger"] == str(ledger)
            and lane_receipt["ledgerSha256"] == BASE_SHA256[lane]
            and lane_receipt["summarySha256"] == BASE_SUMMARY_SHA256[lane]
            and lane_receipt["newOutcome"]
            == ("unexpected" if lane == "candidate" else "expected")
            and lane_receipt["outcomes"] == data["outcomes"]
            and data["schema"] == 1
            and data["inventory"] == str(INVENTORY)
            and data["records"] == COUNT
            and data["ledger"] == str(ledger)
            and data["ledgerSha256"] == BASE_SHA256[lane]
            and data["explicitEvidenceScopes"][previous.SCOPE] == 2,
            f"{lane} RED predecessor ledger or summary drifted",
        )
    return prior


def check_harness() -> None:
    capture.check_source_and_manifest()
    require(
        capture.SOURCE_SHA256 == previous.SOURCE_SHA256,
        "upstream assertion source pin drifted",
    )
    require(
        capture.ARTIFACT_SHA256
        == "5f083ad9425b7b2c253d5be311a338ef712928ea8357aab5035ed67d2d10b3a2",
        "candidate artifact pin drifted",
    )
    require(
        {str(path.relative_to(ROOT)) for path in capture.INPUT_FILES}
        == set(HARNESS_PINS),
        "strict v2 harness input set drifted",
    )
    for relative, expected in HARNESS_PINS.items():
        sealed(ROOT / relative, expected)
    sealed(capture.SNAPSHOT_MANIFEST, SNAPSHOT_MANIFEST_SHA256)
    sealed(
        capture.SOURCE_PACKAGE / "native" / capture.PROFILE / "pwright.node",
        capture.ARTIFACT_SHA256,
    )
    source = (capture.UPSTREAM / capture.SOURCE).read_text()
    marker = "test('should print ws error', async ({ connect, server }) => {"
    body = source[
        source.index(marker) : source.index(
            "\n    test(", source.index(marker) + len(marker)
        )
    ]
    require(
        "startRemoteServer" not in body and "browserType.launch" not in body,
        "selected official test unexpectedly starts a BrowserServer or browser",
    )


def official_outcomes(lane: str) -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    require(lane in {"candidate", "oracle"}, "unknown ledger lane")
    strict_path = capture.RESULT_PARENT / f"{lane}-v2.strict.json"
    sealed(strict_path, STRICT_SHA256[lane])
    strict = verify_strict(strict_path)
    require(
        strict["schema"] == 1
        and strict["mode"] == lane
        and strict["scope"] == capture.SCOPE
        and strict["sourceClassification"] == "implementation-internal"
        and strict["publicApiParityEvidence"] is False
        and strict["promotedToCoreLedger"] is False
        and strict["upstreamCommit"] == capture.COMMIT
        and strict["upstreamSourceSha256"] == capture.SOURCE_SHA256
        and strict["inputs"] == HARNESS_PINS
        and strict["exitCode"] == 0,
        f"{lane} strict v2 diagnostic identity drifted",
    )
    if lane == "candidate":
        require(
            strict["candidateProfile"] == capture.PROFILE
            and strict["candidateArtifactSha256"] == capture.ARTIFACT_SHA256
            and strict["candidateSnapshot"] == str(capture.SNAPSHOT_PACKAGE)
            and strict["candidateSnapshotManifestSha256"] == SNAPSHOT_MANIFEST_SHA256
            and strict["candidateSourceSha256"] == CANDIDATE_SOURCE_SHA256,
            "candidate artifact or snapshot provenance drifted",
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
            "oracle diagnostic contains candidate runtime inputs",
        )
    selected = strict["selected"]
    require(len(selected) == 2, f"{lane} selected count drifted")
    cases = {item["specId"]: item for item in selected}
    require(
        set(cases) == {spec_id for spec_id, _ in TARGETS.values()},
        f"{lane} selected IDs drifted",
    )
    for spec_id, case in cases.items():
        require(
            case["kind"] == dict(TARGETS.values())[spec_id]
            and case["line"] == 122
            and case["title"] == "should print ws error"
            and case["status"] == "expected"
            and case["errors"] == []
            and len(case["attempts"]) == 1
            and case["attempts"][0]["retry"] == 0
            and case["attempts"][0]["status"] == "passed",
            f"{lane} official assertion was not a single clean pass: {spec_id}",
        )
    return strict, cases


def transform_lines(
    inventory_lines: Iterable[bytes],
    baseline_lines: Iterable[bytes],
    *,
    lane: str,
    cases: dict[str, dict[str, Any]],
    prior_result_file: str,
    result_file: str,
    count: int,
    prior_outcomes: dict[str, int],
    prior_scopes: dict[str, int],
) -> tuple[bytes, dict[str, int], dict[str, int]]:
    require(lane in {"candidate", "oracle"}, "unknown ledger lane")
    require(prior_scopes.get(previous.SCOPE) == 2, "RED evidence scope count drifted")
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
            f"{lane} inventory identity or classification drift at row {index}",
        )
        before[record["outcome"]] += 1
        if "evidenceScope" in record:
            before_scopes[record["evidenceScope"]] += 1
        if index in TARGETS:
            spec_id, kind = TARGETS[index]
            require(
                index not in seen and spec_id in cases,
                f"{lane} duplicate/missing target",
            )
            seen.add(index)
            expected_keys = set(inventory_row) | {
                "attempts",
                "outcome",
                "resultFile",
                "evidenceScope",
                "candidateRuntimeRequired",
            }
            if lane == "candidate":
                expected_keys.add("error")
            require(
                set(record) == expected_keys
                and record["specId"] == spec_id
                and record["projectId"] == record["projectName"] == "chromium-library"
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
                and record["classificationReasons"] == CLASSIFICATION_REASONS
                and record["outcome"]
                == ("unexpected" if lane == "candidate" else "expected")
                and record["attempts"] == 1
                and record["resultFile"] == prior_result_file
                and record["evidenceScope"] == previous.SCOPE
                and record["candidateRuntimeRequired"] == (lane == "candidate")
                and (
                    lane != "candidate"
                    or (
                        isinstance(record["error"], str)
                        and "browserType.connect is not supported by the current Pwright native profile"
                        in record["error"]
                    )
                ),
                f"{lane} RED target identity or outcome drift at row {index}",
            )
            case = cases[spec_id]
            require(
                case["kind"] == kind
                and case["line"] == 122
                and case["status"] == "expected"
                and case["errors"] == []
                and len(case["attempts"]) == 1
                and case["attempts"][0]["retry"] == 0
                and case["attempts"][0]["status"] == "passed",
                f"{lane} selected case does not prove target pass",
            )
            promoted = {key: value for key, value in record.items() if key != "error"}
            promoted.update(
                {
                    "outcome": "expected",
                    "resultFile": result_file,
                    "evidenceScope": SCOPE,
                }
            )
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
        rows == count and seen == set(TARGETS),
        f"{lane} target count or position drifted",
    )
    require(
        dict(before) == prior_outcomes and dict(before_scopes) == prior_scopes,
        f"{lane} RED summary differs from ledger",
    )
    expected_outcomes = before.copy()
    if lane == "candidate":
        expected_outcomes["unexpected"] -= 2
        expected_outcomes["expected"] += 2
    expected_scopes = before_scopes.copy()
    del expected_scopes[previous.SCOPE]
    expected_scopes[SCOPE] += 2
    require(
        after == expected_outcomes and after_scopes == expected_scopes,
        f"{lane} GREEN outcome or scope counts drifted",
    )
    return (
        bytes(output),
        dict(sorted(after.items())),
        dict(sorted(after_scopes.items())),
    )


def prepare_lane(
    lane: str, prior: dict[str, Any]
) -> tuple[bytes, bytes, dict[str, Any]]:
    strict, cases = official_outcomes(lane)
    baseline = BASE / f"{lane}.jsonl"
    previous_summary = json.loads((BASE / f"{lane}.summary.json").read_text())
    with INVENTORY.open("rb") as inventory, baseline.open("rb") as source:
        ledger, outcomes, scopes = transform_lines(
            inventory,
            source,
            lane=lane,
            cases=cases,
            prior_result_file=prior["lanes"][lane]["officialResult"],
            result_file=strict["result"],
            count=COUNT,
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
        "strictSidecar": str(capture.RESULT_PARENT / f"{lane}-v2.strict.json"),
        "strictSidecarSha256": STRICT_SHA256[lane],
        "officialResult": strict["result"],
        "officialResultSha256": strict["resultSha256"],
        "candidateArtifactSha256": strict["candidateArtifactSha256"],
        "candidateSnapshotManifestSha256": strict["candidateSnapshotManifestSha256"],
        "candidateSourceSha256": strict["candidateSourceSha256"],
        "ledger": str(destination),
        "ledgerSha256": summary["ledgerSha256"],
        "summarySha256": digest(summary_bytes),
        "oldOutcome": "unexpected" if lane == "candidate" else "expected",
        "newOutcome": "expected",
        "outcomes": outcomes,
    }
    return ledger, summary_bytes, evidence


def prepare_pair() -> dict[str, bytes]:
    prior = check_prior_bundle()
    check_harness()
    prepared = {lane: prepare_lane(lane, prior) for lane in ("candidate", "oracle")}
    receipt = {
        "schema": 1,
        "scope": SCOPE,
        "candidateRuntimeAttested": True,
        "publicApiParityEvidence": False,
        "diagnosticOnly": True,
        "browserServerParityEvidence": False,
        "browserExecutionEvidence": False,
        "selectedOfficialTestRequiresBrowser": False,
        "selectedOfficialTestStartsRemoteServer": False,
        "upstreamAssertionsModified": False,
        "sourceClassificationPreserved": "implementation-internal",
        "captureSidecarPromotedFlagAtCapture": False,
        "promoterSourceSha256": sha256(Path(__file__).resolve()),
        "strictVerifierSha256": HARNESS_PINS[
            "scripts/verify_playwright_core_server_connect_ws_error_strict_v2.py"
        ],
        "harnessInputs": HARNESS_PINS,
        "priorBundle": str(BASE),
        "priorBundleReceiptSha256": BASE_RECEIPT_SHA256,
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "recordsPerLane": COUNT,
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
        "immutable output has missing or extra files",
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
        tempfile.mkdtemp(prefix=".core-server-connect-ws-green-v2-", dir=OUTCOMES)
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
        parser.error(
            f"strict GREEN paired Core Server-connect promotion rejected: {error}"
        )
    print(json.dumps({"scope": receipt["scope"], "lanes": receipt["lanes"]}, indent=2))


if __name__ == "__main__":
    main()
