#!/usr/bin/env python3
"""Verify that the selected upstream Playwright Python corpus is exact."""

from __future__ import annotations

import argparse
import ast
import hashlib
import json
import subprocess
from collections.abc import Mapping
from pathlib import Path
from typing import TypedDict, cast


class Case(TypedDict):
    id: str
    capabilities: list[str]


class Corpus(TypedDict):
    testsTreeModules: int
    testsTreeTopLevelTestFunctions: int
    examplesTreeModules: int
    expandedTests: int


class ExecutionPolicy(TypedDict):
    requiresBrowser: bool
    requiresFixtureServers: bool


class ManifestRequired(TypedDict):
    schema: int
    repository: str
    tag: str
    commit: str
    corpus: Corpus
    bootstrapSelectionOnly: bool
    files: dict[str, str]
    cases: list[Case]


class Manifest(ManifestRequired, total=False):
    execution: ExecutionPolicy
    executionMode: str
    fixtureFiles: dict[str, str]


BROWSERLESS_CAPABILITY = "playwright.lifecycle.browserless"
BROWSER_CALLS = {
    "connect_over_cdp",
    "goto",
    "launch",
    "launch_persistent_context",
    "new_browser_cdp_session",
    "new_context",
    "new_page",
}
PINNED_BROWSER_TYPE_NAME_ARGUMENTS = (
    "browser_type",
    "is_webkit",
    "is_firefox",
    "is_chromium",
)
PINNED_BROWSER_TYPE_EXECUTABLE_PATH_ARGUMENTS = (
    "browser_type",
    "browser_channel",
)
PINNED_BROWSER_TYPE_NAME_SOURCES = {
    "tests/async/test_launcher.py::test_browser_type_name_should_work": {
        "tests/async/test_launcher.py": (
            "648549470d4ba7f393c792b3774b45f358c1be63f1388509038a39a4e33564b3"
        ),
        "tests/conftest.py": "c4236e98ff1731fff1f7ef1c00b37fe3a0cb1e4a3fd07e89a491dc7129ea93cc",
        "tests/async/conftest.py": (
            "3001008bb97ba0cc1ddb397a53df530b6881e3e3ba69a1ed2cac0de94cab4cdb"
        ),
    },
    "tests/sync/test_launcher.py::test_browser_type_name_should_work": {
        "tests/sync/test_launcher.py": (
            "c5e1554d855450ef1c8aacb15eee7307401bb8a7878e8c1f77359d6110f0d94f"
        ),
        "tests/conftest.py": "c4236e98ff1731fff1f7ef1c00b37fe3a0cb1e4a3fd07e89a491dc7129ea93cc",
        "tests/sync/conftest.py": (
            "9ec5a65a98c312de67590792e80d3d9ea45312d56188f45101e71df6f7773d0b"
        ),
    },
}
PINNED_BROWSER_TYPE_EXECUTABLE_PATH_SOURCES = {
    "tests/async/test_launcher.py::test_browser_type_executable_path_should_work": {
        "tests/async/test_launcher.py": (
            "648549470d4ba7f393c792b3774b45f358c1be63f1388509038a39a4e33564b3"
        ),
        "tests/conftest.py": "c4236e98ff1731fff1f7ef1c00b37fe3a0cb1e4a3fd07e89a491dc7129ea93cc",
        "tests/async/conftest.py": (
            "3001008bb97ba0cc1ddb397a53df530b6881e3e3ba69a1ed2cac0de94cab4cdb"
        ),
        "tests/server.py": "2cf1a6b805f99c1c82c5351b67df995f06f2a9a18d21dbd48d4d4dc6079c3030",
    },
    "tests/sync/test_launcher.py::test_browser_type_executable_path_should_work": {
        "tests/sync/test_launcher.py": (
            "c5e1554d855450ef1c8aacb15eee7307401bb8a7878e8c1f77359d6110f0d94f"
        ),
        "tests/conftest.py": "c4236e98ff1731fff1f7ef1c00b37fe3a0cb1e4a3fd07e89a491dc7129ea93cc",
        "tests/sync/conftest.py": (
            "9ec5a65a98c312de67590792e80d3d9ea45312d56188f45101e71df6f7773d0b"
        ),
        "tests/server.py": "2cf1a6b805f99c1c82c5351b67df995f06f2a9a18d21dbd48d4d4dc6079c3030",
    },
}
PERSISTENT_PAGE_ARGUMENT_MODE = "persistent-page-argument-browserless-preflight"
PROXY_BAD_SERVER_MODE = "proxy-bad-server-browserless-preflight"
FETCH_GLOBAL_BROWSERLESS_MODE = "fetch-global-browserless-full-v1"
PINNED_FETCH_GLOBAL_SOURCES = {
    "tests/async/test_fetch_global.py": (
        "04cfb62e2261b1bbed6490fd3027b65d4daddda2e9c5a390d5fcbf46d03c255a"
    ),
    "tests/sync/test_fetch_global.py": (
        "f3768666392be83d3685dfb10038f7a15c45222b69d0ce6eb1cddb08bee30afe"
    ),
    "tests/conftest.py": "c4236e98ff1731fff1f7ef1c00b37fe3a0cb1e4a3fd07e89a491dc7129ea93cc",
    "tests/async/conftest.py": (
        "3001008bb97ba0cc1ddb397a53df530b6881e3e3ba69a1ed2cac0de94cab4cdb"
    ),
    "tests/sync/conftest.py": (
        "9ec5a65a98c312de67590792e80d3d9ea45312d56188f45101e71df6f7773d0b"
    ),
    "tests/async/utils.py": (
        "ee618a20d789a28f953ba23a2c24249f3b21f7bd00b1a2d2435aa75ce9b0b478"
    ),
    "tests/sync/utils.py": (
        "ae98bc4ec9132e71456246209de88e4300c9624bf355d763d1881c2919e29bc4"
    ),
    "tests/server.py": "2cf1a6b805f99c1c82c5351b67df995f06f2a9a18d21dbd48d4d4dc6079c3030",
    "tests/__init__.py": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "tests/async/__init__.py": (
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    ),
    "tests/sync/__init__.py": (
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    ),
}
PINNED_FETCH_GLOBAL_FIXTURE_FILES = {
    "tests/assets/simple.json": (
        "1a30b52dacd02b3698f473e5d9df017c54b1e006846363e878dcc2ade9624712"
    ),
    "tests/assets/empty.html": (
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    ),
    "tests/testserver/cert.pem": (
        "402a4ecc40aba35f8143673ac7c20d02c742954a79dc94523b06a5bd9c5fb18e"
    ),
    "tests/testserver/key.pem": (
        "3302e5f7eb8485e832a10f54a63cc049f98e98a5a2d55671c9ed9b440c59053a"
    ),
}
PINNED_FETCH_GLOBAL_IDS_SHA256 = (
    "1df02248b98853fb4ef28dd4d8f35d64fb464d0eeb18d3ca848240fcc60472b0"
)
PINNED_FETCH_GLOBAL_COUNTS = {
    "tests/async/test_fetch_global.py": 138,
    "tests/sync/test_fetch_global.py": 93,
}
FETCH_GLOBAL_ALLOWED_ARGUMENTS = frozenset(
    {"playwright", "server", "https_server", "tmp_path", "method", "serialization"}
)
PINNED_PROXY_BAD_SERVER_CASE_ID = (
    "tests/async/test_proxy.py::test_should_throw_for_bad_server_value"
)
PINNED_PROXY_BAD_SERVER_ARGUMENTS = ("browser_factory",)
PINNED_PROXY_BAD_SERVER_SOURCES = {
    "tests/async/test_proxy.py": (
        "8907e35d6b84218f6078b34fafed6e63997efc20db81b0c0b8745efb99b1eedd"
    ),
    "tests/conftest.py": ("c4236e98ff1731fff1f7ef1c00b37fe3a0cb1e4a3fd07e89a491dc7129ea93cc"),
    "tests/async/conftest.py": ("3001008bb97ba0cc1ddb397a53df530b6881e3e3ba69a1ed2cac0de94cab4cdb"),
    "tests/server.py": ("2cf1a6b805f99c1c82c5351b67df995f06f2a9a18d21dbd48d4d4dc6079c3030"),
}
PINNED_PROXY_BAD_SERVER_AST_SHA256 = (
    "81d49c3184638a71e250b1985f412fe58cdaa58dd7d452b8d998c2c1a0a51c6a"
)
PINNED_PROXY_BAD_SERVER_CAPABILITIES = [
    "browser-type.launch-proxy-argument-validation",
    BROWSERLESS_CAPABILITY,
]
PINNED_PERSISTENT_PAGE_ARGUMENT_CASE_ID = (
    "tests/async/test_defaultbrowsercontext.py::test_should_throw_if_page_argument_is_passed"
)
PINNED_PERSISTENT_PAGE_ARGUMENT_ARGUMENTS = (
    "browser_type",
    "server",
    "tmp_path",
    "launch_arguments",
)
PINNED_PERSISTENT_PAGE_ARGUMENT_SOURCES = {
    "tests/async/test_defaultbrowsercontext.py": (
        "61dc8a12d0346c7ad07d11dfb1e351582d82e0a71665964c100968478e8c2ac0"
    ),
    "tests/conftest.py": ("c4236e98ff1731fff1f7ef1c00b37fe3a0cb1e4a3fd07e89a491dc7129ea93cc"),
    "tests/async/conftest.py": ("3001008bb97ba0cc1ddb397a53df530b6881e3e3ba69a1ed2cac0de94cab4cdb"),
    "tests/server.py": ("2cf1a6b805f99c1c82c5351b67df995f06f2a9a18d21dbd48d4d4dc6079c3030"),
}
PINNED_PERSISTENT_PAGE_ARGUMENT_AST_SHA256 = (
    "349c8616c2494b5ad296c749c6c7b11c3f033a229edd641ace214b292238fc2d"
)
PINNED_PERSISTENT_PAGE_ARGUMENT_CAPABILITIES = [
    "browser-type.launch-persistent-context-argument-validation",
    BROWSERLESS_CAPABILITY,
]


