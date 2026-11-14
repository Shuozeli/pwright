#!/usr/bin/env python3
"""Atomically promote two authentic browser-free Core CDP-header rows per lane.

The selected upstream file is fixture-host adapted, but its connectOverCDP
calls and assertions are unchanged. Source classification is never promoted.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import tempfile
from collections import Counter
from pathlib import Path
from typing import Any

from run_playwright_core_cdp_headers_strict import (
    ARTIFACT_SHA256,
    ROOT,
    SOURCE,
    SOURCE_SHA256,
    UPSTREAM,
    sha256,
)
from verify_playwright_core_cdp_headers_strict import verify_strict

OUTCOMES = ROOT / ".work/official-conformance/outcomes"
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
OUTPUT = OUTCOMES / "core-cdp-handshake-headers-strict-paired-v1"
COUNT = 48_477
SCOPE = "official-browser-free-cdp-handshake-headers"
TARGETS = {
    31_780: (
        "8d42d2a6326b0b57ff3d-7c45c41ce41b074940e7",
        242,
        "should send extra headers with connect request",
    ),
    31_783: (
        "8d42d2a6326b0b57ff3d-b543150fbf0c73e3e4ee",
        299,
        "should send default User-Agent header with connect request",
    ),
}
LANES = {
    "candidate": {
        "base": OUTCOMES / "playwright-core-candidate-android-glob-source-only-20261011-v1.jsonl",
        "baseSha256": "b1e8c83af4add22cab75f6876dedac3911c12d5ee796871083640c3555366e5c",
        "strict": OUTCOMES / "core-cdp-handshake-headers-strict-candidate-39-v1.strict.json",
        "expected": {31_780: "expected", 31_783: "unexpected"},
    },
    "oracle": {
        "base": OUTCOMES / "playwright-core-oracle-android-glob-source-only-20261011-v1.jsonl",
        "baseSha256": "c004ad9153787b5a00cb78aa00b1f8191d6c87452e1171ec2b0f4aa95fb6d2c7",
        "strict": OUTCOMES / "core-cdp-handshake-headers-strict-oracle-v1.strict.json",
        "expected": {31_780: "expected", 31_783: "expected"},
    },
}
ANSI = re.compile(r"\x1b\[[0-9;]*m")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sealed_file(path: Path, expected_sha256: str) -> None:
    require(path.is_file() and not path.is_symlink(), f"missing or symlinked evidence: {path}")
    require(sha256(path) == expected_sha256, f"SHA-256 drift: {path}")


def selected_report(report: dict[str, Any], lane: str) -> dict[str, dict[str, Any]]:
    """Require the two exact official assertions and retain their raw results."""
    found: dict[str, dict[str, Any]] = {}

    def visit(suite: dict[str, Any]) -> None:
        for spec in suite.get("specs", []):
            require(spec.get("file") == SOURCE.removeprefix("tests/"), "foreign Core spec in result")
            require(len(spec.get("tests", [])) == 1, "Core spec has the wrong project count")
            test = spec["tests"][0]
            spec_id = spec.get("id")
            require(spec_id not in found, "duplicate Core spec result")
            require(test.get("projectId") == "chromium-library" and
                    test.get("expectedStatus") == "passed" and
                    len(test.get("results", [])) == 1, "Core result project or attempts drifted")
            found[spec_id] = {"spec": spec, "test": test, "attempt": test["results"][0]}
        for child in suite.get("suites", []):
            visit(child)

    require(not report.get("errors"), "official Core reporter has top-level errors")
    for suite in report["suites"]:
        visit(suite)
    require(set(found) == {value[0] for value in TARGETS.values()}, "official Core spec IDs drifted")
    for index, (spec_id, line, title) in TARGETS.items():
        entry = found[spec_id]
        spec, test, attempt = entry["spec"], entry["test"], entry["attempt"]
        expected = LANES[lane]["expected"][index]
        require(spec.get("line") == line and spec.get("column") == 5 and
                spec.get("title") == title, f"official Core source location drifted: {spec_id}")
        require(test.get("status") == expected and
                attempt.get("status") == ("passed" if expected == "expected" else "failed") and
                attempt.get("retry") == 0, f"official Core outcome drifted: {spec_id}")
        if expected == "unexpected":
            error = attempt.get("error")
            require(isinstance(error, dict), "default User-Agent failure has no error")
            location = error.get("location")
            message = ANSI.sub("", str(error.get("message", "")))
            require(isinstance(location, dict) and
                    location.get("file") == str(UPSTREAM / SOURCE) and
                    location.get("line") == 311 and location.get("column") == 43 and
                    "Playwright/1.58.0" in message and "Received: undefined" in message and
                    "expect(request.headers['user-agent']).toBe(getUserAgent());" in
                    str(error.get("snippet", "")),
                    "candidate failure is not the pinned default User-Agent assertion")
    return found


def validate_target(record: dict[str, Any], index: int) -> None:
    spec_id, line, title = TARGETS[index]
    require(record == {
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
    }, f"target row {index} is not the exact unrun implementation-internal inventory row")


def prepare_lane(lane: str) -> tuple[bytes, bytes, dict[str, Any]]:
    config = LANES[lane]
    sealed_file(INVENTORY, INVENTORY_SHA256)
    sealed_file(config["base"], config["baseSha256"])
    base_summary_path = config["base"].with_suffix(".summary.json")
    require(base_summary_path.is_file() and not base_summary_path.is_symlink(), "base summary missing")
    base_summary = json.loads(base_summary_path.read_text())
    require(base_summary.get("schema") == 1 and base_summary.get("records") == COUNT and
            base_summary.get("inventory") == str(INVENTORY) and
            base_summary.get("ledger") == str(config["base"]) and
            base_summary.get("ledgerSha256") == config["baseSha256"],
            f"{lane} base summary is not the pinned strict ledger")
    strict_path = config["strict"]
    require(strict_path.is_file() and not strict_path.is_symlink(), "strict sidecar missing")
    strict = verify_strict(strict_path)
    require(strict["mode"] == lane and strict["upstreamSourceSha256"] == SOURCE_SHA256,
            f"{lane} strict sidecar mode/source drifted")
    if lane == "candidate":
        require(strict["candidateArtifactSha256"] == ARTIFACT_SHA256,
                "candidate strict sidecar does not attest the selected artifact")
    result_path = Path(strict["result"])
    found = selected_report(json.loads(result_path.read_text()), lane)
    result_ref = str(result_path)

    original_outcomes: Counter[str] = Counter()
    promoted_outcomes: Counter[str] = Counter()
    original_scopes: Counter[str] = Counter()
    promoted_scopes: Counter[str] = Counter()
    output = bytearray()
    rows = 0
    with config["base"].open("rb") as baseline:
        for index, line in enumerate(baseline):
            require(line.endswith(b"\n"), f"{lane} base has unterminated row {index}")
            record = json.loads(line)
            require(isinstance(record, dict), f"{lane} base has malformed row {index}")
            original_outcomes[record["outcome"]] += 1
            if "evidenceScope" in record:
                original_scopes[record["evidenceScope"]] += 1
            if index in TARGETS:
                validate_target(record, index)
                spec_id = TARGETS[index][0]
                status = config["expected"][index]
                promoted = {**record, "attempts": 1, "outcome": status,
                            "resultFile": result_ref, "evidenceScope": SCOPE,
                            "candidateRuntimeRequired": lane == "candidate"}
                if status == "unexpected":
                    promoted["error"] = ANSI.sub("", found[spec_id]["attempt"]["error"]["message"])
                require(promoted["sourceClassification"] == record["sourceClassification"] and
                        promoted["classificationReasons"] == record["classificationReasons"],
                        "promotion changed source classification")
                output.extend((json.dumps(promoted, sort_keys=True, separators=(",", ":")) + "\n").encode())
                promoted_outcomes[status] += 1
                promoted_scopes[SCOPE] += 1
            else:
                output.extend(line)
                promoted_outcomes[record["outcome"]] += 1
                if "evidenceScope" in record:
                    promoted_scopes[record["evidenceScope"]] += 1
            rows += 1
    require(rows == COUNT and dict(original_outcomes) == base_summary["outcomes"] and
            dict(original_scopes) == base_summary["explicitEvidenceScopes"],
            f"{lane} base inventory count/summary drifted")
    expected_outcomes = original_outcomes.copy()
    expected_outcomes["not-run"] -= 2
    for status in config["expected"].values():
        expected_outcomes[status] += 1
    expected_scopes = original_scopes.copy()
    expected_scopes[SCOPE] += 2
    require(promoted_outcomes == expected_outcomes and promoted_scopes == expected_scopes,
            f"{lane} successor changed more than the two pinned outcomes")
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
        "base": str(config["base"]),
        "baseSha256": config["baseSha256"],
        "strictSidecar": str(strict_path),
        "strictSidecarSha256": sha256(strict_path),
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
    prepared = {lane: prepare_lane(lane) for lane in LANES}
    receipt = {
        "schema": 1,
        "scope": SCOPE,
        "candidateRuntimeAttested": True,
        "publicApiParityEvidence": False,
        "sourceClassificationPreserved": "implementation-internal",
        "promoterSourceSha256": sha256(Path(__file__)),
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "recordsPerLane": COUNT,
        "targetZeroBasedIndices": sorted(TARGETS),
        "targetSpecIds": [TARGETS[index][0] for index in sorted(TARGETS)],
        "lanes": {lane: evidence for lane, (_, _, evidence) in prepared.items()},
    }
    files = {f"{lane}.jsonl": ledger for lane, (ledger, _, _) in prepared.items()}
    files.update({f"{lane}.summary.json": summary for lane, (_, summary, _) in prepared.items()})
    files["promotion-receipt.json"] = (json.dumps(receipt, indent=2, sort_keys=True) + "\n").encode()
    return files


def validate_published_files(directory: Path, expected: dict[str, bytes]) -> None:
    require(directory.is_dir() and not directory.is_symlink(), "immutable paired Core output is missing")
    require({path.name for path in directory.iterdir()} == set(expected),
            "immutable paired Core output has missing or extra files")
    for name, data in expected.items():
        path = directory / name
        require(path.is_file() and not path.is_symlink() and path.read_bytes() == data,
                f"immutable paired Core output drifted: {name}")


def verify_existing() -> dict[str, Any]:
    expected = prepare_pair()
    validate_published_files(OUTPUT, expected)
    return json.loads(expected["promotion-receipt.json"])


def publish(files: dict[str, bytes]) -> None:
    require(not OUTPUT.exists() and not OUTPUT.is_symlink(), "immutable paired Core output already exists")
    temporary = Path(tempfile.mkdtemp(prefix=".core-cdp-headers-strict-pair-", dir=OUTCOMES))
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
        require(not OUTPUT.exists() and not OUTPUT.is_symlink(),
                "immutable paired Core output appeared during publication")
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
            # Recheck all input seals immediately before publishing either lane.
            require(files == prepare_pair(), "paired Core inputs changed before publication")
            publish(files)
            receipt = verify_existing()
    except (AssertionError, OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
        parser.error(f"strict paired Core promotion rejected: {error}")
    print(json.dumps({
        "bundle": str(OUTPUT),
        "inventorySha256": receipt["inventorySha256"],
        "lanes": {lane: {"ledgerSha256": item["ledgerSha256"],
                         "outcomes": item["outcomes"]}
                  for lane, item in receipt["lanes"].items()},
    }, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
