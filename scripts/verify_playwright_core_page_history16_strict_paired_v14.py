"""Independently verify the immutable 16-case Core page-history paired RED ledger.

The pinned upstream source, manifest, two official reports and their provenance,
and the v13 ledger are read-only inputs. This verifier never imports its publisher.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / ".work/official-conformance"
V13 = WORK / "outcomes/core-page-fill45-strict-paired-v13"
V14 = WORK / "outcomes/core-page-history16-strict-paired-v14"
INVENTORY = WORK / "playwright-core-v1.58.0.jsonl"
MANIFEST = (
    ROOT
    / "tests/official/playwright-core-page-history-public-chromium-v1-manifest.json"
)
SOURCE = Path(
    "/Users/cyuan/projects/thirdparty/playwright-v1.58.0/tests/page/page-history.spec.ts"
)
UPSTREAM = SOURCE.parents[2]
PROMOTER = ROOT / "scripts/promote_playwright_core_page_history16_strict_paired_v14.py"
PACKAGE = WORK / "builds/core-history16-candidate-snapshot-20261020-v1/candidate"
PACKAGE_MANIFEST = PACKAGE.parent / "candidate-manifest.json"
ARTIFACT = PACKAGE / "native/official-conformance-page-check/pwright.node"
LIVE_ARTIFACT = (
    ROOT / "packages/pwright/native/official-conformance-page-check/pwright.node"
)
COUNT = 48_477
TARGET_COUNT = 16
FILE = "page/page-history.spec.ts"
PROJECT = "chromium-page"
CAPTURE_SUITE = "library-page-history-public-chromium"
SCOPE_V13 = "official-browser-backed-page-fill-full-file-paired-v13"
SCOPE_V14 = "official-browser-backed-page-history-full-file-paired-v14"
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
MANIFEST_SHA256 = "2b46039f1cdf5dac9f165b11db234c1777dceab1d6d82aa78a91d03afa6210ed"
SOURCE_SHA256 = "2e442a704a5baca8fd98b69b15007c6601d1c859ae343e6d73a7acc0b4109002"
SPEC_IDS_SHA256 = "632d3bb4dd07f8e8b002cbe9a0ea0a7ac01886164e84e3c10fa7aecd10dbf7e6"
CHROME_VERSION_OBSERVED = "155.0.8059.85"
CANDIDATE_PROFILE = "official-conformance-page-check"
CANDIDATE_ARTIFACT_SHA256 = (
    "ce637639beec9343233441498739b24e642debd3334605f29e58eeab8bac75d0"
)
CANDIDATE_SOURCE_SHA256 = (
    "d886564b6d7b99457fea41ff4716b3f7d42388aeb75aca4fde2b075c4d89f43c"
)
PACKAGE_MANIFEST_SHA256 = (
    "b4925a5a2f1855689c7a41582b338bcfb819574607c39fdb5b6d542b05c9edc5"
)
LANES = ("candidate", "oracle")
REPORT_SHA256 = {
    "candidate": "43e04049ae730d667b771440f1157113f71cb11e11cb8818595989fa2f0f4d79",
    "oracle": "a8c5637e40f0c7c5925524dc35aa0b70cb0521b46e647b5e183ec997c43e3351",
}
PROVENANCE_SHA256 = {
    "candidate": "deb717ec903b6914eef435b1a24b7cb141ee34ee5f8d72b10b3cc0f63b5e57b0",
    "oracle": "a64fa6e547ad332ce24cdffb594cff79d4aedb3450ae83ccba38642c4fda571c",
}
REPORT_OUTCOMES = {
    "candidate": {"expected": 11, "unexpected": 5},
    "oracle": {"expected": 13, "unexpected": 3},
}
V13_SHA256 = {
    "candidate.jsonl": "31fd92354d0711550eb5e26e35b375374b13d12f6309414d6be7fc0c36f65416",
    "candidate.summary.json": "3a99c285ee837136a7defe32019b0aa836613b1f9cc89ddc853e9451d2776cd9",
    "oracle.jsonl": "1baef074b29f90a49caa8ce4ae8620499359f03a0017fc0a98a4d1106c271303",
    "oracle.summary.json": "b19f6c985a054aefbf520e2c9598ff215c762beab014e4c72d4833f80e922f91",
    "promotion-receipt.json": "8a10291c3301a8e6e0877387b9d6d279927eba31a1831ef0e3c420353d1c46de",
}
TARGETS = (
    (39_792, "34c3de86d93245a3d399-6afc0abb5612468ce13d", 21),
    (39_795, "34c3de86d93245a3d399-4dfffa636cc215010883", 39),
    (39_798, "34c3de86d93245a3d399-e8c171d7d1bab10f52e4", 55),
    (39_801, "34c3de86d93245a3d399-72f45706d7dc5f2834b1", 95),
    (39_804, "34c3de86d93245a3d399-94501e40733c80c18118", 109),
    (39_807, "34c3de86d93245a3d399-c126322edb449fc1d8ce", 116),
    (39_810, "34c3de86d93245a3d399-884c389b8f2a36fbbd4d", 123),
    (39_813, "34c3de86d93245a3d399-26e2f404dbbae81c1b33", 143),
    (39_816, "34c3de86d93245a3d399-2e235a3d50d52c9643a7", 175),
    (39_819, "34c3de86d93245a3d399-da3eef3821ac9e00159f", 183),
    (39_822, "34c3de86d93245a3d399-62e435ae63d00a9364b3", 191),
    (39_825, "34c3de86d93245a3d399-ca5c817a3c8a357dfe20", 198),
    (39_828, "34c3de86d93245a3d399-1584065d84619c0cde95", 205),
    (39_831, "34c3de86d93245a3d399-d8e4611f384d9260c715", 226),
    (39_834, "34c3de86d93245a3d399-ee5cb1eba1cdc63e5a23", 249),
    (39_837, "34c3de86d93245a3d399-f87c743fe5c65722a574", 281),
)
RED_COMPARISON = {
    "candidateOnly": (
        "34c3de86d93245a3d399-884c389b8f2a36fbbd4d",
        "34c3de86d93245a3d399-1584065d84619c0cde95",
        "34c3de86d93245a3d399-ee5cb1eba1cdc63e5a23",
    ),
    "both": (
        "34c3de86d93245a3d399-72f45706d7dc5f2834b1",
        "34c3de86d93245a3d399-d8e4611f384d9260c715",
    ),
    "oracleOnly": ("34c3de86d93245a3d399-6afc0abb5612468ce13d",),
}
GO_FORWARD_RED_SPEC_ID = "34c3de86d93245a3d399-d8e4611f384d9260c715"
BUNDLE_NAMES = {
    "candidate.jsonl",
    "candidate.summary.json",
    "oracle.jsonl",
    "oracle.summary.json",
    "promotion-receipt.json",
}
TOOLING_SOURCES = (
    (
        "scripts/official_result_destination.py",
        "61cd172cc58af3fecfe09b9d44da5ae67038ab660f7ed8a140b1ddf1784c04ab",
    ),
    (
        "scripts/run_playwright_official_conformance.py",
        "f1e4282361b650daaf0e5eae406e22b099c2a4f9dac796b4a6d594d3543dad29",
    ),
)
HARNESS_SOURCES = {
    "candidate": (
        (
            "tests/official/playwright-core-server-runtime.cjs",
            "58278e6c976da47074a1eb8d682916ebc3d0a41975fe89fe9270d1a664c396fd",
        ),
        (
            "tests/official/playwright-core-candidate-preload.cjs",
            "d17be590ff9334b0d99dd16d0078e4b434a39eae21568b799314ababc6bbf924",
        ),
    ),
    "oracle": (
        (
            "tests/official/playwright-core-server-runtime.cjs",
            "58278e6c976da47074a1eb8d682916ebc3d0a41975fe89fe9270d1a664c396fd",
        ),
        (
            "tests/official/playwright-core-oracle-preload.cjs",
            "ac947bd025a32c148e7b9e650f94eba11846b56c4bdd546dc3630b8edf28d43b",
        ),
    ),
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical_line(row: dict[str, Any]) -> bytes:
    return (json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n").encode()


def sealed(path: Path, expected: str, label: str) -> bytes:
    require(path.is_file() and not path.is_symlink(), f"{label} absent or symlink")
    data = path.read_bytes()
    require(digest(data) == expected, f"{label} SHA-256 changed")
    return data


def object_from(data: bytes, label: str) -> dict[str, Any]:
    value = json.loads(data)
    require(isinstance(value, dict), f"{label} is not an object")
    return value


def report_path(lane: str) -> Path:
    require(lane in LANES, "unknown lane")
    return WORK / f"results/core-history16-{lane}-20261020-v1.json"


def provenance_path(lane: str) -> Path:
    return report_path(lane).with_suffix(".provenance.json")


def _source_records(
    raw: Any,
    records: tuple[tuple[str, str], ...],
    label: str,
    *,
    absolute: bool = False,
) -> None:
    expected = [
        {"path": str(ROOT / path) if absolute else path, "sha256": sha}
        for path, sha in records
    ]
    require(raw == expected, f"{label} source pins differ")
    for relative, sha in records:
        sealed(ROOT / relative, sha, f"{label} source {relative}")


def _candidate_source_sha256(package: Path, source_files: list[str]) -> str:
    """Recompute the runner's length-prefixed source digest independently."""
    measured = hashlib.sha256()
    for relative in source_files:
        path = package / relative
        name = relative.encode()
        measured.update(len(name).to_bytes(8, "big"))
        measured.update(name)
        measured.update(path.stat().st_size.to_bytes(8, "big"))
        with path.open("rb") as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                measured.update(chunk)
    return measured.hexdigest()