def _validate_persistent_page_argument_manifest(
    manifest: Mapping[str, object],
) -> None:
    """Permit the pre-spawn exception only for this exact pinned fixture chain."""
    if manifest.get("executionMode") != PERSISTENT_PAGE_ARGUMENT_MODE:
        raise SystemExit("persistent page-argument case requires its exact execution mode")
    if manifest.get("repository") != "https://github.com/microsoft/playwright-python.git" or (
        manifest.get("tag") != "v1.58.0"
        or manifest.get("commit") != "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
    ):
        raise SystemExit("persistent page-argument case requires pinned upstream v1.58.0")
    if manifest.get("files") != PINNED_PERSISTENT_PAGE_ARGUMENT_SOURCES:
        raise SystemExit("persistent page-argument case requires exact fixture sources")
    if manifest.get("execution") != {
        "requiresBrowser": False,
        "requiresFixtureServers": True,
    }:
        raise SystemExit("persistent page-argument case requires browserless execution")
    if manifest.get("cases") != [
        {
            "id": PINNED_PERSISTENT_PAGE_ARGUMENT_CASE_ID,
            "capabilities": PINNED_PERSISTENT_PAGE_ARGUMENT_CAPABILITIES,
        }
    ]:
        raise SystemExit("persistent page-argument case requires exact selection")


