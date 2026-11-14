#!/usr/bin/env python3
"""Inventory every expanded pytest node in a pinned Playwright Python checkout."""

from __future__ import annotations

import argparse
import ast
import hashlib
import json
import platform
import subprocess
import sys
from collections import Counter
from collections.abc import Mapping
from pathlib import Path
from typing import TypedDict, cast


class NonCollectedModule(TypedDict):
    path: str
    classification: str
    reason: str


class CollectionProfileRequired(TypedDict):
    name: str
    selection: str
    expandedTests: int


class CollectionProfile(CollectionProfileRequired, total=False):
    pythonVersion: str
    pytestVersion: str
    pytestPlaywrightVersion: str
    requirements: str
    requirementsSha256: str


class Corpus(TypedDict):
    policy: str
    testsTreeModules: int
    testsTreeTopLevelTestFunctions: int
    examplesTreeModules: int
    expandedTests: int
    collectionProfiles: list[CollectionProfile]
    defaultBrowsers: list[str]
    ciPlatforms: list[str]
    ciPythonVersions: list[str]
    stableChannels: list[str]
    nonCollectedModules: list[NonCollectedModule]


class Manifest(TypedDict):
    schema: int
    repository: str
    tag: str
    commit: str
    corpus: Corpus


class InventoryRecord(TypedDict):
    id: str
    file: str
    api: str
    sourceClassification: str
    privateImports: list[str]


def _git(upstream: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(upstream), *args],
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip()


