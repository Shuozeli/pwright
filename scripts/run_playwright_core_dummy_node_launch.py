#!/usr/bin/env python3
"""Isolated Core :64 launch of the pinned failing Node asset.

The historical Core runner rejects these schema-2 manifests. The default
wrapper is diagnostic; --strict binds the unchanged official assertion,
candidate runtime artifact, live process/profile ancestry and a fresh result.
Both modes deny unlisted process execution and outbound networking.
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
from typing import cast

from official_result_destination import prepare_result_destination
from run_playwright_official_conformance import _tailscale_runtime

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "tests/official/playwright-core-dummy-node-launch-manifest.json"
TRACE_MANIFEST = (
    ROOT
    / "tests/official/playwright-core-dummy-node-launch-playwright-trace-manifest.json"
)
STRICT_MANIFEST = (
    ROOT / "tests/official/playwright-core-dummy-node-launch-strict-v1-manifest.json"
)
POLICY = "pinned-dummy-node-launch-v1"
STRICT_POLICY = "pinned-dummy-node-launch-strict-v1"
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
SUITE = "library-browsertype-launch-dummy-node-failure-chromium"
SELECTOR = "tests/library/browsertype-launch.spec.ts:64"
SPEC_FILE = "library/browsertype-launch.spec.ts"
SPEC_ID = "d1a604bdf6287c6776f3-522221011eea38945220"
TITLE = "should reject if launched browser fails immediately"
PROJECT = "chromium-library"
ASSET = "tests/assets/dummy_bad_browser_executable.js"
PROFILE = "owned-launch"
TRACE_PROFILE = "playwright-trace"
CANDIDATE_SOURCE_SHA256 = (
    "e5ddf963072529d39640d48cdda83ec01bfdbf034ae0b174b42ec656df7ea038"
)
CANDIDATE_ARTIFACT_SHA256 = (
    "b37efd40f111e305cf976812c4048a6782ce76ab36418d4f13e6ac320f87788a"
)
TRACE_CANDIDATE_SOURCE_SHA256 = (
    "1e98c1dd2738adb8cebf99a418ceacbc1c75d1b03c80135e2613fd704295d42f"
)
TRACE_CANDIDATE_ARTIFACT_SHA256 = (
    "13d6db323b599d57608d99b7f18f8a06856c6425e1931dfdb718f74d356c765d"
)
STRICT_CANDIDATE_SOURCE_SHA256 = (
    "a2bb47a7c22f88b1c8cbaff72fb107de0a015eec85f6a4101d94e0cbddbfca5a"
)
STRICT_CANDIDATE_ARTIFACT_SHA256 = (
    "ab6be17625aadadbfce4bbc72866c72732191572c40b951821a68f947509cf3a"
)
UPSTREAM_SOURCES = {
    "tests/library/browsertype-launch.spec.ts": "64fe24188ec96bb0d5d1ecca5af37986b16d82451b91e4db80593f707126bd05",
    ASSET: "9928a78e7de42f29dfc61783a2f31673b387d7d6ea27cda5bc4472e4338c1f7f",
    "tests/library/playwright.config.ts": "338eb705bb4a17322dfba39d1c6770613bf8cf62cfec5c43bf7cece3e5db8146",
    "tests/config/browserTest.ts": "0ef5440e164d262e7c3d01523448b5d3c4d50b99060604c20f168f04419a13d5",
    "tests/config/testMode.ts": "7c12e6bd3fa7e2e3e4ff296277f4a60656d4d8a0c536cb55d9d6c1e83016f575",
    "tests/config/serverFixtures.ts": "4c0039a149bd61e8707adddc626b345632561bd343749293ddbe9458f6ac617b",
    "packages/playwright-core/src/client/browserType.ts": "f46ad9dc1a7d5c9e90b2adda605ae5949bb138ad42fc48253cd59ec65a604618",
    "packages/playwright-core/src/server/browserType.ts": "47765ae8932d697c126e4a12b4a9998e3f83021b3ecfc159c4572137deb338ac",
    "packages/playwright-core/src/server/utils/processLauncher.ts": "d9c3bc627b0e83cf4d307201cddb63aa7aaf7a4f2878dcc676d92c9e47c4465b",
}
SERVER_PRELOAD = "tests/official/playwright-core-server-runtime.cjs"
CANDIDATE_PRELOAD = "tests/official/playwright-core-candidate-preload.cjs"
GUARD_PRELOAD = "tests/official/playwright-core-dummy-node-launch-preload.cjs"
PROCESS_GUARD = "tests/official/dummy-core-guard/node"
LOCAL_SOURCES = (
    "scripts/run_playwright_core_dummy_node_launch.py",
    "scripts/official_result_destination.py",
    "scripts/run_playwright_official_conformance.py",
    SERVER_PRELOAD,
    CANDIDATE_PRELOAD,
    GUARD_PRELOAD,
    PROCESS_GUARD,
)
STRICT_SOURCES = LOCAL_SOURCES + (
    "scripts/verify_playwright_core_dummy_node_launch_strict_v1.py",
)
UPSTREAM_ARTIFACT_DIRS = (
    "packages/playwright-core/lib",
    "packages/playwright/lib",
)


def digest(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def source_digest(candidate: Path) -> str:
    package = candidate / "package.json"
    source = candidate / "src"
    if (
        not package.is_file()
        or package.is_symlink()
        or not source.is_dir()
        or source.is_symlink()
    ):
        raise ValueError("dummy-node candidate package/src is missing or linked")
    entries = list(source.rglob("*"))
    if any(path.is_symlink() for path in entries):
        raise ValueError("dummy-node candidate source contains a symlink")
    files = [package, *(path for path in entries if path.is_file())]
    value = hashlib.sha256()
    for path in sorted(files, key=lambda item: item.relative_to(candidate).as_posix()):
        relative = path.relative_to(candidate).as_posix().encode()
        value.update(len(relative).to_bytes(8, "big"))
        value.update(relative)
        size = path.stat().st_size
        value.update(size.to_bytes(8, "big"))
        with path.open("rb") as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                value.update(chunk)
    return value.hexdigest()


def directory_digest(directory: Path) -> str:
    if directory.is_symlink() or not directory.is_dir():
        raise ValueError(
            f"dummy-node artifact directory is missing or linked: {directory}"
        )
    entries = list(directory.rglob("*"))
    if any(path.is_symlink() for path in entries):
        raise ValueError(
            f"dummy-node artifact directory contains a symlink: {directory}"
        )
    files = sorted(
        (path for path in entries if path.is_file()),
        key=lambda path: path.relative_to(directory).as_posix(),
    )
    if not files:
        raise ValueError(f"dummy-node artifact directory is empty: {directory}")
    value = hashlib.sha256()
    for path in files:
        relative = path.relative_to(directory).as_posix().encode()
        value.update(len(relative).to_bytes(8, "big"))
        value.update(relative)
        size = path.stat().st_size
        value.update(size.to_bytes(8, "big"))
        with path.open("rb") as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                value.update(chunk)
    return value.hexdigest()


def checked_manifest_path(manifest_path: Path) -> Path:
    if manifest_path.is_symlink():
        raise ValueError("dummy-node manifest path must be the dedicated local file")
    selected = manifest_path.resolve()
    if selected not in (MANIFEST, TRACE_MANIFEST, STRICT_MANIFEST):
        raise ValueError("dummy-node manifest path must be the dedicated local file")
    return selected


def candidate_policy(manifest_path: Path) -> tuple[str, str, str]:
    selected = checked_manifest_path(manifest_path)
    if selected == MANIFEST:
        return PROFILE, CANDIDATE_SOURCE_SHA256, CANDIDATE_ARTIFACT_SHA256
    if selected == STRICT_MANIFEST:
        return (
            TRACE_PROFILE,
            STRICT_CANDIDATE_SOURCE_SHA256,
            STRICT_CANDIDATE_ARTIFACT_SHA256,
        )
    return (
        TRACE_PROFILE,
        TRACE_CANDIDATE_SOURCE_SHA256,
        TRACE_CANDIDATE_ARTIFACT_SHA256,
    )


def pinned_manifest(
    upstream: Path, manifest_path: Path = MANIFEST
) -> dict[str, object]:
    profile, source_sha, artifact_sha = candidate_policy(manifest_path)
    if not manifest_path.is_file():
        raise ValueError("dummy-node manifest path must be the dedicated local file")
    raw = json.loads(manifest_path.read_text())
    expected_root = {
        "schema": 2,
        "repository": "https://github.com/microsoft/playwright.git",
        "tag": "v1.58.0",
        "commit": COMMIT,
        "executionPolicy": STRICT_POLICY
        if manifest_path == STRICT_MANIFEST
        else POLICY,
    }
    if not isinstance(raw, dict) or set(raw) != {*expected_root, "suite"}:
        raise ValueError("dummy-node manifest root schema differs")
    if any(raw[key] != value for key, value in expected_root.items()):
        raise ValueError("dummy-node manifest repository or execution policy differs")
    suite = raw["suite"]
    expected_suite = {
        "id": SUITE,
        "role": (
            "browserless-owned-public-launch"
            if manifest_path == STRICT_MANIFEST
            else "browserless-dummy-process-diagnostic"
        ),
        "runner": "packages/playwright/cli.js",
        "config": "tests/library/playwright.config.ts",
        "project": PROJECT,
        "selector": SELECTOR,
        "specId": SPEC_ID,
        "title": TITLE,
        "requiresBrowser": False,
        "fixtureMode": "dummy-node-launch",
        "candidateAdapter": "test-mode-playwright-core-owned-public-launch",
        "oracleAdapter": "test-mode-playwright-core-public-launch",
        "executable": ASSET,
        "candidateProfile": profile,
        "candidateSourceSha256": source_sha,
        "candidateArtifactSha256": artifact_sha,
        "upstreamSources": UPSTREAM_SOURCES,
    }
    if not isinstance(suite, dict) or suite != expected_suite:
        raise ValueError("dummy-node manifest is not the exact one-spec policy")
    if (upstream / ".env").exists() or (upstream / ".env").is_symlink():
        raise ValueError("dummy-node lane forbids upstream .env overrides")
    for relative, expected in UPSTREAM_SOURCES.items():
        path = upstream / relative
        if path.is_symlink() or not path.is_file() or digest(path) != expected:
            raise ValueError(f"dummy-node upstream source drifted: {relative}")
    asset = upstream / ASSET
    if not os.access(asset, os.X_OK):
        raise ValueError("dummy-node pinned asset is not executable")
    return cast(dict[str, object], raw)


def pinned_checkout(upstream: Path) -> None:
    for arguments, expected in (
        (("rev-parse", "HEAD"), COMMIT),
        (("status", "--porcelain", "--untracked-files=no"), ""),
    ):
        completed = subprocess.run(
            ["git", "-C", str(upstream), *arguments],
            check=True,
            capture_output=True,
            text=True,
        )
        if completed.stdout.strip() != expected:
            raise ValueError(
                "dummy-node upstream checkout differs from pinned clean revision"
            )


def preflight_environment(env: dict[str, str]) -> None:
    forbidden = (
        "NODE_OPTIONS",
        "NODE_PATH",
        "CRPATH",
        "PWTEST_CHANNEL",
        "PWTEST_VIDEO",
        "PWTEST_TRACE",
        "PWRIGHT_CANDIDATE_CDP_ENDPOINT",
        "PWRIGHT_ORACLE_CDP_ENDPOINT",
        "PWRIGHT_CDP_ENDPOINT",
        "PWRIGHT_CDP_WS_URL",
        "PYTHONHOME",
        "PYTHONPATH",
        "PYTHONUSERBASE",
        "PYTHONINSPECT",
    )
    if (
        any(name in env for name in forbidden)
        or any(name.startswith(("DYLD_", "LD_")) for name in env)
        or env.get("PWTEST_MODE", "default") != "default"
    ):
        raise ValueError(
            "dummy-node lane forbids inherited launch, CDP or test-mode overrides"
        )


def command(upstream: Path, node: Path) -> list[str]:
    return [
        str(node),
        str(upstream / "packages/playwright/cli.js"),
        "test",
        "--config=tests/library/playwright.config.ts",
        "--reporter=line,json",
        "--workers=1",
        "--retries=0",
        f"--project={PROJECT}",
        SELECTOR,
    ]


def report_case(result: Path) -> dict[str, object]:
    raw = json.loads(result.read_text())
    if not isinstance(raw, dict) or not isinstance(raw.get("suites"), list):
        raise ValueError("dummy-node result lacks suites")
    found: list[dict[str, object]] = []

    def visit(node: object) -> None:
        if (
            not isinstance(node, dict)
            or not isinstance(node.get("specs", []), list)
            or not isinstance(node.get("suites", []), list)
        ):
            raise ValueError("dummy-node result has a malformed suite")
        found.extend(cast(list[dict[str, object]], node.get("specs", [])))
        for child in cast(list[object], node.get("suites", [])):
            visit(child)

    for top in cast(list[object], raw["suites"]):
        visit(top)
    if len(found) != 1:
        raise ValueError("dummy-node result must contain exactly one spec")
    spec = found[0]
    tests = spec.get("tests")
    if (
        spec.get("id") != SPEC_ID
        or spec.get("file") != SPEC_FILE
        or spec.get("line") != 64
        or spec.get("title") != TITLE
        or not isinstance(tests, list)
        or len(tests) != 1
        or not isinstance(tests[0], dict)
        or tests[0].get("projectId") != PROJECT
        or tests[0].get("expectedStatus") != "passed"
        or not isinstance(tests[0].get("results"), list)
    ):
        raise ValueError("dummy-node result differs from pinned Chromium case")
    return cast(dict[str, object], tests[0])


def strict_report_case(result: Path) -> dict[str, object]:
    raw = json.loads(result.read_text())
    selected = report_case(result)
    specs: list[dict[str, object]] = []

    def visit(suite: dict[str, object]) -> None:
        specs.extend(cast(list[dict[str, object]], suite.get("specs", [])))
        for child in cast(list[dict[str, object]], suite.get("suites", [])):
            visit(child)

    for suite in cast(list[dict[str, object]], raw["suites"]):
        visit(suite)
    attempts = selected.get("results")
    stats = raw.get("stats")
    config = raw.get("config")
    if (
        len(specs) != 1
        or specs[0].get("ok") is not True
        or specs[0].get("column") != 3
        or not isinstance(attempts, list)
        or len(attempts) != 1
        or not isinstance(attempts[0], dict)
        or attempts[0].get("status") != "passed"
        or attempts[0].get("retry") != 0
        or attempts[0].get("workerIndex") != 0
        or attempts[0].get("errors") != []
        or selected.get("status") != "expected"
        or selected.get("annotations") != []
        or not isinstance(config, dict)
        or config.get("workers") != 1
        or not isinstance(config.get("projects"), list)
        or len(
            [
                project
                for project in config["projects"]
                if isinstance(project, dict)
                and project.get("id") == PROJECT
                and project.get("retries") == 0
            ]
        )
        != 1
        or not isinstance(stats, dict)
        or any(
            stats.get(key) != value
            for key, value in {
                "expected": 1,
                "unexpected": 0,
                "skipped": 0,
                "flaky": 0,
            }.items()
        )
        or raw.get("errors") != []
    ):
        raise ValueError(
            "dummy-node strict report requires one passed case and zero skips"
        )
    return selected


def receipt_events(
    receipt: Path, *, mode: str, manifest_sha256: str, executable: Path
) -> None:
    if receipt.is_symlink() or not receipt.is_file():
        raise ValueError("dummy-node receipt is missing or linked")
    events = [json.loads(line) for line in receipt.read_text().splitlines() if line]
    expected = {
        "mode": mode,
        "publicMethod": "chromium.launch",
        "executablePath": str(executable),
        "assetSha256": UPSTREAM_SOURCES[ASSET],
        "manifestSha256": manifest_sha256,
    }
    if events != [
        {"event": name, **expected} for name in ("launch-invoked", "launch-failed")
    ]:
        raise ValueError("dummy-node lane requires one guarded failed public launch")


def strict_launch_receipt(
    receipt: Path,
    *,
    mode: str,
    run_id: str,
    manifest_sha256: str,
    executable: Path,
    process_event: dict[str, object],
    artifact_sha256: str | None,
) -> None:
    if receipt.is_symlink() or not receipt.is_file():
        raise ValueError("dummy-node strict public launch receipt is missing or linked")
    events = [json.loads(line) for line in receipt.read_text().splitlines() if line]
    expected = {
        "mode": mode,
        "publicMethod": "chromium.launch",
        "executablePath": str(executable),
        "assetSha256": UPSTREAM_SOURCES[ASSET],
        "manifestSha256": manifest_sha256,
        "runId": run_id,
        "launcherPid": process_event["guardParentPid"],
        "runtimeProfile": TRACE_PROFILE if mode == "candidate" else None,
        "runtimeArtifactSha256": artifact_sha256,
        "candidatePublicLaunchInherited": True if mode == "candidate" else None,
    }
    if (
        len(events) != 2
        or any(not isinstance(event, dict) for event in events)
        or any(
            event.get("event") != name
            or {
                key: value
                for key, value in event.items()
                if key not in ("event", "launcherParentPid")
            }
            != expected
            or not isinstance(event.get("launcherParentPid"), int)
            for event, name in zip(
                events, ("launch-invoked", "launch-failed"), strict=True
            )
        )
        or events[0].get("launcherParentPid") != events[1].get("launcherParentPid")
    ):
        raise ValueError("dummy-node strict public launch differs from owned process")


def process_receipt_event(receipt: Path, *, executable: Path) -> None:
    if receipt.is_symlink() or not receipt.is_file():
        raise ValueError("dummy-node process receipt is missing or linked")
    events = [json.loads(line) for line in receipt.read_text().splitlines() if line]
    if events != [
        {"event": "dummy-node-exited", "exitCode": 1, "asset": str(executable)}
    ]:
        raise ValueError("dummy-node process did not attest one pinned exit")


def strict_process_receipt(
    receipt: Path, *, run_id: str, mode: str, executable: Path, node: Path
) -> dict[str, object]:
    if receipt.is_symlink() or not receipt.is_file():
        raise ValueError("dummy-node strict process receipt is missing or linked")
    events = [json.loads(line) for line in receipt.read_text().splitlines() if line]
    if len(events) != 1 or not isinstance(events[0], dict):
        raise ValueError("dummy-node strict process receipt requires one launch")
    event = cast(dict[str, object], events[0])
    chain = event.get("ancestry")
    child = event.get("childSnapshot")
    profile_stat = event.get("browserProfileStat")
    arguments = event.get("browserArguments")

    def valid_pid(value: object) -> bool:
        return type(value) is int and value > 1

    if (
        set(event)
        != {
            "event",
            "exitCode",
            "asset",
            "runId",
            "mode",
            "guardPid",
            "guardParentPid",
            "childPid",
            "childReaped",
            "childSnapshot",
            "ancestry",
            "candidateProfile",
            "harnessPid",
            "browserProfilePath",
            "browserProfileStat",
            "browserArguments",
        }
        or event.get("event") != "dummy-node-exited"
        or event.get("exitCode") != 1
        or event.get("asset") != str(executable)
        or event.get("runId") != run_id
        or event.get("mode") != mode
        or event.get("childReaped") is not True
        or not isinstance(chain, list)
        or len(chain) < 2
        or not isinstance(child, dict)
        or set(child) != {"pid", "ppid", "uid", "executable"}
        or not all(
            valid_pid(event.get(key))
            for key in ("guardPid", "guardParentPid", "childPid", "harnessPid")
        )
        or child.get("pid") != event.get("childPid")
        or child.get("ppid") != event.get("guardPid")
        or child.get("uid") != os.getuid()
        or child.get("executable") != str(node)
        or any(
            not isinstance(entry, dict)
            or set(entry) != {"pid", "ppid", "uid", "executable"}
            for entry in chain
        )
        or any(
            not valid_pid(entry.get("pid"))
            or entry.get("uid") != os.getuid()
            or not isinstance(entry.get("executable"), str)
            or not Path(entry["executable"]).is_absolute()
            for entry in chain
        )
        or len({entry["pid"] for entry in chain}) != len(chain)
        or chain[0].get("pid") != event.get("guardPid")
        or chain[0].get("ppid") != event.get("guardParentPid")
        or chain[-1].get("pid") != event.get("harnessPid")
        or not all(
            chain[index].get("ppid") == chain[index + 1].get("pid")
            for index in range(len(chain) - 1)
        )
        or event.get("candidateProfile")
        != (TRACE_PROFILE if mode == "candidate" else None)
    ):
        raise ValueError("dummy-node strict process ancestry/exit/reap differs")
    profile = Path(str(event.get("browserProfilePath", "")))
    if (
        not profile.is_absolute()
        or profile.exists()
        or profile.is_symlink()
        or (mode == "candidate" and not profile.name.startswith("pwright-node-"))
        or not isinstance(profile_stat, dict)
        or set(profile_stat) != {"device", "inode", "uid"}
        or not all(
            type(profile_stat[key]) is int and profile_stat[key] > 0
            for key in ("device", "inode", "uid")
        )
        or profile_stat["uid"] != os.getuid()
        or not isinstance(arguments, list)
        or not all(isinstance(argument, str) for argument in arguments)
        or arguments.count(f"--user-data-dir={profile}") != 1
    ):
        raise ValueError("dummy-node strict owned profile was not released")
    return event


def sandbox_profile(allowed: list[Path]) -> str:
    if len(allowed) != len(set(allowed)) or not all(
        path.is_absolute() for path in allowed
    ):
        raise ValueError("dummy-node sandbox executable allowlist is invalid")
    clauses = [
        "(version 1)",
        "(allow default)",
        "(deny process-exec)",
        "(deny network-outbound)",
    ]
    clauses.extend(
        f"(allow process-exec (literal {json.dumps(str(path))}))" for path in allowed
    )
    return "\n".join(clauses)


def probe_sandbox(profile: str) -> None:
    probe = (
        "import errno, socket, subprocess\n"
        "try:\n"
        "    subprocess.run(['/usr/bin/true'], check=False)\n"
        "except PermissionError:\n"
        "    pass\n"
        "else:\n"
        "    raise SystemExit('exec-deny policy is ineffective')\n"
        "with socket.socket() as connection:\n"
        "    if connection.connect_ex(('127.0.0.1', 9)) != errno.EPERM:\n"
        "        raise SystemExit('network-deny policy is ineffective')\n"
    )
    completed = subprocess.run(
        ["/usr/bin/sandbox-exec", "-p", profile, "/usr/bin/python3", "-c", probe],
        check=False,
        capture_output=True,
        text=True,
    )
    if completed.returncode:
        raise ValueError(
            f"dummy-node sandbox preflight failed: {completed.stderr or completed.stdout}"
        )


def run(args: argparse.Namespace) -> None:
    upstream = args.upstream.resolve()
    strict = bool(args.strict)
    selected_manifest = checked_manifest_path(
        args.manifest or (STRICT_MANIFEST if strict else MANIFEST)
    )
    if strict != (selected_manifest == STRICT_MANIFEST):
        raise ValueError(
            "dummy-node strict runner requires only its dedicated manifest"
        )
    candidate_profile, source_sha, artifact_sha = candidate_policy(selected_manifest)
    env = os.environ.copy()
    preflight_environment(env)
    node_command = shutil.which("node", path="/opt/homebrew/bin:/usr/bin:/bin")
    if node_command is None:
        raise ValueError("dummy-node lane requires system Node")
    node = Path(node_command).resolve()
    if not node.is_file() or node.is_symlink():
        raise ValueError("dummy-node system Node is invalid")
    guard = ROOT / PROCESS_GUARD
    if guard.is_symlink() or not guard.is_file() or not os.access(guard, os.X_OK):
        raise ValueError("dummy-node process guard is missing or linked")
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
    sandbox_policy = sandbox_profile(allowed)
    probe_sandbox(sandbox_policy)
    manifest = pinned_manifest(upstream, selected_manifest)
    selected_manifest_sha256 = digest(selected_manifest)
    pinned_checkout(upstream)
    result = prepare_result_destination(args.result)
    receipt = result.with_suffix(".launch-receipt.jsonl")
    process_receipt = result.with_suffix(".process-receipt.jsonl")
    log_path = result.with_suffix(".log") if strict else None
    strict_path = result.with_suffix(".strict.json") if strict else None
    for target in (receipt, process_receipt, log_path, strict_path):
        if target is None:
            continue
        if target.is_symlink() or target.exists():
            raise ValueError(f"dummy-node receipt destination already exists: {target}")
    runner = upstream / "packages/playwright/cli.js"
    if not runner.is_file() or runner.is_symlink():
        raise ValueError("dummy-node pinned runner is missing or linked")
    candidate = args.candidate.resolve() if args.mode == "candidate" else None
    artifact = (
        candidate / f"native/{candidate_profile}/pwright.node" if candidate else None
    )
    if candidate and (
        source_digest(candidate) != source_sha
        or artifact is None
        or artifact.parent.is_symlink()
        or artifact.parent.parent.is_symlink()
        or artifact.is_symlink()
        or not artifact.is_file()
        or digest(artifact) != artifact_sha
    ):
        raise ValueError("dummy-node candidate source or owned-launch artifact differs")
    upstream_artifacts = (
        {
            relative: directory_digest(upstream / relative)
            for relative in UPSTREAM_ARTIFACT_DIRS
        }
        if strict
        else None
    )
    node_sha256 = digest(node) if strict else None
    local_digests = {
        relative: digest(ROOT / relative)
        for relative in (STRICT_SOURCES if strict else LOCAL_SOURCES)
    }
    bind, host = _tailscale_runtime(env)
    result.parent.mkdir(parents=True, exist_ok=True)
    for target in (receipt, process_receipt):
        target.open("x").close()
    env.pop("CI", None)
    env["PWTEST_MODE"] = "default"
    env["PWRIGHT_OFFICIAL_SERVER_IP"] = bind
    env["PWRIGHT_OFFICIAL_SERVER_HOST"] = host
    env["PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM"] = str(upstream)
    env["PWRIGHT_DUMMY_LAUNCH_MODE"] = args.mode
    env["PWRIGHT_DUMMY_LAUNCH_MANIFEST"] = str(selected_manifest)
    env["PWRIGHT_DUMMY_LAUNCH_MANIFEST_SHA256"] = selected_manifest_sha256
    env["PWRIGHT_DUMMY_LAUNCH_RECEIPT"] = str(receipt)
    env["PWRIGHT_DUMMY_PROCESS_ASSET"] = str(upstream / ASSET)
    env["PWRIGHT_DUMMY_PROCESS_RECEIPT"] = str(process_receipt)
    env["PWRIGHT_DUMMY_REAL_NODE"] = str(node)
    run_id = uuid.uuid4().hex if strict else None
    if run_id:
        env["PWRIGHT_DUMMY_STRICT_RUN_ID"] = run_id
        env["PWRIGHT_DUMMY_HARNESS_PID"] = str(os.getpid())
    env["PATH"] = f"{guard.parent}:/opt/homebrew/bin:/usr/bin:/bin"
    env["PLAYWRIGHT_JSON_OUTPUT_FILE"] = str(result)
    preloads = [SERVER_PRELOAD]
    if candidate:
        env["PWRIGHT_NODE_CANDIDATE"] = str(candidate)
        env["PWRIGHT_NATIVE_PROFILE"] = candidate_profile
        preloads.append(CANDIDATE_PRELOAD)
    else:
        env.pop("PWRIGHT_NODE_CANDIDATE", None)
        env.pop("PWRIGHT_NATIVE_PROFILE", None)
    preloads.append(GUARD_PRELOAD)
    env["NODE_OPTIONS"] = " ".join(
        f"--require={ROOT / relative}" for relative in preloads
    )
    selected_command = command(upstream, node)
    selected_command = [
        "/usr/bin/sandbox-exec",
        "-p",
        sandbox_policy,
        *selected_command,
    ]
    if log_path:
        with log_path.open("x") as log:
            launched = subprocess.Popen(
                selected_command,
                cwd=upstream,
                env=env,
                stdout=log,
                stderr=subprocess.STDOUT,
            )
            completed = subprocess.CompletedProcess(
                selected_command, launched.wait(timeout=120)
            )
            official_cli_pid = launched.pid
    else:
        completed = subprocess.run(
            selected_command, cwd=upstream, env=env, check=False, timeout=120
        )
    if not result.is_file() or result.is_symlink():
        raise ValueError("dummy-node runner did not produce a regular JSON result")
    selected_test = strict_report_case(result) if strict else report_case(result)
    process_event = None
    if strict:
        assert run_id is not None
        process_event = strict_process_receipt(
            process_receipt,
            run_id=run_id,
            mode=args.mode,
            executable=upstream / ASSET,
            node=node,
        )
        if not any(
            entry.get("pid") == official_cli_pid
            and entry.get("executable") == str(node)
            for entry in process_event["ancestry"]
        ):
            raise ValueError(
                "dummy-node strict ancestry excludes selected official CLI PID"
            )
        strict_launch_receipt(
            receipt,
            mode=args.mode,
            run_id=run_id,
            manifest_sha256=selected_manifest_sha256,
            executable=upstream / ASSET,
            process_event=process_event,
            artifact_sha256=artifact_sha if candidate else None,
        )
    else:
        receipt_events(
            receipt,
            mode=args.mode,
            manifest_sha256=selected_manifest_sha256,
            executable=upstream / ASSET,
        )
        process_receipt_event(process_receipt, executable=upstream / ASSET)
    if (
        digest(selected_manifest) != selected_manifest_sha256
        or pinned_manifest(upstream, selected_manifest) != manifest
    ):
        raise ValueError("dummy-node manifest or upstream sources changed during run")
    pinned_checkout(upstream)
    if {
        relative: digest(ROOT / relative) for relative in local_digests
    } != local_digests:
        raise ValueError("dummy-node local harness changed during run")
    if candidate and (
        source_digest(candidate) != source_sha
        or artifact is None
        or digest(artifact) != artifact_sha
    ):
        raise ValueError("dummy-node candidate changed during run")
    if strict and (
        {
            relative: directory_digest(upstream / relative)
            for relative in UPSTREAM_ARTIFACT_DIRS
        }
        != upstream_artifacts
        or digest(node) != node_sha256
    ):
        raise ValueError("dummy-node official runtime artifacts changed during run")
    if completed.returncode == 0 and selected_test.get("status") != "expected":
        raise ValueError(
            "dummy-node runner exited successfully without expected test status"
        )
    if strict and completed.returncode != 0:
        raise ValueError(f"dummy-node strict official test failed; inspect {log_path}")
    provenance = {
        "schema": 2 if strict else 1,
        "evidenceScope": (
            "fixture-adapted-owned-public-launch-strict"
            if strict
            else "diagnostic-public-launch-and-dummy-process-no-profile-attestation"
        ),
        "mode": args.mode,
        "executionPolicy": STRICT_POLICY if strict else POLICY,
        "manifest": str(selected_manifest),
        "manifestSha256": selected_manifest_sha256,
        "commit": COMMIT,
        "suite": SUITE,
        "selector": SELECTOR,
        "specId": SPEC_ID,
        "upstream": str(upstream),
        "candidate": str(candidate) if candidate else None,
        "candidateProfile": candidate_profile if candidate else None,
        "candidateSourceSha256": source_sha if candidate else None,
        "candidateArtifactSha256": artifact_sha if candidate else None,
        "localSources": local_digests,
        "sandboxProfileSha256": hashlib.sha256(sandbox_policy.encode()).hexdigest(),
        "sandboxAllowedExecutables": [str(path) for path in allowed],
        "sandboxNetworkOutbound": "denied",
        "command": selected_command,
        "exitCode": completed.returncode,
        "testStatus": selected_test.get("status"),
        "result": str(result),
        "resultSha256": digest(result),
        "receipt": str(receipt),
        "receiptSha256": digest(receipt),
        "processReceipt": str(process_receipt),
        "processReceiptSha256": digest(process_receipt),
    }
    if strict:
        provenance.update(
            {
                "runId": run_id,
                "officialCliPid": official_cli_pid,
                "upstreamSources": UPSTREAM_SOURCES,
                "upstreamArtifacts": upstream_artifacts,
                "nodeSha256": node_sha256,
                "log": str(log_path),
                "logSha256": digest(log_path),
                "fixtureAdapted": True,
                "unadaptedOfficialPass": False,
            }
        )
    sidecar = result.with_suffix(".provenance.json")
    with sidecar.open("x") as stream:
        json.dump(provenance, stream, indent=2, sort_keys=True)
        stream.write("\n")
    if strict:
        assert strict_path is not None and process_event is not None
        proof = {
            "publicMethod": "chromium.launch",
            "guardPid": process_event["guardPid"],
            "launcherPid": process_event["guardParentPid"],
            "childPid": process_event["childPid"],
            "cliPid": official_cli_pid,
            "harnessPid": process_event["harnessPid"],
            "browserProfilePath": process_event["browserProfilePath"],
            "browserProfileRemoved": True,
            "childReaped": True,
        }
        strict_payload = {
            "schema": 2,
            "scope": "core-dummy-node-launch-strict-v1",
            "mode": args.mode,
            "runId": run_id,
            "officialCliPid": official_cli_pid,
            "suite": SUITE,
            "specId": SPEC_ID,
            "selector": SELECTOR,
            "upstreamCommit": COMMIT,
            "upstreamSources": UPSTREAM_SOURCES,
            "upstreamArtifacts": upstream_artifacts,
            "nodeSha256": node_sha256,
            "manifest": str(selected_manifest),
            "manifestSha256": selected_manifest_sha256,
            "candidateProfile": candidate_profile if candidate else None,
            "candidateSourceSha256": source_sha if candidate else None,
            "candidateArtifactSha256": artifact_sha if candidate else None,
            "localSources": local_digests,
            "result": str(result),
            "resultSha256": digest(result),
            "provenanceSha256": digest(sidecar),
            "launchReceiptSha256": digest(receipt),
            "processReceiptSha256": digest(process_receipt),
            "logSha256": digest(log_path),
            "testStatus": "expected",
            "exitCode": completed.returncode,
            "processOwnershipProof": proof,
            "fixtureAdapted": True,
            "unadaptedOfficialPass": False,
            "upstreamAssertionsModified": False,
            "fixtureAdaptationEligible": True,
            "publicApiParityEvidence": True,
            "diagnosticOnly": False,
            "ledgerEligible": True,
        }
        with strict_path.open("x") as stream:
            json.dump(strict_payload, stream, indent=2, sort_keys=True)
            stream.write("\n")
        from verify_playwright_core_dummy_node_launch_strict_v1 import verify_strict

        verify_strict(strict_path)
    print(
        f"dummy-node Core {'strict' if strict else 'diagnostic'} result={result} provenance={sidecar}",
        flush=True,
    )
    raise SystemExit(completed.returncode)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="mode", required=True)
    for mode in ("candidate", "oracle"):
        selected = commands.add_parser(mode)
        selected.add_argument("upstream", type=Path)
        selected.add_argument("--result", type=Path, required=True)
        selected.add_argument("--manifest", type=Path)
        selected.add_argument("--strict", action="store_true")
        if mode == "candidate":
            selected.add_argument(
                "--candidate", type=Path, default=ROOT / "packages/pwright"
            )
    args = parser.parse_args()
    try:
        run(args)
    except (
        OSError,
        ValueError,
        subprocess.CalledProcessError,
        subprocess.TimeoutExpired,
        json.JSONDecodeError,
    ) as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
