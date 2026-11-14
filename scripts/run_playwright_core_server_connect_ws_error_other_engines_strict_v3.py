#!/usr/bin/env python3
"""Capture four pinned, browser-free Core WS-error assertions in two engines."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any

from official_candidate_package_snapshot_v1 import (
    create_snapshot,
    sha256,
    verify_snapshot,
)
from run_playwright_core_server_connect_ws_error_strict_v2 import (
    assert_public_errors_redacted,
    write_json_exclusive,
)

ROOT = Path(__file__).resolve().parents[1]
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-v1.58.0")
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
SOURCE = "tests/library/browsertype-connect.spec.ts"
SOURCE_SHA256 = "9de4d9c8c71b8509887c81220f2eb0042e89e146c65d0311b020d191814ed2bc"
UPSTREAM_INPUTS = {
    SOURCE: SOURCE_SHA256,
    "tests/config/serverFixtures.ts": "4c0039a149bd61e8707adddc626b345632561bd343749293ddbe9458f6ac617b",
    "tests/config/browserTest.ts": "0ef5440e164d262e7c3d01523448b5d3c4d50b99060604c20f168f04419a13d5",
    "tests/config/testModeFixtures.ts": "5560c5215cb10585ff74e827f71d229c8e45c2b7b0d663691476b9707252f3a8",
    "tests/config/testMode.ts": "7c12e6bd3fa7e2e3e4ff296277f4a60656d4d8a0c536cb55d9d6c1e83016f575",
    "tests/library/playwright.config.ts": "338eb705bb4a17322dfba39d1c6770613bf8cf62cfec5c43bf7cece3e5db8146",
}
MANIFEST = (
    ROOT
    / "tests/official/playwright-core-server-connect-ws-error-other-engines-strict-v3-manifest.json"
)
GUARD = (
    ROOT
    / "tests/official/playwright-core-server-connect-ws-error-strict-v2-preload.cjs"
)
OWNER = ROOT / "tests/official/playwright-core-browser-type-owner-v2-preload.cjs"
COMMON = ROOT / "tests/official/playwright-core-server-runtime.cjs"
CANDIDATE_PRELOAD = ROOT / "tests/official/playwright-core-candidate-preload.cjs"
PINNED_HARNESS = {
    GUARD: "8961683cd908e302ec6a2319c2562e61972930986c70071db8fec2059b02ae71",
    OWNER: "b7dc4e3be3aa4c07ac069bc07adfa5a2e65456efc6fc438e49a84a907ffc06ce",
    COMMON: "58278e6c976da47074a1eb8d682916ebc3d0a41975fe89fe9270d1a664c396fd",
    CANDIDATE_PRELOAD: "d17be590ff9334b0d99dd16d0078e4b434a39eae21568b799314ababc6bbf924",
}
PROFILE = "playwright-server-connect-ws-error"
ARTIFACT_SHA256 = "cbdcb780b8c8f04dfb51ddc867b13a4964c8696aeac0e2a4737f113554fdc1d0"
RESULT_PARENT = (
    ROOT
    / ".work/official-conformance/results/core-server-connect-ws-error-other-engines-strict-v3"
)
SOURCE_PACKAGE = ROOT / "packages/pwright"
SNAPSHOT_PACKAGE = RESULT_PARENT / "candidate-package-snapshot-v3"
SNAPSHOT_MANIFEST = RESULT_PARENT / "candidate-package-snapshot-v3.files.json"
SCOPE = "unchanged-core-ws-dns-error-firefox-webkit-four-cases-diagnostic-only-v3"
PROJECTS = {
    "firefox-library": {
        "suite": "library-firefox-server-connect-ws-error-strict-v3",
        "specs": {
            "ee6df4b3a2217bd356a7-7ff2dc18d1e08d985682": "launchServer",
            "ee6df4b3a2217bd356a7-95a86c47485eb83c0016": "run-server",
        },
    },
    "webkit-library": {
        "suite": "library-webkit-server-connect-ws-error-strict-v3",
        "specs": {
            "ee6df4b3a2217bd356a7-6d5f8a4e2638a54cc171": "launchServer",
            "ee6df4b3a2217bd356a7-184cfa77707c96da434b": "run-server",
        },
    },
}
INPUT_FILES = (
    MANIFEST,
    GUARD,
    OWNER,
    COMMON,
    CANDIDATE_PRELOAD,
    ROOT / "scripts/official_candidate_package_snapshot_v1.py",
    ROOT / "scripts/run_playwright_core_server_connect_ws_error_strict_v2.py",
    ROOT / "scripts/run_playwright_official_conformance.py",
    ROOT / "scripts/official_result_destination.py",
    ROOT
    / "scripts/test_verify_playwright_core_server_connect_ws_error_other_engines_strict_v3.py",
    Path(__file__).resolve(),
    ROOT
    / "scripts/verify_playwright_core_server_connect_ws_error_other_engines_strict_v3.py",
)


def check_browserless_source_and_manifest() -> None:
    assert (
        subprocess.check_output(
            ["git", "-C", str(UPSTREAM), "rev-parse", "HEAD"], text=True
        ).strip()
        == COMMIT
    )
    assert not subprocess.check_output(
        ["git", "-C", str(UPSTREAM), "status", "--porcelain", "--untracked-files=no"],
        text=True,
    ).strip()
    assert all(
        sha256(UPSTREAM / name) == digest for name, digest in UPSTREAM_INPUTS.items()
    )
    assert all(sha256(path) == digest for path, digest in PINNED_HARNESS.items())
    expected_suites = []
    for project, selection in PROJECTS.items():
        expected_suites.append(
            {
                "id": selection["suite"],
                "role": "browser-independent-official-subset",
                "runner": "packages/playwright/cli.js",
                "config": "tests/library/playwright.config.ts",
                "args": [f"{SOURCE}:122"],
                "candidateAdapter": "test-mode-playwright-core",
                "project": project,
                "requiresBrowser": False,
                "expectedTestsPerProject": 2,
                "expectedFiles": 1,
            }
        )
    assert json.loads(MANIFEST.read_text()) == {
        "schema": 1,
        "repository": "https://github.com/microsoft/playwright.git",
        "tag": "v1.58.0",
        "commit": COMMIT,
        "suites": expected_suites,
    }
    source = (UPSTREAM / SOURCE).read_text()
    marker = "test('should print ws error', async ({ connect, server }) => {"
    assert source.count(marker) == 1
    body = source[
        source.index(marker) : source.index(
            "\n    test(", source.index(marker) + len(marker)
        )
    ]
    assert "startRemoteServer" not in body and "browserType.launch" not in body
    for assertion in (
        "const error = await connect(`ws://does-not-exist.problem-domain:10987?secret=MYSECRET`).catch(e => e);",
        "expect(error.message).toContain('<ws connecting> ws://does-not-exist.problem-domain:10987/');",
        "expect(error.message).toContain('<ws error>');",
        "expect(error.message).toContain('getaddrinfo');",
        "expect(error.message).not.toContain('secret=MYSECRET');",
    ):
        assert body.count(assertion) == 1
    browser_fixtures = (UPSTREAM / "tests/config/browserTest.ts").read_text()
    assert (
        "browserType: [async ({ playwright, browserName, mode }, run) => {"
        in browser_fixtures
    )
    assert "await run(playwright[browserName]);" in browser_fixtures
    test_mode = (UPSTREAM / "tests/config/testMode.ts").read_text()
    assert "return require('playwright-core');" in test_mode


def selected_tests(report: dict[str, Any], project: str) -> list[dict[str, Any]]:
    expected_ids = PROJECTS[project]["specs"]
    selected: list[dict[str, Any]] = []

    def visit(suite: dict[str, Any]) -> None:
        for spec in suite.get("specs", []):
            spec_id = spec["id"]
            assert spec_id in expected_ids
            assert (spec["file"], spec["line"], spec["column"], spec["title"]) == (
                "library/browsertype-connect.spec.ts",
                122,
                5,
                "should print ws error",
            )
            assert len(spec["tests"]) == 1
            test = spec["tests"][0]
            assert test["projectId"] == project
            assert test["expectedStatus"] == "passed"
            assert test["status"] in {"expected", "unexpected"}
            assert len(test["results"]) == 1
            attempt = test["results"][0]
            assert attempt["retry"] == 0 and attempt["status"] in {"passed", "failed"}
            passed = attempt["status"] == "passed"
            assert test["status"] == ("expected" if passed else "unexpected")
            assert spec["ok"] == passed
            errors = attempt["errors"]
            assert isinstance(errors, list) and bool(errors) != passed
            selected.append(
                {
                    "project": project,
                    "kind": expected_ids[spec_id],
                    "specId": spec_id,
                    "line": 122,
                    "title": spec["title"],
                    "status": test["status"],
                    "attempts": [
                        {
                            "retry": attempt["retry"],
                            "status": attempt["status"],
                            "workerIndex": attempt["workerIndex"],
                        }
                    ],
                    "errors": [str(error.get("message", "")) for error in errors],
                }
            )
        for child in suite.get("suites", []):
            visit(child)

    assert report["errors"] == []
    for suite in report["suites"]:
        visit(suite)
    assert len(selected) == 2 and {item["specId"] for item in selected} == set(
        expected_ids
    )
    assert report["stats"]["expected"] == sum(
        item["status"] == "expected" for item in selected
    )
    assert report["stats"]["unexpected"] == sum(
        item["status"] == "unexpected" for item in selected
    )
    assert report["stats"]["skipped"] == report["stats"]["flaky"] == 0
    assert_public_errors_redacted(selected)
    return sorted(selected, key=lambda item: item["specId"])


def _paths(mode: str, project: str) -> dict[str, Path]:
    result = RESULT_PARENT / f"{mode}-{project}-v3.json"
    return {
        "result": result,
        "provenance": result.with_suffix(".provenance.json"),
        "receipt": result.with_suffix(".guard-receipt.jsonl"),
        "log": result.with_suffix(".log"),
    }


def run(mode: str) -> dict[str, Any]:
    if sys.flags.optimize:
        raise RuntimeError("strict Core evidence requires Python assertions enabled")
    forbidden = (
        "NODE_OPTIONS",
        "PWTEST_MODE",
        "PWPAGE_IMPL",
        "PWTEST_USE_BIDI_EXPECTATIONS",
        "PWRIGHT_CANDIDATE_CDP_ENDPOINT",
        "PWRIGHT_ORACLE_CDP_ENDPOINT",
        "PWRIGHT_CDP_ENDPOINT",
        "PWRIGHT_CDP_WS_URL",
        "PWRIGHT_NATIVE_PROFILE",
        "PWRIGHT_NODE_CANDIDATE",
        "PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM",
        "PWRIGHT_OFFICIAL_SERVER_IP",
        "PWRIGHT_OFFICIAL_SERVER_HOST",
    )
    if any(os.environ.get(key) for key in forbidden):
        raise ValueError(
            "strict Server-connect diagnostic rejects inherited runtime overrides"
        )
    check_browserless_source_and_manifest()
    artifact = SOURCE_PACKAGE / "native" / PROFILE / "pwright.node"
    if mode == "candidate" and (
        not artifact.is_file() or sha256(artifact) != ARTIFACT_SHA256
    ):
        raise ValueError(
            "pinned Server-connect candidate artifact is missing or changed"
        )
    RESULT_PARENT.mkdir(parents=True, exist_ok=True)
    strict_path = RESULT_PARENT / f"{mode}-v3.strict.json"
    destinations = [strict_path]
    for project in PROJECTS:
        destinations.extend(_paths(mode, project).values())
    if mode == "candidate":
        destinations += [SNAPSHOT_PACKAGE, SNAPSHOT_MANIFEST]
    if any(path.exists() or path.is_symlink() for path in destinations):
        raise ValueError("strict Server-connect diagnostic destination already exists")
    inputs = {str(path.relative_to(ROOT)): sha256(path) for path in INPUT_FILES}
    snapshot = None
    if mode == "candidate":
        snapshot = create_snapshot(
            SOURCE_PACKAGE,
            SNAPSHOT_PACKAGE,
            SNAPSHOT_MANIFEST,
            profile=PROFILE,
            artifact_sha256=ARTIFACT_SHA256,
        )
    runs = []
    for project, selection in PROJECTS.items():
        paths = _paths(mode, project)
        paths["receipt"].open("x").close()
        env = os.environ.copy()
        env["NODE_OPTIONS"] = f"--require={GUARD}"
        env["PWRIGHT_CORE_SERVER_CONNECT_WS_ERROR_MODE"] = mode
        env["PWRIGHT_CORE_SERVER_CONNECT_WS_ERROR_RECEIPT"] = str(paths["receipt"])
        command = [
            "uv",
            "run",
            "--no-project",
            "--with",
            "pytest==8.4.2",
            str(ROOT / "scripts/run_playwright_official_conformance.py"),
            mode,
            str(UPSTREAM),
            "--suite",
            selection["suite"],
            "--manifest",
            str(MANIFEST),
            "--workers",
            "1",
            "--result",
            str(paths["result"]),
        ]
        if mode == "candidate":
            command += [
                "--candidate",
                str(SNAPSHOT_PACKAGE),
                "--candidate-profile",
                PROFILE,
            ]
        completed = subprocess.run(
            command, cwd=ROOT, env=env, capture_output=True, text=True, check=False
        )
        paths["log"].write_text(completed.stdout + completed.stderr)
        assert all(sha256(ROOT / name) == digest for name, digest in inputs.items())
        check_browserless_source_and_manifest()
        if not paths["result"].is_file() or not paths["provenance"].is_file():
            raise RuntimeError(
                f"official {project} runner produced no result/provenance (exit {completed.returncode}): "
                f"{paths['log'].read_text()[-3000:]}"
            )
        selected = selected_tests(json.loads(paths["result"].read_text()), project)
        assert completed.returncode == (
            0 if all(item["status"] == "expected" for item in selected) else 1
        )
        if mode == "oracle":
            assert all(item["status"] == "expected" for item in selected)
        assert ("[pwright-candidate]" in paths["log"].read_text()) == (
            mode == "candidate"
        )
        runs.append(
            {
                "project": project,
                "suite": selection["suite"],
                "result": str(paths["result"]),
                "resultSha256": sha256(paths["result"]),
                "provenanceSha256": sha256(paths["provenance"]),
                "guardReceiptSha256": sha256(paths["receipt"]),
                "logSha256": sha256(paths["log"]),
                "exitCode": completed.returncode,
                "selected": selected,
            }
        )
    if snapshot is not None:
        assert (
            verify_snapshot(
                SNAPSHOT_PACKAGE,
                SNAPSHOT_MANIFEST,
                source_package=SOURCE_PACKAGE,
                profile=PROFILE,
                artifact_sha256=ARTIFACT_SHA256,
            )
            == snapshot
        )
    strict = {
        "schema": 1,
        "scope": SCOPE,
        "sourceClassification": "implementation-internal",
        "fixtureAdapted": True,
        "publicApiParityEvidence": False,
        "promotedToCoreLedger": False,
        "mode": mode,
        "upstreamCommit": COMMIT,
        "upstreamInputs": UPSTREAM_INPUTS,
        "inputs": inputs,
        "candidateProfile": PROFILE if mode == "candidate" else None,
        "candidateArtifactSha256": ARTIFACT_SHA256 if mode == "candidate" else None,
        "candidateSnapshot": str(SNAPSHOT_PACKAGE) if mode == "candidate" else None,
        "candidateSnapshotManifestSha256": sha256(SNAPSHOT_MANIFEST)
        if mode == "candidate"
        else None,
        "candidateSourceSha256": snapshot["candidateSourceSha256"]
        if snapshot is not None
        else None,
        "runs": runs,
    }
    write_json_exclusive(strict_path, strict)
    return strict


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("oracle", "candidate"))
    strict = run(parser.parse_args().mode)
    print(
        f"Core Server-connect other-engine WS error {strict['mode']}: "
        f"{[item['status'] for run in strict['runs'] for item in run['selected']]}"
    )


if __name__ == "__main__":
    main()