def verify_package() -> None:
    manifest = object_from(
        sealed(PACKAGE_MANIFEST, PACKAGE_MANIFEST_SHA256, "frozen candidate manifest"),
        "frozen candidate manifest",
    )
    require(
        set(manifest)
        == {
            "schema",
            "profile",
            "candidateSourceSha256",
            "artifactSha256",
            "sourcePackage",
            "snapshotPackage",
            "files",
        }
        and manifest["schema"] == 1
        and manifest["profile"] == CANDIDATE_PROFILE
        and manifest["candidateSourceSha256"] == CANDIDATE_SOURCE_SHA256
        and manifest["artifactSha256"] == CANDIDATE_ARTIFACT_SHA256
        and manifest["sourcePackage"] == str(ROOT / "packages/pwright")
        and manifest["snapshotPackage"] == str(PACKAGE)
        and isinstance(manifest["files"], list)
        and len(manifest["files"]) == 68,
        "frozen candidate manifest identity differs",
    )
    require(
        PACKAGE.is_dir() and not PACKAGE.is_symlink(),
        "frozen package absent or symlink",
    )
    require(
        (PACKAGE / "src").is_dir() and not (PACKAGE / "src").is_symlink(),
        "frozen candidate src absent or symlink",
    )
    listed: set[str] = set()
    listed_order: list[str] = []
    for item in manifest["files"]:
        require(
            isinstance(item, dict) and set(item) == {"path", "sha256", "size"},
            "malformed candidate manifest file",
        )
        require(isinstance(item["path"], str), "candidate manifest path is not text")
        relative = Path(item["path"])
        require(
            not relative.is_absolute()
            and relative.parts
            and all(part not in {".", ".."} for part in relative.parts)
            and relative.as_posix() == item["path"]
            and relative.as_posix() not in listed,
            "candidate manifest path unsafe or duplicate",
        )
        listed.add(relative.as_posix())
        listed_order.append(relative.as_posix())
        path = PACKAGE / relative
        require(
            path.is_file()
            and not path.is_symlink()
            and path.stat().st_size == item["size"],
            f"candidate package file absent, changed, or symlink: {relative}",
        )
        sealed(path, item["sha256"], f"candidate package file {relative}")
    actual: set[str] = set()
    for path in PACKAGE.rglob("*"):
        require(not path.is_symlink(), f"candidate package symlink: {path}")
        if path.is_file():
            actual.add(path.relative_to(PACKAGE).as_posix())
        else:
            require(path.is_dir(), f"candidate package special file: {path}")
    require(actual == listed, "candidate package file set differs")
    native_relative = ARTIFACT.relative_to(PACKAGE).as_posix()
    source_files = sorted(
        relative
        for relative in listed
        if relative == "package.json" or relative.startswith("src/")
    )
    require(
        listed_order == sorted(listed)
        and "package.json" in source_files
        and len(source_files) > 1
        and listed == set(source_files) | {native_relative},
        "candidate package layout or manifest order differs",
    )
    require(
        _candidate_source_sha256(PACKAGE, source_files) == CANDIDATE_SOURCE_SHA256,
        "candidate snapshot source digest differs from captured provenance",
    )
    require(
        next(
            item["sha256"]
            for item in manifest["files"]
            if item["path"] == ARTIFACT.relative_to(PACKAGE).as_posix()
        )
        == CANDIDATE_ARTIFACT_SHA256,
        "candidate native artifact pin differs",
    )


