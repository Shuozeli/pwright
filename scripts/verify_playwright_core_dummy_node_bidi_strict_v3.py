#!/usr/bin/env python3
"""Independently verify six paired, browserless official BiDi dummy launches."""

from __future__ import annotations

import argparse
import hashlib
import ipaddress
import json
import os
import re
import subprocess
from pathlib import Path
from typing import Any, cast

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = (
    ROOT / "tests/official/playwright-core-dummy-node-bidi-strict-v3-manifest.json"
)
MANIFEST_SHA256 = "045271d513fbe7844e4389beabc4e753a5b34cbf1ad1d79a9f1641e425e9d767"
TASK_RESULTS = (
    ROOT / ".work/official-conformance/results/core-dummy-node-bidi-strict-v3"
)
V9 = (
    ROOT
    / ".work/official-conformance/outcomes"
    / "core-dummy-node-bidi-strict-paired-v9"
)
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
SELECTOR = "tests/library/browsertype-launch.spec.ts:64"
TITLE = "should reject if launched browser fails immediately"
ASSET = "tests/assets/dummy_bad_browser_executable.js"
PROFILE = "bidi-startup-failure"
V9_EVIDENCE_SCOPE = (
    "official-browser-free-browsertype-launch-dummy-node-bidi-config-outcomes-v9"
)
SOURCE_SHA256 = "fefa89f15b0dd2c75b8485d20dcbb181866330a5099595c3405537f16cc3674e"
ARTIFACT_SHA256 = "709c9b9e263d84d3fcdaf9240b2ba27033c0c5b873533a45788b989710455adb"
ASSET_SHA256 = "9928a78e7de42f29dfc61783a2f31673b387d7d6ea27cda5bc4472e4338c1f7f"
CASES = (
    (6110, "bidi-chrome-canary-library", "chromium", "53abd42f8ece2b451e5f"),
    (6111, "bidi-chrome-stable-library", "chromium", "ee1b24a2e94e134f0b8a"),
    (6112, "bidi-chromium-library", "chromium", "f708c432fe2c80d2440a"),
    (6113, "moz-firefox-beta-library", "firefox", "a1c0f40a45acfee26558"),
    (6114, "moz-firefox-library", "firefox", "6ecf3d1ccae0b454c888"),
    (6115, "moz-firefox-nightly-library", "firefox", "88b0839d68b9d44cb588"),
)
CASE_MAP = {
    project: {
        "index": index,
        "project": project,
        "engine": engine,
        "specId": f"d1a604bdf6287c6776f3-{suffix}",
    }
    for index, project, engine, suffix in CASES
}
LOCAL_SOURCES = {
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
}
STRICT_FIELDS = {
    "schema",
    "scope",
    "mode",
    "project",
    "engine",
    "index",
    "specId",
    "selector",
    "upstreamCommit",
    "upstream",
    "upstreamOutputDir",
    "upstreamOutputSnapshot",
    "upstreamSources",
    "upstreamArtifactsSha256",
    "manifest",
    "manifestSha256",
    "candidate",
    "candidateProfile",
    "candidateSourceSha256",
    "candidateArtifactSha256",
    "node",
    "nodeSha256",
    "localSourcesSha256",
    "sandboxProfileSha256",
    "sandboxAllowedExecutables",
    "sandboxNetworkOutbound",
    "tailscaleBind",
    "tailscaleHost",
    "command",
    "outputDir",
    "outputDirWasAbsentBefore",
    "processGroupId",
    "processGroupIsolated",
    "processGroupEmptyAfter",
    "fixturePorts",
    "fixturePortsFreeBefore",
    "fixturePortsFreeAfter",
    "runId",
    "harnessPid",
    "officialCliPid",
    "result",
    "resultSha256",
    "launchReceipt",
    "launchReceiptSha256",
    "profileReceipt",
    "profileReceiptSha256",
    "processReceipt",
    "processReceiptSha256",
    "log",
    "logSha256",
    "testStatus",
    "attemptStatus",
    "exitCode",
    "assertionsExecuted",
    "upstreamAssertionsModified",
    "fixtureAdapted",
    "selectedExecutable",
    "browserExecutionEvidence",
    "candidatePublicMethodAttested",
    "candidateNativeArtifactPinned",
    "dummyProcessExitObserved",
    "publicApiParityEvidence",
}
HEX32 = re.compile(r"^[0-9a-f]{32}$")
HEX64 = re.compile(r"^[0-9a-f]{64}$")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def object_at(path: Path, label: str) -> dict[str, Any]:
    require(path.is_file() and not path.is_symlink(), f"{label} missing or linked")
    value = json.loads(path.read_text())
    require(isinstance(value, dict), f"{label} is not an object")
    return cast(dict[str, Any], value)


