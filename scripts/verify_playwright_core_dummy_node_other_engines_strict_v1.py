#!/usr/bin/env python3
"""Independently verify pinned two-engine official dummy-node RED evidence."""

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
    ROOT
    / "tests/official/playwright-core-dummy-node-other-engines-strict-v1-manifest.json"
)
MANIFEST_SHA256 = "6ac83fa5a81dd7806c2f4737d61684b8aad414b2ea006f72e83bcdbc937fa6b0"
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
ASSET = "tests/assets/dummy_bad_browser_executable.js"
SELECTOR = "tests/library/browsertype-launch.spec.ts:64"
TITLE = "should reject if launched browser fails immediately"
SPEC_IDS = {
    "firefox": "d1a604bdf6287c6776f3-d8df50eebf56e23fa8d3",
    "webkit": "d1a604bdf6287c6776f3-24ac4e8612e7cc735325",
}
LOCAL_SOURCES = {
    "scripts/run_playwright_core_dummy_node_other_engines_strict_v1.py",
    "scripts/run_playwright_core_dummy_node_launch.py",
    "scripts/official_result_destination.py",
    "scripts/run_playwright_official_conformance.py",
    "tests/official/playwright-core-dummy-node-other-engines-preload.cjs",
    "tests/official/dummy-core-other-engines-guard/node",
    "tests/official/playwright-core-server-runtime.cjs",
    "tests/official/playwright-core-candidate-preload.cjs",
}
STRICT_FIELDS = {
    "schema",
    "scope",
    "mode",
    "engine",
    "project",
    "suite",
    "specId",
    "selector",
    "upstreamCommit",
    "upstream",
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
    "candidatePublicMethodAttested",
    "candidateNativeArtifactPinned",
    "candidateNativeRuntimeInvoked",
    "publicApiParityEvidence",
    "diagnosticOnly",
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
    require(path.is_file() and not path.is_symlink(), f"{label} is missing or linked")
    value = json.loads(path.read_text())
    require(isinstance(value, dict), f"{label} is not an object")
    return cast(dict[str, Any], value)


def json_lines(path: Path, label: str) -> list[dict[str, Any]]:
    require(path.is_file() and not path.is_symlink(), f"{label} is missing or linked")
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


def source_digest(candidate: Path) -> str:
    package = candidate / "package.json"
    source = candidate / "src"
    require(
        package.is_file()
        and not package.is_symlink()
        and source.is_dir()
        and not source.is_symlink(),
        "candidate package/source missing",
    )
    entries = list(source.rglob("*"))
    require(
        not any(path.is_symlink() for path in entries),
        "candidate source contains a symlink",
    )
    files = [package, *(path for path in entries if path.is_file())]
    value = hashlib.sha256()
    for path in sorted(files, key=lambda item: item.relative_to(candidate).as_posix()):
        relative = path.relative_to(candidate).as_posix().encode()
        value.update(len(relative).to_bytes(8, "big"))
        value.update(relative)
        value.update(path.stat().st_size.to_bytes(8, "big"))
        with path.open("rb") as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                value.update(chunk)
    return value.hexdigest()


def directory_digest(directory: Path) -> str:
    require(
        directory.is_dir() and not directory.is_symlink(),
        "upstream build directory missing/linked",
    )
    entries = list(directory.rglob("*"))
    require(
        not any(entry.is_symlink() for entry in entries),
        "upstream build contains a symlink",
    )
    files = sorted(
        (entry for entry in entries if entry.is_file()),
        key=lambda entry: entry.relative_to(directory).as_posix(),
    )
    require(bool(files), "upstream build directory is empty")
    value = hashlib.sha256()
    for path in files:
        relative = path.relative_to(directory).as_posix().encode()
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


def official_case(
    report: dict[str, Any], engine: str, mode: str, upstream: Path
) -> None:
    specs: list[dict[str, Any]] = []

    def visit(node: object) -> None:
        require(isinstance(node, dict), "official report suite malformed")
        suite = cast(dict[str, Any], node)
        require(
            isinstance(suite.get("specs", []), list)
            and isinstance(suite.get("suites", []), list),
            "official report suite children malformed",
        )
        specs.extend(suite.get("specs", []))
        for child in suite.get("suites", []):
            visit(child)

    require(isinstance(report.get("suites"), list), "official report lacks suites")
    for suite in report["suites"]:
        visit(suite)
    require(len(specs) == 1, "official report must contain exactly one spec")
    spec = specs[0]
    expected = mode == "oracle"
    status = "expected" if expected else "unexpected"
    attempt_status = "passed" if expected else "failed"
    require(
        spec.get("id") == SPEC_IDS[engine]
        and spec.get("file") == "library/browsertype-launch.spec.ts"
        and spec.get("line") == 64
        and spec.get("column") == 3
        and spec.get("title") == TITLE
        and spec.get("ok") is expected,
        "official report spec identity/status differs",
    )
    tests = spec.get("tests")
    require(
        isinstance(tests, list) and len(tests) == 1 and isinstance(tests[0], dict),
        "official report test count differs",
    )
    test = tests[0]
    results = test.get("results")
    require(
        test.get("projectId") == f"{engine}-library"
        and test.get("expectedStatus") == "passed"
        and test.get("status") == status
        and test.get("annotations") == []
        and isinstance(results, list)
        and len(results) == 1
        and isinstance(results[0], dict)
        and results[0].get("status") == attempt_status
        and results[0].get("retry") == 0
        and results[0].get("workerIndex") == 0,
        "official report did not execute the pinned public assertion exactly once",
    )
    stats = report.get("stats")
    config = report.get("config")
    require(
        isinstance(stats, dict)
        and all(
            stats.get(key) == value
            for key, value in {
                "expected": int(expected),
                "unexpected": int(not expected),
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
                and p.get("id") == f"{engine}-library"
                and p.get("retries") == 0
            ]
        )
        == 1,
        "official report stats/project/retries differ",
    )
    errors = results[0].get("errors")
    if expected:
        require(errors == [] and report.get("errors") == [], "oracle recorded an error")
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
            and report.get("errors") == [],
            "candidate RED lacks the pinned upstream assertion location",
        )
        messages = "\n".join(
            str(error.get("message", "")) for error in errors if isinstance(error, dict)
        )
        require(
            '"Browser logs:"' in messages
            and "browserType.launch is not supported by the current Pwright native profile"
            in messages
            and "> 72 |" in messages,
            "candidate RED is not the pinned launch-logs assertion discrepancy",
        )


