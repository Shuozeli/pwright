#!/usr/bin/env python3
"""Independently verify the three unchanged Core remote-waiter RED cases.

This is a fixture-adapted internal diagnostic. It never promotes ledger rows.
"""

from __future__ import annotations

import argparse
import json
import subprocess
from collections import Counter
from pathlib import Path
from typing import Any

from official_candidate_package_snapshot_v1 import sha256, verify_snapshot
from run_playwright_official_conformance import _candidate_source_sha256

ROOT = Path(__file__).parents[1].resolve()
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-v1.58.0")
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
SUITE = "library-chromium-server-connect-waiters-synthetic-v1"
PROFILE = "playwright-server-connect-session"
MANIFEST = (
    ROOT
    / "tests/official/playwright-core-server-connect-waiters-synthetic-v1-manifest.json"
)
PRELOAD = (
    ROOT
    / "tests/official/playwright-core-server-connect-waiters-synthetic-v1-preload.cjs"
)
SERVER = (
    ROOT
    / "tests/official/playwright-core-server-connect-waiters-synthetic-v1-server.cjs"
)
SOURCE_PACKAGE = ROOT / "packages/pwright"
ARTIFACT_SOURCE = (
    ROOT
    / ".work/official-conformance/artifacts/playwright-server-connect-http-header-order-v1-20261012/pwright.node"
)
ARTIFACT_SHA256 = "0f1c8b1c6140206bdb4692d88ff1d1b9fb235b883739f176f720c3b59361523e"
SOURCE_PACKAGE_SHA256 = (
    "bddad5646a880efc80a94d7bf4b975e4eb06691562ac6602a8df29c3cf97afe2"
)
BASE_ARTIFACT_SHA256 = (
    "cc27493d9006fc1a4a2cb1375fb08861ff93758d3f937522b8ae615c5a0f94f7"
)
SOURCE_HASHES = {
    "tests/library/browsertype-connect.spec.ts": "9de4d9c8c71b8509887c81220f2eb0042e89e146c65d0311b020d191814ed2bc",
    "tests/config/browserTest.ts": "0ef5440e164d262e7c3d01523448b5d3c4d50b99060604c20f168f04419a13d5",
    "tests/config/remoteServer.ts": "ceb98f2d613b5d1639732c3fa647eba661045ebdce799246d6fc9872f5643f56",
    "tests/config/commonFixtures.ts": "cc8aad21b0684cc4958a9d76fa86090db77e93ba855c655773033da9025b9fcc",
    "tests/config/serverFixtures.ts": "4c0039a149bd61e8707adddc626b345632561bd343749293ddbe9458f6ac617b",
    "tests/config/testModeFixtures.ts": "5560c5215cb10585ff74e827f71d229c8e45c2b7b0d663691476b9707252f3a8",
    "packages/protocol/src/protocol.yml": "40aeacb5bd3ed7b93a7e8154b12b521b0716c36e592ea2c64c031872a8d1fbf9",
}
EXPECTED_IDS = {
    396: (
        "should terminate network waiters",
        "ee6df4b3a2217bd356a7-1c39ca2086607d2f91fb",
        "waitForRequest",
    ),
    412: (
        "should reject waitForEvent before browser.close finishes",
        "ee6df4b3a2217bd356a7-4bc8b13f63aaafcc1aa2",
        "waitForEvent",
    ),
    423: (
        "should reject waitForEvent before browser.onDisconnect fires",
        "ee6df4b3a2217bd356a7-4ec492666f541b5c653c",
        "waitForEvent",
    ),
}
EXPECTED_FAILURE_LINES = {396: 401, 412: 417, 423: 428}


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def _read_json(path: Path) -> dict[str, Any]:
    _require(
        path.is_file() and not path.is_symlink(),
        f"missing regular evidence file: {path}",
    )
    value = json.loads(path.read_text())
    _require(isinstance(value, dict), f"evidence is not a JSON object: {path}")
    return value