def _stable_json_line(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def _private_playwright_imports(path: Path) -> list[str]:
    tree = ast.parse(path.read_bytes(), filename=str(path))
    imports: set[str] = set()
    for node in ast.walk(tree):
        names: list[str] = []
        if isinstance(node, ast.ImportFrom) and node.module:
            names.append(node.module)
        elif isinstance(node, ast.Import):
            names.extend(alias.name for alias in node.names)
        for name in names:
            parts = name.split(".")
            if parts[0] == "playwright" and any(part.startswith("_") for part in parts[1:]):
                imports.add(name)
    return sorted(imports)


def _file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _collection_node_ids(output: str, allowed_prefix: str) -> list[str]:
    node_ids = sorted(
        line.strip()
        for line in output.splitlines()
        if "::" in line and line.startswith(allowed_prefix)
    )
    if len(node_ids) != len(set(node_ids)):
        raise SystemExit(f"collection emitted duplicate node IDs: {allowed_prefix}")
    return node_ids


def _collect_profile(
    upstream: Path,
    python_executable: Path,
    profile: CollectionProfile,
) -> tuple[list[str], list[str]]:
    selection = profile["selection"]
    command = [
        str(python_executable),
        "-m",
        "pytest",
        "--collect-only",
        "-q",
        "-o",
        "addopts=",
        selection,
    ]
    result = subprocess.run(
        command,
        cwd=upstream,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode:
        detail = result.stderr.strip() or result.stdout[-4000:].strip()
        raise SystemExit(f"Playwright Python {profile['name']} collection failed:\n{detail}")
    node_ids = _collection_node_ids(result.stdout, f"{selection.rstrip('/')}/")
    expected = profile["expandedTests"]
    if len(node_ids) != expected:
        raise SystemExit(
            f"Playwright Python {profile['name']} count changed: "
            f"expected={expected} observed={len(node_ids)}"
        )
    return node_ids, command


def _runtime_identity(python_executable: Path) -> dict[str, object]:
    probe = """
import importlib.metadata
import json
import platform
import sys

def package_version(name):
    try:
        return importlib.metadata.version(name)
    except importlib.metadata.PackageNotFoundError:
        return None

print(json.dumps({
    "pythonExecutable": sys.executable,
    "pythonPrefix": sys.prefix,
    "pythonVersion": platform.python_version(),
    "playwrightVersion": package_version("playwright"),
    "pytestVersion": package_version("pytest"),
    "pytestPlaywrightVersion": package_version("pytest-playwright"),
}))
"""
    result = subprocess.run(
        [str(python_executable), "-c", probe],
        check=True,
        capture_output=True,
        text=True,
    )
    value = json.loads(result.stdout)
    if not isinstance(value, dict) or not all(isinstance(key, str) for key in value):
        raise SystemExit("collection runtime probe returned an invalid object")
    return cast(dict[str, object], value)


def _verify_profile_runtime(
    profile: CollectionProfile,
    identity: Mapping[str, object],
    manifest: Manifest,
) -> None:
    if identity.get("playwrightVersion") != manifest["tag"].removeprefix("v"):
        raise SystemExit(f"{profile['name']} Playwright runtime differs from pin")
    expected_python = profile.get("pythonVersion")
    actual_python = identity.get("pythonVersion")
    if expected_python and (
        not isinstance(actual_python, str) or not actual_python.startswith(f"{expected_python}.")
    ):
        raise SystemExit(f"{profile['name']} Python runtime differs from profile")
    for profile_key, identity_key in (
        ("pytestVersion", "pytestVersion"),
        ("pytestPlaywrightVersion", "pytestPlaywrightVersion"),
    ):
        expected = profile.get(profile_key)
        if expected is not None and identity.get(identity_key) != expected:
            raise SystemExit(f"{profile['name']} {profile_key} differs from profile")


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
        help="isolated Python 3.10 environment for the official examples job",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=root / ".work/official-conformance/playwright-python-v1.58.0-profiled.jsonl",
    )
    args = parser.parse_args()

    upstream = args.upstream.resolve()
    manifest = cast(Manifest, json.loads(args.manifest.read_text()))
    if manifest["schema"] != 1:
        raise SystemExit(f"unsupported manifest schema: {manifest['schema']}")
    if _git(upstream, "rev-parse", "HEAD") != manifest["commit"]:
        raise SystemExit("Playwright Python checkout does not match the pinned commit")
    if _git(upstream, "status", "--porcelain", "--untracked-files=no"):
        raise SystemExit("Playwright Python checkout has tracked source changes")

    profiles = manifest["corpus"]["collectionProfiles"]
    if [profile.get("name") for profile in profiles] != ["core", "examples"]:
        raise SystemExit("collection profiles must be ordered core then examples")
    profile_pythons = {
        "core": Path(sys.executable).absolute(),
        "examples": args.examples_python.absolute(),
    }
    profile_summaries: list[dict[str, object]] = []
    node_ids: list[str] = []
    for profile in profiles:
        name = profile["name"]
        python_executable = profile_pythons[name]
        identity = _runtime_identity(python_executable)
        _verify_profile_runtime(profile, identity, manifest)
        requirements_value = profile.get("requirements")
        requirements_digest = None
        if requirements_value:
            requirements = root / requirements_value
            requirements_digest = _file_sha256(requirements)
            if requirements_digest != profile.get("requirementsSha256"):
                raise SystemExit(f"{name} dependency lock differs from manifest")
        profile_node_ids, command = _collect_profile(upstream, python_executable, profile)
        node_ids.extend(profile_node_ids)
        profile_summaries.append(
            {
                "name": name,
                "selection": profile["selection"],
                "expandedTests": len(profile_node_ids),
                "nodeIdsSha256": hashlib.sha256("\n".join(profile_node_ids).encode()).hexdigest(),
                "command": command,
                "runtime": identity,
                "requirements": requirements_value,
                "requirementsSha256": requirements_digest,
            }
        )
    node_ids.sort()
    if len(node_ids) != len(set(node_ids)):
        raise SystemExit("Playwright Python profiles emitted duplicate node IDs")
    expected = manifest["corpus"]["expandedTests"]
    if len(node_ids) != expected:
        raise SystemExit(
            f"Playwright Python expanded test count changed: "
            f"expected={expected} observed={len(node_ids)}"
        )

    source_files = {
        path.relative_to(upstream).as_posix()
        for directory in (upstream / "tests", upstream / "examples")
        for path in directory.rglob("test_*.py")
    }
    private_imports = {
        source_file: _private_playwright_imports(upstream / source_file)
        for source_file in source_files
    }
    records: list[InventoryRecord] = [
        {
            "id": node_id,
            "file": node_id.split("::", maxsplit=1)[0],
            "api": (
                "async"
                if node_id.startswith("tests/async/")
                else "sync"
                if node_id.startswith("tests/sync/")
                else "example"
                if node_id.startswith("examples/")
                else "shared"
            ),
            "sourceClassification": (
                "upstream-private-import"
                if private_imports[node_id.split("::", maxsplit=1)[0]]
                else "public-api-source"
            ),
            "privateImports": private_imports[node_id.split("::", maxsplit=1)[0]],
        }
        for node_id in node_ids
    ]
    covered_files = {record["file"] for record in records}
    non_collected = manifest["corpus"].get("nonCollectedModules", [])
    accounted_non_collected = {entry["path"] for entry in non_collected}
    if len(accounted_non_collected) != len(non_collected):
        raise SystemExit("non-collected Python module paths must be unique")
    missing_files = sorted(source_files - covered_files - accounted_non_collected)
    unexpected_files = sorted(covered_files - source_files)
    invalid_non_collected = sorted(accounted_non_collected - source_files)
    collected_exclusions = sorted(accounted_non_collected & covered_files)
    if missing_files or unexpected_files or invalid_non_collected or collected_exclusions:
        raise SystemExit(
            "Python source coverage differs from the source module inventory: "
            f"missing={missing_files} unexpected={unexpected_files} "
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
        "expandedTests": len(records),
        "sourceModules": len(source_files),
        "collectedFiles": len(covered_files),
        "nonCollectedModules": non_collected,
        "apiCounts": {
            api: sum(record["api"] == api for record in records)
            for api in ("async", "sync", "shared", "example")
        },
        "sourceClassificationCounts": dict(
            sorted(Counter(record["sourceClassification"] for record in records).items())
        ),
        "collectionProfiles": profile_summaries,
        "inventoryPythonExecutable": sys.executable,
        "inventoryPythonVersion": platform.python_version(),
        "defaultBrowsers": manifest["corpus"]["defaultBrowsers"],
        "ciPlatforms": manifest["corpus"]["ciPlatforms"],
        "ciPythonVersions": manifest["corpus"]["ciPythonVersions"],
        "stableChannels": manifest["corpus"]["stableChannels"],
        "ledgerSha256": digest,
        "ledger": str(args.output),
    }
    summary_path = args.output.with_suffix(".summary.json")
    summary_path.write_text(json.dumps(summary, indent=2) + "\n")
    print(
        f"official Playwright Python inventory: {len(records)} expanded tests / "
        f"{summary['collectedFiles']} files / sha256={digest}"
    )
    print(f"ledger={args.output}")
    print(f"summary={summary_path}")


if __name__ == "__main__":
    main()
