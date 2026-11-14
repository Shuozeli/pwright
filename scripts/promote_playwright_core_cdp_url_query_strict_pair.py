#!/usr/bin/env python3
"""Atomically promote two sealed browser-free Core CDP URL-query rows per lane."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import tempfile
from collections import Counter
from pathlib import Path
from typing import Any

from promote_playwright_core_cdp_headers_strict_pair import (
    INVENTORY,
    INVENTORY_SHA256,
)
from promote_playwright_core_cdp_headers_strict_pair import (
    verify_existing as verify_headers_pair,
)
from run_playwright_core_cdp_headers_strict import ARTIFACT_SHA256, ROOT, SOURCE, sha256
from verify_playwright_core_cdp_url_query_strict import verify_strict

OUTCOMES = ROOT / ".work/official-conformance/outcomes"
BASE = OUTCOMES / "core-cdp-handshake-headers-strict-paired-v1"
BASE_RECEIPT_SHA256 = "7c6d0d352842b509d6b3e96b06f9aba538f538444dad109c54f239548fecf9bf"
BASE_PROMOTER = ROOT / "scripts/promote_playwright_core_cdp_headers_strict_pair.py"
BASE_PROMOTER_SHA256 = "9e2897c919e3a43873384627db331011b9e5771e2523f70f82643a3a0a74e6f0"
STRICT_VERIFIER = ROOT / "scripts/verify_playwright_core_cdp_url_query_strict.py"
STRICT_VERIFIER_SHA256 = "35bbfa9b2c2b17fa654c83faec6792d1c2bfc42cfc9ac12bc24ef111938d3fd2"
OUTPUT = OUTCOMES / "core-cdp-url-query-strict-paired-v1"
COUNT = 48_477
SCOPE = "official-browser-free-cdp-url-query"
TARGETS = {
    31_781: (
        "8d42d2a6326b0b57ff3d-875e645845c8895a6251",
        275,
        "should keep URL parameters when adding json/version",
    ),
    31_782: (
        "8d42d2a6326b0b57ff3d-6eb553279089d4ca15e6",
        287,
        "should append /json/version with a slash if there isnt one",
    ),
}
LANES = {
    "candidate": {
        "baseSha256": "131e637c7061f7587b0436bd7bc803f1c0419b1a76d714c5f84b6bbcaa3cd823",
        "strict": OUTCOMES / "core-cdp-url-query-strict-candidate-39-v1.strict.json",
        "strictSha256": "ab746961a8ef3ad317d4fd7c089db461f90526ec01d4719c4ffd70b8a208c014",
    },
    "oracle": {
        "baseSha256": "adf29c0f3b4da2f3ae011d42fa3c97491ee8c65efa4e28f00abe4595dcd22889",
        "strict": OUTCOMES / "core-cdp-url-query-strict-oracle-v1.strict.json",
        "strictSha256": "1b5c1fb06e7a909d6f2bf58e55989732699d207d8d4877acc27252dc37a238b5",
    },
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sealed_file(path: Path, expected_sha256: str) -> None:
    require(path.is_file() and not path.is_symlink(), f"missing or symlinked evidence: {path}")
    require(sha256(path) == expected_sha256, f"SHA-256 drift: {path}")


def validate_target(record: dict[str, Any], index: int) -> None:
    spec_id, line, title = TARGETS[index]
    require(
        record
        == {
            "attempts": 0,
            "classificationReasons": [
                "private-client-access",
                "runtime-source-import:packages/playwright-core/lib/server/utils/userAgent",
            ],
            "column": 5,
            "expectedStatus": "passed",
            "file": SOURCE.removeprefix("tests/"),
            "line": line,
            "outcome": "not-run",
            "projectId": "chromium-library",
            "projectName": "chromium-library",
            "role": "official-entrypoint",
            "sourceClassification": "implementation-internal",
            "sourceFile": SOURCE,
            "specId": spec_id,
            "suite": "library",
            "titlePath": [SOURCE.removeprefix("tests/"), title],
        },
        f"target row {index} is not the exact unrun implementation-internal inventory row",
    )


def check_prior_bundle() -> None:
    sealed_file(INVENTORY, INVENTORY_SHA256)
    sealed_file(BASE_PROMOTER, BASE_PROMOTER_SHA256)
    sealed_file(STRICT_VERIFIER, STRICT_VERIFIER_SHA256)
    sealed_file(BASE / "promotion-receipt.json", BASE_RECEIPT_SHA256)
    prior = verify_headers_pair()
    require(prior["inventorySha256"] == INVENTORY_SHA256, "prior pair inventory drifted")
    for lane, config in LANES.items():
        require(
            prior["lanes"][lane]["ledgerSha256"] == config["baseSha256"],
            f"{lane} prior pair ledger drifted",
        )


def check_official_result(strict: dict[str, Any], lane: str) -> None:
    require(strict["mode"] == lane, f"{lane} strict sidecar mode drifted")
    require(strict["exitCode"] == 0, f"{lane} official runner did not pass")
    selected = strict["selected"]
    require(len(selected) == 2, f"{lane} official run does not have two cases")
    require(
        {(entry["specId"], entry["line"], entry["title"]) for entry in selected}
        == {value for value in TARGETS.values()},
        f"{lane} official IDs/locations drifted",
    )
    require(
        all(
            entry["status"] == "expected" and entry["attempts"] == ["passed"] for entry in selected
        ),
        f"{lane} official cases did not both pass",
    )
    if lane == "candidate":
        require(
            strict["candidateArtifactSha256"] == ARTIFACT_SHA256,
            "candidate strict sidecar does not attest the selected native artifact",
        )


def prepare_lane(lane: str) -> tuple[bytes, bytes, dict[str, Any]]:
    config = LANES[lane]
    base = BASE / f"{lane}.jsonl"
    sealed_file(base, config["baseSha256"])
    base_summary_path = BASE / f"{lane}.summary.json"
    require(
        base_summary_path.is_file() and not base_summary_path.is_symlink(), "base summary missing"
    )
    base_summary = json.loads(base_summary_path.read_text())
    require(
        base_summary.get("schema") == 1
        and base_summary.get("records") == COUNT
        and base_summary.get("inventory") == str(INVENTORY)
        and base_summary.get("ledger") == str(base)
        and base_summary.get("ledgerSha256") == config["baseSha256"],
        f"{lane} base summary is not the pinned strict ledger",
    )
    strict_path = config["strict"]
    sealed_file(strict_path, config["strictSha256"])
    strict = verify_strict(strict_path)
    check_official_result(strict, lane)
    result_ref = strict["result"]

    original_outcomes: Counter[str] = Counter()
    promoted_outcomes: Counter[str] = Counter()
    original_scopes: Counter[str] = Counter()
    promoted_scopes: Counter[str] = Counter()
    output = bytearray()
    rows = 0
    with base.open("rb") as baseline:
        for index, line in enumerate(baseline):
            require(line.endswith(b"\n"), f"{lane} base has unterminated row {index}")
            record = json.loads(line)
            require(isinstance(record, dict), f"{lane} base has malformed row {index}")
            original_outcomes[record["outcome"]] += 1
            if "evidenceScope" in record:
                original_scopes[record["evidenceScope"]] += 1
            if index in TARGETS:
                validate_target(record, index)
                promoted = {
                    **record,
                    "attempts": 1,
                    "outcome": "expected",
                    "resultFile": result_ref,
                    "evidenceScope": SCOPE,
                    "candidateRuntimeRequired": lane == "candidate",
                }
                require(
                    promoted["sourceClassification"] == record["sourceClassification"]
                    and promoted["classificationReasons"] == record["classificationReasons"],
                    "promotion changed source classification",
                )
                payload = json.dumps(promoted, sort_keys=True, separators=(",", ":")) + "\n"
                output.extend(payload.encode())
                promoted_outcomes["expected"] += 1
                promoted_scopes[SCOPE] += 1
            else:
                output.extend(line)
                promoted_outcomes[record["outcome"]] += 1
                if "evidenceScope" in record:
                    promoted_scopes[record["evidenceScope"]] += 1
            rows += 1
    require(
        rows == COUNT
        and dict(original_outcomes) == base_summary["outcomes"]
        and dict(original_scopes) == base_summary["explicitEvidenceScopes"],
        f"{lane} base inventory count/summary drifted",
    )
    expected_outcomes = original_outcomes.copy()
    expected_outcomes["not-run"] -= 2
    expected_outcomes["expected"] += 2
    expected_scopes = original_scopes.copy()
    expected_scopes[SCOPE] += 2
    require(
        promoted_outcomes == expected_outcomes and promoted_scopes == expected_scopes,
        f"{lane} successor changed more than two pinned outcomes",
    )
    successor = bytes(output)
    destination = OUTPUT / f"{lane}.jsonl"
    summary = {
        "schema": 1,
        "inventory": str(INVENTORY),
        "records": COUNT,
        "outcomes": dict(sorted(promoted_outcomes.items())),
        "explicitEvidenceScopes": dict(sorted(promoted_scopes.items())),
        "ledgerSha256": digest(successor),
        "ledger": str(destination),
    }
    summary_bytes = (json.dumps(summary, indent=2) + "\n").encode()
    evidence = {
        "base": str(base),
        "baseSha256": config["baseSha256"],
        "strictSidecar": str(strict_path),
        "strictSidecarSha256": config["strictSha256"],
        "officialResult": result_ref,
        "officialResultSha256": strict["resultSha256"],
        "candidateArtifactSha256": ARTIFACT_SHA256 if lane == "candidate" else None,
        "ledger": str(destination),
        "ledgerSha256": summary["ledgerSha256"],
        "summarySha256": digest(summary_bytes),
        "outcomes": summary["outcomes"],
    }
    return successor, summary_bytes, evidence


def prepare_pair() -> dict[str, bytes]:
    check_prior_bundle()
    prepared = {lane: prepare_lane(lane) for lane in LANES}
    receipt = {
        "schema": 1,
        "scope": SCOPE,
        "candidateRuntimeAttested": True,
        "publicApiParityEvidence": False,
        "sourceClassificationPreserved": "implementation-internal",
        "promoterSourceSha256": sha256(Path(__file__)),
        "strictVerifierSha256": STRICT_VERIFIER_SHA256,
        "priorBundle": str(BASE),
        "priorBundleReceiptSha256": BASE_RECEIPT_SHA256,
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "recordsPerLane": COUNT,
        "targetZeroBasedIndices": sorted(TARGETS),
        "targetSpecIds": [TARGETS[index][0] for index in sorted(TARGETS)],
        "lanes": {lane: evidence for lane, (_, _, evidence) in prepared.items()},
    }
    files = {f"{lane}.jsonl": ledger for lane, (ledger, _, _) in prepared.items()}
    files.update({f"{lane}.summary.json": summary for lane, (_, summary, _) in prepared.items()})
    files["promotion-receipt.json"] = (
        json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    ).encode()
    return files


def validate_published_files(directory: Path, expected: dict[str, bytes]) -> None:
    require(
        directory.is_dir() and not directory.is_symlink(),
        "immutable paired Core URL output is missing",
    )
    require(
        {path.name for path in directory.iterdir()} == set(expected),
        "immutable paired Core URL output has missing or extra files",
    )
    for name, data in expected.items():
        path = directory / name
        require(
            path.is_file() and not path.is_symlink() and path.read_bytes() == data,
            f"immutable paired Core URL output drifted: {name}",
        )


def verify_existing() -> dict[str, Any]:
    expected = prepare_pair()
    validate_published_files(OUTPUT, expected)
    return json.loads(expected["promotion-receipt.json"])


def publish(files: dict[str, bytes]) -> None:
    require(
        not OUTPUT.exists() and not OUTPUT.is_symlink(),
        "immutable paired Core URL output already exists",
    )
    temporary = Path(tempfile.mkdtemp(prefix=".core-cdp-url-strict-pair-", dir=OUTCOMES))
    try:
        for name, data in files.items():
            with (temporary / name).open("xb") as stream:
                stream.write(data)
                stream.flush()
                os.fsync(stream.fileno())
        directory_fd = os.open(temporary, os.O_RDONLY)
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)
        require(
            not OUTPUT.exists() and not OUTPUT.is_symlink(),
            "immutable paired Core URL output appeared during publication",
        )
        os.rename(temporary, OUTPUT)
        parent_fd = os.open(OUTCOMES, os.O_RDONLY)
        try:
            os.fsync(parent_fd)
        finally:
            os.close(parent_fd)
    finally:
        if temporary.exists():
            shutil.rmtree(temporary)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--verify-existing", action="store_true")
    args = parser.parse_args()
    try:
        if args.verify_existing:
            receipt = verify_existing()
        else:
            files = prepare_pair()
            require(files == prepare_pair(), "paired Core URL inputs changed before publication")
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
        parser.error(f"strict paired Core URL promotion rejected: {error}")
    print(
        json.dumps(
            {
                "bundle": str(OUTPUT),
                "inventorySha256": receipt["inventorySha256"],
                "lanes": {
                    lane: {"ledgerSha256": item["ledgerSha256"], "outcomes": item["outcomes"]}
                    for lane, item in receipt["lanes"].items()
                },
            },
            indent=2,
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