def verify_provenance(lane: str, data: bytes) -> None:
    value = object_from(data, f"{lane} provenance")
    require(
        value.get("schema") == 2
        and value.get("mode") == lane
        and value.get("suite") == CAPTURE_SUITE
        and value.get("suiteRole") == "browser-backed-official-file"
        and value.get("project") == PROJECT
        and value.get("repository") == "https://github.com/microsoft/playwright.git"
        and value.get("tag") == "v1.58.0"
        and value.get("commit") == COMMIT
        and value.get("upstream") == str(UPSTREAM)
        and value.get("manifest") == str(MANIFEST)
        and value.get("manifestSha256") == MANIFEST_SHA256
        and value.get("result") == str(report_path(lane))
        and value.get("resultSha256") == REPORT_SHA256[lane]
        and value.get("endpoint") == "http://127.0.0.1:9222"
        and value.get("fixtureHost") == "macbookpro-yuacx.tail8f3b66.ts.net"
        and value.get("fixtureBind") == "100.78.97.63"
        and value.get("filters") == ["tests/page/page-history.spec.ts"]
        and value.get("workers") == 1
        and value.get("grep") is None
        and value.get("headed") is False
        and value.get("exitCode") == 1
        and value.get("adapter")
        == (
            "test-mode-playwright-core"
            if lane == "candidate"
            else "test-mode-playwright-core-cdp"
        )
        and value.get("command")
        == [
            "node",
            str(UPSTREAM / "packages/playwright/cli.js"),
            "test",
            "--config=tests/library/playwright.config.ts",
            "--reporter=line,json",
            "--workers=1",
            "--project=chromium-page",
            "tests/page/page-history.spec.ts",
        ],
        f"{lane} provenance identity or source pin differs",
    )
    _source_records(value.get("toolingSources"), TOOLING_SOURCES, f"{lane} tooling")
    _source_records(
        value.get("harnessInputs"),
        HARNESS_SOURCES[lane],
        f"{lane} harness",
        absolute=True,
    )
    if lane == "candidate":
        require(
            value.get("candidate") == str(ROOT / "packages/pwright")
            and value.get("candidateArtifact") == str(LIVE_ARTIFACT)
            and value.get("candidateProfile") == CANDIDATE_PROFILE
            and value.get("candidateArtifactSha256") == CANDIDATE_ARTIFACT_SHA256
            and value.get("candidateSourceSha256") == CANDIDATE_SOURCE_SHA256,
            "candidate provenance runtime pin differs",
        )
    else:
        require(
            all(
                value.get(key) is None
                for key in (
                    "candidate",
                    "candidateArtifact",
                    "candidateProfile",
                    "candidateArtifactSha256",
                    "candidateSourceSha256",
                )
            ),
            "oracle provenance contains candidate runtime",
        )


