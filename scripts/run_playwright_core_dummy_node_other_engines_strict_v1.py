#!/usr/bin/env python3
"""Run pinned Firefox/WebKit :64 through isolated official and Pwright lanes.

The only executable substituted by the harness is the exact upstream exit-1
dummy Node asset. The upstream test body and assertions are never changed.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import uuid
from pathlib import Path
from typing import Any, cast

import run_playwright_core_dummy_node_launch as base
from official_result_destination import prepare_result_destination
from run_playwright_official_conformance import _tailscale_runtime

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = (
    ROOT
    / "tests/official/playwright-core-dummy-node-other-engines-strict-v1-manifest.json"
)
MANIFEST_SHA256 = "6ac83fa5a81dd7806c2f4737d61684b8aad414b2ea006f72e83bcdbc937fa6b0"
PRELOAD = ROOT / "tests/official/playwright-core-dummy-node-other-engines-preload.cjs"
GUARD = ROOT / "tests/official/dummy-core-other-engines-guard/node"
SERVER_PRELOAD = ROOT / "tests/official/playwright-core-server-runtime.cjs"
CANDIDATE_PRELOAD = ROOT / "tests/official/playwright-core-candidate-preload.cjs"
ASSET = "tests/assets/dummy_bad_browser_executable.js"
SELECTOR = "tests/library/browsertype-launch.spec.ts:64"
TITLE = "should reject if launched browser fails immediately"
SOURCE_FILES = (
    "scripts/run_playwright_core_dummy_node_other_engines_strict_v1.py",
    "scripts/run_playwright_core_dummy_node_launch.py",
    "scripts/official_result_destination.py",
    "scripts/run_playwright_official_conformance.py",
    "tests/official/playwright-core-dummy-node-other-engines-preload.cjs",
    "tests/official/dummy-core-other-engines-guard/node",
    "tests/official/playwright-core-server-runtime.cjs",
    "tests/official/playwright-core-candidate-preload.cjs",
)
SPEC_IDS = {
    "firefox": "d1a604bdf6287c6776f3-d8df50eebf56e23fa8d3",
    "webkit": "d1a604bdf6287c6776f3-24ac4e8612e7cc735325",
}


def checked_manifest(upstream: Path) -> dict[str, Any]:
    if (
        MANIFEST.is_symlink()
        or not MANIFEST.is_file()
        or base.digest(MANIFEST) != MANIFEST_SHA256
    ):
        raise ValueError("other-engine manifest source or SHA-256 differs")
    raw = json.loads(MANIFEST.read_text())
    if not isinstance(raw, dict) or set(raw) != {
        "schema",
        "repository",
        "tag",
        "commit",
        "executionPolicy",
        "selector",
        "title",
        "asset",
        "candidateProfile",
        "candidateSourceSha256",
        "candidateArtifactSha256",
        "upstreamSources",
        "cases",
    }:
        raise ValueError("other-engine manifest shape differs")
    if (
        raw["schema"] != 2
        or raw["repository"] != "https://github.com/microsoft/playwright.git"
        or raw["tag"] != "v1.58.0"
        or raw["commit"] != base.COMMIT
        or raw["executionPolicy"] != "pinned-dummy-node-other-engines-strict-v1"
        or raw["selector"] != SELECTOR
        or raw["title"] != TITLE
        or raw["asset"] != ASSET
        or raw["candidateProfile"] != "playwright-trace"
        or raw["candidateSourceSha256"] != base.STRICT_CANDIDATE_SOURCE_SHA256
        or raw["candidateArtifactSha256"] != base.STRICT_CANDIDATE_ARTIFACT_SHA256
        or not isinstance(raw["upstreamSources"], dict)
        or raw["upstreamSources"].get(ASSET) != base.UPSTREAM_SOURCES[ASSET]
        or raw["cases"]
        != [
            {
                "engine": engine,
                "project": f"{engine}-library",
                "suite": f"library-browsertype-launch-dummy-node-failure-{engine}",
                "specId": SPEC_IDS[engine],
            }
            for engine in ("firefox", "webkit")
        ]
    ):
        raise ValueError("other-engine manifest policy/cases differ")
    if (upstream / ".env").exists() or (upstream / ".env").is_symlink():
        raise ValueError("other-engine lane forbids upstream .env")
    base.pinned_checkout(upstream)
    for relative, digest in raw["upstreamSources"].items():
        path = upstream / relative
        if path.is_symlink() or not path.is_file() or base.digest(path) != digest:
            raise ValueError(f"pinned upstream source differs: {relative}")
    asset = upstream / ASSET
    if not os.access(asset, os.X_OK):
        raise ValueError("pinned dummy executable is not executable")
    return cast(dict[str, Any], raw)


def report_case(result: Path, engine: str, mode: str) -> tuple[str, dict[str, Any]]:
    raw = json.loads(result.read_text())
    if not isinstance(raw, dict) or not isinstance(raw.get("suites"), list):
        raise TypeError("official result lacks suites")
    specs: list[dict[str, Any]] = []

    def visit(node: object) -> None:
        if (
            not isinstance(node, dict)
            or not isinstance(node.get("specs", []), list)
            or not isinstance(node.get("suites", []), list)
        ):
            raise TypeError("official result suite is malformed")
        specs.extend(cast(list[dict[str, Any]], node.get("specs", [])))
        for child in cast(list[object], node.get("suites", [])):
            visit(child)

    for suite in raw["suites"]:
        visit(suite)
    if len(specs) != 1:
        raise ValueError("other-engine result must have exactly one spec")
    spec = specs[0]
    tests = spec.get("tests")
    if (
        spec.get("id") != SPEC_IDS[engine]
        or spec.get("file") != "library/browsertype-launch.spec.ts"
        or spec.get("line") != 64
        or spec.get("column") != 3
        or spec.get("title") != TITLE
        or not isinstance(tests, list)
        or len(tests) != 1
        or not isinstance(tests[0], dict)
    ):
        raise ValueError("other-engine result differs from pinned spec")
    test = cast(dict[str, Any], tests[0])
    results = test.get("results")
    status = test.get("status")
    expected = mode == "oracle"
    expected_status = "expected" if expected else "unexpected"
    attempt_status = "passed" if expected else "failed"
    stats = raw.get("stats")
    config = raw.get("config")
    if (
        test.get("projectId") != f"{engine}-library"
        or test.get("expectedStatus") != "passed"
        or test.get("annotations") != []
        or status != expected_status
        or spec.get("ok") is not expected
        or not isinstance(results, list)
        or len(results) != 1
        or not isinstance(results[0], dict)
        or results[0].get("status") != attempt_status
        or results[0].get("retry") != 0
        or results[0].get("workerIndex") != 0
        or not isinstance(config, dict)
        or config.get("workers") != 1
        or len(
            [
                p
                for p in config.get("projects", [])
                if isinstance(p, dict)
                and p.get("id") == f"{engine}-library"
                and p.get("retries") == 0
            ]
        )
        != 1
        or not isinstance(stats, dict)
        or any(
            stats.get(key) != value
            for key, value in {
                "expected": int(expected),
                "unexpected": int(not expected),
                "skipped": 0,
                "flaky": 0,
            }.items()
        )
    ):
        raise ValueError("other-engine strict result status/attempt/count differs")
    if expected and (results[0].get("errors") != [] or raw.get("errors") != []):
        raise ValueError("oracle has an assertion or runner error")
    if not expected and not results[0].get("errors"):
        raise ValueError("candidate RED has no recorded test error")
    return cast(str, status), test


def run(mode: str, engine: str, upstream: Path, result: Path) -> None:
    upstream = upstream.resolve()
    manifest = checked_manifest(upstream)
    if engine not in SPEC_IDS or mode not in ("candidate", "oracle"):
        raise ValueError("other-engine lane must select one pinned engine/mode")
    base.preflight_environment(os.environ.copy())
    node_command = shutil.which("node", path="/opt/homebrew/bin:/usr/bin:/bin")
    if node_command is None:
        raise ValueError("system Node is missing")
    node = Path(node_command).resolve()
    if (
        not node.is_file()
        or node.is_symlink()
        or GUARD.is_symlink()
        or not GUARD.is_file()
        or not os.access(GUARD, os.X_OK)
    ):
        raise ValueError("strict executable or guard is missing/linked")
    allowed = [
        node,
        upstream / ASSET,
        GUARD,
        Path("/usr/bin/env"),
        Path("/usr/bin/python3"),
        Path("/Library/Developer/CommandLineTools/usr/bin/python3"),
        Path(
            "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/bin/python3.9"
        ),
        Path(
            "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/Resources/Python.app/Contents/MacOS/Python"
        ),
    ]
    sandbox = base.sandbox_profile(allowed)
    base.probe_sandbox(sandbox)
    candidate = ROOT / "packages/pwright" if mode == "candidate" else None
    artifact = candidate / "native/playwright-trace/pwright.node" if candidate else None
    if candidate and (
        base.source_digest(candidate) != manifest["candidateSourceSha256"]
        or artifact is None
        or artifact.is_symlink()
        or not artifact.is_file()
        or base.digest(artifact) != manifest["candidateArtifactSha256"]
    ):
        raise ValueError(
            "candidate source/native artifact differs from pinned manifest"
        )
    artifacts = {
        relative: base.directory_digest(upstream / relative)
        for relative in base.UPSTREAM_ARTIFACT_DIRS
    }
    node_sha256 = base.digest(node)
    local_sources = {
        relative: base.digest(ROOT / relative) for relative in SOURCE_FILES
    }
    bind, host = _tailscale_runtime(os.environ.copy())
    result = prepare_result_destination(result)
    paths = {
        "launchReceipt": result.with_suffix(".launch-receipt.jsonl"),
        "profileReceipt": result.with_suffix(".profile-receipt.jsonl"),
        "processReceipt": result.with_suffix(".process-receipt.jsonl"),
        "log": result.with_suffix(".log"),
        "strict": result.with_suffix(".strict.json"),
    }
    for target in (result, result.with_suffix(".provenance.json"), *paths.values()):
        if target.is_symlink() or target.exists():
            raise ValueError(
                f"other-engine result destination already exists: {target}"
            )
    result.parent.mkdir(parents=True, exist_ok=True)
    for name in ("launchReceipt", "profileReceipt", "processReceipt"):
        paths[name].open("x").close()
    run_id = uuid.uuid4().hex
    env = os.environ.copy()
    env.pop("CI", None)
    env["PWTEST_MODE"] = "default"
    env["PWRIGHT_OFFICIAL_SERVER_IP"] = bind
    env["PWRIGHT_OFFICIAL_SERVER_HOST"] = host
    env["PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM"] = str(upstream)
    env["PWRIGHT_OTHER_DUMMY_MODE"] = mode
    env["PWRIGHT_OTHER_DUMMY_ENGINE"] = engine
    env["PWRIGHT_OTHER_DUMMY_RUN_ID"] = run_id
    env["PWRIGHT_OTHER_DUMMY_HARNESS_PID"] = str(os.getpid())
    env["PWRIGHT_OTHER_DUMMY_MANIFEST"] = str(MANIFEST)
    env["PWRIGHT_OTHER_DUMMY_MANIFEST_SHA256"] = MANIFEST_SHA256
    env["PWRIGHT_OTHER_DUMMY_ASSET"] = str(upstream / ASSET)
    env["PWRIGHT_OTHER_DUMMY_REAL_NODE"] = str(node)
    env["PWRIGHT_OTHER_DUMMY_LAUNCH_RECEIPT"] = str(paths["launchReceipt"])
    env["PWRIGHT_OTHER_DUMMY_PROFILE_RECEIPT"] = str(paths["profileReceipt"])
    env["PWRIGHT_OTHER_DUMMY_PROCESS_RECEIPT"] = str(paths["processReceipt"])
    env["PLAYWRIGHT_JSON_OUTPUT_FILE"] = str(result)
    env["PATH"] = f"{GUARD.parent}:/opt/homebrew/bin:/usr/bin:/bin"
    preloads = [SERVER_PRELOAD]
    if candidate:
        env["PWRIGHT_NODE_CANDIDATE"] = str(candidate)
        env["PWRIGHT_NATIVE_PROFILE"] = "playwright-trace"
        preloads.append(CANDIDATE_PRELOAD)
    else:
        env.pop("PWRIGHT_NODE_CANDIDATE", None)
        env.pop("PWRIGHT_NATIVE_PROFILE", None)
    preloads.append(PRELOAD)
    env["NODE_OPTIONS"] = " ".join(f"--require={source}" for source in preloads)
    command = [
        "/usr/bin/sandbox-exec",
        "-p",
        sandbox,
        str(node),
        str(upstream / "packages/playwright/cli.js"),
        "test",
        "--config=tests/library/playwright.config.ts",
        "--reporter=line,json",
        "--workers=1",
        "--retries=0",
        f"--project={engine}-library",
        SELECTOR,
    ]
    with paths["log"].open("x") as log:
        launched = subprocess.Popen(
            command, cwd=upstream, env=env, stdout=log, stderr=subprocess.STDOUT
        )
        try:
            exit_code = launched.wait(timeout=120)
        except subprocess.TimeoutExpired:
            launched.terminate()
            launched.wait(timeout=10)
            raise
        cli_pid = launched.pid
    if result.is_symlink() or not result.is_file():
        raise ValueError("official runner did not write a regular result")
    outcome, test = report_case(result, engine, mode)
    if exit_code != (0 if outcome == "expected" else 1):
        raise ValueError("official CLI exit does not match test outcome")
    if checked_manifest(upstream) != manifest:
        raise ValueError("upstream/manifest drifted during strict execution")
    if {
        relative: base.digest(ROOT / relative) for relative in SOURCE_FILES
    } != local_sources:
        raise ValueError("strict local tooling drifted during execution")
    if {
        relative: base.directory_digest(upstream / relative)
        for relative in base.UPSTREAM_ARTIFACT_DIRS
    } != artifacts or base.digest(node) != node_sha256:
        raise ValueError("official runtime artifact or Node drifted during execution")
    if candidate and (
        base.source_digest(candidate) != manifest["candidateSourceSha256"]
        or artifact is None
        or base.digest(artifact) != manifest["candidateArtifactSha256"]
    ):
        raise ValueError("candidate source/artifact drifted during execution")
    hashes = {key: base.digest(path) for key, path in paths.items() if key != "strict"}
    payload = {
        "schema": 2,
        "scope": "core-dummy-node-other-engines-strict-v1",
        "mode": mode,
        "engine": engine,
        "project": f"{engine}-library",
        "suite": f"library-browsertype-launch-dummy-node-failure-{engine}",
        "specId": SPEC_IDS[engine],
        "selector": SELECTOR,
        "upstreamCommit": base.COMMIT,
        "upstream": str(upstream),
        "upstreamSources": manifest["upstreamSources"],
        "upstreamArtifactsSha256": artifacts,
        "manifest": str(MANIFEST),
        "manifestSha256": MANIFEST_SHA256,
        "candidate": str(candidate) if candidate else None,
        "candidateProfile": "playwright-trace" if candidate else None,
        "candidateSourceSha256": manifest["candidateSourceSha256"]
        if candidate
        else None,
        "candidateArtifactSha256": manifest["candidateArtifactSha256"]
        if candidate
        else None,
        "node": str(node),
        "nodeSha256": node_sha256,
        "localSourcesSha256": local_sources,
        "sandboxProfileSha256": hashlib.sha256(sandbox.encode()).hexdigest(),
        "sandboxAllowedExecutables": [str(path) for path in allowed],
        "sandboxNetworkOutbound": "denied",
        "tailscaleBind": bind,
        "tailscaleHost": host,
        "command": command,
        "runId": run_id,
        "harnessPid": os.getpid(),
        "officialCliPid": cli_pid,
        "result": str(result),
        "resultSha256": base.digest(result),
        "launchReceipt": str(paths["launchReceipt"]),
        "launchReceiptSha256": hashes["launchReceipt"],
        "profileReceipt": str(paths["profileReceipt"]),
        "profileReceiptSha256": hashes["profileReceipt"],
        "processReceipt": str(paths["processReceipt"]),
        "processReceiptSha256": hashes["processReceipt"],
        "log": str(paths["log"]),
        "logSha256": hashes["log"],
        "testStatus": outcome,
        "attemptStatus": test["results"][0]["status"],
        "exitCode": exit_code,
        "assertionsExecuted": True,
        "upstreamAssertionsModified": False,
        "fixtureAdapted": True,
        "candidatePublicMethodAttested": bool(candidate),
        "candidateNativeArtifactPinned": bool(candidate),
        "candidateNativeRuntimeInvoked": False,
        "publicApiParityEvidence": False,
        "diagnosticOnly": False,
    }
    with paths["strict"].open("x") as stream:
        json.dump(payload, stream, sort_keys=True, indent=2)
        stream.write("\n")
    print(
        f"strict {engine} {mode}: {outcome} result={result} sidecar={paths['strict']}",
        flush=True,
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("candidate", "oracle"))
    parser.add_argument("engine", choices=("firefox", "webkit"))
    parser.add_argument("upstream", type=Path)
    parser.add_argument("--result", required=True, type=Path)
    args = parser.parse_args()
    try:
        run(args.mode, args.engine, args.upstream, args.result)
    except (
        OSError,
        TypeError,
        ValueError,
        subprocess.CalledProcessError,
        subprocess.TimeoutExpired,
        json.JSONDecodeError,
    ) as error:
        parser.exit(2, f"strict other-engine lane rejected execution: {error}\n")


if __name__ == "__main__":
    main()
