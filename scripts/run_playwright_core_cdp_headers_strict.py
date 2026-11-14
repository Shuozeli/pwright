#!/usr/bin/env python3
"""Capture two unchanged upstream Core CDP-header tests without a browser."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
from pathlib import Path

ROOT = Path(__file__).parents[1].resolve()
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-v1.58.0")
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
SOURCE = "tests/library/chromium/connect-over-cdp.spec.ts"
SOURCE_SHA256 = "be1e552ad56652303ce851bf026aef01cbba7ffd634e8bfc36c3313e2586226c"
SERVER_FIXTURE = "tests/config/serverFixtures.ts"
SERVER_FIXTURE_SHA256 = "4c0039a149bd61e8707adddc626b345632561bd343749293ddbe9458f6ac617b"
MANIFEST = ROOT / "tests/official/playwright-core-cdp-handshake-headers-strict-v1-manifest.json"
PRELOAD = ROOT / "tests/official/playwright-core-cdp-handshake-headers-strict-v1-preload.cjs"
PROFILE = "official-conformance-page-cdp-handshake-headers"
ARTIFACT_SHA256 = "dfe9607df2c7cfa16d304338e88eb2fa01302b4736e4116c0c67b655a3482769"
SUITE = "library-chromium-cdp-handshake-headers-strict-v1"
TITLES = {
    "should send extra headers with connect request",
    "should send default User-Agent header with connect request",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check_source() -> None:
    assert subprocess.check_output(
        ["git", "-C", str(UPSTREAM), "rev-parse", "HEAD"], text=True
    ).strip() == COMMIT
    assert not subprocess.check_output(
        ["git", "-C", str(UPSTREAM), "status", "--porcelain", "--untracked-files=no"],
        text=True,
    ).strip()
    assert sha256(UPSTREAM / SOURCE) == SOURCE_SHA256
    assert sha256(UPSTREAM / SERVER_FIXTURE) == SERVER_FIXTURE_SHA256
    manifest = json.loads(MANIFEST.read_text())
    assert manifest == {
        "schema": 1,
        "repository": "https://github.com/microsoft/playwright.git",
        "tag": "v1.58.0",
        "commit": COMMIT,
        "suites": [{
            "id": SUITE,
            "role": "browser-independent-official-subset",
            "runner": "packages/playwright/cli.js",
            "config": "tests/library/playwright.config.ts",
            "args": [f"{SOURCE}:242", f"{SOURCE}:299"],
            "candidateAdapter": "test-mode-playwright-core",
            "project": "chromium-library",
            "requiresBrowser": False,
            "expectedTestsPerProject": 2,
            "expectedFiles": 1,
        }],
    }


def selected_tests(report: dict) -> list[dict]:
    found: list[dict] = []

    def visit(suite: dict) -> None:
        for spec in suite.get("specs", []):
            assert spec["file"] == "library/chromium/connect-over-cdp.spec.ts"
            assert len(spec["tests"]) == 1
            test = spec["tests"][0]
            assert test["projectId"] == "chromium-library"
            assert test["results"] and test["status"] in {"expected", "unexpected"}
            found.append({
                "title": spec["title"],
                "specId": spec["id"],
                "status": test["status"],
                "attempts": [item["status"] for item in test["results"]],
            })
        for child in suite.get("suites", []):
            visit(child)

    assert not report.get("errors")
    for suite in report["suites"]:
        visit(suite)
    assert len(found) == 2 and {test["title"] for test in found} == TITLES
    assert len({test["specId"] for test in found}) == 2
    return sorted(found, key=lambda value: value["title"])


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=("candidate", "oracle"))
    parser.add_argument("--result", type=Path, required=True)
    args = parser.parse_args()
    if any(os.environ.get(key) for key in (
        "NODE_OPTIONS", "PWTEST_MODE", "PWPAGE_IMPL", "PWTEST_USE_BIDI_EXPECTATIONS",
        "PWRIGHT_CANDIDATE_CDP_ENDPOINT", "PWRIGHT_ORACLE_CDP_ENDPOINT",
    )):
        parser.error("strict CDP header run rejects inherited runtime overrides")
    check_source()
    artifact = ROOT / "packages/pwright/native" / PROFILE / "pwright.node"
    if args.mode == "candidate":
        assert sha256(artifact) == ARTIFACT_SHA256
    result = args.result.resolve()
    result.parent.mkdir(parents=True, exist_ok=True)
    paths = [
        result,
        result.with_suffix(".provenance.json"),
        result.with_suffix(".strict.json"),
        result.with_suffix(".guard-receipt.jsonl"),
        result.with_suffix(".log"),
    ]
    if any(path.exists() or path.is_symlink() for path in paths):
        parser.error("strict CDP header result destination is not empty")
    receipt = result.with_suffix(".guard-receipt.jsonl")
    receipt.open("x").close()
    inputs = {str(path.relative_to(ROOT)): sha256(path) for path in (MANIFEST, PRELOAD, Path(__file__))}
    env = os.environ.copy()
    env["NODE_OPTIONS"] = f"--require={PRELOAD}"
    env["PWRIGHT_CORE_CDP_HEADERS_GUARD_RECEIPT"] = str(receipt)
    cmd = [
        "uv", "run", "--with", "pytest", str(ROOT / "scripts/run_playwright_official_conformance.py"),
        args.mode, str(UPSTREAM), "--suite", SUITE, "--manifest", str(MANIFEST),
        "--workers", "1", "--result", str(result),
    ]
    if args.mode == "candidate":
        cmd += ["--candidate-profile", PROFILE]
    completed = subprocess.run(cmd, cwd=ROOT, env=env, capture_output=True, text=True)
    log = completed.stdout + completed.stderr
    result.with_suffix(".log").write_text(log)
    assert all(sha256(ROOT / name) == digest for name, digest in inputs.items())
    check_source()
    provenance = result.with_suffix(".provenance.json")
    if not result.is_file() or not provenance.is_file():
        raise RuntimeError(f"official runner produced no result/provenance (exit {completed.returncode}): {log[-3000:]}")
    report = json.loads(result.read_text())
    selected = selected_tests(report)
    records = [json.loads(line) for line in receipt.read_text().splitlines() if line]
    assert records and any(record.get("workerIndex") is not None for record in records)
    assert all(record["sourceSha256"] == SOURCE_SHA256 and record["fixtureHost"].endswith(".ts.net")
               and record["replacements"] == {"websocket": 2, "discovery": 1}
               for record in records)
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
        "logSha256": sha256(result.with_suffix(".log")),
        "exitCode": completed.returncode,
        "selected": selected,
    }
    result.with_suffix(".strict.json").write_text(json.dumps(strict, indent=2, sort_keys=True) + "\n")
    print(json.dumps({"mode": args.mode, "exitCode": completed.returncode, "selected": selected}, indent=2))


if __name__ == "__main__":
    main()