def report_rows(
    lane: str, data: bytes, target_ids: set[str]
) -> dict[str, dict[str, Any]]:
    report = object_from(data, f"{lane} official report")
    stats = report.get("stats")
    require(
        isinstance(stats, dict)
        and all(
            stats.get(key) == value
            for key, value in {
                **REPORT_OUTCOMES[lane],
                "skipped": 0,
                "flaky": 0,
            }.items()
        ),
        f"{lane} report counts differ",
    )
    suites = report.get("suites")
    require(
        isinstance(suites, list) and len(suites) == 1,
        f"{lane} report suite count differs",
    )
    specs: list[dict[str, Any]] = []

    def visit(suite: dict[str, Any]) -> None:
        require(isinstance(suite, dict), "malformed report suite")
        children = suite.get("suites", [])
        entries = suite.get("specs", [])
        require(
            isinstance(children, list) and isinstance(entries, list),
            "malformed report suite children",
        )
        for spec in entries:
            require(isinstance(spec, dict), "malformed report spec")
            specs.append(spec)
        for child in children:
            visit(child)

    for suite in suites:
        visit(suite)
    require(
        len(specs) == TARGET_COUNT, f"{lane} report is not the complete 16-case file"
    )
    rows: dict[str, dict[str, Any]] = {}
    for spec in specs:
        spec_id = spec.get("id")
        tests = spec.get("tests")
        require(
            spec.get("file") == FILE
            and isinstance(spec_id, str)
            and spec_id in target_ids
            and spec_id not in rows
            and isinstance(tests, list)
            and len(tests) == 1
            and isinstance(tests[0], dict),
            f"{lane} report has wrong, missing, or duplicate invocation",
        )
        test = tests[0]
        attempts = test.get("results")
        require(
            test.get("projectId") == PROJECT
            and test.get("expectedStatus") == "passed"
            and test.get("status") in {"expected", "unexpected"}
            and isinstance(attempts, list)
            and len(attempts) == 1
            and isinstance(attempts[0], dict)
            and attempts[0].get("retry") == 0,
            f"{lane} report has an unexecuted or malformed invocation for {spec_id}",
        )
        attempt = attempts[0]
        if test["status"] == "expected":
            require(
                attempt.get("status") == "passed"
                and not attempt.get("error")
                and not attempt.get("errors"),
                f"{lane} report GREEN invocation is not a clean pass for {spec_id}",
            )
            rows[spec_id] = {"outcome": "expected", "attempts": 1}
        else:
            error = attempt.get("error")
            require(
                attempt.get("status") in {"failed", "timedOut"}
                and isinstance(error, dict)
                and isinstance(error.get("message"), str)
                and bool(error["message"])
                and isinstance(attempt.get("errors"), list)
                and bool(attempt["errors"]),
                f"{lane} report RED invocation lacks terminal failure for {spec_id}",
            )
            rows[spec_id] = {
                "outcome": "unexpected",
                "attempts": 1,
                "error": error["message"],
            }
    require(set(rows) == target_ids, f"{lane} report does not cover exact target IDs")
    require(
        {
            outcome: sum(row["outcome"] == outcome for row in rows.values())
            for outcome in ("expected", "unexpected")
        }
        == REPORT_OUTCOMES[lane],
        f"{lane} report statuses contradict totals",
    )
    return rows


