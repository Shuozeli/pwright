"""Run only the pinned upstream-private, browserless Core image-tools bodies.

This lane intentionally does not preload or attest the Pwright runtime. Its
separate tooling keeps existing Core source-only provenance hashes unchanged.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
from pathlib import Path
from typing import Any

from official_result_destination import prepare_result_destination

_ROOT = Path(__file__).parents[1]
_MANIFEST = (
    _ROOT
    / "tests/official/playwright-core-internal-source-only-image-tools-v1-manifest.json"
)
_GUARD = _ROOT / "tests/official/playwright-core-image-tools-source-only-guard.cjs"
_RUNNER = "tests/playwright-test/stable-test-runner/node_modules/@playwright/test/cli.js"
_SUITE_ID = "core461-private-image-tools"
_CASE = "private-image-tools-batch-v1"
_COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
_FILES = {
    "../image_tools/fixtures.spec.ts",
    "../image_tools/unit.spec.ts",
}
_SOURCE_DIGEST = "850aaf28f80268352daaeaed5f58fd20af593956eabd551dae6412804f5353bb"
_INVOCATION_DIGEST = "59b6aa848f07603093d68b74b2c9ccfaa6f10ea7ab239863a6e34b69a581ee74"
_FILE_SPEC_DIGEST = "909053ffba200a5d4db0ed53adaacfac8acb51be3abfcff0a4fad13df4a0f311"
_TOOLING = (
    Path("scripts/official_result_destination.py"),
    Path("scripts/run_playwright_official_image_tools_source_only.py"),
    Path("tests/official/playwright-core-image-tools-source-only-guard.cjs"),
)


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _git(upstream: Path, *args: str) -> str:
    completed = subprocess.run(
        ["git", "-C", str(upstream), *args],
        capture_output=True,
        text=True,
        check=True,
    )
    return completed.stdout.strip()


def _tooling_snapshot() -> list[dict[str, str]]:
    return [
        {"path": str(_ROOT / relative), "sha256": _sha256(_ROOT / relative)}
        for relative in _TOOLING
    ]


def _verify_source(upstream: Path) -> tuple[dict[str, Any], dict[str, Any]]:
    manifest = json.loads(_MANIFEST.read_text())
    if (
        manifest.get("schema") != 1
        or manifest.get("repository") != "https://github.com/microsoft/playwright.git"
        or manifest.get("tag") != "v1.58.0"
        or manifest.get("commit") != _COMMIT
        or len(manifest.get("suites", [])) != 1
    ):
        raise ValueError("image-tools source-only manifest is not the pinned upstream revision")
    if _git(upstream, "rev-parse", "HEAD") != _COMMIT:
        raise ValueError("image-tools source-only upstream checkout does not match the pinned commit")
    if _git(upstream, "status", "--porcelain", "--untracked-files=no"):
        raise ValueError("image-tools source-only upstream checkout has tracked source changes")
    if not _GUARD.is_file() or _GUARD.is_symlink():
        raise ValueError("image-tools source-only guard is missing or a symlink")
    completed = subprocess.run(
        ["node", str(_GUARD), "--check", str(_MANIFEST), _SUITE_ID, str(upstream)],
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode:
        raise ValueError(
            "image-tools source-only guard rejected source: "
            + (completed.stderr.strip() or completed.stdout.strip())[-1000:]
        )
    source = json.loads(completed.stdout)
    if source != {
        "case": _CASE,
        "sourceFiles": [
            "tests/image_tools/fixtures.spec.ts",
            "tests/image_tools/unit.spec.ts",
        ],
        "sourceSha256": _SOURCE_DIGEST,
        "expectedInvocationIdsSha256": _INVOCATION_DIGEST,
        "expectedFileSpecIdsSha256": _FILE_SPEC_DIGEST,
        "expectedProjectIds": ["image_tools"],
        "expectedTestsPerProject": 25,
    }:
        raise ValueError("image-tools source-only guard emitted an unpinned source record")
    return manifest, source


def _verify_result_selection(result: Path, source: dict[str, Any]) -> int:
    """Keep failures as evidence, but reject missing, extra, or fabricated attempts."""
    report = json.loads(result.read_text())
    if not isinstance(report, dict) or not isinstance(report.get("suites"), list) or report.get("errors"):
        raise ValueError("image-tools source-only result lacks suites or has runner errors")
    found_files: set[str] = set()
    found: list[tuple[str, str]] = []
    found_file_specs: list[tuple[str, str]] = []

    def visit(raw_suite: object) -> None:
        if not isinstance(raw_suite, dict):
            raise TypeError("image-tools source-only result contains a malformed suite")
        specs = raw_suite.get("specs", [])
        children = raw_suite.get("suites", [])
        if not isinstance(specs, list) or not isinstance(children, list):
            raise TypeError("image-tools source-only result contains malformed suite lists")
        for raw_spec in specs:
            if not isinstance(raw_spec, dict):
                raise TypeError("image-tools source-only result contains a malformed spec")
            file = raw_spec.get("file")
            spec_id = raw_spec.get("id")
            tests = raw_spec.get("tests")
            if (
                file not in _FILES
                or not isinstance(spec_id, str)
                or re.fullmatch(r"[a-f0-9]{20}-[a-f0-9]{20}", spec_id) is None
                or not isinstance(tests, list)
                or len(tests) != 1
            ):
                raise ValueError("image-tools source-only result contains an unpinned spec")
            test = tests[0]
            if not isinstance(test, dict):
                raise TypeError("image-tools source-only result contains a malformed test")
            results = test.get("results")
            project_id = test.get("projectId")
            if (
                project_id != "image_tools"
                or test.get("expectedStatus") != "passed"
                or test.get("status") not in {"expected", "unexpected", "flaky", "skipped"}
                or not isinstance(results, list)
                or not results
                or not all(
                    isinstance(attempt, dict)
                    and attempt.get("status") in {
                        "passed", "failed", "timedOut", "interrupted", "skipped"
                    }
                    for attempt in results
                )
            ):
                raise ValueError("image-tools source-only result has no valid attempt")
            found_files.add(file)
            found.append((spec_id, project_id))
            found_file_specs.append((f"tests/{file.removeprefix('../')}", spec_id))
        for child in children:
            visit(child)

    for raw_suite in report["suites"]:
        visit(raw_suite)
    if found_files != _FILES or len(found) != 25 or len(set(found)) != 25:
        raise ValueError("image-tools source-only result changed its exact 25 invocations")
    digest = hashlib.sha256(
        "".join(
            f"playwright-test\t{spec_id}\t{project_id}\n"
            for spec_id, project_id in sorted(found)
        ).encode()
    ).hexdigest()
    if digest != source["expectedInvocationIdsSha256"]:
        raise ValueError("image-tools source-only result changed exact invocation IDs")
    file_spec_digest = hashlib.sha256(
        "".join(f"{file}\t{spec_id}\n" for file, spec_id in sorted(found_file_specs)).encode()
    ).hexdigest()
    if file_spec_digest != source["expectedFileSpecIdsSha256"]:
        raise ValueError("image-tools source-only result changed exact file-to-spec mapping")
    return len(found)


def _verify_guard_receipt(receipt: Path) -> int:
    if not receipt.is_file():
        raise ValueError("image-tools source-only guard produced no receipt")
    lines = receipt.read_text().splitlines()
    if len(lines) < 2:
        raise ValueError("image-tools source-only guard never covered worker processes")
    keys: set[tuple[int, int]] = set()
    kinds: list[str] = []
    for line in lines:
        record = json.loads(line)
        if (
            not isinstance(record, dict)
            or set(record) != {"kind", "pid", "threadId", "suite", "case"}
            or record["kind"] not in {"runner-no-io", "worker-no-io", "loader-thread-no-io"}
            or record["suite"] != _SUITE_ID
            or record["case"] != _CASE
            or isinstance(record["pid"], bool)
            or not isinstance(record["pid"], int)
            or record["pid"] < 1
            or isinstance(record["threadId"], bool)
            or not isinstance(record["threadId"], int)
            or record["threadId"] < 0
            or (record["pid"], record["threadId"]) in keys
            or (record["kind"] == "loader-thread-no-io") != (record["threadId"] > 0)
        ):
            raise ValueError("image-tools source-only guard receipt is invalid")
        keys.add((record["pid"], record["threadId"]))
        kinds.append(record["kind"])
    if kinds.count("runner-no-io") != 1 or kinds.count("worker-no-io") < 1:
        raise ValueError("image-tools source-only guard must cover runner and workers")
    return kinds.count("worker-no-io")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("candidate", "oracle"))
    parser.add_argument("upstream", type=Path)
    parser.add_argument("--result", type=Path, required=True)
    args = parser.parse_args()
    for key in (
        "NODE_OPTIONS", "PWTEST_MODE", "PWPAGE_IMPL", "PWTEST_USE_BIDI_EXPECTATIONS",
        "PWRIGHT_CORE_SOURCE_ONLY_TEST_WORKER", "PWRIGHT_NODE_CANDIDATE",
        "PWRIGHT_NATIVE_PROFILE", "IMAGE_TOOLS_FIXTURES", "GITHUB_ACTIONS", "GITLAB_CI",
    ):
        if os.environ.get(key):
            parser.error(f"image-tools source-only lane forbids inherited {key}")
    if os.environ.get("JENKINS_URL") and os.environ.get("BUILD_URL"):
        parser.error("image-tools source-only lane forbids Jenkins Git metadata environment")
    upstream = args.upstream.resolve()
    result = prepare_result_destination(args.result)
    receipt = result.with_suffix(".guard-receipt.jsonl")
    if receipt.is_symlink() or receipt.exists():
        parser.error(f"image-tools source-only guard receipt already exists: {receipt}")
    tooling = _tooling_snapshot()
    try:
        manifest, source = _verify_source(upstream)
    except (OSError, TypeError, ValueError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        parser.error(str(error))
    suite = manifest["suites"][0]
    runner = upstream / _RUNNER
    if not runner.is_file() or runner.is_symlink():
        parser.error(f"image-tools source-only runner is missing or a symlink: {runner}")
    command = [
        "node", str(runner), "test", f"--config={suite['config']}",
        "--reporter=line,json", "--workers=1", "--project=image_tools", *suite["args"],
    ]
    env = os.environ.copy()
    for key in (
        "PWRIGHT_OFFICIAL_CROSS_ORIGIN_FORWARD", "PWRIGHT_CANDIDATE_CDP_ENDPOINT",
        "PWRIGHT_ORACLE_CDP_ENDPOINT", "PWRIGHT_OFFICIAL_SERVER_IP",
        "PWRIGHT_OFFICIAL_SERVER_HOST",
    ):
        env.pop(key, None)
    env.update({
        "PLAYWRIGHT_JSON_OUTPUT_FILE": str(result),
        "PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM": str(upstream),
        "PWRIGHT_CORE_SOURCE_ONLY_ACTIVE": "1",
        "PWRIGHT_CORE_SOURCE_ONLY_MANIFEST": str(_MANIFEST),
        "PWRIGHT_CORE_SOURCE_ONLY_SUITE": _SUITE_ID,
        "PWRIGHT_CORE_SOURCE_ONLY_GUARD_RECEIPT": str(receipt),
        "NODE_OPTIONS": f"--require={_GUARD}",
    })
    result.parent.mkdir(parents=True, exist_ok=True)
    receipt.open("x").close()
    print(f"running pinned upstream-private image-tools mode={args.mode} count=25 browser=false", flush=True)
    print(f"result={result}", flush=True)
    completed = subprocess.run(command, cwd=upstream, env=env, check=False)
    if not result.is_file():
        raise SystemExit("image-tools source-only runner did not produce JSON evidence")
    if _tooling_snapshot() != tooling:
        raise SystemExit("image-tools source-only tooling changed during execution")
    try:
        count = _verify_result_selection(result, source)
        workers = _verify_guard_receipt(receipt)
        _, post_source = _verify_source(upstream)
    except (OSError, TypeError, ValueError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        raise SystemExit(f"image-tools source-only evidence invalid: {error}") from error
    if post_source != source:
        raise SystemExit("image-tools source-only pinned inputs changed during execution")
    provenance = {
        "schema": 1,
        "kind": "pinned-upstream-private-image-tools-source-only",
        "mode": args.mode,
        "repository": manifest["repository"],
        "tag": manifest["tag"],
        "commit": manifest["commit"],
        "manifest": str(_MANIFEST),
        "manifestSha256": _sha256(_MANIFEST),
        "upstream": str(upstream),
        "suite": _SUITE_ID,
        "suiteRole": suite["role"],
        "adapter": "upstream-private-source-only-no-pwright-runtime",
        "sourceOnlyExecution": {
            "scope": "upstream-private-source-only",
            "candidateRuntimeAttested": False,
            "publicApiParityEvidence": False,
            "source": source,
            "reportedTests": count,
            "guardReceipt": str(receipt),
            "guardReceiptSha256": _sha256(receipt),
            "guardedWorkerProcesses": workers,
        },
        "requiresBrowser": False,
        "project": "image_tools",
        "filters": suite["args"],
        "workers": 1,
        "toolingSources": tooling,
        "command": command,
        "exitCode": completed.returncode,
        "result": str(result),
        "resultSha256": _sha256(result),
    }
    if _tooling_snapshot() != tooling:
        raise SystemExit("image-tools source-only tooling changed before provenance write")
    destination = result.with_suffix(".provenance.json")
    destination.write_text(json.dumps(provenance, indent=2, sort_keys=True) + "\n")
    print(f"provenance={destination}", flush=True)
    raise SystemExit(completed.returncode)


if __name__ == "__main__":
    main()
