#!/usr/bin/env python3
"""Resolve every official Python fixture closure name to its definitions."""

from __future__ import annotations

import argparse
import hashlib
import inspect
import json
import os
import platform
import subprocess
import sys
import tempfile
from collections import Counter
from collections.abc import Callable, Mapping, Sequence
from pathlib import Path
from typing import Protocol, TypedDict, cast

import pytest
from inventory_playwright_python_official_fixtures import (
    CollectionProfile,
    _file_sha256,
    _git,
    _inventory_profiles,
    _records,
    _stable_json_line,
    _verify_inventory,
)

_PLUGIN_BINDINGS_OUTPUT = "PWRIGHT_PYTHON_FIXTURE_BINDINGS_OUTPUT"
_PLUGIN_DEFINITIONS_OUTPUT = "PWRIGHT_PYTHON_FIXTURE_DEFINITIONS_OUTPUT"
_PLUGIN_UPSTREAM = "PWRIGHT_PYTHON_FIXTURE_UPSTREAM"


class Manifest(TypedDict):
    schema: int
    repository: str
    tag: str
    commit: str


class DefinitionRecord(TypedDict):
    id: str
    fixtureName: str
    scope: str
    baseId: str
    originKind: str
    originPath: str
    module: str
    line: int | None
    function: str
    dependencies: list[str]
    autouse: bool
    parameterized: bool
    parameterCount: int
    semanticRole: str


class FixtureBinding(TypedDict):
    name: str
    definitionIds: list[str]
    selectedDefinitionId: str


class BindingRecord(TypedDict):
    id: str
    fixtureRuntimeBand: str
    fixtureServerBand: str
    fixtureBindings: list[FixtureBinding]


class _FixtureDefinition(Protocol):
    argname: str
    baseid: str
    func: Callable[..., object]
    scope: str
    argnames: tuple[str, ...]
    params: Sequence[object] | None
    _autouse: bool


class _FixtureInfo(Protocol):
    names_closure: list[str]
    name2fixturedefs: dict[str, Sequence[_FixtureDefinition] | None]


class _CollectedItem(Protocol):
    nodeid: str
    _fixtureinfo: _FixtureInfo


_ROLE_BY_FIXTURE_NAME = {
    "_artifacts_recorder": "artifact-lifecycle",
    "_client_certificate_server": "fixture-server",
    "_pw_artifacts_folder": "artifact-lifecycle",
    "_skip_webkit_darwin": "platform-gate",
    "_verify_url": "navigation-target",
    "after_each_hook": "test-lifecycle",
    "assert_to_be_golden": "assertion-support",
    "assetdir": "assertion-support",
    "base_url": "navigation-target",
    "before_each": "test-lifecycle",
    "browser": "browser-object-lifecycle",
    "browser_channel": "browser-selection",
    "browser_context_args": "browser-selection",
    "browser_factory": "browser-launch-factory",
    "browser_name": "browser-selection",
    "browser_type": "browser-runtime",
    "browser_type_launch_args": "browser-selection",
    "browser_version": "browser-observation",
    "calls": "test-local-support",
    "context": "browser-object-lifecycle",
    "context_factory": "browser-launch-factory",
    "connect_options": "browser-selection",
    "default_same_site_cookie_value": "browser-expectation",
    "delete_output_dir": "artifact-lifecycle",
    "device": "browser-selection",
    "event_loop_policy": "pytest-runtime",
    "fails_on_401": "browser-expectation",
    "headless": "browser-selection",
    "https_server": "fixture-server",
    "is_chromium": "platform-gate",
    "is_firefox": "platform-gate",
    "is_headless_shell": "platform-gate",
    "is_linux": "platform-gate",
    "is_mac": "platform-gate",
    "is_webkit": "platform-gate",
    "is_win": "platform-gate",
    "launch_arguments": "browser-selection",
    "launch_browser": "browser-launch-factory",
    "launch_persistent": "browser-launch-factory",
    "launch_persistent_context": "browser-launch-factory",
    "launch_server": "browser-launch-factory",
    "method": "test-local-support",
    "new_context": "browser-launch-factory",
    "output_path": "artifact-lifecycle",
    "page": "browser-object-lifecycle",
    "playwright": "browser-runtime",
    "pytestconfig": "pytest-runtime",
    "request": "pytest-runtime",
    "run_around_tests": "test-lifecycle",
    "selectors": "browser-runtime",
    "serialization": "test-local-support",
    "server": "fixture-server",
    "show_trace_viewer": "diagnostic-support",
    "skip_by_browser": "platform-gate",
    "skip_by_platform": "platform-gate",
    "start_server": "fixture-server",
    "sync_gather": "browser-runtime",
    "tmp_path": "pytest-runtime",
    "tmp_path_factory": "pytest-runtime",
    "utils": "test-local-support",
}


