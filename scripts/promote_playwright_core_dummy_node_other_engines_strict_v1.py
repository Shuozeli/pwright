#!/usr/bin/env python3
"""Publish an immutable paired Core ledger for the Firefox/WebKit dummy launch RED.

The candidate runs the real public launch method but fails the unchanged official
assertion. The oracle passes. This is discrepancy evidence, not parity credit or
an attestation that the candidate entered its native runtime.
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

ROOT = Path(__file__).resolve().parents[1]
OUTCOMES = ROOT / ".work/official-conformance/outcomes"
BASE = OUTCOMES / "core-upstream-skips-strict-paired-v1"
OUTPUT = OUTCOMES / "core-dummy-node-other-engines-strict-paired-v1"
RESULTS = (
    ROOT / ".work/official-conformance/results/core-dummy-node-other-engines-strict-v1"
)
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
MANIFEST = (
    ROOT
    / "tests/official/playwright-core-dummy-node-other-engines-strict-v1-manifest.json"
)
VERIFIER = ROOT / "scripts/verify_playwright_core_dummy_node_other_engines_strict_v1.py"
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
MANIFEST_SHA256 = "6ac83fa5a81dd7806c2f4737d61684b8aad414b2ea006f72e83bcdbc937fa6b0"
VERIFIER_SHA256 = "01bcf055ef6a3912c6d5d4a49219567d19f48c4a329f30904a2e7065e6518fec"
BASE_RECEIPT_SHA256 = "d2e85289725b8889666174680f16b0b6f83c12e359593454bc956a7fef139c9e"
BASE_LEDGER_SHA256 = {
    "candidate": "c88d637642c711f40334e785e95cf9d0874fa68a4b0a189250b0d5d6cab10694",
    "oracle": "553b8ac282d2bb13e71bd3c1cd9c02ff67940e06ab58e25eb0f54b75551ab82e",
}
BASE_SUMMARY_SHA256 = {
    "candidate": "d0026f09f5bb5e5d344deb122f7d1ebc3929e2cb18773d555727f39ef1bc04a7",
    "oracle": "98305670b0c751bf69811ea08a2f9707ff697c415213953ecaff77c393d02acb",
}
SIDECAR_SHA256 = {
    (
        "firefox",
        "candidate",
    ): "ab3b08d7acb5a04006fd2c63e7c3e65ad2256da6c48f61246d6058967988518a",
    (
        "firefox",
        "oracle",
    ): "5e93392b874731d96f0579dc447b2e5987689a058b34f4c783d97f87a715542d",
    (
        "webkit",
        "candidate",
    ): "73f09b2d0c5f0cb763155f5a9fa503fa21e442a3708ee386f7cef59dfc56effa",
    (
        "webkit",
        "oracle",
    ): "464473a72ea0d236e2d83235c70f8e6a5132f7c5d43a616b1aa5abf60e0e4c9a",
}
SPEC_IDS = {
    "firefox": "d1a604bdf6287c6776f3-d8df50eebf56e23fa8d3",
    "webkit": "d1a604bdf6287c6776f3-24ac4e8612e7cc735325",
}
TARGET_INDEX = {"firefox": 31_614, "webkit": 31_615}
LANES = ("candidate", "oracle")
ENGINES = ("firefox", "webkit")
COUNT = 48_477
SCOPE = "official-browser-free-browsertype-dummy-node-other-engines-strict-red-v1"
SELECTOR = "tests/library/browsertype-launch.spec.ts:64"
SOURCE_FILE = "tests/library/browsertype-launch.spec.ts"
SPEC_FILE = "library/browsertype-launch.spec.ts"
TITLE = "should reject if launched browser fails immediately"
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
SOURCE_SHA256 = "64fe24188ec96bb0d5d1ecca5af37986b16d82451b91e4db80593f707126bd05"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha_file(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def sealed(path: Path, expected: str, label: str) -> None:
    require(
        len(expected) == 64 and all(c in "0123456789abcdef" for c in expected),
        f"invalid SHA-256 pin for {label}",
    )
    require(path.is_file() and not path.is_symlink(), f"missing or linked {label}")
    require(sha_file(path) == expected, f"{label} SHA-256 drifted")


def object_json(data: bytes, label: str) -> dict[str, Any]:
    value = json.loads(data)
    require(isinstance(value, dict), f"{label} is not a JSON object")
    return value


def sidecar_path(engine: str, lane: str) -> Path:
    require(engine in ENGINES and lane in LANES, "unknown evidence lane")
    return RESULTS / f"oct20-{lane}-{engine}-v2.strict.json"


def check_prior_bundle() -> dict[str, dict[str, Any]]:
    sealed(INVENTORY, INVENTORY_SHA256, "Core inventory")
    sealed(BASE / "promotion-receipt.json", BASE_RECEIPT_SHA256, "predecessor receipt")
    receipt = object_json(
        (BASE / "promotion-receipt.json").read_bytes(), "predecessor receipt"
    )
    require(
        receipt.get("schema") == 1
        and receipt.get("scope") == "official-upstream-skip-accounting-only-strict-v1"
        and receipt.get("promotionKind") == "skip-accounting-only"
        and receipt.get("recordsPerLane") == COUNT
        and receipt.get("inventory") == str(INVENTORY)
        and receipt.get("inventorySha256") == INVENTORY_SHA256
        and set(receipt.get("lanes", {})) == set(LANES),
        "predecessor receipt identity drifted",
    )
    summaries: dict[str, dict[str, Any]] = {}
    for lane in LANES:
        ledger = BASE / f"{lane}.jsonl"
        summary_path = BASE / f"{lane}.summary.json"
        sealed(ledger, BASE_LEDGER_SHA256[lane], f"{lane} predecessor ledger")
        sealed(summary_path, BASE_SUMMARY_SHA256[lane], f"{lane} predecessor summary")
        summary = object_json(summary_path.read_bytes(), f"{lane} predecessor summary")
        lane_receipt = receipt["lanes"][lane]
        require(
            summary.get("schema") == 1
            and summary.get("inventory") == str(INVENTORY)
            and summary.get("records") == COUNT
            and summary.get("ledger") == str(ledger)
            and summary.get("ledgerSha256") == BASE_LEDGER_SHA256[lane]
            and isinstance(summary.get("outcomes"), dict)
            and isinstance(summary.get("explicitEvidenceScopes"), dict)
            and lane_receipt.get("ledger") == str(ledger)
            and lane_receipt.get("ledgerSha256") == BASE_LEDGER_SHA256[lane]
            and lane_receipt.get("summarySha256") == BASE_SUMMARY_SHA256[lane]
            and lane_receipt.get("outcomes") == summary["outcomes"],
            f"{lane} predecessor ledger/summary/receipt drifted",
        )
        summaries[lane] = summary
    return summaries


def check_evidence() -> dict[tuple[str, str], dict[str, Any]]:
    sealed(MANIFEST, MANIFEST_SHA256, "strict manifest")
    sealed(VERIFIER, VERIFIER_SHA256, "independent strict verifier")
    manifest = object_json(MANIFEST.read_bytes(), "strict manifest")
    require(
        manifest.get("schema") == 2
        and manifest.get("commit") == COMMIT
        and manifest.get("selector") == SELECTOR
        and manifest.get("title") == TITLE
        and manifest.get("upstreamSources", {}).get(SOURCE_FILE) == SOURCE_SHA256
        and manifest.get("cases")
        == [
            {
                "engine": engine,
                "project": f"{engine}-library",
                "suite": f"library-browsertype-launch-dummy-node-failure-{engine}",
                "specId": SPEC_IDS[engine],
            }
            for engine in ENGINES
        ],
        "strict manifest case selection drifted",
    )
    paths = [sidecar_path(engine, lane) for engine in ENGINES for lane in LANES]
    for engine in ENGINES:
        for lane in LANES:
            sealed(
                sidecar_path(engine, lane),
                SIDECAR_SHA256[(engine, lane)],
                f"{engine} {lane} strict sidecar",
            )
    from verify_playwright_core_dummy_node_other_engines_strict_v1 import verify_all

    evidence = verify_all(paths)
    require(
        set(evidence) == set(SIDECAR_SHA256),
        "independent verifier returned incomplete pair",
    )
    run_ids: set[str] = set()
    result_paths: set[str] = set()
    for engine in ENGINES:
        for lane in LANES:
            strict = evidence[(engine, lane)]
            require(
                strict.get("scope") == "core-dummy-node-other-engines-strict-v1"
                and strict.get("engine") == engine
                and strict.get("mode") == lane
                and strict.get("specId") == SPEC_IDS[engine]
                and strict.get("selector") == SELECTOR
                and strict.get("upstreamCommit") == COMMIT
                and strict.get("manifestSha256") == MANIFEST_SHA256
                and strict.get("upstreamSources") == manifest["upstreamSources"]
                and strict.get("testStatus")
                == ("expected" if lane == "oracle" else "unexpected")
                and strict.get("attemptStatus")
                == ("passed" if lane == "oracle" else "failed")
                and strict.get("assertionsExecuted") is True
                and strict.get("upstreamAssertionsModified") is False
                and strict.get("fixtureAdapted") is True
                and strict.get("candidateNativeRuntimeInvoked") is False
                and strict.get("publicApiParityEvidence") is False
                and strict.get("diagnosticOnly") is False,
                f"{engine} {lane} is not the pinned official RED/green evidence",
            )
            run_ids.add(strict["runId"])
            result_paths.add(strict["result"])
    require(
        len(run_ids) == len(result_paths) == 4,
        "strict evidence reused a run ID or official result",
    )
    return evidence


def target_row(source: dict[str, Any], row: dict[str, Any], engine: str) -> None:
    require(
        source.get("specId") == SPEC_IDS[engine]
        and source.get("projectId") == source.get("projectName") == f"{engine}-library"
        and source.get("sourceFile") == SOURCE_FILE
        and source.get("file") == SPEC_FILE
        and source.get("line") == 64
        and source.get("column") == 3
        and source.get("titlePath") == [SPEC_FILE, TITLE]
        and source.get("sourceClassification") == "public-api-candidate"
        and source.get("classificationReasons") == []
        and source.get("role") == "official-entrypoint"
        and source.get("suite") == "library"
        and source.get("expectedStatus") == "passed"
        and row == {**source, "attempts": 0, "outcome": "not-run"},
        f"{engine} target is not the exact unrun public API row",
    )


def transform_lane(
    lane: str,
    evidence: dict[tuple[str, str], dict[str, Any]],
    prior: dict[str, Any],
) -> tuple[bytes, dict[str, int], dict[str, int]]:
    require(lane in LANES, "unknown ledger lane")
    before: Counter[str] = Counter()
    after: Counter[str] = Counter()
    before_scopes: Counter[str] = Counter()
    after_scopes: Counter[str] = Counter()
    seen: set[str] = set()
    output = bytearray()
    rows = 0
    with INVENTORY.open("rb") as inventory, (BASE / f"{lane}.jsonl").open("rb") as base:
        for index, (inventory_line, line) in enumerate(zip_longest(inventory, base)):
            require(
                inventory_line is not None and line is not None,
                f"{lane} row count differs from inventory",
            )
            require(
                inventory_line.endswith(b"\n") and line.endswith(b"\n"),
                f"{lane} row {index} is unterminated",
            )
            source = object_json(inventory_line, f"inventory row {index}")
            row = object_json(line, f"{lane} predecessor row {index}")
            require(
                all(row.get(key) == value for key, value in source.items()),
                f"{lane} row {index} inventory metadata differs",
            )
            require(
                row.get("outcome") in {"expected", "unexpected", "skipped", "not-run"}
                and type(row.get("attempts")) is int
                and row["attempts"] >= 0,
                f"{lane} row {index} outcome/attempts invalid",
            )
            before[row["outcome"]] += 1
            if "evidenceScope" in row:
                require(isinstance(row["evidenceScope"], str), "invalid evidence scope")
                before_scopes[row["evidenceScope"]] += 1
            engine = next(
                (name for name, target in TARGET_INDEX.items() if index == target), None
            )
            if engine is None:
                require(
                    row.get("specId") not in SPEC_IDS.values(),
                    f"{lane} duplicate target spec ID at row {index}",
                )
                output.extend(line)
                after[row["outcome"]] += 1
                if "evidenceScope" in row:
                    after_scopes[row["evidenceScope"]] += 1
            else:
                require(engine not in seen, f"{lane} target repeated")
                seen.add(engine)
                target_row(source, row, engine)
                promoted = {
                    **row,
                    "attempts": 1,
                    "outcome": "unexpected" if lane == "candidate" else "expected",
                    "resultFile": evidence[(engine, lane)]["result"],
                    "evidenceScope": SCOPE,
                    "candidateRuntimeRequired": False,
                    "candidateNativeRuntimeInvoked": False,
                    "publicApiParityEvidence": False,
                }
                output.extend(
                    (
                        json.dumps(promoted, sort_keys=True, separators=(",", ":"))
                        + "\n"
                    ).encode()
                )
                after[promoted["outcome"]] += 1
                after_scopes[SCOPE] += 1
            rows += 1
    require(rows == COUNT and seen == set(ENGINES), f"{lane} target count differs")
    require(
        dict(before) == prior["outcomes"]
        and dict(before_scopes) == prior["explicitEvidenceScopes"],
        f"{lane} predecessor summary differs from rows",
    )
    expected_outcomes = before.copy()
    expected_outcomes["not-run"] -= 2
    expected_outcomes["unexpected" if lane == "candidate" else "expected"] += 2
    expected_scopes = before_scopes.copy()
    expected_scopes[SCOPE] += 2
    require(
        before["not-run"] >= 2
        and after == expected_outcomes
        and after_scopes == expected_scopes,
        f"{lane} successor changed more than two rows/scopes",
    )
    return (
        bytes(output),
        dict(sorted(after.items())),
        dict(sorted(after_scopes.items())),
    )


def prepare_pair() -> dict[str, bytes]:
    prior = check_prior_bundle()
    evidence = check_evidence()
    files: dict[str, bytes] = {}
    lane_receipts: dict[str, dict[str, Any]] = {}
    for lane in LANES:
        ledger, outcomes, scopes = transform_lane(lane, evidence, prior[lane])
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
        files[f"{lane}.jsonl"] = ledger
        files[f"{lane}.summary.json"] = summary_bytes
        lane_receipts[lane] = {
            "base": str(BASE / f"{lane}.jsonl"),
            "baseSha256": BASE_LEDGER_SHA256[lane],
            "ledger": str(destination),
            "ledgerSha256": digest(ledger),
            "summarySha256": digest(summary_bytes),
            "oldOutcome": "not-run",
            "newOutcome": "unexpected" if lane == "candidate" else "expected",
            "promotedRows": 2,
            "candidateRuntimeAttested": False,
            "publicApiParityEvidence": False,
            "outcomes": outcomes,
            "evidence": {
                engine: {
                    "strictSidecar": str(sidecar_path(engine, lane)),
                    "strictSidecarSha256": SIDECAR_SHA256[(engine, lane)],
                    "runId": evidence[(engine, lane)]["runId"],
                    "officialResult": evidence[(engine, lane)]["result"],
                    "officialResultSha256": evidence[(engine, lane)]["resultSha256"],
                    "testStatus": evidence[(engine, lane)]["testStatus"],
                }
                for engine in ENGINES
            },
        }
    receipt = {
        "schema": 1,
        "scope": SCOPE,
        "promotionKind": "public-api-official-red",
        "publicApiBehaviorAttested": True,
        "publicApiParityEvidence": False,
        "candidateRuntimeAttested": False,
        "candidateNativeRuntimeInvoked": False,
        "diagnosticOnly": False,
        "assertionsExecuted": True,
        "upstreamAssertionsModified": False,
        "fixtureAdapted": True,
        "promoterSourceSha256": sha_file(Path(__file__)),
        "strictVerifierSha256": VERIFIER_SHA256,
        "priorBundle": str(BASE),
        "priorBundleReceiptSha256": BASE_RECEIPT_SHA256,
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "recordsPerLane": COUNT,
        "promotedRowsPerLane": 2,
        "targetZeroBasedIndexes": [TARGET_INDEX[engine] for engine in ENGINES],
        "targetSpecIds": [SPEC_IDS[engine] for engine in ENGINES],
        "upstreamCommit": COMMIT,
        "selectedUpstreamSpecSha256": SOURCE_SHA256,
        "manifestSha256": MANIFEST_SHA256,
        "lanes": lane_receipts,
    }
    files["promotion-receipt.json"] = (
        json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    ).encode()
    return files


def verify_existing() -> dict[str, Any]:
    require(OUTPUT.is_dir() and not OUTPUT.is_symlink(), "immutable successor missing")
    expected = prepare_pair()
    require(
        {path.name for path in OUTPUT.iterdir()} == set(expected),
        "immutable successor file set differs",
    )
    for name, payload in expected.items():
        path = OUTPUT / name
        require(
            path.is_file() and not path.is_symlink() and path.read_bytes() == payload,
            f"immutable successor drifted: {name}",
        )
    return object_json(expected["promotion-receipt.json"], "successor receipt")


def publish_exclusive(staging: Path, destination: Path) -> None:
    libc = ctypes.CDLL(None, use_errno=True)
    if sys.platform == "darwin":
        rename = libc.renameatx_np
        flag, at_fdcwd = 0x4, -2
    elif sys.platform.startswith("linux"):
        rename = libc.renameat2
        flag, at_fdcwd = 0x1, -100
    else:
        raise ValueError("atomic no-replace publication unavailable")
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
    require(not OUTPUT.exists() and not OUTPUT.is_symlink(), "successor already exists")
    OUTCOMES.mkdir(parents=True, exist_ok=True)
    staging = Path(
        tempfile.mkdtemp(prefix=".core-dummy-other-engines-strict-v1-", dir=OUTCOMES)
    )
    try:
        for name, payload in files.items():
            with (staging / name).open("xb") as stream:
                stream.write(payload)
                stream.flush()
                os.fsync(stream.fileno())
        directory_fd = os.open(staging, os.O_RDONLY)
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)
        publish_exclusive(staging, OUTPUT)
        parent_fd = os.open(OUTCOMES, os.O_RDONLY)
        try:
            os.fsync(parent_fd)
        finally:
            os.close(parent_fd)
    finally:
        if staging.exists():
            shutil.rmtree(staging)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify-existing", action="store_true")
    parser.add_argument("--preflight", action="store_true")
    args = parser.parse_args()
    try:
        require(
            not (args.verify_existing and args.preflight),
            "choose only one read-only mode",
        )
        if args.verify_existing:
            receipt = verify_existing()
        elif args.preflight:
            require(not OUTPUT.exists(), "successor already exists")
            files = prepare_pair()
            receipt = object_json(files["promotion-receipt.json"], "preflight receipt")
        else:
            files = prepare_pair()
            require(files == prepare_pair(), "evidence changed before publication")
            publish(files)
            receipt = verify_existing()
    except (
        OSError,
        ValueError,
        KeyError,
        TypeError,
        IndexError,
        json.JSONDecodeError,
    ) as error:
        parser.exit(2, f"strict paired Core two-engine promotion rejected: {error}\n")
    print(json.dumps({"scope": receipt["scope"], "lanes": receipt["lanes"]}, indent=2))


if __name__ == "__main__":
    main()
