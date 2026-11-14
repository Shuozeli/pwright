#!/usr/bin/env python3
"""Inventory every expanded test in a pinned Playwright core checkout."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
from collections import Counter
from pathlib import Path
from typing import Any, TypedDict, cast


class SourceInventory(TypedDict):
    specTs: int
    specTsx: int
    specJs: int
    total: int
    testsTree: int
    examplesTree: int


class SuiteRequired(TypedDict):
    id: str
    role: str
    runner: str
    expectedTests: int
    expectedFiles: int


class Suite(SuiteRequired, total=False):
    config: str
    args: list[str]


class NonCollectedSpec(TypedDict):
    path: str
    classification: str
    parentSuite: str
    reason: str


class Manifest(TypedDict):
    schema: int
    repository: str
    tag: str
    commit: str
    sourceInventory: SourceInventory
    nonCollectedSpecs: list[NonCollectedSpec]
    suites: list[Suite]


def _git(upstream: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(upstream), *args],
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip()


def _tracked_specs(upstream: Path) -> list[str]:
    result = _git(
        upstream,
        "ls-files",
        "*spec.ts",
        "*spec.tsx",
        "*spec.js",
    )
    return sorted(line for line in result.splitlines() if line)


def _verify_source(upstream: Path, manifest: Manifest) -> list[str]:
    if _git(upstream, "rev-parse", "HEAD") != manifest["commit"]:
        raise SystemExit("Playwright core checkout does not match the pinned commit")
    if _git(upstream, "status", "--porcelain", "--untracked-files=no"):
        raise SystemExit("Playwright core checkout has tracked source changes")

    specs = _tracked_specs(upstream)
    expected = manifest["sourceInventory"]
    observed = {
        "specTs": sum(path.endswith(".spec.ts") for path in specs),
        "specTsx": sum(path.endswith(".spec.tsx") for path in specs),
        "specJs": sum(path.endswith(".spec.js") for path in specs),
        "total": len(specs),
        "testsTree": sum(path.startswith("tests/") for path in specs),
        "examplesTree": sum(path.startswith("examples/") for path in specs),
    }
    if observed != expected:
        raise SystemExit(
            "tracked spec inventory differs from the manifest: "
            f"expected={expected} observed={observed}"
        )
    return specs


def _classify_source(upstream: Path, relative: str) -> tuple[str, list[str]]:
    source = (upstream / relative).read_text(errors="replace")
    runtime_source = re.sub(r"\bimport\s+type\b.*?;", "", source, flags=re.DOTALL)
    reasons: list[str] = []
    specifiers = re.findall(
        r"(?:\bfrom\s*|\brequire\(\s*)['\"]([^'\"]+)['\"]",
        runtime_source,
    )
    for specifier in specifiers:
        if specifier.startswith(
            ("playwright-core/lib/", "playwright/lib/", "@playwright/test/lib/")
        ):
            reasons.append(f"runtime-internal-import:{specifier}")
            continue
        if specifier.startswith("."):
            resolved = (upstream / relative).parent.joinpath(specifier).resolve()
            try:
                resolved_relative = resolved.relative_to(upstream).as_posix()
            except ValueError:
                continue
            if resolved_relative.startswith("packages/"):
                reasons.append(f"runtime-source-import:{resolved_relative}")
    if re.search(
        r"\btoImpl(?:InWorkerScope)?\b|\._(?:connection|channel|initializer|guid)\b",
        runtime_source,
    ):
        reasons.append("private-client-access")
    reasons = sorted(set(reasons))
    if reasons:
        return "implementation-internal", reasons
    if relative.startswith(
        ("tests/library/", "tests/page/", "tests/components/", "examples/")
    ):
        return "public-api-candidate", []
    return "product-infrastructure", []


def _parse_json_report(stdout: str, suite_id: str) -> dict[str, Any]:
    start = stdout.find("{")
    if start == -1:
        raise SystemExit(f"suite {suite_id} did not emit a JSON report")
    try:
        return cast(dict[str, Any], json.loads(stdout[start:]))
    except json.JSONDecodeError as error:
        raise SystemExit(f"suite {suite_id} emitted invalid JSON: {error}") from error


def _collect_suite(
    upstream: Path,
    suite: Suite,
    source_classifications: dict[str, tuple[str, list[str]]],
) -> list[dict[str, object]]:
    command = [
        "node",
        str(upstream / suite["runner"]),
        "test",
        "--list",
        "--reporter=json",
    ]
    if config := suite.get("config"):
        command.append(f"--config={config}")
    command.extend(suite.get("args", []))
    result = subprocess.run(
        command,
        cwd=upstream,
        capture_output=True,
        text=True,
    )
    if result.returncode:
        detail = result.stderr.strip() or result.stdout[-4000:].strip()
        raise SystemExit(f"suite {suite['id']} collection failed:\n{detail}")

    report = _parse_json_report(result.stdout, suite["id"])
    if errors := report.get("errors"):
        raise SystemExit(f"suite {suite['id']} reported collection errors: {errors}")
    root_dir = Path(report["config"]["rootDir"])

    records: list[dict[str, object]] = []

    def walk(node: dict[str, Any], title_path: tuple[str, ...]) -> None:
        node_title = node.get("title")
        next_path = title_path + ((node_title,) if node_title else ())
        for spec in node.get("specs", []):
            source_file = (root_dir / spec["file"]).resolve()
            try:
                source_relative = source_file.relative_to(upstream).as_posix()
            except ValueError as error:
                raise SystemExit(
                    f"suite {suite['id']} collected a file outside upstream: "
                    f"{source_file}"
                ) from error
            for test in spec.get("tests", []):
                records.append(
                    {
                        "suite": suite["id"],
                        "role": suite["role"],
                        "specId": spec["id"],
                        "projectId": test["projectId"],
                        "projectName": test["projectName"],
                        "file": spec["file"],
                        "sourceFile": source_relative,
                        "sourceClassification": source_classifications[source_relative][
                            0
                        ],
                        "classificationReasons": source_classifications[
                            source_relative
                        ][1],
                        "line": spec["line"],
                        "column": spec["column"],
                        "titlePath": [*next_path, spec["title"]],
                        "expectedStatus": test["expectedStatus"],
                    }
                )
        for child in node.get("suites", []):
            walk(child, next_path)

    for root_suite in report.get("suites", []):
        walk(root_suite, ())

    files = {cast(str, record["file"]) for record in records}
    if len(records) != suite["expectedTests"]:
        raise SystemExit(
            f"suite {suite['id']} test count changed: "
            f"expected={suite['expectedTests']} observed={len(records)}"
        )
    if len(files) != suite["expectedFiles"]:
        raise SystemExit(
            f"suite {suite['id']} file count changed: "
            f"expected={suite['expectedFiles']} observed={len(files)}"
        )
    return records


def _stable_json_line(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def main() -> None:
    root = Path(__file__).parents[1]
    parser = argparse.ArgumentParser()
    parser.add_argument("upstream", type=Path)
    parser.add_argument(
        "--manifest",
        type=Path,
        default=root / "tests/official/playwright-core-manifest.json",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=root / ".work/official-conformance/playwright-core-v1.58.0.jsonl",
    )
    args = parser.parse_args()

    upstream = args.upstream.resolve()
    manifest = cast(Manifest, json.loads(args.manifest.read_text()))
    if manifest["schema"] != 1:
        raise SystemExit(f"unsupported manifest schema: {manifest['schema']}")
    specs = _verify_source(upstream, manifest)
    source_classifications = {
        source: _classify_source(upstream, source) for source in specs
    }

    records: list[dict[str, object]] = []
    suite_summaries: list[dict[str, object]] = []
    for suite in manifest["suites"]:
        suite_records = _collect_suite(upstream, suite, source_classifications)
        records.extend(suite_records)
        suite_summaries.append(
            {
                "id": suite["id"],
                "role": suite["role"],
                "tests": len(suite_records),
                "files": len({record["file"] for record in suite_records}),
            }
        )
        print(
            f"collected {suite['id']}: {len(suite_records)} tests / "
            f"{suite['expectedFiles']} files",
            flush=True,
        )

    records.sort(
        key=lambda record: (
            cast(str, record["suite"]),
            cast(str, record["file"]),
            cast(int, record["line"]),
            cast(str, record["projectId"]),
            cast(str, record["specId"]),
        )
    )
    covered_specs = {cast(str, record["sourceFile"]) for record in records}
    accounted_non_collected = {entry["path"] for entry in manifest["nonCollectedSpecs"]}
    if len(accounted_non_collected) != len(manifest["nonCollectedSpecs"]):
        raise SystemExit("non-collected spec paths must be unique")
    missing_specs = sorted(set(specs) - covered_specs - accounted_non_collected)
    unexpected_specs = sorted(covered_specs - set(specs))
    invalid_non_collected = sorted(accounted_non_collected - set(specs))
    collected_exclusions = sorted(accounted_non_collected & covered_specs)
    if (
        missing_specs
        or unexpected_specs
        or invalid_non_collected
        or collected_exclusions
    ):
        raise SystemExit(
            "collected source coverage differs from the tracked spec inventory: "
            f"missing={missing_specs} unexpected={unexpected_specs} "
            f"invalidNonCollected={invalid_non_collected} "
            f"collectedExclusions={collected_exclusions}"
        )
    payload = b"".join(_stable_json_line(record) for record in records)
    digest = hashlib.sha256(payload).hexdigest()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(payload)

    summary = {
        "schema": 1,
        "repository": manifest["repository"],
        "tag": manifest["tag"],
        "commit": manifest["commit"],
        "trackedSpecFiles": len(specs),
        "coveredSpecFiles": len(covered_specs),
        "nonCollectedSpecFiles": manifest["nonCollectedSpecs"],
        "expandedTestInvocations": len(records),
        "sourceClassificationCounts": dict(
            sorted(
                Counter(record["sourceClassification"] for record in records).items()
            )
        ),
        "ledgerSha256": digest,
        "ledger": str(args.output),
        "suites": suite_summaries,
    }
    summary_path = args.output.with_suffix(".summary.json")
    summary_path.write_text(json.dumps(summary, indent=2) + "\n")
    print(
        f"official Playwright core inventory: {len(specs)} tracked specs / "
        f"{len(records)} expanded invocations / sha256={digest}"
    )
    print(f"ledger={args.output}")
    print(f"summary={summary_path}")


if __name__ == "__main__":
    main()
