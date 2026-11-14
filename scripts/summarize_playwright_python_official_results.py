#!/usr/bin/env python3
"""Join official pytest JUnit reports to the complete Python test ledger."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path
from typing import cast

Key = tuple[str, str]
GENERATED_FIELDS = frozenset(
    {"attempts", "durationSeconds", "error", "outcome", "resultFile"}
)


def _stable_json_line(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def _junit_key(node_id: str) -> Key:
    source, *segments = node_id.split("::")
    if not segments or not source.endswith(".py"):
        raise SystemExit(f"unsupported pytest inventory ID: {node_id}")
    module = source.removesuffix(".py").replace("/", ".")
    classname = ".".join((module, *segments[:-1]))
    return classname, segments[-1]


def _load_inventory(path: Path) -> tuple[list[dict[str, object]], dict[Key, int]]:
    records = [
        cast(dict[str, object], json.loads(line))
        for line in path.read_text().splitlines()
    ]
    keys: dict[Key, int] = {}
    for index, record in enumerate(records):
        key = _junit_key(cast(str, record["id"]))
        if key in keys:
            raise SystemExit(f"duplicate JUnit inventory key: {key}")
        keys[key] = index
        record["outcome"] = "not-run"
        record["attempts"] = 0
    return records, keys


def _load_base(path: Path, records: list[dict[str, object]]) -> None:
    base_records = [
        cast(dict[str, object], json.loads(line))
        for line in path.read_text().splitlines()
    ]
    if len(base_records) != len(records):
        raise SystemExit(
            "base ledger record count differs from inventory: "
            f"{len(base_records)} != {len(records)}"
        )
    # The record counts are checked immediately above, so plain zip is exact
    # here and keeps this conformance tool compatible with Python 3.9.
    for index, (inventory_record, base_record) in enumerate(zip(records, base_records)):
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


def _case_outcome(case: ET.Element) -> tuple[str, str | None]:
    if (failure := case.find("failure")) is not None:
        return "failed", (failure.get("message") or failure.text or "").splitlines()[0]
    if (error := case.find("error")) is not None:
        return "error", (error.get("message") or error.text or "").splitlines()[0]
    if (skipped := case.find("skipped")) is not None:
        return "skipped", (skipped.get("message") or skipped.text or "").splitlines()[0]
    return "passed", None


def _apply_report(
    records: list[dict[str, object]],
    keys: dict[Key, int],
    report_path: Path,
    applied_report_keys: set[Key],
    *,
    replace_base: bool,
    verified_digest: str | None = None,
) -> None:
    report_bytes = report_path.read_bytes()
    if verified_digest is not None and hashlib.sha256(report_bytes).hexdigest() != verified_digest:
        raise SystemExit(f"result changed after provenance verification: {report_path}")
    root = ET.fromstring(report_bytes)
    seen: set[Key] = set()
    for case in root.iter("testcase"):
        classname = case.get("classname")
        name = case.get("name")
        if classname is None or name is None:
            raise SystemExit(f"JUnit testcase lacks classname/name: {report_path}")
        key = (classname, name)
        if key not in keys:
            raise SystemExit(f"result is absent from inventory: {key}")
        if key in seen:
            raise SystemExit(f"result report repeats a test: {key}")
        seen.add(key)
        if key in applied_report_keys:
            raise SystemExit(f"multiple new reports cover the same test: {key}")
        applied_report_keys.add(key)
        record = records[keys[key]]
        if record["outcome"] != "not-run" and not replace_base:
            raise SystemExit(f"multiple reports cover the same test: {key}")
        record.pop("durationSeconds", None)
        record.pop("error", None)
        record.pop("resultFile", None)
        outcome, error = _case_outcome(case)
        record["outcome"] = outcome
        record["attempts"] = 1
        record["durationSeconds"] = float(case.get("time", "0"))
        record["resultFile"] = str(report_path)
        if error:
            record["error"] = error


def _generic_result_mode(output: Path, base: Path | None) -> str:
    for mode in ("candidate", "oracle"):
        stem = f"playwright-python-{mode}"
        if output.name == f"{stem}.jsonl" or output.name.startswith(f"{stem}-"):
            if base is not None and not (
                base.name == f"{stem}.jsonl" or base.name.startswith(f"{stem}-")
            ):
                raise SystemExit("Python result base and successor must use the same lane")
            return mode
    raise SystemExit("Python --result successor name must identify the candidate or oracle lane")


def _verified_generic_result(report_path: Path, mode: str) -> str:
    from verify_playwright_python_official_provenance import verify_provenance

    if report_path.is_symlink():
        raise SystemExit(f"Python --result cannot be a symlink: {report_path}")
    report_path = report_path.resolve()
    provenance_path = report_path.with_suffix(".provenance.json")
    if (
        not report_path.is_file() or report_path.is_symlink()
        or not provenance_path.is_file() or provenance_path.is_symlink()
    ):
        raise SystemExit(f"Python --result requires a regular result and sibling provenance: {report_path}")
    try:
        payload = verify_provenance(
            json.loads(provenance_path.read_text()), provenance_path=provenance_path
        )
    except (OSError, KeyError, TypeError, ValueError, subprocess.CalledProcessError) as error:
        raise SystemExit(f"Python --result provenance verification failed: {error}") from error
    schema = payload.get("schema")
    if (
        not isinstance(schema, int) or isinstance(schema, bool) or schema < 2
        or payload.get("mode") != mode
        or payload.get("result") != str(report_path)
        or payload.get("collectOnly") is not False
        or payload.get("bootstrapOnly") is not False
    ):
        raise SystemExit(
            "Python --result requires exact source-bound execution provenance, lane, and result"
        )
    return cast(str, payload["resultSha256"])


def main() -> None:
    root = Path(__file__).parents[1]
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--inventory",
        type=Path,
        default=root
        / ".work/official-conformance/playwright-python-v1.58.0-profiled.jsonl",
    )
    parser.add_argument(
        "--base",
        type=Path,
        help=(
            "previous ledger to validate against the inventory before applying replacement reports"
        ),
    )
    parser.add_argument(
        "--result",
        action="append",
        type=Path,
        default=[],
        help="pytest JUnit execution report; collection-only XML is not an outcome",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=root
        / ".work/official-conformance/outcomes/playwright-python-candidate.jsonl",
    )
    args = parser.parse_args()

    generic_result_mode = (
        _generic_result_mode(args.output, args.base) if args.result else None
    )

    records, keys = _load_inventory(args.inventory.resolve())
    if args.base is not None:
        _load_base(args.base.resolve(), records)
    applied_report_keys: set[Key] = set()
    for report in args.result:
        verified_digest = _verified_generic_result(
            report, cast(str, generic_result_mode)
        )
        report_path = report.resolve()
        _apply_report(
            records,
            keys,
            report_path,
            applied_report_keys,
            replace_base=args.base is not None,
            verified_digest=verified_digest,
        )

    payload = b"".join(_stable_json_line(record) for record in records)
    digest = hashlib.sha256(payload).hexdigest()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(payload)
    counts = Counter(cast(str, record["outcome"]) for record in records)
    summary = {
        "schema": 1,
        "inventory": str(args.inventory.resolve()),
        "records": len(records),
        "outcomes": dict(sorted(counts.items())),
        "ledgerSha256": digest,
        "ledger": str(args.output.resolve()),
    }
    summary_path = args.output.with_suffix(".summary.json")
    summary_path.write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
