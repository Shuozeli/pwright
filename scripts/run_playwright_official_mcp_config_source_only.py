"""Run only pinned upstream-private MCP sandbox tests under a no-I/O guard.

Candidate mode deliberately does not load or attest Pwright. Both modes run
the identical upstream implementation and therefore provide source-only,
implementation-internal evidence rather than public API parity evidence.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
from pathlib import Path

from official_result_destination import prepare_result_destination
from verify_playwright_official_mcp_config_source_only import (
    _ARGS,
    _CONFIG,
    _GUARD,
    _MANIFEST,
    _PROJECTS,
    _RUNNER,
    _SUITE,
    _sha256,
    tooling_snapshot,
    verify_guard_receipt,
    verify_manifest,
    verify_result,
    verify_upstream,
)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("candidate", "oracle"))
    parser.add_argument("upstream", type=Path)
    parser.add_argument("--result", type=Path, required=True)
    args = parser.parse_args()
    for key in (
        "NODE_OPTIONS", "CI", "PWTEST_MODE", "PWPAGE_IMPL", "PWTEST_USE_BIDI_EXPECTATIONS",
        "PW_TAG", "PW_CLOCK", "PWRIGHT_CORE_SOURCE_ONLY_TEST_WORKER",
        "PWRIGHT_NODE_CANDIDATE", "PWRIGHT_NATIVE_PROFILE", "GITHUB_ACTIONS", "GITLAB_CI",
    ):
        if os.environ.get(key):
            parser.error(f"MCP source-only lane forbids inherited {key}")
    if os.environ.get("JENKINS_URL") and os.environ.get("BUILD_URL"):
        parser.error("MCP source-only lane forbids Jenkins Git metadata environment")
    upstream = args.upstream.resolve()
    result = prepare_result_destination(args.result)
    receipt = result.with_suffix(".guard-receipt.jsonl")
    if receipt.is_symlink() or receipt.exists():
        parser.error(f"MCP source-only guard receipt already exists: {receipt}")
    try:
        manifest = verify_manifest()
        source = verify_upstream(upstream)
        tooling = tooling_snapshot()
    except (OSError, TypeError, ValueError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        parser.error(str(error))
    runner = upstream / _RUNNER
    if not runner.is_file() or runner.is_symlink():
        parser.error(f"MCP source-only runner is missing or a symlink: {runner}")
    command = [
        "node", str(runner), "test", f"--config={_CONFIG}",
        "--reporter=line,json", "--workers=1",
        *[f"--project={project}" for project in _PROJECTS], *_ARGS,
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
        "PWRIGHT_CORE_SOURCE_ONLY_SUITE": _SUITE,
        "PWRIGHT_CORE_SOURCE_ONLY_GUARD_RECEIPT": str(receipt),
        "NODE_OPTIONS": f"--require={_GUARD}",
    })
    result.parent.mkdir(parents=True, exist_ok=True)
    receipt.open("x").close()
    print(f"running pinned upstream-private MCP config mode={args.mode} count=8 browser=false", flush=True)
    print(f"result={result}", flush=True)
    completed = subprocess.run(command, cwd=upstream, env=env, check=False)
    if not result.is_file():
        raise SystemExit("MCP source-only runner did not produce JSON evidence")
    if tooling_snapshot() != tooling:
        raise SystemExit("MCP source-only tooling changed during execution")
    try:
        count = verify_result(result)["passed"]
        workers = verify_guard_receipt(receipt)
        post_source = verify_upstream(upstream)
    except (OSError, TypeError, ValueError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        raise SystemExit(f"MCP source-only evidence invalid: {error}") from error
    if post_source != source:
        raise SystemExit("MCP source-only pinned inputs changed during execution")
    suite = manifest["suites"][0]
    provenance = {
        "schema": 1,
        "kind": "pinned-upstream-private-mcp-config-source-only",
        "mode": args.mode,
        "repository": manifest["repository"],
        "tag": manifest["tag"],
        "commit": manifest["commit"],
        "manifest": str(_MANIFEST),
        "manifestSha256": _sha256(_MANIFEST),
        "upstream": str(upstream),
        "suite": _SUITE,
        "suiteRole": suite["role"],
        "adapter": suite["sourceOnlyAdapter"],
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
        "projects": _PROJECTS,
        "filters": _ARGS,
        "workers": 1,
        "toolingSources": tooling,
        "command": command,
        "exitCode": completed.returncode,
        "result": str(result),
        "resultSha256": _sha256(result),
    }
    if tooling_snapshot() != tooling:
        raise SystemExit("MCP source-only tooling changed before provenance write")
    destination = result.with_suffix(".provenance.json")
    destination.write_text(json.dumps(provenance, indent=2, sort_keys=True) + "\n")
    print(f"provenance={destination}", flush=True)
    raise SystemExit(completed.returncode)


if __name__ == "__main__":
    main()
