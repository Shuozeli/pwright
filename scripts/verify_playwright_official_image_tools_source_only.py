"""Independently verify the pinned upstream-private image-tools pass receipt.

This is deliberately separate from the runner and from the historical Core
source-only verifier. A valid receipt is not Pwright runtime or public-API
parity evidence.
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

_ROOT = Path(__file__).parents[1]
_MANIFEST = _ROOT / "tests/official/playwright-core-internal-source-only-image-tools-v2-manifest.json"
_GUARD = _ROOT / "tests/official/playwright-core-image-tools-source-only-v2-guard.cjs"
_RUNNER = "tests/playwright-test/stable-test-runner/node_modules/@playwright/test/cli.js"
_ARGS = ["tests/image_tools/fixtures.spec.ts", "tests/image_tools/unit.spec.ts"]
_SOURCE_FILES = [
    *_ARGS,
    "tests/image_tools/utils.ts",
    "tests/playwright-test/playwright.config.ts",
    "tests/playwright-test/stable-test-runner/index.js",
    "tests/playwright-test/stable-test-runner/package-lock.json",
]
_COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
_SUITE = "core461-private-image-tools"
_CASE = "private-image-tools-batch-v1"
_SOURCE_SHA256 = "850aaf28f80268352daaeaed5f58fd20af593956eabd551dae6412804f5353bb"
_INVOCATION_SHA256 = "59b6aa848f07603093d68b74b2c9ccfaa6f10ea7ab239863a6e34b69a581ee74"
_FILE_SPEC_SHA256 = "909053ffba200a5d4db0ed53adaacfac8acb51be3abfcff0a4fad13df4a0f311"
_TOOLING = [
    _ROOT / "scripts/official_result_destination.py",
    _ROOT / "scripts/run_playwright_official_image_tools_source_only.py",
    _ROOT / "scripts/run_playwright_official_image_tools_source_only_v2.py",
    _ROOT / "tests/official/playwright-core-internal-source-only-image-tools-v1-manifest.json",
    _ROOT / "tests/official/playwright-core-image-tools-source-only-guard.cjs",
    _GUARD,
]


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def _sha256(file: Path) -> str:
    return hashlib.sha256(file.read_bytes()).hexdigest()


def _git(upstream: Path, *arguments: str) -> str:
    return subprocess.run(
        ["git", "-C", str(upstream), *arguments],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()


def _source_digest(upstream: Path) -> str:
    """Hash the six pinned sources and all 42 selected PNGs, not the runner's record."""
    fixture_root = upstream / "tests/image_tools/fixtures"
    _require(fixture_root.is_dir() and not fixture_root.is_symlink(), "pinned fixture root is missing or a symlink")
    pngs: list[str] = []

    def visit(directory: Path) -> None:
        for entry in os.scandir(directory):
            child = Path(entry.path)
            _require(not entry.is_symlink(), "pinned fixture tree contains a symlink")
            if entry.is_dir(follow_symlinks=False):
                visit(child)
            elif entry.is_file(follow_symlinks=False) and re.search(r"-(?:actual|expected)\.png$", entry.name):
                pngs.append(child.relative_to(upstream).as_posix())

    visit(fixture_root)
    _require(len(pngs) == 42, "pinned fixture tree does not contain exactly 42 PNGs")
    digests: list[list[str]] = []
    for relative in sorted([*_SOURCE_FILES, *pngs]):
        file = upstream / relative
        _require(file.is_file() and not file.is_symlink(), f"pinned source is missing or a symlink: {relative}")
        digests.append([relative, _sha256(file)])
    return hashlib.sha256(json.dumps(digests, separators=(",", ":")).encode()).hexdigest()


def _verify_upstream(upstream: Path) -> None:
    _require(upstream.is_dir() and upstream.resolve() == upstream, "upstream path is missing or unresolved")
    _require(_git(upstream, "rev-parse", "HEAD") == _COMMIT, "upstream commit changed")
    _require(not _git(upstream, "status", "--porcelain", "--untracked-files=no"), "upstream tracked sources changed")
    _require(not (upstream / ".env").exists(), "upstream .env override is present")
    _require(_source_digest(upstream) == _SOURCE_SHA256, "pinned source or PNG SHA-256 mismatch")