def verify_observed_failure_stage(lane: str, data: bytes) -> None:
    """The same RED test fails at different observed assertions in each lane."""
    report = object_from(data, f"{lane} official report")
    spec = next(
        entry
        for entry in report["suites"][0]["specs"]
        if entry.get("id") == GO_FORWARD_RED_SPEC_ID
    )
    attempt = spec["tests"][0]["results"][0]
    details = "\n".join(error.get("message", "") for error in attempt["errors"])
    api, source_line = (
        ("page.waitForSelector", ":246:14")
        if lane == "candidate"
        else ("page.goBack", ":229:14")
    )
    require(
        api in details and f"page-history.spec.ts{source_line}" in details,
        f"{lane} shared RED observed failure stage differs",
    )


def red_comparison(reports: dict[str, dict[str, dict[str, Any]]]) -> dict[str, Any]:
    candidate_red = {
        spec_id
        for spec_id, row in reports["candidate"].items()
        if row["outcome"] == "unexpected"
    }
    oracle_red = {
        spec_id
        for spec_id, row in reports["oracle"].items()
        if row["outcome"] == "unexpected"
    }
    observed = {
        "candidateOnly": candidate_red - oracle_red,
        "both": candidate_red & oracle_red,
        "oracleOnly": oracle_red - candidate_red,
    }
    require(
        all(observed[key] == set(ids) for key, ids in RED_COMPARISON.items()),
        "history RED intersection or difference differs",
    )
    return {
        "candidateOnlySpecIds": sorted(observed["candidateOnly"]),
        "bothRedSpecIds": sorted(observed["both"]),
        "oracleOnlySpecIds": sorted(observed["oracleOnly"]),
        "candidateOnlyCount": len(observed["candidateOnly"]),
        "bothRedCount": len(observed["both"]),
        "oracleOnlyCount": len(observed["oracleOnly"]),
        "bothRedDifferentObservedStage": {
            "specId": GO_FORWARD_RED_SPEC_ID,
            "candidate": "page.waitForSelector at upstream source line 246",
            "oracle": "page.goBack at upstream source line 229",
        },
    }