def verify_one(strict_path: Path) -> dict[str, Any]:
    strict_path = strict_path.resolve()
    strict = object_at(strict_path, "strict sidecar")
    require(set(strict) == STRICT_FIELDS, "strict sidecar fields differ")
    engine = strict.get("engine")
    mode = strict.get("mode")
    require(
        engine in SPEC_IDS and mode in ("candidate", "oracle"),
        "strict lane identity differs",
    )
    expected = mode == "oracle"
    run_id = strict.get("runId")
    require(
        strict["schema"] == 2
        and strict["scope"] == "core-dummy-node-other-engines-strict-v1"
        and strict["project"] == f"{engine}-library"
        and strict["suite"] == f"library-browsertype-launch-dummy-node-failure-{engine}"
        and strict["specId"] == SPEC_IDS[engine]
        and strict["selector"] == SELECTOR
        and strict["upstreamCommit"] == COMMIT
        and isinstance(run_id, str)
        and bool(HEX32.fullmatch(run_id))
        and strict["testStatus"] == ("expected" if expected else "unexpected")
        and strict["attemptStatus"] == ("passed" if expected else "failed")
        and strict["exitCode"] == (0 if expected else 1)
        and strict["assertionsExecuted"] is True
        and strict["upstreamAssertionsModified"] is False
        and strict["fixtureAdapted"] is True
        and strict["candidatePublicMethodAttested"] is (not expected)
        and strict["candidateNativeArtifactPinned"] is (not expected)
        and strict["candidateNativeRuntimeInvoked"] is False
        and strict["publicApiParityEvidence"] is False
        and strict["diagnosticOnly"] is False,
        "strict lane status/case/attestation differs",
    )
    hashed(MANIFEST, strict["manifestSha256"], "manifest")
    require(
        strict["manifest"] == str(MANIFEST)
        and strict["manifestSha256"] == MANIFEST_SHA256,
        "strict manifest identity differs",
    )
    manifest = object_at(MANIFEST, "manifest")
    require(
        manifest.get("commit") == COMMIT
        and manifest.get("selector") == SELECTOR
        and manifest.get("asset") == ASSET
        and manifest.get("executionPolicy")
        == "pinned-dummy-node-other-engines-strict-v1"
        and manifest.get("cases")
        == [
            {
                "engine": e,
                "project": f"{e}-library",
                "suite": f"library-browsertype-launch-dummy-node-failure-{e}",
                "specId": SPEC_IDS[e],
            }
            for e in ("firefox", "webkit")
        ],
        "manifest source selection differs",
    )
    upstream = Path(strict["upstream"])
    require(
        upstream.is_absolute() and upstream.is_dir() and not upstream.is_symlink(),
        "upstream checkout path differs",
    )
    for args, expected_output in (
        (["rev-parse", "HEAD"], COMMIT),
        (["status", "--porcelain", "--untracked-files=no"], ""),
    ):
        value = subprocess.run(
            ["git", "-C", str(upstream), *args],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        require(
            value == expected_output, "upstream checkout revision/cleanliness differs"
        )
    sources = manifest.get("upstreamSources")
    require(
        isinstance(sources, dict)
        and sources.get(ASSET)
        == "9928a78e7de42f29dfc61783a2f31673b387d7d6ea27cda5bc4472e4338c1f7f"
        and strict["upstreamSources"] == sources,
        "upstream source map differs",
    )
    for relative, sha in sources.items():
        hashed(upstream / relative, sha, relative)
    require(not (upstream / ".env").exists(), "upstream .env override appeared")
    artifacts = strict["upstreamArtifactsSha256"]
    require(
        isinstance(artifacts, dict)
        and set(artifacts)
        == {"packages/playwright-core/lib", "packages/playwright/lib"},
        "upstream runtime artifact set differs",
    )
    for relative, sha in artifacts.items():
        require(
            directory_digest(upstream / relative) == sha,
            "upstream runtime artifact drifted",
        )
    local = strict["localSourcesSha256"]
    require(
        isinstance(local, dict) and set(local) == LOCAL_SOURCES,
        "local tooling source set differs",
    )
    for relative, sha in local.items():
        hashed(ROOT / relative, sha, relative)
    candidate = ROOT / "packages/pwright"
    if not expected:
        require(
            strict["candidate"] == str(candidate)
            and strict["candidateProfile"] == "playwright-trace"
            and strict["candidateSourceSha256"] == manifest["candidateSourceSha256"]
            and strict["candidateArtifactSha256"] == manifest["candidateArtifactSha256"]
            and source_digest(candidate) == manifest["candidateSourceSha256"],
            "candidate package/profile/source differs",
        )
        hashed(
            candidate / "native/playwright-trace/pwright.node",
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
            "oracle contains candidate runtime identity",
        )
    node = Path(strict["node"])
    hashed(node, strict["nodeSha256"], "system Node")
    allowed = strict["sandboxAllowedExecutables"]
    require(
        isinstance(allowed, list)
        and allowed
        == [
            str(node),
            str(upstream / ASSET),
            str(ROOT / "tests/official/dummy-core-other-engines-guard/node"),
            "/usr/bin/env",
            "/usr/bin/python3",
            "/Library/Developer/CommandLineTools/usr/bin/python3",
            "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/bin/python3.9",
            "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/Resources/Python.app/Contents/MacOS/Python",
        ],
        "sandbox executable allowlist differs",
    )
    sandbox = sandbox_profile([Path(value) for value in allowed])
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
            "--config=tests/library/playwright.config.ts",
            "--reporter=line,json",
            "--workers=1",
            "--retries=0",
            f"--project={engine}-library",
            SELECTOR,
        ],
        "sandbox policy or official command differs",
    )
    bind = ipaddress.ip_address(strict["tailscaleBind"])
    require(
        bind in ipaddress.ip_network("100.64.0.0/10")
        and isinstance(strict["tailscaleHost"], str)
        and strict["tailscaleHost"].endswith(".tail8f3b66.ts.net"),
        "fixture bind is not a Tailscale address",
    )
    result = Path(strict["result"])
    require(
        result.is_absolute() and strict_path == result.with_suffix(".strict.json"),
        "strict/result path relation differs",
    )
    paths = {
        "result": result,
        "launchReceipt": result.with_suffix(".launch-receipt.jsonl"),
        "profileReceipt": result.with_suffix(".profile-receipt.jsonl"),
        "processReceipt": result.with_suffix(".process-receipt.jsonl"),
        "log": result.with_suffix(".log"),
    }
    for key, path in paths.items():
        require(strict[key] == str(path), f"{key} path differs")
        hashed(path, strict[f"{key}Sha256"], key)
    report = object_at(result, "official report")
    official_case(report, engine, mode, upstream)
    launch = json_lines(paths["launchReceipt"], "launch receipt")
    require(
        len(launch) == 2
        and [event.get("event") for event in launch]
        == ["launch-invoked", "launch-failed"],
        "public launch events differ",
    )
    expected_fields = {
        "mode": mode,
        "engine": engine,
        "runId": run_id,
        "publicMethod": f"{engine}.launch",
        "executablePath": str(upstream / ASSET),
        "assetSha256": sources[ASSET],
        "manifestSha256": MANIFEST_SHA256,
        "candidateProfile": "playwright-trace" if not expected else None,
        "candidateArtifactSha256": manifest["candidateArtifactSha256"]
        if not expected
        else None,
        "candidatePublicLaunchInherited": True if not expected else None,
    }
    for event in launch:
        require(
            set(event) == {"event", "workerPid", "workerParentPid", *expected_fields}
            and all(event[key] == value for key, value in expected_fields.items())
            and type(event["workerPid"]) is int
            and event["workerPid"] > 1
            and event["workerParentPid"] == strict["officialCliPid"],
            "guarded public launch call differs",
        )
    require(
        launch[0]["workerPid"] == launch[1]["workerPid"], "public launch worker changed"
    )
    profiles = json_lines(paths["profileReceipt"], "profile receipt")
    processes = json_lines(paths["processReceipt"], "process receipt")
    if not expected:
        require(
            not profiles and not processes,
            "candidate RED unexpectedly launched a dummy process/profile",
        )
    else:
        require(
            len(profiles) == 1 and len(processes) == 1,
            "oracle lacks exactly one profile/process",
        )
        profile = profiles[0]
        process = processes[0]
        profile_path = Path(profile.get("path", ""))
        require(
            set(profile)
            == {
                "event",
                "mode",
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
            and profile["engine"] == engine
            and profile["runId"] == run_id
            and profile["workerPid"] == launch[0]["workerPid"]
            and profile_path.is_absolute()
            and profile_path.name.startswith(f"playwright_{engine}dev_profile-")
            and not profile_path.exists()
            and not profile_path.is_symlink()
            and all(
                type(profile[key]) is int and profile[key] > 0
                for key in ("device", "inode", "uid")
            )
            and profile["uid"] == os.getuid(),
            "oracle profile creation/cleanup differs",
        )
        chain = process.get("ancestry")
        child = process.get("childSnapshot")
        require(
            set(process)
            == {
                "event",
                "mode",
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
            and process["event"] == "dummy-node-exited"
            and process["mode"] == mode
            and process["engine"] == engine
            and process["runId"] == run_id
            and process["asset"] == str(upstream / ASSET)
            and process["exitCode"] == 1
            and process["childReaped"] is True
            and process["guardParentPid"] == launch[0]["workerPid"]
            and process["harnessPid"] == strict["harnessPid"]
            and isinstance(chain, list)
            and len(chain) >= 3
            and all(
                isinstance(entry, dict)
                and set(entry) == {"pid", "ppid", "uid", "executable"}
                for entry in chain
            )
            and chain[0]["pid"] == process["guardPid"]
            and chain[0]["ppid"] == process["guardParentPid"]
            and chain[-1]["pid"] == strict["harnessPid"]
            and all(
                chain[index]["ppid"] == chain[index + 1]["pid"]
                for index in range(len(chain) - 1)
            )
            and any(
                entry["pid"] == strict["officialCliPid"]
                and entry["executable"] == str(node)
                for entry in chain
            )
            and isinstance(child, dict)
            and set(child) == {"pid", "ppid", "uid", "executable"}
            and child["pid"] == process["childPid"]
            and child["ppid"] == process["guardPid"]
            and child["uid"] == os.getuid()
            and child["executable"] == str(node),
            "oracle dummy process ancestry/exit differs",
        )
        args = process["browserArguments"]
        require(
            isinstance(args, list) and all(isinstance(arg, str) for arg in args),
            "browser arguments malformed",
        )
        if engine == "firefox":
            require(
                process["browserProfile"]
                == {
                    "path": str(profile_path),
                    "device": profile["device"],
                    "inode": profile["inode"],
                    "uid": profile["uid"],
                }
                and args.count("-profile") == 1
                and args[args.index("-profile") + 1] == str(profile_path),
                "Firefox profile was not owned by the dummy launch",
            )
        else:
            require(
                process["browserProfile"] is None
                and not any(
                    arg.startswith("--user-data-dir") or arg == "-profile"
                    for arg in args
                ),
                "WebKit nonpersistent profile flags differ",
            )
    return strict


def verify_all(paths: list[Path]) -> dict[tuple[str, str], dict[str, Any]]:
    require(len(paths) == 4, "strict two-engine evidence requires four sidecars")
    verified: dict[tuple[str, str], dict[str, Any]] = {}
    for path in paths:
        evidence = verify_one(path)
        key = evidence["engine"], evidence["mode"]
        require(key not in verified, "duplicate engine/mode evidence")
        verified[key] = evidence
    require(
        set(verified)
        == {(e, mode) for e in SPEC_IDS for mode in ("candidate", "oracle")},
        "paired engine/mode coverage differs",
    )
    require(
        len({item["runId"] for item in verified.values()}) == 4
        and len({item["officialCliPid"] for item in verified.values()}) == 4
        and len({item["result"] for item in verified.values()}) == 4,
        "four official lanes are not isolated executions",
    )
    for engine in SPEC_IDS:
        candidate = verified[(engine, "candidate")]
        oracle = verified[(engine, "oracle")]
        require(
            candidate["runId"] != oracle["runId"]
            and candidate["officialCliPid"] != oracle["officialCliPid"]
            and candidate["upstreamArtifactsSha256"]
            == oracle["upstreamArtifactsSha256"]
            and candidate["nodeSha256"] == oracle["nodeSha256"]
            and candidate["manifestSha256"] == oracle["manifestSha256"],
            "candidate/oracle lanes are not distinct same-source executions",
        )
    return verified


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("strict_sidecars", type=Path, nargs="+")
    args = parser.parse_args()
    try:
        if len(args.strict_sidecars) == 1:
            sidecar = verify_one(args.strict_sidecars[0])
            print(
                f"verified strict {sidecar['engine']} {sidecar['mode']} {sidecar['testStatus']}"
            )
        else:
            verify_all(args.strict_sidecars)
            print("verified strict two-engine paired official RED evidence")
    except (
        OSError,
        TypeError,
        ValueError,
        KeyError,
        IndexError,
        json.JSONDecodeError,
        subprocess.CalledProcessError,
    ) as error:
        parser.exit(2, f"strict two-engine evidence invalid: {error}\n")


if __name__ == "__main__":
    main()