def verify_upstream() -> None:
    actual_commit = subprocess.check_output(
        ["git", "-C", str(UPSTREAM), "rev-parse", "HEAD"], text=True
    ).strip()
    _require(actual_commit == COMMIT, "pinned Playwright checkout commit changed")
    tracked_changes = subprocess.check_output(
        ["git", "-C", str(UPSTREAM), "status", "--porcelain", "--untracked-files=no"],
        text=True,
    ).strip()
    _require(not tracked_changes, "pinned Playwright checkout has tracked changes")
    for relative, digest in SOURCE_HASHES.items():
        _require(
            sha256(UPSTREAM / relative) == digest,
            f"pinned upstream source changed: {relative}",
        )
    manifest = _read_json(MANIFEST)
    _require(
        manifest
        == {
            "schema": 1,
            "repository": "https://github.com/microsoft/playwright.git",
            "tag": "v1.58.0",
            "commit": COMMIT,
            "suites": [
                {
                    "id": SUITE,
                    "role": "browser-independent-fixture-adapted-diagnostic",
                    "runner": "packages/playwright/cli.js",
                    "config": "tests/library/playwright.config.ts",
                    "args": [
                        f"tests/library/browsertype-connect.spec.ts:{line}"
                        for line in EXPECTED_IDS
                    ],
                    "candidateAdapter": "test-mode-playwright-core",
                    "candidateArtifactSource": str(ARTIFACT_SOURCE.relative_to(ROOT)),
                    "candidateArtifactSha256": ARTIFACT_SHA256,
                    "candidateSourceSha256": SOURCE_PACKAGE_SHA256,
                    "baseProfileArtifactSha256": BASE_ARTIFACT_SHA256,
                    "project": "chromium-library",
                    "requiresBrowser": False,
                    "expectedTestsPerProject": 3,
                    "expectedFiles": 1,
                }
            ],
        },
        "synthetic waiter manifest changed",
    )


def _selected_tests(report: dict[str, Any], mode: str) -> list[dict[str, Any]]:
    found: list[dict[str, Any]] = []

    def visit(suite: dict[str, Any], titles: tuple[str, ...]) -> None:
        current = titles + (str(suite.get("title", "")),)
        for spec in suite.get("specs", []):
            line = spec.get("line")
            _require(line in EXPECTED_IDS, f"unexpected selected spec line: {line}")
            title, spec_id, absent_method = EXPECTED_IDS[line]
            _require(
                spec.get("file") == "library/browsertype-connect.spec.ts"
                and spec.get("title") == title
                and spec.get("id") == spec_id
                and "run-server" in current
                and "launchServer" not in current,
                f"selected spec identity changed at line {line}",
            )
            tests = spec.get("tests")
            _require(
                isinstance(tests, list) and len(tests) == 1,
                "selected spec must have one project test",
            )
            test = tests[0]
            _require(
                test.get("projectId") == "chromium-library"
                and test.get("expectedStatus") == "passed",
                "wrong Core project or expected status",
            )
            attempts = test.get("results")
            _require(
                isinstance(attempts, list) and len(attempts) == 1,
                "retries or missing attempt obscure diagnostic",
            )
            attempt = attempts[0]
            _require(attempt.get("retry") == 0, "retry changed")
            status = attempt.get("status")
            errors = attempt.get("errors")
            _require(isinstance(errors, list), "attempt errors missing")
            if mode == "oracle":
                _require(
                    status == "passed"
                    and test.get("status") == "expected"
                    and spec.get("ok") is True
                    and not errors,
                    f"oracle case {line} did not pass",
                )
            else:
                messages = [str(error.get("message", "")) for error in errors]
                _require(
                    status == "failed"
                    and test.get("status") == "unexpected"
                    and spec.get("ok") is False
                    and len(messages) == 1,
                    f"candidate case {line} was not a single failure",
                )
                _require(
                    absent_method in messages[0] and "is not a function" in messages[0],
                    f"candidate case {line} failed outside the expected missing waiter",
                )
                _require(
                    errors[0].get("location")
                    == {
                        "file": str(
                            UPSTREAM / "tests/library/browsertype-connect.spec.ts"
                        ),
                        "line": EXPECTED_FAILURE_LINES[line],
                        "column": 17 if line == 396 else 31,
                    },
                    f"candidate case {line} did not fail at first waiter call",
                )
            found.append(
                {
                    "line": line,
                    "title": title,
                    "specId": spec_id,
                    "status": test["status"],
                    "assertionReached": mode == "oracle",
                    "failureAtFirstWaiterInvocation": mode == "candidate",
                    "errorMessages": [
                        str(error.get("message", "")) for error in errors
                    ],
                }
            )
        for child in suite.get("suites", []):
            visit(child, current)

    _require(report.get("errors") == [], "Core runner reported global errors")
    for suite in report.get("suites", []):
        visit(suite, ())
    _require(
        sorted(item["line"] for item in found) == list(EXPECTED_IDS),
        "case selection is not exactly the three pinned run-server rows",
    )
    stats = report.get("stats", {})
    _require(
        stats.get("expected") == (3 if mode == "oracle" else 0)
        and stats.get("unexpected") == (0 if mode == "oracle" else 3)
        and stats.get("skipped") == 0
        and stats.get("flaky") == 0,
        "Core report statistics differ from the exact three-case lane",
    )
    return sorted(found, key=lambda item: item["line"])


