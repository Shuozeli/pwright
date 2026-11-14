#!/usr/bin/env python3
"""Capture or verify one pinned official Core test using real public launch."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
from pathlib import Path
from typing import cast


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "tests/official/playwright-core-owned-launch-manifest.json"
LOCAL_SOURCES = (
    "scripts/run_playwright_core_owned_launch.py",
    "tests/official/playwright-core-owned-launch-preload.cjs",
)
CANDIDATE_PRELOAD = "tests/official/playwright-core-candidate-preload.cjs"
SPEC_FILE = "library/browsertype-launch.spec.ts"
SPEC_SELECTOR = "tests/library/browsertype-launch.spec.ts:130"
SPEC_ID = "d1a604bdf6287c6776f3-7af9e16dff447fc147e4"
PROJECT = "chromium-library"
PROFILE = "owned-launch"
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
SHA256 = re.compile(r"[0-9a-f]{64}\Z")


def digest(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def source_digest(candidate: Path) -> str:
    package = candidate / "package.json"
    source = candidate / "src"
    if not package.is_file() or not source.is_dir():
        raise ValueError("candidate must contain package.json and src")
    files = [package, *(item for item in source.rglob("*") if item.is_file())]
    value = hashlib.sha256()
    for item in sorted(files, key=lambda path: path.relative_to(candidate).as_posix()):
        relative = item.relative_to(candidate).as_posix().encode()
        value.update(len(relative).to_bytes(8, "big"))
        value.update(relative)
        size = item.stat().st_size
        value.update(size.to_bytes(8, "big"))
        with item.open("rb") as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                value.update(chunk)
    return value.hexdigest()


def pinned_manifest(upstream: Path) -> dict[str, object]:
    raw = json.loads(MANIFEST.read_text())
    if not isinstance(raw, dict) or raw.get("schema") != 1:
        raise ValueError("owned Core manifest schema must be 1")
    manifest = cast(dict[str, object], raw)
    if (
        manifest.get("repository") != "https://github.com/microsoft/playwright.git"
        or manifest.get("tag") != "v1.58.0"
        or manifest.get("commit") != COMMIT
    ):
        raise ValueError("owned Core manifest repository or revision differs")
    suite = manifest.get("suite")
    if not isinstance(suite, dict):
        raise ValueError("owned Core manifest must contain exactly one suite")
    expected = {
        "id": "library-browsertype-launch-close-twice-chromium-owned",
        "role": "browser-backed-owned-launch-subset",
        "runner": "packages/playwright/cli.js",
        "config": "tests/library/playwright.config.ts",
        "project": PROJECT,
        "selector": SPEC_SELECTOR,
        "specId": SPEC_ID,
        "title": "should be callable twice",
        "candidateAdapter": "test-mode-playwright-core-owned-public-launch",
        "oracleAdapter": "test-mode-playwright-core-owned-public-launch",
    }
    for key, value in expected.items():
        if suite.get(key) != value:
            raise ValueError(
                f"owned Core manifest {key} differs from the one-spec lane"
            )
    sources = suite.get("upstreamSources")
    if not isinstance(sources, dict) or set(sources) != {
        SPEC_SELECTOR.split(":")[0],
        "tests/library/playwright.config.ts",
        "tests/config/browserTest.ts",
        "tests/config/testMode.ts",
        "packages/playwright/src/index.ts",
        "packages/playwright-core/src/client/browserType.ts",
    }:
        raise ValueError("owned Core manifest has an unexpected source set")
    for relative, expected_sha in sources.items():
        if (
            not isinstance(relative, str)
            or not isinstance(expected_sha, str)
            or not SHA256.fullmatch(expected_sha)
        ):
            raise ValueError("owned Core manifest has a malformed source digest")
        path = upstream / relative
        if not path.is_file() or digest(path) != expected_sha:
            raise ValueError(f"owned Core upstream source drifted: {relative}")
    return manifest


def pinned_checkout(upstream: Path, manifest: dict[str, object]) -> None:
    revision = subprocess.run(
        ["git", "-C", str(upstream), "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    if revision != manifest.get("commit"):
        raise ValueError("owned Core checkout does not match the pinned commit")
    dirty = subprocess.run(
        ["git", "-C", str(upstream), "status", "--porcelain", "--untracked-files=no"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    if dirty:
        raise ValueError("owned Core checkout has tracked source changes")


def command(upstream: Path) -> list[str]:
    return [
        "node",
        str(upstream / "packages/playwright/cli.js"),
        "test",
        "--config=tests/library/playwright.config.ts",
        "--reporter=line,json",
        "--workers=1",
        "--retries=0",
        f"--project={PROJECT}",
        SPEC_SELECTOR,
    ]


def report_case(result: Path) -> dict[str, object]:
    raw = json.loads(result.read_text())
    if not isinstance(raw, dict) or not isinstance(raw.get("suites"), list):
        raise ValueError("owned Core JSON result has no suites")
    found: list[dict[str, object]] = []

    def visit(node: object) -> None:
        if (
            not isinstance(node, dict)
            or not isinstance(node.get("specs"), list)
            or not isinstance(node.get("suites"), list)
        ):
            raise ValueError("owned Core JSON result has a malformed suite")
        found.extend(cast(list[dict[str, object]], node["specs"]))
        for child in cast(list[object], node["suites"]):
            visit(child)

    for top in cast(list[object], raw["suites"]):
        visit(top)
    if len(found) != 1:
        raise ValueError(
            f"owned Core result must report exactly one spec, got {len(found)}"
        )
    spec = found[0]
    tests = spec.get("tests")
    if (
        spec.get("id") != SPEC_ID
        or spec.get("file") != SPEC_FILE
        or spec.get("line") != 130
        or spec.get("title") != "should be callable twice"
        or not isinstance(tests, list)
        or len(tests) != 1
        or not isinstance(tests[0], dict)
        or tests[0].get("projectId") != PROJECT
        or tests[0].get("expectedStatus") != "passed"
        or not isinstance(tests[0].get("results"), list)
    ):
        raise ValueError("owned Core result does not identify the pinned Chromium test")
    return cast(dict[str, object], tests[0])


def passed_case(test: dict[str, object]) -> bool:
    results = test.get("results")
    return (
        test.get("status") == "expected"
        and isinstance(results, list)
        and len(results) == 1
        and isinstance(results[0], dict)
        and results[0].get("status") == "passed"
        and results[0].get("retry") == 0
    )


def receipt_events(
    receipt: Path, *, mode: str, executable: Path, require_success: bool
) -> list[dict[str, object]]:
    records = [json.loads(line) for line in receipt.read_text().splitlines() if line]
    if not all(isinstance(item, dict) for item in records):
        raise ValueError("owned Core launch receipt contains non-object events")
    events = cast(list[dict[str, object]], records)
    for event in events:
        if (
            set(event)
            != {"event", "mode", "publicMethod", "executablePath", "headless"}
            or event["mode"] != mode
            or event["publicMethod"] != "chromium.launch"
            or event["executablePath"] != str(executable)
            or not isinstance(event["headless"], bool)
        ):
            raise ValueError(
                "owned Core launch receipt has unexpected fields or identity"
            )
    if require_success and [event["event"] for event in events] != [
        "launch-invoked",
        "launch-succeeded",
    ]:
        raise ValueError(
            "successful owned Core result needs one real public launch receipt"
        )
    if require_success and events[0]["headless"] != events[1]["headless"]:
        raise ValueError("owned Core launch receipt changed headless mode mid-call")
    return events


def _fresh(path: Path) -> Path:
    result = path.parent.resolve() / path.name
    for output in (
        result,
        result.with_suffix(".provenance.json"),
        result.with_suffix(".launch-receipt.jsonl"),
    ):
        if output.is_symlink() or output.exists():
            raise ValueError(f"owned Core destination already exists: {output}")
    return result


def run(args: argparse.Namespace) -> None:
    upstream = args.upstream.resolve()
    executable = args.executable.resolve()
    result = _fresh(args.result)
    if (
        not args.executable.is_absolute()
        or not executable.is_file()
        or not os.access(executable, os.X_OK)
    ):
        raise ValueError("--executable must be an absolute executable file")
    manifest = pinned_manifest(upstream)
    pinned_checkout(upstream, manifest)
    runner = upstream / "packages/playwright/cli.js"
    if not runner.is_file():
        raise ValueError("pinned Core test runner is missing")
    candidate = args.candidate.resolve() if args.mode == "candidate" else None
    artifact = candidate / "native/owned-launch/pwright.node" if candidate else None
    if candidate and (
        not (candidate / "package.json").is_file() or not artifact.is_file()
    ):
        raise ValueError("candidate package or owned-launch native artifact is missing")
    if candidate and not (candidate / "src").is_dir():
        raise ValueError("candidate source is missing")

    env = os.environ.copy()
    for name in (
        "PWRIGHT_CANDIDATE_CDP_ENDPOINT",
        "PWRIGHT_ORACLE_CDP_ENDPOINT",
        "PWRIGHT_CDP_ENDPOINT",
    ):
        env.pop(name, None)
    if env.get("PWTEST_MODE", "default") != "default" or env.get("PWTEST_CHANNEL"):
        raise ValueError(
            "owned Core lane requires default test mode and no channel override"
        )
    if env.get("NODE_OPTIONS"):
        raise ValueError("owned Core lane requires empty inherited NODE_OPTIONS")
    receipt = result.with_suffix(".launch-receipt.jsonl")
    result.parent.mkdir(parents=True, exist_ok=True)
    receipt.write_text("")
    env.pop("CI", None)
    env["PWTEST_MODE"] = "default"
    env["CRPATH"] = str(executable)
    env["PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM"] = str(upstream)
    env["PWRIGHT_OFFICIAL_OWNED_LAUNCH_MODE"] = args.mode
    env["PWRIGHT_OFFICIAL_OWNED_LAUNCH_RECEIPT"] = str(receipt)
    env["PLAYWRIGHT_JSON_OUTPUT_FILE"] = str(result)
    harness = [ROOT / CANDIDATE_PRELOAD] if candidate else []
    harness.append(ROOT / LOCAL_SOURCES[1])
    env["NODE_OPTIONS"] = " ".join(f"--require={item}" for item in harness)
    if candidate:
        env["PWRIGHT_NODE_CANDIDATE"] = str(candidate)
        env["PWRIGHT_NATIVE_PROFILE"] = PROFILE
    else:
        env.pop("PWRIGHT_NODE_CANDIDATE", None)
        env.pop("PWRIGHT_NATIVE_PROFILE", None)

    selected_command = command(upstream)
    completed = subprocess.run(selected_command, cwd=upstream, env=env, check=False)
    if not result.is_file():
        raise ValueError("owned Core runner did not produce a JSON result")
    test = report_case(result)
    receipt_events(
        receipt,
        mode=args.mode,
        executable=executable,
        require_success=completed.returncode == 0,
    )
    if completed.returncode == 0 and not passed_case(test):
        raise ValueError(
            "owned Core runner exited successfully without the expected test outcome"
        )
    local_sources = [*LOCAL_SOURCES, *([CANDIDATE_PRELOAD] if candidate else [])]
    provenance = {
        "schema": 1,
        "launchMode": "owned",
        "evidenceScope": "diagnostic-public-launch-no-process-profile-attestation",
        "mode": args.mode,
        "manifest": str(MANIFEST),
        "manifestSha256": digest(MANIFEST),
        "commit": manifest["commit"],
        "suite": cast(dict[str, object], manifest["suite"])["id"],
        "project": PROJECT,
        "specId": SPEC_ID,
        "upstream": str(upstream),
        "executable": str(executable),
        "executableSha256": digest(executable),
        "candidate": str(candidate) if candidate else None,
        "candidateProfile": PROFILE if candidate else None,
        "candidateSourceSha256": source_digest(candidate) if candidate else None,
        "candidateArtifactSha256": digest(artifact) if artifact else None,
        "localSources": {
            relative: digest(ROOT / relative) for relative in local_sources
        },
        "command": selected_command,
        "exitCode": completed.returncode,
        "result": str(result),
        "resultSha256": digest(result),
        "receipt": str(receipt),
        "receiptSha256": digest(receipt),
    }
    sidecar = result.with_suffix(".provenance.json")
    sidecar.write_text(json.dumps(provenance, indent=2, sort_keys=True) + "\n")
    verify(sidecar)
    print(f"owned Core result={result} provenance={sidecar}", flush=True)
    raise SystemExit(completed.returncode)


def verify(sidecar: Path) -> None:
    raw = json.loads(sidecar.read_text())
    if not isinstance(raw, dict):
        raise ValueError("owned Core provenance must be an object")
    value = cast(dict[str, object], raw)
    expected_keys = {
        "schema",
        "launchMode",
        "evidenceScope",
        "mode",
        "manifest",
        "manifestSha256",
        "commit",
        "suite",
        "project",
        "specId",
        "upstream",
        "executable",
        "executableSha256",
        "candidate",
        "candidateProfile",
        "candidateSourceSha256",
        "candidateArtifactSha256",
        "localSources",
        "command",
        "exitCode",
        "result",
        "resultSha256",
        "receipt",
        "receiptSha256",
    }
    if (
        set(value) != expected_keys
        or value["schema"] != 1
        or value["launchMode"] != "owned"
        or value["evidenceScope"]
        != "diagnostic-public-launch-no-process-profile-attestation"
    ):
        raise ValueError("owned Core provenance schema or fields differ")
    mode = value["mode"]
    if mode not in ("candidate", "oracle"):
        raise ValueError("owned Core provenance mode must be candidate or oracle")
    upstream = Path(cast(str, value["upstream"]))
    manifest = pinned_manifest(upstream)
    pinned_checkout(upstream, manifest)
    if value["manifest"] != str(MANIFEST) or value["manifestSha256"] != digest(
        MANIFEST
    ):
        raise ValueError("owned Core manifest identity changed")
    if (
        value["commit"] != manifest["commit"]
        or value["suite"] != cast(dict[str, object], manifest["suite"])["id"]
        or value["project"] != PROJECT
        or value["specId"] != SPEC_ID
    ):
        raise ValueError("owned Core pinned suite identity differs")
    executable = Path(cast(str, value["executable"]))
    if (
        not executable.is_absolute()
        or not executable.is_file()
        or value["executableSha256"] != digest(executable)
    ):
        raise ValueError("owned Core executable identity changed")
    candidate = Path(cast(str, value["candidate"])) if mode == "candidate" else None
    artifact = candidate / "native/owned-launch/pwright.node" if candidate else None
    if candidate:
        if (
            value["candidateProfile"] != PROFILE
            or value["candidateSourceSha256"] != source_digest(candidate)
            or not artifact.is_file()
            or value["candidateArtifactSha256"] != digest(artifact)
        ):
            raise ValueError("owned Core candidate source or artifact identity changed")
    elif any(
        value[key] is not None
        for key in (
            "candidate",
            "candidateProfile",
            "candidateSourceSha256",
            "candidateArtifactSha256",
        )
    ):
        raise ValueError("owned Core oracle provenance contains candidate fields")
    local_sources = [*LOCAL_SOURCES, *([CANDIDATE_PRELOAD] if candidate else [])]
    if value["localSources"] != {
        relative: digest(ROOT / relative) for relative in local_sources
    }:
        raise ValueError("owned Core local harness source identity changed")
    if (
        value["command"] != command(upstream)
        or isinstance(value["exitCode"], bool)
        or not isinstance(value["exitCode"], int)
    ):
        raise ValueError("owned Core command or exit code differs")
    result = Path(cast(str, value["result"]))
    receipt = Path(cast(str, value["receipt"]))
    if (
        not result.is_absolute()
        or sidecar.resolve() != result.with_suffix(".provenance.json").resolve()
        or not result.is_file()
        or value["resultSha256"] != digest(result)
        or receipt != result.with_suffix(".launch-receipt.jsonl")
        or not receipt.is_file()
        or value["receiptSha256"] != digest(receipt)
    ):
        raise ValueError("owned Core result or launch receipt identity changed")
    test = report_case(result)
    succeeded = value["exitCode"] == 0
    receipt_events(
        receipt, mode=cast(str, mode), executable=executable, require_success=succeeded
    )
    if succeeded and not passed_case(test):
        raise ValueError("owned Core successful run lacks expected test outcome")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="action", required=True)
    for mode in ("candidate", "oracle"):
        selected = commands.add_parser(mode)
        selected.add_argument("upstream", type=Path)
        selected.add_argument("--executable", type=Path, required=True)
        selected.add_argument("--result", type=Path, required=True)
        if mode == "candidate":
            selected.add_argument(
                "--candidate", type=Path, default=ROOT / "packages/pwright"
            )
    inspect = commands.add_parser("verify")
    inspect.add_argument("provenance", type=Path)
    args = parser.parse_args()
    try:
        if args.action == "verify":
            verify(args.provenance)
            print(f"verified owned Core public-launch provenance: {args.provenance}")
        else:
            args.mode = args.action
            run(args)
    except (
        OSError,
        ValueError,
        subprocess.CalledProcessError,
        json.JSONDecodeError,
    ) as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
