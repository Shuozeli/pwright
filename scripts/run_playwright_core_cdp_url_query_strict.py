#!/usr/bin/env python3
"""Run two unchanged upstream Core CDP URL-query assertions without a browser."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
from pathlib import Path
from typing import Any

from run_playwright_core_cdp_headers_strict import (
    ARTIFACT_SHA256,
    COMMIT,
    PROFILE,
    ROOT,
    SERVER_FIXTURE_SHA256,
    SOURCE,
    SOURCE_SHA256,
    UPSTREAM,
    check_source,
    sha256,
)

MANIFEST = ROOT / "tests/official/playwright-core-cdp-url-query-strict-v1-manifest.json"
PRELOAD = ROOT / "tests/official/playwright-core-cdp-url-query-strict-v1-preload.cjs"
COMMON = ROOT / "scripts/run_playwright_core_cdp_headers_strict.py"
SUITE = "library-chromium-cdp-url-query-strict-v1"
CASES = {
    "8d42d2a6326b0b57ff3d-875e645845c8895a6251": (
        275,
        "should keep URL parameters when adding json/version",
    ),
    "8d42d2a6326b0b57ff3d-6eb553279089d4ca15e6": (
        287,
        "should append /json/version with a slash if there isnt one",
    ),
}


def check_manifest() -> None:
    check_source()
    manifest = json.loads(MANIFEST.read_text())
    assert manifest == {
        "schema": 1,
        "repository": "https://github.com/microsoft/playwright.git",
        "tag": "v1.58.0",
        "commit": COMMIT,
        "suites": [
            {
                "id": SUITE,
                "role": "browser-independent-official-subset",
                "runner": "packages/playwright/cli.js",
                "config": "tests/library/playwright.config.ts",
                "args": [f"{SOURCE}:275", f"{SOURCE}:287"],
                "candidateAdapter": "test-mode-playwright-core",
                "project": "chromium-library",
                "requiresBrowser": False,
                "expectedTestsPerProject": 2,
                "expectedFiles": 1,
            }
        ],
    }


def selected_tests(report: dict[str, Any]) -> list[dict[str, Any]]:
    selected: list[dict[str, Any]] = []

    def visit(suite: dict[str, Any]) -> None:
        for spec in suite.get("specs", []):
            spec_id = spec["id"]
            assert spec_id in CASES
            line, title = CASES[spec_id]
            assert spec["file"] == SOURCE.removeprefix("tests/")
            assert (spec["line"], spec["column"], spec["title"]) == (line, 5, title)
            assert len(spec["tests"]) == 1
            test = spec["tests"][0]
            assert test["projectId"] == "chromium-library"
            assert test["expectedStatus"] == "passed"
            assert test["status"] in {"expected", "unexpected"}
            assert len(test["results"]) == 1
            selected.append(
                {
                    "title": title,
                    "line": line,
                    "specId": spec_id,
                    "status": test["status"],
                    "attempts": [test["results"][0]["status"]],
                }
            )
        for child in suite.get("suites", []):
            visit(child)

    assert not report.get("errors")
    for suite in report["suites"]:
        visit(suite)
    assert len(selected) == 2
    assert {item["specId"] for item in selected} == set(CASES)
    return sorted(selected, key=lambda item: item["line"])


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=("candidate", "oracle"))
    parser.add_argument("--result", type=Path, required=True)
    args = parser.parse_args()
    forbidden = (
        "NODE_OPTIONS",
        "PWTEST_MODE",
        "PWPAGE_IMPL",
        "PWTEST_USE_BIDI_EXPECTATIONS",
        "PWRIGHT_CANDIDATE_CDP_ENDPOINT",
        "PWRIGHT_ORACLE_CDP_ENDPOINT",
    )
    if any(os.environ.get(key) for key in forbidden):
        parser.error("strict CDP URL run rejects inherited runtime overrides")
    check_manifest()
    artifact = ROOT / "packages/pwright/native" / PROFILE / "pwright.node"
    if args.mode == "candidate":
        assert sha256(artifact) == ARTIFACT_SHA256
    result = args.result.resolve()
    result.parent.mkdir(parents=True, exist_ok=True)
    evidence = [
        result,
        result.with_suffix(".provenance.json"),
        result.with_suffix(".strict.json"),
        result.with_suffix(".guard-receipt.jsonl"),
        result.with_suffix(".log"),
    ]
    if any(path.exists() or path.is_symlink() for path in evidence):
        parser.error("strict CDP URL result destination is not empty")
    receipt = result.with_suffix(".guard-receipt.jsonl")
    receipt.open("x").close()
    inputs = {
        str(path.relative_to(ROOT)): sha256(path)
        for path in (MANIFEST, PRELOAD, COMMON, Path(__file__))
    }
    env = os.environ.copy()
    env["NODE_OPTIONS"] = f"--require={PRELOAD}"
    env["PWRIGHT_CORE_CDP_URL_GUARD_RECEIPT"] = str(receipt)
    command = [
        "uv",
        "run",
        "--with",
        "pytest",
        str(ROOT / "scripts/run_playwright_official_conformance.py"),
        args.mode,
        str(UPSTREAM),
        "--suite",
        SUITE,
        "--manifest",
        str(MANIFEST),
        "--workers",
        "1",
        "--result",
        str(result),
    ]
    if args.mode == "candidate":
        command += ["--candidate-profile", PROFILE]
    completed = subprocess.run(command, cwd=ROOT, env=env, capture_output=True, text=True)
    log = completed.stdout + completed.stderr
    log_path = result.with_suffix(".log")
    log_path.write_text(log)
    assert all(sha256(ROOT / name) == digest for name, digest in inputs.items())
    check_manifest()
    provenance = result.with_suffix(".provenance.json")
    if not result.is_file() or not provenance.is_file():
        raise RuntimeError(
            f"official runner produced no result/provenance "
            f"(exit {completed.returncode}): {log[-3000:]}"
        )
    selected = selected_tests(json.loads(result.read_text()))
    records = [json.loads(line) for line in receipt.read_text().splitlines() if line]
    assert records and any(item.get("workerIndex") == "0" for item in records)
    assert all(
        item["sourceSha256"] == SOURCE_SHA256
        and item["fixtureHost"].endswith(".ts.net")
        and item["replacements"] == {"urlWithSlash": 1, "urlWithoutSlash": 1}
        for item in records
    )
    if args.mode == "candidate":
        assert "[pwright-candidate]" in log
    strict = {
        "schema": 1,
        "scope": "unchanged-upstream-assertions-browser-free-fixture-host-adaptation",
        "mode": args.mode,
        "upstreamCommit": COMMIT,
        "upstreamSourceSha256": SOURCE_SHA256,
        "serverFixtureSha256": SERVER_FIXTURE_SHA256,
        "inputs": inputs,
        "candidateProfile": PROFILE if args.mode == "candidate" else None,
        "candidateArtifactSha256": ARTIFACT_SHA256 if args.mode == "candidate" else None,
        "result": str(result),
        "resultSha256": sha256(result),
        "provenanceSha256": sha256(provenance),
        "guardReceiptSha256": sha256(receipt),
        "logSha256": sha256(log_path),
        "exitCode": completed.returncode,
        "selected": selected,
    }
    result.with_suffix(".strict.json").write_text(
        json.dumps(strict, indent=2, sort_keys=True) + "\n"
    )
    print(
        json.dumps(
            {"mode": args.mode, "exitCode": completed.returncode, "selected": selected},
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