def _receipts(path: Path, mode: str, host: str, bind: str) -> dict[str, int]:
    _require(
        path.is_file() and not path.is_symlink(), f"missing fixture receipt: {path}"
    )
    records = [json.loads(line) for line in path.read_text().splitlines() if line]
    _require(
        records and all(isinstance(item, dict) for item in records),
        "invalid fixture receipt",
    )
    _require(
        all(
            item.get("mode") == mode
            and item.get("fixtureHost") == host
            and item.get("fixtureBind") == bind
            for item in records
        ),
        "mixed lane or fixture address in receipt",
    )
    preloads = [item for item in records if item.get("kind") == "preload"]
    _require(
        preloads and any(item.get("workerIndex") == "0" for item in preloads),
        "test worker preload absent",
    )
    _require(
        all(
            item.get("upstreamSourceSha256")
            == SOURCE_HASHES["tests/library/browsertype-connect.spec.ts"]
            and item.get("upstreamFixtureSha256")
            == SOURCE_HASHES["tests/config/remoteServer.ts"]
            and item.get("testSourceAdapted") is False
            and item.get("fixtureAdapted") is True
            and item.get("cdpEndpointPresent") is False
            and item.get("ownerBound") is (True if mode == "candidate" else None)
            for item in preloads
        ),
        "preload source, candidate identity or browser guard changed",
    )
    _require(
        any(
            item.get("kind") == "fixture-source-read" and item.get("workerIndex") == "0"
            for item in records
        ),
        "run-server fixture adaptation did not reach worker",
    )
    synthetic = [item for item in records if item.get("synthetic") is True]
    _require(
        synthetic
        and all(item.get("browserProcessLaunched") is False for item in synthetic),
        "fixture claimed a browser launch",
    )
    _require(
        not any(
            item.get("kind") in {"start-error", "protocol-error", "websocket-error"}
            for item in synthetic
        ),
        "synthetic protocol or server error",
    )
    by_kind = {
        kind: [item for item in synthetic if item.get("kind") == kind]
        for kind in (
            "listening",
            "upgrade",
            "root-initialize",
            "bootstrap-sent",
            "new-context-replied",
            "new-page-replied",
            "close",
            "stop-requested",
            "stop",
        )
    }
    for kind, items in by_kind.items():
        _require(len(items) == 3, f"expected three isolated synthetic {kind} receipts")
        _require(
            len({item.get("pid") for item in items}) == 3,
            f"synthetic {kind} process reuse or missing case",
        )
    _require(
        all(
            item.get("endpointPath") == "/browser-synthetic-waiters-v1"
            for item in by_kind["listening"]
        ),
        "fixture endpoint changed",
    )
    _require(
        all(
            item.get("sdkLanguage") == "javascript" and item.get("id") == 1
            for item in by_kind["root-initialize"]
        ),
        "Root.initialize changed",
    )
    _require(
        all(
            item.get("frames") == 8 and item.get("version") == "155.0"
            for item in by_kind["bootstrap-sent"]
        ),
        "synthetic bootstrap changed",
    )
    _require(
        all(
            item.get("contextGuid") == "context@1"
            for item in by_kind["new-context-replied"]
        ),
        "synthetic Context was not created",
    )
    _require(
        all(item.get("pageGuid") == "page@1" for item in by_kind["new-page-replied"]),
        "synthetic Page was not created",
    )
    for pid in {item["pid"] for item in by_kind["listening"]}:
        sequence = [item["kind"] for item in synthetic if item.get("pid") == pid]
        required = [
            "listening",
            "upgrade",
            "root-initialize",
            "bootstrap-sent",
            "new-context-replied",
            "new-page-replied",
        ]
        positions = [sequence.index(kind) for kind in required]
        _require(
            positions == sorted(positions), "synthetic wire/teardown order changed"
        )
        _require(
            sequence.index("new-page-replied")
            < sequence.index("close")
            < sequence.index("stop")
            and sequence.index("stop-requested") < sequence.index("stop"),
            "synthetic teardown order changed",
        )
        close = next(item for item in by_kind["close"] if item.get("pid") == pid)
        _require(close.get("phase") == 3, "fixture closed before Page creation")
    waiter_info = [
        item for item in synthetic if item.get("kind") == "wait-event-info-replied"
    ]
    subscriptions = [
        item for item in synthetic if item.get("kind") == "subscription-replied"
    ]
    if mode == "oracle":
        _require(
            Counter((item.get("phase"), item.get("event")) for item in waiter_info)
            == Counter(
                {
                    ("before", "request"): 1,
                    ("before", "response"): 1,
                    ("before", "download"): 2,
                    ("log", None): 4,
                }
            )
            and all(item.get("guid") == "page@1" for item in waiter_info),
            "oracle waiters were not armed on the synthetic Page",
        )
        _require(
            Counter((item.get("event"), item.get("enabled")) for item in subscriptions)
            == Counter({("request", True): 1, ("response", True): 1})
            and all(item.get("guid") == "page@1" for item in subscriptions),
            "oracle request/response subscriptions were not armed",
        )
        _require(
            any(
                item.get("forcedByFixtureTeardown") is False
                for item in by_kind["close"]
            ),
            "oracle browser.close lacked a clean WebSocket close",
        )
    else:
        _require(
            not waiter_info and not subscriptions,
            "candidate unexpectedly armed a waiter before the missing-method failure",
        )
    return {kind: len(items) for kind, items in by_kind.items()}


