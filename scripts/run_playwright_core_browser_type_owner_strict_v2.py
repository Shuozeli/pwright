#!/usr/bin/env python3
"""Capture exact official BrowserType.executablePath assertions in paired lanes."""

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
SOURCE = "tests/library/browsertype-basic.spec.ts"
SOURCE_SHA256 = "5f51d798d443c1b2d9884dd47bd5776382098ca61e76bd317a11d37bf50f81d8"
FIXTURE = "packages/playwright/src/index.ts"
FIXTURE_SHA256 = "1d7a56955dd8163b263bcce3a1155c0bed82a53ac3ef7ae7b8d69659f5227e82"
MANIFEST = ROOT / "tests/official/playwright-core-browser-type-owner-strict-v2-manifest.json"
PRELOAD = ROOT / "tests/official/playwright-core-browser-type-owner-v2-preload.cjs"
PROFILE = "browser-registry"
ARTIFACT_SHA256 = "641dd2867c3ccb5a3961020bdd2dd3e738cee2a15da141c654b1261e2cb4dedb"
SUITE = "library-browsertype-executable-path-owner-strict-v2"
IDS = {
    "chromium-library": "6b03a5a0308bccda8853-2cb507360f36bef87bf6",
    "firefox-library": "6b03a5a0308bccda8853-3692f14db41d17bf119c",
    "webkit-library": "6b03a5a0308bccda8853-c7ac17524d35be6489fc",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check_source() -> None:
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
    assert sha256(UPSTREAM / SOURCE) == SOURCE_SHA256
    assert sha256(UPSTREAM / FIXTURE) == FIXTURE_SHA256
    text = (UPSTREAM / SOURCE).read_text()
    assert text.count("test('browserType.executablePath should work'") == 1
    assert "expect(fs.existsSync(executablePath)).toBe(true);" in text
    assert "(browserType as any)._playwright._defaultLaunchOptions.executablePath" in text
    assert json.loads(MANIFEST.read_text()) == {
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
                "args": [f"{SOURCE}:21"],
                "candidateAdapter": "test-mode-playwright-core",
                "requiresBrowser": False,
                "expectedTestsPerProject": 1,
                "expectedFiles": 1,
            }
        ],
    }


def selected_tests(report: dict) -> list[dict]:
    found: list[dict] = []

    def visit(suite: dict) -> None:
        for spec in suite.get("specs", []):
            assert spec["file"] == "library/browsertype-basic.spec.ts"
            assert spec["title"] == "browserType.executablePath should work"
            assert len(spec["tests"]) == 1
            test = spec["tests"][0]
            project = test["projectId"]
            assert IDS.get(project) == spec["id"]
            assert test["status"] in {"expected", "unexpected"}
            assert len(test["results"]) == 1
            attempt = test["results"][0]
            assert attempt["status"] in {"passed", "failed", "timedOut"}
            assert isinstance(attempt["workerIndex"], int)
            found.append(
                {
                    "projectId": project,
                    "specId": spec["id"],
                    "status": test["status"],
                    "attemptStatus": attempt["status"],
                    "workerIndex": attempt["workerIndex"],
                }
            )
        for child in suite.get("suites", []):
            visit(child)

    assert not report.get("errors")
    for suite in report["suites"]:
        visit(suite)
    assert len(found) == 3 and {row["projectId"] for row in found} == set(IDS)
    assert len({row["workerIndex"] for row in found}) == 3
    return sorted(found, key=lambda row: row["projectId"])


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=("candidate", "oracle"))
    parser.add_argument("--result", type=Path, required=True)
    args = parser.parse_args()
    for key in (
        "NODE_OPTIONS",
        "PWTEST_MODE",
        "PWPAGE_IMPL",
        "PWTEST_USE_BIDI_EXPECTATIONS",
        "PWRIGHT_NODE_CANDIDATE",
        "PWRIGHT_NATIVE_PROFILE",
        "PWRIGHT_CORE_BROWSER_TYPE_OWNER_RECEIPT",
        "PWRIGHT_CANDIDATE_CDP_ENDPOINT",
        "PWRIGHT_ORACLE_CDP_ENDPOINT",
    ):
        if os.environ.get(key):
            parser.error(f"strict BrowserType run rejects inherited {key}")
    check_source()
    artifact = ROOT / "packages/pwright/native" / PROFILE / "pwright.node"
    if args.mode == "candidate":
        assert sha256(artifact) == ARTIFACT_SHA256
    result = args.result.resolve()
    result.parent.mkdir(parents=True, exist_ok=True)
    paths = [
        result,
        *(
            result.with_suffix(suffix)
            for suffix in (
                ".provenance.json",
                ".strict.json",
                ".owner-receipt.jsonl",
                ".log",
            )
        ),
    ]
    if any(path.exists() or path.is_symlink() for path in paths):
        parser.error("strict BrowserType result destination is not empty")
    inputs = {
        str(path.relative_to(ROOT)): sha256(path) for path in (MANIFEST, PRELOAD, Path(__file__))
    }
    env = os.environ.copy()
    receipt = result.with_suffix(".owner-receipt.jsonl")
    if args.mode == "candidate":
        receipt.open("x").close()
        env["NODE_OPTIONS"] = f"--require={PRELOAD}"
        env["PWRIGHT_CORE_BROWSER_TYPE_OWNER_RECEIPT"] = str(receipt)
        env["PWRIGHT_NODE_CANDIDATE"] = str(ROOT / "packages/pwright")
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
    result.with_suffix(".log").write_text(log)
    assert all(sha256(ROOT / name) == digest for name, digest in inputs.items())
    check_source()
    provenance = result.with_suffix(".provenance.json")
    if not result.is_file() or not provenance.is_file():
        raise RuntimeError(f"official runner did not produce result/provenance: {log[-2500:]}")
    selected = selected_tests(json.loads(result.read_text()))
    if args.mode == "candidate":
        records = [json.loads(line) for line in receipt.read_text().splitlines() if line]
        workers = {record["workerIndex"] for record in records if record["workerIndex"] is not None}
        assert workers == {str(row["workerIndex"]) for row in selected}
        assert all(
            record["candidate"] == str(ROOT / "packages/pwright") and record["ownerBound"] is True
            for record in records
        )
        assert "[pwright-candidate]" in log
    else:
        assert not receipt.exists()
    strict = {
        "schema": 1,
        "scope": "unchanged-official-assertions-browserless-private-owner-fixture-adaptation-v2",
        "mode": args.mode,
        "upstreamCommit": COMMIT,
        "upstreamSourceSha256": SOURCE_SHA256,
        "upstreamFixtureSha256": FIXTURE_SHA256,
        "inputs": inputs,
        "candidateProfile": PROFILE if args.mode == "candidate" else None,
        "candidateArtifactSha256": ARTIFACT_SHA256 if args.mode == "candidate" else None,
        "result": str(result),
        "resultSha256": sha256(result),
        "provenanceSha256": sha256(provenance),
        "ownerReceiptSha256": sha256(receipt) if args.mode == "candidate" else None,
        "logSha256": sha256(result.with_suffix(".log")),
        "exitCode": completed.returncode,
        "selected": selected,
    }
    result.with_suffix(".strict.json").write_text(
        json.dumps(strict, indent=2, sort_keys=True) + "\n"
    )
    print(
        json.dumps(
            {"mode": args.mode, "exitCode": completed.returncode, "selected": selected}, indent=2
        )
    )


if __name__ == "__main__":
    main()
