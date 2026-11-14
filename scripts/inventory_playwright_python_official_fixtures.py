#!/usr/bin/env python3
"""Inventory pytest fixture closures for every pinned Playwright Python row."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path
from typing import Protocol, TypedDict, cast

import pytest

_PLUGIN_OUTPUT = "PWRIGHT_PYTHON_FIXTURE_INVENTORY_OUTPUT"


class Manifest(TypedDict):
    schema: int
    repository: str
    tag: str
    commit: str


class CollectionProfile(TypedDict):
    name: str
    selection: str
    expandedTests: int


class FixtureRecord(TypedDict):
    id: str
    directFixtureNames: list[str]
    initialFixtureNames: list[str]
    fixtureClosure: list[str]


class _FixtureInfo(Protocol):
    argnames: tuple[str, ...]
    initialnames: tuple[str, ...]
    names_closure: list[str]


class _CollectedItem(Protocol):
    nodeid: str
    fixturenames: list[str]
    _fixtureinfo: _FixtureInfo


def _stable_json_line(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def _file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _git(upstream: Path, *args: str) -> str:
    completed = subprocess.run(
        ["git", "-C", str(upstream), *args],
        check=True,
        capture_output=True,
        text=True,
    )
    return completed.stdout.strip()


def _names(value: object, label: str) -> list[str]:
    if not isinstance(value, (list, tuple)):
        raise TypeError(f"{label} must be a list or tuple")
    raw_names = cast("list[object] | tuple[object, ...]", value)
    if not all(isinstance(name, str) and name for name in raw_names):
        raise TypeError(f"{label} must contain non-empty strings")
    names = cast(list[str], list(raw_names))
    if len(names) != len(set(names)):
        raise ValueError(f"{label} must not contain duplicates")
    return names


def _fixture_record(raw_item: object) -> FixtureRecord:
    item = cast(_CollectedItem, raw_item)
    node_id = getattr(item, "nodeid", None)
    if not isinstance(node_id, str) or not node_id:
        raise TypeError("collected item must have a non-empty nodeid")
    fixture_info = getattr(item, "_fixtureinfo", None)
    if fixture_info is None:
        raise TypeError(f"collected item lacks fixture metadata: {node_id}")
    direct = _names(getattr(fixture_info, "argnames", None), "direct fixture names")
    initial = _names(
        getattr(fixture_info, "initialnames", None), "initial fixture names"
    )
    closure = _names(getattr(fixture_info, "names_closure", None), "fixture closure")
    public_closure = _names(
        getattr(item, "fixturenames", None), "public fixture closure"
    )
    if closure != public_closure:
        raise ValueError(f"pytest fixture closure views differ: {node_id}")
    if not set(direct).issubset(closure) or not set(initial).issubset(closure):
        raise ValueError(f"fixture closure omits direct or initial names: {node_id}")
    return {
        "id": node_id,
        "directFixtureNames": direct,
        "initialFixtureNames": initial,
        "fixtureClosure": closure,
    }


@pytest.hookimpl(trylast=True)
def pytest_collection_modifyitems(items: list[pytest.Item]) -> None:
    """Write raw collection metadata when this module is loaded as a plugin."""

    output_value = os.environ.get(_PLUGIN_OUTPUT)
    if output_value is None:
        return
    records = [
        _fixture_record(item)
        for item in items
        if item.nodeid.startswith(("tests/", "examples/"))
    ]
    output = Path(output_value)
    output.write_bytes(b"".join(_stable_json_line(record) for record in records))


def _records(path: Path) -> list[dict[str, object]]:
    records: list[dict[str, object]] = []
    for line_number, line in enumerate(path.read_text().splitlines(), start=1):
        try:
            value = json.loads(line)
        except json.JSONDecodeError as error:
            raise SystemExit(
                f"invalid JSON on {path}:{line_number}: {error}"
            ) from error
        if not isinstance(value, dict) or not all(
            isinstance(key, str) for key in value
        ):
            raise SystemExit(f"record must be an object on {path}:{line_number}")
        records.append(cast(dict[str, object], value))
    return records


def _ordered_fixture_records(
    inventory_records: list[dict[str, object]], raw_records: list[dict[str, object]]
) -> list[FixtureRecord]:
    by_id: dict[str, FixtureRecord] = {}
    for index, raw_record in enumerate(raw_records):
        if set(raw_record) != {
            "id",
            "directFixtureNames",
            "initialFixtureNames",
            "fixtureClosure",
        }:
            raise SystemExit(f"fixture record {index} has unexpected fields")
        node_id = raw_record.get("id")
        if not isinstance(node_id, str) or not node_id:
            raise SystemExit(f"fixture record {index} lacks an ID")
        if node_id in by_id:
            raise SystemExit(f"fixture collection repeats node ID: {node_id}")
        by_id[node_id] = cast(FixtureRecord, raw_record)

    inventory_ids: list[str] = []
    for index, record in enumerate(inventory_records):
        node_id = record.get("id")
        if not isinstance(node_id, str) or not node_id:
            raise SystemExit(f"inventory record {index} lacks an ID")
        inventory_ids.append(node_id)
    missing = sorted(set(inventory_ids) - set(by_id))
    unexpected = sorted(set(by_id) - set(inventory_ids))
    if missing or unexpected or len(inventory_ids) != len(set(inventory_ids)):
        raise SystemExit(
            "fixture collection differs from canonical inventory: "
            f"missing={missing[:8]} unexpected={unexpected[:8]}"
        )
    return [by_id[node_id] for node_id in inventory_ids]


def _verify_direct_inputs(
    audit_records: list[dict[str, object]], fixture_records: list[FixtureRecord]
) -> None:
    if len(audit_records) != len(fixture_records):
        raise SystemExit("invocation audit count differs from fixture collection")
    for index, (audit_record, fixture_record) in enumerate(
        zip(audit_records, fixture_records)
    ):
        if audit_record.get("id") != fixture_record["id"]:
            raise SystemExit(f"invocation audit order differs at record {index}")
        parameters = _names(
            audit_record.get("pytestParameters"), "audit pytest parameters"
        )
        if parameters != fixture_record["directFixtureNames"]:
            raise SystemExit(
                "source and pytest direct inputs differ at "
                f"{fixture_record['id']}: {parameters} != "
                f"{fixture_record['directFixtureNames']}"
            )


def _verify_invocation_audit(path: Path, fixture_records: list[FixtureRecord]) -> str:
    summary_path = path.with_suffix(".summary.json")
    if not summary_path.is_file():
        raise SystemExit(f"invocation-audit summary is missing: {summary_path}")
    summary = cast(dict[str, object], json.loads(summary_path.read_text()))
    digest = _file_sha256(path)
    if summary.get("ledgerSha256") != digest:
        raise SystemExit("invocation-audit digest differs from its summary")
    audit_records = _records(path)
    _verify_direct_inputs(audit_records, fixture_records)
    return digest


def _verify_inventory(path: Path, commit: str) -> tuple[list[dict[str, object]], str]:
    summary_path = path.with_suffix(".summary.json")
    if not summary_path.is_file():
        raise SystemExit(f"inventory summary is missing: {summary_path}")
    summary = cast(dict[str, object], json.loads(summary_path.read_text()))
    digest = _file_sha256(path)
    if summary.get("commit") != commit or summary.get("ledgerSha256") != digest:
        raise SystemExit("canonical inventory pin or digest differs")
    records = _records(path)
    if summary.get("expandedTests") != len(records):
        raise SystemExit("canonical inventory count differs from its summary")
    return records, digest


def _inventory_profiles(path: Path) -> list[CollectionProfile]:
    summary_path = path.with_suffix(".summary.json")
    summary = cast(dict[str, object], json.loads(summary_path.read_text()))
    raw_profiles = summary.get("collectionProfiles")
    if not isinstance(raw_profiles, list):
        raise SystemExit("canonical inventory has no collection profiles")
    profiles: list[CollectionProfile] = []
    for index, raw_profile in enumerate(raw_profiles):
        if not isinstance(raw_profile, dict):
            raise SystemExit(f"collection profile {index} is not an object")
        name = raw_profile.get("name")
        selection = raw_profile.get("selection")
        expanded_tests = raw_profile.get("expandedTests")
        if (
            not isinstance(name, str)
            or not isinstance(selection, str)
            or not isinstance(expanded_tests, int)
        ):
            raise SystemExit(f"collection profile {index} is invalid")
        profiles.append(
            {
                "name": name,
                "selection": selection,
                "expandedTests": expanded_tests,
            }
        )
    if [profile["name"] for profile in profiles] != ["core", "examples"]:
        raise SystemExit("collection profiles must be ordered core then examples")
    return profiles


def _collect_profile(
    upstream: Path,
    python_executable: Path,
    profile: CollectionProfile,
) -> tuple[list[dict[str, object]], list[str]]:
    with tempfile.TemporaryDirectory(
        prefix=f"pwright-python-fixtures-{profile['name']}-"
    ) as directory:
        raw_output = Path(directory) / "raw.jsonl"
        environment = os.environ.copy()
        existing_python_path = environment.get("PYTHONPATH")
        python_paths = [str(Path(__file__).parent.resolve())]
        if existing_python_path:
            python_paths.append(existing_python_path)
        environment["PYTHONPATH"] = os.pathsep.join(python_paths)
        environment[_PLUGIN_OUTPUT] = str(raw_output)
        command = [
            str(python_executable),
            "-m",
            "pytest",
            "--collect-only",
            "-q",
            "-o",
            "addopts=",
            "-p",
            Path(__file__).stem,
            profile["selection"],
        ]
        completed = subprocess.run(
            command,
            cwd=upstream,
            env=environment,
            capture_output=True,
            text=True,
            check=False,
        )
        if completed.returncode:
            detail = completed.stderr.strip() or completed.stdout[-4000:].strip()
            raise SystemExit(
                f"Playwright Python {profile['name']} fixture collection failed:\n"
                f"{detail}"
            )
        if not raw_output.is_file():
            raise SystemExit(
                f"pytest fixture plugin did not produce {profile['name']} ledger"
            )
        records = _records(raw_output)
    if len(records) != profile["expandedTests"]:
        raise SystemExit(
            f"{profile['name']} fixture count differs from inventory profile"
        )
    return records, command


def main() -> None:
    root = Path(__file__).parents[1]
    parser = argparse.ArgumentParser()
    parser.add_argument("upstream", type=Path)
    parser.add_argument(
        "--manifest",
        type=Path,
        default=root / "packages/pwright-python/tests/official/manifest.json",
    )
    parser.add_argument(
        "--examples-python",
        type=Path,
        required=True,
        help="isolated Python environment for the official examples profile",
    )
    parser.add_argument(
        "--inventory",
        type=Path,
        default=root
        / ".work/official-conformance/playwright-python-v1.58.0-profiled.jsonl",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=root
        / ".work/official-conformance/playwright-python-v1.58.0-profiled-fixtures.jsonl",
    )
    parser.add_argument(
        "--invocation-audit",
        type=Path,
        required=True,
        help="6,526-row invocation audit containing source pytestParameters",
    )
    args = parser.parse_args()

    upstream = args.upstream.resolve()
    manifest = cast(Manifest, json.loads(args.manifest.read_text()))
    if manifest["schema"] != 1:
        raise SystemExit("unsupported Playwright Python manifest schema")
    if _git(upstream, "rev-parse", "HEAD") != manifest["commit"]:
        raise SystemExit("Playwright Python checkout does not match the pinned commit")
    if _git(upstream, "status", "--porcelain", "--untracked-files=no"):
        raise SystemExit("Playwright Python checkout has tracked source changes")
    inventory_records, inventory_digest = _verify_inventory(
        args.inventory.resolve(), manifest["commit"]
    )
    profiles = _inventory_profiles(args.inventory.resolve())
    python_by_profile = {
        "core": Path(sys.executable).absolute(),
        "examples": args.examples_python.absolute(),
    }
    raw_records: list[dict[str, object]] = []
    commands: list[dict[str, object]] = []
    for profile in profiles:
        profile_records, command = _collect_profile(
            upstream, python_by_profile[profile["name"]], profile
        )
        raw_records.extend(profile_records)
        commands.append({"name": profile["name"], "command": command})
    records = _ordered_fixture_records(inventory_records, raw_records)
    invocation_audit = args.invocation_audit.resolve()
    invocation_audit_digest = _verify_invocation_audit(invocation_audit, records)

    payload = b"".join(_stable_json_line(record) for record in records)
    digest = hashlib.sha256(payload).hexdigest()
    output = args.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(payload)

    direct_counts: Counter[str] = Counter()
    initial_counts: Counter[str] = Counter()
    closure_counts: Counter[str] = Counter()
    closure_size_counts: Counter[int] = Counter()
    for record in records:
        direct_counts.update(record["directFixtureNames"])
        initial_counts.update(record["initialFixtureNames"])
        closure_counts.update(record["fixtureClosure"])
        closure_size_counts[len(record["fixtureClosure"])] += 1
    summary = {
        "schema": 1,
        "repository": manifest["repository"],
        "tag": manifest["tag"],
        "commit": manifest["commit"],
        "inventory": str(args.inventory.resolve()),
        "inventorySha256": inventory_digest,
        "invocationAudit": str(invocation_audit),
        "invocationAuditSha256": invocation_audit_digest,
        "collector": str(Path(__file__).resolve()),
        "collectorSha256": _file_sha256(Path(__file__)),
        "pythonExecutable": sys.executable,
        "pythonPrefix": sys.prefix,
        "pythonVersion": platform.python_version(),
        "pytestVersion": pytest.__version__,
        "commands": commands,
        "records": len(records),
        "directFixtureNameCounts": dict(sorted(direct_counts.items())),
        "initialFixtureNameCounts": dict(sorted(initial_counts.items())),
        "closureFixtureNameCounts": dict(sorted(closure_counts.items())),
        "closureSizeCounts": {
            str(size): count for size, count in sorted(closure_size_counts.items())
        },
        "ledgerSha256": digest,
        "ledger": str(output),
    }
    summary_path = output.with_suffix(".summary.json")
    summary_path.write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
