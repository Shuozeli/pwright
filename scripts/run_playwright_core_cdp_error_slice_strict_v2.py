#!/usr/bin/env python3
"""Capture the pinned Core CDP error shard, including restarted failure workers."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
from pathlib import Path
from typing import Any

import run_playwright_core_cdp_error_slice_strict_v1 as base

ROOT = base.ROOT
SCOPE = base.SCOPE + "-worker-restart-aware-v2"
RESULT_PARENT = ROOT / ".work/official-conformance/results/core-cdp-error-slice-strict-v2"
BASE_RUNNER = Path(base.__file__).resolve()


def selected_tests(report: dict[str, Any]) -> list[dict[str, Any]]:
    selected: list[dict[str, Any]] = []

    def visit(suite: dict[str, Any]) -> None:
        for spec in suite.get("specs", []):
            spec_id = spec["id"]
            assert spec_id in base.CASES
            line, title = base.CASES[spec_id]
            assert spec["file"] == base.REPORT_FILE
            assert (spec["line"], spec["column"], spec["title"]) == (line, 5, title)
            assert len(spec["tests"]) == 1
            test = spec["tests"][0]
            assert test["projectId"] == "chromium-library"
            assert test["expectedStatus"] == "passed"
            assert test["status"] in {"expected", "unexpected"}
            assert len(test["results"]) == 1
            attempt = test["results"][0]
            assert attempt["retry"] == 0
            worker_index = attempt["workerIndex"]
            assert isinstance(worker_index, int) and worker_index >= 0
            assert attempt["status"] in {"passed", "failed"}
            passed = attempt["status"] == "passed"
            assert test["status"] == ("expected" if passed else "unexpected")
            assert spec["ok"] == passed
            errors = attempt["errors"]
            assert isinstance(errors, list) and (not errors) == passed
            selected.append(
                {
                    "title": title,
                    "line": line,
                    "specId": spec_id,
                    "status": test["status"],
                    "attempts": [
                        {
                            "retry": attempt["retry"],
                            "workerIndex": worker_index,
                            "status": attempt["status"],
                        }
                    ],
                    "errors": [error.get("message", "") for error in errors],
                }
            )
        for child in suite.get("suites", []):
            visit(child)

    assert report["errors"] == []
    for suite in report["suites"]:
        visit(suite)
    assert len(selected) == len(base.CASES)
    assert {item["specId"] for item in selected} == set(base.CASES)
    selected.sort(key=lambda item: item["line"])
    stats = report["stats"]
    passed = sum(item["status"] == "expected" for item in selected)
    assert stats["expected"] == passed
    assert stats["unexpected"] == len(base.CASES) - passed
    assert stats["skipped"] == 0 and stats["flaky"] == 0
    return selected


def checked_receipts(path: Path, selected: list[dict[str, Any]], host: str) -> None:
    lines = path.read_text().splitlines()
    assert lines and all(lines)
    records = [json.loads(line) for line in lines]
    worker_indices = {str(item["attempts"][0]["workerIndex"]) for item in selected}
    assert {
        record["workerIndex"] for record in records if record["workerIndex"] is not None
    } == worker_indices
    assert len({record["workerIndex"] for record in records}) == len(records)
    assert all(
        set(record)
        == {
            "pid",
            "workerIndex",
            "sourceSha256",
            "adaptedSha256",
            "fixtureHost",
            "replacements",
        }
        for record in records
    )
    assert all(
        isinstance(record["pid"], int)
        and record["pid"] > 0
        and (record["workerIndex"] is None or record["workerIndex"] in worker_indices)
        and record["sourceSha256"] == base.SOURCE_SHA256
        and record["adaptedSha256"] == base.adapted_source_sha256(host)
        and record["fixtureHost"] == host
        and record["replacements"] == {"websocket": 1, "serverPrefixCases": 0}
        for record in records
    )


def run(mode: str, result: Path) -> dict[str, Any]:
    forbidden = (
        "NODE_OPTIONS",
        "PWTEST_MODE",
        "PWPAGE_IMPL",
        "PWTEST_USE_BIDI_EXPECTATIONS",
        "PWRIGHT_CANDIDATE_CDP_ENDPOINT",
        "PWRIGHT_ORACLE_CDP_ENDPOINT",
        "PWRIGHT_NATIVE_PROFILE",
        "PWRIGHT_NODE_CANDIDATE",
        "PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM",
        "PWRIGHT_OFFICIAL_SERVER_HOST",
        "PWRIGHT_CORE_CDP_ERROR_SLICE_GUARD_RECEIPT",
    )
    if any(os.environ.get(key) for key in forbidden):
        raise ValueError("strict CDP error slice rejects inherited runtime overrides")
    base.check_manifest()
    artifact = ROOT / "packages/pwright/native" / base.PROFILE / "pwright.node"
    if mode == "candidate" and (
        not artifact.is_file() or base.sha256(artifact) != base.ARTIFACT_SHA256
    ):
        raise ValueError("pinned candidate UA-profile artifact is missing or changed")
    result = result.absolute()
    if result.parent != RESULT_PARENT or result.suffix != ".json":
        raise ValueError("strict CDP error slice result must be in its v2 result directory")
    RESULT_PARENT.mkdir(parents=True, exist_ok=True)
    outputs = [
        result,
        result.with_suffix(".provenance.json"),
        result.with_suffix(".strict.json"),
        result.with_suffix(".guard-receipt.jsonl"),
        result.with_suffix(".log"),
    ]
    if any(path.exists() or path.is_symlink() for path in outputs):
        raise ValueError("strict CDP error slice result destination is not empty")
    receipt = result.with_suffix(".guard-receipt.jsonl")
    receipt.open("x").close()
    inputs = {
        str(path.relative_to(ROOT)): base.sha256(path)
        for path in (
            base.MANIFEST,
            base.PRELOAD,
            base.COMMON_PRELOAD,
            BASE_RUNNER,
            Path(__file__).resolve(),
        )
    }
    env = os.environ.copy()
    env["NODE_OPTIONS"] = f"--require={base.PRELOAD}"
    env["PWRIGHT_CORE_CDP_ERROR_SLICE_GUARD_RECEIPT"] = str(receipt)
    command = [
        "uv",
        "run",
        "--with",
        "pytest",
        str(ROOT / "scripts/run_playwright_official_conformance.py"),
        mode,
        str(base.UPSTREAM),
        "--suite",
        base.SUITE,
        "--manifest",
        str(base.MANIFEST),
        "--workers",
        "1",
        "--result",
        str(result),
    ]
    if mode == "candidate":
        command += ["--candidate-profile", base.PROFILE]
    completed = subprocess.run(
        command, check=False, cwd=ROOT, env=env, capture_output=True, text=True
    )
    log = completed.stdout + completed.stderr
    log_path = result.with_suffix(".log")
    log_path.write_text(log)
    assert all(base.sha256(ROOT / name) == digest for name, digest in inputs.items())
    base.check_manifest()
    provenance = result.with_suffix(".provenance.json")
    if not result.is_file() or not provenance.is_file():
        raise RuntimeError(
            "official runner produced no result/provenance "
            f"(exit {completed.returncode}): {log[-3000:]}"
        )
    selected = selected_tests(json.loads(result.read_text()))
    assert completed.returncode == (
        0 if all(item["status"] == "expected" for item in selected) else 1
    )
    assert "[pwright-candidate]" in log if mode == "candidate" else "[pwright-candidate]" not in log
    provenance_data = json.loads(provenance.read_text())
    checked_receipts(receipt, selected, provenance_data["fixtureHost"])
    strict = {
        "schema": 3,
        "scope": SCOPE,
        "mode": mode,
        "upstreamCommit": base.COMMIT,
        "upstreamSourceSha256": base.SOURCE_SHA256,
        "serverFixtureSha256": base.SERVER_FIXTURE_SHA256,
        "commonPreloadSha256": base.COMMON_PRELOAD_SHA256,
        "inputs": inputs,
        "candidateProfile": base.PROFILE if mode == "candidate" else None,
        "candidateArtifactSha256": base.ARTIFACT_SHA256 if mode == "candidate" else None,
        "result": str(result),
        "resultSha256": base.sha256(result),
        "provenanceSha256": base.sha256(provenance),
        "guardReceiptSha256": base.sha256(receipt),
        "logSha256": base.sha256(log_path),
        "exitCode": completed.returncode,
        "selected": selected,
    }
    base.write_json_atomic(result.with_suffix(".strict.json"), strict)
    return strict


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("candidate", "oracle"))
    parser.add_argument("--result", type=Path, required=True)
    args = parser.parse_args()
    try:
        strict = run(args.mode, args.result)
    except (AssertionError, OSError, ValueError, RuntimeError, json.JSONDecodeError) as error:
        parser.error(str(error))
    print(
        json.dumps(
            {
                "mode": strict["mode"],
                "exitCode": strict["exitCode"],
                "selected": strict["selected"],
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
