#!/usr/bin/env python3
"""Capture six pinned BiDi-config public launch failures without a browser.

The official test and assertion are unchanged. The only executable selected by
the test is upstream's exit-1 Node asset. A macOS sandbox permits that asset and
the exact process guard, denies outbound networking, and the upstream server
fixture is bound to the Tailscale address by the existing server preload.
"""

from __future__ import annotations

import argparse
import hashlib
import ipaddress
import json
import os
import shutil
import signal
import subprocess
import time
import uuid
from pathlib import Path
from typing import Any, cast

import run_playwright_core_dummy_node_launch as base
from official_result_destination import prepare_result_destination
from run_playwright_official_conformance import _tailscale_runtime

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = (
    ROOT / "tests/official/playwright-core-dummy-node-bidi-strict-v3-manifest.json"
)
MANIFEST_SHA256 = "045271d513fbe7844e4389beabc4e753a5b34cbf1ad1d79a9f1641e425e9d767"
PRELOAD = ROOT / "tests/official/playwright-core-dummy-node-bidi-strict-v3-preload.cjs"
SERVER_PRELOAD = ROOT / "tests/official/playwright-core-server-runtime.cjs"
CANDIDATE_PRELOAD = ROOT / "tests/official/playwright-core-candidate-preload.cjs"
CHROMIUM_GUARD = ROOT / "tests/official/dummy-core-guard/node"
FIREFOX_GUARD = ROOT / "tests/official/dummy-core-bidi-firefox-guard/node"
SELECTOR = "tests/library/browsertype-launch.spec.ts:64"
ASSET = "tests/assets/dummy_bad_browser_executable.js"
TITLE = "should reject if launched browser fails immediately"
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
PROFILE = "bidi-startup-failure"
V9_EVIDENCE_SCOPE = (
    "official-browser-free-browsertype-launch-dummy-node-bidi-config-outcomes-v9"
)
SOURCE_SHA256 = "fefa89f15b0dd2c75b8485d20dcbb181866330a5099595c3405537f16cc3674e"
ARTIFACT_SHA256 = "709c9b9e263d84d3fcdaf9240b2ba27033c0c5b873533a45788b989710455adb"
TASK_RESULTS = (
    ROOT / ".work/official-conformance/results/core-dummy-node-bidi-strict-v3"
)
FIXTURE_PORTS = (8907, 8908, 8909, 8910)
V9 = (
    ROOT
    / ".work/official-conformance/outcomes"
    / "core-dummy-node-bidi-strict-paired-v9"
)
SOURCE_FILES = (
    "scripts/run_playwright_core_dummy_node_bidi_strict_v3.py",
    "scripts/verify_playwright_core_dummy_node_bidi_strict_v3.py",
    "scripts/test_verify_playwright_core_dummy_node_bidi_strict_v3.py",
    "scripts/run_playwright_core_dummy_node_launch.py",
    "scripts/official_result_destination.py",
    "scripts/run_playwright_official_conformance.py",
    "tests/official/playwright-core-dummy-node-bidi-strict-v3-preload.cjs",
    "tests/official/playwright-core-server-runtime.cjs",
    "tests/official/playwright-core-candidate-preload.cjs",
    "tests/official/dummy-core-guard/node",
    "tests/official/dummy-core-bidi-firefox-guard/node",
)


def directory_snapshot(directory: Path) -> list[dict[str, str]]:
    if directory.is_symlink() or not directory.is_dir():
        raise ValueError(f"protected output directory missing or linked: {directory}")
    entries = []
    for path in sorted(directory.rglob("*")):
        if path.is_symlink():
            raise ValueError("protected output directory contains a link")
        relative = path.relative_to(directory).as_posix()
        if path.is_dir():
            entries.append({"path": relative, "kind": "directory"})
        elif path.is_file():
            entries.append(
                {"path": relative, "kind": "file", "sha256": base.digest(path)}
            )
        else:
            raise ValueError("protected output directory contains a special file")
    return entries


def listening_fixture_pids() -> set[int]:
    command = ["/usr/sbin/lsof", "-nP", "-iTCP:8907-8910", "-sTCP:LISTEN", "-Fp"]
    completed = subprocess.run(command, capture_output=True, text=True, check=False)
    if completed.returncode not in (0, 1) or completed.stderr:
        raise ValueError("cannot determine ownership of fixed fixture listener ports")
    lines = completed.stdout.splitlines()
    if any(not line.startswith("p") or not line[1:].isdigit() for line in lines):
        raise ValueError("fixture listener inventory is malformed")
    return {int(line[1:]) for line in lines}