def _verify_manifest(record: dict[str, Any]) -> dict[str, Any]:
    _require(_MANIFEST.is_file() and not _MANIFEST.is_symlink(), "pinned manifest is missing or a symlink")
    _require(record.get("manifest") == str(_MANIFEST), "provenance manifest path changed")
    _require(record.get("manifestSha256") == _sha256(_MANIFEST), "manifest SHA-256 mismatch")
    manifest = json.loads(_MANIFEST.read_text())
    _require(
        manifest.get("schema") == 1
        and manifest.get("repository") == "https://github.com/microsoft/playwright.git"
        and manifest.get("tag") == "v1.58.0"
        and manifest.get("commit") == _COMMIT
        and isinstance(manifest.get("suites"), list)
        and len(manifest["suites"]) == 1,
        "manifest revision or suite count changed",
    )
    suite = manifest["suites"][0]
    _require(
        isinstance(suite, dict)
        and suite.get("id") == _SUITE
        and suite.get("role") == "implementation-internal-source-only"
        and suite.get("sourceOnlyCase") == _CASE
        and suite.get("sourceOnlyAdapter") == "upstream-private-source-only-no-pwright-runtime"
        and "candidateAdapter" not in suite
        and suite.get("fixtureMode") == "pinned-private-source-only-no-io"
        and suite.get("requiresBrowser") is False
        and suite.get("runner") == _RUNNER
        and suite.get("config") == "tests/playwright-test/playwright.config.ts"
        and suite.get("args") == _ARGS
        and suite.get("expectedProjectIds") == ["image_tools"]
        and suite.get("expectedTestsPerProject") == 25
        and suite.get("expectedFiles") == 2
        and suite.get("upstreamSourceSetSha256") == _SOURCE_SHA256
        and suite.get("expectedInvocationIdsSha256") == _INVOCATION_SHA256
        and suite.get("expectedFileSpecIdsSha256") == _FILE_SPEC_SHA256,
        "manifest selection or digest changed",
    )
    return manifest


def _verify_result(result: Path) -> dict[str, int]:
    report = json.loads(result.read_text())
    _require(isinstance(report, dict), "result is not a JSON object")
    _require(report.get("errors") == [], "result has runner errors")
    _require(isinstance(report.get("suites"), list), "result has no suites")
    stats = report.get("stats")
    _require(
        isinstance(stats, dict)
        and stats.get("expected") == 25
        and stats.get("skipped") == 0
        and stats.get("unexpected") == 0
        and stats.get("flaky") == 0,
        "result statistics are not exactly 25 passes",
    )
    invocations: list[tuple[str, str]] = []
    file_specs: list[tuple[str, str]] = []
    by_file = {"fixtures.spec.ts": 0, "unit.spec.ts": 0}

    def visit(raw_suite: object) -> None:
        _require(isinstance(raw_suite, dict), "result contains a malformed suite")
        specs = raw_suite.get("specs", [])
        children = raw_suite.get("suites", [])
        _require(isinstance(specs, list) and isinstance(children, list), "result suite lists are malformed")
        for raw_spec in specs:
            _require(isinstance(raw_spec, dict), "result contains a malformed spec")
            file = raw_spec.get("file")
            spec_id = raw_spec.get("id")
            tests = raw_spec.get("tests")
            _require(
                file in {"../image_tools/fixtures.spec.ts", "../image_tools/unit.spec.ts"}
                and isinstance(spec_id, str)
                and re.fullmatch(r"[a-f0-9]{20}-[a-f0-9]{20}", spec_id) is not None
                and isinstance(tests, list)
                and len(tests) == 1,
                "result contains an unpinned spec or file",
            )
            test = tests[0]
            _require(isinstance(test, dict), "result contains a malformed test")
            attempts = test.get("results")
            _require(
                test.get("projectId") == "image_tools"
                and test.get("expectedStatus") == "passed"
                and test.get("status") == "expected"
                and isinstance(attempts, list)
                and len(attempts) == 1
                and isinstance(attempts[0], dict)
                and attempts[0].get("status") == "passed"
                and attempts[0].get("retry") == 0,
                "result contains a non-pass, skipped, or retried invocation",
            )
            invocations.append((spec_id, "image_tools"))
            relative = "tests/" + file.removeprefix("../")
            file_specs.append((relative, spec_id))
            by_file[Path(file).name] += 1
        for child in children:
            visit(child)

    for raw_suite in report["suites"]:
        visit(raw_suite)
    _require(len(invocations) == 25 and len(set(invocations)) == 25, "result does not contain 25 unique invocations")
    _require(by_file == {"fixtures.spec.ts": 21, "unit.spec.ts": 4}, "result changed 21/4 file split")
    invocation_digest = hashlib.sha256(
        "".join(f"playwright-test\t{spec_id}\t{project}\n" for spec_id, project in sorted(invocations)).encode()
    ).hexdigest()
    _require(invocation_digest == _INVOCATION_SHA256, "result changed exact invocation IDs")
    file_digest = hashlib.sha256(
        "".join(f"{file}\t{spec_id}\n" for file, spec_id in sorted(file_specs)).encode()
    ).hexdigest()
    _require(file_digest == _FILE_SPEC_SHA256, "result changed exact file-to-ID mapping")
    return {"passed": 25, "fixtures": 21, "unit": 4}


