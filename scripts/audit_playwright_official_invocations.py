#!/usr/bin/env python3
"""Classify every pinned Playwright Core invocation without changing outcomes."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
from collections import Counter, defaultdict
from collections.abc import Iterable
from pathlib import Path
from typing import Any, TypedDict, cast

Key = tuple[str, str, str]
PRIVATE_TEST_HOOK = re.compile(r"\b(__testHook[A-Za-z0-9_]*)\b")
PRIVATE_CLIENT_ACCESS = re.compile(
    r"\btoImpl(?:InWorkerScope)?\b|"
    r"\._(?:connection|channel|initializer|guid|playwright|defaultLaunchOptions|tracing)\b"
)
PUBLIC_API_SOURCE_PREFIXES = (
    "tests/library/",
    "tests/page/",
    "tests/components/",
    "examples/",
)
LOCAL_IMPLEMENTATIONS_UNDER_TEST = (
    ("tests/image_tools/", "tests/image_tools"),
    ("tests/library/events/", "tests/library/events/utils"),
    (
        "tests/library/component-parser.spec.ts",
        "packages/playwright-core/lib/utils/isomorphic/selectorParser",
    ),
    (
        "tests/library/css-parser.spec.ts",
        "packages/playwright-core/lib/utils/isomorphic/cssParser",
    ),
    ("tests/library/debug-controller.spec.ts", "debug-controller-internals"),
    ("tests/library/locator-generator.spec.ts", "locator-generator-internals"),
    (
        "tests/library/proxy-pattern.spec.ts",
        "packages/playwright-core/lib/server/utils/socksProxy",
    ),
    ("tests/library/unit/", "tests/library/unit"),
    ("tests/page/page-leaks.spec.ts", "page-lifecycle-leak-internals"),
)
DIRECT_PRIVATE_IMPLEMENTATIONS_UNDER_TEST = (
    (
        "tests/library/browsercontext-viewport.spec.ts",
        r"\bdeviceDescriptors\b",
        "packages/playwright-core/lib/server/deviceDescriptors",
    ),
    (
        "tests/library/browsertype-launch-selenium.spec.ts",
        r"(?<![\w.])start\s*\(",
        "packages/playwright-core/lib/outofprocess",
    ),
    (
        "tests/page/interception.spec.ts",
        r"\b(?:globToRegexPattern|urlMatches)\s*\(",
        "packages/playwright-core/lib/utils/isomorphic/urlMatch",
    ),
    (
        "tests/page/page-aria-snapshot-ai.spec.ts",
        r"\basLocator\s*\(",
        "playwright-core/lib/utils",
    ),
)
PRODUCT_INFRASTRUCTURE_ROLES = (("official-example", "example-application-validation"),)
PRODUCT_INFRASTRUCTURE_SOURCES = (
    ("tests/stress/", "stress-harness"),
    ("tests/playwright-test/", "playwright-test-runner"),
    ("tests/mcp/", "mcp-product"),
    ("tests/installation/", "installation-packaging"),
    ("tests/electron/", "electron-integration"),
    ("tests/android/", "android-integration"),
)
SHARED_PRIVATE_MACHINERY = (
    (
        "tests/library/channels.spec.ts",
        "expectScopeState",
        "shared-private-fixture:expectScopeState",
    ),
    (
        "tests/library/page-event-crash.spec.ts",
        "crash",
        "shared-private-fixture:crash",
    ),
)


class Manifest(TypedDict):
    schema: int
    repository: str
    tag: str
    commit: str


def _stable_json_line(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def _file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _git(upstream: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(upstream), *args],
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip()


def _verify_upstream(upstream: Path, manifest: Manifest) -> None:
    if _git(upstream, "rev-parse", "HEAD") != manifest["commit"]:
        raise SystemExit("Playwright core checkout does not match the pinned commit")
    if _git(upstream, "status", "--porcelain", "--untracked-files=no"):
        raise SystemExit("Playwright core checkout has tracked source changes")


def _verify_inventory(path: Path, expected_commit: str) -> tuple[int, str]:
    summary_path = path.with_suffix(".summary.json")
    if not summary_path.is_file():
        raise SystemExit(f"inventory summary is missing: {summary_path}")
    summary = cast(dict[str, Any], json.loads(summary_path.read_text()))
    if summary.get("commit") != expected_commit:
        raise SystemExit("inventory summary does not match the pinned commit")
    digest = _file_sha256(path)
    if summary.get("ledgerSha256") != digest:
        raise SystemExit("inventory bytes do not match the recorded SHA-256")
    expected_records = summary.get("expandedTestInvocations")
    if not isinstance(expected_records, int) or expected_records < 1:
        raise SystemExit("inventory summary has no positive invocation count")
    return expected_records, digest


def _verify_outcomes(path: Path, expected_records: int) -> str:
    summary_path = path.with_suffix(".summary.json")
    if not summary_path.is_file():
        raise SystemExit(f"outcome summary is missing: {summary_path}")
    summary = cast(dict[str, Any], json.loads(summary_path.read_text()))
    if summary.get("records") != expected_records:
        raise SystemExit("outcome ledger count does not match the inventory")
    digest = _file_sha256(path)
    if summary.get("ledgerSha256") != digest:
        raise SystemExit("outcome ledger bytes do not match the recorded SHA-256")
    return digest


def _records(path: Path) -> Iterable[dict[str, object]]:
    with path.open() as stream:
        for line_number, line in enumerate(stream, start=1):
            try:
                yield cast(dict[str, object], json.loads(line))
            except json.JSONDecodeError as error:
                raise SystemExit(
                    f"inventory line {line_number} is not valid JSON: {error}"
                ) from error


def _source_starts(
    inventory: Path, expected_records: int
) -> dict[str, tuple[int, ...]]:
    starts: defaultdict[str, set[int]] = defaultdict(set)
    keys: set[Key] = set()
    observed = 0
    for record in _records(inventory):
        try:
            key = (
                cast(str, record["suite"]),
                cast(str, record["specId"]),
                cast(str, record["projectId"]),
            )
            source_file = cast(str, record["sourceFile"])
            line = cast(int, record["line"])
        except KeyError as error:
            raise SystemExit(f"inventory record lacks {error.args[0]}") from error
        if key in keys:
            raise SystemExit(f"inventory repeats invocation key: {key}")
        if not isinstance(source_file, str) or not source_file:
            raise SystemExit(f"inventory has an invalid source file for {key}")
        if not isinstance(line, int) or line < 1:
            raise SystemExit(f"inventory has an invalid source line for {key}")
        keys.add(key)
        starts[source_file].add(line)
        observed += 1
    if observed != expected_records:
        raise SystemExit(
            "inventory invocation count differs from its summary: "
            f"expected={expected_records} observed={observed}"
        )
    return {source: tuple(sorted(lines)) for source, lines in starts.items()}


def _source_segments(
    upstream: Path, starts_by_source: dict[str, tuple[int, ...]]
) -> dict[tuple[str, int], tuple[int, str]]:
    segments: dict[tuple[str, int], tuple[int, str]] = {}
    for source_file, starts in starts_by_source.items():
        path = upstream / source_file
        if not path.is_file():
            raise SystemExit(f"inventory source is missing: {source_file}")
        lines = path.read_text(errors="replace").splitlines(keepends=True)
        for index, start in enumerate(starts):
            if start > len(lines):
                raise SystemExit(
                    f"inventory source line is outside {source_file}: {start}"
                )
            next_start = (
                starts[index + 1] if index + 1 < len(starts) else len(lines) + 1
            )
            end = next_start - 1
            segments[(source_file, start)] = (end, "".join(lines[start - 1 : end]))
    return segments


def _classify_invocation(
    record: dict[str, object], source_segment: str
) -> tuple[str, list[str]]:
    source_classification = cast(str, record["sourceClassification"])
    source_file = cast(str, record.get("sourceFile", ""))
    role = cast(str, record.get("role", ""))
    source_reasons = [
        cast(str, reason)
        for reason in cast(list[object], record["classificationReasons"])
    ]
    hook_names = sorted(set(PRIVATE_TEST_HOOK.findall(source_segment)))
    hook_reasons = [
        f"implementation-private-test-hook:{hook_name}" for hook_name in hook_names
    ]
    direct_private_reasons = (
        ["private-client-access"]
        if PRIVATE_CLIENT_ACCESS.search(source_segment)
        else []
    )
    shared_private_reasons = [
        reason
        for expected_source, identifier, reason in SHARED_PRIVATE_MACHINERY
        if source_file == expected_source
        and re.search(rf"\b{re.escape(identifier)}\b", source_segment)
    ]
    local_implementation_reasons = [
        f"local-implementation-under-test:{implementation}"
        for prefix, implementation in LOCAL_IMPLEMENTATIONS_UNDER_TEST
        if source_file.startswith(prefix)
    ]
    local_implementation_reasons.extend(
        f"local-implementation-under-test:{implementation}"
        for expected_source, call_pattern, implementation in DIRECT_PRIVATE_IMPLEMENTATIONS_UNDER_TEST
        if source_file == expected_source and re.search(call_pattern, source_segment)
    )
    product_infrastructure_reasons = [
        f"product-infrastructure-role:{reason}"
        for expected_role, reason in PRODUCT_INFRASTRUCTURE_ROLES
        if role == expected_role
    ]
    product_infrastructure_reasons.extend(
        f"product-infrastructure-source:{reason}"
        for prefix, reason in PRODUCT_INFRASTRUCTURE_SOURCES
        if source_file.startswith(prefix)
    )

    if source_classification == "implementation-internal":
        non_private_source_reasons = [
            reason for reason in source_reasons if reason != "private-client-access"
        ]
        hard_internal_reasons = sorted(
            set(
                direct_private_reasons
                + shared_private_reasons
                + local_implementation_reasons
            )
        )
        if hard_internal_reasons:
            return source_classification, sorted(
                set(non_private_source_reasons + hard_internal_reasons + hook_reasons)
            )
        if non_private_source_reasons:
            if source_file.startswith(PUBLIC_API_SOURCE_PREFIXES):
                return "public-api-with-private-test-helper", sorted(
                    set(non_private_source_reasons + hook_reasons)
                )
            return "product-infrastructure", sorted(
                set(
                    non_private_source_reasons
                    + hook_reasons
                    + product_infrastructure_reasons
                )
            )
        source_reasons = []
        source_classification = (
            "public-api-candidate"
            if source_file.startswith(PUBLIC_API_SOURCE_PREFIXES)
            else "product-infrastructure"
        )
    if source_classification == "product-infrastructure":
        reasons = sorted(
            set(source_reasons + hook_reasons + product_infrastructure_reasons)
        )
        if not reasons:
            raise SystemExit(
                f"product infrastructure has no explicit reason: {source_file}"
            )
        return source_classification, reasons
    if source_classification != "public-api-candidate":
        raise SystemExit(f"unknown source classification: {source_classification}")
    if local_implementation_reasons:
        return "implementation-internal", sorted(
            set(source_reasons + hook_reasons + local_implementation_reasons)
        )
    if direct_private_reasons:
        return "implementation-internal", sorted(
            set(hook_reasons + direct_private_reasons)
        )
    if product_infrastructure_reasons:
        return "product-infrastructure", sorted(
            set(source_reasons + hook_reasons + product_infrastructure_reasons)
        )
    if hook_reasons:
        return "public-api-with-implementation-private-hook", hook_reasons
    return source_classification, source_reasons


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
        "--inventory",
        type=Path,
        default=root / ".work/official-conformance/playwright-core-v1.58.0.jsonl",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=root / ".work/official-conformance/"
        "playwright-core-v1.58.0-invocation-classifications.jsonl",
    )
    parser.add_argument(
        "--outcomes",
        type=Path,
        help="optional complete outcome ledger to cross-tabulate without modifying it",
    )
    args = parser.parse_args()

    upstream = args.upstream.resolve()
    manifest = cast(Manifest, json.loads(args.manifest.read_text()))
    if manifest["schema"] != 1:
        raise SystemExit(f"unsupported manifest schema: {manifest['schema']}")
    _verify_upstream(upstream, manifest)

    inventory = args.inventory.resolve()
    expected_records, inventory_digest = _verify_inventory(
        inventory, manifest["commit"]
    )
    starts_by_source = _source_starts(inventory, expected_records)
    segments = _source_segments(upstream, starts_by_source)

    outcomes = args.outcomes.resolve() if args.outcomes is not None else None
    outcome_digest = (
        _verify_outcomes(outcomes, expected_records) if outcomes is not None else None
    )
    outcome_records = iter(_records(outcomes)) if outcomes is not None else None

    output = args.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    classification_counts: Counter[str] = Counter()
    reason_counts: Counter[str] = Counter()
    outcome_classification_counts: Counter[str] = Counter()
    digest = hashlib.sha256()
    observed = 0
    with output.open("wb") as stream:
        for record in _records(inventory):
            source_file = cast(str, record["sourceFile"])
            start = cast(int, record["line"])
            end, segment = segments[(source_file, start)]
            classification, reasons = _classify_invocation(record, segment)
            if outcome_records is not None:
                try:
                    outcome_record = next(outcome_records)
                except StopIteration as error:
                    raise SystemExit(
                        "outcome ledger ended before the inventory"
                    ) from error
                inventory_key = (
                    record["suite"],
                    record["specId"],
                    record["projectId"],
                )
                outcome_key = (
                    outcome_record["suite"],
                    outcome_record["specId"],
                    outcome_record["projectId"],
                )
                if inventory_key != outcome_key:
                    raise SystemExit(
                        "outcome ledger order differs from the inventory: "
                        f"{outcome_key} != {inventory_key}"
                    )
                outcome = cast(str, outcome_record["outcome"])
                outcome_classification_counts[f"{outcome}:{classification}"] += 1
            audit_record = {
                "suite": record["suite"],
                "role": record["role"],
                "specId": record["specId"],
                "projectId": record["projectId"],
                "projectName": record["projectName"],
                "sourceFile": source_file,
                "sourceStartLine": start,
                "sourceEndLine": end,
                "titlePath": record["titlePath"],
                "sourceClassification": record["sourceClassification"],
                "invocationClassification": classification,
                "invocationClassificationReasons": reasons,
            }
            payload = _stable_json_line(audit_record)
            stream.write(payload)
            digest.update(payload)
            classification_counts[classification] += 1
            reason_counts.update(reasons)
            observed += 1
    if observed != expected_records:
        raise SystemExit(
            "classified invocation count changed unexpectedly: "
            f"expected={expected_records} observed={observed}"
        )
    if outcome_records is not None:
        try:
            next(outcome_records)
        except StopIteration:
            pass
        else:
            raise SystemExit("outcome ledger has records after the inventory ended")

    summary = {
        "schema": 1,
        "repository": manifest["repository"],
        "tag": manifest["tag"],
        "commit": manifest["commit"],
        "inventory": str(inventory),
        "inventorySha256": inventory_digest,
        "records": observed,
        "invocationClassificationCounts": dict(sorted(classification_counts.items())),
        "invocationClassificationReasonCounts": dict(sorted(reason_counts.items())),
        "outcomeLedger": str(outcomes) if outcomes is not None else None,
        "outcomeLedgerSha256": outcome_digest,
        "outcomeClassificationCounts": dict(
            sorted(outcome_classification_counts.items())
        ),
        "ledgerSha256": digest.hexdigest(),
        "ledger": str(output),
    }
    summary_path = output.with_suffix(".summary.json")
    summary_path.write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
