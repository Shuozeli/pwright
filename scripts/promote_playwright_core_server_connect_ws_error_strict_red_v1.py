"""Promote two verified official Core Server-connect diagnostics as red evidence.

This immutable successor records candidate failures rather than masking them.
It does not assert public API parity or BrowserServer/browser execution.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import tempfile
from collections import Counter
from collections.abc import Iterable
from itertools import zip_longest
from pathlib import Path
from typing import Any

import promote_playwright_core_cdp_error_slice_strict_v3 as previous
import run_playwright_core_server_connect_ws_error_strict_v1 as capture
from verify_playwright_core_server_connect_ws_error_strict_v1 import verify_strict

ROOT = capture.ROOT
OUTCOMES = ROOT / ".work/official-conformance/outcomes"
BASE = OUTCOMES / "core-cdp-error-slice-strict-paired-v3"
OUTPUT = OUTCOMES / "core-server-connect-ws-error-strict-red-paired-v1"
BASE_RECEIPT_SHA256 = "1f65ad5e2d26ec874fa7c386439c2ef28be58335233cfd5bf4d1f397356d555d"
BASE_PROMOTER_SHA256 = (
    "582a8d633d27a2ac7945b6481090792a4b15867a8416512c5a88cee2a3b22977"
)
BASE_SHA256 = {
    "candidate": "400f1e29f84da3f83076bd97d89ebcf76f20ae4cec48e5c518337fb1f7c56736",
    "oracle": "4c4a397cd098fd28e42b4e579217d808a034b8657a21e09228a1a7fd9dec844b",
}
BASE_SUMMARY_SHA256 = {
    "candidate": "601e4aad2ba94da8d4457efe5f764c1f0b289bd40d79a956176f71363c73123a",
    "oracle": "0105ebaec532326f71a9626e99da44206981ee9f164cc5c0284e037cfa1332f8",
}
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
COUNT = 48_477
TARGETS = {
    31_232: ("ee6df4b3a2217bd356a7-4cd655b8b4dbe5ced07a", "run-server"),
    31_233: ("ee6df4b3a2217bd356a7-ebb1741f4d1b656436cc", "launchServer"),
}
CLASSIFICATION_REASONS = [
    "private-client-access",
    "runtime-source-import:packages/playwright-core/lib/server/utils/network",
    "runtime-source-import:packages/playwright-core/lib/server/utils/userAgent",
]
SCOPE = "official-browser-free-server-connect-ws-error-internal-diagnostic-red-v1"
HARNESS_PINS = {
    "scripts/official_candidate_package_snapshot_v1.py": "20af90af081554d2102f34ca337e7ff1bff438488dfe50f3db0cf8e7d7363fbe",
    "scripts/official_result_destination.py": "61cd172cc58af3fecfe09b9d44da5ae67038ab660f7ed8a140b1ddf1784c04ab",
    "scripts/run_playwright_core_server_connect_ws_error_strict_v1.py": "e87256372e7fd8e6fd89d33071005c923f74a6162f6ac373fe9aa4a16a5065f4",
    "scripts/run_playwright_official_conformance.py": "21f1df10e77485bf29b67b31e548a1bc5126d50dc47cdeb115a9cf6798704a7b",
    "scripts/verify_playwright_core_server_connect_ws_error_strict_v1.py": "b2ecf10994ec7952ec095e0371c83dd8ae83ba3403f034fbee38b3563d3e7b46",
    "tests/official/playwright-core-browser-type-owner-v2-preload.cjs": "b7dc4e3be3aa4c07ac069bc07adfa5a2e65456efc6fc438e49a84a907ffc06ce",
    "tests/official/playwright-core-candidate-preload.cjs": "d17be590ff9334b0d99dd16d0078e4b434a39eae21568b799314ababc6bbf924",
    "tests/official/playwright-core-server-connect-ws-error-strict-v1-manifest.json": "1b1d00bdddc5ecfa21b02b1826ac7caf5a26ac8ff351ae93f430c67a021989ca",
    "tests/official/playwright-core-server-connect-ws-error-strict-v1-preload.cjs": "8961683cd908e302ec6a2319c2562e61972930986c70071db8fec2059b02ae71",
    "tests/official/playwright-core-server-runtime.cjs": "58278e6c976da47074a1eb8d682916ebc3d0a41975fe89fe9270d1a664c396fd",
}
STRICT_SHA256 = {
    "candidate": "2698e8ddbb2932c488c1f93a19a2e2e59a378efd74eccb48b0d666d199a4b073",
    "oracle": "a4c646fd17652b6f59f685ec4b616a4196aefd39972e23cb247ab5bd848457ad",
}
SNAPSHOT_MANIFEST_SHA256 = (
    "698c8f4a5aa9a93fdc9345c5ebd7e729f85f779a771f11e029222ec5f11524a7"
)
ARTIFACT_SHA256 = "641dd2867c3ccb5a3961020bdd2dd3e738cee2a15da141c654b1261e2cb4dedb"
SOURCE_SHA256 = capture.SOURCE_SHA256


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
    sealed(BASE / "promotion-receipt.json", BASE_RECEIPT_SHA256)
    prior = json.loads((BASE / "promotion-receipt.json").read_text())
    require(
        prior["schema"] == 1
        and prior["scope"] == previous.SCOPE
        and prior["inventory"] == str(INVENTORY)
        and prior["inventorySha256"] == INVENTORY_SHA256
        and prior["recordsPerLane"] == COUNT
        and prior["promoterSourceSha256"] == BASE_PROMOTER_SHA256
        and prior["sourceClassificationPreserved"] == "implementation-internal"
        and prior["publicApiParityEvidence"] is False,
        "prior paired receipt identity drifted",
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
            and summary_data["schema"] == 1
            and summary_data["inventory"] == str(INVENTORY)
            and summary_data["records"] == COUNT
            and summary_data["ledger"] == str(ledger)
            and summary_data["ledgerSha256"] == BASE_SHA256[lane],
            f"{lane} prior ledger or summary drifted",
        )
    return prior


def check_harness() -> None:
    capture.check_source_and_manifest()
    require(SOURCE_SHA256 == capture.SOURCE_SHA256, "upstream source pin drifted")
    require(
        ARTIFACT_SHA256 == capture.ARTIFACT_SHA256, "candidate artifact pin drifted"
    )
    require(
        {str(path.relative_to(ROOT)) for path in capture.INPUT_FILES}
        == set(HARNESS_PINS),
        "strict diagnostic harness input set drifted",
    )
    for relative, expected in HARNESS_PINS.items():
        sealed(ROOT / relative, expected)
    sealed(capture.SNAPSHOT_MANIFEST, SNAPSHOT_MANIFEST_SHA256)
    sealed(
        capture.SOURCE_PACKAGE / "native" / capture.PROFILE / "pwright.node",
        ARTIFACT_SHA256,
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
    strict_path = capture.RESULT_PARENT / f"{lane}-v1.strict.json"
    sealed(strict_path, STRICT_SHA256[lane])
    strict = verify_strict(strict_path)
    expected_status = "unexpected" if lane == "candidate" else "expected"
    expected_attempt = "failed" if lane == "candidate" else "passed"
    require(
        strict["mode"] == lane
        and strict["schema"] == 1
        and strict["scope"] == capture.SCOPE
        and strict["sourceClassification"] == "implementation-internal"
        and strict["publicApiParityEvidence"] is False
        and strict["promotedToCoreLedger"] is False
        and strict["upstreamCommit"] == capture.COMMIT
        and strict["upstreamSourceSha256"] == SOURCE_SHA256
        and strict["serverFixtureSha256"] == capture.SERVER_FIXTURE_SHA256
        and strict["testModeFixtureSha256"] == capture.TEST_MODE_FIXTURE_SHA256
        and strict["inputs"] == HARNESS_PINS
        and strict["exitCode"] == (1 if lane == "candidate" else 0),
        f"{lane} strict diagnostic identity drifted",
    )
    if lane == "candidate":
        require(
            strict["candidateProfile"] == capture.PROFILE
            and strict["candidateArtifactSha256"] == ARTIFACT_SHA256
            and strict["candidateSnapshot"] == str(capture.SNAPSHOT_PACKAGE)
            and strict["candidateSnapshotManifestSha256"] == SNAPSHOT_MANIFEST_SHA256
            and isinstance(strict["candidateSourceSha256"], str)
            and len(strict["candidateSourceSha256"]) == 64,
            "candidate runtime artifact or snapshot provenance drifted",
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
    require(len(selected) == 2, f"{lane} strict diagnostic selected count drifted")
    cases = {item["specId"]: item for item in selected}
    require(
        set(cases) == {spec_id for spec_id, _ in TARGETS.values()},
        f"{lane} strict diagnostic selected identities drifted",
    )
    for spec_id, item in cases.items():
        require(
            item["status"] == expected_status
            and len(item["attempts"]) == 1
            and item["attempts"][0]["retry"] == 0
            and item["attempts"][0]["status"] == expected_attempt
            and ((bool(item["errors"])) == (lane == "candidate")),
            f"{lane} official assertion outcome drifted: {spec_id}",
        )
        if lane == "candidate":
            require(
                "browserType.connect is not supported by the current Pwright native profile"
                in item["errors"][0]
                and "<ws connecting> ws://does-not-exist.problem-domain:10987/"
                in item["errors"][0],
                f"{lane} actual mismatch differs from the captured red diagnostic",
            )
    return strict, cases


def transform_lines(
    inventory_lines: Iterable[bytes],
    baseline_lines: Iterable[bytes],
    *,
    lane: str,
    cases: dict[str, dict[str, Any]],
    result_file: str,
    count: int,
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
            require(
                set(record) == set(inventory_row) | {"attempts", "outcome"}
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
                and record["outcome"] == "not-run"
                and record["attempts"] == 0,
                f"{lane} target identity or baseline outcome drift at row {index}",
            )
            case = cases[spec_id]
            require(
                case["kind"] == kind and case["line"] == 122,
                f"{lane} official case does not match ledger target",
            )
            promoted = {
                **record,
                "attempts": 1,
                "outcome": case["status"],
                "resultFile": result_file,
                "evidenceScope": SCOPE,
                "candidateRuntimeRequired": lane == "candidate",
            }
            if lane == "candidate":
                require(
                    case["status"] == "unexpected" and len(case["errors"]) == 1,
                    "candidate official failure was not preserved",
                )
                promoted["error"] = case["errors"][0]
            else:
                require(
                    case["status"] == "expected" and case["errors"] == [],
                    "oracle official pass was not preserved",
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
            after[case["status"]] += 1
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
        f"{lane} baseline summary differs from ledger",
    )
    expected = before.copy()
    expected["not-run"] -= len(TARGETS)
    expected["unexpected" if lane == "candidate" else "expected"] += len(TARGETS)
    require(
        expected["not-run"] >= 0 and after == expected,
        f"{lane} promoted outcome counts drifted",
    )
    scopes = before_scopes.copy()
    scopes[SCOPE] += len(TARGETS)
    require(after_scopes == scopes, f"{lane} promoted evidence scope counts drifted")
    return (
        bytes(output),
        dict(sorted(after.items())),
        dict(sorted(after_scopes.items())),
    )


def prepare_lane(lane: str) -> tuple[bytes, bytes, dict[str, Any]]:
    strict, cases = official_outcomes(lane)
    baseline = BASE / f"{lane}.jsonl"
    previous_summary = json.loads((BASE / f"{lane}.summary.json").read_text())
    with INVENTORY.open("rb") as inventory, baseline.open("rb") as source:
        ledger, outcomes, scopes = transform_lines(
            inventory,
            source,
            lane=lane,
            cases=cases,
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
    receipt = {
        "base": str(baseline),
        "baseSha256": BASE_SHA256[lane],
        "strictSidecar": str(capture.RESULT_PARENT / f"{lane}-v1.strict.json"),
        "strictSidecarSha256": STRICT_SHA256[lane],
        "officialResult": strict["result"],
        "officialResultSha256": strict["resultSha256"],
        "candidateArtifactSha256": strict["candidateArtifactSha256"],
        "candidateSnapshotManifestSha256": strict["candidateSnapshotManifestSha256"],
        "candidateSourceSha256": strict["candidateSourceSha256"],
        "ledger": str(destination),
        "ledgerSha256": summary["ledgerSha256"],
        "summarySha256": digest(summary_bytes),
        "oldOutcome": "not-run",
        "newOutcome": "unexpected" if lane == "candidate" else "expected",
        "outcomes": outcomes,
    }
    return ledger, summary_bytes, receipt


def prepare_pair() -> dict[str, bytes]:
    check_prior_bundle()
    check_harness()
    prepared = {lane: prepare_lane(lane) for lane in ("candidate", "oracle")}
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
            "scripts/verify_playwright_core_server_connect_ws_error_strict_v1.py"
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


def publish(files: dict[str, bytes]) -> None:
    require(
        not OUTPUT.exists() and not OUTPUT.is_symlink(),
        "immutable output already exists",
    )
    OUTCOMES.mkdir(parents=True, exist_ok=True)
    temporary = Path(
        tempfile.mkdtemp(prefix=".core-server-connect-ws-red-v1-", dir=OUTCOMES)
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
        previous._publish_exclusive(temporary, OUTPUT)
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
            f"strict RED paired Core Server-connect promotion rejected: {error}"
        )
    print(json.dumps({"scope": receipt["scope"], "lanes": receipt["lanes"]}, indent=2))


if __name__ == "__main__":
    main()