def verify_lane(
    result_dir: Path, mode: str, snapshot_data: dict[str, Any]
) -> dict[str, Any]:
    result = result_dir / f"{mode}.json"
    provenance = result_dir / f"{mode}.provenance.json"
    receipt = result_dir / f"{mode}.receipt.jsonl"
    log = result_dir / f"{mode}.log"
    report = _read_json(result)
    source = _read_json(provenance)
    _require(log.is_file() and not log.is_symlink(), "lane log missing")
    expected_command = [
        "node",
        str(UPSTREAM / "packages/playwright/cli.js"),
        "test",
        "--config=tests/library/playwright.config.ts",
        "--reporter=line,json",
        "--workers=1",
        "--project=chromium-library",
        "--grep=run-server",
        *[f"tests/library/browsertype-connect.spec.ts:{line}" for line in EXPECTED_IDS],
    ]
    _require(
        source.get("mode") == mode
        and source.get("suite") == SUITE
        and source.get("commit") == COMMIT
        and source.get("suiteRole") == "browser-independent-fixture-adapted-diagnostic"
        and source.get("manifestSha256") == sha256(MANIFEST)
        and source.get("resultSha256") == sha256(result)
        and source.get("command") == expected_command
        and source.get("grep") == "run-server"
        and source.get("project") == "chromium-library"
        and source.get("workers") == 1,
        f"{mode} provenance identity changed",
    )
    host = source.get("fixtureHost")
    bind = source.get("fixtureBind")
    _require(
        isinstance(host, str)
        and host.endswith(".ts.net")
        and isinstance(bind, str)
        and bind.startswith("100."),
        "Tailscale fixture address missing",
    )
    snapshot = result_dir / "candidate-package-snapshot"
    if mode == "candidate":
        _require(
            source.get("candidate") == str(snapshot)
            and source.get("candidateProfile") == PROFILE
            and source.get("candidateArtifactSha256")
            == snapshot_data.get("artifactSha256")
            and source.get("candidateSourceSha256")
            == snapshot_data.get("candidateSourceSha256"),
            "candidate selected addon differs from frozen package",
        )
        _require(
            "[pwright-candidate]" in log.read_text(), "candidate did not load Pwright"
        )
    else:
        _require(
            source.get("candidate") is None and source.get("candidateProfile") is None,
            "oracle loaded candidate",
        )
        _require(
            "[pwright-candidate]" not in log.read_text(),
            "oracle log contains Pwright adapter",
        )
    selected = _selected_tests(report, mode)
    _require(
        source.get("exitCode") == (0 if mode == "oracle" else 1),
        "lane process exit differs from expected RED diagnostic",
    )
    counts = _receipts(receipt, mode, host, bind)
    return {
        "selected": selected,
        "passed": 3 if mode == "oracle" else 0,
        "failed": 0 if mode == "oracle" else 3,
        "fixtureHost": host,
        "fixtureBind": bind,
        "resultSha256": sha256(result),
        "provenanceSha256": sha256(provenance),
        "receiptSha256": sha256(receipt),
        "logSha256": sha256(log),
        "wireReceiptCounts": counts,
    }