def expected_summary(
    lane: str, old_summary: dict[str, Any], ledger_bytes: bytes
) -> dict[str, Any]:
    require(lane in LANES, "unknown lane")
    outcomes = dict(old_summary["outcomes"])
    scopes = dict(old_summary["explicitEvidenceScopes"])
    require(
        SCOPE_V14 not in scopes and outcomes["not-run"] >= TARGET_COUNT,
        "v14 already promoted",
    )
    outcomes["not-run"] -= TARGET_COUNT
    for outcome, count in REPORT_OUTCOMES[lane].items():
        outcomes[outcome] += count
    scopes[SCOPE_V14] = TARGET_COUNT
    return {
        "schema": 1,
        "inventory": str(INVENTORY),
        "records": COUNT,
        "outcomes": dict(sorted(outcomes.items())),
        "explicitEvidenceScopes": dict(sorted(scopes.items())),
        "ledgerSha256": digest(ledger_bytes),
        "ledger": str(V14 / f"{lane}.jsonl"),
    }


def verify_inputs() -> dict[str, Any]:
    inventory_lines = sealed(INVENTORY, INVENTORY_SHA256, "Core inventory").splitlines(
        keepends=True
    )
    require(len(inventory_lines) == COUNT, "Core inventory row count changed")
    manifest = object_from(
        sealed(MANIFEST, MANIFEST_SHA256, "history manifest"), "history manifest"
    )
    suite = manifest.get("suites")
    require(
        manifest.get("schema") == 1
        and manifest.get("repository") == "https://github.com/microsoft/playwright.git"
        and manifest.get("tag") == "v1.58.0"
        and manifest.get("commit") == COMMIT
        and isinstance(suite, list)
        and len(suite) == 1
        and suite[0].get("id") == CAPTURE_SUITE
        and suite[0].get("project") == PROJECT
        and suite[0].get("crossOriginForward") is True
        and suite[0].get("expectedTestsPerProject") == TARGET_COUNT
        and suite[0].get("expectedFiles") == 1
        and suite[0].get("expectedSpecIdsSha256") == SPEC_IDS_SHA256
        and suite[0].get("upstreamTestSourceSha256") == SOURCE_SHA256,
        "history manifest selection or cross-origin fixture setting differs",
    )
    sealed(SOURCE, SOURCE_SHA256, "upstream page-history source")
    require(
        digest(
            "".join(
                f"{spec_id}\n"
                for spec_id in sorted(spec_id for _, spec_id, _ in TARGETS)
            ).encode()
        )
        == SPEC_IDS_SHA256,
        "literal target IDs do not match source-pinned digest",
    )
    selected: dict[int, dict[str, Any]] = {}
    for index, raw in enumerate(inventory_lines):
        row = json.loads(raw)
        if (
            row.get("suite") == "library"
            and row.get("projectId") == PROJECT
            and row.get("file") == FILE
        ):
            selected[index] = row
    require(
        len(selected) == TARGET_COUNT
        and set(selected) == {index for index, _, _ in TARGETS},
        "inventory target indexes differ",
    )
    for index, spec_id, line in TARGETS:
        require(
            selected[index].get("specId") == spec_id
            and selected[index].get("line") == line,
            f"inventory history source identity differs at {index}",
        )
    old_files = {
        name: sealed(V13 / name, sha, f"v13 {name}") for name, sha in V13_SHA256.items()
    }
    old_receipt = object_from(old_files["promotion-receipt.json"], "v13 receipt")
    require(
        old_receipt.get("scope") == SCOPE_V13
        and old_receipt.get("recordsPerLane") == COUNT
        and old_receipt.get("fullCorpusExecution") is False
        and old_receipt.get("publicApiParityEvidence") is False,
        "v13 predecessor scope differs",
    )
    old_lines: dict[str, list[bytes]] = {}
    old_summaries: dict[str, dict[str, Any]] = {}
    replacements: dict[str, dict[int, dict[str, Any]]] = {}
    reports: dict[str, dict[str, dict[str, Any]]] = {}
    target_ids = {spec_id for _, spec_id, _ in TARGETS}
    verify_package()
    for lane in LANES:
        lines = old_files[f"{lane}.jsonl"].splitlines(keepends=True)
        require(len(lines) == COUNT, f"v13 {lane} row count differs")
        old_lines[lane] = lines
        summary = object_from(old_files[f"{lane}.summary.json"], f"v13 {lane} summary")
        require(
            summary.get("schema") == 1
            and summary.get("records") == COUNT
            and summary.get("ledgerSha256") == V13_SHA256[f"{lane}.jsonl"]
            and old_receipt["lanes"][lane]["ledgerSha256"]
            == V13_SHA256[f"{lane}.jsonl"]
            and old_receipt["lanes"][lane]["summarySha256"]
            == V13_SHA256[f"{lane}.summary.json"]
            and sum(summary["outcomes"].values()) == COUNT,
            f"v13 {lane} summary or receipt chain differs",
        )
        old_summaries[lane] = summary
        verify_provenance(
            lane,
            sealed(
                provenance_path(lane), PROVENANCE_SHA256[lane], f"{lane} provenance"
            ),
        )
        report_data = sealed(
            report_path(lane), REPORT_SHA256[lane], f"{lane} official report"
        )
        reports[lane] = report_rows(lane, report_data, target_ids)
        verify_observed_failure_stage(lane, report_data)
        replacements[lane] = {}
        for index, spec_id, _ in TARGETS:
            old = object_from(lines[index], f"v13 {lane} target {index}")
            require(
                old == selected[index] | {"attempts": 0, "outcome": "not-run"},
                f"v13 {lane} target {index} is not the untested inventory row",
            )
            new = dict(old)
            new.update(reports[lane][spec_id])
            new.update(
                {
                    "resultFile": str(report_path(lane)),
                    "evidenceScope": SCOPE_V14,
                    "fixtureAdapted": True,
                    "browserExecutionEvidence": True,
                    "publicApiParityEvidence": False,
                }
            )
            replacements[lane][index] = new
    red_comparison(reports)
    return {
        "oldLines": old_lines,
        "oldSummaries": old_summaries,
        "oldReceipt": old_receipt,
        "targets": selected,
        "replacementRows": replacements,
        "reports": reports,
    }


