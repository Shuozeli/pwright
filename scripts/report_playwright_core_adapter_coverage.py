#!/usr/bin/env python3
"""Audit runnable routes for every pinned Playwright Core inventory row.

This is a coverage report, not an official test run or an applicability ruling.
It never turns a missing adapter into a passing or non-applicable outcome.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter, defaultdict
from collections.abc import Iterator
from itertools import zip_longest
from pathlib import Path
from typing import cast

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_MANIFEST = ROOT / "tests/official/playwright-core-manifest.json"
STRICT_MANIFEST = (
    ROOT / "tests/official/playwright-core-dummy-node-launch-strict-v1-manifest.json"
)
SUCCESSOR_MANIFEST = (
    ROOT
    / "tests/official/playwright-core-dummy-node-other-engines-strict-v1-manifest.json"
)
COHORT_MANIFESTS = {
    "bidi-executable-path": ROOT
    / "tests/official/playwright-core-skip-accounting-bidi-executable-path-oct20-manifest.json",
    "firefox": ROOT
    / "tests/official/playwright-core-skip-accounting-firefox-library-oct20-manifest.json",
    "launcher-macos": ROOT
    / "tests/official/playwright-core-skip-accounting-launcher-macos-oct20-manifest.json",
}
DEFAULT_INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
DEFAULT_PAIR = (
    ROOT
    / ".work/official-conformance/outcomes/core-dummy-node-other-engines-strict-paired-v1"
)
PRIOR_PAIR_NAME = "core-upstream-skips-strict-paired-v1"
BASE_PAIR_NAME = "core-dummy-node-launch-strict-paired-v1"
EXPECTED_COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
EXPECTED_MANIFEST_SHA256 = (
    "b0345cbfeec1aafa02265759720b6bb8991fa72c238d798e497d2ace42f52c16"
)
EXPECTED_RECEIPT_SHA256 = (
    "27131a3e31ff27d3a7dd676f97581c8fcdf09e19a76647d3ffc83d689edf15c5"
)
EXPECTED_PRIOR_RECEIPT_SHA256 = (
    "d2e85289725b8889666174680f16b0b6f83c12e359593454bc956a7fef139c9e"
)
EXPECTED_BASE_RECEIPT_SHA256 = (
    "ea1bb96b3bc5c2eb56082c6abe67e0e7fcf2e7e3a17c0df10946ab67feeaa013"
)
EXPECTED_ROWS = 48_477
SOURCE_FIELDS = frozenset(
    {
        "classificationReasons",
        "column",
        "expectedStatus",
        "file",
        "line",
        "projectId",
        "projectName",
        "role",
        "sourceClassification",
        "sourceFile",
        "specId",
        "suite",
        "titlePath",
    }
)
CLASSIFICATIONS = frozenset(
    {"public-api-candidate", "implementation-internal", "product-infrastructure"}
)
OUTCOMES = frozenset({"expected", "unexpected", "skipped", "not-run"})

# Explicit project matrix for the pinned v1.58.0 inventory. Unknown projects
# fail closed; a channel name alone is never evidence of runtime parity.
PROJECTS: dict[str, dict[str, tuple[str, str, str]]] = {
    "android": {
        "android-native": ("android-device", "android", "Android device/emulator"),
        "android-page": ("android-device", "android", "Android page fixture"),
    },
    "bidi": {
        **{
            f"{channel}-{scope}": ("webdriver-bidi", channel, engine)
            for channel, engine in (
                ("bidi-chromium", "chromium"),
                ("bidi-chrome-canary", "chromium"),
                ("bidi-chrome-stable", "chromium"),
                ("moz-firefox", "firefox"),
                ("moz-firefox-beta", "firefox"),
                ("moz-firefox-nightly", "firefox"),
            )
            for scope in ("library", "page")
        },
    },
    "electron": {
        "electron-api": ("electron-app", "electron", "Electron application"),
        "electron-page": ("electron-app", "electron", "Electron page fixture"),
    },
    "installation": {
        "installation tests": ("package-install", "none", "package-manager fixture"),
    },
    "library": {
        f"{engine}-{scope}": (
            "chromium-cdp" if engine == "chromium" else "upstream-browser-transport",
            engine,
            engine,
        )
        for engine in ("chromium", "firefox", "webkit")
        for scope in ("library", "page")
    },
    "mcp": {
        engine: ("mcp-browser-runtime", engine, engine)
        for engine in ("chrome", "chromium", "firefox", "webkit")
    },
    "playwright-test": {
        "image_tools": ("test-runner-internal", "none", "image tools"),
        "playwright-test": ("test-runner-internal", "none", "test runner"),
    },
    "stress": {
        engine: ("browser-stress", engine, engine)
        for engine in ("chromium", "firefox", "webkit")
    },
    "components-aggregate": {
        "": ("component-test-aggregate", "none", "component runner"),
    },
    "example-github-api": {"": ("example-fixture", "none", "GitHub API example")},
    "example-mock-battery": {"": ("example-fixture", "none", "battery example")},
    "example-mock-filesystem": {"": ("example-fixture", "none", "filesystem example")},
    "example-todomvc": {"chromium": ("example-browser", "chromium", "chromium")},
    "example-svgomg": {
        engine: ("example-browser", engine, engine)
        for engine in ("chromium", "firefox", "webkit")
    },
}
for _component in (
    "component-react-vite",
    "component-react17",
    "component-svelte-vite",
    "component-svelte",
    "component-vue-cli",
    "component-vue-vite",
):
    PROJECTS[_component] = {
        engine: ("component-browser", engine, engine)
        for engine in ("chromium", "firefox", "webkit")
    }


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


JsonObject = dict[str, object]


def json_lines(path: Path) -> Iterator[JsonObject]:
    with path.open(encoding="utf-8") as source:
        for number, line in enumerate(source, 1):
            try:
                row = json.loads(line)
            except json.JSONDecodeError as error:
                raise ValueError(f"{path}:{number}: invalid JSON") from error
            if not isinstance(row, dict):
                raise TypeError(f"{path}:{number}: expected JSON object")
            yield cast(JsonObject, row)


def _load_object(path: Path) -> JsonObject:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise TypeError(f"{path}: expected JSON object")
    return cast(JsonObject, value)


def _validated_inputs(
    manifest_path: Path,
    inventory_path: Path,
    candidate_path: Path,
    oracle_path: Path,
    receipt_path: Path,
    auxiliary_manifest_root: Path | None,
) -> tuple[dict[str, JsonObject], dict[str, str], dict[str, str], JsonObject]:
    strict_manifest_path = (
        auxiliary_manifest_root / STRICT_MANIFEST.name
        if auxiliary_manifest_root is not None
        else STRICT_MANIFEST
    )
    successor_manifest_path = (
        auxiliary_manifest_root / SUCCESSOR_MANIFEST.name
        if auxiliary_manifest_root is not None
        else SUCCESSOR_MANIFEST
    )
    cohort_manifest_paths = (
        {
            name: auxiliary_manifest_root / path.name
            for name, path in COHORT_MANIFESTS.items()
        }
        if auxiliary_manifest_root is not None
        else COHORT_MANIFESTS
    )
    prior_pair = receipt_path.parent.parent / PRIOR_PAIR_NAME
    base_pair = receipt_path.parent.parent / BASE_PAIR_NAME
    prior_receipt_path = prior_pair / "promotion-receipt.json"
    base_receipt_path = base_pair / "promotion-receipt.json"
    prior_lane_paths = {
        lane: prior_pair / f"{lane}.jsonl" for lane in ("candidate", "oracle")
    }
    base_lane_paths = {
        lane: base_pair / f"{lane}.jsonl" for lane in ("candidate", "oracle")
    }
    for path in (
        manifest_path,
        strict_manifest_path,
        successor_manifest_path,
        *cohort_manifest_paths.values(),
        inventory_path,
        candidate_path,
        oracle_path,
        receipt_path,
        prior_receipt_path,
        base_receipt_path,
        *prior_lane_paths.values(),
        *base_lane_paths.values(),
    ):
        if path.is_symlink() or not path.is_file():
            raise ValueError(f"missing or symlinked input: {path}")
    if any(path.is_symlink() for path in (base_pair, prior_pair, receipt_path.parent)):
        raise ValueError("paired ledger bundle directory is symlinked")
    manifest = _load_object(manifest_path)
    successor_manifest = _load_object(successor_manifest_path)
    receipt = _load_object(receipt_path)
    prior_receipt = _load_object(prior_receipt_path)
    base_receipt = _load_object(base_receipt_path)
    if (
        manifest.get("schema"),
        manifest.get("repository"),
        manifest.get("tag"),
        manifest.get("commit"),
    ) != (1, "https://github.com/microsoft/playwright.git", "v1.58.0", EXPECTED_COMMIT):
        raise ValueError("official manifest provenance drifted")
    suites = manifest.get("suites")
    if not isinstance(suites, list) or len(suites) != len(PROJECTS):
        raise ValueError("official suite list is incomplete")
    by_suite: dict[str, JsonObject] = {}
    for suite in suites:
        if not isinstance(suite, dict) or not isinstance(suite.get("id"), str):
            raise TypeError("malformed suite entry")
        suite_id = cast(str, suite["id"])
        if suite_id in by_suite or suite_id not in PROJECTS:
            raise ValueError(f"duplicate or unknown suite: {suite_id}")
        if type(suite.get("expectedTests")) is not int or suite["expectedTests"] < 1:
            raise ValueError(f"invalid expectedTests: {suite_id}")
        for adapter in ("candidateAdapter", "oracleAdapter"):
            if adapter in suite and not isinstance(suite[adapter], str):
                raise ValueError(f"invalid {adapter}: {suite_id}")
        by_suite[suite_id] = cast(JsonObject, suite)
    if (
        set(by_suite) != set(PROJECTS)
        or sum(cast(int, s["expectedTests"]) for s in by_suite.values())
        != EXPECTED_ROWS
    ):
        raise ValueError("manifest suite coverage/count drifted")
    if (
        any(
            suite_id != "library"
            and ("candidateAdapter" in suite or "oracleAdapter" in suite)
            for suite_id, suite in by_suite.items()
        )
        or by_suite["library"].get("candidateAdapter") != "test-mode-playwright-core"
        or by_suite["library"].get("oracleAdapter") != "test-mode-playwright-core-cdp"
    ):
        raise ValueError("adapter matrix changed; update route audit before reporting")

    hashes = {
        "manifest": sha256(manifest_path),
        "strictManifest": sha256(strict_manifest_path),
        "successorManifest": sha256(successor_manifest_path),
        "inventory": sha256(inventory_path),
        "candidate": sha256(candidate_path),
        "oracle": sha256(oracle_path),
        "receipt": sha256(receipt_path),
        "priorReceipt": sha256(prior_receipt_path),
        "baseReceipt": sha256(base_receipt_path),
        "priorCandidate": sha256(prior_lane_paths["candidate"]),
        "priorOracle": sha256(prior_lane_paths["oracle"]),
        "baseCandidate": sha256(base_lane_paths["candidate"]),
        "baseOracle": sha256(base_lane_paths["oracle"]),
    }
    cohort_hashes = {name: sha256(path) for name, path in cohort_manifest_paths.items()}
    hashes.update(
        {f"cohortManifest:{name}": digest for name, digest in cohort_hashes.items()}
    )
    if hashes["manifest"] != EXPECTED_MANIFEST_SHA256:
        raise ValueError("pinned full Core manifest SHA-256 drifted")
    if hashes["receipt"] != EXPECTED_RECEIPT_SHA256:
        raise ValueError("pinned paired receipt SHA-256 drifted")
    if hashes["priorReceipt"] != EXPECTED_PRIOR_RECEIPT_SHA256:
        raise ValueError("pinned prior paired receipt SHA-256 drifted")
    if hashes["baseReceipt"] != EXPECTED_BASE_RECEIPT_SHA256:
        raise ValueError("pinned base paired receipt SHA-256 drifted")
    if (
        successor_manifest.get("schema"),
        successor_manifest.get("repository"),
        successor_manifest.get("tag"),
        successor_manifest.get("commit"),
    ) != (2, "https://github.com/microsoft/playwright.git", "v1.58.0", EXPECTED_COMMIT):
        raise ValueError("strict successor manifest provenance drifted")
    cases = successor_manifest.get("cases")
    if not isinstance(cases, list) or not cases:
        raise ValueError("strict successor manifest cases are incomplete")
    target_cases: dict[str, str] = {}
    for case in cases:
        if not isinstance(case, dict):
            raise TypeError("strict successor manifest case is malformed")
        spec_id, project = case.get("specId"), case.get("project")
        if (
            not isinstance(spec_id, str)
            or not isinstance(project, str)
            or project not in PROJECTS["library"]
            or spec_id in target_cases
        ):
            raise ValueError("strict successor manifest case is malformed")
        target_cases[spec_id] = project
    if (
        receipt.get("schema") != 1
        or receipt.get("recordsPerLane") != EXPECTED_ROWS
        or receipt.get("upstreamCommit") != EXPECTED_COMMIT
        or receipt.get("scope")
        != "official-browser-free-browsertype-dummy-node-other-engines-strict-red-v1"
        or receipt.get("promotionKind") != "public-api-official-red"
        or receipt.get("assertionsExecuted") is not True
        or receipt.get("fixtureAdapted") is not True
        or receipt.get("candidateNativeRuntimeInvoked") is not False
        or receipt.get("publicApiParityEvidence") is not False
        or receipt.get("publicApiBehaviorAttested") is not True
        or receipt.get("candidateRuntimeAttested") is not False
        or receipt.get("promotedRowsPerLane") != len(target_cases)
        or receipt.get("upstreamAssertionsModified") is not False
    ):
        raise ValueError("paired ledger receipt provenance drifted")
    if receipt.get("inventorySha256") != hashes["inventory"]:
        raise ValueError("inventory does not match paired ledger receipt")
    if receipt.get("manifestSha256") != hashes["successorManifest"]:
        raise ValueError("strict successor manifest does not match paired receipt")
    upstream_sources = successor_manifest.get("upstreamSources")
    target_indexes = receipt.get("targetZeroBasedIndexes")
    if (
        not isinstance(upstream_sources, dict)
        or receipt.get("selectedUpstreamSpecSha256")
        != upstream_sources.get("tests/library/browsertype-launch.spec.ts")
        or receipt.get("targetSpecIds") != list(target_cases)
        or not isinstance(target_indexes, list)
        or len(target_indexes) != len(target_cases)
        or any(type(index) is not int or index < 0 for index in target_indexes)
        or len(set(target_indexes)) != len(target_cases)
    ):
        raise ValueError("strict successor target evidence drifted")
    if (
        receipt.get("priorBundleReceiptSha256") != hashes["priorReceipt"]
        or Path(str(receipt.get("priorBundle", ""))).name != PRIOR_PAIR_NAME
    ):
        raise ValueError("prior paired bundle does not match successor receipt")
    if (
        prior_receipt.get("schema") != 1
        or prior_receipt.get("recordsPerLane") != EXPECTED_ROWS
        or prior_receipt.get("upstreamCommit") != EXPECTED_COMMIT
        or prior_receipt.get("inventorySha256") != hashes["inventory"]
        or prior_receipt.get("scope")
        != "official-upstream-skip-accounting-only-strict-v1"
        or prior_receipt.get("promotionKind") != "skip-accounting-only"
        or prior_receipt.get("skipAccountingOnly") is not True
        or prior_receipt.get("publicApiParityEvidence") is not False
        or prior_receipt.get("publicApiBehaviorAttested") is not False
        or prior_receipt.get("candidateRuntimeAttested") is not False
    ):
        raise ValueError("prior paired ledger receipt provenance drifted")
    if (
        prior_receipt.get("priorBundleReceiptSha256") != hashes["baseReceipt"]
        or Path(str(prior_receipt.get("priorBundle", ""))).name != BASE_PAIR_NAME
    ):
        raise ValueError("base paired bundle does not match skip receipt")
    if (
        base_receipt.get("schema") != 1
        or base_receipt.get("recordsPerLane") != EXPECTED_ROWS
        or base_receipt.get("upstreamCommit") != EXPECTED_COMMIT
        or base_receipt.get("inventorySha256") != hashes["inventory"]
        or base_receipt.get("manifestSha256") != hashes["strictManifest"]
    ):
        raise ValueError("strict launch manifest does not match base paired receipt")
    if prior_receipt.get("manifestSha256ByCohort") != cohort_hashes:
        raise ValueError("skip cohort manifests do not match prior receipt")
    evidence = prior_receipt.get("evidence")
    if not isinstance(evidence, dict) or set(evidence) != set(COHORT_MANIFESTS):
        raise ValueError("skip cohort evidence is incomplete")
    for name, path in cohort_manifest_paths.items():
        entry = evidence[name]
        if (
            not isinstance(entry, dict)
            or entry.get("manifestSha256") != cohort_hashes[name]
            or Path(str(entry.get("manifest", ""))).name != path.name
        ):
            raise ValueError(f"skip cohort manifest evidence drifted: {name}")
    lanes = receipt.get("lanes")
    if not isinstance(lanes, dict) or set(lanes) != {"candidate", "oracle"}:
        raise ValueError("paired ledger receipt lanes drifted")
    prior_lanes = prior_receipt.get("lanes")
    if not isinstance(prior_lanes, dict) or set(prior_lanes) != {"candidate", "oracle"}:
        raise ValueError("prior paired ledger receipt lanes drifted")
    base_lanes = base_receipt.get("lanes")
    if not isinstance(base_lanes, dict) or set(base_lanes) != {"candidate", "oracle"}:
        raise ValueError("base paired ledger receipt lanes drifted")
    bundle_directory = receipt_path.resolve().parent
    if (
        candidate_path.resolve().parent != bundle_directory
        or oracle_path.resolve().parent != bundle_directory
    ):
        raise ValueError("paired ledgers and receipt are not co-located")
    for lane, path in (("candidate", candidate_path), ("oracle", oracle_path)):
        entry = lanes[lane]
        prior_entry = prior_lanes[lane]
        base_entry = base_lanes[lane]
        if (
            not isinstance(entry, dict)
            or entry.get("ledgerSha256") != hashes[lane]
            or Path(str(entry.get("ledger", ""))).name != f"{lane}.jsonl"
            or path.name != f"{lane}.jsonl"
            or entry.get("promotedRows") != len(target_cases)
            or entry.get("oldOutcome") != "not-run"
            or entry.get("newOutcome")
            != ("unexpected" if lane == "candidate" else "expected")
            or entry.get("publicApiParityEvidence") is not False
            or entry.get("candidateRuntimeAttested") is not False
        ):
            raise ValueError(f"{lane} ledger does not match paired receipt")
        if (
            not isinstance(prior_entry, dict)
            or prior_entry.get("ledgerSha256") != hashes[f"prior{lane.title()}"]
            or Path(str(prior_entry.get("ledger", ""))).name != f"{lane}.jsonl"
            or entry.get("baseSha256") != hashes[f"prior{lane.title()}"]
            or Path(str(entry.get("base", ""))).name != f"{lane}.jsonl"
        ):
            raise ValueError(
                f"{lane} prior ledger chain does not match paired receipts"
            )
        if (
            not isinstance(base_entry, dict)
            or base_entry.get("ledgerSha256") != hashes[f"base{lane.title()}"]
            or Path(str(base_entry.get("ledger", ""))).name != f"{lane}.jsonl"
            or prior_entry.get("baseSha256") != hashes[f"base{lane.title()}"]
            or Path(str(prior_entry.get("base", ""))).name != f"{lane}.jsonl"
        ):
            raise ValueError(f"{lane} base ledger chain does not match paired receipts")
    return by_suite, hashes, target_cases, receipt


def _adapter_state(
    suite: JsonObject, project: str, lane: str
) -> tuple[str | None, str]:
    adapter = suite.get(f"{lane}Adapter")
    if adapter is None:
        return None, f"no-{lane}-adapter-configured"
    if not isinstance(adapter, str):
        raise TypeError(f"invalid {lane} adapter")
    if suite["id"] != "library" or project not in PROJECTS["library"]:
        raise ValueError("configured adapter has no audited project route")
    if not project.startswith("chromium-"):
        return adapter, "configured-preload-only-injects-chromium-cdp"
    return adapter, "route-configured-but-execution-and-parity-unverified"


def audit(
    manifest_path: Path,
    inventory_path: Path,
    candidate_path: Path,
    oracle_path: Path,
    receipt_path: Path,
    *,
    auxiliary_manifest_root: Path | None = None,
) -> JsonObject:
    suites, hashes, target_cases, receipt = _validated_inputs(
        manifest_path,
        inventory_path,
        candidate_path,
        oracle_path,
        receipt_path,
        auxiliary_manifest_root,
    )
    target_indexes = dict(
        zip(target_cases, cast(list[int], receipt["targetZeroBasedIndexes"]))
    )
    counts: dict[tuple[str, str], Counter[str]] = defaultdict(Counter)
    suite_totals: Counter[str] = Counter()
    outcome_totals: dict[str, Counter[str]] = {
        "candidate": Counter(),
        "oracle": Counter(),
    }
    seen: set[tuple[str, str, str]] = set()
    seen_targets: set[str] = set()
    total = 0
    for rows in zip_longest(
        json_lines(inventory_path), json_lines(candidate_path), json_lines(oracle_path)
    ):
        inventory, candidate, oracle = rows
        total += 1
        if inventory is None or candidate is None or oracle is None:
            raise ValueError(f"row {total}: inventory/paired ledger length mismatch")
        if set(inventory) != SOURCE_FIELDS:
            raise ValueError(f"row {total}: inventory source schema drifted")
        suite_id, project = inventory["suite"], inventory["projectId"]
        if not isinstance(suite_id, str) or not isinstance(project, str):
            raise TypeError(f"row {total}: malformed suite/project")
        if suite_id not in suites or project not in PROJECTS[suite_id]:
            raise ValueError(f"row {total}: unknown suite/project {suite_id}/{project}")
        if (
            inventory["role"] != suites[suite_id]["role"]
            or inventory["projectName"] != project
        ):
            raise ValueError(f"row {total}: suite role or project name drifted")
        classification = inventory["sourceClassification"]
        if not isinstance(classification, str) or classification not in CLASSIFICATIONS:
            raise ValueError(f"row {total}: unknown source classification")
        spec_id = inventory["specId"]
        if not isinstance(spec_id, str):
            raise TypeError(f"row {total}: malformed spec ID")
        key = suite_id, project, spec_id
        if key in seen:
            raise ValueError(f"row {total}: duplicate suite/project/spec ID")
        seen.add(key)
        if spec_id in target_cases:
            if (
                suite_id != "library"
                or project != target_cases[spec_id]
                or total - 1 != target_indexes[spec_id]
                or inventory["file"] != "library/browsertype-launch.spec.ts"
                or inventory["line"] != 64
                or classification != "public-api-candidate"
            ):
                raise ValueError(f"row {total}: strict successor target drifted")
            seen_targets.add(spec_id)
        group = counts[(suite_id, project)]
        group["total"] += 1
        suite_totals[suite_id] += 1
        for lane, ledger in (("candidate", candidate), ("oracle", oracle)):
            for field in SOURCE_FIELDS:
                if ledger.get(field) != inventory[field]:
                    raise ValueError(
                        f"row {total}: {lane} source field drifted: {field}"
                    )
            outcome = ledger.get("outcome")
            attempts = ledger.get("attempts")
            if (
                not isinstance(outcome, str)
                or outcome not in OUTCOMES
                or type(attempts) is not int
                or attempts < 0
            ):
                raise ValueError(f"row {total}: {lane} outcome schema drifted")
            if outcome == "not-run" and attempts != 0:
                raise ValueError(f"row {total}: {lane} not-run has attempts")
            if spec_id in target_cases and (
                outcome != ("unexpected" if lane == "candidate" else "expected")
                or attempts != 1
                or ledger.get("evidenceScope") != receipt["scope"]
                or ledger.get("publicApiParityEvidence") is not False
                or ledger.get("candidateRuntimeRequired") is not False
                or ledger.get("candidateNativeRuntimeInvoked") is not False
            ):
                raise ValueError(f"row {total}: {lane} strict red outcome drifted")
            outcome_totals[lane][outcome] += 1
            if outcome == "not-run":
                group[f"{lane}NotRun"] += 1
                group[f"{lane}NotRun:{classification}"] += 1
    if (
        total != EXPECTED_ROWS
        or set(suite_totals) != set(suites)
        or seen_targets != set(target_cases)
    ):
        raise ValueError("inventory total/suite coverage drifted")
    for suite_id, suite in suites.items():
        if suite_totals[suite_id] != suite["expectedTests"]:
            raise ValueError(f"suite count drifted: {suite_id}")
        if {project for name, project in counts if name == suite_id} != set(
            PROJECTS[suite_id]
        ):
            raise ValueError(f"project coverage drifted: {suite_id}")
    if (
        sum(outcome_totals["candidate"].values()) != EXPECTED_ROWS
        or sum(outcome_totals["oracle"].values()) != EXPECTED_ROWS
    ):
        raise ValueError("outcome count conservation failed")
    for lane in ("candidate", "oracle"):
        lane_receipt = cast(
            dict[str, object], cast(dict[str, object], receipt["lanes"])[lane]
        )
        if lane_receipt.get("outcomes") != dict(outcome_totals[lane]):
            raise ValueError(f"{lane} receipt outcome totals drifted")

    projects: list[JsonObject] = []
    blocked_not_run = 0
    blocked_public_not_run = 0
    engine_incompatible_not_run = 0
    engine_incompatible_public_not_run = 0
    chromium_route_not_run = 0
    for suite_id, project in sorted(counts):
        suite = suites[suite_id]
        group = counts[(suite_id, project)]
        protocol, channel, environment = PROJECTS[suite_id][project]
        candidate_adapter, candidate_reason = _adapter_state(
            suite, project, "candidate"
        )
        oracle_adapter, oracle_reason = _adapter_state(suite, project, "oracle")
        public_not_run = group["candidateNotRun:public-api-candidate"]
        if candidate_reason == "no-candidate-adapter-configured":
            blocked_not_run += group["candidateNotRun"]
            blocked_public_not_run += public_not_run
        elif candidate_reason == "configured-preload-only-injects-chromium-cdp":
            engine_incompatible_not_run += group["candidateNotRun"]
            engine_incompatible_public_not_run += public_not_run
        elif candidate_reason == "route-configured-but-execution-and-parity-unverified":
            chromium_route_not_run += group["candidateNotRun"]
        projects.append(
            {
                "suite": suite_id,
                "projectId": project,
                "role": suite["role"],
                "protocolRequirement": protocol,
                "channelRequirement": channel,
                "environmentRequirement": environment,
                "configuredCandidateAdapter": candidate_adapter,
                "configuredOracleAdapter": oracle_adapter,
                "candidateBlockedReason": candidate_reason,
                "oracleBlockedReason": oracle_reason,
                "applicabilityDisposition": "unresolved-not-automatically-excluded",
                "total": group["total"],
                "candidateNotRun": group["candidateNotRun"],
                "oracleNotRun": group["oracleNotRun"],
                "candidatePublicApiCandidateNotRun": public_not_run,
                "oraclePublicApiCandidateNotRun": group[
                    "oracleNotRun:public-api-candidate"
                ],
            }
        )
    if (
        sum(cast(int, p["total"]) for p in projects) != EXPECTED_ROWS
        or sum(cast(int, p["candidateNotRun"]) for p in projects)
        != outcome_totals["candidate"]["not-run"]
        or sum(cast(int, p["oracleNotRun"]) for p in projects)
        != outcome_totals["oracle"]["not-run"]
    ):
        raise ValueError("project-level count conservation failed")
    if (
        blocked_not_run + engine_incompatible_not_run + chromium_route_not_run
        != outcome_totals["candidate"]["not-run"]
    ):
        raise ValueError("candidate route count conservation failed")
    return {
        "schema": 1,
        "purpose": "adapter-coverage-not-test-execution-or-applicability",
        "upstreamCommit": EXPECTED_COMMIT,
        "hashesSha256": hashes,
        "summary": {
            "suiteCount": len(suites),
            "projectCount": len(projects),
            "inventoryRows": total,
            "candidateOutcomes": dict(sorted(outcome_totals["candidate"].items())),
            "oracleOutcomes": dict(sorted(outcome_totals["oracle"].items())),
            "candidateNotRunWithoutSuiteAdapter": blocked_not_run,
            "candidatePublicApiCandidateNotRunWithoutSuiteAdapter": blocked_public_not_run,
            "candidateNotRunWithConfiguredAdapterIncompatibleEngine": engine_incompatible_not_run,
            "candidatePublicApiCandidateNotRunWithConfiguredAdapterIncompatibleEngine": engine_incompatible_public_not_run,
            "candidateNotRunWithChromiumRouteConfigured": chromium_route_not_run,
        },
        "projects": projects,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--inventory", type=Path, default=DEFAULT_INVENTORY)
    parser.add_argument(
        "--candidate", type=Path, default=DEFAULT_PAIR / "candidate.jsonl"
    )
    parser.add_argument("--oracle", type=Path, default=DEFAULT_PAIR / "oracle.jsonl")
    parser.add_argument(
        "--receipt", type=Path, default=DEFAULT_PAIR / "promotion-receipt.json"
    )
    args = parser.parse_args()
    try:
        report = audit(
            args.manifest, args.inventory, args.candidate, args.oracle, args.receipt
        )
    except (OSError, ValueError, KeyError, TypeError) as error:
        parser.exit(2, f"adapter coverage rejected input: {error}\n")
    print(json.dumps(report, sort_keys=True, indent=2))


if __name__ == "__main__":
    main()