def _normalized_origin(
    source: Path | None,
    upstream: Path,
    prefix: Path,
    base_prefix: Path,
) -> tuple[str, str]:
    if source is None:
        return "generated", ""
    resolved = source.resolve()
    for kind, root in (
        ("upstream", upstream.resolve()),
        ("environment", prefix.resolve()),
        ("standard-library", base_prefix.resolve()),
    ):
        try:
            return kind, resolved.relative_to(root).as_posix()
        except ValueError:
            pass
    parts = resolved.parts
    if "site-packages" in parts:
        index = parts.index("site-packages")
        return "installed-package", Path(*parts[index + 1 :]).as_posix()
    return "external", resolved.as_posix()


def _semantic_role(fixture_name: str, module: str, function: str) -> str:
    if module == "_pytest.python" and function.endswith(
        "get_direct_param_fixture_func"
    ):
        return "parameter-value"
    try:
        return _ROLE_BY_FIXTURE_NAME[fixture_name]
    except KeyError as error:
        raise ValueError(
            f"fixture lacks an explicit semantic role: {fixture_name}"
        ) from error


def _definition_payload(
    fixture_name: str,
    raw_definition: _FixtureDefinition,
    upstream: Path,
    prefix: Path,
    base_prefix: Path,
) -> dict[str, object]:
    if raw_definition.argname != fixture_name:
        raise ValueError(
            f"fixture definition name differs: {fixture_name} != "
            f"{raw_definition.argname}"
        )
    function = inspect.unwrap(raw_definition.func)
    module = getattr(function, "__module__", "")
    qualname = getattr(function, "__qualname__", getattr(function, "__name__", ""))
    source: Path | None = None
    line: int | None = None
    try:
        source_value = inspect.getsourcefile(function)
        if source_value:
            source = Path(source_value)
        line = inspect.getsourcelines(function)[1]
    except (OSError, TypeError):
        pass
    origin_kind, origin_path = _normalized_origin(source, upstream, prefix, base_prefix)
    params = raw_definition.params
    return {
        "fixtureName": fixture_name,
        "scope": raw_definition.scope,
        "baseId": raw_definition.baseid,
        "originKind": origin_kind,
        "originPath": origin_path,
        "module": module,
        "line": line,
        "function": qualname,
        "dependencies": list(raw_definition.argnames),
        "autouse": bool(raw_definition._autouse),
        "parameterized": params is not None,
        "parameterCount": 0 if params is None else len(params),
        "semanticRole": _semantic_role(fixture_name, module, qualname),
    }


def _definition_record(payload: Mapping[str, object]) -> DefinitionRecord:
    digest = hashlib.sha256(_stable_json_line(payload)).hexdigest()
    return cast(DefinitionRecord, {"id": f"fixturedef:{digest}", **payload})


def _request_definition() -> DefinitionRecord:
    payload: dict[str, object] = {
        "fixtureName": "request",
        "scope": "function",
        "baseId": "",
        "originKind": "pytest-builtin",
        "originPath": "",
        "module": "_pytest.fixtures",
        "line": None,
        "function": "request",
        "dependencies": [],
        "autouse": False,
        "parameterized": False,
        "parameterCount": 0,
        "semanticRole": "pytest-runtime",
    }
    return _definition_record(payload)


def _unresolved_definition(fixture_name: str) -> DefinitionRecord:
    payload: dict[str, object] = {
        "fixtureName": fixture_name,
        "scope": "unknown",
        "baseId": "",
        "originKind": "unresolved",
        "originPath": "",
        "module": "",
        "line": None,
        "function": fixture_name,
        "dependencies": [],
        "autouse": False,
        "parameterized": False,
        "parameterCount": 0,
        "semanticRole": _semantic_role(fixture_name, "", fixture_name),
    }
    return _definition_record(payload)