def process_group_members(group_id: int) -> set[int]:
    completed = subprocess.run(
        ["/bin/ps", "-axo", "pid=,pgid="], capture_output=True, text=True, check=True
    )
    members = set()
    for line in completed.stdout.splitlines():
        fields = line.split()
        if len(fields) != 2 or not all(field.isdigit() for field in fields):
            raise ValueError("process group inventory is malformed")
        if int(fields[1]) == group_id:
            members.add(int(fields[0]))
    return members


def wait_group_empty(
    group_id: int, seconds: float, leader: subprocess.Popen[object]
) -> bool:
    deadline = time.monotonic() + seconds
    while True:
        leader.poll()  # Reap an exited group leader before inspecting ps output.
        if not process_group_members(group_id):
            return True
        if time.monotonic() >= deadline:
            return False
        time.sleep(0.05)


def stop_owned_group(group_id: int, leader: subprocess.Popen[object]) -> None:
    for sig in (signal.SIGTERM, signal.SIGKILL):
        try:
            os.killpg(group_id, sig)
        except ProcessLookupError:
            return
        try:
            if wait_group_empty(group_id, 2.0, leader):
                return
        except (OSError, ValueError, subprocess.CalledProcessError):
            continue
    raise ValueError("isolated official process group could not be reaped")


def require_tailscale_runtime(bind: str, host: str) -> None:
    address = ipaddress.ip_address(bind)
    if address not in ipaddress.ip_network("100.64.0.0/10") or not host.endswith(
        ".tail8f3b66.ts.net"
    ):
        raise ValueError(
            "BiDi dummy fixtures must bind to the verified Tailscale tailnet"
        )


