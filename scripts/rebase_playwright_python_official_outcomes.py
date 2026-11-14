#!/usr/bin/env python3
"""Rebase Python outcomes onto the corrected profiled official inventory."""

from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter
from pathlib import Path
from typing import TypedDict, cast

GENERATED_FIELDS = frozenset(
    {"attempts", "durationSeconds", "error", "outcome", "resultFile"}
)


class RebaseResult(TypedDict):
    records: list[dict[str, object]]
    rebasedIds: int


def _stable_json_line(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def _file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _records(path: Path) -> list[dict[str, object]]:
    records: list[dict[str, object]] = []
    for line_number, line in enumerate(path.read_text().splitlines(), start=1):
        value = json.loads(line)
        if not isinstance(value, dict) or not all(
            isinstance(key, str) for key in value
        ):
            raise SystemExit(f"invalid object at {path}:{line_number}")
        records.append(cast(dict[str, object], value))
    return records


def _verified_ledger(path: Path, count_key: str) -> tuple[list[dict[str, object]], str]:
    summary_path = path.with_suffix(".summary.json")
    summary = cast(dict[str, object], json.loads(summary_path.read_text()))
    digest = _file_sha256(path)
    if summary.get("ledgerSha256") != digest:
        raise SystemExit(f"ledger digest differs from summary: {path}")
    records = _records(path)
    if summary.get(count_key) != len(records):
        raise SystemExit(f"ledger count differs from summary: {path}")
    return records, digest


def _source_id(target_id: str, source_ids: set[str]) -> tuple[str, bool]:
    if target_id in source_ids:
        return target_id, False
    suffix = "[chromium]"
    if target_id.startswith("examples/todomvc/") and target_id.endswith(suffix):
        candidate = target_id[: -len(suffix)]
        if candidate in source_ids:
            return candidate, True
    raise SystemExit(f"target inventory ID has no exact source outcome: {target_id}")


def _inventory_fields(record: dict[str, object]) -> dict[str, object]:
    return {key: value for key, value in record.items() if key not in GENERATED_FIELDS}


def _rebase_records(
    source_inventory: list[dict[str, object]],
    target_inventory: list[dict[str, object]],
    source_outcomes: list[dict[str, object]],
) -> RebaseResult:
    if len(source_inventory) != len(source_outcomes):
        raise SystemExit("source inventory and outcomes have different counts")
    source_by_id: dict[str, dict[str, object]] = {}
    for index, (inventory, outcome) in enumerate(
        zip(source_inventory, source_outcomes)
    ):
        if _inventory_fields(outcome) != inventory:
            raise SystemExit(f"source outcome differs from inventory at record {index}")
        node_id = inventory.get("id")
        if not isinstance(node_id, str) or node_id in source_by_id:
            raise SystemExit(f"source inventory has invalid ID at record {index}")
        source_by_id[node_id] = outcome

    consumed: set[str] = set()
    rebased_ids = 0
    records: list[dict[str, object]] = []
    for index, inventory in enumerate(target_inventory):
        target_id = inventory.get("id")
        if not isinstance(target_id, str):
            raise SystemExit(f"target inventory has invalid ID at record {index}")
        source_id, rebased = _source_id(target_id, set(source_by_id))
        if source_id in consumed:
            raise SystemExit(f"multiple target IDs consume source outcome: {source_id}")
        consumed.add(source_id)
        source_outcome = source_by_id[source_id]
        if rebased:
            generated = set(source_outcome) & GENERATED_FIELDS
            if (
                source_outcome.get("outcome") != "not-run"
                or source_outcome.get("attempts") != 0
                or generated != {"attempts", "outcome"}
            ):
                raise SystemExit(
                    f"only pristine not-run example outcomes may be rebased: {source_id}"
                )
            rebased_ids += 1
        record = dict(inventory)
        for field in GENERATED_FIELDS:
            if field in source_outcome:
                record[field] = source_outcome[field]
        records.append(record)
    if consumed != set(source_by_id):
        raise SystemExit("source outcomes contain IDs absent from target inventory")
    return {"records": records, "rebasedIds": rebased_ids}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-inventory", type=Path, required=True)
    parser.add_argument("--target-inventory", type=Path, required=True)
    parser.add_argument("--source-outcomes", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    source_inventory, source_inventory_digest = _verified_ledger(
        args.source_inventory.resolve(), "expandedTests"
    )
    target_inventory, target_inventory_digest = _verified_ledger(
        args.target_inventory.resolve(), "expandedTests"
    )
    source_outcomes, source_outcomes_digest = _verified_ledger(
        args.source_outcomes.resolve(), "records"
    )
    result = _rebase_records(source_inventory, target_inventory, source_outcomes)
    records = result["records"]
    if result["rebasedIds"] != 25:
        raise SystemExit(
            f"profile correction must rebase exactly 25 IDs: {result['rebasedIds']}"
        )
    payload = b"".join(_stable_json_line(record) for record in records)
    digest = hashlib.sha256(payload).hexdigest()
    output = args.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(payload)
    outcomes = Counter(cast(str, record["outcome"]) for record in records)
    summary = {
        "schema": 1,
        "sourceInventory": str(args.source_inventory.resolve()),
        "sourceInventorySha256": source_inventory_digest,
        "targetInventory": str(args.target_inventory.resolve()),
        "targetInventorySha256": target_inventory_digest,
        "sourceOutcomes": str(args.source_outcomes.resolve()),
        "sourceOutcomesSha256": source_outcomes_digest,
        "records": len(records),
        "rebasedIds": result["rebasedIds"],
        "outcomes": dict(sorted(outcomes.items())),
        "ledgerSha256": digest,
        "ledger": str(output),
    }
    summary_path = output.with_suffix(".summary.json")
    summary_path.write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