def _validate_proxy_bad_server_manifest(manifest: Mapping[str, object]) -> None:
    """Permit only the exact pre-dispatch proxy type rejection."""
    if manifest.get("executionMode") != PROXY_BAD_SERVER_MODE:
        raise SystemExit("proxy bad-server case requires its exact execution mode")
    if manifest.get("repository") != "https://github.com/microsoft/playwright-python.git" or (
        manifest.get("tag") != "v1.58.0"
        or manifest.get("commit") != "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
    ):
        raise SystemExit("proxy bad-server case requires pinned upstream v1.58.0")
    if manifest.get("files") != PINNED_PROXY_BAD_SERVER_SOURCES:
        raise SystemExit("proxy bad-server case requires exact fixture sources")
    if manifest.get("execution") != {
        "requiresBrowser": False,
        "requiresFixtureServers": True,
    }:
        raise SystemExit("proxy bad-server case requires browserless execution")
    if manifest.get("cases") != [
        {
            "id": PINNED_PROXY_BAD_SERVER_CASE_ID,
            "capabilities": PINNED_PROXY_BAD_SERVER_CAPABILITIES,
        }
    ]:
        raise SystemExit("proxy bad-server case requires exact selection")


def _fetch_global_case_function(case_id: str) -> tuple[str, str]:
    relative, separator, selected = case_id.partition("::")
    function, bracket, parameters = selected.partition("[")
    if (
        not separator
        or relative not in PINNED_FETCH_GLOBAL_COUNTS
        or not bracket
        or not parameters.endswith("]")
        or not parameters[:-1]
        or "[" in parameters
    ):
        raise SystemExit(f"invalid complete fetch-global case ID: {case_id}")
    return relative, function


