#!/usr/bin/env python3
"""Seal one browser-free, fixture-adapted Core version-mismatch diagnostic.

The unchanged official assertion passed once in each lane. This internal
server-fixture result does not establish public API or browser parity.
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

from promote_playwright_core_server_connect_ws_error_other_engines_strict_v3 import (
    verify_existing as verify_predecessor,
)
from run_playwright_core_runserver_url_owner_v3 import verify_envelope

ROOT = Path(__file__).resolve().parents[1]
OUTCOMES = ROOT / ".work/official-conformance/outcomes"
PREDECESSOR = OUTCOMES / "core-server-connect-ws-error-other-engines-strict-paired-v3"
OUTPUT = OUTCOMES / "core-server-connect-version-mismatch-url-owner-strict-paired-v3"
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
RESULTS = (
    ROOT
    / ".work/official-conformance/results"
    / "core-server-connect-version-mismatch-url-owner-v3/strict-pair-20261020-v3"
)
SCOPE = "official-browser-free-server-version-mismatch-internal-diagnostic-green-v3"
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
PREDECESSOR_RECEIPT_SHA256 = (
    "59c7cbb11ad7284ab8aedadb8c850cd4552ffaa0b035b70430c8b4e2e4ef0ef7"
)
PREDECESSOR_LEDGER_SHA256 = {
    "candidate": "e7d5924d9face0aaac68f5bf1db35516cb6d687871aa34c55361b961e69aee2d",
    "oracle": "3db929fbe661c35aee7f05c9ea4ae3353a0fc4dd249550a1ea43f918f1de8865",
}
PREDECESSOR_SUMMARY_SHA256 = {
    "candidate": "296b32931c67822a2b205db6b81b4200ff4f416fa57b5aade22e4a981a6ff474",
    "oracle": "3c6fd4b568c23776f76790156403ecac47503b31e2811139edb4c72919a2e4ae",
}
ENVELOPE_SHA256 = {
    "candidate": "11ea37b4233f799f73f02ff6e7a0836f0635aadd7dd7af8b42ed9f3b7f998f8b",
    "oracle": "54641e1250d83dcd657f9657b56549ecba80ffcfdf458d01863ead6f11ace230",
}
SUCCESSOR_SHA256 = {
    "candidate": "e8485c418287054c78c2b1331d2cd258103d556fb0aca2bf988a5ac738f17e04",
    "oracle": "6994207de60e01ad2e36dc296fe20922e39ab2483d574d02b77d42e18bb2fa3c",
}
NEW_ROW_SHA256 = {
    "candidate": "f0f5f77fcaa91a8e205681a38fd3b2c4b9ac13c32d8e61fb2a05e3eb47f58ee6",
    "oracle": "e61164d0548840a8fce13f76208c6c468a9d94078b98c27652bc10ad033f242f",
}
OLD_ROW_SHA256 = "6a21f2459feb6258cf0ec4e8ca7b445896544a2aac77d9f747f17b504375d8bc"
TARGET_INDEX = 31_532
COUNT = 48_477
TARGET_ID = "ee6df4b3a2217bd356a7-d60bdc61fb009792457d"
PAIRED_VERIFIER = (
    ROOT / "scripts/verify_playwright_core_server_connect_version_mismatch_paired_v3.py"
)
PAIRED_VERIFIER_SHA256 = (
    "1f12fe4302358f9e472e03cace4cb7d177274cfef0321184afb5bbcf8afbf182"
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def sealed(path: Path, expected: str, label: str) -> None:
    require(path.is_file() and not path.is_symlink(), f"missing or linked {label}")
    require(sha256(path) == expected, f"{label} SHA-256 drifted")


def visit_specs(suite: dict[str, Any], results: list[dict[str, Any]]) -> None:
    results.extend(suite.get("specs", []))
    for child in suite.get("suites", []):
        visit_specs(child, results)


def official_evidence(lane: str) -> dict[str, Any]:
    path = RESULTS / f"{lane}.runserver-url-owner-v3-attestation.json"
    sealed(path, ENVELOPE_SHA256[lane], f"{lane} v3 envelope")
    envelope = verify_envelope(path)
    require(
        envelope["mode"] == lane
        and envelope["specId"] == TARGET_ID
        and envelope["status"] == "expected"
        and envelope["runnerExitCode"] == 0,
        f"{lane} official assertion did not pass",
    )
    report = json.loads(Path(envelope["result"]).read_text())
    specs: list[dict[str, Any]] = []
    for suite in report["suites"]:
        visit_specs(suite, specs)
    require(
        len(specs) == 1 and len(specs[0].get("tests", [])) == 1,
        f"{lane} official result selected another test",
    )
    spec = specs[0]
    test = spec["tests"][0]
    require(
        spec["id"] == TARGET_ID
        and spec["line"] == 1046
        and spec["ok"] is True
        and test["projectId"] == "chromium-library"
        and test["expectedStatus"] == "passed"
        and test["status"] == "expected"
        and len(test["results"]) == 1
        and test["results"][0]["retry"] == 0
        and test["results"][0]["status"] == "passed"
        and test["results"][0]["errors"] == [],
        f"{lane} official result was not one clean pass",
    )
    return envelope


def check_inputs() -> dict[str, dict[str, Any]]:
    sealed(INVENTORY, INVENTORY_SHA256, "Core source inventory")
    sealed(
        PREDECESSOR / "promotion-receipt.json",
        PREDECESSOR_RECEIPT_SHA256,
        "predecessor receipt",
    )
    previous = verify_predecessor()
    require(
        previous["sourceClassificationPreserved"] == "implementation-internal"
        and previous["fixtureAdapted"] is True
        and previous["publicApiParityEvidence"] is False,
        "predecessor scope drifted",
    )
    for lane in ("candidate", "oracle"):
        sealed(
            PREDECESSOR / f"{lane}.jsonl",
            PREDECESSOR_LEDGER_SHA256[lane],
            f"{lane} predecessor ledger",
        )
        sealed(
            PREDECESSOR / f"{lane}.summary.json",
            PREDECESSOR_SUMMARY_SHA256[lane],
            f"{lane} predecessor summary",
        )
    sealed(PAIRED_VERIFIER, PAIRED_VERIFIER_SHA256, "independent paired verifier")
    evidence = {lane: official_evidence(lane) for lane in ("candidate", "oracle")}
    require(
        evidence["candidate"]["harnessInputs"] == evidence["oracle"]["harnessInputs"],
        "paired v3 harness inputs differ",
    )
    return evidence


def transform(
    lane: str, result: str, previous: dict[str, Any]
) -> tuple[bytes, dict[str, int], dict[str, int]]:
    output = bytearray()
    before: Counter[str] = Counter()
    after: Counter[str] = Counter()
    before_scopes: Counter[str] = Counter()
    after_scopes: Counter[str] = Counter()
    rows = 0
    with (
        INVENTORY.open("rb") as inventory,
        (PREDECESSOR / f"{lane}.jsonl").open("rb") as predecessor,
    ):
        for index, (source_raw, raw) in enumerate(zip_longest(inventory, predecessor)):
            require(
                source_raw is not None
                and raw is not None
                and source_raw.endswith(b"\n")
                and raw.endswith(b"\n"),
                f"{lane} inventory/ledger row count or termination drifted at {index}",
            )
            source = json.loads(source_raw)
            row = json.loads(raw)
            require(
                isinstance(source, dict)
                and isinstance(row, dict)
                and all(row.get(key) == value for key, value in source.items()),
                f"{lane} inventory identity/classification drifted at {index}",
            )
            before[row["outcome"]] += 1
            if "evidenceScope" in row:
                before_scopes[row["evidenceScope"]] += 1
            if index == TARGET_INDEX:
                require(
                    digest(raw) == OLD_ROW_SHA256
                    and row == {**source, "attempts": 0, "outcome": "not-run"}
                    and row["specId"] == TARGET_ID
                    and row["projectId"] == row["projectName"] == "chromium-library"
                    and row["file"] == "library/browsertype-connect.spec.ts"
                    and row["sourceFile"] == "tests/library/browsertype-connect.spec.ts"
                    and row["line"] == 1046
                    and row["suite"] == "library"
                    and row["sourceClassification"] == "implementation-internal"
                    and row["role"] == "official-entrypoint"
                    and row["titlePath"]
                    == [
                        "library/browsertype-connect.spec.ts",
                        "should refuse connecting when versions do not match",
                    ],
                    f"{lane} target row identity drifted",
                )
                new_row = {
                    **row,
                    "attempts": 1,
                    "candidateRuntimeRequired": lane == "candidate",
                    "evidenceScope": SCOPE,
                    "outcome": "expected",
                    "resultFile": result,
                }
                encoded = (
                    json.dumps(new_row, sort_keys=True, separators=(",", ":")) + "\n"
                ).encode()
                require(
                    digest(encoded) == NEW_ROW_SHA256[lane], "target row SHA drifted"
                )
                output.extend(encoded)
                after["expected"] += 1
                after_scopes[SCOPE] += 1
            else:
                require(row["specId"] != TARGET_ID, f"{lane} target moved/duplicated")
                output.extend(raw)
                after[row["outcome"]] += 1
                if "evidenceScope" in row:
                    after_scopes[row["evidenceScope"]] += 1
            rows += 1
    require(rows == COUNT, f"{lane} source inventory count drifted")
    require(
        dict(before) == previous["outcomes"]
        and dict(before_scopes) == previous["explicitEvidenceScopes"],
        f"{lane} predecessor summary differs from ledger",
    )
    wanted = before.copy()
    wanted["not-run"] -= 1
    wanted["expected"] += 1
    scopes = before_scopes.copy()
    scopes[SCOPE] += 1
    require(after == wanted and after_scopes == scopes, f"{lane} delta is not one row")
    ledger = bytes(output)
    require(digest(ledger) == SUCCESSOR_SHA256[lane], f"{lane} successor SHA drifted")
    return ledger, dict(sorted(after.items())), dict(sorted(after_scopes.items()))


def prepare_lane(
    lane: str, envelope: dict[str, Any]
) -> tuple[bytes, bytes, dict[str, Any]]:
    previous_summary = json.loads((PREDECESSOR / f"{lane}.summary.json").read_text())
    ledger, outcomes, scopes = transform(lane, envelope["result"], previous_summary)
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
    summary_raw = (json.dumps(summary, indent=2) + "\n").encode()
    evidence = {
        "base": str(PREDECESSOR / f"{lane}.jsonl"),
        "baseSha256": PREDECESSOR_LEDGER_SHA256[lane],
        "baseSummarySha256": PREDECESSOR_SUMMARY_SHA256[lane],
        "strictEnvelope": str(
            RESULTS / f"{lane}.runserver-url-owner-v3-attestation.json"
        ),
        "strictEnvelopeSha256": ENVELOPE_SHA256[lane],
        "rawProvenance": envelope["rawProvenance"],
        "rawProvenanceSha256": envelope["rawProvenanceSha256"],
        "ownerReceipt": envelope["ownerReceipt"],
        "ownerReceiptSha256": envelope["ownerReceiptSha256"],
        "officialResult": envelope["result"],
        "officialResultSha256": envelope["resultSha256"],
        "candidateArtifactSha256": (
            envelope["snapshot"]["addonSha256"] if lane == "candidate" else None
        ),
        "ledger": str(destination),
        "ledgerSha256": digest(ledger),
        "summarySha256": digest(summary_raw),
        "oldOutcome": "not-run",
        "newOutcome": "expected",
        "promotedRows": 1,
        "oldRowSha256": OLD_ROW_SHA256,
        "newRowSha256": NEW_ROW_SHA256[lane],
        "outcomes": outcomes,
    }
    return ledger, summary_raw, evidence


def prepare_pair() -> dict[str, bytes]:
    evidence = check_inputs()
    prepared = {
        lane: prepare_lane(lane, evidence[lane]) for lane in ("candidate", "oracle")
    }
    harness = {
        str(Path(item["path"]).relative_to(ROOT)): item["sha256"]
        for item in evidence["candidate"]["harnessInputs"]
    }
    require(
        len(harness) == len(evidence["candidate"]["harnessInputs"]),
        "duplicate harness input",
    )
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
        "selectedOfficialTestStartsRemoteServer": True,
        "upstreamAssertionsModified": False,
        "captureSidecarPromotedFlagAtCapture": False,
        "promoterSourceSha256": sha256(Path(__file__).resolve()),
        "pairedVerifierSha256": PAIRED_VERIFIER_SHA256,
        "harnessInputs": harness,
        "priorBundle": str(PREDECESSOR),
        "priorBundleReceiptSha256": PREDECESSOR_RECEIPT_SHA256,
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "recordsPerLane": COUNT,
        "promotedRowsPerLane": 1,
        "targetZeroBasedIndexes": [TARGET_INDEX],
        "targetSpecIds": [TARGET_ID],
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


def publish_exclusive(staging: Path, destination: Path) -> None:
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
        tempfile.mkdtemp(prefix=".core-version-mismatch-v3-", dir=OUTCOMES)
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
        publish_exclusive(temporary, OUTPUT)
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
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()
    require(not sys.flags.optimize, "promotion requires Python assertions enabled")
    require(not (args.verify_existing and args.dry_run), "select one operation")
    if args.verify_existing:
        receipt = verify_existing()
    elif args.dry_run:
        receipt = json.loads(prepare_pair()["promotion-receipt.json"])
    else:
        files = prepare_pair()
        require(files == prepare_pair(), "paired inputs changed before publication")
        publish(files)
        receipt = verify_existing()
    print(json.dumps({"scope": receipt["scope"], "lanes": receipt["lanes"]}, indent=2))


if __name__ == "__main__":
    main()
