"""Verify the eight pinned, upstream-private MCP sandbox-configuration tests.

This source-only receipt says nothing about Pwright runtime or public API parity.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path
from typing import Any

_ROOT = Path(__file__).parents[1]
_MANIFEST = _ROOT / "tests/official/playwright-core-mcp-config-source-only-v1-manifest.json"
_GUARD = _ROOT / "tests/official/playwright-core-mcp-config-source-only-v1-guard.cjs"
_SHARED_GUARD = _ROOT / "tests/official/playwright-core-internal-source-only-guard.cjs"
_RUNNER = "packages/playwright/cli.js"
_CONFIG = "tests/mcp/playwright.config.ts"
_ARGS = ["tests/mcp/config.spec.ts:82", "tests/mcp/config.spec.ts:87"]
_PROJECTS = ["chrome", "chromium", "firefox", "webkit"]
_SUITE = "core462-private-mcp-config-sandbox"
_CASE = "private-mcp-config-sandbox-v1"
_COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
_SOURCE_SHA256 = "f50fe7dec188ab2957f19b4681df621181821e296df251d940747e7fd297b0a8"
_IDS_SHA256 = "7be17705f68f69aca3afec656c3fe4142c71a49583c59a31b296d9c6a296b00c"
_SHARED_GUARD_SHA256 = "f80debd1433efa271ddd063af799332aac4cefc6386876b16775d38d448fde6a"
_SOURCE_FILES = [
    "package-lock.json",
    _RUNNER,
    "packages/playwright/lib/common/process.js",
    "packages/playwright/lib/mcp/browser/config.js",
    "packages/playwright/src/mcp/browser/config.ts",
    "tests/config/serverFixtures.ts",
    "tests/mcp/config.spec.ts",
    "tests/mcp/fixtures.ts",
    _CONFIG,
]
_INVOCATIONS = [
    (82, "chrome", "9196cb1f5e62547dc5c9-69a718c5f51fb8dc7faa"),
    (82, "chromium", "9196cb1f5e62547dc5c9-fd4a6ec16bc60a1e0666"),
    (82, "firefox", "9196cb1f5e62547dc5c9-f6eabc76476719c34bf5"),
    (82, "webkit", "9196cb1f5e62547dc5c9-ecaf9642b768357b001c"),
    (87, "chrome", "9196cb1f5e62547dc5c9-95d0ab28bd553b34dec5"),
    (87, "chromium", "9196cb1f5e62547dc5c9-28c2d94b2411e26d5592"),
    (87, "firefox", "9196cb1f5e62547dc5c9-bd861023e2299e369470"),
    (87, "webkit", "9196cb1f5e62547dc5c9-a52d59b5d1439fd414fb"),
]
_TITLES = {
    82: "should enable sandbox by default (no --no-sandbox flag)",
    87: "should disable sandbox when --no-sandbox flag is passed",
}
_TOOLING = [
    _ROOT / "scripts/official_result_destination.py",
    _ROOT / "scripts/run_playwright_official_mcp_config_source_only.py",
    _ROOT / "scripts/verify_playwright_official_mcp_config_source_only.py",
    _MANIFEST,
    _GUARD,
    _SHARED_GUARD,
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


def tooling_snapshot() -> list[dict[str, str]]:
    _require(all(file.is_file() and not file.is_symlink() for file in _TOOLING), "source-only tooling is missing or a symlink")
    return [{"path": str(file), "sha256": _sha256(file)} for file in _TOOLING]


def _source_digest(upstream: Path) -> str:
    pairs: list[list[str]] = []
    for relative in sorted(_SOURCE_FILES):
        file = upstream / relative
        _require(file.is_file() and not file.is_symlink(), f"upstream source is missing or a symlink: {relative}")
        pairs.append([relative, _sha256(file)])
    return hashlib.sha256(json.dumps(pairs, separators=(",", ":")).encode()).hexdigest()


def verify_upstream(upstream: Path) -> dict[str, Any]:
    _require(upstream.is_dir() and upstream.resolve() == upstream, "upstream path is missing or unresolved")
    _require(_git(upstream, "rev-parse", "HEAD") == _COMMIT, "upstream commit changed")
    _require(not _git(upstream, "status", "--porcelain", "--untracked-files=no"), "upstream tracked sources changed")
    _require(not (upstream / ".env").exists(), "upstream .env override is present")
    _require(_sha256(_SHARED_GUARD) == _SHARED_GUARD_SHA256, "shared no-I/O guard changed")
    _require(_source_digest(upstream) == _SOURCE_SHA256, "upstream source SHA-256 changed")
    completed = subprocess.run(
        ["node", str(_GUARD), "--check", str(_MANIFEST), _SUITE, str(upstream)],
        capture_output=True,
        text=True,
        check=True,
    )
    source = json.loads(completed.stdout)
    _require(source == {
        "case": _CASE,
        "sourceFiles": sorted(_SOURCE_FILES),
        "sourceSha256": _SOURCE_SHA256,
        "expectedInvocationIdsSha256": _IDS_SHA256,
        "expectedProjectIds": _PROJECTS,
        "expectedTestsPerProject": 2,
        "expectedInvocations": [
            {"line": line, "projectId": project, "specId": spec_id}
            for line, project, spec_id in _INVOCATIONS
        ],
    }, "source guard returned an unpinned selection")
    return source


def verify_manifest() -> dict[str, Any]:
    _require(_MANIFEST.is_file() and not _MANIFEST.is_symlink(), "manifest is missing or a symlink")
    manifest = json.loads(_MANIFEST.read_text())
    _require(isinstance(manifest, dict) and set(manifest) == {"schema", "repository", "tag", "commit", "suites"}, "manifest shape changed")
    _require(
        manifest["schema"] == 1 and manifest["repository"] == "https://github.com/microsoft/playwright.git"
        and manifest["tag"] == "v1.58.0" and manifest["commit"] == _COMMIT
        and isinstance(manifest["suites"], list) and len(manifest["suites"]) == 1,
        "manifest upstream revision changed",
    )
    suite = manifest["suites"][0]
    _require(isinstance(suite, dict) and set(suite) == {
        "id", "role", "runner", "config", "args", "sourceOnlyAdapter", "requiresBrowser",
        "fixtureMode", "sourceOnlyCase", "upstreamSourceSetSha256", "expectedInvocationIdsSha256",
        "expectedTestsPerProject", "expectedFiles", "expectedProjectIds", "expectedInvocations",
    }, "manifest suite shape changed")
    _require(
        suite["id"] == _SUITE and suite["role"] == "implementation-internal-source-only"
        and suite["runner"] == _RUNNER and suite["config"] == _CONFIG and suite["args"] == _ARGS
        and suite["sourceOnlyAdapter"] == "upstream-private-source-only-no-pwright-runtime"
        and suite["requiresBrowser"] is False and suite["fixtureMode"] == "pinned-private-source-only-no-io"
        and suite["sourceOnlyCase"] == _CASE and suite["upstreamSourceSetSha256"] == _SOURCE_SHA256
        and suite["expectedInvocationIdsSha256"] == _IDS_SHA256
        and suite["expectedTestsPerProject"] == 2 and suite["expectedFiles"] == 1
        and suite["expectedProjectIds"] == _PROJECTS
        and suite["expectedInvocations"] == [
            {"line": line, "projectId": project, "specId": spec_id}
            for line, project, spec_id in _INVOCATIONS
        ],
        "manifest suite selection changed",
    )
    return manifest


def verify_result(result: Path) -> dict[str, int]:
    report = json.loads(result.read_text())
    _require(isinstance(report, dict) and report.get("errors") == [], "result has runner errors")
    _require(isinstance(report.get("suites"), list), "result has no suites")
    stats = report.get("stats")
    _require(isinstance(stats, dict) and all(stats.get(key) == value for key, value in {
        "expected": 8, "skipped": 0, "unexpected": 0, "flaky": 0,
    }.items()), "result statistics are not exactly eight passes")
    found: list[tuple[int, str, str]] = []

    def visit(raw_suite: object) -> None:
        _require(isinstance(raw_suite, dict), "result has malformed suite")
        specs = raw_suite.get("specs", [])
        children = raw_suite.get("suites", [])
        _require(isinstance(specs, list) and isinstance(children, list), "result suite lists are malformed")
        for spec in specs:
            _require(isinstance(spec, dict), "result has malformed spec")
            line, spec_id, tests = spec.get("line"), spec.get("id"), spec.get("tests")
            _require(
                spec.get("file") == "mcp/config.spec.ts" and type(line) is int and line in _TITLES
                and spec.get("title") == _TITLES[line] and isinstance(spec_id, str)
                and isinstance(tests, list) and len(tests) == 1,
                "result contains an unpinned file, line, or title",
            )
            test = tests[0]
            _require(isinstance(test, dict), "result has malformed test")
            attempts = test.get("results")
            _require(
                test.get("projectId") in _PROJECTS and test.get("expectedStatus") == "passed"
                and test.get("status") == "expected" and isinstance(attempts, list)
                and len(attempts) == 1 and isinstance(attempts[0], dict)
                and attempts[0].get("status") == "passed" and attempts[0].get("retry") == 0,
                "result contains a failure, skip, or retried invocation",
            )
            found.append((line, test["projectId"], spec_id))
        for child in children:
            visit(child)

    for raw_suite in report["suites"]:
        visit(raw_suite)
    _require(len(found) == 8 and set(found) == set(_INVOCATIONS), "result changed exact eight invocation IDs or projects")
    digest = hashlib.sha256(
        "".join(f"playwright-test\t{spec_id}\t{project}\n" for _, project, spec_id in sorted(found, key=lambda item: (item[2], item[1]))).encode()
    ).hexdigest()
    _require(digest == _IDS_SHA256, "result changed invocation digest")
    return {"passed": 8, "lines": 2, "projects": 4}


def verify_guard_receipt(receipt: Path) -> int:
    _require(receipt.is_file() and not receipt.is_symlink(), "guard receipt is missing or a symlink")
    records = [json.loads(line) for line in receipt.read_text().splitlines()]
    _require(len(records) >= 2, "guard receipt did not cover runner and worker")
    seen: set[tuple[int, int]] = set()
    runner_pids: set[int] = set()
    worker_pids: set[int] = set()
    for item in records:
        _require(isinstance(item, dict) and set(item) == {"kind", "pid", "threadId", "suite", "case"}, "guard receipt shape changed")
        kind, pid, thread = item["kind"], item["pid"], item["threadId"]
        _require(
            kind in {"runner-no-io", "worker-no-io", "loader-thread-no-io"}
            and item["suite"] == _SUITE and item["case"] == _CASE
            and type(pid) is int and pid > 0 and type(thread) is int and thread >= 0
            and (kind == "loader-thread-no-io") == (thread > 0)
            and (pid, thread) not in seen,
            "guard receipt has an unpinned or duplicate process",
        )
        seen.add((pid, thread))
        if kind == "runner-no-io":
            runner_pids.add(pid)
        if kind == "worker-no-io":
            worker_pids.add(pid)
    _require(len(runner_pids) == 1 and worker_pids and runner_pids.isdisjoint(worker_pids), "guard receipt lacks separate runner and worker")
    _require(all(pid in runner_pids | worker_pids for pid, _ in seen), "guard receipt has unaccounted loader thread")
    return len(worker_pids)


def verify(provenance_path: Path) -> dict[str, Any]:
    _require(provenance_path.is_file() and not provenance_path.is_symlink(), "provenance is missing or a symlink")
    record = json.loads(provenance_path.read_text())
    _require(isinstance(record, dict) and set(record) == {
        "schema", "kind", "mode", "repository", "tag", "commit", "manifest", "manifestSha256",
        "upstream", "suite", "suiteRole", "adapter", "sourceOnlyExecution", "requiresBrowser",
        "projects", "filters", "workers", "toolingSources", "command", "exitCode", "result", "resultSha256",
    }, "provenance shape changed")
    _require(
        record["schema"] == 1 and record["kind"] == "pinned-upstream-private-mcp-config-source-only"
        and record["mode"] in {"candidate", "oracle"} and record["suite"] == _SUITE
        and record["suiteRole"] == "implementation-internal-source-only"
        and record["adapter"] == "upstream-private-source-only-no-pwright-runtime"
        and record["requiresBrowser"] is False and record["projects"] == _PROJECTS
        and record["filters"] == _ARGS and record["workers"] == 1 and record["exitCode"] == 0,
        "provenance scope, selection, or exit code changed",
    )
    manifest = verify_manifest()
    _require(all(record[key] == manifest[key] for key in ("repository", "tag", "commit")), "provenance upstream revision changed")
    _require(record["manifest"] == str(_MANIFEST) and record["manifestSha256"] == _sha256(_MANIFEST), "manifest provenance changed")
    upstream = Path(record["upstream"])
    source = verify_upstream(upstream)
    result = Path(record["result"])
    _require(result.is_file() and not result.is_symlink(), "result is missing or a symlink")
    _require(result.with_suffix(".provenance.json") == provenance_path and record["resultSha256"] == _sha256(result), "result provenance changed")
    command = [
        "node", str(upstream / _RUNNER), "test", f"--config={_CONFIG}",
        "--reporter=line,json", "--workers=1", *[f"--project={project}" for project in _PROJECTS], *_ARGS,
    ]
    _require(record["command"] == command, "runner command changed")
    _require(record["toolingSources"] == tooling_snapshot(), "tooling SHA-256 or selection changed")
    execution = record["sourceOnlyExecution"]
    _require(isinstance(execution, dict) and set(execution) == {
        "scope", "candidateRuntimeAttested", "publicApiParityEvidence", "source", "reportedTests",
        "guardReceipt", "guardReceiptSha256", "guardedWorkerProcesses",
    }, "source-only execution shape changed")
    _require(
        execution["scope"] == "upstream-private-source-only"
        and execution["candidateRuntimeAttested"] is False and execution["publicApiParityEvidence"] is False
        and execution["source"] == source and execution["reportedTests"] == 8,
        "source-only attestation flags or source record changed",
    )
    receipt = Path(execution["guardReceipt"])
    _require(receipt == result.with_suffix(".guard-receipt.jsonl"), "guard receipt path changed")
    _require(execution["guardReceiptSha256"] == _sha256(receipt), "guard receipt SHA-256 changed")
    workers = verify_guard_receipt(receipt)
    _require(execution["guardedWorkerProcesses"] == workers, "guarded worker count changed")
    counts = verify_result(result)
    return {
        "scope": "upstream-private-source-only", "classification": "implementation-internal",
        "suite": _SUITE, "mode": record["mode"], "resultSha256": record["resultSha256"],
        **counts, "guardedWorkerProcesses": workers,
        "candidateRuntimeAttested": False, "publicApiParityEvidence": False,
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