def json_lines(path: Path, label: str) -> list[dict[str, Any]]:
    require(path.is_file() and not path.is_symlink(), f"{label} missing or linked")
    rows = [json.loads(line) for line in path.read_text().splitlines() if line]
    require(
        all(isinstance(row, dict) for row in rows), f"{label} has a non-object event"
    )
    return cast(list[dict[str, Any]], rows)


def hashed(path: Path, sha: object, label: str) -> None:
    require(
        isinstance(sha, str)
        and bool(HEX64.fullmatch(sha))
        and path.is_file()
        and not path.is_symlink()
        and digest(path) == sha,
        f"{label} SHA-256 differs",
    )


def directory_snapshot(directory: Path) -> list[dict[str, str]]:
    require(
        directory.is_dir() and not directory.is_symlink(),
        "protected output directory missing or linked",
    )
    entries = []
    for path in sorted(directory.rglob("*")):
        require(not path.is_symlink(), "protected output directory contains a link")
        relative = path.relative_to(directory).as_posix()
        if path.is_dir():
            entries.append({"path": relative, "kind": "directory"})
        elif path.is_file():
            entries.append({"path": relative, "kind": "file", "sha256": digest(path)})
        else:
            raise ValueError("protected output directory contains a special file")
    return entries


def check_output_contract(strict: dict[str, Any], result: Path, upstream: Path) -> Path:
    """Pure path and safety-flag check for the selected CLI output override."""
    run_id = strict.get("runId")
    require(
        isinstance(run_id, str) and bool(HEX32.fullmatch(run_id)),
        "output run ID differs",
    )
    require(
        result.parent == TASK_RESULTS and result.suffix == ".json",
        "result escaped task-owned results",
    )
    output = TASK_RESULTS / f"{result.stem}-{run_id}.output"
    require(
        strict.get("outputDir") == str(output)
        and strict.get("outputDirWasAbsentBefore") is True
        and strict.get("upstreamOutputDir") == str(upstream / "test-results")
        and strict.get("processGroupId") == strict.get("officialCliPid")
        and strict.get("processGroupIsolated") is True
        and strict.get("processGroupEmptyAfter") is True
        and strict.get("fixturePorts") == [8907, 8908, 8909, 8910]
        and strict.get("fixturePortsFreeBefore") is True
        and strict.get("fixturePortsFreeAfter") is True,
        "output/resource isolation attestation differs",
    )
    return output


def check_tailscale_binding(bind: str, host: str) -> None:
    address = ipaddress.ip_address(bind)
    require(
        address in ipaddress.ip_network("100.64.0.0/10")
        and isinstance(host, str)
        and host.endswith(".tail8f3b66.ts.net"),
        "fixture bind is not Tailscale-only",
    )


def tree_digest(directory: Path, *, package: Path | None = None) -> str:
    require(
        directory.is_dir() and not directory.is_symlink(),
        "source tree missing or linked",
    )
    entries = list(directory.rglob("*"))
    require(
        not any(entry.is_symlink() for entry in entries), "source tree contains link"
    )
    root = package.parent if package else directory
    files = [entry for entry in entries if entry.is_file()]
    if package:
        require(
            package.is_file() and not package.is_symlink(), "package missing or linked"
        )
        files.insert(0, package)
    require(bool(files), "source tree is empty")
    value = hashlib.sha256()
    for path in sorted(files, key=lambda item: item.relative_to(root).as_posix()):
        relative = path.relative_to(root).as_posix().encode()
        value.update(len(relative).to_bytes(8, "big"))
        value.update(relative)
        value.update(path.stat().st_size.to_bytes(8, "big"))
        with path.open("rb") as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                value.update(chunk)
    return value.hexdigest()


