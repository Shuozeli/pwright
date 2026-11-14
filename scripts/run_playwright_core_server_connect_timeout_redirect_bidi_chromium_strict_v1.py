#!/usr/bin/env python3
"""Capture five pinned, browserless Core Bidi connect assertions in two lanes."""

from __future__ import annotations

import argparse
import json
import os
import socket
import subprocess
import sys
from pathlib import Path
from typing import Any

from official_candidate_package_snapshot_v1 import sha256, verify_snapshot
from run_playwright_official_conformance import _tailscale_runtime

ROOT = Path(__file__).resolve().parents[1]
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-v1.58.0")
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
SOURCE = "tests/library/browsertype-connect.spec.ts"
SOURCE_SHA256 = "9de4d9c8c71b8509887c81220f2eb0042e89e146c65d0311b020d191814ed2bc"
UPSTREAM_INPUTS = {
    SOURCE: SOURCE_SHA256,
    "tests/bidi/playwright.config.ts": (
        "9d9e2bb9df999850649c3126091a9492dd38957f581be5bc85871abbf9a0a6d8"
    ),
    "tests/bidi/expectationUtil.ts": (
        "f43f925e304f45363078f4cc8f2573dc3ebd453324de4021d20c718db6646adb"
    ),
    "tests/config/baseTest.ts": "4fc41689120135a7cfbe066431ebc5b3751b1a6745e2d48e7013336da556a52c",
    "tests/config/browserTest.ts": (
        "0ef5440e164d262e7c3d01523448b5d3c4d50b99060604c20f168f04419a13d5"
    ),
    "tests/config/commonFixtures.ts": (
        "cc8aad21b0684cc4958a9d76fa86090db77e93ba855c655773033da9025b9fcc"
    ),
    "tests/config/platformFixtures.ts": (
        "bb414c74d5013fbc5900434fa1bfb0b58210899862265ec9a701372a4a27b1d3"
    ),
    "tests/config/serverFixtures.ts": (
        "4c0039a149bd61e8707adddc626b345632561bd343749293ddbe9458f6ac617b"
    ),
    "tests/config/testModeFixtures.ts": (
        "5560c5215cb10585ff74e827f71d229c8e45c2b7b0d663691476b9707252f3a8"
    ),
    "tests/config/testMode.ts": "7c12e6bd3fa7e2e3e4ff296277f4a60656d4d8a0c536cb55d9d6c1e83016f575",
    "tests/config/testserver/index.ts": (
        "4330e0b94c88fdba07401efd3f4cd30a229bb48da037d8c880cd79ba0bae7307"
    ),
}
MANIFEST = (
    ROOT
    / "tests/official"
    / "playwright-core-server-connect-timeout-redirect-bidi-chromium-strict-v1-manifest.json"
)
REDIRECT_GUARD = (
    ROOT / "tests/official/playwright-core-server-connect-redirect-timeout-strict-v1-preload.cjs"
)
TIMEOUT_GUARD = (
    ROOT / "tests/official/playwright-core-server-connect-timeouts-strict-v1-preload.cjs"
)
OWNER = ROOT / "tests/official/playwright-core-browser-type-owner-v2-preload.cjs"
COMMON = ROOT / "tests/official/playwright-core-server-runtime.cjs"
CANDIDATE_PRELOAD = ROOT / "tests/official/playwright-core-candidate-preload.cjs"
PINNED_HARNESS = {
    REDIRECT_GUARD: "8ec3f6bc852d54b90b913e9fb328d976e8dd16845ce54c0d8fcdf5e224699e65",
    TIMEOUT_GUARD: "dc1b4f7d460cc428d89c6d8273bc3d4171b2f04c2b8588ce98b567565fe0833f",
    OWNER: "b7dc4e3be3aa4c07ac069bc07adfa5a2e65456efc6fc438e49a84a907ffc06ce",
    COMMON: "58278e6c976da47074a1eb8d682916ebc3d0a41975fe89fe9270d1a664c396fd",
    CANDIDATE_PRELOAD: "d17be590ff9334b0d99dd16d0078e4b434a39eae21568b799314ababc6bbf924",
}
RESULT_DIR = (
    ROOT
    / ".work/official-conformance/results"
    / "core-server-connect-timeout-redirect-bidi-chromium-strict-v1"
    / "strict-pair-20261020-v1"
)
SUITE = "bidi-chromium-server-connect-timeout-redirect-strict-v1"
PROJECT = "bidi-chromium-library"
PROFILE = "playwright-server-connect-session"
SNAPSHOT = (
    ROOT
    / ".work/official-conformance/artifacts"
    / "playwright-server-connect-http-header-order-v1-20261012"
    / "candidate-package-snapshot"
)
SNAPSHOT_MANIFEST = SNAPSHOT.parent / "candidate-package-snapshot.files.json"
SNAPSHOT_MANIFEST_SHA256 = "a79c22bc1d160cc46379806820ee10d06ad4762aee3c037522a798cb129599b8"
SOURCE_PACKAGE = SNAPSHOT.parent / "candidate-package"
ARTIFACT_SHA256 = "0f1c8b1c6140206bdb4692d88ff1d1b9fb235b883739f176f720c3b59361523e"
CANDIDATE_SOURCE_SHA256 = "bddad5646a880efc80a94d7bf4b975e4eb06691562ac6602a8df29c3cf97afe2"
SCOPE = (
    "official-browserless-core-server-connect-timeout-redirect-bidi-chromium-internal-diagnostic-v1"
)
FIXTURE_ADAPTATION = "two-literal-localhost-ws-urls-plus-common-server-bind-host"
EXPECTED = {
    "ee6df4b3a2217bd356a7-5817a5ed954cfa532ace": (
        221,
        5,
        "launchServer",
        "should timeout in socket while connecting",
    ),
    "ee6df4b3a2217bd356a7-5b07f2a5e757a16ed468": (
        221,
        5,
        "run-server",
        "should timeout in socket while connecting",
    ),
    "ee6df4b3a2217bd356a7-30b6e0f369ffebc5d854": (
        228,
        5,
        "launchServer",
        "should timeout in connect while connecting",
    ),
    "ee6df4b3a2217bd356a7-50b361500ef280984f89": (
        228,
        5,
        "run-server",
        "should timeout in connect while connecting",
    ),
    "ee6df4b3a2217bd356a7-27eeb64a07b3673ac59b": (
        1056,
        1,
        "standalone",
        "should timeout after redirect when connecting over http",
    ),
}
INPUT_FILES = (
    MANIFEST,
    *PINNED_HARNESS,
    ROOT / "scripts/official_candidate_package_snapshot_v1.py",
    ROOT / "scripts/run_playwright_official_conformance.py",
    ROOT / "scripts/official_result_destination.py",
    ROOT / "scripts/verify_playwright_official_provenance.py",
    Path(__file__).resolve(),
    ROOT
    / "scripts/verify_playwright_core_server_connect_timeout_redirect_bidi_chromium_strict_v1.py",
    ROOT
    / "scripts"
    / "test_verify_playwright_core_server_connect_timeout_redirect_bidi_chromium_strict_v1.py",
)


