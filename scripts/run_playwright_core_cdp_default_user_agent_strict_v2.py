"""Capture the unchanged official Core CDP default-User-Agent assertion."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).parents[1].resolve()
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-v1.58.0")
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
SOURCE = "tests/library/chromium/connect-over-cdp.spec.ts"
SOURCE_SHA256 = "be1e552ad56652303ce851bf026aef01cbba7ffd634e8bfc36c3313e2586226c"
SERVER_FIXTURE = "tests/config/serverFixtures.ts"
SERVER_FIXTURE_SHA256 = (
    "4c0039a149bd61e8707adddc626b345632561bd343749293ddbe9458f6ac617b"
)
MANIFEST = (
    ROOT
    / "tests/official/playwright-core-cdp-default-user-agent-strict-v2-manifest.json"
)
PRELOAD = (
    ROOT / "tests/official/playwright-core-cdp-default-user-agent-strict-v2-preload.cjs"
)
PROFILE = "official-conformance-page-cdp-default-user-agent"
SUITE = "library-chromium-cdp-default-user-agent-strict-v2"
TITLE = "should send default User-Agent header with connect request"
SPEC_ID = "8d42d2a6326b0b57ff3d-b543150fbf0c73e3e4ee"
REPORT_FILE = "library/chromium/connect-over-cdp.spec.ts"
SCOPE = "unchanged-upstream-assertions-single-test-browser-free-fixture-host-adaptation"


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
                "args": [f"{SOURCE}:299"],
                "candidateAdapter": "test-mode-playwright-core",
                "project": "chromium-library",
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
            assert spec["id"] == SPEC_ID
            assert spec["file"] == REPORT_FILE
            assert spec["line"] == 299 and spec["column"] == 5
            assert spec["title"] == TITLE and len(spec["tests"]) == 1
            test = spec["tests"][0]
            assert test["projectId"] == "chromium-library"
            assert test["expectedStatus"] == "passed"
            assert test["status"] in {"expected", "unexpected"}
            assert len(test["results"]) == 1
            attempt = test["results"][0]
            assert attempt["retry"] == 0 and attempt["workerIndex"] == 0
            assert attempt["status"] in {"passed", "failed", "timedOut"}
            passed = attempt["status"] == "passed"
            assert test["status"] == ("expected" if passed else "unexpected")
            assert spec["ok"] == passed
            assert (attempt["errors"] == []) == passed
            found.append(
                {
                    "title": spec["title"],
                    "specId": spec["id"],
                    "file": spec["file"],
                    "line": spec["line"],
                    "column": spec["column"],
                    "expectedStatus": test["expectedStatus"],
                    "status": test["status"],
                    "attempts": [
                        {"retry": attempt["retry"], "status": attempt["status"]}
                    ],
                }
            )
        for child in suite.get("suites", []):
            visit(child)

    assert report["errors"] == []
    for suite in report["suites"]:
        visit(suite)
    assert len(found) == 1
    passed = found[0]["status"] == "expected"
    stats = report["stats"]
    assert stats["expected"] == int(passed)
    assert stats["unexpected"] == int(not passed)
    assert stats["skipped"] == 0 and stats["flaky"] == 0
    return found


def write_json_atomic(destination: Path, value: dict) -> None:
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


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=("candidate", "oracle"))
    parser.add_argument("--result", type=Path, required=True)
    parser.add_argument("--candidate-artifact-sha256")
    args = parser.parse_args()
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
        "PWRIGHT_CORE_CDP_DEFAULT_UA_V2_GUARD_RECEIPT",
    )
    if any(os.environ.get(key) for key in forbidden):
        parser.error(
            "strict CDP default User-Agent run rejects inherited runtime overrides"
        )
    if args.mode == "candidate":
        if not args.candidate_artifact_sha256 or not re.fullmatch(
            r"[0-9a-f]{64}", args.candidate_artifact_sha256
        ):
            parser.error("candidate requires a pinned artifact SHA-256")
        artifact = ROOT / "packages/pwright/native" / PROFILE / "pwright.node"
        if not artifact.is_file() or sha256(artifact) != args.candidate_artifact_sha256:
            parser.error(f"opt-in {PROFILE} artifact is missing or SHA-256 differs")
    elif args.candidate_artifact_sha256 is not None:
        parser.error("oracle must not receive a candidate artifact SHA-256")
    check_source()
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
        parser.error("strict CDP default User-Agent result destination is not empty")
    receipt = result.with_suffix(".guard-receipt.jsonl")
    receipt.open("x").close()
    inputs = {
        str(path.relative_to(ROOT)): sha256(path)
        for path in (MANIFEST, PRELOAD, Path(__file__).resolve())
    }
    env = os.environ.copy()
    env["NODE_OPTIONS"] = f"--require={PRELOAD}"
    env["PWRIGHT_CORE_CDP_DEFAULT_UA_V2_GUARD_RECEIPT"] = str(receipt)
    cmd = [
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
        cmd += ["--candidate-profile", PROFILE]
    completed = subprocess.run(
        cmd, check=False, cwd=ROOT, env=env, capture_output=True, text=True
    )
    log = completed.stdout + completed.stderr
    result.with_suffix(".log").write_text(log)
    assert all(sha256(ROOT / name) == digest for name, digest in inputs.items())
    check_source()
    provenance = result.with_suffix(".provenance.json")
    if not result.is_file() or not provenance.is_file():
        raise RuntimeError(
            "official runner produced no result/provenance "
            f"(exit {completed.returncode}): {log[-3000:]}"
        )
    selected = selected_tests(json.loads(result.read_text()))
    assert completed.returncode == (0 if selected[0]["status"] == "expected" else 1)
    records = [json.loads(line) for line in receipt.read_text().splitlines() if line]
    assert records and any(record.get("workerIndex") == "0" for record in records)
    if args.mode == "candidate":
        assert "[pwright-candidate]" in log
    else:
        assert "[pwright-candidate]" not in log
    strict = {
        "schema": 2,
        "scope": SCOPE,
        "mode": args.mode,
        "upstreamCommit": COMMIT,
        "upstreamSourceSha256": SOURCE_SHA256,
        "serverFixtureSha256": SERVER_FIXTURE_SHA256,
        "inputs": inputs,
        "candidateProfile": PROFILE if args.mode == "candidate" else None,
        "candidateArtifactSha256": args.candidate_artifact_sha256
        if args.mode == "candidate"
        else None,
        "result": str(result),
        "resultSha256": sha256(result),
        "provenanceSha256": sha256(provenance),
        "guardReceiptSha256": sha256(receipt),
        "logSha256": sha256(result.with_suffix(".log")),
        "exitCode": completed.returncode,
        "selected": selected,
    }
    write_json_atomic(result.with_suffix(".strict.json"), strict)
    print(
        json.dumps(
            {"mode": args.mode, "exitCode": completed.returncode, "selected": selected},
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
