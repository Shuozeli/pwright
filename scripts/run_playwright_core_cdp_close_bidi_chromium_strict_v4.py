"""Capture the unchanged Core CDP Close assertion in three Bidi-named projects."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import Any

import run_playwright_core_cdp_error_slice_strict_v1 as cdp_base
from official_candidate_package_snapshot_v1 import sha256, verify_snapshot

ROOT = Path(__file__).resolve().parents[1]
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-v1.58.0")
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
SOURCE = "tests/library/chromium/connect-over-cdp.spec.ts"
SOURCE_SHA256 = "be1e552ad56652303ce851bf026aef01cbba7ffd634e8bfc36c3313e2586226c"
UPSTREAM_INPUTS = {
    SOURCE: SOURCE_SHA256,
    "tests/bidi/playwright.config.ts": "9d9e2bb9df999850649c3126091a9492dd38957f581be5bc85871abbf9a0a6d8",
    "tests/bidi/expectationUtil.ts": "f43f925e304f45363078f4cc8f2573dc3ebd453324de4021d20c718db6646adb",
    "tests/config/baseTest.ts": "4fc41689120135a7cfbe066431ebc5b3751b1a6745e2d48e7013336da556a52c",
    "tests/config/browserTest.ts": "0ef5440e164d262e7c3d01523448b5d3c4d50b99060604c20f168f04419a13d5",
    "tests/config/commonFixtures.ts": "cc8aad21b0684cc4958a9d76fa86090db77e93ba855c655773033da9025b9fcc",
    "tests/config/platformFixtures.ts": "bb414c74d5013fbc5900434fa1bfb0b58210899862265ec9a701372a4a27b1d3",
    "tests/config/serverFixtures.ts": "4c0039a149bd61e8707adddc626b345632561bd343749293ddbe9458f6ac617b",
    "tests/config/testModeFixtures.ts": "5560c5215cb10585ff74e827f71d229c8e45c2b7b0d663691476b9707252f3a8",
    "tests/config/testMode.ts": "7c12e6bd3fa7e2e3e4ff296277f4a60656d4d8a0c536cb55d9d6c1e83016f575",
    "tests/config/testserver/index.ts": "4330e0b94c88fdba07401efd3f4cd30a229bb48da037d8c880cd79ba0bae7307",
}
MANIFEST = (
    ROOT
    / "tests/official/playwright-core-cdp-close-bidi-chromium-strict-v4-manifest.json"
)
URL_ONLY_PRELOAD = (
    ROOT / "tests/official/playwright-core-cdp-error-slice-strict-v1-preload.cjs"
)
SERVER_PRELOAD = ROOT / "tests/official/playwright-core-server-runtime.cjs"
CANDIDATE_PRELOAD = ROOT / "tests/official/playwright-core-candidate-preload.cjs"
PINNED_HARNESS = {
    URL_ONLY_PRELOAD: "27b6feb7bba6cd7c2e879f648667b3f0f3eea5b8c4cb8fb3dcd70bfc1c5488ee",
    SERVER_PRELOAD: "58278e6c976da47074a1eb8d682916ebc3d0a41975fe89fe9270d1a664c396fd",
    CANDIDATE_PRELOAD: "d17be590ff9334b0d99dd16d0078e4b434a39eae21568b799314ababc6bbf924",
}
PROFILE = "official-conformance-page-cdp-errors"
ARTIFACT_SHA256 = "0d7dcb931c42d5e912b594ec20fd12e202cc76590c7cab93187fe6829388f5c6"
SOURCE_PACKAGE = ROOT / "packages/pwright"
FROZEN_SNAPSHOT = (
    ROOT
    / ".work/official-conformance/results/core-cdp-error-slice-strict-v3/candidate-package-snapshot-v1"
)
FROZEN_MANIFEST = (
    ROOT
    / ".work/official-conformance/results/core-cdp-error-slice-strict-v3/candidate-package-snapshot-v1.files.json"
)
FROZEN_MANIFEST_SHA256 = (
    "c90f2f9d9b7904610abfeb3a37ee720e988b37748bedf8849cf36c7658534115"
)
FROZEN_SOURCE_SHA256 = (
    "c4d7f9022e10ef17445bab1edacf01931d526a69d5e8415a0bf770140164dfe7"
)
RESULT_PARENT = (
    ROOT / ".work/official-conformance/results/core-cdp-close-bidi-chromium-strict-v4"
)
SCOPE = "unchanged-core-cdp-custom-ws-close-three-bidi-named-projects-frozen-v3-candidate-browserless-diagnostic-only-v4"
TITLE = "should print custom ws close error"
LINE = 572
FILTER = f"{SOURCE}:{LINE}"
REPORT_FILE = "library/chromium/connect-over-cdp.spec.ts"
PROJECTS = {
    "bidi-chromium-library": {
        "suite": "bidi-chromium-cdp-close-strict-v4",
        "specId": "8d42d2a6326b0b57ff3d-da88fd98989a23696b8f",
    },
    "bidi-chrome-stable-library": {
        "suite": "bidi-chrome-stable-cdp-close-strict-v4",
        "specId": "8d42d2a6326b0b57ff3d-abec76758cb8ccb82b80",
    },
    "bidi-chrome-canary-library": {
        "suite": "bidi-chrome-canary-cdp-close-strict-v4",
        "specId": "8d42d2a6326b0b57ff3d-bd81835b26c6ab884876",
    },
}
INPUT_FILES = (
    MANIFEST,
    URL_ONLY_PRELOAD,
    SERVER_PRELOAD,
    CANDIDATE_PRELOAD,
    ROOT / "scripts/official_candidate_package_snapshot_v1.py",
    ROOT / "scripts/run_playwright_official_conformance.py",
    ROOT / "scripts/official_result_destination.py",
    ROOT / "scripts/run_playwright_core_cdp_error_slice_strict_v1.py",
    Path(__file__).resolve(),
    ROOT / "scripts/verify_playwright_core_cdp_close_bidi_chromium_strict_v4.py",
    ROOT / "scripts/test_verify_playwright_core_cdp_close_bidi_chromium_strict_v4.py",
)


def check_source_and_manifest() -> None:
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
    assert not (UPSTREAM / ".env").exists() and not (UPSTREAM / ".env").is_symlink()
    assert all(
        path.is_file() and not path.is_symlink() and sha256(path) == digest
        for path, digest in PINNED_HARNESS.items()
    )
    assert json.loads(MANIFEST.read_text()) == {
        "schema": 1,
        "repository": "https://github.com/microsoft/playwright.git",
        "tag": "v1.58.0",
        "commit": COMMIT,
        "suites": [
            {
                "id": selection["suite"],
                "role": "browser-independent-official-subset",
                "runner": "packages/playwright/cli.js",
                "config": "tests/bidi/playwright.config.ts",
                "args": [FILTER],
                "candidateAdapter": "test-mode-playwright-core",
                "project": project,
                "requiresBrowser": False,
                "expectedTestsPerProject": 1,
                "expectedFiles": 1,
            }
            for project, selection in PROJECTS.items()
        ],
    }
    source = (UPSTREAM / SOURCE).read_text()
    marker = f"test('{TITLE}'"
    assert source.count(marker) == 1
    start = source.index(marker)
    next_test = source.find("\ntest(", start + len(marker))
    body = source[start : next_test if next_test >= 0 else len(source)]
    for exact in (
        "server.onceWebSocketConnection((ws, request) => {",
        "ws.close(4123, 'Oh my!');",
        "browserType.connectOverCDP(`ws://localhost:${server.PORT}/ws`).catch(e => e)",
        "expect(error.message).toContain(`Browser logs:\\n\\nOh my!\\n`);",
    ):
        assert body.count(exact) == 1
    assert "browserType.launch(" not in body and "startRemoteServer(" not in body
    assert (
        "await run(playwright[browserName]);"
        in (UPSTREAM / "tests/config/browserTest.ts").read_text()
    )
    bidi_config = (UPSTREAM / "tests/bidi/playwright.config.ts").read_text()
    assert "retries: 0" in bidi_config
    for project in PROJECTS:
        assert project.removesuffix("-library") in bidi_config


def check_frozen_snapshot() -> dict[str, Any]:
    if not re.fullmatch(r"[0-9a-f]{64}", ARTIFACT_SHA256):
        raise ValueError("candidate artifact SHA256 is not pinned")
    if not FROZEN_MANIFEST.is_file() or FROZEN_MANIFEST.is_symlink():
        raise ValueError("frozen candidate manifest is missing or a symlink")
    if sha256(FROZEN_MANIFEST) != FROZEN_MANIFEST_SHA256:
        raise ValueError("frozen candidate manifest SHA256 changed")
    snapshot = verify_snapshot(
        FROZEN_SNAPSHOT,
        FROZEN_MANIFEST,
        source_package=SOURCE_PACKAGE,
        profile=PROFILE,
        artifact_sha256=ARTIFACT_SHA256,
    )
    if snapshot["candidateSourceSha256"] != FROZEN_SOURCE_SHA256:
        raise ValueError("frozen candidate source SHA256 changed")
    return snapshot


def _check_environment() -> None:
    forbidden = (
        "NODE_OPTIONS",
        "CI",
        "DEBUG",
        "PWDEBUG",
        "PW_TAG",
        "PWTEST_TRACE",
        "PWTEST_MODE",
        "PWPAGE_IMPL",
        "PWTEST_USE_BIDI_EXPECTATIONS",
        "BIDI_CRPATH",
        "BIDI_FFPATH",
        "PWTEST_FIREFOX_USER_PREFS",
        "PWRIGHT_CANDIDATE_CDP_ENDPOINT",
        "PWRIGHT_ORACLE_CDP_ENDPOINT",
        "PWRIGHT_CDP_ENDPOINT",
        "PWRIGHT_CDP_WS_URL",
        "PWRIGHT_NATIVE_PROFILE",
        "PWRIGHT_NODE_CANDIDATE",
        "PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM",
        "PWRIGHT_OFFICIAL_SERVER_IP",
        "PWRIGHT_OFFICIAL_SERVER_HOST",
        "PWRIGHT_OFFICIAL_CROSS_ORIGIN_FORWARD",
        "TAILSCALE_IP",
        "TAILSCALE_HOST",
        "PWRIGHT_CORE_CDP_ERROR_SLICE_GUARD_RECEIPT",
    )
    if any(key in os.environ for key in forbidden):
        raise ValueError(
            "strict Bidi-config CDP Close capture rejects inherited runtime overrides"
        )


def selected_tests(report: dict[str, Any], project: str) -> list[dict[str, Any]]:
    expected_id = PROJECTS[project]["specId"]
    selected: list[dict[str, Any]] = []

    def visit(suite: dict[str, Any]) -> None:
        for spec in suite.get("specs", []):
            assert spec["id"] == expected_id
            assert (spec["file"], spec["line"], spec["column"], spec["title"]) == (
                REPORT_FILE,
                LINE,
                5,
                TITLE,
            )
            assert len(spec["tests"]) == 1
            test = spec["tests"][0]
            assert test["projectId"] == project and test["expectedStatus"] == "passed"
            assert test["status"] in {"expected", "unexpected"}
            assert len(test["results"]) == 1
            attempt = test["results"][0]
            assert attempt["retry"] == 0
            assert (
                isinstance(attempt["workerIndex"], int) and attempt["workerIndex"] >= 0
            )
            assert attempt["status"] in {"passed", "failed"}
            passed = attempt["status"] == "passed"
            assert test["status"] == ("expected" if passed else "unexpected")
            assert spec["ok"] == passed
            errors = attempt["errors"]
            assert isinstance(errors, list) and (not errors) == passed
            assert all(
                isinstance(error, dict) and isinstance(error.get("message"), str)
                for error in errors
            )
            selected.append(
                {
                    "project": project,
                    "specId": expected_id,
                    "line": LINE,
                    "title": TITLE,
                    "status": test["status"],
                    "attempts": [
                        {
                            "retry": attempt["retry"],
                            "status": attempt["status"],
                            "workerIndex": attempt["workerIndex"],
                        }
                    ],
                    "errors": [error["message"] for error in errors],
                }
            )
        for child in suite.get("suites", []):
            visit(child)

    assert report["errors"] == []
    for suite in report["suites"]:
        visit(suite)
    assert len(selected) == 1
    stats = report["stats"]
    assert stats["expected"] == int(selected[0]["status"] == "expected")
    assert stats["unexpected"] == int(selected[0]["status"] == "unexpected")
    assert stats["skipped"] == stats["flaky"] == 0
    return selected


def paths(mode: str, project: str) -> dict[str, Path]:
    result = RESULT_PARENT / f"{mode}-{project}-v4.json"
    return {
        "result": result,
        "provenance": result.with_suffix(".provenance.json"),
        "receipt": result.with_suffix(".guard-receipt.jsonl"),
        "log": result.with_suffix(".log"),
    }


def run(mode: str) -> dict[str, Any]:
    if sys.flags.optimize:
        raise RuntimeError("strict Core capture requires Python assertions enabled")
    _check_environment()
    check_source_and_manifest()
    oracle_sha256 = None
    if mode == "candidate":
        from verify_playwright_core_cdp_close_bidi_chromium_strict_v4 import (
            verify_strict,
        )

        oracle_path = RESULT_PARENT / "oracle-v4.strict.json"
        oracle = verify_strict(oracle_path)
        assert oracle["mode"] == "oracle"
        assert all(
            one["status"] == "expected"
            for run_record in oracle["runs"]
            for one in run_record["selected"]
        )
        oracle_sha256 = sha256(oracle_path)
        check_frozen_snapshot()
    elif mode != "oracle":
        raise ValueError("mode must be oracle or candidate")
    strict_path = RESULT_PARENT / f"{mode}-v4.strict.json"
    destinations = [strict_path]
    for project in PROJECTS:
        destinations.extend(paths(mode, project).values())
    if any(path.exists() or path.is_symlink() for path in destinations):
        raise ValueError(
            "strict Bidi-config CDP Close result destination already exists"
        )
    RESULT_PARENT.mkdir(parents=True, exist_ok=True)
    inputs = {str(path.relative_to(ROOT)): sha256(path) for path in INPUT_FILES}
    runs = []
    for project, selection in PROJECTS.items():
        output = paths(mode, project)
        output["receipt"].open("x").close()
        env = os.environ.copy()
        env["NODE_OPTIONS"] = f"--require={URL_ONLY_PRELOAD}"
        env["PWRIGHT_CORE_CDP_ERROR_SLICE_GUARD_RECEIPT"] = str(output["receipt"])
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
            str(output["result"]),
        ]
        if mode == "candidate":
            command.extend(
                ["--candidate", str(FROZEN_SNAPSHOT), "--candidate-profile", PROFILE]
            )
        completed = subprocess.run(
            command, cwd=ROOT, env=env, capture_output=True, text=True, check=False
        )
        output["log"].write_text(completed.stdout + completed.stderr)
        assert all(sha256(ROOT / name) == digest for name, digest in inputs.items())
        check_source_and_manifest()
        if mode == "candidate":
            check_frozen_snapshot()
        if not output["result"].is_file() or not output["provenance"].is_file():
            raise RuntimeError(
                f"official {project} runner produced no result/provenance "
                f"(exit {completed.returncode}): {output['log'].read_text()[-3000:]}"
            )
        selected = selected_tests(json.loads(output["result"].read_text()), project)
        assert completed.returncode == (0 if selected[0]["status"] == "expected" else 1)
        if mode == "oracle":
            assert selected[0]["status"] == "expected"
        assert ("[pwright-candidate]" in output["log"].read_text()) == (
            mode == "candidate"
        )
        runs.append(
            {
                "project": project,
                "suite": selection["suite"],
                "result": str(output["result"]),
                "resultSha256": sha256(output["result"]),
                "provenanceSha256": sha256(output["provenance"]),
                "guardReceiptSha256": sha256(output["receipt"]),
                "logSha256": sha256(output["log"]),
                "exitCode": completed.returncode,
                "selected": selected,
            }
        )
    strict = {
        "schema": 1,
        "scope": SCOPE,
        "protocolUnderTest": "CDP-over-WebSocket",
        "bidiProjectNamesOnly": True,
        "fixtureAdapted": True,
        "currentLivePackageParityAttested": False,
        "publicApiParityEvidence": False,
        "promotedToCoreLedger": False,
        "mode": mode,
        "upstreamCommit": COMMIT,
        "upstreamInputs": UPSTREAM_INPUTS,
        "pinnedHarness": {
            str(path.relative_to(ROOT)): digest
            for path, digest in PINNED_HARNESS.items()
        },
        "inputs": inputs,
        "candidateProfile": PROFILE if mode == "candidate" else None,
        "candidateArtifactSha256": ARTIFACT_SHA256 if mode == "candidate" else None,
        "candidateSnapshot": str(FROZEN_SNAPSHOT) if mode == "candidate" else None,
        "candidateSnapshotManifestSha256": FROZEN_MANIFEST_SHA256
        if mode == "candidate"
        else None,
        "candidateSourceSha256": FROZEN_SOURCE_SHA256 if mode == "candidate" else None,
        "pairedOracleStrictSha256": oracle_sha256,
        "runs": runs,
    }
    cdp_base.write_json_atomic(strict_path, strict)
    return strict


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("oracle", "candidate"))
    args = parser.parse_args()
    try:
        strict = run(args.mode)
    except (
        AssertionError,
        OSError,
        ValueError,
        RuntimeError,
        json.JSONDecodeError,
    ) as error:
        parser.error(f"strict Bidi-config CDP Close capture failed: {error}")
    print(
        f"Core Bidi-config CDP Close {strict['mode']}: "
        f"{[one['status'] for run_record in strict['runs'] for one in run_record['selected']]}"
    )


if __name__ == "__main__":
    main()