def _fixture_runtime_band(
    names: Sequence[str], unresolved_names: Sequence[str] = ()
) -> str:
    if unresolved_names:
        return "unresolved-fixture-definition"
    closure = set(names)
    if "page" in closure:
        return "fixture-creates-page"
    if "context" in closure:
        return "fixture-creates-context"
    if "browser" in closure:
        return "fixture-launches-browser"
    if "browser_type" in closure:
        return "browser-type-without-fixture-launch"
    if "playwright" in closure:
        return "playwright-runtime-only"
    return "no-playwright-runtime-fixture"


def _fixture_server_band(names: Sequence[str]) -> str:
    closure = set(names)
    explicit = closure.intersection(
        {"server", "https_server", "_client_certificate_server"}
    )
    if "start_server" in closure and explicit:
        return "autouse-plus-explicit-server"
    if "start_server" in closure:
        return "autouse-server-only"
    if explicit:
        return "explicit-server-without-autouse"
    return "no-server-fixture"


def _item_records(
    raw_item: object, upstream: Path
) -> tuple[BindingRecord, dict[str, DefinitionRecord]]:
    item = cast(_CollectedItem, raw_item)
    fixture_info = item._fixtureinfo
    definitions: dict[str, DefinitionRecord] = {}
    bindings: list[FixtureBinding] = []
    for name in fixture_info.names_closure:
        raw_definitions = fixture_info.name2fixturedefs.get(name)
        resolved: list[DefinitionRecord]
        if raw_definitions:
            resolved = [
                _definition_record(
                    _definition_payload(
                        name,
                        raw_definition,
                        upstream,
                        Path(sys.prefix),
                        Path(sys.base_prefix),
                    )
                )
                for raw_definition in raw_definitions
            ]
        elif name == "request":
            resolved = [_request_definition()]
        else:
            resolved = [_unresolved_definition(name)]
        for definition in resolved:
            previous = definitions.setdefault(definition["id"], definition)
            if previous != definition:
                raise ValueError(f"fixture definition ID collision: {definition['id']}")
        definition_ids = [definition["id"] for definition in resolved]
        bindings.append(
            {
                "name": name,
                "definitionIds": definition_ids,
                "selectedDefinitionId": definition_ids[-1],
            }
        )
    names = [binding["name"] for binding in bindings]
    unresolved_names = [
        binding["name"]
        for binding in bindings
        if definitions[binding["selectedDefinitionId"]]["originKind"] == "unresolved"
    ]
    return (
        {
            "id": item.nodeid,
            "fixtureRuntimeBand": _fixture_runtime_band(names, unresolved_names),
            "fixtureServerBand": _fixture_server_band(names),
            "fixtureBindings": bindings,
        },
        definitions,
    )


@pytest.hookimpl(trylast=True)
def pytest_collection_modifyitems(items: list[pytest.Item]) -> None:
    """Write binding and definition ledgers when loaded as a pytest plugin."""

    bindings_value = os.environ.get(_PLUGIN_BINDINGS_OUTPUT)
    definitions_value = os.environ.get(_PLUGIN_DEFINITIONS_OUTPUT)
    upstream_value = os.environ.get(_PLUGIN_UPSTREAM)
    if bindings_value is None and definitions_value is None and upstream_value is None:
        return
    if bindings_value is None or definitions_value is None or upstream_value is None:
        raise RuntimeError("fixture-definition plugin environment is incomplete")
    definitions: dict[str, DefinitionRecord] = {}
    bindings: list[BindingRecord] = []
    upstream = Path(upstream_value).resolve()
    for item in items:
        if not item.nodeid.startswith(("tests/", "examples/")):
            continue
        binding, item_definitions = _item_records(item, upstream)
        bindings.append(binding)
        for definition_id, definition in item_definitions.items():
            previous = definitions.setdefault(definition_id, definition)
            if previous != definition:
                raise RuntimeError(f"fixture definition differs: {definition_id}")
    Path(bindings_value).write_bytes(
        b"".join(_stable_json_line(record) for record in bindings)
    )
    Path(definitions_value).write_bytes(
        b"".join(
            _stable_json_line(definitions[definition_id])
            for definition_id in sorted(definitions)
        )
    )