def sandbox_profile(paths: list[Path]) -> str:
    require(
        len(paths) == len(set(paths)) and all(path.is_absolute() for path in paths),
        "sandbox allowlist invalid",
    )
    return "\n".join(
        [
            "(version 1)",
            "(allow default)",
            "(deny process-exec)",
            "(deny network-outbound)",
            *(
                f"(allow process-exec (literal {json.dumps(str(path))}))"
                for path in paths
            ),
        ]
    )


def check_report(
    report: dict[str, Any],
    case: dict[str, Any],
    mode: str,
    upstream: Path,
    status: str,
    output_dir: Path,
) -> None:
    """Pure validation of the unchanged official test result."""
    specs: list[dict[str, Any]] = []

    def visit(node: object) -> None:
        require(isinstance(node, dict), "official suite malformed")
        suite = cast(dict[str, Any], node)
        require(
            isinstance(suite.get("specs", []), list)
            and isinstance(suite.get("suites", []), list),
            "official suite children malformed",
        )
        specs.extend(suite.get("specs", []))
        for child in suite.get("suites", []):
            visit(child)

    require(isinstance(report.get("suites"), list), "official report lacks suites")
    for suite in report["suites"]:
        visit(suite)
    require(
        len(specs) == 1 and isinstance(specs[0], dict),
        "official report did not select one spec",
    )
    spec = specs[0]
    passing = status == "expected"
    require(
        status in ("expected", "unexpected") and (mode == "candidate" or passing),
        "official lane outcome differs",
    )
    require(
        spec.get("id") == case["specId"]
        and spec.get("file") == "library/browsertype-launch.spec.ts"
        and spec.get("line") == 64
        and spec.get("column") == 3
        and spec.get("title") == TITLE
        and spec.get("ok") is passing,
        "official spec identity/status differs",
    )
    tests = spec.get("tests")
    require(
        isinstance(tests, list) and len(tests) == 1 and isinstance(tests[0], dict),
        "official test count differs",
    )
    test = tests[0]
    results = test.get("results")
    require(
        test.get("projectId") == case["project"]
        and test.get("expectedStatus") == "passed"
        and test.get("status") == ("expected" if passing else "unexpected")
        and test.get("annotations") == []
        and isinstance(results, list)
        and len(results) == 1
        and isinstance(results[0], dict)
        and results[0].get("status") == ("passed" if passing else "failed")
        and results[0].get("retry") == 0
        and results[0].get("workerIndex") == 0,
        "official attempt did not run exactly once",
    )
    stats = report.get("stats")
    config = report.get("config")
    require(
        isinstance(stats, dict)
        and all(
            stats.get(key) == value
            for key, value in {
                "expected": int(passing),
                "unexpected": int(not passing),
                "skipped": 0,
                "flaky": 0,
            }.items()
        )
        and isinstance(config, dict)
        and config.get("workers") == 1
        and isinstance(config.get("projects"), list)
        and len(
            [
                p
                for p in config["projects"]
                if isinstance(p, dict)
                and p.get("id") == case["project"]
                and p.get("retries") == 0
                and p.get("outputDir") == str(output_dir)
            ]
        )
        == 1
        and report.get("errors") == [],
        "official report stats/project/retries differ",
    )
    errors = results[0].get("errors")
    if passing:
        require(errors == [], "oracle assertion failed")
    else:
        require(
            isinstance(errors, list)
            and len(errors) == 1
            and isinstance(errors[0], dict)
            and errors[0].get("location")
            == {
                "file": str(upstream / "tests/library/browsertype-launch.spec.ts"),
                "line": 72,
                "column": 32,
            }
            and '"Browser logs:"' in str(errors[0].get("message", "")),
            "candidate RED is not the original launch-logs assertion",
        )


