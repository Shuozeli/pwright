#!/usr/bin/env python3
"""Seal the three paired official BrowserType path results without touching prior ledgers."""

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

from run_playwright_core_browser_type_owner_strict_v2 import ARTIFACT_SHA256, IDS, ROOT, sha256
from verify_playwright_core_browser_type_owner_strict_v2 import verify_strict

OUTCOMES = ROOT / ".work/official-conformance/outcomes"
RESULTS = ROOT / ".work/official-conformance/results"
BASE = OUTCOMES / "core-cdp-url-query-strict-paired-v1"
BASE_RECEIPT_SHA256 = "e5e0e8e0fbdb61d3d150f69a5592707b9d68cf4351e5133385925713ef5a1479"
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
OUTPUT = OUTCOMES / "core-browser-type-owner-strict-paired-v2"
SCOPE = "official-browser-free-browsertype-executable-path-owner-v2"
COUNT = 48_477
TARGETS = {
    31_211: ("chromium-library", IDS["chromium-library"]),
    31_212: ("firefox-library", IDS["firefox-library"]),
    31_213: ("webkit-library", IDS["webkit-library"]),
}
LANES = {
    "candidate": {
        "baseSha256": "8d4d6a5804657f9757b535c78caf293f41c059ee71375e46fc4413a46245df09",
        "strict": RESULTS / "playwright-core-candidate-browsertype-owner-strict-v2.strict.json",
        "strictSha256": "8d420350c1e0354c1294868e206e6010c7da792aae90cb908fd6787d128d6e45",
    },
    "oracle": {
        "baseSha256": "58503837e2cc5fedd1e4055bc4703ec65d7d7cdc26475ad02ba3b2b871d75fb5",
        "strict": RESULTS / "playwright-core-oracle-browsertype-owner-strict-v2.strict.json",
        "strictSha256": "84a16f6210e7d4cc5eac0424f92e10a202764d8d174d0d9787ec8c05b8050ec1",
    },
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sealed(path: Path, expected_sha: str) -> None:
    require(path.is_file() and not path.is_symlink(), f"missing/symlink evidence: {path}")
    require(sha256(path) == expected_sha, f"SHA-256 drift: {path}")


def check_prior_bundle() -> None:
    sealed(INVENTORY, INVENTORY_SHA256)
    sealed(BASE / "promotion-receipt.json", BASE_RECEIPT_SHA256)
    receipt = json.loads((BASE / "promotion-receipt.json").read_text())
    require(receipt["inventory"] == str(INVENTORY), "prior inventory path drifted")
    require(receipt["inventorySha256"] == INVENTORY_SHA256, "prior inventory drifted")
    for lane, config in LANES.items():
        sealed(BASE / f"{lane}.jsonl", config["baseSha256"])
        require(
            receipt["lanes"][lane]["ledgerSha256"] == config["baseSha256"],
            f"{lane} prior receipt ledger drifted",
        )


def check_target(record: dict[str, Any], index: int, lane: str) -> None:
    project, spec_id = TARGETS[index]
    require(record["specId"] == spec_id, f"{lane} target {index} ID drifted")
    require(
        record["projectId"] == project and record["projectName"] == project,
        f"{lane} target {index} project drifted",
    )
    require(
        record["file"] == "library/browsertype-basic.spec.ts"
        and record["sourceFile"] == "tests/library/browsertype-basic.spec.ts"
        and record["line"] == 21,
        f"{lane} target {index} location drifted",
    )
    require(
        record["titlePath"]
        == [
            "library/browsertype-basic.spec.ts",
            "browserType.executablePath should work",
        ],
        f"{lane} target {index} title drifted",
    )
    require(
        record["sourceClassification"] == "public-api-candidate"
        and record["classificationReasons"] == [],
        f"{lane} target {index} classification drifted",
    )
    require(
        record["outcome"] == "unexpected" and record["attempts"] == 1,
        f"{lane} target {index} is not the previous red result",
    )
    require(
        record["expectedStatus"] == "passed" and record["role"] == "official-entrypoint",
        f"{lane} target {index} expected status drifted",
    )
    require(
        "error" in record and "resultFile" in record,
        f"{lane} target {index} previous evidence missing",
    )


def prepare_lane(lane: str) -> tuple[bytes, bytes, dict[str, Any]]:
    config = LANES[lane]
    base = BASE / f"{lane}.jsonl"
    sealed(base, config["baseSha256"])
    summary_path = BASE / f"{lane}.summary.json"
    require(
        summary_path.is_file() and not summary_path.is_symlink(), f"{lane} base summary missing"
    )
    baseline_summary = json.loads(summary_path.read_text())
    require(
        baseline_summary["records"] == COUNT
        and baseline_summary["ledgerSha256"] == config["baseSha256"]
        and baseline_summary["ledger"] == str(base)
        and baseline_summary["inventory"] == str(INVENTORY),
        f"{lane} base summary drifted",
    )
    strict_path = config["strict"]
    sealed(strict_path, config["strictSha256"])
    strict = verify_strict(strict_path)
    require(
        strict["mode"] == lane and strict["exitCode"] == 0,
        f"{lane} official execution was not successful",
    )
    selected = strict["selected"]
    require(
        len(selected) == 3
        and all(
            row["status"] == "expected"
            and row["attemptStatus"] == "passed"
            and IDS[row["projectId"]] == row["specId"]
            for row in selected
        ),
        f"{lane} official selected rows did not all pass",
    )
    if lane == "candidate":
        require(
            strict["candidateArtifactSha256"] == ARTIFACT_SHA256, "candidate artifact proof drifted"
        )

    before: Counter[str] = Counter()
    after: Counter[str] = Counter()
    before_scopes: Counter[str] = Counter()
    after_scopes: Counter[str] = Counter()
    output = bytearray()
    rows = 0
    with base.open("rb") as baseline:
        for index, line in enumerate(baseline):
            require(line.endswith(b"\n"), f"{lane} row {index} has no terminator")
            record = json.loads(line)
            require(isinstance(record, dict), f"{lane} row {index} is malformed")
            before[record["outcome"]] += 1
            if "evidenceScope" in record:
                before_scopes[record["evidenceScope"]] += 1
            if index in TARGETS:
                check_target(record, index, lane)
                promoted = {key: value for key, value in record.items() if key != "error"}
                promoted.update(
                    {
                        "outcome": "expected",
                        "resultFile": strict["result"],
                        "evidenceScope": SCOPE,
                        "candidateRuntimeRequired": lane == "candidate",
                    }
                )
                require(
                    promoted["sourceClassification"] == record["sourceClassification"]
                    and promoted["classificationReasons"] == record["classificationReasons"],
                    "promotion changed source classification",
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
        and dict(before) == baseline_summary["outcomes"]
        and dict(before_scopes) == baseline_summary["explicitEvidenceScopes"],
        f"{lane} base ledger and summary disagree",
    )
    expected = before.copy()
    expected["unexpected"] -= 3
    expected["expected"] += 3
    scopes = before_scopes.copy()
    scopes[SCOPE] += 3
    require(
        after == expected and after_scopes == scopes,
        f"{lane} successor changed more than three rows",
    )
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
    evidence = {
        "base": str(base),
        "baseSha256": config["baseSha256"],
        "strictSidecar": str(strict_path),
        "strictSidecarSha256": config["strictSha256"],
        "officialResult": strict["result"],
        "officialResultSha256": strict["resultSha256"],
        "candidateArtifactSha256": ARTIFACT_SHA256 if lane == "candidate" else None,
        "ledger": str(destination),
        "ledgerSha256": summary["ledgerSha256"],
        "summarySha256": digest(summary_bytes),
        "outcomes": summary["outcomes"],
    }
    return ledger, summary_bytes, evidence


def prepare_pair() -> dict[str, bytes]:
    check_prior_bundle()
    prepared = {lane: prepare_lane(lane) for lane in LANES}
    receipt = {
        "schema": 1,
        "scope": SCOPE,
        "candidateRuntimeAttested": True,
        "publicApiParityEvidence": True,
        "sourceClassificationPreserved": "public-api-candidate",
        "promoterSourceSha256": sha256(Path(__file__)),
        "priorBundle": str(BASE),
        "priorBundleReceiptSha256": BASE_RECEIPT_SHA256,
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "recordsPerLane": COUNT,
        "targetZeroBasedIndices": sorted(TARGETS),
        "targetSpecIds": [TARGETS[index][1] for index in sorted(TARGETS)],
        "lanes": {lane: evidence for lane, (_, _, evidence) in prepared.items()},
    }
    files = {f"{lane}.jsonl": ledger for lane, (ledger, _, _) in prepared.items()}
    files.update({f"{lane}.summary.json": summary for lane, (_, summary, _) in prepared.items()})
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
    require(not OUTPUT.exists() and not OUTPUT.is_symlink(), "immutable output already exists")
    temporary = Path(tempfile.mkdtemp(prefix=".core-browser-type-owner-v2-", dir=OUTCOMES))
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
        require(
            not OUTPUT.exists() and not OUTPUT.is_symlink(),
            "immutable output appeared during publication",
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
            require(files == prepare_pair(), "paired Core inputs changed before publication")
            publish(files)
            receipt = verify_existing()
    except (AssertionError, OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        parser.error(f"strict paired BrowserType promotion rejected: {error}")
    print(
        json.dumps(
            {
                "scope": receipt["scope"],
                "lanes": {
                    lane: evidence["outcomes"] for lane, evidence in receipt["lanes"].items()
                },
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