def expected_lane_receipt(
    lane: str, files: dict[str, bytes], checked: dict[str, Any]
) -> dict[str, Any]:
    new_lines = files[f"{lane}.jsonl"].splitlines(keepends=True)
    changed = [
        {
            "index": index,
            "specId": spec_id,
            "oldRowSha256": digest(checked["oldLines"][lane][index]),
            "newRowSha256": digest(new_lines[index]),
            "oldOutcome": "not-run",
            "newOutcome": checked["replacementRows"][lane][index]["outcome"],
            "oldAttempts": 0,
            "newAttempts": 1,
        }
        for index, spec_id, _ in TARGETS
    ]
    summary = object_from(files[f"{lane}.summary.json"], f"v14 {lane} summary")
    return {
        "base": str(V13 / f"{lane}.jsonl"),
        "baseSha256": V13_SHA256[f"{lane}.jsonl"],
        "baseSummarySha256": V13_SHA256[f"{lane}.summary.json"],
        "ledger": str(V14 / f"{lane}.jsonl"),
        "ledgerSha256": digest(files[f"{lane}.jsonl"]),
        "summarySha256": digest(files[f"{lane}.summary.json"]),
        "officialResult": str(report_path(lane)),
        "officialResultSha256": REPORT_SHA256[lane],
        "provenance": str(provenance_path(lane)),
        "provenanceSha256": PROVENANCE_SHA256[lane],
        "changedRows": changed,
        "promotedRows": TARGET_COUNT,
        "unchangedRawRows": COUNT - TARGET_COUNT,
        "expectedRows": REPORT_OUTCOMES[lane]["expected"],
        "unexpectedRows": REPORT_OUTCOMES[lane]["unexpected"],
        "outcomes": summary["outcomes"],
    }