def _verified_fixture_closure(
    path: Path, inventory_records: list[dict[str, object]]
) -> tuple[list[dict[str, object]], str]:
    summary_path = path.with_suffix(".summary.json")
    summary = cast(dict[str, object], json.loads(summary_path.read_text()))
    digest = _file_sha256(path)
    if summary.get("ledgerSha256") != digest:
        raise SystemExit("fixture-closure digest differs from its summary")
    records = _records(path)
    inventory_ids = [record.get("id") for record in inventory_records]
    closure_ids = [record.get("id") for record in records]
    if inventory_ids != closure_ids:
        raise SystemExit("fixture closure differs from canonical inventory order")
    return records, digest


def _verified_ordered_ledger(
    path: Path,
    inventory_records: list[dict[str, object]],
    label: str,
) -> tuple[list[dict[str, object]], str]:
    summary_path = path.with_suffix(".summary.json")
    summary = cast(dict[str, object], json.loads(summary_path.read_text()))
    digest = _file_sha256(path)
    if summary.get("ledgerSha256") != digest:
        raise SystemExit(f"{label} digest differs from its summary")
    records = _records(path)
    inventory_ids = [record.get("id") for record in inventory_records]
    record_ids = [record.get("id") for record in records]
    if inventory_ids != record_ids:
        raise SystemExit(f"{label} differs from canonical inventory order")
    return records, digest


def _applicability_outcome_counts(
    bindings: Sequence[BindingRecord],
    audit_records: Sequence[Mapping[str, object]],
    outcome_records: Sequence[Mapping[str, object]],
    band_field: str,
) -> dict[str, dict[str, dict[str, int]]]:
    counts: Counter[tuple[str, str, str]] = Counter()
    for binding, audit, outcome in zip(bindings, audit_records, outcome_records):
        if binding["id"] != audit.get("id") or binding["id"] != outcome.get("id"):
            raise SystemExit("fixture status join differs from canonical order")
        band = binding.get(band_field)
        applicability = audit.get("applicability")
        outcome_value = outcome.get("outcome")
        if not all(
            isinstance(value, str) and value
            for value in (band, applicability, outcome_value)
        ):
            raise SystemExit("fixture status join has an invalid classification")
        counts[cast(tuple[str, str, str], (band, applicability, outcome_value))] += 1
    nested: dict[str, dict[str, dict[str, int]]] = {}
    for (band, applicability, outcome), count in sorted(counts.items()):
        nested.setdefault(band, {}).setdefault(applicability, {})[outcome] = count
    return nested


def _ordered_bindings(
    closure_records: list[dict[str, object]], raw_records: list[dict[str, object]]
) -> list[BindingRecord]:
    by_id: dict[str, BindingRecord] = {}
    for index, raw_record in enumerate(raw_records):
        node_id = raw_record.get("id")
        if not isinstance(node_id, str) or not node_id:
            raise SystemExit(f"fixture binding {index} lacks an ID")
        if node_id in by_id:
            raise SystemExit(f"fixture bindings repeat node ID: {node_id}")
        by_id[node_id] = cast(BindingRecord, raw_record)
    ordered: list[BindingRecord] = []
    for index, closure_record in enumerate(closure_records):
        node_id = closure_record.get("id")
        if not isinstance(node_id, str) or node_id not in by_id:
            raise SystemExit(f"fixture binding is missing at record {index}")
        binding = by_id[node_id]
        names = [entry["name"] for entry in binding["fixtureBindings"]]
        if names != closure_record.get("fixtureClosure"):
            raise SystemExit(f"fixture binding closure differs: {node_id}")
        ordered.append(binding)
    if len(ordered) != len(by_id):
        raise SystemExit("fixture bindings contain unexpected node IDs")
    return ordered