def checked_manifest(upstream: Path) -> dict[str, Any]:
    if (
        MANIFEST.is_symlink()
        or not MANIFEST.is_file()
        or base.digest(MANIFEST) != MANIFEST_SHA256
    ):
        raise ValueError("BiDi dummy manifest missing or SHA-256 differs")
    manifest = json.loads(MANIFEST.read_text())
    expected_cases = [
        (6110, "bidi-chrome-canary-library", "chromium", "53abd42f8ece2b451e5f"),
        (6111, "bidi-chrome-stable-library", "chromium", "ee1b24a2e94e134f0b8a"),
        (6112, "bidi-chromium-library", "chromium", "f708c432fe2c80d2440a"),
        (6113, "moz-firefox-beta-library", "firefox", "a1c0f40a45acfee26558"),
        (6114, "moz-firefox-library", "firefox", "6ecf3d1ccae0b454c888"),
        (6115, "moz-firefox-nightly-library", "firefox", "88b0839d68b9d44cb588"),
    ]
    required_cases = [
        {
            "index": index,
            "project": project,
            "engine": engine,
            "specId": f"d1a604bdf6287c6776f3-{suffix}",
        }
        for index, project, engine, suffix in expected_cases
    ]
    if (
        not isinstance(manifest, dict)
        or set(manifest)
        != {
            "schema",
            "repository",
            "tag",
            "commit",
            "executionPolicy",
            "selector",
            "title",
            "asset",
            "config",
            "candidateProfile",
            "candidateSourceSha256",
            "candidateArtifactSha256",
            "v9CandidateLedgerSha256",
            "v9OracleLedgerSha256",
            "v9ReceiptSha256",
            "upstreamSources",
            "cases",
        }
        or manifest["schema"] != 1
        or manifest["repository"] != "https://github.com/microsoft/playwright.git"
        or manifest["tag"] != "v1.58.0"
        or manifest["commit"] != COMMIT
        or manifest["executionPolicy"]
        != "pinned-bidi-dummy-node-public-launch-strict-v3"
        or manifest["selector"] != SELECTOR
        or manifest["title"] != TITLE
        or manifest["asset"] != ASSET
        or manifest["config"] != "tests/bidi/playwright.config.ts"
        or manifest["candidateProfile"] != PROFILE
        or manifest["candidateSourceSha256"] != SOURCE_SHA256
        or manifest["candidateArtifactSha256"] != ARTIFACT_SHA256
        or manifest["cases"] != required_cases
    ):
        raise ValueError("BiDi dummy manifest identity/cases differ")
    base.pinned_checkout(upstream)
    if (upstream / ".env").exists() or (upstream / ".env").is_symlink():
        raise ValueError("BiDi dummy lane forbids upstream .env")
    for relative, sha256 in cast(dict[str, str], manifest["upstreamSources"]).items():
        source = upstream / relative
        if source.is_symlink() or not source.is_file() or base.digest(source) != sha256:
            raise ValueError(f"BiDi dummy upstream source drifted: {relative}")
    if not os.access(upstream / ASSET, os.X_OK):
        raise ValueError("BiDi dummy upstream asset is not executable")
    for name, key in (
        ("candidate", "v9CandidateLedgerSha256"),
        ("oracle", "v9OracleLedgerSha256"),
    ):
        ledger = V9 / f"{name}.jsonl"
        if (
            ledger.is_symlink()
            or not ledger.is_file()
            or base.digest(ledger) != manifest[key]
        ):
            raise ValueError(f"BiDi dummy v9 {name} predecessor differs")
        rows = ledger.read_text().splitlines()
        if len(rows) != 48_477:
            raise ValueError(f"BiDi dummy v9 {name} row count differs")
        for case in required_cases:
            row = json.loads(rows[case["index"]])
            if any(
                row.get(field) != value
                for field, value in {
                    "projectId": case["project"],
                    "specId": case["specId"],
                    "file": "library/browsertype-launch.spec.ts",
                    "line": 64,
                    "column": 3,
                    "sourceClassification": "public-api-candidate",
                    "outcome": "unexpected" if name == "candidate" else "expected",
                    "attempts": 1,
                    "evidenceScope": V9_EVIDENCE_SCOPE,
                    "publicApiParityEvidence": False,
                }.items()
            ):
                raise ValueError(f"BiDi dummy v9 {name} target row differs")
    receipt = V9 / "promotion-receipt.json"
    if (
        receipt.is_symlink()
        or not receipt.is_file()
        or base.digest(receipt) != manifest["v9ReceiptSha256"]
    ):
        raise ValueError("BiDi dummy v9 predecessor receipt differs")
    source = (upstream / "tests/library/browsertype-launch.spec.ts").read_text()
    body = "\n".join(source.splitlines()[63:73])
    if (
        not body.startswith(
            "it('should reject if launched browser fails immediately', "
            "async ({ mode, browserType, asset, isWindows, channel }) => {"
        )
        or "browserType.launch({ executablePath: asset('dummy_bad_browser_executable.js') })"
        not in body
        or (
            "expect(waitError!.message).toContain(isWindows ? "
            "'browserType.launch: spawn UNKNOWN' : 'Browser logs:')"
        )
        not in body
        or "__testHook" in body
        or "._" in body
    ):
        raise ValueError("BiDi dummy unchanged public test body differs")
    return cast(dict[str, Any], manifest)