def _verify_guard_receipt(receipt: Path) -> int:
    lines = receipt.read_text().splitlines()
    _require(len(lines) >= 2, "guard receipt did not cover runner and worker")
    keys: set[tuple[int, int]] = set()
    counts = {"runner-no-io": 0, "worker-no-io": 0, "loader-thread-no-io": 0}
    runner_pid: int | None = None
    worker_pids: set[int] = set()
    for line in lines:
        item = json.loads(line)
        _require(isinstance(item, dict) and set(item) == {"kind", "pid", "threadId", "suite", "case"}, "guard record is malformed")
        kind, pid, thread = item["kind"], item["pid"], item["threadId"]
        _require(
            kind in counts and item["suite"] == _SUITE and item["case"] == _CASE
            and type(pid) is int and pid > 0 and type(thread) is int and thread >= 0
            and (pid, thread) not in keys
            and (kind == "loader-thread-no-io") == (thread > 0),
            "guard record has an unpinned process or duplicate",
        )
        keys.add((pid, thread))
        counts[kind] += 1
        if kind == "runner-no-io":
            runner_pid = pid
        if kind == "worker-no-io":
            worker_pids.add(pid)
    _require(
        counts["runner-no-io"] == 1 and counts["worker-no-io"] >= 1
        and runner_pid not in worker_pids,
        "guard receipt did not cover one runner and separate workers",
    )
    _require(
        all(pid == runner_pid or pid in worker_pids for pid, _ in keys),
        "guard receipt contains an unaccounted loader thread",
    )
    return counts["worker-no-io"]