def verify_directory(result_dir: Path) -> dict[str, Any]:
    result_dir = result_dir.resolve()
    _require(
        result_dir.is_dir() and not result_dir.is_symlink(),
        "result directory missing or symlinked",
    )
    _require(
        result_dir.name.startswith("core-connect-waiters-synthetic-v1-red-"),
        "result directory does not carry versioned RED name",
    )
    verify_upstream()
    snapshot = result_dir / "candidate-package-snapshot"
    overlay = result_dir / "candidate-source-overlay"
    snapshot_manifest = result_dir / "candidate-package-snapshot.files.json"
    snapshot_data = _read_json(snapshot_manifest)
    artifact_digest = snapshot_data.get("artifactSha256")
    _require(
        artifact_digest == ARTIFACT_SHA256,
        "candidate artifact differs from pinned context-close-capable addon",
    )
    _require(
        _candidate_source_sha256(overlay) == SOURCE_PACKAGE_SHA256,
        "candidate source overlay differs from pinned package source",
    )
    _require(
        sha256(overlay / "native" / PROFILE / "pwright.node") == ARTIFACT_SHA256,
        "candidate source overlay addon differs from pinned artifact",
    )
    verify_snapshot(
        snapshot,
        snapshot_manifest,
        source_package=overlay,
        profile=PROFILE,
        artifact_sha256=artifact_digest,
    )
    _require(
        snapshot_data.get("candidateSourceSha256") == SOURCE_PACKAGE_SHA256,
        "snapshot source differs from pinned package source",
    )
    for item in snapshot_data["files"]:
        relative = item["path"]
        staged = overlay / relative
        _require(
            staged.is_file()
            and not staged.is_symlink()
            and staged.stat().st_size == item["size"]
            and sha256(staged) == item["sha256"],
            f"candidate source overlay differs from immutable snapshot: {relative}",
        )
    actual_overlay_files = []
    for path in overlay.rglob("*"):
        _require(not path.is_symlink(), "candidate source overlay contains a symlink")
        if path.is_file():
            actual_overlay_files.append(path.relative_to(overlay).as_posix())
        else:
            _require(path.is_dir(), "candidate source overlay contains a non-file")
    _require(
        sorted(actual_overlay_files)
        == [item["path"] for item in snapshot_data["files"]],
        "candidate source overlay has missing or extra files",
    )
    oracle = verify_lane(result_dir, "oracle", snapshot_data)
    candidate = verify_lane(result_dir, "candidate", snapshot_data)
    _require(
        oracle["fixtureHost"] == candidate["fixtureHost"]
        and oracle["fixtureBind"] == candidate["fixtureBind"],
        "paired fixture addresses differ",
    )
    return {
        "schema": 1,
        "scope": "three-unchanged-core-connect-waiter-assertions-with-synthetic-run-server-fixture",
        "classification": "official-derived-fixture-adapted-internal-diagnostic",
        "publicApiParityEvidence": False,
        "unadaptedOfficialPass": False,
        "fullPlaywrightServerSupport": False,
        "positiveEventDeliveryEvidence": False,
        "candidateAssertionReached": False,
        "candidateWaitForResponseInvocationReached": False,
        "candidateTerminalOrderingEvidence": False,
        "promotedToCoreLedger": False,
        "upstreamCommit": COMMIT,
        "upstreamSourceSha256": SOURCE_HASHES,
        "manifestSha256": sha256(MANIFEST),
        "preloadSha256": sha256(PRELOAD),
        "syntheticServerSha256": sha256(SERVER),
        "candidateSnapshotManifestSha256": sha256(snapshot_manifest),
        "candidateSourceSha256": SOURCE_PACKAGE_SHA256,
        "candidateArtifactSource": str(ARTIFACT_SOURCE.relative_to(ROOT)),
        "candidateArtifactSha256": artifact_digest,
        "fixtureHost": oracle["fixtureHost"],
        "fixtureBind": oracle["fixtureBind"],
        "lanes": {"oracle": oracle, "candidate": candidate},
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("result_dir", type=Path)
    args = parser.parse_args()
    attestation = verify_directory(args.result_dir)
    print("verified fixture-adapted Core connect waiters: oracle 3/3; candidate 0/3")
    print(f"candidate addon: {attestation['candidateArtifactSha256']}")


if __name__ == "__main__":
    main()