def check_launch_events(
    events: list[dict[str, Any]],
    case: dict[str, Any],
    mode: str,
    strict: dict[str, Any],
    manifest: dict[str, Any],
) -> int:
    """Pure validation that public BrowserType.launch was entered and rejected once."""
    require(
        len(events) == 2
        and [item.get("event") for item in events]
        == ["launch-invoked", "launch-failed"],
        "public launch event sequence differs",
    )
    upstream = Path(strict["upstream"])
    expected = {
        "mode": mode,
        "project": case["project"],
        "engine": case["engine"],
        "runId": strict["runId"],
        "publicMethod": f"{case['engine']}.launch",
        "executablePath": str(upstream / ASSET),
        "assetSha256": ASSET_SHA256,
        "manifestSha256": MANIFEST_SHA256,
        "candidateProfile": PROFILE if mode == "candidate" else None,
        "candidateArtifactSha256": manifest["candidateArtifactSha256"]
        if mode == "candidate"
        else None,
        "candidatePublicLaunchInherited": True if mode == "candidate" else None,
    }
    for event in events:
        require(
            set(event) == {"event", "workerPid", "workerParentPid", *expected}
            and all(event.get(key) == value for key, value in expected.items())
            and type(event.get("workerPid")) is int
            and event["workerPid"] > 1
            and event.get("workerParentPid") == strict["officialCliPid"],
            "public launch receipt identity differs",
        )
    require(
        events[0]["workerPid"] == events[1]["workerPid"], "public launch worker changed"
    )
    return cast(int, events[0]["workerPid"])


