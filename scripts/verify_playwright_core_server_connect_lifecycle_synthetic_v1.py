#!/usr/bin/env python3
"""Verify the browser-free, fixture-adapted Core connect lifecycle pair.

This verifier deliberately never promotes a case to the official conformance
ledger. It checks the unchanged assertions, the exact fixture substitution,
both isolated client lanes, and the synthetic protocol's wire receipts.
"""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any

from official_candidate_package_snapshot_v1 import sha256, verify_snapshot

ROOT = Path(__file__).parents[1].resolve()
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-v1.58.0")
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
SUITE = "library-chromium-server-connect-lifecycle-synthetic-v1"
PROFILE = "playwright-server-connect-session"
MANIFEST = (
    ROOT
    / "tests/official/playwright-core-server-connect-lifecycle-synthetic-v1-manifest.json"
)
PRELOAD = (
    ROOT
    / "tests/official/playwright-core-server-connect-lifecycle-synthetic-v1-preload.cjs"
)
SERVER = (
    ROOT
    / "tests/official/playwright-core-server-connect-lifecycle-synthetic-v1-server.cjs"
)
SOURCE_PACKAGE = ROOT / "packages/pwright"
SOURCE_HASHES = {
    "tests/library/browsertype-connect.spec.ts": "9de4d9c8c71b8509887c81220f2eb0042e89e146c65d0311b020d191814ed2bc",
    "tests/config/remoteServer.ts": "ceb98f2d613b5d1639732c3fa647eba661045ebdce799246d6fc9872f5643f56",
    "tests/config/commonFixtures.ts": "cc8aad21b0684cc4958a9d76fa86090db77e93ba855c655773033da9025b9fcc",
    "tests/config/serverFixtures.ts": "4c0039a149bd61e8707adddc626b345632561bd343749293ddbe9458f6ac617b",
    "tests/config/testModeFixtures.ts": "5560c5215cb10585ff74e827f71d229c8e45c2b7b0d663691476b9707252f3a8",
    "packages/playwright-core/src/cli/driver.ts": "e9d60f99fc34f6a47f323f32f1a42469743b7868a5e76ad092b4a598b1e6ad71",
}
EXPECTED_IDS = {
    304: (
        "disconnected event should have browser as argument",
        "ee6df4b3a2217bd356a7-309fb651d7b47f0d6473",
    ),
    321: (
        "should set the browser connected state",
        "ee6df4b3a2217bd356a7-622e140cd95e69b765a5",
    ),
}


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
    _require(
        subprocess.check_output(
            ["git", "-C", str(UPSTREAM), "rev-parse", "HEAD"], text=True
        ).strip()
        == COMMIT,
        "pinned Playwright checkout commit changed",
    )
    _require(
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
                        "tests/library/browsertype-connect.spec.ts:304",
                        "tests/library/browsertype-connect.spec.ts:321",
                    ],
                    "candidateAdapter": "test-mode-playwright-core",
                    "project": "chromium-library",
                    "requiresBrowser": False,
                    "expectedTestsPerProject": 2,
                    "expectedFiles": 1,
                }
            ],
        },
        "synthetic lifecycle manifest changed",
    )