def _verified_definitions(
    raw_records: Sequence[Mapping[str, object]], bindings: list[BindingRecord]
) -> list[DefinitionRecord]:
    definitions: dict[str, DefinitionRecord] = {}
    for index, raw_record in enumerate(raw_records):
        definition_id = raw_record.get("id")
        if not isinstance(definition_id, str) or not definition_id:
            raise SystemExit(f"fixture definition {index} lacks an ID")
        definition = cast(DefinitionRecord, dict(raw_record))
        previous = definitions.setdefault(definition_id, definition)
        if previous != definition:
            raise SystemExit(f"fixture definition ID collision: {definition_id}")
    selected_ids: set[str] = set()
    all_ids: set[str] = set()
    for record in bindings:
        names = {binding["name"] for binding in record["fixtureBindings"]}
        for binding in record["fixtureBindings"]:
            definition_ids = binding["definitionIds"]
            if (
                not definition_ids
                or binding["selectedDefinitionId"] != definition_ids[-1]
            ):
                raise SystemExit(f"invalid fixture resolution chain: {record['id']}")
            all_ids.update(definition_ids)
            selected_ids.add(binding["selectedDefinitionId"])
            selected = definitions.get(binding["selectedDefinitionId"])
            if selected is None:
                raise SystemExit(
                    f"fixture binding references unknown definition: {record['id']}"
                )
            missing_dependencies = set(selected["dependencies"]) - names
            if missing_dependencies:
                raise SystemExit(
                    f"selected fixture dependencies are absent from closure: "
                    f"{record['id']} {sorted(missing_dependencies)}"
                )
    if all_ids != set(definitions) or not selected_ids:
        raise SystemExit("fixture definition registry has missing or unused entries")
    return [definitions[definition_id] for definition_id in sorted(definitions)]