def check_process_events(
    profiles: list[dict[str, Any]],
    processes: list[dict[str, Any]],
    case: dict[str, Any],
    mode: str,
    strict: dict[str, Any],
    worker_pid: int,
    *,
    profile_removed: bool = True,
) -> None:
    """Pure receipt validation; caller supplies observed profile-removal state."""
    engine = case["engine"]
    require(len(processes) == 1, "lane lacks exactly one exit-1 dummy process")
    require(
        len(profiles) == (0 if mode == "candidate" else 1),
        "lane profile receipt count differs",
    )
    process = processes[0]
    chain = process.get("ancestry")
    child = process.get("childSnapshot")
    require(
        process.get("event") == "dummy-node-exited"
        and process.get("runId") == strict["runId"]
        and process.get("mode") == mode
        and process.get("asset") == str(Path(strict["upstream"]) / ASSET)
        and process.get("exitCode") == 1
        and process.get("childReaped") is True
        and process.get("guardParentPid") == worker_pid
        and process.get("harnessPid") == strict["harnessPid"]
        and isinstance(chain, list)
        and len(chain) >= 3
        and all(
            isinstance(item, dict) and set(item) == {"pid", "ppid", "uid", "executable"}
            for item in chain
        )
        and chain[0]["pid"] == process.get("guardPid")
        and chain[0]["ppid"] == worker_pid
        and chain[-1]["pid"] == strict["harnessPid"]
        and all(chain[i]["ppid"] == chain[i + 1]["pid"] for i in range(len(chain) - 1))
        and any(
            item["pid"] == strict["officialCliPid"]
            and item["executable"] == strict["node"]
            for item in chain
        )
        and isinstance(child, dict)
        and set(child) == {"pid", "ppid", "uid", "executable"}
        and child["pid"] == process.get("childPid")
        and child["ppid"] == process.get("guardPid")
        and child["uid"] == os.getuid()
        and child["executable"] == strict["node"],
        "dummy process exit/ancestry differs",
    )
    args = process.get("browserArguments")
    require(
        isinstance(args, list) and all(isinstance(arg, str) for arg in args),
        "dummy launch arguments malformed",
    )
    if engine == "firefox":
        require(
            set(process)
            == {
                "event",
                "mode",
                "project",
                "engine",
                "runId",
                "asset",
                "exitCode",
                "guardPid",
                "guardParentPid",
                "childPid",
                "childReaped",
                "childSnapshot",
                "ancestry",
                "harnessPid",
                "browserArguments",
                "browserProfile",
            }
            and process["engine"] == engine
            and process["project"] == case["project"],
            "Firefox guard schema/engine differs",
        )
        profile_data = process["browserProfile"]
        require(
            isinstance(profile_data, dict)
            and set(profile_data) == {"path", "device", "inode", "uid"},
            "Firefox guard profile missing",
        )
        profile_path = Path(profile_data["path"])
        require(
            profile_path.is_absolute()
            and profile_removed
            and all(
                type(profile_data[key]) is int and profile_data[key] > 0
                for key in ("device", "inode", "uid")
            )
            and profile_data["uid"] == os.getuid()
            and args.count("--profile") == 1
            and args.index("--profile") + 1 < len(args)
            and args[args.index("--profile") + 1] == str(profile_path)
            and not any(
                arg == "-profile"
                or arg.startswith(("-profile=", "--profile=", "--user-data-dir"))
                for arg in args
            ),
            "Firefox dummy profile creation/ownership differs",
        )
        require(
            profile_path.name.startswith(
                "pwright-node-bidi-startup-"
                if mode == "candidate"
                else "playwright_firefoxdev_profile-"
            ),
            "Firefox profile was not owned by the selected lane",
        )
        if mode == "oracle":
            profile = profiles[0]
            require(
                set(profile)
                == {
                    "event",
                    "mode",
                    "project",
                    "engine",
                    "runId",
                    "workerPid",
                    "path",
                    "device",
                    "inode",
                    "uid",
                }
                and profile["event"] == "profile-created"
                and profile["mode"] == mode
                and profile["project"] == case["project"]
                and profile["engine"] == engine
                and profile["runId"] == strict["runId"]
                and profile["workerPid"] == worker_pid
                and profile_path.name.startswith("playwright_firefoxdev_profile-")
                and all(
                    profile[key] == profile_data[key]
                    for key in ("path", "device", "inode", "uid")
                ),
                "Firefox oracle profile receipt differs",
            )
    else:
        require(
            set(process)
            == {
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
            },
            "Chromium guard schema differs",
        )
        require(
            process["candidateProfile"] == (PROFILE if mode == "candidate" else None),
            "Chromium guard candidate profile differs",
        )
        profile_path = Path(process["browserProfilePath"])
        stat = process["browserProfileStat"]
        require(
            profile_path.is_absolute()
            and profile_removed
            and isinstance(stat, dict)
            and set(stat) == {"device", "inode", "uid"}
            and all(type(stat[key]) is int and stat[key] > 0 for key in stat)
            and stat["uid"] == os.getuid()
            and len([arg for arg in args if arg.startswith("--user-data-dir=")]) == 1
            and f"--user-data-dir={profile_path}" in args,
            "Chromium dummy launch profile ownership differs",
        )
        if mode == "oracle":
            profile = profiles[0]
            require(
                set(profile)
                == {
                    "event",
                    "mode",
                    "project",
                    "engine",
                    "runId",
                    "workerPid",
                    "path",
                    "device",
                    "inode",
                    "uid",
                }
                and profile["event"] == "profile-created"
                and profile["mode"] == mode
                and profile["project"] == case["project"]
                and profile["engine"] == engine
                and profile["runId"] == strict["runId"]
                and profile["workerPid"] == worker_pid
                and profile["path"] == str(profile_path)
                and all(profile[key] == stat[key] for key in stat)
                and profile_path.name.startswith("playwright_chromiumdev_profile-"),
                "Chromium oracle profile receipt differs",
            )
        else:
            require(
                profile_path.name.startswith("pwright-node-bidi-startup-"),
                "candidate Chromium profile was not Pwright-owned",
            )


