#!/usr/bin/env python3
"""Capture three unchanged official Core CDP error cases without Chrome."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import tempfile
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-v1.58.0")
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
SOURCE = "tests/library/chromium/connect-over-cdp.spec.ts"
SOURCE_SHA256 = "be1e552ad56652303ce851bf026aef01cbba7ffd634e8bfc36c3313e2586226c"
SERVER_FIXTURE = "tests/config/serverFixtures.ts"
SERVER_FIXTURE_SHA256 = "4c0039a149bd61e8707adddc626b345632561bd343749293ddbe9458f6ac617b"
COMMON_PRELOAD = ROOT / "tests/official/playwright-core-server-runtime.cjs"
COMMON_PRELOAD_SHA256 = "58278e6c976da47074a1eb8d682916ebc3d0a41975fe89fe9270d1a664c396fd"
MANIFEST = ROOT / "tests/official/playwright-core-cdp-error-slice-strict-v1-manifest.json"
PRELOAD = ROOT / "tests/official/playwright-core-cdp-error-slice-strict-v1-preload.cjs"
PROFILE = "official-conformance-page-cdp-default-user-agent"
ARTIFACT_SHA256 = "92f039f5530075841ca7d59ff523c1f6b7a5ec4884a29ff20042ef3c95826fb2"
SUITE = "library-chromium-cdp-error-slice-strict-v1"
REPORT_FILE = "library/chromium/connect-over-cdp.spec.ts"
SCOPE = "unchanged-upstream-assertions-three-cdp-errors-browser-free-fixture-host-adaptation"
CASES = {
    "8d42d2a6326b0b57ff3d-de38f5af50cb42c10550": (
        399,
        "should report an expected error when the endpointURL returns a non-expected status code",
    ),
    "8d42d2a6326b0b57ff3d-858b23a33867c47c6bb2": (
        411,
        "should report an expected error when the endpoint URL JSON "
        "webSocketDebuggerUrl is undefined",
    ),
    "8d42d2a6326b0b57ff3d-2a0f5cff9ee48ad78688": (
        572,
        "should print custom ws close error",
    ),
}
SELECTED_LINES = (399, 411, 572)
RESULT_PARENT = ROOT / ".work/official-conformance/results/core-cdp-error-slice-strict-v1"


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
    assert sha256(UPSTREAM / SERVER_FIXTURE) == SERVER_FIXTURE_SHA256
    assert sha256(COMMON_PRELOAD) == COMMON_PRELOAD_SHA256


def check_manifest() -> None:
    check_source()
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
                "args": [f"{SOURCE}:{line}" for line in SELECTED_LINES],
                "candidateAdapter": "test-mode-playwright-core",
                "project": "chromium-library",
                "requiresBrowser": False,
                "expectedTestsPerProject": 3,
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
            assert spec["file"] == REPORT_FILE
            assert (spec["line"], spec["column"], spec["title"]) == (line, 5, title)
            assert len(spec["tests"]) == 1
            test = spec["tests"][0]
            assert test["projectId"] == "chromium-library"
            assert test["expectedStatus"] == "passed"
            assert test["status"] in {"expected", "unexpected"}
            assert len(test["results"]) == 1
            attempt = test["results"][0]
            assert attempt["retry"] == 0 and attempt["workerIndex"] == 0
            assert attempt["status"] in {"passed", "failed"}
            passed = attempt["status"] == "passed"
            assert test["status"] == ("expected" if passed else "unexpected")
            assert spec["ok"] == passed
            errors = attempt["errors"]
            assert isinstance(errors, list) and (not errors) == passed
            selected.append(
                {
                    "title": title,
                    "line": line,
                    "specId": spec_id,
                    "status": test["status"],
                    "attempts": [{"retry": attempt["retry"], "status": attempt["status"]}],
                    "errors": [error.get("message", "") for error in errors],
                }
            )
        for child in suite.get("suites", []):
            visit(child)

    assert report["errors"] == []
    for suite in report["suites"]:
        visit(suite)
    assert len(selected) == len(CASES)
    assert {item["specId"] for item in selected} == set(CASES)
    selected.sort(key=lambda item: item["line"])
    stats = report["stats"]
    passed = sum(item["status"] == "expected" for item in selected)
    assert stats["expected"] == passed
    assert stats["unexpected"] == len(CASES) - passed
    assert stats["skipped"] == 0 and stats["flaky"] == 0
    return selected


def adapted_source_sha256(host: str) -> str:
    source = (UPSTREAM / SOURCE).read_text()
    title = "test('should print custom ws close error'"
    marker = "browserType.connectOverCDP(`ws://localhost:${server.PORT}/ws`).catch(e => e)"
    assert source.count(title) == 1
    start = source.index(title)
    next_test = source.find("\ntest(", start + len(title))
    end = len(source) if next_test < 0 else next_test
    body = source[start:end]
    assert body.count(marker) == 1
    changed = body.replace(
        marker,
        f"browserType.connectOverCDP(`ws://{host}:${{server.PORT}}/ws`).catch(e => e)",
    )
    adapted = source[:start] + changed + source[end:]
    return hashlib.sha256(adapted.encode()).hexdigest()


def write_json_atomic(destination: Path, value: dict[str, Any]) -> None:
    with tempfile.NamedTemporaryFile(
        mode="w",
        encoding="utf-8",
        dir=destination.parent,
        prefix=f".{destination.name}.",
        suffix=".tmp",
        delete=False,
    ) as stream:
        temporary = Path(stream.name)
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())
    try:
        os.link(temporary, destination)
    finally:
        temporary.unlink(missing_ok=True)


def run(mode: str, result: Path) -> dict[str, Any]:
    forbidden = (
        "NODE_OPTIONS",
        "PWTEST_MODE",
        "PWPAGE_IMPL",
        "PWTEST_USE_BIDI_EXPECTATIONS",
        "PWRIGHT_CANDIDATE_CDP_ENDPOINT",
        "PWRIGHT_ORACLE_CDP_ENDPOINT",
        "PWRIGHT_NATIVE_PROFILE",
        "PWRIGHT_NODE_CANDIDATE",
        "PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM",
        "PWRIGHT_OFFICIAL_SERVER_HOST",
        "PWRIGHT_CORE_CDP_ERROR_SLICE_GUARD_RECEIPT",
    )
    if any(os.environ.get(key) for key in forbidden):
        raise ValueError("strict CDP error slice rejects inherited runtime overrides")
    check_manifest()
    artifact = ROOT / "packages/pwright/native" / PROFILE / "pwright.node"
    if mode == "candidate" and (not artifact.is_file() or sha256(artifact) != ARTIFACT_SHA256):
        raise ValueError("pinned candidate UA-profile artifact is missing or changed")
    result = result.absolute()
    if result.parent != RESULT_PARENT or result.suffix != ".json":
        raise ValueError("strict CDP error slice result must be in its dedicated result directory")
    RESULT_PARENT.mkdir(parents=True, exist_ok=True)
    outputs = [
        result,
        result.with_suffix(".provenance.json"),
        result.with_suffix(".strict.json"),
        result.with_suffix(".guard-receipt.jsonl"),
        result.with_suffix(".log"),
    ]
    if any(path.exists() or path.is_symlink() for path in outputs):
        raise ValueError("strict CDP error slice result destination is not empty")
    receipt = result.with_suffix(".guard-receipt.jsonl")
    receipt.open("x").close()
    inputs = {
        str(path.relative_to(ROOT)): sha256(path)
        for path in (MANIFEST, PRELOAD, COMMON_PRELOAD, Path(__file__).resolve())
    }
    env = os.environ.copy()
    env["NODE_OPTIONS"] = f"--require={PRELOAD}"
    env["PWRIGHT_CORE_CDP_ERROR_SLICE_GUARD_RECEIPT"] = str(receipt)
    command = [
        "uv",
        "run",
        "--with",
        "pytest",
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
        str(result),
    ]
    if mode == "candidate":
        command += ["--candidate-profile", PROFILE]
    completed = subprocess.run(
        command, check=False, cwd=ROOT, env=env, capture_output=True, text=True
    )
    log = completed.stdout + completed.stderr
    log_path = result.with_suffix(".log")
    log_path.write_text(log)
    assert all(sha256(ROOT / name) == digest for name, digest in inputs.items())
    check_manifest()
    provenance = result.with_suffix(".provenance.json")
    if not result.is_file() or not provenance.is_file():
        raise RuntimeError(
            "official runner produced no result/provenance "
            f"(exit {completed.returncode}): {log[-3000:]}"
        )
    selected = selected_tests(json.loads(result.read_text()))
    assert completed.returncode == (
        0 if all(item["status"] == "expected" for item in selected) else 1
    )
    records = [json.loads(line) for line in receipt.read_text().splitlines() if line]
    assert records and any(record.get("workerIndex") == "0" for record in records)
    if mode == "candidate":
        assert "[pwright-candidate]" in log
    else:
        assert "[pwright-candidate]" not in log
    strict = {
        "schema": 2,
        "scope": SCOPE,
        "mode": mode,
        "upstreamCommit": COMMIT,
        "upstreamSourceSha256": SOURCE_SHA256,
        "serverFixtureSha256": SERVER_FIXTURE_SHA256,
        "commonPreloadSha256": COMMON_PRELOAD_SHA256,
        "inputs": inputs,
        "candidateProfile": PROFILE if mode == "candidate" else None,
        "candidateArtifactSha256": ARTIFACT_SHA256 if mode == "candidate" else None,
        "result": str(result),
        "resultSha256": sha256(result),
        "provenanceSha256": sha256(provenance),
        "guardReceiptSha256": sha256(receipt),
        "logSha256": sha256(log_path),
        "exitCode": completed.returncode,
        "selected": selected,
    }
    write_json_atomic(result.with_suffix(".strict.json"), strict)
    return strict


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("candidate", "oracle"))
    parser.add_argument("--result", type=Path, required=True)
    args = parser.parse_args()
    try:
        strict = run(args.mode, args.result)
    except (AssertionError, OSError, ValueError, RuntimeError, json.JSONDecodeError) as error:
        parser.error(str(error))
    print(
        json.dumps(
            {"mode": args.mode, "exitCode": strict["exitCode"], "selected": strict["selected"]},
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