def _collect_profile(
    upstream: Path,
    python_executable: Path,
    profile: CollectionProfile,
) -> tuple[list[dict[str, object]], list[dict[str, object]], list[str]]:
    with tempfile.TemporaryDirectory(
        prefix=f"pwright-python-fixture-defs-{profile['name']}-"
    ) as directory:
        raw_bindings = Path(directory) / "bindings.jsonl"
        raw_definitions = Path(directory) / "definitions.jsonl"
        environment = os.environ.copy()
        existing_python_path = environment.get("PYTHONPATH")
        python_paths = [str(Path(__file__).parent.resolve())]
        if existing_python_path:
            python_paths.append(existing_python_path)
        environment["PYTHONPATH"] = os.pathsep.join(python_paths)
        environment[_PLUGIN_BINDINGS_OUTPUT] = str(raw_bindings)
        environment[_PLUGIN_DEFINITIONS_OUTPUT] = str(raw_definitions)
        environment[_PLUGIN_UPSTREAM] = str(upstream)
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
                f"Playwright Python {profile['name']} fixture definition "
                f"collection failed:\n{detail}"
            )
        if not raw_bindings.is_file() or not raw_definitions.is_file():
            raise SystemExit(
                f"pytest plugin did not produce {profile['name']} definition ledgers"
            )
        bindings = _records(raw_bindings)
        definitions = _records(raw_definitions)
    if len(bindings) != profile["expandedTests"]:
        raise SystemExit(
            f"{profile['name']} fixture definition count differs from profile"
        )
    return bindings, definitions, command


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
        "--inventory",
        type=Path,
        default=root
        / ".work/official-conformance/playwright-python-v1.58.0-profiled.jsonl",
    )
    parser.add_argument("--fixture-closure", type=Path, required=True)
    parser.add_argument("--invocation-audit", type=Path, required=True)
    parser.add_argument("--outcomes", type=Path, required=True)
    parser.add_argument("--examples-python", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--definitions-output", type=Path, required=True)
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
    fixture_closure = args.fixture_closure.resolve()
    closure_records, closure_digest = _verified_fixture_closure(
        fixture_closure, inventory_records
    )
    invocation_audit = args.invocation_audit.resolve()
    audit_records, audit_digest = _verified_ordered_ledger(
        invocation_audit, inventory_records, "invocation audit"
    )
    outcomes = args.outcomes.resolve()
    outcome_records, outcome_digest = _verified_ordered_ledger(
        outcomes, inventory_records, "outcome ledger"
    )

    profiles = _inventory_profiles(args.inventory.resolve())
    python_by_profile = {
        "core": Path(sys.executable).absolute(),
        "examples": args.examples_python.absolute(),
    }
    raw_bindings: list[dict[str, object]] = []
    raw_definitions: list[dict[str, object]] = []
    commands: list[dict[str, object]] = []
    for profile in profiles:
        profile_bindings, profile_definitions, command = _collect_profile(
            upstream, python_by_profile[profile["name"]], profile
        )
        raw_bindings.extend(profile_bindings)
        raw_definitions.extend(profile_definitions)
        commands.append({"name": profile["name"], "command": command})
    bindings = _ordered_bindings(closure_records, raw_bindings)
    definitions = _verified_definitions(raw_definitions, bindings)

    bindings_payload = b"".join(_stable_json_line(record) for record in bindings)
    definitions_payload = b"".join(_stable_json_line(record) for record in definitions)
    output = args.output.resolve()
    definitions_output = args.definitions_output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    definitions_output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(bindings_payload)
    definitions_output.write_bytes(definitions_payload)

    runtime_band_counts: Counter[str] = Counter()
    server_band_counts: Counter[str] = Counter()
    selected_definition_counts: Counter[str] = Counter()
    resolution_depth_counts: Counter[int] = Counter()
    for record in bindings:
        runtime_band_counts[record["fixtureRuntimeBand"]] += 1
        server_band_counts[record["fixtureServerBand"]] += 1
        for binding in record["fixtureBindings"]:
            selected_definition_counts[binding["selectedDefinitionId"]] += 1
            resolution_depth_counts[len(binding["definitionIds"])] += 1
    scope_counts = Counter(record["scope"] for record in definitions)
    origin_counts = Counter(record["originKind"] for record in definitions)
    role_counts = Counter(record["semanticRole"] for record in definitions)
    fixture_definition_counts = Counter(record["fixtureName"] for record in definitions)
    runtime_status_counts = _applicability_outcome_counts(
        bindings,
        audit_records,
        outcome_records,
        "fixtureRuntimeBand",
    )
    server_status_counts = _applicability_outcome_counts(
        bindings,
        audit_records,
        outcome_records,
        "fixtureServerBand",
    )
    unresolved_ids = {
        record["id"] for record in definitions if record["originKind"] == "unresolved"
    }
    unresolved_rows = [
        record["id"]
        for record in bindings
        if any(
            binding["selectedDefinitionId"] in unresolved_ids
            for binding in record["fixtureBindings"]
        )
    ]
    summary = {
        "schema": 1,
        "repository": manifest["repository"],
        "tag": manifest["tag"],
        "commit": manifest["commit"],
        "inventory": str(args.inventory.resolve()),
        "inventorySha256": inventory_digest,
        "fixtureClosure": str(fixture_closure),
        "fixtureClosureSha256": closure_digest,
        "invocationAudit": str(invocation_audit),
        "invocationAuditSha256": audit_digest,
        "outcomes": str(outcomes),
        "outcomesSha256": outcome_digest,
        "collector": str(Path(__file__).resolve()),
        "collectorSha256": _file_sha256(Path(__file__)),
        "pythonExecutable": sys.executable,
        "pythonPrefix": sys.prefix,
        "pythonVersion": platform.python_version(),
        "pytestVersion": pytest.__version__,
        "commands": commands,
        "records": len(bindings),
        "definitions": len(definitions),
        "runtimeBandCounts": dict(sorted(runtime_band_counts.items())),
        "runtimeBandApplicabilityOutcomeCounts": runtime_status_counts,
        "serverBandCounts": dict(sorted(server_band_counts.items())),
        "serverBandApplicabilityOutcomeCounts": server_status_counts,
        "scopeCounts": dict(sorted(scope_counts.items())),
        "originKindCounts": dict(sorted(origin_counts.items())),
        "semanticRoleDefinitionCounts": dict(sorted(role_counts.items())),
        "fixtureDefinitionCounts": dict(sorted(fixture_definition_counts.items())),
        "resolutionDepthCounts": {
            str(depth): count
            for depth, count in sorted(resolution_depth_counts.items())
        },
        "autouseDefinitions": sum(record["autouse"] for record in definitions),
        "parameterizedDefinitions": sum(
            record["parameterized"] for record in definitions
        ),
        "unresolvedDefinitions": sum(
            record["originKind"] == "unresolved" for record in definitions
        ),
        "unresolvedFixtureNames": sorted(
            record["fixtureName"]
            for record in definitions
            if record["originKind"] == "unresolved"
        ),
        "unresolvedRows": unresolved_rows,
        "selectedDefinitionCounts": dict(sorted(selected_definition_counts.items())),
        "bindingsLedger": str(output),
        "bindingsLedgerSha256": hashlib.sha256(bindings_payload).hexdigest(),
        "definitionsLedger": str(definitions_output),
        "definitionsLedgerSha256": hashlib.sha256(definitions_payload).hexdigest(),
    }
    summary_path = output.with_suffix(".summary.json")
    summary_path.write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