def checked_manifest() -> dict[str, Any]:
    hashed(MANIFEST, MANIFEST_SHA256, "manifest")
    manifest = object_at(MANIFEST, "manifest")
    require(
        set(manifest)
        == {
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
        and manifest["schema"] == 1
        and manifest["repository"] == "https://github.com/microsoft/playwright.git"
        and manifest["tag"] == "v1.58.0"
        and manifest["commit"] == COMMIT
        and manifest["executionPolicy"]
        == "pinned-bidi-dummy-node-public-launch-strict-v3"
        and manifest["selector"] == SELECTOR
        and manifest["title"] == TITLE
        and manifest["asset"] == ASSET
        and manifest["config"] == "tests/bidi/playwright.config.ts"
        and manifest["candidateProfile"] == PROFILE
        and manifest["candidateSourceSha256"] == SOURCE_SHA256
        and manifest["candidateArtifactSha256"] == ARTIFACT_SHA256
        and manifest["cases"] == list(CASE_MAP.values()),
        "manifest identity/cases differ",
    )
    require(
        isinstance(manifest["upstreamSources"], dict)
        and manifest["upstreamSources"].get(ASSET) == ASSET_SHA256,
        "upstream source map differs",
    )
    for name, key in (
        ("candidate", "v9CandidateLedgerSha256"),
        ("oracle", "v9OracleLedgerSha256"),
    ):
        ledger = V9 / f"{name}.jsonl"
        hashed(ledger, manifest[key], f"v9 {name} predecessor")
        rows = ledger.read_text().splitlines()
        require(len(rows) == 48_477, "v9 predecessor row count differs")
        for case in CASE_MAP.values():
            row = json.loads(rows[case["index"]])
            require(
                all(
                    row.get(k) == v
                    for k, v in {
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
                ),
                f"v9 {name} case {case['index']} differs",
            )
    hashed(V9 / "promotion-receipt.json", manifest["v9ReceiptSha256"], "v9 receipt")
    return manifest


def verify_one(strict_path: Path) -> dict[str, Any]:
    strict_path = strict_path.parent.resolve() / strict_path.name
    strict = object_at(strict_path, "strict sidecar")
    require(set(strict) == STRICT_FIELDS, "strict sidecar fields differ")
    manifest = checked_manifest()
    case = CASE_MAP.get(strict.get("project"))
    mode = strict.get("mode")
    require(
        case is not None and mode in ("candidate", "oracle"),
        "strict lane identity differs",
    )
    candidate = mode == "candidate"
    require(
        strict["schema"] == 1
        and strict["scope"] == "core-dummy-node-bidi-strict-v3"
        and strict["project"] == case["project"]
        and strict["engine"] == case["engine"]
        and strict["index"] == case["index"]
        and strict["specId"] == case["specId"]
        and strict["selector"] == SELECTOR
        and strict["upstreamCommit"] == COMMIT
        and isinstance(strict["runId"], str)
        and bool(HEX32.fullmatch(strict["runId"]))
        and type(strict["officialCliPid"]) is int
        and strict["officialCliPid"] > 1
        and type(strict["harnessPid"]) is int
        and strict["harnessPid"] > 1
        and strict["testStatus"]
        in (("expected", "unexpected") if candidate else ("expected",))
        and strict["attemptStatus"]
        == ("passed" if strict["testStatus"] == "expected" else "failed")
        and strict["exitCode"] == (0 if strict["testStatus"] == "expected" else 1)
        and strict["assertionsExecuted"] is True
        and strict["upstreamAssertionsModified"] is False
        and strict["fixtureAdapted"] is True
        and strict["selectedExecutable"] == "pinned-exit-1-node-asset"
        and strict["browserExecutionEvidence"] is False
        and strict["candidatePublicMethodAttested"] is candidate
        and strict["candidateNativeArtifactPinned"] is candidate
        and strict["publicApiParityEvidence"] is False,
        "strict lane status/case/attestation differs",
    )
    require(
        strict["manifest"] == str(MANIFEST)
        and strict["manifestSha256"] == MANIFEST_SHA256,
        "strict manifest binding differs",
    )
    upstream = Path(strict["upstream"])
    require(
        upstream.is_absolute() and upstream.is_dir() and not upstream.is_symlink(),
        "upstream checkout path differs",
    )
    for args, expected in (
        (["rev-parse", "HEAD"], COMMIT),
        (["status", "--porcelain", "--untracked-files=no"], ""),
    ):
        completed = subprocess.run(
            ["git", "-C", str(upstream), *args],
            check=True,
            capture_output=True,
            text=True,
        )
        require(
            completed.stdout.strip() == expected,
            "upstream revision/cleanliness differs",
        )
    require(
        not (upstream / ".env").exists() and not (upstream / ".env").is_symlink(),
        "upstream .env override appeared",
    )
    result = Path(strict["result"])
    require(
        result.is_absolute() and strict_path == result.with_suffix(".strict.json"),
        "strict/result path relation differs",
    )
    output_dir = check_output_contract(strict, result, upstream)
    require(
        output_dir.is_dir() and not output_dir.is_symlink(),
        "task-owned output directory missing or linked",
    )
    require(
        strict["upstreamOutputSnapshot"]
        == directory_snapshot(upstream / "test-results"),
        "pre-existing upstream test-results changed",
    )
    sources = manifest["upstreamSources"]
    require(strict["upstreamSources"] == sources, "upstream source map differs")
    for relative, sha in sources.items():
        hashed(upstream / relative, sha, relative)
    require(
        os.access(upstream / ASSET, os.X_OK),
        "pinned dummy Node asset is not executable",
    )
    artifacts = strict["upstreamArtifactsSha256"]
    require(
        isinstance(artifacts, dict)
        and set(artifacts)
        == {"packages/playwright-core/lib", "packages/playwright/lib"},
        "upstream build set differs",
    )
    for relative, sha in artifacts.items():
        require(
            tree_digest(upstream / relative) == sha,
            f"upstream build {relative} drifted",
        )
    local = strict["localSourcesSha256"]
    require(
        isinstance(local, dict) and set(local) == LOCAL_SOURCES,
        "local tooling source set differs",
    )
    for relative, sha in local.items():
        hashed(ROOT / relative, sha, relative)
    candidate_path = ROOT / "packages/pwright"
    if candidate:
        require(
            strict["candidate"] == str(candidate_path)
            and strict["candidateProfile"] == PROFILE
            and strict["candidateSourceSha256"] == manifest["candidateSourceSha256"]
            and strict["candidateArtifactSha256"] == manifest["candidateArtifactSha256"]
            and tree_digest(
                candidate_path / "src", package=candidate_path / "package.json"
            )
            == manifest["candidateSourceSha256"],
            "candidate frozen source/profile differs",
        )
        hashed(
            candidate_path / f"native/{PROFILE}/pwright.node",
            strict["candidateArtifactSha256"],
            "candidate native artifact",
        )
    else:
        require(
            all(
                strict[key] is None
                for key in (
                    "candidate",
                    "candidateProfile",
                    "candidateSourceSha256",
                    "candidateArtifactSha256",
                )
            ),
            "oracle contains candidate runtime",
        )
    node = Path(strict["node"])
    require(node.is_absolute() and node.resolve() == node, "system Node path differs")
    hashed(node, strict["nodeSha256"], "system Node")
    guard = ROOT / (
        "tests/official/dummy-core-guard/node"
        if case["engine"] == "chromium"
        else "tests/official/dummy-core-bidi-firefox-guard/node"
    )
    allowed = strict["sandboxAllowedExecutables"]
    require(
        allowed
        == [
            str(node),
            str(upstream / ASSET),
            str(guard),
            "/usr/bin/env",
            "/usr/bin/python3",
            "/Library/Developer/CommandLineTools/usr/bin/python3",
            "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/bin/python3.9",
            "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/Resources/Python.app/Contents/MacOS/Python",
        ],
        "sandbox executable allowlist differs",
    )
    sandbox = sandbox_profile([Path(item) for item in allowed])
    require(
        strict["sandboxProfileSha256"] == hashlib.sha256(sandbox.encode()).hexdigest()
        and strict["sandboxNetworkOutbound"] == "denied"
        and strict["command"]
        == [
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
            f"--project={case['project']}",
            SELECTOR,
        ],
        "sandbox policy/official command differs",
    )
    check_tailscale_binding(strict["tailscaleBind"], strict["tailscaleHost"])
    paths = {
        key: result.with_suffix(suffix)
        for key, suffix in {
            "launchReceipt": ".launch-receipt.jsonl",
            "profileReceipt": ".profile-receipt.jsonl",
            "processReceipt": ".process-receipt.jsonl",
            "log": ".log",
        }.items()
    }
    paths["result"] = result
    for key, path in paths.items():
        require(strict[key] == str(path), f"{key} path differs")
        hashed(path, strict[f"{key}Sha256"], key)
    check_report(
        object_at(result, "official report"),
        case,
        mode,
        upstream,
        strict["testStatus"],
        output_dir,
    )
    worker = check_launch_events(
        json_lines(paths["launchReceipt"], "launch receipt"),
        case,
        mode,
        strict,
        manifest,
    )
    profiles = json_lines(paths["profileReceipt"], "profile receipt")
    processes = json_lines(paths["processReceipt"], "process receipt")
    profile_removed = not any(
        Path(item.get("path", "")).exists() or Path(item.get("path", "")).is_symlink()
        for item in profiles
    )
    if processes and case["engine"] == "chromium":
        path = Path(processes[0].get("browserProfilePath", ""))
        profile_removed = (
            profile_removed and not path.exists() and not path.is_symlink()
        )
    if processes and case["engine"] == "firefox":
        profile_data = processes[0].get("browserProfile")
        if isinstance(profile_data, dict):
            path = Path(profile_data.get("path", ""))
            profile_removed = (
                profile_removed and not path.exists() and not path.is_symlink()
            )
    check_process_events(
        profiles, processes, case, mode, strict, worker, profile_removed=profile_removed
    )
    require(
        strict["dummyProcessExitObserved"] is bool(processes),
        "dummy exit attestation differs",
    )
    return strict


def check_pairs(evidence: list[dict[str, Any]]) -> None:
    """Pure check that all 12 lanes are independent and paired to one source."""
    require(len(evidence) == 12, "BiDi dummy evidence requires 12 lanes")
    keyed = {(item["project"], item["mode"]): item for item in evidence}
    require(
        len(keyed) == 12
        and set(keyed)
        == {
            (project, mode) for project in CASE_MAP for mode in ("candidate", "oracle")
        },
        "BiDi dummy paired case coverage differs",
    )
    for field in ("runId", "officialCliPid", "result"):
        require(
            len({item[field] for item in evidence}) == 12,
            f"BiDi dummy {field} executions not isolated",
        )
    for project in CASE_MAP:
        candidate = keyed[(project, "candidate")]
        oracle = keyed[(project, "oracle")]
        require(
            candidate["upstreamArtifactsSha256"] == oracle["upstreamArtifactsSha256"]
            and candidate["nodeSha256"] == oracle["nodeSha256"]
            and candidate["manifestSha256"] == oracle["manifestSha256"]
            and candidate["upstreamSources"] == oracle["upstreamSources"],
            "candidate/oracle lanes are not same-source pairs",
        )


def verify_all(paths: list[Path]) -> None:
    require(len(paths) == 12, "strict BiDi dummy capture requires 12 sidecars")
    check_pairs([verify_one(path) for path in paths])


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("strict_sidecars", type=Path, nargs="+")
    args = parser.parse_args()
    try:
        if len(args.strict_sidecars) == 1:
            item = verify_one(args.strict_sidecars[0])
            print(f"verified strict BiDi dummy {item['project']} {item['mode']}")
        else:
            verify_all(args.strict_sidecars)
            print("verified 12 strict BiDi dummy official lanes")
    except (
        OSError,
        TypeError,
        ValueError,
        KeyError,
        IndexError,
        json.JSONDecodeError,
        subprocess.CalledProcessError,
    ) as error:
        parser.exit(2, f"strict BiDi dummy evidence invalid: {error}\n")


if __name__ == "__main__":
    main()