def _validate_fetch_global_manifest(manifest: Mapping[str, object]) -> None:
    """Lock all 231 expanded request-only IDs and their full upstream fixture inputs."""
    if manifest.get("executionMode") != FETCH_GLOBAL_BROWSERLESS_MODE:
        raise SystemExit("fetch-global suite requires its exact browserless execution mode")
    if (
        manifest.get("repository") != "https://github.com/microsoft/playwright-python.git"
        or manifest.get("tag") != "v1.58.0"
        or manifest.get("commit") != "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
        or manifest.get("bootstrapSelectionOnly") is not False
    ):
        raise SystemExit("fetch-global suite requires the pinned full upstream corpus")
    if manifest.get("execution") != {
        "requiresBrowser": False,
        "requiresFixtureServers": True,
    }:
        raise SystemExit("fetch-global suite requires browserless fixture-server execution")
    if manifest.get("files") != PINNED_FETCH_GLOBAL_SOURCES:
        raise SystemExit("fetch-global suite requires exact test and fixture sources")
    if manifest.get("fixtureFiles") != PINNED_FETCH_GLOBAL_FIXTURE_FILES:
        raise SystemExit("fetch-global suite requires exact HTTP and TLS fixture files")

    cases = manifest.get("cases")
    if not isinstance(cases, list) or len(cases) != 231:
        raise SystemExit("fetch-global suite must select all 231 expanded IDs")
    ids: list[str] = []
    counts = {relative: 0 for relative in PINNED_FETCH_GLOBAL_COUNTS}
    for raw_case in cases:
        if not isinstance(raw_case, dict) or set(raw_case) != {"id", "capabilities"}:
            raise SystemExit("fetch-global suite contains a malformed case")
        case_id = raw_case["id"]
        if not isinstance(case_id, str):
            raise SystemExit("fetch-global suite contains a non-string ID")
        relative, _ = _fetch_global_case_function(case_id)
        api = "async-api" if relative.startswith("tests/async/") else "sync-api"
        if raw_case["capabilities"] != [
            BROWSERLESS_CAPABILITY,
            "api-request-context",
            api,
        ]:
            raise SystemExit(f"fetch-global case has changed capabilities: {case_id}")
        ids.append(case_id)
        counts[relative] += 1
    digest = hashlib.sha256(("\n".join(ids) + "\n").encode()).hexdigest()
    if (
        counts != PINNED_FETCH_GLOBAL_COUNTS
        or len(ids) != len(set(ids))
        or digest != PINNED_FETCH_GLOBAL_IDS_SHA256
    ):
        raise SystemExit("fetch-global suite differs from the complete pinned inventory")


def _verify_fetch_global_fixture_files(
    upstream: Path,
    expected_files: Mapping[str, str] = PINNED_FETCH_GLOBAL_FIXTURE_FILES,
) -> None:
    for relative, expected_hash in expected_files.items():
        source = upstream / relative
        if hashlib.sha256(source.read_bytes()).hexdigest() != expected_hash:
            raise SystemExit(f"fetch-global fixture file drifted: {relative}")


def _verify_fetch_global_browserless_function(
    relative: str,
    function: str,
    selected: ast.FunctionDef | ast.AsyncFunctionDef,
) -> None:
    """Allow only the pinned request-context fixture path, never browser creation."""
    arguments = selected.args
    fixture_names = tuple(argument.arg for argument in arguments.args)
    if (
        arguments.posonlyargs
        or arguments.kwonlyargs
        or arguments.vararg is not None
        or arguments.kwarg is not None
        or not set(fixture_names).issubset(FETCH_GLOBAL_ALLOWED_ARGUMENTS)
        or "playwright" not in fixture_names
    ):
        raise SystemExit(f"fetch-global case has unapproved fixtures: {relative}::{function}")
    for node in ast.walk(selected):
        if not isinstance(node, ast.Call) or not isinstance(node.func, ast.Attribute):
            continue
        if node.func.attr == "new_context":
            request = node.func.value
            if (
                isinstance(request, ast.Attribute)
                and request.attr == "request"
                and isinstance(request.value, ast.Name)
                and request.value.id == "playwright"
            ):
                continue
        if node.func.attr in BROWSER_CALLS:
            raise SystemExit(
                "fetch-global case reaches a browser operation: "
                f"{relative}::{function} -> {node.func.attr}"
            )