def official_case(
    result: Path, case: dict[str, Any], mode: str, upstream: Path, output_dir: Path
) -> tuple[str, dict[str, Any]]:
    report = json.loads(result.read_text())
    if not isinstance(report, dict) or not isinstance(report.get("suites"), list):
        raise ValueError("BiDi dummy official JSON report is malformed")
    specs: list[dict[str, Any]] = []

    def visit(node: object) -> None:
        if (
            not isinstance(node, dict)
            or not isinstance(node.get("specs", []), list)
            or not isinstance(node.get("suites", []), list)
        ):
            raise ValueError("BiDi dummy official suite is malformed")
        specs.extend(cast(list[dict[str, Any]], node.get("specs", [])))
        for child in cast(list[object], node.get("suites", [])):
            visit(child)

    for suite in cast(list[object], report["suites"]):
        visit(suite)
    if len(specs) != 1:
        raise ValueError("BiDi dummy official report is not exactly one spec")
    spec = specs[0]
    tests = spec.get("tests")
    if (
        spec.get("id") != case["specId"]
        or spec.get("file") != "library/browsertype-launch.spec.ts"
        or spec.get("line") != 64
        or spec.get("column") != 3
        or spec.get("title") != TITLE
        or not isinstance(tests, list)
        or len(tests) != 1
        or not isinstance(tests[0], dict)
    ):
        raise ValueError("BiDi dummy official spec identity differs")
    test = cast(dict[str, Any], tests[0])
    attempts = test.get("results")
    outcome = test.get("status")
    expected = outcome == "expected"
    if (
        outcome not in ("expected", "unexpected")
        or (mode == "oracle" and not expected)
        or spec.get("ok") is not expected
        or test.get("projectId") != case["project"]
        or test.get("expectedStatus") != "passed"
        or test.get("annotations") != []
        or not isinstance(attempts, list)
        or len(attempts) != 1
        or not isinstance(attempts[0], dict)
        or attempts[0].get("status") != ("passed" if expected else "failed")
        or attempts[0].get("retry") != 0
        or attempts[0].get("workerIndex") != 0
        or not isinstance(report.get("stats"), dict)
        or any(
            report["stats"].get(field) != value
            for field, value in {
                "expected": int(expected),
                "unexpected": int(not expected),
                "skipped": 0,
                "flaky": 0,
            }.items()
        )
        or report.get("errors") != []
        or not isinstance(report.get("config"), dict)
        or report["config"].get("workers") != 1
        or len(
            [
                project
                for project in report["config"].get("projects", [])
                if isinstance(project, dict)
                and project.get("id") == case["project"]
                and project.get("retries") == 0
                and project.get("outputDir") == str(output_dir)
            ]
        )
        != 1
    ):
        raise ValueError("BiDi dummy official outcome/attempt differs")
    errors = attempts[0].get("errors")
    if expected and errors != []:
        raise ValueError("BiDi dummy passing assertion has errors")
    if not expected and (
        not isinstance(errors, list)
        or len(errors) != 1
        or not isinstance(errors[0], dict)
        or errors[0].get("location")
        != {
            "file": str(upstream / "tests/library/browsertype-launch.spec.ts"),
            "line": 72,
            "column": 32,
        }
        or '"Browser logs:"' not in str(errors[0].get("message", ""))
    ):
        raise ValueError("BiDi dummy RED is not the original public assertion")
    return cast(str, outcome), test


def checked_process_and_profile(
    paths: dict[str, Path], mode: str, engine: str, project: str, run_id: str
) -> None:
    """Require the guard to observe one exited Node and a removed owned profile."""
    processes = [
        json.loads(line)
        for line in paths["processReceipt"].read_text().splitlines()
        if line
    ]
    profiles = [
        json.loads(line)
        for line in paths["profileReceipt"].read_text().splitlines()
        if line
    ]
    if len(processes) != 1 or len(profiles) != (0 if mode == "candidate" else 1):
        raise ValueError(
            "BiDi dummy requires one process and the expected profile receipt"
        )
    process = processes[0]
    if any(
        process.get(key) != value
        for key, value in {
            "event": "dummy-node-exited",
            "mode": mode,
            "runId": run_id,
            "exitCode": 1,
            "childReaped": True,
        }.items()
    ):
        raise ValueError("BiDi dummy guard did not reap exactly one exit-1 Node child")
    if engine == "firefox":
        if process.get("engine") != engine or process.get("project") != project:
            raise ValueError("BiDi dummy Firefox guard project differs")
        data = process.get("browserProfile")
        profile = Path(data.get("path", "")) if isinstance(data, dict) else Path("")
    else:
        profile = Path(process.get("browserProfilePath", ""))
    prefix = (
        "pwright-node-bidi-startup-"
        if mode == "candidate"
        else f"playwright_{engine}dev_profile-"
    )
    if (
        not profile.is_absolute()
        or not profile.name.startswith(prefix)
        or profile.exists()
        or profile.is_symlink()
    ):
        raise ValueError("BiDi dummy owned browser profile was not cleaned")
    if mode == "oracle" and profiles[0].get("path") != str(profile):
        raise ValueError("BiDi dummy oracle profile receipt does not match guard")


