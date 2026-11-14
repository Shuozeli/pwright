"""Independently replay the six-row Core BiDi dummy-node v9-to-v10 rerun.

The v9 RED candidate rows remain historical evidence. A separate, immutable
v10 successor may record the six independently captured v3 GREEN reruns, but
neither a dummy process nor an upstream assertion proves browser/BiDi parity.
This module does not import or trust the v10 promoter's transformation.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib
import json
import sys
from collections import Counter
from collections.abc import Iterable, Mapping
from itertools import zip_longest
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
OUTCOMES = ROOT / ".work/official-conformance/outcomes"
V8 = OUTCOMES / "core-server-connect-close-error-bidi-chromium-strict-paired-v8"
V9 = OUTCOMES / "core-dummy-node-bidi-strict-paired-v9"
V10 = OUTCOMES / "core-dummy-node-bidi-strict-paired-v10"
RESULTS = ROOT / ".work/official-conformance/results/core-dummy-node-bidi-strict-v3"
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
MANIFEST = (
    ROOT / "tests/official/playwright-core-dummy-node-bidi-strict-v3-manifest.json"
)
STRICT_CAPTURE = ROOT / "scripts/run_playwright_core_dummy_node_bidi_strict_v3.py"
STRICT_VERIFIER = ROOT / "scripts/verify_playwright_core_dummy_node_bidi_strict_v3.py"
V9_VERIFIER = ROOT / "scripts/verify_playwright_core_dummy_node_bidi_paired_v9.py"
PROMOTER = ROOT / "scripts/promote_playwright_core_dummy_node_bidi_strict_v10.py"
COUNT = 48_477
LANES = ("candidate", "oracle")
SCOPE = "official-browser-free-browsertype-launch-dummy-node-bidi-config-rerun-v10"
V9_SCOPE = "official-browser-free-browsertype-launch-dummy-node-bidi-config-outcomes-v9"
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
V8_RECEIPT_SHA256 = "4c0a72a233440263a38bf718fa003fa2e8f8c36ef0243aaac9811c314e14dd68"
V8_LEDGER_SHA256 = {
    "candidate": "d41e853872aaed73e303184909b9323e7229e1e5068307ac831419bf7b5ab1d6",
    "oracle": "c6fd28c4db0184f5821489907e5b489f500f2752c34a5d097ade1b68f5f565b4",
}
V8_SUMMARY_SHA256 = {
    "candidate": "fb81213cf7674ba26ba170f5216afc1143c72456ba5873da61132d652cb15339",
    "oracle": "10b68d822d275d0b2a916ee559048ce44fb646a388a82ef40f97ef40a3805f86",
}
V9_VERIFIER_SHA256 = "fe377e1414984f1e261feb089444777f2191fe953d04d13081dae8f8f3b97b98"
V9_RECEIPT_SHA256 = "f35fa767281337803ea16ece4a12bb36de52523fe9e348fc7f4467402c3f215f"
V9_LEDGER_SHA256 = {
    "candidate": "330918e67a4a650cca9cb1142016a7329d501bba731c38e8deaf3254012bb77e",
    "oracle": "c5365c8c68841b23abb5a9cbbed6bb19a8e6a93adbea6970c9c1d2fe30e43602",
}
V9_SUMMARY_SHA256 = {
    "candidate": "6cc9fb132c1f7d849c7eb3f5e817582885c4631e94af9d55408350aec3b6bd30",
    "oracle": "d041eb8c518a8d692e87bf7b29cd1741db0b1594e969ecaab58a749845a74c38",
}
MANIFEST_SHA256 = "045271d513fbe7844e4389beabc4e753a5b34cbf1ad1d79a9f1641e425e9d767"
STRICT_CAPTURE_SHA256 = (
    "dfbbb9be2f9e928b43c22a56a686aa0abe6819636ba23eeaad70b2e83a80a9b0"
)
STRICT_VERIFIER_SHA256 = (
    "1cc694c9a5b2e77faf67c60ff68f026931814d0f92c51e23835f456658ea3515"
)
SOURCE_SHA256 = "64fe24188ec96bb0d5d1ecca5af37986b16d82451b91e4db80593f707126bd05"
TARGETS = {
    6110: (
        "bidi-chrome-canary-library",
        "chromium",
        "d1a604bdf6287c6776f3-53abd42f8ece2b451e5f",
    ),
    6111: (
        "bidi-chrome-stable-library",
        "chromium",
        "d1a604bdf6287c6776f3-ee1b24a2e94e134f0b8a",
    ),
    6112: (
        "bidi-chromium-library",
        "chromium",
        "d1a604bdf6287c6776f3-f708c432fe2c80d2440a",
    ),
    6113: (
        "moz-firefox-beta-library",
        "firefox",
        "d1a604bdf6287c6776f3-a1c0f40a45acfee26558",
    ),
    6114: (
        "moz-firefox-library",
        "firefox",
        "d1a604bdf6287c6776f3-6ecf3d1ccae0b454c888",
    ),
    6115: (
        "moz-firefox-nightly-library",
        "firefox",
        "d1a604bdf6287c6776f3-88b0839d68b9d44cb588",
    ),
}
SIDECAR_SHA256 = {
    (
        "candidate",
        "bidi-chrome-canary-library",
    ): "98a8efea1a52cfcd59345902c7fdfa3f2d85d0468c38b9ab608b95b356ba1c22",
    (
        "candidate",
        "bidi-chrome-stable-library",
    ): "306516d156f3bbcf1a1af005d41f6f15ca92756442a93c104b21b785965c2711",
    (
        "candidate",
        "bidi-chromium-library",
    ): "17f3138aaf9f4045ee674df5be929488758176dea993a5eef9d557058e3171d3",
    (
        "candidate",
        "moz-firefox-beta-library",
    ): "49bfc418a068dee99fea6033ea5f757b51dbbfc4d601fa92568c90ded1264fcd",
    (
        "candidate",
        "moz-firefox-library",
    ): "00f135241d8b5d2d5c747fb01786d466af6eb844dab35c867a0941a2f78901d6",
    (
        "candidate",
        "moz-firefox-nightly-library",
    ): "de72e0db2d8cb1be1b3ab7584f580930bf940bc08e9c69dd82b80cfecfd1d26d",
    (
        "oracle",
        "bidi-chrome-canary-library",
    ): "7ed1c495fead5b5ebd7eebb2512a2d02d21ab5ca5a9c447d367ab0ad9031b9fe",
    (
        "oracle",
        "bidi-chrome-stable-library",
    ): "693be2d77b9187a2202e0995504f597a0c2c79d5c9fc2ebdc1106acb4e5c2616",
    (
        "oracle",
        "bidi-chromium-library",
    ): "0b618f2f13f25af0e9a03f1fef732ed9ee6e54cb783377ddfef6b51c852aef23",
    (
        "oracle",
        "moz-firefox-beta-library",
    ): "f3c0890ed98a5c461a2629f50b11e408efd5cb51298936555d7eb8cb1cf8af67",
    (
        "oracle",
        "moz-firefox-library",
    ): "647714b9cb443009102687c4b2260b6f96e18361454c25cfc7e0e243d86e4457",
    (
        "oracle",
        "moz-firefox-nightly-library",
    ): "4d2513745fd9882d0d14a05c201fd1564781978906475ef72c35d2caadd17461",
}
EXPECTED_FILES = {
    "candidate.jsonl",
    "candidate.summary.json",
    "oracle.jsonl",
    "oracle.summary.json",
    "promotion-receipt.json",
}
RECEIPT_FIELDS = {
    "schema",
    "scope",
    "promotionKind",
    "sourceClassificationPreserved",
    "publicApiParityEvidence",
    "browserExecutionEvidence",
    "bidiProtocolParityEvidence",
    "fixtureAdapted",
    "upstreamAssertionsModified",
    "selectedExecutable",
    "promoterSourceSha256",
    "pairedVerifierSha256",
    "strictCaptureSha256",
    "strictVerifierSha256",
    "strictManifestSha256",
    "priorVerifierSha256",
    "priorBundle",
    "priorBundleReceiptSha256",
    "inventory",
    "inventorySha256",
    "recordsPerLane",
    "changedRowsPerLane",
    "unchangedRawRowsPerLane",
    "targetZeroBasedIndexes",
    "targetSpecIds",
    "upstreamCommit",
    "selectedUpstreamSpecSha256",
    "candidateGreenRows",
    "candidateRedRows",
    "oracleGreenRows",
    "lanes",
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sealed(path: Path, expected: str, label: str) -> bytes:
    require(path.is_file() and not path.is_symlink(), f"{label} missing or linked")
    data = path.read_bytes()
    require(digest(data) == expected, f"{label} SHA-256 drifted")
    return data


def object_json(data: bytes, label: str) -> dict[str, Any]:
    value = json.loads(data)
    require(isinstance(value, dict), f"{label} is not a JSON object")
    return value


def canonical_line(record: dict[str, Any]) -> bytes:
    return (json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n").encode()


def sidecar_path(lane: str, project: str) -> Path:
    require((lane, project) in SIDECAR_SHA256, "unknown strict v3 case")
    return RESULTS / f"{lane}-{project}.strict.json"


def target_source(project: str, spec_id: str) -> dict[str, Any]:
    return {
        "classificationReasons": [],
        "column": 3,
        "expectedStatus": "passed",
        "file": "library/browsertype-launch.spec.ts",
        "line": 64,
        "projectId": project,
        "projectName": project,
        "role": "official-entrypoint",
        "sourceClassification": "public-api-candidate",
        "sourceFile": "tests/library/browsertype-launch.spec.ts",
        "specId": spec_id,
        "suite": "bidi",
        "titlePath": [
            "library/browsertype-launch.spec.ts",
            "should reject if launched browser fails immediately",
        ],
    }


def verify_v9_predecessor() -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    """Validate sealed v9 without live-replaying its now-obsolete v2 source.

    The v2 strict verifier ties its contemporary candidate source to the old
    RED run. That source correctly changed for the v3 fix. Replaying it against
    today's source would reject the immutable historical evidence, so we seal
    v8/v9 by their published hashes and independently compare raw ledgers.
    """
    inventory = sealed(INVENTORY, INVENTORY_SHA256, "Core inventory").splitlines(
        keepends=True
    )
    sealed(V9_VERIFIER, V9_VERIFIER_SHA256, "v9 independent verifier source")
    sealed(V8 / "promotion-receipt.json", V8_RECEIPT_SHA256, "v8 receipt")
    receipt_bytes = sealed(
        V9 / "promotion-receipt.json", V9_RECEIPT_SHA256, "v9 receipt"
    )
    receipt = object_json(receipt_bytes, "v9 receipt")
    require(
        receipt_bytes == (json.dumps(receipt, indent=2, sort_keys=True) + "\n").encode()
        and receipt["scope"] == V9_SCOPE
        and receipt["priorBundle"] == str(V8)
        and receipt["priorBundleReceiptSha256"] == V8_RECEIPT_SHA256
        and receipt["recordsPerLane"] == COUNT
        and receipt["changedRowsPerLane"] == 6
        and receipt["targetZeroBasedIndexes"] == list(TARGETS)
        and receipt["targetSpecIds"] == [case[2] for case in TARGETS.values()]
        and receipt["sourceClassificationPreserved"] == "public-api-candidate"
        and receipt["publicApiParityEvidence"] is False
        and receipt["browserExecutionEvidence"] is False
        and receipt["bidiProtocolParityEvidence"] is False
        and receipt["candidateGreenRows"] == 0
        and receipt["candidateRedRows"] == 6
        and receipt["inventory"] == str(INVENTORY)
        and receipt["inventorySha256"] == INVENTORY_SHA256
        and set(receipt["lanes"]) == set(LANES),
        "v9 sealed receipt identity or limitations differ",
    )
    summaries = {}
    for lane in LANES:
        old_bytes = sealed(
            V8 / f"{lane}.jsonl", V8_LEDGER_SHA256[lane], f"v8 {lane} ledger"
        )
        sealed(
            V8 / f"{lane}.summary.json", V8_SUMMARY_SHA256[lane], f"v8 {lane} summary"
        )
        new_bytes = sealed(
            V9 / f"{lane}.jsonl", V9_LEDGER_SHA256[lane], f"v9 {lane} ledger"
        )
        summary = object_json(
            sealed(
                V9 / f"{lane}.summary.json",
                V9_SUMMARY_SHA256[lane],
                f"v9 {lane} summary",
            ),
            f"v9 {lane} summary",
        )
        lane_receipt = receipt["lanes"][lane]
        require(
            summary["schema"] == 1
            and summary["inventory"] == str(INVENTORY)
            and summary["records"] == COUNT
            and summary["ledger"] == str(V9 / f"{lane}.jsonl")
            and summary["ledgerSha256"] == V9_LEDGER_SHA256[lane]
            and lane_receipt["base"] == str(V8 / f"{lane}.jsonl")
            and lane_receipt["baseSha256"] == V8_LEDGER_SHA256[lane]
            and lane_receipt["baseSummarySha256"] == V8_SUMMARY_SHA256[lane]
            and lane_receipt["ledger"] == summary["ledger"]
            and lane_receipt["ledgerSha256"] == summary["ledgerSha256"]
            and lane_receipt["summarySha256"] == V9_SUMMARY_SHA256[lane]
            and lane_receipt["promotedRows"] == 6
            and lane_receipt["outcomes"] == summary["outcomes"]
            and set(lane_receipt["strictEvidence"])
            == {project for project, _, _ in TARGETS.values()},
            f"v9 {lane} receipt or summary differs",
        )
        old_lines = old_bytes.splitlines(keepends=True)
        new_lines = new_bytes.splitlines(keepends=True)
        require(
            len(inventory) == len(old_lines) == len(new_lines) == COUNT,
            f"v8/v9 {lane} row count differs",
        )
        outcomes: Counter[str] = Counter()
        scopes: Counter[str] = Counter()
        changes = []
        for index, (source_raw, old_raw, new_raw) in enumerate(
            zip(inventory, old_lines, new_lines, strict=True)
        ):
            if index not in TARGETS:
                require(
                    old_raw == new_raw,
                    f"v8/v9 {lane} non-target raw row {index} changed",
                )
            else:
                project, _, spec_id = TARGETS[index]
                source = object_json(source_raw, f"inventory target {index}")
                old = object_json(old_raw, f"v8 {lane} target {index}")
                new = object_json(new_raw, f"v9 {lane} target {index}")
                status = "unexpected" if lane == "candidate" else "expected"
                require(
                    source == target_source(project, spec_id)
                    and old == {**source, "attempts": 0, "outcome": "not-run"}
                    and new
                    == {
                        **source,
                        "attempts": 1,
                        "evidenceScope": V9_SCOPE,
                        "outcome": status,
                        "publicApiParityEvidence": False,
                        "resultFile": lane_receipt["strictEvidence"][project][
                            "officialResult"
                        ],
                    }
                    and new_raw == canonical_line(new),
                    f"v8/v9 {lane} target row {index} differs",
                )
                changes.append(
                    {
                        "index": index,
                        "specId": spec_id,
                        "oldRowSha256": digest(old_raw),
                        "newRowSha256": digest(new_raw),
                        "outcome": status,
                    }
                )
            row = object_json(new_raw, f"v9 {lane} row {index}")
            outcomes[row["outcome"]] += 1
            if "evidenceScope" in row:
                scopes[row["evidenceScope"]] += 1
        require(
            changes == lane_receipt["changedRows"]
            and dict(outcomes) == summary["outcomes"]
            and dict(scopes) == summary["explicitEvidenceScopes"]
            and scopes[V9_SCOPE] == 6,
            f"v9 {lane} row arithmetic or changed-row receipt differs",
        )
        summaries[lane] = summary
    return receipt, summaries


def verify_inputs() -> tuple[
    dict[str, Any], dict[str, dict[str, Any]], dict[tuple[str, str], dict[str, Any]]
]:
    """Seal historical v9 and live-replay all twelve strict v3 sidecars."""
    require(
        not sys.flags.optimize, "paired Core verification requires assertions enabled"
    )
    prior_receipt, prior_summaries = verify_v9_predecessor()
    manifest = object_json(
        sealed(MANIFEST, MANIFEST_SHA256, "strict v3 manifest"), "strict v3 manifest"
    )
    expected_cases = [
        {"index": index, "project": project, "engine": engine, "specId": spec_id}
        for index, (project, engine, spec_id) in TARGETS.items()
    ]
    require(
        manifest.get("schema") == 1
        and manifest.get("commit") == COMMIT
        and manifest.get("selector") == "tests/library/browsertype-launch.spec.ts:64"
        and manifest.get("candidateProfile") == "bidi-startup-failure"
        and manifest.get("cases") == expected_cases
        and manifest.get("v9CandidateLedgerSha256") == V9_LEDGER_SHA256["candidate"]
        and manifest.get("v9OracleLedgerSha256") == V9_LEDGER_SHA256["oracle"]
        and manifest.get("v9ReceiptSha256") == V9_RECEIPT_SHA256
        and manifest.get("upstreamSources", {}).get(
            "tests/library/browsertype-launch.spec.ts"
        )
        == SOURCE_SHA256,
        "strict v3 manifest target/predecessor identity differs",
    )
    sealed(STRICT_CAPTURE, STRICT_CAPTURE_SHA256, "strict v3 capture source")
    sealed(STRICT_VERIFIER, STRICT_VERIFIER_SHA256, "strict v3 verifier source")
    originals = {
        (lane, project): sealed(
            sidecar_path(lane, project),
            SIDECAR_SHA256[(lane, project)],
            f"{lane} {project} v3 sidecar",
        )
        for lane in LANES
        for project, _, _ in TARGETS.values()
    }
    strict = importlib.import_module("verify_playwright_core_dummy_node_bidi_strict_v3")
    strict.verify_all(
        [
            sidecar_path(lane, project)
            for lane in LANES
            for project, _, _ in TARGETS.values()
        ]
    )
    sealed(INVENTORY, INVENTORY_SHA256, "Core inventory after strict replay")
    sealed(
        V9 / "promotion-receipt.json",
        V9_RECEIPT_SHA256,
        "v9 receipt after strict replay",
    )
    sealed(MANIFEST, MANIFEST_SHA256, "strict v3 manifest after replay")
    sealed(STRICT_CAPTURE, STRICT_CAPTURE_SHA256, "strict v3 capture after replay")
    sealed(STRICT_VERIFIER, STRICT_VERIFIER_SHA256, "strict v3 verifier after replay")
    for lane in LANES:
        sealed(
            V9 / f"{lane}.jsonl",
            V9_LEDGER_SHA256[lane],
            f"v9 {lane} ledger after replay",
        )
        sealed(
            V9 / f"{lane}.summary.json",
            V9_SUMMARY_SHA256[lane],
            f"v9 {lane} summary after replay",
        )
    evidence = {}
    for key, original in originals.items():
        lane, project = key
        current = sealed(
            sidecar_path(lane, project),
            SIDECAR_SHA256[key],
            f"{lane} {project} sidecar after replay",
        )
        require(current == original, f"{lane} {project} sidecar changed during replay")
        item = object_json(original, f"{lane} {project} strict sidecar")
        require(
            item["mode"] == lane
            and item["project"] == project
            and item["testStatus"] == "expected"
            and item["attemptStatus"] == "passed"
            and item["dummyProcessExitObserved"] is True
            and item["publicApiParityEvidence"] is False
            and item["browserExecutionEvidence"] is False
            and item["fixtureAdapted"] is True
            and item["upstreamAssertionsModified"] is False,
            f"{lane} {project} v3 outcome/limitations differ",
        )
        evidence[key] = item
    return prior_receipt, prior_summaries, evidence


def verify_successor_lines(
    inventory_lines: Iterable[bytes],
    predecessor_lines: Iterable[bytes],
    successor_lines: Iterable[bytes],
    *,
    lane: str,
    evidence: Mapping[tuple[str, str], Mapping[str, Any]],
    previous_results: Mapping[str, str],
    prior_outcomes: Mapping[str, int],
    prior_scopes: Mapping[str, int],
    expected_count: int = COUNT,
    targets: Mapping[int, tuple[str, str, str]] = TARGETS,
) -> tuple[dict[str, int], dict[str, int], list[dict[str, Any]]]:
    """Require exactly six rerun deltas and byte-identical non-target rows."""
    require(lane in LANES and len(targets) == 6, "lane or target count differs")
    ids = {case[2] for case in targets.values()}
    require(len(ids) == 6, "target IDs duplicate")
    require(
        {(lane, case[0]) for case in targets.values()} <= set(evidence)
        and {case[0] for case in targets.values()} <= set(previous_results),
        "strict target evidence incomplete",
    )
    before: Counter[str] = Counter()
    after: Counter[str] = Counter()
    before_scopes: Counter[str] = Counter()
    after_scopes: Counter[str] = Counter()
    changed: list[dict[str, Any]] = []
    count = 0
    for index, (source_raw, old_raw, new_raw) in enumerate(
        zip_longest(inventory_lines, predecessor_lines, successor_lines)
    ):
        require(
            source_raw is not None and old_raw is not None and new_raw is not None,
            f"{lane} row count differs at {index}",
        )
        require(
            all(raw.endswith(b"\n") for raw in (source_raw, old_raw, new_raw)),
            f"{lane} unterminated row {index}",
        )
        source = object_json(source_raw, f"inventory row {index}")
        old = object_json(old_raw, f"v9 {lane} row {index}")
        require(
            all(old.get(key) == value for key, value in source.items())
            and old.get("outcome") in {"expected", "unexpected", "skipped", "not-run"}
            and type(old.get("attempts")) is int
            and old["attempts"] >= 0,
            f"{lane} predecessor identity/outcome differs at {index}",
        )
        before[old["outcome"]] += 1
        if "evidenceScope" in old:
            require(isinstance(old["evidenceScope"], str), "v9 scope malformed")
            before_scopes[old["evidenceScope"]] += 1
        if index in targets:
            project, engine, spec_id = targets[index]
            sidecar = evidence[(lane, project)]
            old_status = "unexpected" if lane == "candidate" else "expected"
            require(
                source == target_source(project, spec_id)
                and old
                == {
                    **source,
                    "attempts": 1,
                    "evidenceScope": V9_SCOPE,
                    "outcome": old_status,
                    "publicApiParityEvidence": False,
                    "resultFile": previous_results[project],
                }
                and sidecar["project"] == project
                and sidecar["engine"] == engine
                and sidecar["index"] == index
                and sidecar["specId"] == spec_id
                and sidecar["mode"] == lane
                and sidecar["testStatus"] == "expected",
                f"{lane} target predecessor/sidecar identity differs at {index}",
            )
            rerun = {
                **old,
                "attempts": 2,
                "evidenceScope": SCOPE,
                "outcome": "expected",
                "resultFile": sidecar["result"],
            }
            require(
                new_raw == canonical_line(rerun) and new_raw != old_raw,
                f"{lane} target row {index} is not the exact allowed delta",
            )
            changed.append(
                {
                    "index": index,
                    "specId": spec_id,
                    "oldRowSha256": digest(old_raw),
                    "newRowSha256": digest(new_raw),
                    "oldOutcome": old_status,
                    "newOutcome": "expected",
                }
            )
            after["expected"] += 1
            after_scopes[SCOPE] += 1
        else:
            require(old_raw == new_raw, f"{lane} non-target row {index} changed")
            require(old.get("specId") not in ids, f"{lane} target ID at wrong row")
            after[old["outcome"]] += 1
            if "evidenceScope" in old:
                after_scopes[old["evidenceScope"]] += 1
        count += 1
    require(
        count == expected_count
        and [row["index"] for row in changed] == sorted(targets),
        f"{lane} row count or target positions differ",
    )
    expected_outcomes = Counter(prior_outcomes)
    expected_outcomes["unexpected" if lane == "candidate" else "expected"] -= 6
    expected_outcomes["expected"] += 6
    expected_scopes = Counter(prior_scopes)
    expected_scopes[V9_SCOPE] -= 6
    expected_scopes[SCOPE] += 6
    expected_scopes = Counter(
        {key: count for key, count in expected_scopes.items() if count}
    )
    require(
        dict(before) == dict(prior_outcomes)
        and dict(before_scopes) == dict(prior_scopes)
        and prior_scopes.get(V9_SCOPE) == 6
        and before["unexpected" if lane == "candidate" else "expected"] >= 6
        and after == expected_outcomes
        and after_scopes == expected_scopes,
        f"{lane} outcome/scope arithmetic differs",
    )
    return dict(sorted(after.items())), dict(sorted(after_scopes.items())), changed


def expected_header(
    evidence: Mapping[tuple[str, str], Mapping[str, Any]],
) -> dict[str, Any]:
    require(
        all(
            evidence[(lane, project)]["testStatus"] == "expected"
            for lane in LANES
            for project, _, _ in TARGETS.values()
        ),
        "v3 rerun statuses are not all expected",
    )
    require(
        PROMOTER.is_file() and not PROMOTER.is_symlink(),
        "v10 promoter missing or linked",
    )
    return {
        "schema": 1,
        "scope": SCOPE,
        "promotionKind": "public-api-candidate-official-rerun-outcomes",
        "sourceClassificationPreserved": "public-api-candidate",
        "publicApiParityEvidence": False,
        "browserExecutionEvidence": False,
        "bidiProtocolParityEvidence": False,
        "fixtureAdapted": True,
        "upstreamAssertionsModified": False,
        "selectedExecutable": "pinned-exit-1-node-asset",
        "promoterSourceSha256": digest(PROMOTER.read_bytes()),
        "pairedVerifierSha256": digest(Path(__file__).read_bytes()),
        "strictCaptureSha256": STRICT_CAPTURE_SHA256,
        "strictVerifierSha256": STRICT_VERIFIER_SHA256,
        "strictManifestSha256": MANIFEST_SHA256,
        "priorVerifierSha256": V9_VERIFIER_SHA256,
        "priorBundle": str(V9),
        "priorBundleReceiptSha256": V9_RECEIPT_SHA256,
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "recordsPerLane": COUNT,
        "changedRowsPerLane": 6,
        "unchangedRawRowsPerLane": COUNT - 6,
        "targetZeroBasedIndexes": list(TARGETS),
        "targetSpecIds": [case[2] for case in TARGETS.values()],
        "upstreamCommit": COMMIT,
        "selectedUpstreamSpecSha256": SOURCE_SHA256,
        "candidateGreenRows": 6,
        "candidateRedRows": 0,
        "oracleGreenRows": 6,
    }


def strict_provenance(
    lane: str, evidence: Mapping[tuple[str, str], Mapping[str, Any]]
) -> dict[str, Any]:
    result = {}
    for project, _, _ in TARGETS.values():
        strict = evidence[(lane, project)]
        result[project] = {
            "strictSidecar": str(sidecar_path(lane, project)),
            "strictSidecarSha256": SIDECAR_SHA256[(lane, project)],
            "runId": strict["runId"],
            "officialResult": strict["result"],
            "officialResultSha256": strict["resultSha256"],
            "launchReceiptSha256": strict["launchReceiptSha256"],
            "profileReceiptSha256": strict["profileReceiptSha256"],
            "processReceiptSha256": strict["processReceiptSha256"],
            "logSha256": strict["logSha256"],
            "testStatus": strict["testStatus"],
            "attemptStatus": strict["attemptStatus"],
            "dummyProcessExitObserved": strict["dummyProcessExitObserved"],
            "candidateSourceSha256": strict["candidateSourceSha256"],
            "candidateArtifactSha256": strict["candidateArtifactSha256"],
        }
    return result


def expected_lane(
    lane: str,
    evidence: Mapping[tuple[str, str], Mapping[str, Any]],
    predecessor: Mapping[str, Any],
    new_bytes: bytes,
    summary_bytes: bytes,
    outcomes: Mapping[str, int],
    changed: list[dict[str, Any]],
) -> dict[str, Any]:
    return {
        "base": str(V9 / f"{lane}.jsonl"),
        "baseSha256": V9_LEDGER_SHA256[lane],
        "baseSummarySha256": V9_SUMMARY_SHA256[lane],
        "ledger": str(V10 / f"{lane}.jsonl"),
        "ledgerSha256": digest(new_bytes),
        "summarySha256": digest(summary_bytes),
        "oldOutcomeByProject": {
            project: "unexpected" if lane == "candidate" else "expected"
            for project, _, _ in TARGETS.values()
        },
        "newOutcomesByProject": {
            project: evidence[(lane, project)]["testStatus"]
            for project, _, _ in TARGETS.values()
        },
        "changedRows": changed,
        "promotedRows": 6,
        "unchangedRawRows": COUNT - 6,
        "outcomes": dict(outcomes),
        "priorStrictEvidence": predecessor["strictEvidence"],
        "strictEvidence": strict_provenance(lane, evidence),
    }


def verify_payloads(
    files: Mapping[str, bytes],
    *,
    checked_inputs: tuple[
        dict[str, Any], dict[str, dict[str, Any]], dict[tuple[str, str], dict[str, Any]]
    ]
    | None = None,
) -> dict[str, Any]:
    require(set(files) == EXPECTED_FILES, "v10 file set differs")
    prior_receipt, prior_summaries, evidence = checked_inputs or verify_inputs()
    receipt = object_json(files["promotion-receipt.json"], "v10 receipt")
    require(set(receipt) == RECEIPT_FIELDS, "v10 receipt field set differs")
    require(
        {key: value for key, value in receipt.items() if key != "lanes"}
        == expected_header(evidence)
        and isinstance(receipt["lanes"], dict)
        and set(receipt["lanes"]) == set(LANES)
        and files["promotion-receipt.json"]
        == (json.dumps(receipt, indent=2, sort_keys=True) + "\n").encode(),
        "v10 receipt header or canonical bytes differ",
    )
    inventory_lines = sealed(
        INVENTORY, INVENTORY_SHA256, "Core inventory during v10 replay"
    ).splitlines(keepends=True)
    for lane in LANES:
        old_bytes = sealed(
            V9 / f"{lane}.jsonl",
            V9_LEDGER_SHA256[lane],
            f"v9 {lane} ledger during v10 replay",
        )
        new_bytes = files[f"{lane}.jsonl"]
        previous = prior_receipt["lanes"][lane]
        previous_results = {
            project: prior["officialResult"]
            for project, prior in previous["strictEvidence"].items()
        }
        outcomes, scopes, changed = verify_successor_lines(
            inventory_lines,
            old_bytes.splitlines(keepends=True),
            new_bytes.splitlines(keepends=True),
            lane=lane,
            evidence=evidence,
            previous_results=previous_results,
            prior_outcomes=prior_summaries[lane]["outcomes"],
            prior_scopes=prior_summaries[lane]["explicitEvidenceScopes"],
        )
        summary = {
            "schema": 1,
            "inventory": str(INVENTORY),
            "records": COUNT,
            "outcomes": outcomes,
            "explicitEvidenceScopes": scopes,
            "ledgerSha256": digest(new_bytes),
            "ledger": str(V10 / f"{lane}.jsonl"),
        }
        summary_bytes = (json.dumps(summary, indent=2) + "\n").encode()
        require(
            files[f"{lane}.summary.json"] == summary_bytes,
            f"{lane} v10 summary differs from independently counted rows",
        )
        require(
            receipt["lanes"][lane]
            == expected_lane(
                lane, evidence, previous, new_bytes, summary_bytes, outcomes, changed
            )
            and previous["ledgerSha256"] == V9_LEDGER_SHA256[lane],
            f"{lane} v10 receipt differs from sidecars and row hashes",
        )
    return receipt


def verify_existing() -> dict[str, Any]:
    require(
        V10.is_dir() and not V10.is_symlink(),
        "immutable v10 successor missing or linked",
    )
    require(
        {entry.name for entry in V10.iterdir()} == EXPECTED_FILES,
        "immutable v10 file set differs",
    )
    files = {}
    for name in EXPECTED_FILES:
        path = V10 / name
        require(
            path.is_file() and not path.is_symlink(),
            f"immutable v10 {name} missing or linked",
        )
        files[name] = path.read_bytes()
    return verify_payloads(files)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    try:
        receipt = verify_existing()
    except (
        AssertionError,
        OSError,
        ValueError,
        KeyError,
        TypeError,
        json.JSONDecodeError,
    ) as error:
        parser.exit(2, f"Core v10 paired successor invalid: {error}\n")
    print(
        json.dumps(
            {
                "scope": receipt["scope"],
                "outcomes": {
                    lane: receipt["lanes"][lane]["outcomes"] for lane in LANES
                },
                "ledgerSha256": {
                    lane: receipt["lanes"][lane]["ledgerSha256"] for lane in LANES
                },
            },
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