def _verify_fetch_global_source_files(
    function_nodes: dict[str, dict[str, ast.FunctionDef | ast.AsyncFunctionDef]],
    case_ids: list[str],
) -> None:
    for relative in PINNED_FETCH_GLOBAL_COUNTS:
        selected_functions = {
            _fetch_global_case_function(case_id)[1]
            for case_id in case_ids
            if case_id.startswith(f"{relative}::")
        }
        source_functions = {
            function
            for function in function_nodes[relative]
            if function.startswith("test_")
        }
        if selected_functions != source_functions:
            raise SystemExit(f"fetch-global suite omits whole-file tests: {relative}")
        for function in sorted(source_functions):
            _verify_fetch_global_browserless_function(
                relative, function, function_nodes[relative][function]
            )


# Shared pinned validators have public cross-module names; retain the private
# names for historical tests and local callers without duplicating policy.
validate_persistent_page_argument_manifest = _validate_persistent_page_argument_manifest
validate_proxy_bad_server_manifest = _validate_proxy_bad_server_manifest


def _git(upstream: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(upstream), *args],
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip()


def _is_private_playwright_module(name: str) -> bool:
    parts = name.split(".")
    return parts[0] == "playwright" and any(part.startswith("_") for part in parts[1:])


def _browserless_fixture_arguments(
    relative: str, function: str, source_hashes: dict[str, str]
) -> tuple[str, ...]:
    case_id = f"{relative}::{function}"
    pinned_sources = PINNED_BROWSER_TYPE_NAME_SOURCES.get(case_id)
    allowed_arguments = PINNED_BROWSER_TYPE_NAME_ARGUMENTS
    if pinned_sources is None:
        pinned_sources = PINNED_BROWSER_TYPE_EXECUTABLE_PATH_SOURCES.get(case_id)
        allowed_arguments = PINNED_BROWSER_TYPE_EXECUTABLE_PATH_ARGUMENTS
        if pinned_sources is None:
            if case_id == PINNED_PERSISTENT_PAGE_ARGUMENT_CASE_ID:
                pinned_sources = PINNED_PERSISTENT_PAGE_ARGUMENT_SOURCES
                allowed_arguments = PINNED_PERSISTENT_PAGE_ARGUMENT_ARGUMENTS
            elif case_id == PINNED_PROXY_BAD_SERVER_CASE_ID:
                pinned_sources = PINNED_PROXY_BAD_SERVER_SOURCES
                allowed_arguments = PINNED_PROXY_BAD_SERVER_ARGUMENTS
            else:
                return ()
    for path, expected_hash in pinned_sources.items():
        if source_hashes.get(path) != expected_hash:
            raise SystemExit(f"browserless fixture source mismatch: {case_id} -> {path}")
    return allowed_arguments


