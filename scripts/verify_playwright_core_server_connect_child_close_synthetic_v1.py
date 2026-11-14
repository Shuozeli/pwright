#!/usr/bin/env python3
"""Verify the one-case, browser-free Core child-close diagnostic pair.

This is official-derived, fixture-adapted evidence only. It gives no Core
ledger credit and is not an unadapted BrowserServer parity pass.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path
from typing import Any

from official_candidate_package_snapshot_v1 import sha256, verify_snapshot

ROOT = Path(__file__).parents[1].resolve()
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-v1.58.0")
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
SUITE = "library-chromium-server-connect-child-close-synthetic-v1"
PROFILE = "playwright-server-connect-session"
SOURCE_PACKAGE = ROOT / "packages/pwright"
MANIFEST = (
    ROOT
    / "tests/official/playwright-core-server-connect-child-close-synthetic-v1-manifest.json"
)
PRELOAD = (
    ROOT
    / "tests/official/playwright-core-server-connect-child-close-synthetic-v1-preload.cjs"
)
SERVER = (
    ROOT
    / "tests/official/playwright-core-server-connect-child-close-synthetic-v1-server.cjs"
)
BASE_PRELOAD = (
    ROOT
    / "tests/official/playwright-core-server-connect-lifecycle-synthetic-v1-preload.cjs"
)
SOURCE_HASHES = {
    "tests/library/browsertype-connect.spec.ts": "9de4d9c8c71b8509887c81220f2eb0042e89e146c65d0311b020d191814ed2bc",
    "tests/config/remoteServer.ts": "ceb98f2d613b5d1639732c3fa647eba661045ebdce799246d6fc9872f5643f56",
    "tests/config/commonFixtures.ts": "cc8aad21b0684cc4958a9d76fa86090db77e93ba855c655773033da9025b9fcc",
    "tests/config/serverFixtures.ts": "4c0039a149bd61e8707adddc626b345632561bd343749293ddbe9458f6ac617b",
    "tests/config/testModeFixtures.ts": "5560c5215cb10585ff74e827f71d229c8e45c2b7b0d663691476b9707252f3a8",
    "packages/playwright-core/src/cli/driver.ts": "e9d60f99fc34f6a47f323f32f1a42469743b7868a5e76ad092b4a598b1e6ad71",
}
TEST_LINE = 382
TEST_TITLE = "should emit close events on pages and contexts"
TEST_ID = "ee6df4b3a2217bd356a7-9750d96e9f0010d8c81a"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def read_json(path: Path) -> dict[str, Any]:
    require(
        path.is_file() and not path.is_symlink(),
        f"missing regular evidence file: {path}",
    )
    value = json.loads(path.read_text())
    require(isinstance(value, dict), f"evidence is not a JSON object: {path}")
    return value


def verify_upstream() -> None:
    require(
        subprocess.check_output(
            ["git", "-C", str(UPSTREAM), "rev-parse", "HEAD"], text=True
        ).strip()
        == COMMIT,
        "pinned Playwright commit changed",
    )
    require(
        not subprocess.check_output(
            [
                "git",
                "-C",
                str(UPSTREAM),
                "status",
                "--porcelain",
                "--untracked-files=no",
            ],
            text=True,
        ).strip(),
        "pinned Playwright checkout has tracked changes",
    )
    for relative, digest in SOURCE_HASHES.items():
        require(
            sha256(UPSTREAM / relative) == digest,
            f"pinned upstream source changed: {relative}",
        )
    require(
        read_json(MANIFEST)
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
                    "args": [f"tests/library/browsertype-connect.spec.ts:{TEST_LINE}"],
                    "candidateAdapter": "test-mode-playwright-core",
                    "project": "chromium-library",
                    "requiresBrowser": False,
                    "expectedTestsPerProject": 1,
                    "expectedFiles": 1,
                }
            ],
        },
        "child-close diagnostic manifest changed",
    )


def selected_test(report: dict[str, Any]) -> dict[str, Any]:
    found: list[dict[str, Any]] = []

    def visit(suite: dict[str, Any], titles: tuple[str, ...]) -> None:
        current = (*titles, str(suite.get("title", "")))
        for spec in suite.get("specs", []):
            require(
                spec.get("line") == TEST_LINE
                and spec.get("file") == "library/browsertype-connect.spec.ts"
                and spec.get("title") == TEST_TITLE
                and spec.get("id") == TEST_ID
                and "run-server" in current
                and "launchServer" not in current,
                "selected close-event spec identity changed",
            )
            tests = spec.get("tests")
            require(
                isinstance(tests, list) and len(tests) == 1,
                "selected spec must have one test",
            )
            test = tests[0]
            attempts = test.get("results")
            require(
                test.get("projectId") == "chromium-library"
                and test.get("expectedStatus") == "passed"
                and isinstance(attempts, list)
                and len(attempts) == 1,
                "selected test project or retry count changed",
            )
            attempt = attempts[0]
            passed = attempt.get("status") == "passed"
            require(
                attempt.get("status") in {"passed", "failed", "timedOut"}
                and attempt.get("retry") == 0
                and test.get("status") == ("expected" if passed else "unexpected")
                and spec.get("ok") == passed,
                "selected close-event status is inconsistent",
            )
            errors = attempt.get("errors")
            require(
                isinstance(errors, list) and bool(errors) != passed,
                "selected errors differ from status",
            )
            found.append(
                {
                    "line": TEST_LINE,
                    "title": TEST_TITLE,
                    "specId": TEST_ID,
                    "status": test["status"],
                    "errorMessages": [
                        str(error.get("message", "")) for error in errors
                    ],
                }
            )
        for child in suite.get("suites", []):
            visit(child, current)

    require(report.get("errors") == [], "Core runner reported global errors")
    for suite in report.get("suites", []):
        visit(suite, ())
    require(len(found) == 1, "exact one-case selection missing or duplicated")
    passed = found[0]["status"] == "expected"
    stats = report.get("stats", {})
    require(
        stats.get("expected") == int(passed)
        and stats.get("unexpected") == int(not passed)
        and stats.get("skipped") == 0
        and stats.get("flaky") == 0,
        "Core report statistics differ from one selected case",
    )
    return found[0]


def verify_receipt(
    path: Path, mode: str, host: str, bind: str, passed: bool
) -> dict[str, int]:
    require(path.is_file() and not path.is_symlink(), "fixture receipt missing")
    records = [json.loads(line) for line in path.read_text().splitlines() if line]
    require(
        records and all(isinstance(item, dict) for item in records),
        "invalid fixture receipt",
    )
    require(
        all(item.get("mode") == mode for item in records), "receipt mixes client lanes"
    )
    preloads = [item for item in records if item.get("kind") == "preload"]
    require(
        preloads and any(item.get("workerIndex") == "0" for item in preloads),
        "test worker preload absent",
    )
    require(
        all(
            item.get("upstreamSourceSha256")
            == SOURCE_HASHES["tests/library/browsertype-connect.spec.ts"]
            and item.get("upstreamFixtureSha256")
            == SOURCE_HASHES["tests/config/remoteServer.ts"]
            and item.get("testSourceAdapted") is False
            and item.get("fixtureAdapted") is True
            and item.get("cdpEndpointPresent") is False
            and item.get("ownerBound") is (True if mode == "candidate" else None)
            and item.get("fixtureHost") == host
            and item.get("fixtureBind") == bind
            for item in preloads
        ),
        "preload source, lane, browser guard, or address changed",
    )
    require(
        any(
            item.get("kind") == "fixture-source-read" and item.get("workerIndex") == "0"
            for item in records
        ),
        "remoteServer fixture adaptation did not reach worker",
    )
    selected = [
        item for item in records if item.get("kind") == "selected-child-close-source"
    ]
    require(
        selected
        and any(item.get("workerIndex") == "0" for item in selected)
        and all(
            item.get("upstreamSourceSha256")
            == SOURCE_HASHES["tests/library/browsertype-connect.spec.ts"]
            and item.get("selectedBodySha256")
            == hashlib.sha256(_selected_body().encode()).hexdigest()
            and item.get("testSourceAdapted") is False
            for item in selected
        ),
        "selected original assertion was not pinned",
    )
    synthetic = [item for item in records if item.get("synthetic") is True]
    require(
        synthetic
        and all(item.get("browserProcessLaunched") is False for item in synthetic),
        "browser launch claim in synthetic receipt",
    )
    require(
        all(
            item.get("fixtureHost") == host and item.get("fixtureBind") == bind
            for item in synthetic
        ),
        "synthetic fixture address drift",
    )
    require(
        not any(
            item.get("kind") in {"start-error", "websocket-error", "protocol-error"}
            for item in synthetic
        ),
        "synthetic fixture protocol error",
    )
    counts = {
        kind: sum(item.get("kind") == kind for item in synthetic)
        for kind in (
            "listening",
            "upgrade",
            "root-initialize",
            "bootstrap-sent",
            "context-created",
            "page-created",
            "stop-requested",
            "close",
            "stop",
        )
    }
    require(
        counts["listening"] == counts["stop-requested"] == counts["stop"] == 1,
        "synthetic server did not cleanly start and stop",
    )
    require(
        counts["upgrade"] == counts["root-initialize"] == counts["bootstrap-sent"] == 1,
        "synthetic bootstrap incomplete",
    )
    require(
        any(
            item.get("sdkLanguage") == "javascript" and item.get("id") == 1
            for item in synthetic
            if item.get("kind") == "root-initialize"
        ),
        "Root.initialize changed",
    )
    if passed:
        require(
            counts["context-created"] == counts["page-created"] == 1,
            "passing close assertion lacks Context/Page graph",
        )
        require(
            any(
                item.get("serverInitiated") is True
                for item in synthetic
                if item.get("kind") == "close"
            ),
            "passing close assertion lacks server-initiated WebSocket close",
        )
    return counts


def _selected_body() -> str:
    source = (UPSTREAM / "tests/library/browsertype-connect.spec.ts").read_text()
    start = source.index("    test('should emit close events on pages and contexts'")
    end = source.index("\n    });", start) + len("\n    });")
    return source[start:end]


def verify_directory(result_dir: Path) -> dict[str, Any]:
    result_dir = result_dir.resolve()
    require(
        result_dir.is_dir() and not result_dir.is_symlink(),
        "result directory missing or symlinked",
    )
    verify_upstream()
    snapshot = result_dir / "candidate-package-snapshot"
    snapshot_manifest = result_dir / "candidate-package-snapshot.files.json"
    snapshot_data = read_json(snapshot_manifest)
    artifact_digest = snapshot_data.get("artifactSha256")
    require(isinstance(artifact_digest, str), "candidate artifact digest missing")
    verify_snapshot(
        snapshot,
        snapshot_manifest,
        source_package=SOURCE_PACKAGE,
        profile=PROFILE,
        artifact_sha256=artifact_digest,
    )
    expected_command = [
        "node",
        str(UPSTREAM / "packages/playwright/cli.js"),
        "test",
        "--config=tests/library/playwright.config.ts",
        "--reporter=line,json",
        "--workers=1",
        "--project=chromium-library",
        "--grep=run-server",
        f"tests/library/browsertype-connect.spec.ts:{TEST_LINE}",
    ]
    lanes: dict[str, Any] = {}
    host: str | None = None
    bind: str | None = None
    for mode in ("oracle", "candidate"):
        result = result_dir / f"{mode}.json"
        provenance = result_dir / f"{mode}.provenance.json"
        receipt = result_dir / f"{mode}.receipt.jsonl"
        log = result_dir / f"{mode}.log"
        report = read_json(result)
        source = read_json(provenance)
        require(log.is_file() and not log.is_symlink(), "lane log missing")
        require(
            source.get("mode") == mode
            and source.get("suite") == SUITE
            and source.get("commit") == COMMIT
            and source.get("suiteRole")
            == "browser-independent-fixture-adapted-diagnostic"
            and source.get("manifestSha256") == sha256(MANIFEST)
            and source.get("resultSha256") == sha256(result)
            and source.get("command") == expected_command
            and source.get("grep") == "run-server"
            and source.get("project") == "chromium-library"
            and source.get("workers") == 1,
            f"{mode} provenance identity changed",
        )
        if host is None:
            host = source.get("fixtureHost")
            bind = source.get("fixtureBind")
        require(
            source.get("fixtureHost") == host
            and source.get("fixtureBind") == bind
            and isinstance(host, str)
            and host.endswith(".ts.net")
            and isinstance(bind, str)
            and bind.startswith("100."),
            "paired Tailscale fixture address missing",
        )
        if mode == "candidate":
            require(
                source.get("candidate") == str(snapshot)
                and source.get("candidateProfile") == PROFILE
                and source.get("candidateArtifactSha256") == artifact_digest
                and source.get("candidateSourceSha256")
                == snapshot_data.get("candidateSourceSha256")
                and "[pwright-candidate]" in log.read_text(),
                "candidate snapshot or adapter differs from run",
            )
        else:
            require(
                source.get("candidate") is None
                and source.get("candidateProfile") is None
                and "[pwright-candidate]" not in log.read_text(),
                "oracle loaded candidate adapter",
            )
        selected = selected_test(report)
        passed = selected["status"] == "expected"
        require(
            source.get("exitCode") == (0 if passed else 1),
            "lane exit code differs from report",
        )
        counts = verify_receipt(receipt, mode, host, bind, passed)
        if mode == "oracle":
            require(
                passed,
                "independent Playwright oracle must pass selected close-event assertion",
            )
        lanes[mode] = {
            "selected": selected,
            "passed": int(passed),
            "resultSha256": sha256(result),
            "provenanceSha256": sha256(provenance),
            "receiptSha256": sha256(receipt),
            "logSha256": sha256(log),
            "wireReceiptCounts": counts,
        }
    return {
        "schema": 1,
        "scope": "one-unchanged-core-child-close-assertion-with-synthetic-run-server-fixture",
        "classification": "official-derived-fixture-adapted-internal-diagnostic",
        "publicApiParityEvidence": False,
        "unadaptedOfficialPass": False,
        "fullPlaywrightServerSupport": False,
        "promotedToCoreLedger": False,
        "upstreamCommit": COMMIT,
        "upstreamSourceSha256": SOURCE_HASHES,
        "manifestSha256": sha256(MANIFEST),
        "preloadSha256": sha256(PRELOAD),
        "sharedFixturePreloadSha256": sha256(BASE_PRELOAD),
        "syntheticServerSha256": sha256(SERVER),
        "candidateSnapshotManifestSha256": sha256(snapshot_manifest),
        "candidateArtifactSha256": artifact_digest,
        "fixtureBind": bind,
        "fixtureHost": host,
        "lanes": lanes,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("result_dir", type=Path)
    args = parser.parse_args()
    attestation = verify_directory(args.result_dir)
    print(
        "verified fixture-adapted Core child-close: "
        f"oracle {attestation['lanes']['oracle']['passed']}/1; "
        f"candidate {attestation['lanes']['candidate']['passed']}/1"
    )


if __name__ == "__main__":
    main()
