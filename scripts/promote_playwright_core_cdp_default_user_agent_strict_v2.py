"""Seal one official default-User-Agent result per Core ledger lane.

The oracle row already passed in the predecessor. This successor refreshes its
evidence without changing its outcome; only the candidate outcome turns green.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import tempfile
from collections import Counter
from itertools import zip_longest
from pathlib import Path
from typing import Any

from run_playwright_core_cdp_default_user_agent_strict_v2 import (
    COMMIT,
    MANIFEST,
    PRELOAD,
    PROFILE,
    ROOT,
    SOURCE,
    SOURCE_SHA256,
    SPEC_ID,
    selected_tests,
    sha256,
)
from run_playwright_core_cdp_default_user_agent_strict_v2 import SCOPE as STRICT_SCOPE
from verify_playwright_core_cdp_default_user_agent_strict_v2 import verify_strict

OUTCOMES = ROOT / ".work/official-conformance/outcomes"
RESULTS = ROOT / ".work/official-conformance/results"
BASE = OUTCOMES / "core-browser-type-owner-strict-paired-v2"
BASE_RECEIPT_SHA256 = "26e45506688e5bf994393a6d7555cbfd3adc5dffeec1de9370f98b38d504cd0d"
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
OUTPUT = OUTCOMES / "core-cdp-default-user-agent-strict-paired-v2"
SCOPE = "official-browser-free-cdp-default-user-agent-strict-v2"
OLD_SCOPE = "official-browser-free-cdp-handshake-headers"
COUNT = 48_477
TARGET_INDEX = 31_783
ARTIFACT_SHA256 = "92f039f5530075841ca7d59ff523c1f6b7a5ec4884a29ff20042ef3c95826fb2"
VERIFIER = ROOT / "scripts/verify_playwright_core_cdp_default_user_agent_strict_v2.py"
VERIFIER_SHA256 = "779743e5e6b1069e03e51a95a52bc4892782f40cba0a2388b52a812672f80285"
HARNESS_PINS = {
    str(
        MANIFEST.relative_to(ROOT)
    ): "75e0537d31ef2d0a37f41a072637fb5d50c0f941f4502b28c72bb6bffd7e379c",
    str(
        PRELOAD.relative_to(ROOT)
    ): "f0178a88b275db05d7f351ea3c10842e9a79b3f76d7678d31cd2bf1990d9b1e1",
    "scripts/run_playwright_core_cdp_default_user_agent_strict_v2.py": (
        "3f7b3dececa1e1a075e259349cef1d6"
        "b0584d1a93cc47eb047346593aeb9a440"
    ),
}
LANES = {
    "candidate": {
        "baseSha256": "40625a1de093799e3ad19b0059721c95fd84f3fa04654f373d273ff80abd2e71",
        "strict": RESULTS
        / "playwright-core-candidate-cdp-default-user-agent-strict-v2.strict.json",
        "strictSha256": "92ad46b5b076bf04b53e89dda8a025f41d5e6713282a495d05274c0d274ea2bd",
        "oldOutcome": "unexpected",
    },
    "oracle": {
        "baseSha256": "a03b9b2f9cf1ec06e58db2ce08bad2c770b8eb8620f73ee1aec82d0415f10fd5",
        "strict": RESULTS
        / "playwright-core-oracle-cdp-default-user-agent-strict-v2.strict.json",
        "strictSha256": "a3b9427bef86b8b4a5314625c5638e832971240546f8f042f2a91b1f50db1392",
        "oldOutcome": "expected",
    },
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
CLASSIFICATION_REASONS = [
    "private-client-access",
    "runtime-source-import:packages/playwright-core/lib/server/utils/userAgent",
]


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sealed(path: Path, expected_sha256: str) -> None:
    require(
        len(expected_sha256) == 64
        and all(c in "0123456789abcdef" for c in expected_sha256),
        f"unfilled or invalid SHA-256 pin: {path}",
    )
    require(
        path.is_file() and not path.is_symlink(), f"missing/symlink evidence: {path}"
    )
    require(sha256(path) == expected_sha256, f"SHA-256 drift: {path}")


def check_prior_bundle() -> None:
    sealed(INVENTORY, INVENTORY_SHA256)
    sealed(BASE / "promotion-receipt.json", BASE_RECEIPT_SHA256)
    receipt = json.loads((BASE / "promotion-receipt.json").read_text())
    require(
        receipt["inventory"] == str(INVENTORY)
        and receipt["inventorySha256"] == INVENTORY_SHA256
        and receipt["recordsPerLane"] == COUNT,
        "prior bundle inventory drifted",
    )
    for lane, config in LANES.items():
        base = BASE / f"{lane}.jsonl"
        sealed(base, config["baseSha256"])
        require(
            receipt["lanes"][lane]["ledger"] == str(base)
            and receipt["lanes"][lane]["ledgerSha256"] == config["baseSha256"],
            f"{lane} prior receipt ledger drifted",
        )
        summary = BASE / f"{lane}.summary.json"
        sealed(summary, receipt["lanes"][lane]["summarySha256"])
        record = json.loads(summary.read_text())
        require(
            record["records"] == COUNT
            and record["inventory"] == str(INVENTORY)
            and record["ledger"] == str(base)
            and record["ledgerSha256"] == config["baseSha256"],
            f"{lane} prior summary drifted",
        )


def check_harness() -> None:
    sealed(VERIFIER, VERIFIER_SHA256)
    for relative, expected_sha256 in HARNESS_PINS.items():
        sealed(ROOT / relative, expected_sha256)


def check_target(record: dict[str, Any], lane: str) -> None:
    require(
        record["specId"] == SPEC_ID
        and record["projectId"] == record["projectName"] == "chromium-library",
        f"{lane} target identity drifted",
    )
    require(
        record["sourceFile"] == SOURCE
        and record["file"] == SOURCE.removeprefix("tests/")
        and record["line"] == 299
        and record["column"] == 5
        and record["titlePath"]
        == [
            SOURCE.removeprefix("tests/"),
            "should send default User-Agent header with connect request",
        ],
        f"{lane} target location drifted",
    )
    require(
        record["sourceClassification"] == "implementation-internal"
        and record["classificationReasons"] == CLASSIFICATION_REASONS
        and record["expectedStatus"] == "passed"
        and record["role"] == "official-entrypoint"
        and record["suite"] == "library",
        f"{lane} target source classification drifted",
    )
    require(
        record["outcome"] == LANES[lane]["oldOutcome"]
        and record["attempts"] == 1
        and record["evidenceScope"] == OLD_SCOPE
        and record["candidateRuntimeRequired"] is (lane == "candidate")
        and isinstance(record["resultFile"], str),
        f"{lane} target prior outcome/evidence drifted",
    )
    if lane == "candidate":
        require(
            isinstance(record.get("error"), str)
            and "Received: undefined" in record["error"],
            "candidate target did not contain the pinned User-Agent failure",
        )
    else:
        require("error" not in record, "oracle target unexpectedly contains an error")


def official_pass(lane: str) -> dict[str, Any]:
    config = LANES[lane]
    strict_path = config["strict"]
    sealed(strict_path, config["strictSha256"])
    strict = verify_strict(strict_path)
    require(
        strict["mode"] == lane
        and strict["exitCode"] == 0
        and strict["scope"] == STRICT_SCOPE
        and strict["upstreamCommit"] == COMMIT
        and strict["upstreamSourceSha256"] == SOURCE_SHA256
        and strict["inputs"] == HARNESS_PINS,
        f"{lane} strict official provenance drifted",
    )
    if lane == "candidate":
        require(
            strict["candidateProfile"] == PROFILE
            and strict["candidateArtifactSha256"] == ARTIFACT_SHA256,
            "candidate strict sidecar does not attest the selected opt-in artifact",
        )
    else:
        require(
            strict["candidateProfile"] is None
            and strict["candidateArtifactSha256"] is None,
            "oracle strict sidecar contains candidate input",
        )
    result = Path(strict["result"])
    require(
        result.is_absolute()
        and result.is_file()
        and not result.is_symlink()
        and result.resolve()
        == strict_path.with_name(
            strict_path.name.removesuffix(".strict.json") + ".json"
        ).resolve()
        and sha256(result) == strict["resultSha256"],
        f"{lane} official result identity drifted",
    )
    selected = selected_tests(json.loads(result.read_text()))
    require(
        strict["selected"] == selected
        and len(selected) == 1
        and selected[0]["specId"] == SPEC_ID
        and selected[0]["status"] == "expected"
        and selected[0]["attempts"] == [{"retry": 0, "status": "passed"}],
        f"{lane} official User-Agent assertion did not pass",
    )
    return strict


def prepare_lane(lane: str) -> tuple[bytes, bytes, dict[str, Any]]:
    require(lane in LANES, f"unknown lane: {lane}")
    config = LANES[lane]
    base = BASE / f"{lane}.jsonl"
    prior_summary = json.loads((BASE / f"{lane}.summary.json").read_text())
    strict = official_pass(lane)
    before: Counter[str] = Counter()
    after: Counter[str] = Counter()
    before_scopes: Counter[str] = Counter()
    after_scopes: Counter[str] = Counter()
    output = bytearray()
    rows = 0
    with INVENTORY.open("rb") as inventory, base.open("rb") as baseline:
        for index, pair in enumerate(zip_longest(inventory, baseline)):
            inventory_line, line = pair
            require(
                inventory_line is not None and line is not None,
                f"{lane} inventory and baseline lengths differ at row {index}",
            )
            require(
                inventory_line.endswith(b"\n") and line.endswith(b"\n"),
                f"{lane} row {index} is unterminated",
            )
            source = json.loads(inventory_line)
            record = json.loads(line)
            require(
                isinstance(source, dict) and isinstance(record, dict),
                f"{lane} row {index} is malformed",
            )
            require(
                all(record.get(key) == source.get(key) for key in IDENTITY_FIELDS),
                f"{lane} inventory identity/classification drift at row {index}",
            )
            before[record["outcome"]] += 1
            if "evidenceScope" in record:
                before_scopes[record["evidenceScope"]] += 1
            if index == TARGET_INDEX:
                check_target(record, lane)
                promoted = {
                    key: value for key, value in record.items() if key != "error"
                }
                promoted.update(
                    {
                        "outcome": "expected",
                        "resultFile": strict["result"],
                        "evidenceScope": SCOPE,
                        "candidateRuntimeRequired": lane == "candidate",
                    }
                )
                require(
                    all(promoted[key] == record[key] for key in IDENTITY_FIELDS)
                    and promoted["attempts"] == record["attempts"],
                    "target promotion changed inventory identity/classification",
                )
                output.extend(
                    (
                        json.dumps(promoted, sort_keys=True, separators=(",", ":"))
                        + "\n"
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
        rows == COUNT
        and dict(before) == prior_summary["outcomes"]
        and dict(before_scopes) == prior_summary["explicitEvidenceScopes"],
        f"{lane} prior summary disagrees with its ledger",
    )
    expected_outcomes = before.copy()
    if lane == "candidate":
        expected_outcomes["unexpected"] -= 1
        expected_outcomes["expected"] += 1
    expected_scopes = before_scopes.copy()
    expected_scopes[OLD_SCOPE] -= 1
    expected_scopes[SCOPE] += 1
    require(
        after == expected_outcomes and after_scopes == expected_scopes,
        f"{lane} successor changed more than the pinned User-Agent row",
    )
    successor = bytes(output)
    destination = OUTPUT / f"{lane}.jsonl"
    summary = {
        "schema": 1,
        "inventory": str(INVENTORY),
        "records": COUNT,
        "outcomes": dict(sorted(after.items())),
        "explicitEvidenceScopes": dict(sorted(after_scopes.items())),
        "ledgerSha256": digest(successor),
        "ledger": str(destination),
    }
    summary_bytes = (json.dumps(summary, indent=2) + "\n").encode()
    evidence = {
        "base": str(base),
        "baseSha256": config["baseSha256"],
        "strictSidecar": str(config["strict"]),
        "strictSidecarSha256": config["strictSha256"],
        "officialResult": strict["result"],
        "officialResultSha256": strict["resultSha256"],
        "candidateArtifactSha256": ARTIFACT_SHA256 if lane == "candidate" else None,
        "ledger": str(destination),
        "ledgerSha256": summary["ledgerSha256"],
        "summarySha256": digest(summary_bytes),
        "oldOutcome": config["oldOutcome"],
        "newOutcome": "expected",
        "outcomes": summary["outcomes"],
    }
    return successor, summary_bytes, evidence


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
        "targetZeroBasedIndex": TARGET_INDEX,
        "targetSpecId": SPEC_ID,
        "lanes": {lane: evidence for lane, (_, _, evidence) in prepared.items()},
    }
    files = {f"{lane}.jsonl": ledger for lane, (ledger, _, _) in prepared.items()}
    files.update(
        {f"{lane}.summary.json": summary for lane, (_, summary, _) in prepared.items()}
    )
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
    temporary = Path(tempfile.mkdtemp(prefix=".core-cdp-default-ua-v2-", dir=OUTCOMES))
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
            require(
                files == prepare_pair(), "paired Core inputs changed before publication"
            )
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
            f"strict paired Core default User-Agent promotion rejected: {error}"
        )
    print(
        json.dumps(
            {
                "scope": receipt["scope"],
                "lanes": {
                    lane: evidence["outcomes"]
                    for lane, evidence in receipt["lanes"].items()
                },
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