def run(mode: str, project: str, upstream: Path, result: Path) -> None:
    upstream = upstream.resolve()
    manifest = checked_manifest(upstream)
    cases = {case["project"]: case for case in manifest["cases"]}
    if mode not in ("candidate", "oracle") or project not in cases:
        raise ValueError("BiDi dummy mode/project is not one of the six pinned cases")
    case = cast(dict[str, Any], cases[project])
    engine = cast(str, case["engine"])
    base.preflight_environment(os.environ.copy())
    if any(
        name in os.environ
        for name in (
            "BIDI_CRPATH",
            "BIDI_FFPATH",
            "PWTEST_USE_BIDI_EXPECTATIONS",
            "PLAYWRIGHT_SERVICE_OS",
            "PWTEST_FIREFOX_USER_PREFS",
        )
    ):
        raise ValueError(
            "BiDi dummy lane forbids browser/channel expectation overrides"
        )
    node_command = shutil.which("node", path="/opt/homebrew/bin:/usr/bin:/bin")
    if node_command is None:
        raise ValueError("BiDi dummy lane requires system Node")
    node = Path(node_command).resolve()
    guard = CHROMIUM_GUARD if engine == "chromium" else FIREFOX_GUARD
    if any(
        path.is_symlink() or not path.is_file() for path in (node, guard)
    ) or not os.access(guard, os.X_OK):
        raise ValueError("BiDi dummy runtime/guard is missing or linked")
    allowed = [
        node,
        upstream / ASSET,
        guard,
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
    artifact = candidate / f"native/{PROFILE}/pwright.node" if candidate else None
    if candidate and (
        base.source_digest(candidate) != manifest["candidateSourceSha256"]
        or artifact is None
        or artifact.is_symlink()
        or not artifact.is_file()
        or base.digest(artifact) != manifest["candidateArtifactSha256"]
    ):
        raise ValueError("BiDi dummy candidate frozen source/artifact differs")
    artifacts = {
        relative: base.directory_digest(upstream / relative)
        for relative in base.UPSTREAM_ARTIFACT_DIRS
    }
    node_sha256 = base.digest(node)
    local_sources = {
        relative: base.digest(ROOT / relative) for relative in SOURCE_FILES
    }
    bind, host = _tailscale_runtime(os.environ.copy())
    require_tailscale_runtime(bind, host)
    upstream_output_dir = upstream / "test-results"
    upstream_output_snapshot = directory_snapshot(upstream_output_dir)
    if listening_fixture_pids():
        raise ValueError(
            "BiDi dummy fixed fixture ports 8907-8910 already have listeners"
        )
    result = prepare_result_destination(result)
    if result.parent != TASK_RESULTS or result.suffix != ".json":
        raise ValueError(
            "BiDi dummy result must be a JSON file in the task-owned results directory"
        )
    for parent in (
        ROOT / ".work",
        ROOT / ".work/official-conformance",
        TASK_RESULTS.parent,
    ):
        if parent.is_symlink() or not parent.is_dir():
            raise ValueError("BiDi dummy task results ancestor missing or linked")
    if TASK_RESULTS.is_symlink() or (
        TASK_RESULTS.exists() and not TASK_RESULTS.is_dir()
    ):
        raise ValueError(
            "BiDi dummy task results directory is linked or not a directory"
        )
    TASK_RESULTS.mkdir(exist_ok=True)
    if TASK_RESULTS.is_symlink() or TASK_RESULTS.resolve() != TASK_RESULTS:
        raise ValueError("BiDi dummy task results directory escaped its root")
    run_id = uuid.uuid4().hex
    output_dir = TASK_RESULTS / f"{result.stem}-{run_id}.output"
    paths = {
        "launchReceipt": result.with_suffix(".launch-receipt.jsonl"),
        "profileReceipt": result.with_suffix(".profile-receipt.jsonl"),
        "processReceipt": result.with_suffix(".process-receipt.jsonl"),
        "log": result.with_suffix(".log"),
        "strict": result.with_suffix(".strict.json"),
    }
    for target in (
        result,
        result.with_suffix(".provenance.json"),
        output_dir,
        *paths.values(),
    ):
        if target.is_symlink() or target.exists():
            raise ValueError(f"BiDi dummy result destination already exists: {target}")
    for key in ("launchReceipt", "profileReceipt", "processReceipt"):
        paths[key].open("x").close()
    env = os.environ.copy()
    env.pop("CI", None)
    env["PWTEST_MODE"] = "default"
    env["PWRIGHT_OFFICIAL_SERVER_IP"] = bind
    env["PWRIGHT_OFFICIAL_SERVER_HOST"] = host
    env["PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM"] = str(upstream)
    for name, value in {
        "PWRIGHT_BIDI_DUMMY_MODE": mode,
        "PWRIGHT_BIDI_DUMMY_PROJECT": project,
        "PWRIGHT_BIDI_DUMMY_ENGINE": engine,
        "PWRIGHT_BIDI_DUMMY_RUN_ID": run_id,
        "PWRIGHT_BIDI_DUMMY_MANIFEST": str(MANIFEST),
        "PWRIGHT_BIDI_DUMMY_MANIFEST_SHA256": MANIFEST_SHA256,
        "PWRIGHT_BIDI_DUMMY_LAUNCH_RECEIPT": str(paths["launchReceipt"]),
        "PWRIGHT_BIDI_DUMMY_PROFILE_RECEIPT": str(paths["profileReceipt"]),
    }.items():
        env[name] = value
    if engine == "chromium":
        env.update(
            {
                "PWRIGHT_DUMMY_PROCESS_ASSET": str(upstream / ASSET),
                "PWRIGHT_DUMMY_PROCESS_RECEIPT": str(paths["processReceipt"]),
                "PWRIGHT_DUMMY_REAL_NODE": str(node),
                "PWRIGHT_DUMMY_STRICT_RUN_ID": run_id,
                "PWRIGHT_DUMMY_HARNESS_PID": str(os.getpid()),
                "PWRIGHT_DUMMY_LAUNCH_MODE": mode,
            }
        )
    else:
        env.update(
            {
                "PWRIGHT_BIDI_FIREFOX_GUARD_ASSET": str(upstream / ASSET),
                "PWRIGHT_BIDI_FIREFOX_GUARD_PROCESS_RECEIPT": str(
                    paths["processReceipt"]
                ),
                "PWRIGHT_BIDI_FIREFOX_GUARD_REAL_NODE": str(node),
                "PWRIGHT_BIDI_FIREFOX_GUARD_NODE_SHA256": node_sha256,
                "PWRIGHT_BIDI_FIREFOX_GUARD_HARNESS_PID": str(os.getpid()),
            }
        )
    env["PLAYWRIGHT_JSON_OUTPUT_FILE"] = str(result)
    env["PATH"] = f"{guard.parent}:/opt/homebrew/bin:/usr/bin:/bin"
    preloads = [SERVER_PRELOAD]
    if candidate:
        env["PWRIGHT_NODE_CANDIDATE"] = str(candidate)
        env["PWRIGHT_NATIVE_PROFILE"] = PROFILE
        preloads.append(CANDIDATE_PRELOAD)
    else:
        env.pop("PWRIGHT_NODE_CANDIDATE", None)
        env.pop("PWRIGHT_NATIVE_PROFILE", None)
    preloads.append(PRELOAD)
    env["NODE_OPTIONS"] = " ".join(f"--require={preload}" for preload in preloads)
    command = [
        "/usr/bin/sandbox-exec",
        "-p",
        sandbox,
        str(node),
        str(upstream / "packages/playwright/cli.js"),
        "test",
        "--config=tests/bidi/playwright.config.ts",
        "--reporter=line,json",
        "--workers=1",
        "--retries=0",
        f"--output={output_dir}",
        f"--project={project}",
        SELECTOR,
    ]
    group_leaked = False
    with paths["log"].open("x") as log:
        launched = subprocess.Popen(
            command,
            cwd=upstream,
            env=env,
            stdout=log,
            stderr=subprocess.STDOUT,
            start_new_session=True,
        )
        cli_pid = launched.pid
        try:
            exit_code = launched.wait(timeout=120)
        finally:
            try:
                group_leaked = not wait_group_empty(cli_pid, 2.0, launched)
            except (OSError, ValueError, subprocess.CalledProcessError):
                group_leaked = True
            if group_leaked:
                stop_owned_group(cli_pid, launched)
            launched.wait(timeout=5)
            if process_group_members(cli_pid):
                raise ValueError(
                    "BiDi dummy isolated process group retained descendants"
                )
            if listening_fixture_pids():
                raise ValueError(
                    "BiDi dummy fixture listener remained after official CLI exit"
                )
    if group_leaked:
        raise ValueError("BiDi dummy official CLI leaked descendants requiring cleanup")
    if directory_snapshot(upstream_output_dir) != upstream_output_snapshot:
        raise ValueError(
            "BiDi dummy official CLI mutated pre-existing upstream test-results"
        )
    if output_dir.is_symlink() or not output_dir.is_dir():
        raise ValueError("BiDi dummy fresh task-owned output directory was not created")
    if result.is_symlink() or not result.is_file():
        raise ValueError("BiDi dummy official runner did not emit JSON")
    outcome, test = official_case(result, case, mode, upstream, output_dir)
    if exit_code != (0 if outcome == "expected" else 1):
        raise ValueError("BiDi dummy CLI exit differs from official outcome")
    if (
        checked_manifest(upstream) != manifest
        or {relative: base.digest(ROOT / relative) for relative in SOURCE_FILES}
        != local_sources
    ):
        raise ValueError("BiDi dummy tooling/source drifted during execution")
    if {
        relative: base.directory_digest(upstream / relative)
        for relative in base.UPSTREAM_ARTIFACT_DIRS
    } != artifacts or base.digest(node) != node_sha256:
        raise ValueError("BiDi dummy upstream build/Node drifted during execution")
    if candidate and (
        base.source_digest(candidate) != manifest["candidateSourceSha256"]
        or artifact is None
        or base.digest(artifact) != manifest["candidateArtifactSha256"]
    ):
        raise ValueError("BiDi dummy candidate drifted during execution")
    if directory_snapshot(upstream_output_dir) != upstream_output_snapshot:
        raise ValueError(
            "BiDi dummy upstream test-results changed during receipt validation"
        )
    launches = [
        json.loads(line)
        for line in paths["launchReceipt"].read_text().splitlines()
        if line
    ]
    if len(launches) != 2 or [event.get("event") for event in launches] != [
        "launch-invoked",
        "launch-failed",
    ]:
        raise ValueError("BiDi dummy public launch was not observed exactly once")
    processes = [
        json.loads(line)
        for line in paths["processReceipt"].read_text().splitlines()
        if line
    ]
    checked_process_and_profile(paths, mode, engine, project, run_id)
    hashes = {key: base.digest(path) for key, path in paths.items() if key != "strict"}
    sidecar = {
        "schema": 1,
        "scope": "core-dummy-node-bidi-strict-v3",
        "mode": mode,
        "project": project,
        "engine": engine,
        "index": case["index"],
        "specId": case["specId"],
        "selector": SELECTOR,
        "upstreamCommit": COMMIT,
        "upstream": str(upstream),
        "upstreamOutputDir": str(upstream_output_dir),
        "upstreamOutputSnapshot": upstream_output_snapshot,
        "upstreamSources": manifest["upstreamSources"],
        "upstreamArtifactsSha256": artifacts,
        "manifest": str(MANIFEST),
        "manifestSha256": MANIFEST_SHA256,
        "candidate": str(candidate) if candidate else None,
        "candidateProfile": PROFILE if candidate else None,
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
        "outputDir": str(output_dir),
        "outputDirWasAbsentBefore": True,
        "processGroupId": cli_pid,
        "processGroupIsolated": True,
        "processGroupEmptyAfter": True,
        "fixturePorts": list(FIXTURE_PORTS),
        "fixturePortsFreeBefore": True,
        "fixturePortsFreeAfter": True,
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
        "selectedExecutable": "pinned-exit-1-node-asset",
        "browserExecutionEvidence": False,
        "candidatePublicMethodAttested": bool(candidate),
        "candidateNativeArtifactPinned": bool(candidate),
        "dummyProcessExitObserved": bool(processes),
        "publicApiParityEvidence": False,
    }
    with paths["strict"].open("x") as stream:
        json.dump(sidecar, stream, sort_keys=True, indent=2)
        stream.write("\n")
    print(
        f"strict BiDi dummy {mode} {project}: {outcome} {paths['strict']}", flush=True
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("candidate", "oracle"))
    parser.add_argument("project")
    parser.add_argument("upstream", type=Path)
    parser.add_argument("--result", required=True, type=Path)
    args = parser.parse_args()
    try:
        run(args.mode, args.project, args.upstream, args.result)
    except (
        OSError,
        TypeError,
        ValueError,
        subprocess.CalledProcessError,
        subprocess.TimeoutExpired,
        json.JSONDecodeError,
    ) as error:
        parser.exit(2, f"strict BiDi dummy capture rejected: {error}\n")


if __name__ == "__main__":
    main()