def paths(mode: str) -> dict[str, Path]:
    if mode not in {"oracle", "candidate"}:
        raise ValueError("invalid paired lane")
    result = RESULT_DIR / f"{mode}-v1.json"
    return {
        "result": result,
        "provenance": result.with_suffix(".provenance.json"),
        "receipt": result.with_suffix(".guard-receipt.jsonl"),
        "log": result.with_suffix(".log"),
        "strict": result.with_suffix(".strict.json"),
    }


def check_inputs() -> dict[str, Any]:
    if (
        subprocess.check_output(
            ["git", "-C", str(UPSTREAM), "rev-parse", "HEAD"], text=True
        ).strip()
        != COMMIT
    ):
        raise ValueError("upstream commit drifted")
    if subprocess.check_output(
        ["git", "-C", str(UPSTREAM), "status", "--porcelain", "--untracked-files=no"],
        text=True,
    ).strip():
        raise ValueError("upstream tracked source is dirty")
    if any(sha256(UPSTREAM / name) != digest for name, digest in UPSTREAM_INPUTS.items()):
        raise ValueError("pinned upstream source drifted")
    if any(sha256(path) != digest for path, digest in PINNED_HARNESS.items()):
        raise ValueError("pinned preload drifted")
    expected_manifest = {
        "schema": 1,
        "repository": "https://github.com/microsoft/playwright.git",
        "tag": "v1.58.0",
        "commit": COMMIT,
        "suites": [
            {
                "id": SUITE,
                "role": "browser-independent-fixture-host-adapted-diagnostic",
                "runner": "packages/playwright/cli.js",
                "config": "tests/bidi/playwright.config.ts",
                "args": [f"{SOURCE}:221", f"{SOURCE}:228", f"{SOURCE}:1056"],
                "candidateAdapter": "test-mode-playwright-core",
                "project": PROJECT,
                "requiresBrowser": False,
                "expectedTestsPerProject": 5,
                "expectedFiles": 1,
            }
        ],
    }
    if json.loads(MANIFEST.read_text()) != expected_manifest:
        raise ValueError("five-row manifest drifted")
    source = (UPSTREAM / SOURCE).read_text()
    snippets = (
        "test('should timeout in socket while connecting', async ({ connect, server }) => {",
        "test('should timeout in connect while connecting', async ({ connect, server }) => {",
        "test('should timeout after redirect when connecting over http', "
        "async ({ connect, server }) => {",
        "expect(e.message).toContain('browserType.connect: Timeout 1000ms exceeded');",
        "expect(e.message).toContain('browserType.connect: Timeout 100ms exceeded');",
        "expect(error.message).toContain('Timeout 2000ms exceeded.');",
        "await expect.poll(() => aborted).toBe(true);",
    )
    if any(source.count(snippet) != 1 for snippet in snippets):
        raise ValueError("selected upstream assertions drifted")
    for line in (221, 228, 1056):
        body = source.splitlines()[line - 1 : line + (12 if line == 1056 else 7)]
        if not body or any(
            "startRemoteServer" in part or "browserType.launch" in part for part in body
        ):
            raise ValueError("selected official test requires a browser")
    snapshot = verify_snapshot(
        SNAPSHOT,
        SNAPSHOT_MANIFEST,
        source_package=SOURCE_PACKAGE,
        profile=PROFILE,
        artifact_sha256=ARTIFACT_SHA256,
    )
    if (
        sha256(SNAPSHOT_MANIFEST) != SNAPSHOT_MANIFEST_SHA256
        or snapshot["candidateSourceSha256"] != CANDIDATE_SOURCE_SHA256
    ):
        raise ValueError("pinned candidate snapshot drifted")
    return snapshot


