"""Verify the pinned upstream-private reporter-helper source-only receipt.

This is product-infrastructure evidence, never Pwright runtime or public API
compatibility evidence. The candidate label does not load the candidate.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path
from typing import Any

_ROOT = Path(__file__).parents[1]
_MANIFEST = _ROOT / "tests/official/playwright-core-fit-to-width-source-only-v1-manifest.json"
_GUARD = _ROOT / "tests/official/playwright-core-fit-to-width-source-only-v1-guard.cjs"
_NO_IO_GUARD = _ROOT / "tests/official/playwright-core-image-tools-source-only-guard.cjs"
_RUNNER = "tests/playwright-test/stable-test-runner/node_modules/@playwright/test/cli.js"
_CONFIG = "tests/playwright-test/playwright.config.ts"
_TEST = "tests/playwright-test/fit-to-width.spec.ts"
_ARGS = [_TEST]
_PROJECT = "playwright-test"
_SUITE = "core463-private-fit-to-width"
_CASE = "private-fit-to-width-v1"
_COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
_SOURCE_FILES = sorted([
    "packages/playwright/lib/reporters/base.js",
    "packages/playwright/src/reporters/base.ts",
    "tests/config/commonFixtures.ts",
    "tests/config/serverFixtures.ts",
    _TEST,
    "tests/playwright-test/playwright-test-fixtures.ts",
    _CONFIG,
    "tests/playwright-test/stable-test-runner/index.js",
    "tests/playwright-test/stable-test-runner/package-lock.json",
])
_SOURCE_SHA256 = "b79360d265bc9f66eda62558a41e6e62b1f8ea3fae94b976ebe936a4a975b0cf"
_TEST_SHA256 = "ee10bc8218f91870611fbc097c27759838e2a2946bd434a2d1459a579d8662e3"
_IDS_SHA256 = "99b4845c0c87acbb50f4d5ba1caa0cc1d747c77443f95740211c45e6a5dd6282"
_FILE_SPEC_SHA256 = "efa42c756737f3d2f704707742eca8f44c7febe189a57150e8131d69971959ba"
_INVOCATIONS = [
    {"line": 20, "projectId": _PROJECT, "specId": "43ecc55b07560c6b6ffc-c2e63de005eea391e61b"},
    {"line": 25, "projectId": _PROJECT, "specId": "43ecc55b07560c6b6ffc-9aca4b4e10409bacd54b"},
]
_TOOLING = [
    _ROOT / "scripts/official_result_destination.py",
    _ROOT / "scripts/run_playwright_official_fit_to_width_source_only.py",
    _ROOT / "scripts/verify_playwright_official_fit_to_width_source_only.py",
    _GUARD,
    _NO_IO_GUARD,
]


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def _sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _file_sha256(path: Path) -> str:
    return _sha256(path.read_bytes())


def _git(upstream: Path, *arguments: str) -> str:
    return subprocess.run(
        ["git", "-C", str(upstream), *arguments],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()


def tooling_snapshot() -> list[dict[str, str]]:
    for file in _TOOLING:
        _require(file.is_file() and not file.is_symlink(), f"source-only tooling is missing or a symlink: {file}")
    return [{"path": str(file), "sha256": _file_sha256(file)} for file in _TOOLING]


def verify_manifest() -> dict[str, Any]:
    _require(_MANIFEST.is_file() and not _MANIFEST.is_symlink(), "fit-to-width manifest is missing or a symlink")
    manifest = json.loads(_MANIFEST.read_bytes())
    _require(isinstance(manifest, dict) and set(manifest) == {"schema", "repository", "tag", "commit", "suites"},
             "fit-to-width manifest keys changed")
    _require(manifest["schema"] == 1 and manifest["repository"] == "https://github.com/microsoft/playwright.git"
             and manifest["tag"] == "v1.58.0" and manifest["commit"] == _COMMIT,
             "fit-to-width upstream revision changed")
    suites = manifest["suites"]
    _require(isinstance(suites, list) and len(suites) == 1, "fit-to-width manifest suite count changed")
    suite = suites[0]
    _require(isinstance(suite, dict) and suite == {
        "id": _SUITE,
        "role": "implementation-internal-source-only",
        "runner": _RUNNER,
        "config": _CONFIG,
        "args": _ARGS,
        "sourceOnlyAdapter": "upstream-private-source-only-no-pwright-runtime",
        "requiresBrowser": False,
        "fixtureMode": "pinned-private-source-only-no-io",
        "sourceOnlyCase": _CASE,
        "upstreamSourceSetSha256": _SOURCE_SHA256,
        "expectedInvocationIdsSha256": _IDS_SHA256,
        "expectedFileSpecIdsSha256": _FILE_SPEC_SHA256,
        "expectedTestsPerProject": 2,
        "expectedFiles": 1,
        "expectedProjectIds": [_PROJECT],
        "expectedInvocations": _INVOCATIONS,
    }, "fit-to-width manifest selection changed")
    return manifest


def verify_upstream(upstream: Path) -> dict[str, Any]:
    _require(upstream.is_dir() and upstream.resolve() == upstream, "upstream path is missing or unresolved")
    _require(_git(upstream, "rev-parse", "HEAD") == _COMMIT, "upstream commit changed")
    _require(not _git(upstream, "status", "--porcelain", "--untracked-files=no"), "upstream tracked sources changed")
    _require(not (upstream / ".env").exists(), "upstream .env override is present")
    pairs: list[list[str]] = []
    for relative in _SOURCE_FILES:
        file = upstream / relative
        _require(file.is_file() and not file.is_symlink(), f"upstream source is missing or a symlink: {relative}")
        pairs.append([relative, _file_sha256(file)])
    digest = _sha256(json.dumps(pairs, separators=(",", ":")).encode())
    _require(digest == _SOURCE_SHA256, "upstream test, implementation, fixture, or config digest changed")
    _require(_file_sha256(upstream / _TEST) == _TEST_SHA256, "upstream fit-to-width test changed")
    command = ["node", str(_GUARD), "--check", str(_MANIFEST), _SUITE, str(upstream)]
    checked = subprocess.run(command, capture_output=True, text=True, check=False)
    _require(checked.returncode == 0, "fit-to-width AST/no-I/O guard rejected pinned source")
    source = json.loads(checked.stdout)
    _require(source == {
        "case": _CASE,
        "sourceFiles": _SOURCE_FILES,
        "sourceSha256": _SOURCE_SHA256,
        "testSha256": _TEST_SHA256,
        "expectedInvocationIdsSha256": _IDS_SHA256,
        "expectedFileSpecIdsSha256": _FILE_SPEC_SHA256,
        "expectedProjectIds": [_PROJECT],
        "expectedInvocations": _INVOCATIONS,
        "expectedTestsPerProject": 2,
    }, "fit-to-width guard emitted an unpinned source record")
    return source


def verify_result(result: Path, *, parsed_bytes: bytes | None = None) -> dict[str, int]:
    _require(result.is_file() and not result.is_symlink(), "result is missing or a symlink")
    report = json.loads(parsed_bytes if parsed_bytes is not None else result.read_bytes())
    _require(isinstance(report, dict) and report.get("errors") == [], "result has runner errors")
    _require(isinstance(report.get("suites"), list), "result has no suites")
    stats = report.get("stats")
    _require(isinstance(stats, dict) and stats.get("expected") == 2 and stats.get("unexpected") == 0
             and stats.get("flaky") == 0 and stats.get("skipped") == 0,
             "result statistics are not exactly two passes")
    found: list[tuple[str, str]] = []
    file_specs: list[tuple[str, str]] = []
    exact_ids = {item["specId"] for item in _INVOCATIONS}

    def visit(raw_suite: object) -> None:
        _require(isinstance(raw_suite, dict), "result contains a malformed suite")
        specs = raw_suite.get("specs", [])
        children = raw_suite.get("suites", [])
        _require(isinstance(specs, list) and isinstance(children, list), "result suite lists are malformed")
        for raw_spec in specs:
            _require(isinstance(raw_spec, dict), "result contains a malformed spec")
            spec_id = raw_spec.get("id")
            tests = raw_spec.get("tests")
            _require(raw_spec.get("file") == "fit-to-width.spec.ts" and spec_id in exact_ids
                     and isinstance(tests, list) and len(tests) == 1,
                     "result contains an unpinned spec or file")
            test = tests[0]
            _require(isinstance(test, dict), "result contains a malformed test")
            attempts = test.get("results")
            _require(test.get("projectId") == _PROJECT and test.get("expectedStatus") == "passed"
                     and test.get("status") == "expected" and isinstance(attempts, list)
                     and len(attempts) == 1 and isinstance(attempts[0], dict)
                     and attempts[0].get("status") == "passed" and attempts[0].get("retry") == 0,
                     "result contains a non-pass, skipped, or retried invocation")
            found.append((spec_id, _PROJECT))
            file_specs.append((_TEST, spec_id))
        for child in children:
            visit(child)

    for raw_suite in report["suites"]:
        visit(raw_suite)
    _require(len(found) == 2 and len(set(found)) == 2, "result does not contain two unique invocations")
    digest = _sha256("".join(f"playwright-test\t{spec_id}\t{project}\n" for spec_id, project in sorted(found)).encode())
    _require(digest == _IDS_SHA256, "result invocation IDs changed")
    file_digest = _sha256("".join(f"{file}\t{spec_id}\n" for file, spec_id in sorted(file_specs)).encode())
    _require(file_digest == _FILE_SPEC_SHA256, "result file-to-ID mapping changed")
    return {"passed": 2}


def verify_guard_receipt(receipt: Path, *, mode: str, parsed_bytes: bytes | None = None) -> int:
    _require(receipt.is_file() and not receipt.is_symlink(), "guard receipt is missing or a symlink")
    _require(mode in {"candidate", "oracle"}, "guard receipt mode is invalid")
    lines = (parsed_bytes if parsed_bytes is not None else receipt.read_bytes()).decode().splitlines()
    _require(len(lines) >= 2, "guard receipt did not cover runner and worker")
    keys: set[tuple[int, int]] = set()
    counts = {"runner-no-io": 0, "worker-no-io": 0, "loader-thread-no-io": 0}
    runner_pid: int | None = None
    worker_pids: set[int] = set()
    for line in lines:
        item = json.loads(line)
        _require(isinstance(item, dict) and set(item) == {"kind", "pid", "threadId", "suite", "mode", "case"},
                 "guard record is malformed")
        kind, pid, thread = item["kind"], item["pid"], item["threadId"]
        _require(kind in counts and item["suite"] == _SUITE and item["mode"] == mode and item["case"] == _CASE
                 and type(pid) is int and pid > 0 and type(thread) is int and thread >= 0
                 and (pid, thread) not in keys and (kind == "loader-thread-no-io") == (thread > 0),
                 "guard record has an unpinned process or duplicate")
        keys.add((pid, thread))
        counts[kind] += 1
        if kind == "runner-no-io":
            runner_pid = pid
        if kind == "worker-no-io":
            worker_pids.add(pid)
    _require(counts["runner-no-io"] == 1 and counts["worker-no-io"] >= 1
             and runner_pid not in worker_pids and all(pid == runner_pid or pid in worker_pids for pid, _ in keys),
             "guard receipt did not cover one runner and separate workers")
    return counts["worker-no-io"]


def verify(provenance_path: Path) -> dict[str, Any]:
    _require(provenance_path.is_file() and not provenance_path.is_symlink(), "provenance is missing or a symlink")
    record = json.loads(provenance_path.read_bytes())
    _require(isinstance(record, dict) and set(record) == {
        "schema", "kind", "mode", "repository", "tag", "commit", "manifest", "manifestSha256",
        "upstream", "suite", "suiteRole", "adapter", "sourceOnlyExecution", "requiresBrowser",
        "project", "filters", "workers", "toolingSources", "command", "exitCode", "result", "resultSha256",
    }, "provenance contains missing or contradictory fields")
    _require(record.get("schema") == 1 and record.get("kind") == "pinned-upstream-private-fit-to-width-source-only"
             and record.get("mode") in {"candidate", "oracle"} and record.get("suite") == _SUITE
             and record.get("suiteRole") == "implementation-internal-source-only"
             and record.get("adapter") == "upstream-private-source-only-no-pwright-runtime"
             and record.get("requiresBrowser") is False and record.get("project") == _PROJECT
             and record.get("filters") == _ARGS and record.get("workers") == 1 and record.get("exitCode") == 0,
             "provenance scope, selection, or exit code changed")
    manifest = verify_manifest()
    _require(all(record.get(key) == manifest[key] for key in ("repository", "tag", "commit")),
             "provenance upstream revision changed")
    _require(record.get("manifest") == str(_MANIFEST) and record.get("manifestSha256") == _file_sha256(_MANIFEST),
             "provenance manifest path or SHA-256 changed")
    upstream = Path(record["upstream"])
    source = verify_upstream(upstream)
    result = Path(record["result"])
    _require(result.is_file() and not result.is_symlink() and result.with_suffix(".provenance.json") == provenance_path,
             "result and provenance paths differ")
    result_bytes = result.read_bytes()
    _require(record.get("resultSha256") == _sha256(result_bytes), "result SHA-256 mismatch")
    _require(record.get("command") == [
        "node", str(upstream / _RUNNER), "test", f"--config={_CONFIG}",
        "--reporter=line,json", "--workers=1", f"--project={_PROJECT}", *_ARGS,
    ], "runner command changed")
    _require(record.get("toolingSources") == tooling_snapshot(), "tooling selection or SHA-256 changed")
    execution = record.get("sourceOnlyExecution")
    _require(isinstance(execution, dict) and set(execution) == {
        "scope", "candidateRuntimeAttested", "publicApiParityEvidence", "source",
        "reportedTests", "guardReceipt", "guardReceiptSha256", "guardedWorkerProcesses",
    }, "source-only execution record contains missing or contradictory fields")
    _require(execution.get("scope") == "upstream-private-source-only"
             and execution.get("candidateRuntimeAttested") is False
             and execution.get("publicApiParityEvidence") is False
             and execution.get("reportedTests") == 2 and execution.get("source") == source,
             "source-only attestation flags or source record changed")
    receipt = Path(execution["guardReceipt"])
    _require(receipt == result.with_suffix(".guard-receipt.jsonl"), "guard receipt path changed")
    _require(receipt.is_file() and not receipt.is_symlink(), "guard receipt is missing or a symlink")
    receipt_bytes = receipt.read_bytes()
    _require(execution.get("guardReceiptSha256") == _sha256(receipt_bytes), "guard receipt SHA-256 mismatch")
    workers = verify_guard_receipt(receipt, mode=record["mode"], parsed_bytes=receipt_bytes)
    _require(execution.get("guardedWorkerProcesses") == workers, "guarded worker count changed")
    counts = verify_result(result, parsed_bytes=result_bytes)
    return {
        "scope": "upstream-private-source-only",
        "classification": "product-infrastructure",
        "suite": _SUITE,
        "mode": record["mode"],
        "resultSha256": record["resultSha256"],
        "passed": counts["passed"],
        "guardedWorkerProcesses": workers,
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
