"""Prepare an immutable post-fix successor for three official Core CDP errors.

The candidate's three v2 failures become passes only when independent v3
strict evidence proves all three unchanged official assertions passed.
The oracle's three existing passes receive fresh v3 evidence. All other rows
and all inventoried implementation-internal classifications are preserved.
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
from itertools import zip_longest
from pathlib import Path
from typing import Any

import promote_playwright_core_cdp_error_slice_strict_v2 as previous
import run_playwright_core_cdp_error_slice_strict_v1 as source
import run_playwright_core_cdp_error_slice_strict_v2 as worker_aware
import run_playwright_core_cdp_error_slice_strict_v3 as capture
from verify_playwright_core_cdp_error_slice_strict_v3 import verify_strict

ROOT = source.ROOT
OUTCOMES = ROOT / ".work/official-conformance/outcomes"
BASE = OUTCOMES / "core-cdp-error-slice-strict-paired-v2"
OUTPUT = OUTCOMES / "core-cdp-error-slice-strict-paired-v3"
BASE_RECEIPT_SHA256 = "92c6720100cfbecdef6ad36e3bd8fdaf3f327251b4200f83dafeb0ef893e4cb4"
BASE_PROMOTER_SHA256 = "1a41f80e08d476c803d6bd76d351bba943e5216ca7312dedb9340434faedbe99"
BASE_SHA256 = {
    "candidate": "c6304c593856a8a94b24ae173ea385609016139989dd079fc4ef4878f811facd",
    "oracle": "6478b726429f40bec7d8c22f943442c57345f246070f2c384386ba33cff376d2",
}
INVENTORY = previous.INVENTORY
INVENTORY_SHA256 = previous.INVENTORY_SHA256
COUNT = 48_477
TARGETS = previous.TARGETS
IDENTITY_FIELDS = previous.IDENTITY_FIELDS
CLASSIFICATION_REASONS = previous.CLASSIFICATION_REASONS
OLD_SCOPE = previous.SCOPE
SCOPE = "official-browser-free-cdp-error-slice-post-fix-strict-paired-v3"
OLD_RESULT_PARENT = ROOT / ".work/official-conformance/results/core-cdp-error-slice-strict-v2"
ARTIFACT_PATH = ROOT / "packages/pwright/native/official-conformance-page-cdp-errors/pwright.node"
ARTIFACT_SHA256 = "0d7dcb931c42d5e912b594ec20fd12e202cc76590c7cab93187fe6829388f5c6"
SNAPSHOT_MANIFEST_SHA256 = "c90f2f9d9b7904610abfeb3a37ee720e988b37748bedf8849cf36c7658534115"
HARNESS_PINS = {
    "tests/official/playwright-core-cdp-error-slice-strict-v3-manifest.json": (
        "f52a83319e706359162b5d1d684461dab8eae0b997cb26d22e2936d499c91d21"
    ),
    "tests/official/playwright-core-cdp-error-slice-strict-v1-preload.cjs": (
        "27b6feb7bba6cd7c2e879f648667b3f0f3eea5b8c4cb8fb3dcd70bfc1c5488ee"
    ),
    "tests/official/playwright-core-server-runtime.cjs": (
        "58278e6c976da47074a1eb8d682916ebc3d0a41975fe89fe9270d1a664c396fd"
    ),
    "scripts/run_playwright_core_cdp_error_slice_strict_v1.py": (
        "f88141d837297ce694a950355e9360d4f08908df02b1a1c8d4b7b6fb65f0e326"
    ),
    "scripts/run_playwright_core_cdp_error_slice_strict_v2.py": (
        "eee94842481ff020c0fc850c67068e1e4d30cff3ed9b48502780085dafe3651f"
    ),
    "scripts/official_candidate_package_snapshot_v1.py": (
        "20af90af081554d2102f34ca337e7ff1bff438488dfe50f3db0cf8e7d7363fbe"
    ),
    "scripts/run_playwright_core_cdp_error_slice_strict_v3.py": (
        "a6647888806606ef976a5ad7dc829befd436a896885e28106e6a8e856f0f308c"
    ),
    "scripts/verify_playwright_core_cdp_error_slice_strict_v3.py": (
        "35903482deaec3d4d03fcbe51775fd4a9fc5bb9c5bed90f0015c8d3edcb054da"
    ),
}
VERIFIER = ROOT / "scripts/verify_playwright_core_cdp_error_slice_strict_v3.py"
LANES = {
    lane: {
        "strict": capture.RESULT_PARENT / f"{lane}-v1.strict.json",
        "strictSha256": (
            "8fafa6df5fc6f23b07dced0eeda1c31fd3112faca100cfa8de9da6c80fdd292c"
            if lane == "candidate"
            else "a643218682535391d8d322c118775b7a7940f02b31e2c473e27facf312b301d1"
        ),
        "oldOutcome": "unexpected" if lane == "candidate" else "expected",
    }
    for lane in ("candidate", "oracle")
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256(path: Path) -> str:
    return digest(path.read_bytes())


def sealed(path: Path, expected: str) -> None:
    require(
        len(expected) == 64 and all(character in "0123456789abcdef" for character in expected),
        f"unfilled or invalid SHA-256 pin: {path}",
    )
    require(path.is_file() and not path.is_symlink(), f"missing or linked evidence: {path}")
    require(sha256(path) == expected, f"SHA-256 drift: {path}")


def check_prior_bundle() -> dict[str, Any]:
    sealed(INVENTORY, INVENTORY_SHA256)
    sealed(Path(previous.__file__).resolve(), BASE_PROMOTER_SHA256)
    sealed(BASE / "promotion-receipt.json", BASE_RECEIPT_SHA256)
    prior = json.loads((BASE / "promotion-receipt.json").read_text())
    require(
        prior["inventory"] == str(INVENTORY)
        and prior["inventorySha256"] == INVENTORY_SHA256
        and prior["recordsPerLane"] == COUNT
        and prior["promoterSourceSha256"] == BASE_PROMOTER_SHA256
        and prior["targetZeroBasedIndexes"] == sorted(TARGETS)
        and prior["targetSpecIds"] == [TARGETS[index][1] for index in sorted(TARGETS)]
        and prior["sourceClassificationPreserved"] == "implementation-internal"
        and prior["publicApiParityEvidence"] is False,
        "prior paired receipt identity drifted",
    )
    for lane, expected in BASE_SHA256.items():
        ledger = BASE / f"{lane}.jsonl"
        sealed(ledger, expected)
        require(
            prior["lanes"][lane]["ledger"] == str(ledger)
            and prior["lanes"][lane]["ledgerSha256"] == expected
            and prior["lanes"][lane]["newOutcome"] == LANES[lane]["oldOutcome"],
            f"{lane} prior ledger receipt drifted",
        )
        summary = BASE / f"{lane}.summary.json"
        sealed(summary, prior["lanes"][lane]["summarySha256"])
        summary_data = json.loads(summary.read_text())
        require(
            summary_data["schema"] == 1
            and summary_data["records"] == COUNT
            and summary_data["inventory"] == str(INVENTORY)
            and summary_data["ledger"] == str(ledger)
            and summary_data["ledgerSha256"] == expected,
            f"{lane} prior summary drifted",
        )
    return prior


def check_harness() -> None:
    require(
        {str(path.relative_to(ROOT)) for path in capture.INPUT_FILES} == set(HARNESS_PINS),
        "strict v3 harness input set drifted",
    )
    for relative, expected in HARNESS_PINS.items():
        sealed(ROOT / relative, expected)
    require(
        HARNESS_PINS[str(VERIFIER.relative_to(ROOT))] == sha256(VERIFIER),
        "verifier pin drifted",
    )
    require(
        ARTIFACT_SHA256 == capture.ARTIFACT_SHA256
        and capture.PROFILE == "official-conformance-page-cdp-errors",
        "v3 candidate artifact/profile pin drifted",
    )
    sealed(ARTIFACT_PATH, ARTIFACT_SHA256)
    sealed(capture.SNAPSHOT_MANIFEST, SNAPSHOT_MANIFEST_SHA256)
    capture.check_manifest()


def official_passes(lane: str) -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    config = LANES[lane]
    strict_path = config["strict"]
    sealed(strict_path, config["strictSha256"])
    strict = verify_strict(strict_path)
    require(
        strict["schema"] == 4
        and strict["mode"] == lane
        and strict["scope"] == capture.SCOPE
        and strict["upstreamCommit"] == source.COMMIT
        and strict["upstreamSourceSha256"] == source.SOURCE_SHA256
        and strict["serverFixtureSha256"] == source.SERVER_FIXTURE_SHA256
        and strict["commonPreloadSha256"] == source.COMMON_PRELOAD_SHA256
        and strict["inputs"] == HARNESS_PINS
        and strict["candidateArtifactSha256"] == ARTIFACT_SHA256
        and strict["exitCode"] == 0,
        f"{lane} strict official pass provenance drifted",
    )
    require(
        strict["candidateProfile"] == (capture.PROFILE if lane == "candidate" else None)
        and strict["pairedOracleStrictSha256"]
        == (LANES["oracle"]["strictSha256"] if lane == "candidate" else None),
        f"{lane} oracle/runtime pairing drifted",
    )
    if lane == "candidate":
        require(
            strict["candidateSnapshot"] == str(capture.SNAPSHOT_PACKAGE)
            and strict["candidateSnapshotManifestSha256"] == SNAPSHOT_MANIFEST_SHA256
            and isinstance(strict["candidateSourceSha256"], str)
            and len(strict["candidateSourceSha256"]) == 64,
            "candidate package snapshot provenance missing",
        )
    else:
        require(
            strict["candidateSnapshot"] is None
            and strict["candidateSnapshotManifestSha256"] is None
            and strict["candidateSourceSha256"] is None,
            "oracle contains candidate snapshot provenance",
        )
    result = Path(strict["result"])
    require(
        result == capture.RESULT_PARENT / f"{lane}-v1.json"
        and result.is_file()
        and not result.is_symlink()
        and result.with_suffix(".strict.json") == strict_path
        and sha256(result) == strict["resultSha256"],
        f"{lane} official result identity drifted",
    )
    selected = worker_aware.selected_tests(json.loads(result.read_text()))
    require(strict["selected"] == selected and len(selected) == 3, f"{lane} selected set drifted")
    by_id = {item["specId"]: item for item in selected}
    require(len(by_id) == 3, f"{lane} selected duplicate spec IDs")
    for _, spec_id, _ in TARGETS.values():
        item = by_id[spec_id]
        require(
            item["status"] == "expected"
            and len(item["attempts"]) == 1
            and item["attempts"][0]["retry"] == 0
            and item["attempts"][0]["status"] == "passed"
            and item["errors"] == [],
            f"{lane} official assertion is not an actual pass: {spec_id}",
        )
    return strict, by_id


def check_target(
    record: dict[str, Any], inventory_row: dict[str, Any], index: int, lane: str
) -> None:
    line, spec_id, title = TARGETS[index]
    expected_keys = set(inventory_row) | {
        "attempts",
        "candidateRuntimeRequired",
        "evidenceScope",
        "outcome",
        "resultFile",
    }
    if lane == "candidate":
        expected_keys.add("error")
    require(set(record) == expected_keys, f"{lane} target {index} prior fields drifted")
    require(
        record["specId"] == spec_id
        and record["projectId"] == record["projectName"] == "chromium-library"
        and record["sourceFile"] == source.SOURCE
        and record["file"] == source.REPORT_FILE
        and record["line"] == line
        and record["column"] == 5
        and record["titlePath"] == [source.REPORT_FILE, title]
        and record["expectedStatus"] == "passed"
        and record["role"] == "official-entrypoint"
        and record["suite"] == "library"
        and record["sourceClassification"] == "implementation-internal"
        and record["classificationReasons"] == CLASSIFICATION_REASONS
        and record["outcome"] == LANES[lane]["oldOutcome"]
        and record["attempts"] == 1
        and record["candidateRuntimeRequired"] is (lane == "candidate")
        and record["evidenceScope"] == OLD_SCOPE
        and record["resultFile"] == str(OLD_RESULT_PARENT / f"{lane}-v1.json"),
        f"{lane} target {index} identity, classification, or old outcome drifted",
    )
    if lane == "candidate":
        require(isinstance(record["error"], str) and bool(record["error"]), "missing old failure")


def prepare_lane(lane: str) -> tuple[bytes, bytes, dict[str, Any]]:
    require(lane in LANES, f"unknown lane: {lane}")
    strict, cases = official_passes(lane)
    prior_summary = json.loads((BASE / f"{lane}.summary.json").read_text())
    before: Counter[str] = Counter()
    after: Counter[str] = Counter()
    before_scopes: Counter[str] = Counter()
    after_scopes: Counter[str] = Counter()
    output = bytearray()
    rows = 0
    with INVENTORY.open("rb") as inventory, (BASE / f"{lane}.jsonl").open("rb") as baseline:
        for index, (source_line, line) in enumerate(zip_longest(inventory, baseline)):
            require(source_line is not None and line is not None, f"{lane} row count drifted")
            require(source_line.endswith(b"\n") and line.endswith(b"\n"), "unterminated row")
            inventory_row = json.loads(source_line)
            record = json.loads(line)
            require(
                isinstance(inventory_row, dict)
                and isinstance(record, dict)
                and all(record.get(key) == inventory_row.get(key) for key in IDENTITY_FIELDS),
                f"{lane} inventory identity/classification drift at row {index}",
            )
            before[record["outcome"]] += 1
            if "evidenceScope" in record:
                before_scopes[record["evidenceScope"]] += 1
            if index in TARGETS:
                check_target(record, inventory_row, index, lane)
                promoted = {key: value for key, value in record.items() if key != "error"}
                promoted.update(
                    {
                        "outcome": "expected",
                        "resultFile": strict["result"],
                        "evidenceScope": SCOPE,
                    }
                )
                require(
                    all(promoted[key] == record[key] for key in IDENTITY_FIELDS)
                    and promoted["attempts"] == 1,
                    "promotion changed target inventory metadata",
                )
                output.extend(
                    (json.dumps(promoted, sort_keys=True, separators=(",", ":")) + "\n").encode()
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
        rows == COUNT
        and dict(before) == prior_summary["outcomes"]
        and dict(before_scopes) == prior_summary["explicitEvidenceScopes"],
        f"{lane} baseline summary differs from ledger",
    )
    expected = before.copy()
    if lane == "candidate":
        expected["unexpected"] -= 3
        expected["expected"] += 3
    scopes = before_scopes.copy()
    scopes[OLD_SCOPE] -= 3
    scopes[SCOPE] += 3
    require(after == expected and after_scopes == scopes, f"{lane} promoted counts drifted")
    ledger = bytes(output)
    destination = OUTPUT / f"{lane}.jsonl"
    summary = {
        "schema": 1,
        "inventory": str(INVENTORY),
        "records": COUNT,
        "outcomes": dict(sorted(after.items())),
        "explicitEvidenceScopes": dict(sorted(after_scopes.items())),
        "ledgerSha256": digest(ledger),
        "ledger": str(destination),
    }
    summary_bytes = (json.dumps(summary, indent=2) + "\n").encode()
    receipt = {
        "base": str(BASE / f"{lane}.jsonl"),
        "baseSha256": BASE_SHA256[lane],
        "strictSidecar": str(LANES[lane]["strict"]),
        "strictSidecarSha256": LANES[lane]["strictSha256"],
        "officialResult": strict["result"],
        "officialResultSha256": strict["resultSha256"],
        "candidateArtifactSha256": ARTIFACT_SHA256,
        "candidateSnapshotManifestSha256": strict["candidateSnapshotManifestSha256"],
        "candidateSourceSha256": strict["candidateSourceSha256"],
        "ledger": str(destination),
        "ledgerSha256": summary["ledgerSha256"],
        "summarySha256": digest(summary_bytes),
        "oldOutcome": LANES[lane]["oldOutcome"],
        "newOutcome": "expected",
        "outcomes": summary["outcomes"],
    }
    return ledger, summary_bytes, receipt


def prepare_pair() -> dict[str, bytes]:
    check_prior_bundle()
    check_harness()
    prepared = {lane: prepare_lane(lane) for lane in LANES}
    receipt = {
        "schema": 1,
        "scope": SCOPE,
        "candidateRuntimeAttested": True,
        "publicApiParityEvidence": False,
        "sourceClassificationPreserved": "implementation-internal",
        "promoterSourceSha256": sha256(Path(__file__).resolve()),
        "strictVerifierSha256": HARNESS_PINS[str(VERIFIER.relative_to(ROOT))],
        "harnessInputs": HARNESS_PINS,
        "priorBundle": str(BASE),
        "priorBundleReceiptSha256": BASE_RECEIPT_SHA256,
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "recordsPerLane": COUNT,
        "targetZeroBasedIndexes": sorted(TARGETS),
        "targetSpecIds": [TARGETS[index][1] for index in sorted(TARGETS)],
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
        flag = 0x4
        at_fdcwd = -2
    elif sys.platform.startswith("linux"):
        rename = libc.renameat2
        flag = 0x1
        at_fdcwd = -100
    else:
        raise ValueError("atomic no-replace directory publication is unavailable")
    rename.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p, ctypes.c_uint]
    rename.restype = ctypes.c_int
    result = rename(at_fdcwd, os.fsencode(staging), at_fdcwd, os.fsencode(destination), flag)
    if result != 0:
        raise ValueError(f"atomic no-replace publication failed: {os.strerror(ctypes.get_errno())}")


def publish(files: dict[str, bytes]) -> None:
    require(not OUTPUT.exists() and not OUTPUT.is_symlink(), "immutable output already exists")
    temporary = Path(tempfile.mkdtemp(prefix=".core-cdp-error-slice-v3-", dir=OUTCOMES))
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
        parser.error(f"strict paired Core CDP error v3 promotion rejected: {error}")
    print(json.dumps({"scope": receipt["scope"], "lanes": receipt["lanes"]}, indent=2))


if __name__ == "__main__":
    main()