def check_runtime() -> None:
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
        raise ValueError("strict paired capture rejects inherited runtime overrides")
    bind, host = _tailscale_runtime(os.environ.copy())
    if socket.gethostbyname(host) != bind:
        raise ValueError("Tailscale MagicDNS does not resolve to the bind address")
    # Five single-attempt cases can use worker indexes 0..4 if failures restart
    # Playwright's one-at-a-time worker; each worker owns four fixture ports.
    for port in range(8907, 8927):
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
            listener.bind((bind, port))


def selected_tests(report: dict[str, Any]) -> list[dict[str, Any]]:
    selected = []

    def visit(suite: dict[str, Any], kind: str = "standalone") -> None:
        if suite.get("title") in {"launchServer", "run-server"}:
            kind = suite["title"]
        for spec in suite.get("specs", []):
            spec_id = spec["id"]
            if spec_id not in EXPECTED:
                raise ValueError("official report contains an unselected spec")
            line, column, expected_kind, title = EXPECTED[spec_id]
            if (
                spec["file"],
                spec["line"],
                spec["column"],
                spec["title"],
                kind,
            ) != (
                "library/browsertype-connect.spec.ts",
                line,
                column,
                title,
                expected_kind,
            ):
                raise ValueError("official report spec identity drifted")
            if len(spec["tests"]) != 1:
                raise ValueError("official report did not run one project invocation")
            test = spec["tests"][0]
            if test["projectId"] != PROJECT or test["expectedStatus"] != "passed":
                raise ValueError("official report changed its project or expectation")
            if len(test["results"]) != 1:
                raise ValueError("official report retried or omitted an attempt")
            attempt = test["results"][0]
            if (
                attempt["retry"] != 0
                or attempt["status"] not in {"passed", "failed"}
                or type(attempt["workerIndex"]) is not int
                or not 0 <= attempt["workerIndex"] <= 4
            ):
                raise ValueError("official report attempt is malformed")
            passed = attempt["status"] == "passed"
            if test["status"] != ("expected" if passed else "unexpected") or spec["ok"] != passed:
                raise ValueError("official report status disagrees with the attempt")
            if bool(attempt["errors"]) == passed:
                raise ValueError("official report error list disagrees with the attempt")
            selected.append(
                {
                    "specId": spec_id,
                    "project": PROJECT,
                    "kind": kind,
                    "line": line,
                    "title": title,
                    "status": test["status"],
                    "attempts": [
                        {
                            "retry": attempt["retry"],
                            "status": attempt["status"],
                            "workerIndex": attempt["workerIndex"],
                        }
                    ],
                    "errors": [str(error.get("message", "")) for error in attempt["errors"]],
                }
            )
        for child in suite.get("suites", []):
            visit(child, kind)

    if report["errors"] != []:
        raise ValueError("official runner reported suite errors")
    for suite in report["suites"]:
        visit(suite)
    if len(selected) != 5 or {item["specId"] for item in selected} != set(EXPECTED):
        raise ValueError("official runner did not execute the exact five specs")
    stats = report["stats"]
    if (
        stats["expected"] != sum(item["status"] == "expected" for item in selected)
        or stats["unexpected"] != sum(item["status"] == "unexpected" for item in selected)
        or stats["skipped"] != 0
        or stats["flaky"] != 0
    ):
        raise ValueError("official report statistics disagree with selected results")
    return sorted(selected, key=lambda item: item["specId"])