def verify(provenance_path: Path) -> dict[str, Any]:
    _require(provenance_path.is_file() and not provenance_path.is_symlink(), "provenance file is missing or a symlink")
    record = json.loads(provenance_path.read_text())
    _require(isinstance(record, dict), "provenance is not a JSON object")
    _require(set(record) == {
        "schema", "kind", "mode", "repository", "tag", "commit", "manifest",
        "manifestSha256", "upstream", "suite", "suiteRole", "adapter",
        "sourceOnlyExecution", "requiresBrowser", "project", "filters", "workers",
        "toolingSources", "command", "exitCode", "result", "resultSha256",
    }, "provenance contains missing or contradictory fields")
    _require(
        record.get("schema") == 1
        and record.get("kind") == "pinned-upstream-private-image-tools-source-only"
        and record.get("mode") in {"candidate", "oracle"}
        and record.get("suite") == _SUITE
        and record.get("suiteRole") == "implementation-internal-source-only"
        and record.get("adapter") == "upstream-private-source-only-no-pwright-runtime"
        and record.get("requiresBrowser") is False
        and record.get("project") == "image_tools"
        and record.get("filters") == _ARGS
        and record.get("workers") == 1
        and record.get("exitCode") == 0,
        "provenance scope, selection, or exit code changed",
    )
    manifest = _verify_manifest(record)
    _require(
        all(record.get(key) == manifest[key] for key in ("repository", "tag", "commit")),
        "provenance upstream revision changed",
    )
    upstream = Path(record["upstream"])
    _verify_upstream(upstream)
    result = Path(record["result"])
    _require(result.is_file() and not result.is_symlink(), "result is missing or a symlink")
    _require(result.with_suffix(".provenance.json") == provenance_path, "result and provenance paths differ")
    _require(record.get("resultSha256") == _sha256(result), "result SHA-256 mismatch")
    expected_command = [
        "node", str(upstream / _RUNNER), "test",
        "--config=tests/playwright-test/playwright.config.ts", "--reporter=line,json",
        "--workers=1", "--project=image_tools", *_ARGS,
    ]
    _require(record.get("command") == expected_command, "runner command changed")
    _require(
        all(file.is_file() and not file.is_symlink() for file in _TOOLING),
        "source-only tooling is missing or a symlink",
    )
    _require(
        record.get("toolingSources") == [
            {"path": str(file), "sha256": _sha256(file)} for file in _TOOLING
        ],
        "tooling SHA-256 or selection changed",
    )
    execution = record.get("sourceOnlyExecution")
    _require(isinstance(execution, dict), "source-only execution record is missing")
    _require(set(execution) == {
        "scope", "candidateRuntimeAttested", "publicApiParityEvidence", "source",
        "reportedTests", "guardReceipt", "guardReceiptSha256", "guardedWorkerProcesses",
    }, "source-only execution record contains missing or contradictory fields")
    _require(
        execution.get("scope") == "upstream-private-source-only"
        and execution.get("candidateRuntimeAttested") is False
        and execution.get("publicApiParityEvidence") is False
        and execution.get("reportedTests") == 25
        and execution.get("source") == {
            "case": _CASE,
            "sourceFiles": _ARGS,
            "sourceSha256": _SOURCE_SHA256,
            "expectedInvocationIdsSha256": _INVOCATION_SHA256,
            "expectedFileSpecIdsSha256": _FILE_SPEC_SHA256,
            "expectedProjectIds": ["image_tools"],
            "expectedTestsPerProject": 25,
        },
        "source-only attestation flags or source record changed",
    )
    receipt = Path(execution["guardReceipt"])
    _require(receipt.is_file() and not receipt.is_symlink(), "guard receipt is missing or a symlink")
    _require(receipt == result.with_suffix(".guard-receipt.jsonl"), "guard receipt path changed")
    _require(execution.get("guardReceiptSha256") == _sha256(receipt), "guard receipt SHA-256 mismatch")
    worker_count = _verify_guard_receipt(receipt)
    _require(execution.get("guardedWorkerProcesses") == worker_count, "guarded worker count changed")
    counts = _verify_result(result)
    return {
        "scope": "upstream-private-source-only",
        "classification": "implementation-internal",
        "suite": _SUITE,
        "mode": record["mode"],
        "resultSha256": record["resultSha256"],
        "passed": counts["passed"],
        "fixtureTests": counts["fixtures"],
        "unitTests": counts["unit"],
        "guardedWorkerProcesses": worker_count,
        "candidateRuntimeAttested": False,
        "publicApiParityEvidence": False,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("provenance", type=Path)
    args = parser.parse_args()
    try:
        summary = verify(args.provenance.absolute())
    except (OSError, KeyError, TypeError, ValueError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        parser.error(str(error))
    print(json.dumps(summary, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