def expected_receipt(
    files: dict[str, bytes], checked: dict[str, Any]
) -> dict[str, Any]:
    require(
        PROMOTER.is_file() and not PROMOTER.is_symlink(),
        "publisher source absent or symlink",
    )
    return {
        "schema": 1,
        "scope": SCOPE_V14,
        "promotionKind": "browser-backed-public-api-full-file-paired-red-status",
        "priorBundle": str(V13),
        "priorBundleReceiptSha256": V13_SHA256["promotion-receipt.json"],
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "manifest": str(MANIFEST),
        "manifestSha256": MANIFEST_SHA256,
        "upstreamSourceSha256": SOURCE_SHA256,
        "targetSpecIdsSha256": SPEC_IDS_SHA256,
        "recordsPerLane": COUNT,
        "targetZeroBasedIndexes": [index for index, _, _ in TARGETS],
        "changedRowsPerLane": {lane: TARGET_COUNT for lane in LANES},
        "unchangedRawRowsPerLane": {lane: COUNT - TARGET_COUNT for lane in LANES},
        "fixtureAdapted": True,
        "browserExecutionEvidence": True,
        "completeFileExecution": True,
        "fullCorpusExecution": False,
        "chromeVersionObserved": CHROME_VERSION_OBSERVED,
        "launchMode": "cdp-adapter",
        "publicApiParityEvidence": False,
        "upstreamAssertionsModified": False,
        "candidateExpectedRows": 11,
        "candidateUnexpectedRows": 5,
        "oracleExpectedRows": 13,
        "oracleUnexpectedRows": 3,
        "redComparison": red_comparison(checked["reports"]),
        "candidateProfile": CANDIDATE_PROFILE,
        "candidateArtifactSha256": CANDIDATE_ARTIFACT_SHA256,
        "candidateSourceSha256": CANDIDATE_SOURCE_SHA256,
        "candidatePackage": str(PACKAGE),
        "candidatePackageManifest": str(PACKAGE_MANIFEST),
        "candidatePackageManifestSha256": PACKAGE_MANIFEST_SHA256,
        "promoterSourceSha256": digest(PROMOTER.read_bytes()),
        "verifierSourceSha256": digest(Path(__file__).read_bytes()),
        "lanes": {lane: expected_lane_receipt(lane, files, checked) for lane in LANES},
    }


def verify_payloads(
    files: dict[str, bytes], checked: dict[str, Any] | None = None
) -> dict[str, Any]:
    require(set(files) == BUNDLE_NAMES, "v14 bundle files differ")
    inputs = checked if checked is not None else verify_inputs()
    for lane in LANES:
        old_lines = inputs["oldLines"][lane]
        new_lines = files[f"{lane}.jsonl"].splitlines(keepends=True)
        require(len(new_lines) == COUNT, f"v14 {lane} row count differs")
        replacements = inputs["replacementRows"][lane]
        for index, (old_raw, new_raw) in enumerate(
            zip(old_lines, new_lines, strict=True)
        ):
            if index in replacements:
                require(
                    new_raw == canonical_line(replacements[index]),
                    f"v14 {lane} target row {index} differs",
                )
            else:
                require(
                    new_raw == old_raw, f"v14 {lane} off-target raw row {index} changed"
                )
        summary = object_from(files[f"{lane}.summary.json"], f"v14 {lane} summary")
        require(
            summary
            == expected_summary(
                lane, inputs["oldSummaries"][lane], files[f"{lane}.jsonl"]
            ),
            f"v14 {lane} summary differs",
        )
    receipt = object_from(files["promotion-receipt.json"], "v14 receipt")
    require(receipt == expected_receipt(files, inputs), "v14 receipt differs")
    return receipt


def verify_existing() -> dict[str, Any]:
    require(V14.is_dir() and not V14.is_symlink(), "v14 bundle is absent or symlink")
    require(
        {path.name for path in V14.iterdir()} == BUNDLE_NAMES,
        "v14 published file set differs",
    )
    require(
        all(
            (V14 / name).is_file() and not (V14 / name).is_symlink()
            for name in BUNDLE_NAMES
        ),
        "v14 published file is absent or symlink",
    )
    return verify_payloads({name: (V14 / name).read_bytes() for name in BUNDLE_NAMES})


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify-existing", action="store_true", required=True)
    parser.parse_args()
    try:
        receipt = verify_existing()
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
        parser.exit(
            2, f"strict paired Core history v14 verification rejected: {error}\n"
        )
    print(
        json.dumps(
            {"verified": True, "scope": receipt["scope"], "destination": str(V14)},
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