def capture_lane(mode: str, inputs: dict[str, str]) -> dict[str, Any]:
    output = paths(mode)
    output["receipt"].open("x").close()
    env = os.environ.copy()
    env["NODE_OPTIONS"] = f"--require={REDIRECT_GUARD} --require={TIMEOUT_GUARD}"
    env["PWRIGHT_CORE_SERVER_REDIRECT_TIMEOUT_V1_MODE"] = mode
    env["PWRIGHT_CORE_SERVER_REDIRECT_TIMEOUT_V1_RECEIPT"] = str(output["receipt"])
    env["PWRIGHT_CORE_SERVER_CONNECT_TIMEOUTS_V1_MODE"] = mode
    env["PWRIGHT_CORE_SERVER_CONNECT_TIMEOUTS_V1_RECEIPT"] = str(output["receipt"])
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
        SUITE,
        "--manifest",
        str(MANIFEST),
        "--workers",
        "1",
        "--result",
        str(output["result"]),
    ]
    if mode == "candidate":
        command.extend(["--candidate", str(SNAPSHOT), "--candidate-profile", PROFILE])
    completed = subprocess.run(
        command,
        cwd=ROOT,
        env=env,
        capture_output=True,
        text=True,
        check=False,
        timeout=240,
    )
    output["log"].write_text(completed.stdout + completed.stderr)
    if {path: sha256(ROOT / path) for path in inputs} != inputs:
        raise ValueError("local capture source changed during execution")
    check_inputs()
    if not output["result"].is_file() or not output["provenance"].is_file():
        raise RuntimeError(
            f"{mode} runner produced no result/provenance (exit {completed.returncode}): "
            f"{output['log'].read_text()[-3000:]}"
        )
    selected = selected_tests(json.loads(output["result"].read_text()))
    expected_exit = 0 if all(item["status"] == "expected" for item in selected) else 1
    if completed.returncode != expected_exit:
        raise ValueError(f"{mode} runner exit does not match the five selected results")
    if mode == "oracle" and expected_exit != 0:
        raise ValueError("pinned upstream oracle unexpectedly failed")
    if ("[pwright-candidate]" in output["log"].read_text()) != (mode == "candidate"):
        raise ValueError("candidate adapter runtime marker disagrees with lane")
    return {
        "suite": SUITE,
        "project": PROJECT,
        "result": str(output["result"]),
        "resultSha256": sha256(output["result"]),
        "provenanceSha256": sha256(output["provenance"]),
        "guardReceiptSha256": sha256(output["receipt"]),
        "logSha256": sha256(output["log"]),
        "exitCode": completed.returncode,
        "selected": selected,
    }


def capture() -> dict[str, Any]:
    if sys.flags.optimize:
        raise RuntimeError("strict Core evidence requires Python assertions enabled")
    check_runtime()
    snapshot = check_inputs()
    inputs = {str(path.relative_to(ROOT)): sha256(path) for path in INPUT_FILES}
    RESULT_DIR.mkdir(parents=True, exist_ok=False)
    lanes = {}
    for mode in ("oracle", "candidate"):
        run = capture_lane(mode, inputs)
        strict = {
            "schema": 1,
            "scope": SCOPE,
            "sourceClassification": "implementation-internal",
            "fixtureAdapted": True,
            "fixtureAdaptation": FIXTURE_ADAPTATION,
            "publicApiParityEvidence": False,
            "promotedToCoreLedger": False,
            "realBrowserServer": False,
            "mode": mode,
            "upstreamCommit": COMMIT,
            "upstreamInputs": UPSTREAM_INPUTS,
            "inputs": inputs,
            "candidateProfile": PROFILE if mode == "candidate" else None,
            "candidateArtifactSha256": ARTIFACT_SHA256 if mode == "candidate" else None,
            "candidateSnapshot": str(SNAPSHOT) if mode == "candidate" else None,
            "candidateSnapshotManifestSha256": SNAPSHOT_MANIFEST_SHA256
            if mode == "candidate"
            else None,
            "candidateSourceSha256": snapshot["candidateSourceSha256"]
            if mode == "candidate"
            else None,
            "run": run,
        }
        with paths(mode)["strict"].open("x") as stream:
            json.dump(strict, stream, indent=2, sort_keys=True)
            stream.write("\n")
        lanes[mode] = strict
    from verify_playwright_core_server_connect_timeout_redirect_bidi_chromium_strict_v1 import (
        verify_pair,
    )

    verify_pair(paths("oracle")["strict"], paths("candidate")["strict"])
    return lanes


def main() -> None:
    argparse.ArgumentParser(description=__doc__).parse_args()
    lanes = capture()
    for mode, strict in lanes.items():
        print(f"{mode}: {[item['status'] for item in strict['run']['selected']]}")
    print(f"diagnostic only, not promoted: {RESULT_DIR}")


if __name__ == "__main__":
    main()