def _verify_browserless_function(
    relative: str,
    function: str,
    functions: dict[str, ast.FunctionDef | ast.AsyncFunctionDef],
    allowed_fixture_arguments: tuple[str, ...] = (),
) -> None:
    selected = functions[function]
    arguments = selected.args
    if (
        arguments.posonlyargs
        or tuple(argument.arg for argument in arguments.args) != allowed_fixture_arguments
        or arguments.kwonlyargs
        or arguments.vararg is not None
        or arguments.kwarg is not None
    ):
        raise SystemExit(f"browserless case has unapproved fixtures: {relative}::{function}")
    persistent_preflight = (
        f"{relative}::{function}" == PINNED_PERSISTENT_PAGE_ARGUMENT_CASE_ID
        and allowed_fixture_arguments == PINNED_PERSISTENT_PAGE_ARGUMENT_ARGUMENTS
    )
    if persistent_preflight:
        observed_ast_hash = hashlib.sha256(
            ast.dump(selected, include_attributes=False).encode()
        ).hexdigest()
        if observed_ast_hash != PINNED_PERSISTENT_PAGE_ARGUMENT_AST_SHA256:
            raise SystemExit(f"browserless pre-spawn test AST changed: {relative}::{function}")
    if (
        f"{relative}::{function}" == PINNED_PROXY_BAD_SERVER_CASE_ID
        and allowed_fixture_arguments == PINNED_PROXY_BAD_SERVER_ARGUMENTS
    ):
        observed_ast_hash = hashlib.sha256(
            ast.dump(selected, include_attributes=False).encode()
        ).hexdigest()
        if observed_ast_hash != PINNED_PROXY_BAD_SERVER_AST_SHA256:
            raise SystemExit(f"browserless pre-spawn test AST changed: {relative}::{function}")
    pending = [function]
    visited: set[str] = set()
    persistent_calls = 0
    while pending:
        current = pending.pop()
        if current in visited:
            continue
        visited.add(current)
        for node in ast.walk(functions[current]):
            if not isinstance(node, ast.Call):
                continue
            if (
                persistent_preflight
                and current == function
                and (
                    isinstance(node.func, ast.Attribute)
                    and node.func.attr == "launch_persistent_context"
                    and isinstance(node.func.value, ast.Name)
                    and node.func.value.id == "browser_type"
                    and len(node.args) == 1
                    and isinstance(node.args[0], ast.Name)
                    and node.args[0].id == "tmp_path"
                    and len(node.keywords) == 1
                    and node.keywords[0].arg is None
                    and isinstance(node.keywords[0].value, ast.Name)
                    and node.keywords[0].value.id == "options"
                )
            ):
                persistent_calls += 1
                continue
            if isinstance(node.func, ast.Attribute) and node.func.attr in BROWSER_CALLS:
                raise SystemExit(
                    "browserless case reaches a browser operation: "
                    f"{relative}::{function} -> {node.func.attr}"
                )
            if isinstance(node.func, ast.Name) and node.func.id in functions:
                pending.append(node.func.id)
    if persistent_preflight and persistent_calls != 1:
        raise SystemExit(f"browserless pre-spawn call changed: {relative}::{function}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("upstream", type=Path)
    parser.add_argument(
        "--manifest",
        type=Path,
        default=Path(__file__).parents[1] / "packages/pwright-python/tests/official/manifest.json",
    )
    args = parser.parse_args()

    manifest = cast(Manifest, json.loads(args.manifest.read_text()))
    if manifest["schema"] != 1:
        raise SystemExit(f"unsupported manifest schema: {manifest['schema']}")
    if _git(args.upstream, "rev-parse", "HEAD") != manifest["commit"]:
        raise SystemExit("upstream commit does not match the pinned corpus")
    if _git(args.upstream, "status", "--porcelain"):
        raise SystemExit("upstream checkout is dirty")

    case_ids = [case["id"] for case in manifest["cases"]]
    if len(case_ids) != len(set(case_ids)):
        raise SystemExit("selected upstream test IDs must be unique")
    for case in manifest["cases"]:
        capabilities = case["capabilities"]
        if not capabilities or len(capabilities) != len(set(capabilities)):
            raise SystemExit(f"capabilities must be nonempty and unique: {case['id']}")
    if manifest.get("executionMode") == PERSISTENT_PAGE_ARGUMENT_MODE or (
        manifest.get("execution", {}).get("requiresBrowser") is False
        and PINNED_PERSISTENT_PAGE_ARGUMENT_CASE_ID in case_ids
    ):
        _validate_persistent_page_argument_manifest(manifest)
    if manifest.get("executionMode") == PROXY_BAD_SERVER_MODE or (
        manifest.get("execution", {}).get("requiresBrowser") is False
        and PINNED_PROXY_BAD_SERVER_CASE_ID in case_ids
    ):
        _validate_proxy_bad_server_manifest(manifest)
    fetch_global_full = manifest.get("executionMode") == FETCH_GLOBAL_BROWSERLESS_MODE
    if fetch_global_full:
        _validate_fetch_global_manifest(manifest)
        _verify_fetch_global_fixture_files(args.upstream)

    functions: dict[str, set[str]] = {}
    function_nodes: dict[str, dict[str, ast.FunctionDef | ast.AsyncFunctionDef]] = {}
    source_hashes: dict[str, str] = {}
    for relative, expected_hash in manifest["files"].items():
        source = (args.upstream / relative).read_bytes()
        observed_hash = hashlib.sha256(source).hexdigest()
        if observed_hash != expected_hash:
            raise SystemExit(f"source hash mismatch: {relative}")
        source_hashes[relative] = observed_hash
        tree = ast.parse(source, filename=relative)
        nodes = {
            node.name: node
            for node in tree.body
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
        }
        function_nodes[relative] = nodes
        functions[relative] = set(nodes)

    for case_id in case_ids:
        relative, function = (
            _fetch_global_case_function(case_id)
            if fetch_global_full
            else case_id.split("::", maxsplit=1)
        )
        if function not in functions.get(relative, set()):
            raise SystemExit(f"selected upstream test is missing: {case_id}")

    execution = manifest.get("execution")
    if execution is not None:
        if set(execution) != {"requiresBrowser", "requiresFixtureServers"}:
            raise SystemExit("manifest execution policy must declare both requirements")
        if not all(isinstance(value, bool) for value in execution.values()):
            raise SystemExit("manifest execution requirements must be booleans")
        if not execution["requiresFixtureServers"]:
            raise SystemExit("pinned upstream autouse start_server requires fixture servers")
        if not execution["requiresBrowser"]:
            for case in manifest["cases"]:
                if BROWSERLESS_CAPABILITY not in case["capabilities"]:
                    raise SystemExit("every browserless manifest case must declare its capability")
            if fetch_global_full:
                _verify_fetch_global_source_files(function_nodes, case_ids)
            else:
                for case in manifest["cases"]:
                    relative, function = case["id"].split("::", maxsplit=1)
                    fixture_arguments = _browserless_fixture_arguments(
                        relative, function, source_hashes
                    )
                    _verify_browserless_function(
                        relative, function, function_nodes[relative], fixture_arguments
                    )

    module_count = 0
    test_count = 0
    private_module_count = 0
    private_test_count = 0
    for path in sorted((args.upstream / "tests").rglob("test_*.py")):
        tree = ast.parse(path.read_bytes(), filename=str(path))
        tests = [
            node
            for node in tree.body
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
            and node.name.startswith("test_")
        ]
        uses_private_import = any(
            (
                isinstance(node, ast.ImportFrom)
                and node.module is not None
                and _is_private_playwright_module(node.module)
            )
            or (
                isinstance(node, ast.Import)
                and any(_is_private_playwright_module(alias.name) for alias in node.names)
            )
            for node in ast.walk(tree)
        )
        module_count += 1
        test_count += len(tests)
        if uses_private_import:
            private_module_count += 1
            private_test_count += len(tests)

    example_module_count = sum(1 for _ in (args.upstream / "examples").rglob("test_*.py"))
    corpus = manifest["corpus"]
    observed_source_counts = (
        module_count,
        test_count,
        example_module_count,
    )
    expected_source_counts = (
        corpus["testsTreeModules"],
        corpus["testsTreeTopLevelTestFunctions"],
        corpus["examplesTreeModules"],
    )
    if observed_source_counts != expected_source_counts:
        raise SystemExit(
            "upstream Python source inventory changed: "
            f"expected={expected_source_counts} observed={observed_source_counts}"
        )

    case_label = (
        "bootstrap cases only" if manifest["bootstrapSelectionOnly"] else "fixed selected cases"
    )
    print(
        f"official Playwright Python corpus verified: {manifest['tag']} "
        f"{manifest['commit']} ({corpus['expandedTests']} expanded tests; "
        f"{len(manifest['cases'])} {case_label})"
    )
    print(
        "upstream inventory: "
        f"{module_count} modules / {test_count} tests; "
        f"{module_count - private_module_count} modules / "
        f"{test_count - private_test_count} tests avoid playwright._* imports; "
        f"{private_module_count} modules / {private_test_count} tests import "
        f"upstream-private namespaces; {example_module_count} example modules"
    )


if __name__ == "__main__":
    main()
