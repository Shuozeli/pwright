#!/usr/bin/env python3
"""Join Playwright JSON reports to the complete pinned invocation ledger."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from collections import Counter
from pathlib import Path
from typing import Any, cast

Key = tuple[str, str, str]
GENERATED_FIELDS = frozenset({
    "attempts", "error", "outcome", "resultFile", "evidenceScope",
    "candidateRuntimeRequired",
})
IMAGE_TOOLS_V2_BASES = {
    "candidate": (
        "playwright-core-candidate-route55-cookies-frozen-20261011-v1.jsonl",
        "142f659550fb63bc32653a9019ff030d2afe46da3cea8c645be9fdbae0aa2051",
    ),
    "oracle": (
        "playwright-core-oracle-route55-blank-anchor-20261011-v2.jsonl",
        "02e1ec394ada0b806821a05ebd98bfae8c8ffa72e4628d1d1d8079ed141fa8f8",
    ),
}
MCP_CONFIG_V1_BASES = {
    "candidate": (
        "playwright-core-candidate-core461-image-tools-source-only-v2.jsonl",
        "63660ff6364254d93fa0b6d408bbb9e87575c631368d715d83edf0116c2a525f",
    ),
    "oracle": (
        "playwright-core-oracle-core461-image-tools-source-only-v2.jsonl",
        "d22c37309ebd1e87b740282dd1b4b45a80605b2c1fc63c8ee196e79e77d34419",
    ),
}
CORE_BATCH_INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
CORE_BATCH_AUDIT_SHA256 = "927498c7f312c32dfcaff5ee8b58b597f5af8f879f1a5cd5bcbb19341fdb4412"
CORE_BATCH_COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
CORE_BATCH_EVENT_REASON = "local-implementation-under-test:tests/library/events/utils"
CORE_BATCH_COUNTS = {"library": (852, 147), "bidi": (1704, 294)}


def _stable_json_line(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def _file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _load_inventory(path: Path) -> tuple[list[dict[str, object]], dict[Key, int]]:
    records = [
        cast(dict[str, object], json.loads(line))
        for line in path.read_text().splitlines()
    ]
    keys: dict[Key, int] = {}
    for index, record in enumerate(records):
        key = (
            cast(str, record["suite"]),
            cast(str, record["specId"]),
            cast(str, record["projectId"]),
        )
        if key in keys:
            raise SystemExit(f"duplicate inventory key: {key}")
        keys[key] = index
        record["outcome"] = "not-run"
        record["attempts"] = 0
    return records, keys


def _load_base(
    path: Path,
    records: list[dict[str, object]],
) -> None:
    base_records = [
        cast(dict[str, object], json.loads(line))
        for line in path.read_text().splitlines()
    ]
    if len(base_records) != len(records):
        raise SystemExit(
            f"base ledger record count differs from inventory: "
            f"{len(base_records)} != {len(records)}"
        )

    for index, (inventory_record, base_record) in enumerate(
        zip(records, base_records, strict=True)
    ):
        expected_inventory = {
            key: value
            for key, value in inventory_record.items()
            if key not in GENERATED_FIELDS
        }
        actual_inventory = {
            key: value
            for key, value in base_record.items()
            if key not in GENERATED_FIELDS
        }
        if actual_inventory != expected_inventory:
            raise SystemExit(f"base ledger differs from inventory at record {index}")
        if "outcome" not in base_record or "attempts" not in base_record:
            raise SystemExit(f"base ledger lacks outcome fields at record {index}")

        for field in GENERATED_FIELDS:
            if field in base_record:
                inventory_record[field] = base_record[field]


def _result_error(test: dict[str, Any]) -> str | None:
    for result in test.get("results", []):
        error = result.get("error") or {}
        if message := error.get("message"):
            return cast(str, message).splitlines()[0]
        for nested in result.get("errors", []):
            if message := nested.get("message"):
                return cast(str, message).splitlines()[0]
    return None


def _apply_report(
    records: list[dict[str, object]],
    keys: dict[Key, int],
    suite_id: str,
    report_path: Path,
    applied_report_keys: set[Key],
    *,
    replace_base: bool,
    source_only_pairs: set[tuple[str, str]] | None = None,
    source_only_batch: bool = False,
    verified_digest: str | None = None,
) -> None:
    if source_only_batch and source_only_pairs is None:
        raise SystemExit("source-only batch requires verified invocation pairs")
    report_bytes = report_path.read_bytes()
    if verified_digest is not None and hashlib.sha256(report_bytes).hexdigest() != verified_digest:
        raise SystemExit(f"result changed after provenance verification: {report_path}")
    report = cast(dict[str, Any], json.loads(report_bytes))
    seen: set[Key] = set()

    def walk(suite: dict[str, Any]) -> None:
        for spec in suite.get("specs", []):
            for test in spec.get("tests", []):
                key = (suite_id, spec["id"], test["projectId"])
                if key not in keys:
                    raise SystemExit(f"result is absent from inventory: {key}")
                if key in seen:
                    raise SystemExit(f"result report repeats an invocation: {key}")
                seen.add(key)
                if key in applied_report_keys:
                    raise SystemExit(
                        f"multiple new reports cover the same invocation: {key}"
                    )
                applied_report_keys.add(key)
                record = records[keys[key]]
                if record["outcome"] != "not-run" and (
                    not replace_base or source_only_batch
                ):
                    raise SystemExit(
                        f"source-only batch cannot overwrite an existing outcome: {key}"
                        if source_only_batch
                        else f"multiple reports cover the same invocation: {key}"
                    )
                record.pop("error", None)
                record.pop("resultFile", None)
                record.pop("evidenceScope", None)
                record.pop("candidateRuntimeRequired", None)
                record["outcome"] = test["status"]
                record["attempts"] = len(test.get("results", []))
                record["resultFile"] = str(report_path)
                if source_only_pairs is not None:
                    record["evidenceScope"] = "upstream-private-source-only"
                    record["candidateRuntimeRequired"] = False
                if error := _result_error(test):
                    record["error"] = error
        for child in suite.get("suites", []):
            walk(child)

    for suite in report.get("suites", []):
        walk(suite)
    if source_only_pairs is not None and {
        (spec_id, project_id) for _, spec_id, project_id in seen
    } != source_only_pairs:
        raise SystemExit("source-only report keys differ from verified provenance")


def _source_only_selection(
    report_path: Path,
    expected_mode: str | None = None,
) -> tuple[set[tuple[str, str]], bool, str]:
    from verify_playwright_official_provenance import verify_provenance

    provenance_path = report_path.with_suffix(".provenance.json")
    if not provenance_path.is_file():
        raise SystemExit(f"source-only result lacks provenance: {provenance_path}")
    payload = verify_provenance(
        json.loads(provenance_path.read_text()), provenance_path=provenance_path
    )
    if payload["schema"] != 3 or payload["result"] != str(report_path):
        raise SystemExit("source-only result requires exact schema-3 provenance")
    if expected_mode is not None and payload["mode"] != expected_mode:
        raise SystemExit("source-only result lane differs from the destination ledger")
    evidence = cast(dict[str, Any], payload["sourceOnlyExecution"])
    source = cast(dict[str, Any], evidence["source"])
    if source["case"] == "private-internal-batch-v1":
        report = cast(dict[str, Any], json.loads(report_path.read_text()))
        pairs: set[tuple[str, str]] = set()

        def visit(suite: dict[str, Any]) -> None:
            for spec in suite.get("specs", []):
                for test in spec.get("tests", []):
                    pairs.add((spec["id"], test["projectId"]))
            for child in suite.get("suites", []):
                visit(child)

        for suite in report["suites"]:
            visit(suite)
        if len(pairs) != evidence["reportedTests"]:
            raise SystemExit("source-only batch report has duplicate invocation pairs")
        return pairs, True, cast(str, payload["resultSha256"])
    return set(zip(
        cast(list[str], source["expectedSpecIds"]),
        cast(list[str], source["expectedProjectIds"]),
        strict=True,
    )), False, cast(str, payload["resultSha256"])


def _source_only_pairs(report_path: Path) -> set[tuple[str, str]]:
    return _source_only_selection(report_path)[0]


def _verified_core_batch_event_rows(inventory_path: Path) -> dict[Key, str]:
    root = Path(__file__).parents[1]
    inventory = root / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
    audit = root / ".work/official-conformance/playwright-core-v1.58.0-invocation-classifications.jsonl"
    summary_path = audit.with_suffix(".summary.json")
    if (
        inventory_path.resolve() != inventory.resolve()
        or inventory.is_symlink()
        or audit.is_symlink()
        or summary_path.is_symlink()
        or not inventory.is_file()
        or not audit.is_file()
        or not summary_path.is_file()
    ):
        raise SystemExit("Core460 source-only event audit requires its pinned inventory and files")
    summary = cast(dict[str, Any], json.loads(summary_path.read_text()))
    if (
        summary.get("schema") != 1
        or summary.get("repository") != "https://github.com/microsoft/playwright.git"
        or summary.get("tag") != "v1.58.0"
        or summary.get("commit") != CORE_BATCH_COMMIT
        or summary.get("inventory") != str(inventory.resolve())
        or summary.get("inventorySha256") != CORE_BATCH_INVENTORY_SHA256
        or summary.get("records") != 48477
        or summary.get("ledger") != str(audit.resolve())
        or summary.get("ledgerSha256") != CORE_BATCH_AUDIT_SHA256
        or _file_sha256(inventory) != CORE_BATCH_INVENTORY_SHA256
        or _file_sha256(audit) != CORE_BATCH_AUDIT_SHA256
    ):
        raise SystemExit("Core460 source-only event audit lost its pinned source or digest")

    event_rows: dict[Key, str] = {}
    observed = 0
    with audit.open() as source:
        for line in source:
            observed += 1
            row = cast(dict[str, Any], json.loads(line))
            if (
                row.get("sourceClassification") != "public-api-candidate"
                or row.get("invocationClassification") != "implementation-internal"
                or row.get("invocationClassificationReasons") != [CORE_BATCH_EVENT_REASON]
            ):
                continue
            source_file = row.get("sourceFile")
            if (
                row.get("role") != "official-entrypoint"
                or not isinstance(source_file, str)
                or not source_file.startswith("tests/library/events/")
                or not source_file.endswith(".spec.ts")
            ):
                raise SystemExit("Core460 source-only event audit contains an unapproved exception")
            key = (row.get("suite"), row.get("specId"), row.get("projectId"))
            if key in event_rows:
                raise SystemExit("Core460 source-only event audit repeats an invocation")
            event_rows[cast(Key, key)] = source_file
    counts = Counter(key[0] for key in event_rows)
    if observed != 48477 or counts != {"library": 147, "bidi": 294}:
        raise SystemExit("Core460 source-only event audit changed its exact 441 exceptions")
    return event_rows


def _check_generic_source_only_inventory_rows(
    records: list[dict[str, object]],
    keys: dict[Key, int],
    suite_id: str,
    pairs: set[tuple[str, str]],
    *,
    source_only_batch: bool = False,
    inventory_path: Path | None = None,
) -> None:
    event_rows: dict[Key, str] = {}
    if source_only_batch:
        expected = CORE_BATCH_COUNTS.get(suite_id)
        if inventory_path is None or expected is None or len(pairs) != expected[0]:
            raise SystemExit("Core460 source-only batch changed its pinned suite or size")
        event_rows = _verified_core_batch_event_rows(inventory_path)
    public_rows = 0
    for spec_id, project_id in pairs:
        key = (suite_id, spec_id, project_id)
        if key not in keys:
            raise SystemExit(f"source-only invocation is absent from inventory: {key}")
        record = records[keys[key]]
        if record.get("role") != "official-entrypoint":
            raise SystemExit(f"source-only target is not a private entrypoint: {key}")
        if record.get("sourceClassification") == "implementation-internal":
            continue
        if (
            not source_only_batch
            or record.get("sourceClassification") != "public-api-candidate"
            or record.get("sourceFile") != event_rows.get(key)
        ):
            raise SystemExit(f"source-only target is not a private entrypoint: {key}")
        public_rows += 1
    if source_only_batch and public_rows != CORE_BATCH_COUNTS[suite_id][1]:
        raise SystemExit("Core460 source-only batch changed its exact public inventory labels")


def _image_tools_source_only_selection(
    report_path: Path, expected_mode: str
) -> tuple[set[tuple[str, str]], str]:
    from verify_playwright_official_image_tools_source_only import verify

    provenance_path = report_path.with_suffix(".provenance.json")
    if not provenance_path.is_file() or provenance_path.is_symlink():
        raise SystemExit(f"image-tools source-only result lacks provenance: {provenance_path}")
    try:
        verified = verify(provenance_path)
    except (OSError, KeyError, TypeError, ValueError, subprocess.CalledProcessError) as error:
        raise SystemExit(f"image-tools source-only provenance invalid: {error}") from error
    if (
        verified["mode"] != expected_mode
        or verified["passed"] != 25
        or verified["candidateRuntimeAttested"] is not False
        or verified["publicApiParityEvidence"] is not False
    ):
        raise SystemExit("image-tools source-only result has a mode, pass, or attestation mismatch")
    provenance = cast(dict[str, Any], json.loads(provenance_path.read_text()))
    if provenance.get("result") != str(report_path):
        raise SystemExit("image-tools source-only result path differs from verified provenance")
    report = cast(dict[str, Any], json.loads(report_path.read_text()))
    pairs: set[tuple[str, str]] = set()
    count = 0

    def visit(suite: dict[str, Any]) -> None:
        nonlocal count
        for spec in suite.get("specs", []):
            for test in spec.get("tests", []):
                attempts = test.get("results")
                if (
                    test.get("projectId") != "image_tools"
                    or test.get("expectedStatus") != "passed"
                    or test.get("status") != "expected"
                    or not isinstance(attempts, list)
                    or len(attempts) != 1
                    or not isinstance(attempts[0], dict)
                    or attempts[0].get("status") != "passed"
                    or attempts[0].get("retry") != 0
                ):
                    raise SystemExit("image-tools source-only report contains a non-expected outcome")
                count += 1
                pairs.add((spec["id"], test["projectId"]))
        for child in suite.get("suites", []):
            visit(child)

    for suite in report["suites"]:
        visit(suite)
    if count != 25 or len(pairs) != 25:
        raise SystemExit("image-tools source-only report must contain 25 unique invocations")
    return pairs, cast(str, verified["resultSha256"])


def _check_image_tools_inventory_rows(
    records: list[dict[str, object]],
    keys: dict[Key, int],
    pairs: set[tuple[str, str]],
) -> None:
    if len(pairs) != 25:
        raise SystemExit("image-tools source-only selection must contain 25 invocations")
    for spec_id, project_id in pairs:
        key = ("playwright-test", spec_id, project_id)
        if key not in keys:
            raise SystemExit(f"image-tools source-only invocation is absent from inventory: {key}")
        record = records[keys[key]]
        if (
            record.get("sourceClassification") != "implementation-internal"
            or record.get("role") != "official-entrypoint"
            or record.get("outcome") != "not-run"
        ):
            raise SystemExit(f"image-tools source-only target is not an unrun private entrypoint: {key}")


def _check_image_tools_destination(
    root: Path, base: Path, output: Path, mode: str
) -> None:
    mode_prefix = f"playwright-core-{mode}-"
    base_name, base_digest = IMAGE_TOOLS_V2_BASES[mode]
    expected_base = root / ".work/official-conformance/outcomes" / base_name
    if (
        base.resolve() != expected_base
        or base.is_symlink()
        or not base.is_file()
        or _file_sha256(base) != base_digest
        or not output.name.startswith(mode_prefix)
        or output.resolve() == base.resolve()
        or output.parent.resolve() != expected_base.parent
        or output.suffix != ".jsonl"
    ):
        raise SystemExit(
            "image-tools source-only base must be the pinned current strict ledger "
            "and successor must match its mode"
        )
    for destination in (output, output.with_suffix(".summary.json")):
        if destination.is_symlink() or destination.exists():
            raise SystemExit(f"image-tools source-only successor already exists: {destination}")


def _mcp_config_source_only_selection(
    report_path: Path, expected_mode: str
) -> tuple[set[tuple[str, str]], str]:
    from verify_playwright_official_mcp_config_source_only import (
        verify,
        verify_manifest,
    )

    provenance_path = report_path.with_suffix(".provenance.json")
    if not provenance_path.is_file() or provenance_path.is_symlink():
        raise SystemExit(f"MCP config source-only result lacks provenance: {provenance_path}")
    try:
        verified = verify(provenance_path)
        manifest = verify_manifest()
    except (OSError, KeyError, TypeError, ValueError, subprocess.CalledProcessError) as error:
        raise SystemExit(f"MCP config source-only provenance invalid: {error}") from error
    provenance = cast(dict[str, Any], json.loads(provenance_path.read_text()))
    if (
        provenance.get("result") != str(report_path)
        or verified.get("resultSha256") != _file_sha256(report_path)
    ):
        raise SystemExit("MCP config source-only result path or digest differs from verified provenance")
    if (
        verified["mode"] != expected_mode
        or verified["passed"] != 8
        or verified["classification"] != "implementation-internal"
        or verified["scope"] != "upstream-private-source-only"
        or verified["candidateRuntimeAttested"] is not False
        or verified["publicApiParityEvidence"] is not False
    ):
        raise SystemExit("MCP config source-only result has a mode, pass, or attestation mismatch")
    invocations = manifest["suites"][0]["expectedInvocations"]
    pairs = {(entry["specId"], entry["projectId"]) for entry in invocations}
    if len(pairs) != 8:
        raise SystemExit("MCP config source-only selection must contain eight unique invocations")
    return pairs, cast(str, verified["resultSha256"])


def _check_mcp_config_inventory_rows(
    records: list[dict[str, object]],
    keys: dict[Key, int],
    pairs: set[tuple[str, str]],
) -> None:
    if len(pairs) != 8:
        raise SystemExit("MCP config source-only selection must contain eight invocations")
    for spec_id, project_id in pairs:
        key = ("mcp", spec_id, project_id)
        if key not in keys:
            raise SystemExit(f"MCP config source-only invocation is absent from inventory: {key}")
        record = records[keys[key]]
        if (
            record.get("sourceClassification") != "implementation-internal"
            or record.get("role") != "official-entrypoint"
            or record.get("outcome") != "not-run"
            or record.get("attempts") != 0
        ):
            raise SystemExit(f"MCP config source-only target is not an unrun private entrypoint: {key}")


def _check_mcp_config_destination(
    root: Path, base: Path, output: Path, mode: str
) -> None:
    mode_prefix = f"playwright-core-{mode}-"
    base_name, base_digest = MCP_CONFIG_V1_BASES[mode]
    expected_base = root / ".work/official-conformance/outcomes" / base_name
    if (
        base.resolve() != expected_base
        or base.is_symlink()
        or not base.is_file()
        or _file_sha256(base) != base_digest
        or not output.name.startswith(mode_prefix)
        or output.resolve() == base.resolve()
        or output.parent.resolve() != expected_base.parent
        or output.suffix != ".jsonl"
    ):
        raise SystemExit(
            "MCP config source-only base must be the pinned current strict ledger "
            "and successor must match its mode"
        )
    for destination in (output, output.with_suffix(".summary.json")):
        if destination.is_symlink() or destination.exists():
            raise SystemExit(f"MCP config source-only successor already exists: {destination}")


def _generic_result_mode(output: Path, base: Path | None) -> str:
    for mode in ("candidate", "oracle"):
        stem = f"playwright-core-{mode}"
        if output.name == f"{stem}.jsonl" or output.name.startswith(f"{stem}-"):
            if base is not None and not (
                base.name == f"{stem}.jsonl" or base.name.startswith(f"{stem}-")
            ):
                raise SystemExit("Core result base and successor must use the same lane")
            return mode
    raise SystemExit("Core --result successor name must identify the candidate or oracle lane")


def _verified_generic_result(report_path: Path, mode: str) -> str:
    from verify_playwright_official_provenance import verify_provenance

    if report_path.is_symlink():
        raise SystemExit(f"Core --result cannot be a symlink: {report_path}")
    report_path = report_path.resolve()
    provenance_path = report_path.with_suffix(".provenance.json")
    if (
        not report_path.is_file() or report_path.is_symlink()
        or not provenance_path.is_file() or provenance_path.is_symlink()
    ):
        raise SystemExit(f"Core --result requires a regular result and sibling provenance: {report_path}")
    try:
        payload = verify_provenance(
            json.loads(provenance_path.read_text()), provenance_path=provenance_path
        )
    except (OSError, KeyError, TypeError, ValueError, subprocess.CalledProcessError) as error:
        raise SystemExit(f"Core --result provenance verification failed: {error}") from error
    if (
        payload.get("schema") != 2
        or payload.get("mode") != mode
        or payload.get("result") != str(report_path)
        or (mode == "candidate" and payload.get("candidateArtifactSha256") is None)
    ):
        raise SystemExit(
            "Core --result requires exact schema-2 lane, result, and candidate artifact evidence"
        )
    return cast(str, payload["resultSha256"])


def main() -> None:
    root = Path(__file__).parents[1]
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--inventory",
        type=Path,
        default=root / ".work/official-conformance/playwright-core-v1.58.0.jsonl",
    )
    parser.add_argument(
        "--base",
        type=Path,
        help=(
            "previous ledger to validate against the inventory before applying "
            "replacement reports"
        ),
    )
    parser.add_argument(
        "--result",
        action="append",
        default=[],
        metavar="SUITE=PATH",
        help="official JSON report and its manifest suite id",
    )
    parser.add_argument(
        "--source-only-result",
        action="append",
        default=[],
        metavar="SUITE=PATH",
        help="schema-3 verified private source-only report; never public API parity",
    )
    parser.add_argument(
        "--image-tools-source-only-result",
        metavar="SUITE=PATH",
        help="one independently verified 25-case image-tools source-only report",
    )
    parser.add_argument(
        "--image-tools-mode",
        choices=("candidate", "oracle"),
        help="required destination mode for the image-tools source-only successor",
    )
    parser.add_argument(
        "--mcp-config-source-only-result",
        metavar="SUITE=PATH",
        help="one independently verified eight-case MCP config source-only report",
    )
    parser.add_argument(
        "--mcp-config-mode",
        choices=("candidate", "oracle"),
        help="required destination mode for the MCP config source-only successor",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=root
        / ".work/official-conformance/outcomes/playwright-core-candidate.jsonl",
    )
    args = parser.parse_args()

    generic_result_mode = (
        _generic_result_mode(args.output, args.base)
        if args.result or args.source_only_result else None
    )

    image_tools_value = args.image_tools_source_only_result
    mcp_config_value = args.mcp_config_source_only_result
    if image_tools_value is not None and mcp_config_value is not None:
        parser.error("one source-only successor cohort must be promoted at a time")
    if image_tools_value is not None:
        if args.image_tools_mode is None or args.base is None:
            parser.error("image-tools source-only promotion requires --image-tools-mode and --base")
        if args.result or args.source_only_result:
            parser.error("image-tools source-only promotion must contain only its exact 25 invocations")
        if not image_tools_value.startswith("playwright-test="):
            parser.error("image-tools source-only result must use suite playwright-test")
        try:
            _check_image_tools_destination(root, args.base, args.output, args.image_tools_mode)
        except SystemExit as error:
            parser.error(str(error))
    elif args.image_tools_mode is not None:
        parser.error("--image-tools-mode requires --image-tools-source-only-result")
    if mcp_config_value is not None:
        if args.mcp_config_mode is None or args.base is None:
            parser.error("MCP config source-only promotion requires --mcp-config-mode and --base")
        if args.result or args.source_only_result:
            parser.error("MCP config source-only promotion must contain only its exact eight invocations")
        if not mcp_config_value.startswith("mcp="):
            parser.error("MCP config source-only result must use suite mcp")
        try:
            _check_mcp_config_destination(root, args.base, args.output, args.mcp_config_mode)
        except SystemExit as error:
            parser.error(str(error))
    elif args.mcp_config_mode is not None:
        parser.error("--mcp-config-mode requires --mcp-config-source-only-result")

    records, keys = _load_inventory(args.inventory.resolve())
    if args.base is not None:
        _load_base(args.base.resolve(), records)
    applied_report_keys: set[Key] = set()
    if image_tools_value is not None:
        _, raw_path = image_tools_value.split("=", maxsplit=1)
        report_path = Path(raw_path).resolve()
        pairs, verified_digest = _image_tools_source_only_selection(report_path, args.image_tools_mode)
        _check_image_tools_inventory_rows(records, keys, pairs)
        _apply_report(
            records, keys, "playwright-test", report_path, applied_report_keys,
            replace_base=True, source_only_pairs=pairs, source_only_batch=True,
            verified_digest=verified_digest,
        )
    if mcp_config_value is not None:
        _, raw_path = mcp_config_value.split("=", maxsplit=1)
        report_path = Path(raw_path).resolve()
        pairs, verified_digest = _mcp_config_source_only_selection(report_path, args.mcp_config_mode)
        _check_mcp_config_inventory_rows(records, keys, pairs)
        _apply_report(
            records, keys, "mcp", report_path, applied_report_keys,
            replace_base=True, source_only_pairs=pairs, source_only_batch=True,
            verified_digest=verified_digest,
        )
    for value, source_only in [
        *((entry, False) for entry in args.result),
        *((entry, True) for entry in args.source_only_result),
    ]:
        if "=" not in value:
            parser.error(f"--result must be SUITE=PATH, got {value!r}")
        suite_id, raw_path = value.split("=", maxsplit=1)
        supplied_report = Path(raw_path)
        report_path = supplied_report.resolve()
        if source_only:
            source_only_pairs, source_only_batch, verified_digest = _source_only_selection(
                report_path, cast(str, generic_result_mode)
            )
            _check_generic_source_only_inventory_rows(
                records, keys, suite_id, source_only_pairs,
                source_only_batch=source_only_batch,
                inventory_path=args.inventory.resolve(),
            )
        else:
            source_only_pairs, source_only_batch = None, False
            verified_digest = _verified_generic_result(
                supplied_report, cast(str, generic_result_mode)
            )
        _apply_report(
            records,
            keys,
            suite_id,
            report_path,
            applied_report_keys,
            replace_base=args.base is not None,
            source_only_pairs=source_only_pairs,
            source_only_batch=source_only_batch,
            verified_digest=verified_digest,
        )

    payload = b"".join(_stable_json_line(record) for record in records)
    digest = hashlib.sha256(payload).hexdigest()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    if image_tools_value is not None or mcp_config_value is not None:
        with args.output.open("xb") as output:
            output.write(payload)
    else:
        args.output.write_bytes(payload)
    counts = Counter(cast(str, record["outcome"]) for record in records)
    scopes = Counter(
        cast(str, record["evidenceScope"])
        for record in records if "evidenceScope" in record
    )
    summary = {
        "schema": 1,
        "inventory": str(args.inventory.resolve()),
        "records": len(records),
        "outcomes": dict(sorted(counts.items())),
        "explicitEvidenceScopes": dict(sorted(scopes.items())),
        "ledgerSha256": digest,
        "ledger": str(args.output.resolve()),
    }
    summary_path = args.output.with_suffix(".summary.json")
    if image_tools_value is not None or mcp_config_value is not None:
        with summary_path.open("x") as output:
            output.write(json.dumps(summary, indent=2) + "\n")
    else:
        summary_path.write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
