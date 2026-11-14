#!/usr/bin/env python3
"""Independently replay the six-row Core public-launch v8-to-v9 successor.

The six official results may differ between candidate projects. Every other
raw ledger line must remain identical to the sealed v8 predecessor. This
module never imports the promoter or treats source classification as parity.
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
RESULTS = ROOT / ".work/official-conformance/results/core-dummy-node-bidi-strict-v2"
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
MANIFEST = ROOT / "tests/official/playwright-core-dummy-node-bidi-strict-v2-manifest.json"
STRICT_CAPTURE = ROOT / "scripts/run_playwright_core_dummy_node_bidi_strict_v2.py"
STRICT_VERIFIER = ROOT / "scripts/verify_playwright_core_dummy_node_bidi_strict_v2.py"
V8_VERIFIER = (
    ROOT / "scripts/verify_playwright_core_server_connect_close_error_bidi_chromium_paired_v8.py"
)
PROMOTER = ROOT / "scripts/promote_playwright_core_dummy_node_bidi_strict_v9.py"
COUNT = 48_477
LANES = ("candidate", "oracle")
SCOPE = "official-browser-free-browsertype-launch-dummy-node-bidi-config-outcomes-v9"
V8_SCOPE = (
    "official-browser-free-server-connect-close-error-bidi-chromium-"
    "fixture-host-adapted-internal-diagnostic-green-v8"
)
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
V8_RECEIPT_SHA256 = "4c0a72a233440263a38bf718fa003fa2e8f8c36ef0243aaac9811c314e14dd68"
V8_VERIFIER_SHA256 = "d9396a008572d0e70412ce3afcd3b9001e46ea66fc714edc6f463d49e1a3a321"
V8_LEDGER_SHA256 = {
    "candidate": "d41e853872aaed73e303184909b9323e7229e1e5068307ac831419bf7b5ab1d6",
    "oracle": "c6fd28c4db0184f5821489907e5b489f500f2752c34a5d097ade1b68f5f565b4",
}
V8_SUMMARY_SHA256 = {
    "candidate": "fb81213cf7674ba26ba170f5216afc1143c72456ba5873da61132d652cb15339",
    "oracle": "10b68d822d275d0b2a916ee559048ce44fb646a388a82ef40f97ef40a3805f86",
}
MANIFEST_SHA256 = "670c6a4b2b015c3ffb4bf5ec26b1d37f7ee623a9eccafacb2bab4d82c420fcd2"
SOURCE_SHA256 = "64fe24188ec96bb0d5d1ecca5af37986b16d82451b91e4db80593f707126bd05"
STRICT_CAPTURE_SHA256 = "5a30e115437e162d34e591e62d9a98002a427c6d4f3e6196378c7f7451a42ff9"
STRICT_VERIFIER_SHA256 = "2175c2599c7112cb8dc89d16557750f8b47c4bbd946fb1781112a651d8db62a7"
TARGETS = {
    6110: ("bidi-chrome-canary-library", "chromium", "d1a604bdf6287c6776f3-53abd42f8ece2b451e5f"),
    6111: ("bidi-chrome-stable-library", "chromium", "d1a604bdf6287c6776f3-ee1b24a2e94e134f0b8a"),
    6112: ("bidi-chromium-library", "chromium", "d1a604bdf6287c6776f3-f708c432fe2c80d2440a"),
    6113: ("moz-firefox-beta-library", "firefox", "d1a604bdf6287c6776f3-a1c0f40a45acfee26558"),
    6114: ("moz-firefox-library", "firefox", "d1a604bdf6287c6776f3-6ecf3d1ccae0b454c888"),
    6115: ("moz-firefox-nightly-library", "firefox", "d1a604bdf6287c6776f3-88b0839d68b9d44cb588"),
}
SIDECAR_SHA256 = {
    (
        "candidate",
        "bidi-chrome-canary-library",
    ): "393378dd79114d4729cabd8b221a51526a646de9ebd28724f7c021b17073d136",
    (
        "candidate",
        "bidi-chrome-stable-library",
    ): "de474975f21e21e4b583e7b9b6d3ce479208899f90bb1f738beb295e9278ece0",
    (
        "candidate",
        "bidi-chromium-library",
    ): "1f0e428d0cc00b187058ad10fc95e39b4c461b599b037e9fb1da405481bb4abe",
    (
        "candidate",
        "moz-firefox-beta-library",
    ): "c1e8853f3e83f17a07f9186262fda67a80a425d5816c03136dded21ddfe2a11a",
    (
        "candidate",
        "moz-firefox-library",
    ): "225788ef9dffdab020e41a26ed7cdcac0759c9edee0a848b05082a4b0b974115",
    (
        "candidate",
        "moz-firefox-nightly-library",
    ): "8eb1c864e1b84c9cafb39e62e5b1c778cb7f6283ce75f3fe841164b785a5cd88",
    (
        "oracle",
        "bidi-chrome-canary-library",
    ): "bf3d624580314ccd47a6792a0ca635cd20920524a3a44d8e6f61e43f2136de6a",
    (
        "oracle",
        "bidi-chrome-stable-library",
    ): "ebd041dc23702dbc5948f950308fd6b1de61fed16a63a680f093c704a34c8b5f",
    (
        "oracle",
        "bidi-chromium-library",
    ): "854eb6257b333e649be508668229bdd62103eb879aabfbaeeb122ba359ab760f",
    (
        "oracle",
        "moz-firefox-beta-library",
    ): "7a87a0e997bcb313eae4d7bada2e226f63b23622fea88f9692580b813f2133c6",
    (
        "oracle",
        "moz-firefox-library",
    ): "662d8942bbfd089985576921249c89730ab256c959a4dace75eb73119e3ac3d3",
    (
        "oracle",
        "moz-firefox-nightly-library",
    ): "d08f792ec715c6ce02faec30275987fede2ce08b8f0968f93da6fbb3885bffd4",
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
    "priorBundle",
    "priorBundleReceiptSha256",
    "inventory",
    "inventorySha256",
    "recordsPerLane",
    "changedRowsPerLane",
    "targetZeroBasedIndexes",
    "targetSpecIds",
    "upstreamCommit",
    "selectedUpstreamSpecSha256",
    "candidateGreenRows",
    "candidateRedRows",
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


def sidecar_path(lane: str, project: str) -> Path:
    require((lane, project) in SIDECAR_SHA256, "unknown strict case")
    return RESULTS / f"oct20-v2-{lane}-{project}.strict.json"


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


def verify_inputs() -> tuple[
    dict[str, Any], dict[str, dict[str, Any]], dict[tuple[str, str], dict[str, Any]]
]:
    """Check the full v8 chain and all twelve immutable strict v2 sidecars."""
    require(not sys.flags.optimize, "paired Core verification requires assertions enabled")
    sealed(INVENTORY, INVENTORY_SHA256, "Core inventory")
    sealed(V8_VERIFIER, V8_VERIFIER_SHA256, "v8 independent verifier")
    sealed(V8 / "promotion-receipt.json", V8_RECEIPT_SHA256, "v8 receipt")
    for lane in LANES:
        sealed(V8 / f"{lane}.jsonl", V8_LEDGER_SHA256[lane], f"v8 {lane} ledger")
        sealed(V8 / f"{lane}.summary.json", V8_SUMMARY_SHA256[lane], f"v8 {lane} summary")
    v8 = importlib.import_module(
        "verify_playwright_core_server_connect_close_error_bidi_chromium_paired_v8"
    )
    prior_receipt = v8.verify_existing()
    require(
        prior_receipt["scope"] == V8_SCOPE
        and prior_receipt["recordsPerLane"] == COUNT
        and prior_receipt["publicApiParityEvidence"] is False
        and prior_receipt["lanes"]["candidate"]["ledgerSha256"] == V8_LEDGER_SHA256["candidate"]
        and prior_receipt["lanes"]["oracle"]["ledgerSha256"] == V8_LEDGER_SHA256["oracle"],
        "v8 predecessor identity differs",
    )
    prior_summaries = {
        lane: object_json(
            sealed(V8 / f"{lane}.summary.json", V8_SUMMARY_SHA256[lane], f"v8 {lane} summary"),
            f"v8 {lane} summary",
        )
        for lane in LANES
    }
    manifest = object_json(
        sealed(MANIFEST, MANIFEST_SHA256, "strict v2 manifest"), "strict v2 manifest"
    )
    expected_cases = [
        {"index": index, "project": project, "engine": engine, "specId": spec_id}
        for index, (project, engine, spec_id) in TARGETS.items()
    ]
    require(
        manifest.get("schema") == 1
        and manifest.get("commit") == "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
        and manifest.get("selector") == "tests/library/browsertype-launch.spec.ts:64"
        and manifest.get("cases") == expected_cases
        and manifest.get("v8CandidateLedgerSha256") == V8_LEDGER_SHA256["candidate"]
        and manifest.get("v8OracleLedgerSha256") == V8_LEDGER_SHA256["oracle"]
        and manifest.get("v8ReceiptSha256") == V8_RECEIPT_SHA256
        and manifest.get("upstreamSources", {}).get("tests/library/browsertype-launch.spec.ts")
        == SOURCE_SHA256,
        "strict v2 manifest target/predecessor identity differs",
    )
    sealed(STRICT_CAPTURE, STRICT_CAPTURE_SHA256, "strict v2 capture source")
    sealed(STRICT_VERIFIER, STRICT_VERIFIER_SHA256, "strict v2 verifier source")
    strict = importlib.import_module("verify_playwright_core_dummy_node_bidi_strict_v2")
    paths = [sidecar_path(lane, project) for lane in LANES for project, _, _ in TARGETS.values()]
    originals = {
        (lane, project): sealed(
            sidecar_path(lane, project),
            SIDECAR_SHA256[(lane, project)],
            f"{lane} {project} sidecar",
        )
        for lane in LANES
        for project, _, _ in TARGETS.values()
    }
    strict.verify_all(paths)
    sealed(INVENTORY, INVENTORY_SHA256, "Core inventory after strict replay")
    sealed(V8 / "promotion-receipt.json", V8_RECEIPT_SHA256, "v8 receipt after strict replay")
    for lane in LANES:
        sealed(
            V8 / f"{lane}.jsonl", V8_LEDGER_SHA256[lane], f"v8 {lane} ledger after strict replay"
        )
        sealed(
            V8 / f"{lane}.summary.json",
            V8_SUMMARY_SHA256[lane],
            f"v8 {lane} summary after strict replay",
        )
    sealed(MANIFEST, MANIFEST_SHA256, "strict v2 manifest after replay")
    sealed(STRICT_CAPTURE, STRICT_CAPTURE_SHA256, "strict v2 capture source after replay")
    sealed(STRICT_VERIFIER, STRICT_VERIFIER_SHA256, "strict v2 verifier source after replay")
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
            and item["testStatus"]
            in (("expected", "unexpected") if lane == "candidate" else ("expected",))
            and item["publicApiParityEvidence"] is False
            and item["browserExecutionEvidence"] is False
            and item["fixtureAdapted"] is True
            and item["upstreamAssertionsModified"] is False,
            f"{lane} {project} strict outcome/limitations differ",
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
    prior_outcomes: Mapping[str, int],
    prior_scopes: Mapping[str, int],
    expected_count: int = COUNT,
    targets: Mapping[int, tuple[str, str, str]] = TARGETS,
) -> tuple[dict[str, int], dict[str, int], list[dict[str, Any]]]:
    """Require exactly six canonical outcome deltas and 48,471 identical lines."""
    require(lane in LANES and len(targets) == 6, "lane or target count differs")
    ids = {case[2] for case in targets.values()}
    require(len(ids) == 6, "target IDs duplicate")
    require(
        {(lane, case[0]) for case in targets.values()} <= set(evidence),
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
        old = object_json(old_raw, f"v8 {lane} row {index}")
        require(
            all(old.get(key) == value for key, value in source.items())
            and old.get("outcome") in {"expected", "unexpected", "skipped", "not-run"}
            and type(old.get("attempts")) is int
            and old["attempts"] >= 0,
            f"{lane} predecessor identity/outcome differs at {index}",
        )
        before[old["outcome"]] += 1
        if "evidenceScope" in old:
            require(isinstance(old["evidenceScope"], str), "v8 scope malformed")
            before_scopes[old["evidenceScope"]] += 1
        if index in targets:
            project, engine, spec_id = targets[index]
            sidecar = evidence[(lane, project)]
            require(
                source == target_source(project, spec_id)
                and old == {**source, "attempts": 0, "outcome": "not-run"}
                and sidecar["project"] == project
                and sidecar["engine"] == engine
                and sidecar["index"] == index
                and sidecar["specId"] == spec_id
                and sidecar["mode"] == lane,
                f"{lane} target identity/classification differs at {index}",
            )
            status = sidecar["testStatus"]
            require(
                status in (("expected", "unexpected") if lane == "candidate" else ("expected",)),
                f"{lane} official status is ineligible at {index}",
            )
            promoted = {
                **old,
                "attempts": 1,
                "evidenceScope": SCOPE,
                "outcome": status,
                "publicApiParityEvidence": False,
                "resultFile": sidecar["result"],
            }
            canonical = (
                json.dumps(promoted, sort_keys=True, separators=(",", ":")) + "\n"
            ).encode()
            require(
                new_raw == canonical and new_raw != old_raw,
                f"{lane} target row {index} is not the exact allowed delta",
            )
            changed.append(
                {
                    "index": index,
                    "specId": spec_id,
                    "oldRowSha256": digest(old_raw),
                    "newRowSha256": digest(new_raw),
                    "outcome": status,
                }
            )
            after[status] += 1
            after_scopes[SCOPE] += 1
        else:
            require(old_raw == new_raw, f"{lane} non-target row {index} changed")
            require(old.get("specId") not in ids, f"{lane} target ID at wrong row")
            after[old["outcome"]] += 1
            if "evidenceScope" in old:
                after_scopes[old["evidenceScope"]] += 1
        count += 1
    require(
        count == expected_count and [row["index"] for row in changed] == sorted(targets),
        f"{lane} row count or target positions differ",
    )
    expected_outcomes = Counter(prior_outcomes)
    expected_outcomes["not-run"] -= 6
    for project, _, _ in targets.values():
        expected_outcomes[evidence[(lane, project)]["testStatus"]] += 1
    expected_scopes = Counter(prior_scopes)
    expected_scopes[SCOPE] += 6
    require(
        dict(before) == dict(prior_outcomes)
        and dict(before_scopes) == dict(prior_scopes)
        and prior_scopes.get(SCOPE, 0) == 0
        and before["not-run"] >= 6
        and after == expected_outcomes
        and after_scopes == expected_scopes,
        f"{lane} outcome/scope arithmetic differs",
    )
    return dict(sorted(after.items())), dict(sorted(after_scopes.items())), changed


def expected_header(evidence: Mapping[tuple[str, str], Mapping[str, Any]]) -> dict[str, Any]:
    statuses = [
        evidence[("candidate", project)]["testStatus"] for project, _, _ in TARGETS.values()
    ]
    require(
        all(status in ("expected", "unexpected") for status in statuses),
        "candidate statuses invalid",
    )
    require(PROMOTER.is_file() and not PROMOTER.is_symlink(), "v9 promoter missing or linked")
    return {
        "schema": 1,
        "scope": SCOPE,
        "promotionKind": "public-api-candidate-official-outcomes",
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
        "priorBundle": str(V8),
        "priorBundleReceiptSha256": V8_RECEIPT_SHA256,
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "recordsPerLane": COUNT,
        "changedRowsPerLane": 6,
        "targetZeroBasedIndexes": list(TARGETS),
        "targetSpecIds": [case[2] for case in TARGETS.values()],
        "upstreamCommit": "961381ec73a9fa2661d92504eab1a6bd6eaffc75",
        "selectedUpstreamSpecSha256": SOURCE_SHA256,
        "candidateGreenRows": statuses.count("expected"),
        "candidateRedRows": statuses.count("unexpected"),
    }


def expected_lane(
    lane: str,
    evidence: Mapping[tuple[str, str], Mapping[str, Any]],
    new_bytes: bytes,
    summary_bytes: bytes,
    outcomes: Mapping[str, int],
    changed: list[dict[str, Any]],
) -> dict[str, Any]:
    strict_evidence = {}
    for project, _, _ in TARGETS.values():
        strict = evidence[(lane, project)]
        strict_evidence[project] = {
            "strictSidecar": str(sidecar_path(lane, project)),
            "strictSidecarSha256": SIDECAR_SHA256[(lane, project)],
            "runId": strict["runId"],
            "officialResult": strict["result"],
            "officialResultSha256": strict["resultSha256"],
            "launchReceiptSha256": strict["launchReceiptSha256"],
            "profileReceiptSha256": strict["profileReceiptSha256"],
            "processReceiptSha256": strict["processReceiptSha256"],
            "testStatus": strict["testStatus"],
            "attemptStatus": strict["attemptStatus"],
            "dummyProcessExitObserved": strict["dummyProcessExitObserved"],
            "candidateSourceSha256": strict["candidateSourceSha256"],
            "candidateArtifactSha256": strict["candidateArtifactSha256"],
        }
    return {
        "base": str(V8 / f"{lane}.jsonl"),
        "baseSha256": V8_LEDGER_SHA256[lane],
        "baseSummarySha256": V8_SUMMARY_SHA256[lane],
        "ledger": str(V9 / f"{lane}.jsonl"),
        "ledgerSha256": digest(new_bytes),
        "summarySha256": digest(summary_bytes),
        "oldOutcome": "not-run",
        "newOutcomesByProject": {
            project: evidence[(lane, project)]["testStatus"] for project, _, _ in TARGETS.values()
        },
        "changedRows": changed,
        "promotedRows": 6,
        "outcomes": dict(outcomes),
        "strictEvidence": strict_evidence,
    }


def verify_payloads(
    files: Mapping[str, bytes],
    *,
    checked_inputs: tuple[
        dict[str, Any], dict[str, dict[str, Any]], dict[tuple[str, str], dict[str, Any]]
    ]
    | None = None,
) -> dict[str, Any]:
    """Verify candidate bytes without relying on the producer's transforms."""
    require(set(files) == EXPECTED_FILES, "v9 file set differs")
    prior_receipt, prior_summaries, evidence = checked_inputs or verify_inputs()
    receipt = object_json(files["promotion-receipt.json"], "v9 receipt")
    require(set(receipt) == RECEIPT_FIELDS, "v9 receipt field set differs")
    require(
        {key: value for key, value in receipt.items() if key != "lanes"}
        == expected_header(evidence)
        and isinstance(receipt["lanes"], dict)
        and set(receipt["lanes"]) == set(LANES)
        and files["promotion-receipt.json"]
        == (json.dumps(receipt, indent=2, sort_keys=True) + "\n").encode(),
        "v9 receipt header or canonical bytes differ",
    )
    inventory_lines = sealed(
        INVENTORY, INVENTORY_SHA256, "Core inventory during v9 replay"
    ).splitlines(keepends=True)
    for lane in LANES:
        old_bytes = sealed(
            V8 / f"{lane}.jsonl", V8_LEDGER_SHA256[lane], f"v8 {lane} ledger during v9 replay"
        )
        new_bytes = files[f"{lane}.jsonl"]
        prior = prior_summaries[lane]
        outcomes, scopes, changed = verify_successor_lines(
            inventory_lines,
            old_bytes.splitlines(keepends=True),
            new_bytes.splitlines(keepends=True),
            lane=lane,
            evidence=evidence,
            prior_outcomes=prior["outcomes"],
            prior_scopes=prior["explicitEvidenceScopes"],
        )
        summary = {
            "schema": 1,
            "inventory": str(INVENTORY),
            "records": COUNT,
            "outcomes": outcomes,
            "explicitEvidenceScopes": scopes,
            "ledgerSha256": digest(new_bytes),
            "ledger": str(V9 / f"{lane}.jsonl"),
        }
        summary_bytes = (json.dumps(summary, indent=2) + "\n").encode()
        require(
            files[f"{lane}.summary.json"] == summary_bytes,
            f"{lane} v9 summary differs from independently counted rows",
        )
        require(
            receipt["lanes"][lane]
            == expected_lane(lane, evidence, new_bytes, summary_bytes, outcomes, changed)
            and prior_receipt["lanes"][lane]["ledgerSha256"] == V8_LEDGER_SHA256[lane],
            f"{lane} v9 receipt differs from sidecars and row hashes",
        )
    return receipt


def verify_existing() -> dict[str, Any]:
    require(V9.is_dir() and not V9.is_symlink(), "immutable v9 successor missing or linked")
    require(
        {entry.name for entry in V9.iterdir()} == EXPECTED_FILES, "immutable v9 file set differs"
    )
    files = {}
    for name in EXPECTED_FILES:
        path = V9 / name
        require(path.is_file() and not path.is_symlink(), f"immutable v9 {name} missing or linked")
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
        parser.exit(2, f"Core v9 paired successor invalid: {error}\n")
    print(
        json.dumps(
            {
                "scope": receipt["scope"],
                "outcomes": {lane: receipt["lanes"][lane]["outcomes"] for lane in LANES},
                "ledgerSha256": {lane: receipt["lanes"][lane]["ledgerSha256"] for lane in LANES},
            },
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