def _selected_tests(report: dict[str, Any]) -> list[dict[str, Any]]:
    found: list[dict[str, Any]] = []

    def visit(suite: dict[str, Any], titles: tuple[str, ...]) -> None:
        current = titles + (suite.get("title", ""),)
        for spec in suite.get("specs", []):
            line = spec.get("line")
            _require(line in EXPECTED_IDS, f"unexpected selected spec line: {line}")
            title, spec_id = EXPECTED_IDS[line]
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
            _require(test.get("projectId") == "chromium-library", "wrong Core project")
            _require(
                test.get("expectedStatus") == "passed", "unexpected expected status"
            )
            attempts = test.get("results")
            _require(
                isinstance(attempts, list) and len(attempts) == 1,
                "retries obscure diagnostic",
            )
            attempt = attempts[0]
            status = attempt.get("status")
            _require(
                status in {"passed", "failed"} and attempt.get("retry") == 0,
                "invalid attempt",
            )
            expected = status == "passed"
            _require(
                test.get("status") == ("expected" if expected else "unexpected"),
                "status mismatch",
            )
            _require(spec.get("ok") == expected, "spec status mismatch")
            errors = attempt.get("errors")
            _require(
                isinstance(errors, list) and bool(errors) != expected,
                "error list mismatch",
            )
            found.append(
                {
                    "line": line,
                    "title": title,
                    "specId": spec_id,
                    "status": test["status"],
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
        sorted(item["line"] for item in found) == [304, 321],
        "case selection is not exact",
    )
    stats = report.get("stats", {})
    expected = sum(item["status"] == "expected" for item in found)
    _require(
        stats.get("expected") == expected
        and stats.get("unexpected") == 2 - expected
        and stats.get("skipped") == 0
        and stats.get("flaky") == 0,
        "Core report statistics differ from the selected cases",
    )
    return sorted(found, key=lambda item: item["line"])


def _receipts(
    path: Path, mode: str, host: str, bind: str, passed: int
) -> dict[str, Any]:
    _require(
        path.is_file() and not path.is_symlink(), f"missing fixture receipt: {path}"
    )
    records = [json.loads(line) for line in path.read_text().splitlines() if line]
    _require(
        records and all(isinstance(item, dict) for item in records),
        "invalid fixture receipt",
    )
    _require(
        all(item.get("mode") == mode for item in records),
        "mixed client lanes in receipt",
    )
    _require(
        all(
            item.get("fixtureHost") == host and item.get("fixtureBind") == bind
            for item in records
        ),
        "receipt address drift",
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
        "preload source, mode or browser guard changed",
    )
    _require(
        any(
            item.get("kind") == "fixture-source-read" and item.get("workerIndex") == "0"
            for item in records
        ),
        "fixture adaptation did not reach worker",
    )
    fixture_records = [item for item in records if item.get("synthetic") is True]
    _require(
        fixture_records
        and all(
            item.get("browserProcessLaunched") is False for item in fixture_records
        ),
        "fixture claimed a browser launch",
    )
    _require(
        not any(
            item.get("kind") in {"start-error", "websocket-error"}
            for item in fixture_records
        ),
        "fixture protocol error",
    )
    by_kind = {
        kind: [item for item in fixture_records if item.get("kind") == kind]
        for kind in (
            "listening",
            "upgrade",
            "root-initialize",
            "bootstrap-sent",
            "close",
            "stop-requested",
            "stop",
        )
    }
    _require(
        len(by_kind["listening"]) == len(by_kind["stop"]) == 2,
        "expected two isolated synthetic servers",
    )
    _require(
        len({item.get("pid") for item in by_kind["listening"]}) == 2,
        "fixture process reuse changed",
    )
    _require(len(by_kind["stop-requested"]) == 2, "fixture teardown signal missing")
    _require(
        all(
            item.get("endpointPath") == "/browser-synthetic"
            for item in by_kind["listening"]
        ),
        "fixture WS path changed",
    )
    _require(
        len(by_kind["upgrade"])
        == len(by_kind["root-initialize"])
        == len(by_kind["bootstrap-sent"])
        and passed <= len(by_kind["bootstrap-sent"]) <= 2,
        "successful assertions lack matching WS/Root.initialize bootstrap receipts",
    )
    _require(
        all(
            item.get("sdkLanguage") == "javascript" and item.get("id") == 1
            for item in by_kind["root-initialize"]
        ),
        "Root.initialize command changed",
    )
    _require(
        all(
            item.get("frames") == 8 and item.get("version") == "155.0"
            for item in by_kind["bootstrap-sent"]
        ),
        "synthetic GUID graph changed",
    )
    clean_close_pids = {
        item.get("pid")
        for item in by_kind["close"]
        if item.get("forcedByFixtureTeardown") is False
    }
    _require(
        len(clean_close_pids) >= passed,
        "successful browser closes lack distinct clean WS close receipts",
    )
    return {kind: len(items) for kind, items in by_kind.items()}


def verify_directory(result_dir: Path) -> dict[str, Any]:
    result_dir = result_dir.resolve()
    _require(
        result_dir.is_dir() and not result_dir.is_symlink(),
        "result directory missing or symlinked",
    )
    verify_upstream()
    snapshot = result_dir / "candidate-package-snapshot"
    snapshot_manifest = result_dir / "candidate-package-snapshot.files.json"
    snapshot_data = _read_json(snapshot_manifest)
    artifact_digest = snapshot_data.get("artifactSha256")
    _require(isinstance(artifact_digest, str), "candidate artifact hash missing")
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
        "tests/library/browsertype-connect.spec.ts:304",
        "tests/library/browsertype-connect.spec.ts:321",
    ]
    lanes: dict[str, Any] = {}
    host: str | None = None
    bind: str | None = None
    for mode in ("oracle", "candidate"):
        result = result_dir / f"{mode}.json"
        provenance = result_dir / f"{mode}.provenance.json"
        receipt = result_dir / f"{mode}.receipt.jsonl"
        log = result_dir / f"{mode}.log"
        report = _read_json(result)
        source = _read_json(provenance)
        _require(log.is_file() and not log.is_symlink(), "lane log missing")
        _require(
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
        _require(
            source.get("fixtureHost") == host and source.get("fixtureBind") == bind,
            "paired fixture address changed",
        )
        _require(
            isinstance(host, str)
            and host.endswith(".ts.net")
            and isinstance(bind, str)
            and bind.startswith("100."),
            "paired Tailscale address missing",
        )
        if mode == "candidate":
            _require(
                source.get("candidate") == str(snapshot)
                and source.get("candidateProfile") == PROFILE
                and source.get("candidateArtifactSha256") == artifact_digest
                and source.get("candidateSourceSha256")
                == snapshot_data.get("candidateSourceSha256"),
                "candidate snapshot or selected addon differs from run",
            )
            _require(
                "[pwright-candidate]" in log.read_text(),
                "candidate did not load Pwright",
            )
        else:
            _require(
                source.get("candidate") is None
                and source.get("candidateProfile") is None,
                "oracle loaded candidate",
            )
            _require(
                "[pwright-candidate]" not in log.read_text(),
                "oracle log contains Pwright adapter",
            )
        selected = _selected_tests(report)
        passed = sum(item["status"] == "expected" for item in selected)
        _require(
            source.get("exitCode") == (0 if passed == 2 else 1),
            "lane process exit differs from report",
        )
        counts = _receipts(receipt, mode, host, bind, passed)
        if mode == "oracle":
            _require(
                passed == 2,
                "independent Playwright oracle must pass both lifecycle assertions",
            )
        lanes[mode] = {
            "selected": selected,
            "passed": passed,
            "resultSha256": sha256(result),
            "provenanceSha256": sha256(provenance),
            "receiptSha256": sha256(receipt),
            "logSha256": sha256(log),
            "wireReceiptCounts": counts,
        }
    return {
        "schema": 1,
        "scope": "two-unchanged-core-connect-lifecycle-assertions-with-synthetic-run-server-fixture",
        "classification": "official-derived-fixture-adapted-internal-diagnostic",
        "publicApiParityEvidence": False,
        "unadaptedOfficialPass": False,
        "fullPlaywrightServerSupport": False,
        "promotedToCoreLedger": False,
        "upstreamCommit": COMMIT,
        "upstreamSourceSha256": SOURCE_HASHES,
        "manifestSha256": sha256(MANIFEST),
        "preloadSha256": sha256(PRELOAD),
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
        "verified fixture-adapted Core connect lifecycle: "
        f"oracle {attestation['lanes']['oracle']['passed']}/2; "
        f"candidate {attestation['lanes']['candidate']['passed']}/2"
    )


if __name__ == "__main__":
    main()
