"""Atomically seal three genuine official CDP error outcomes in each Core lane.

This slice exercises implementation-internal tests. Its candidate failures are
kept as unexpected; only the independent oracle's three passes are expected.
No non-target ledger byte or inventory classification may change.
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

import run_playwright_core_cdp_error_slice_strict_v1 as source
import run_playwright_core_cdp_error_slice_strict_v2 as capture
from verify_playwright_core_cdp_error_slice_strict_v2 import verify_strict

ROOT = source.ROOT
OUTCOMES = ROOT / ".work/official-conformance/outcomes"
BASE = OUTCOMES / "core-cdp-default-user-agent-strict-paired-v2"
OUTPUT = OUTCOMES / "core-cdp-error-slice-strict-paired-v2"
BASE_RECEIPT_SHA256 = "1fccc683abb6a19505585c032449dab260b85f18312fd55f6ac6027e253b30cd"
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
COUNT = 48_477
SCOPE = "official-browser-free-cdp-error-slice-worker-restart-aware-strict-v2"
SOURCE_FILE = source.SOURCE
CLASSIFICATION_REASONS = [
    "private-client-access",
    "runtime-source-import:packages/playwright-core/lib/server/utils/userAgent",
]
TARGETS = {
    31_787: (
        399,
        "8d42d2a6326b0b57ff3d-de38f5af50cb42c10550",
        "should report an expected error when the endpointURL returns a non-expected status code",
    ),
    31_788: (
        411,
        "8d42d2a6326b0b57ff3d-858b23a33867c47c6bb2",
        "should report an expected error when the endpoint URL JSON "
        "webSocketDebuggerUrl is undefined",
    ),
    31_796: (
        572,
        "8d42d2a6326b0b57ff3d-2a0f5cff9ee48ad78688",
        "should print custom ws close error",
    ),
}
BASE_SHA256 = {
    "candidate": "b69852a0f582958f563cecdf055bda37373a3e222378fb4efdb1cea98ac79824",
    "oracle": "4198bf312aea62aada3e5f99f0eeb2daca13c980657d01fb81c1141a915b08a8",
}
HARNESS_PINS = {
    "tests/official/playwright-core-cdp-error-slice-strict-v1-manifest.json": (
        "a59502654e1d3b30c32cd4d5d1e52b82ace071eb28e9524415d01b0c9e099a66"
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
}
VERIFIER = ROOT / "scripts/verify_playwright_core_cdp_error_slice_strict_v2.py"
VERIFIER_SHA256 = "d059b55943d5a4506f5499e48d4ea9bd06eee40a244a84b620c394e4a08bb7a0"
LANES = {
    lane: {
        "strict": capture.RESULT_PARENT / f"{lane}-v1.strict.json",
        "strictSha256": {
            "candidate": "7a06364c0ee12f7a319aa5e5055246a5b74a83ed844a15476abbaa42060c4ac0",
            "oracle": "64acdef8e78b040361c1a7567c4ea997ae09f9f14c8a8591aa4a13c1e7b41db3",
        }[lane],
        "outcome": "unexpected" if lane == "candidate" else "expected",
        "exitCode": 1 if lane == "candidate" else 0,
    }
    for lane in ("candidate", "oracle")
}
IDENTITY_FIELDS = (
    "specId",
    "projectId",
    "projectName",
    "sourceClassification",
    "classificationReasons",
    "sourceFile",
    "file",
    "line",
    "column",
    "expectedStatus",
    "role",
    "suite",
    "titlePath",
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
        len(expected) == 64 and all(character in "0123456789abcdef" for character in expected),
        f"unfilled or invalid SHA-256 pin: {path}",
    )
    require(path.is_file() and not path.is_symlink(), f"missing or linked evidence: {path}")
    require(sha256(path) == expected, f"SHA-256 drift: {path}")


def check_prior_bundle() -> dict[str, Any]:
    sealed(INVENTORY, INVENTORY_SHA256)
    sealed(BASE / "promotion-receipt.json", BASE_RECEIPT_SHA256)
    prior = json.loads((BASE / "promotion-receipt.json").read_text())
    require(
        prior["inventory"] == str(INVENTORY)
        and prior["inventorySha256"] == INVENTORY_SHA256
        and prior["recordsPerLane"] == COUNT,
        "prior inventory receipt drifted",
    )
    for lane, expected in BASE_SHA256.items():
        ledger = BASE / f"{lane}.jsonl"
        sealed(ledger, expected)
        require(
            prior["lanes"][lane]["ledger"] == str(ledger)
            and prior["lanes"][lane]["ledgerSha256"] == expected,
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
    sealed(VERIFIER, VERIFIER_SHA256)
    for relative, expected in HARNESS_PINS.items():
        sealed(ROOT / relative, expected)
    source.check_manifest()


def official_results(lane: str) -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    config = LANES[lane]
    strict_path = config["strict"]
    sealed(strict_path, config["strictSha256"])
    strict = verify_strict(strict_path)
    require(
        strict["mode"] == lane
        and strict["scope"] == capture.SCOPE
        and strict["upstreamCommit"] == source.COMMIT
        and strict["upstreamSourceSha256"] == source.SOURCE_SHA256
        and strict["serverFixtureSha256"] == source.SERVER_FIXTURE_SHA256
        and strict["commonPreloadSha256"] == source.COMMON_PRELOAD_SHA256
        and strict["inputs"] == HARNESS_PINS
        and strict["exitCode"] == config["exitCode"],
        f"{lane} strict provenance drifted",
    )
    require(
        strict["candidateProfile"] == (source.PROFILE if lane == "candidate" else None)
        and strict["candidateArtifactSha256"]
        == (source.ARTIFACT_SHA256 if lane == "candidate" else None),
        f"{lane} runtime artifact provenance drifted",
    )
    result = Path(strict["result"])
    require(
        result.is_absolute()
        and result.with_suffix(".strict.json") == strict_path
        and result.is_file()
        and not result.is_symlink()
        and sha256(result) == strict["resultSha256"],
        f"{lane} official result identity drifted",
    )
    selected = capture.selected_tests(json.loads(result.read_text()))
    require(strict["selected"] == selected and len(selected) == 3, f"{lane} selected set drifted")
    by_id = {item["specId"]: item for item in selected}
    require(len(by_id) == 3, f"{lane} selected duplicate spec IDs")
    for _, spec_id, _ in TARGETS.values():
        item = by_id[spec_id]
        require(
            item["status"] == config["outcome"]
            and len(item["attempts"]) == 1
            and item["attempts"][0]["retry"] == 0
            and item["attempts"][0]["status"] == ("failed" if lane == "candidate" else "passed"),
            f"{lane} official CDP assertion outcome drifted: {spec_id}",
        )
        require(
            all(isinstance(error, str) and error for error in item["errors"])
            and bool(item["errors"]) == (lane == "candidate"),
            f"{lane} official CDP assertion error receipt drifted: {spec_id}",
        )
    return strict, by_id


def check_target(record: dict[str, Any], inventory_row: dict[str, Any], index: int) -> None:
    line, spec_id, title = TARGETS[index]
    require(
        record == {**inventory_row, "attempts": 0, "outcome": "not-run"},
        f"target {index} is not a pristine baseline row",
    )
    require(
        record["specId"] == spec_id
        and record["projectId"] == record["projectName"] == "chromium-library"
        and record["sourceFile"] == SOURCE_FILE
        and record["file"] == SOURCE_FILE.removeprefix("tests/")
        and record["line"] == line
        and record["column"] == 5
        and record["titlePath"] == [SOURCE_FILE.removeprefix("tests/"), title]
        and record["expectedStatus"] == "passed"
        and record["role"] == "official-entrypoint"
        and record["suite"] == "library"
        and record["sourceClassification"] == "implementation-internal"
        and record["classificationReasons"] == CLASSIFICATION_REASONS,
        f"target {index} identity or classification drifted",
    )


def prepare_lane(lane: str) -> tuple[bytes, bytes, dict[str, Any]]:
    require(lane in LANES, f"unknown lane: {lane}")
    strict, cases = official_results(lane)
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
                check_target(record, inventory_row, index)
                item = cases[record["specId"]]
                promoted = {
                    **record,
                    "attempts": 1,
                    "outcome": LANES[lane]["outcome"],
                    "resultFile": strict["result"],
                    "evidenceScope": SCOPE,
                    "candidateRuntimeRequired": lane == "candidate",
                }
                if lane == "candidate":
                    promoted["error"] = "\n".join(item["errors"])
                require(
                    all(promoted[key] == record[key] for key in IDENTITY_FIELDS),
                    "target promotion changed inventory identity/classification",
                )
                output.extend(
                    (json.dumps(promoted, sort_keys=True, separators=(",", ":")) + "\n").encode()
                )
                after[promoted["outcome"]] += 1
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
    expected["not-run"] -= 3
    expected[LANES[lane]["outcome"]] += 3
    scopes = before_scopes.copy()
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
        "candidateArtifactSha256": source.ARTIFACT_SHA256 if lane == "candidate" else None,
        "ledger": str(destination),
        "ledgerSha256": summary["ledgerSha256"],
        "summarySha256": digest(summary_bytes),
        "oldOutcome": "not-run",
        "newOutcome": LANES[lane]["outcome"],
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
        "strictVerifierSha256": VERIFIER_SHA256,
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
    temporary = Path(tempfile.mkdtemp(prefix=".core-cdp-error-slice-v2-", dir=OUTCOMES))
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
        parser.error(f"strict paired Core CDP error promotion rejected: {error}")
    print(json.dumps({"scope": receipt["scope"], "lanes": receipt["lanes"]}, indent=2))


if __name__ == "__main__":
    main()
