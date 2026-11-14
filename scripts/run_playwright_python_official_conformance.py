#!/usr/bin/env python3
"""Run the same pinned upstream pytest nodes against oracle or candidate."""

from __future__ import annotations

import argparse
import copy
import hashlib
import importlib.metadata
import ipaddress
import json
import os
import shutil
import signal
import subprocess
import sys
import urllib.request
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import TypedDict, cast
from urllib.parse import urlsplit, urlunsplit

from check_playwright_python_official_source import (
    PERSISTENT_PAGE_ARGUMENT_MODE,
    PINNED_PERSISTENT_PAGE_ARGUMENT_CASE_ID,
    PINNED_PROXY_BAD_SERVER_CASE_ID,
    PROXY_BAD_SERVER_MODE,
    validate_persistent_page_argument_manifest,
    validate_proxy_bad_server_manifest,
)
from official_result_destination import prepare_result_destination
from playwright_python_official_provenance import (
    browser_registry_diagnostic_selected,
    browser_registry_snapshot,
    build_provenance,
    file_sha256,
    probe_collection_profile_identity,
    probe_official_browsers_json,
    probe_runtime_identity,
    require_browser_registry_unchanged,
    require_tooling_unchanged,
    tooling_sources,
    tree_sha256,
)


class Case(TypedDict):
    id: str
    capabilities: list[str]


class CorpusRequired(TypedDict):
    expandedTests: int


class Corpus(CorpusRequired, total=False):
    collectionProfiles: list[dict[str, object]]
    defaultBrowsers: list[str]


class ExecutionPolicy(TypedDict):
    requiresBrowser: bool
    requiresFixtureServers: bool


class ManifestRequired(TypedDict):
    schema: int
    repository: str
    tag: str
    commit: str
    corpus: Corpus
    cases: list[Case]


class Manifest(ManifestRequired, total=False):
    execution: ExecutionPolicy
    executionMode: str


class InventoryRecord(TypedDict):
    file: str
    id: str


Key = tuple[str, str]
BROWSERLESS_CAPABILITY = "playwright.lifecycle.browserless"
DEFAULT_CDP_ENDPOINT = "http://127.0.0.1:9222"
JUNIT_LAUNCH_MODE_PROPERTY = "pwright.official.launchMode"
SYNC_LIFECYCLE_LOOP_MODE = "same-thread-asyncio-loop"
SYNC_LIFECYCLE_COMMIT = "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
SYNC_LIFECYCLE_SOURCE_SHA256 = "f371ee96a8eacc0721e7e3514223992d37ecba8fec86e6396584de4049e03579"
SYNC_LIFECYCLE_CASE_IDS = (
    "tests/sync/test_sync.py::test_sync_playwright_multiple_times",
    "tests/sync/test_sync.py::test_sync_playwright_stop_multiple_times",
)
SYNC_LIFECYCLE_LOOP_WRAPPER = (
    "import asyncio, sys\n"
    "import pytest\n"
    "async def main():\n"
    "    return pytest.main(sys.argv[1:])\n"
    "raise SystemExit(asyncio.run(main()))\n"
)
SYNC_LIFECYCLE_LOOP_WRAPPER_SHA256 = hashlib.sha256(
    SYNC_LIFECYCLE_LOOP_WRAPPER.encode("utf-8")
).hexdigest()
PERSISTENT_PREFLIGHT_ORACLE_SOURCES = {
    "driver/package/lib/server/browserType.js": (
        "08266f672f1ba9dedd77fb30008c7590e6400dcf9bf497fe3fe61ce0e74bcbf0"
    ),
    "driver/package/lib/server/webkit/webkit.js": (
        "b909650336cb30edf5bcc137ff9a686fa65be4f1efe5f4359058667bda37213d"
    ),
}
PERSISTENT_PREFLIGHT_CANDIDATE_SOURCES = {
    "async_api.py": ("ab5179823973fbcc52247f42d51da700c3ba81003aa7a317c8c938c26aba0537"),
}
PERSISTENT_PREFLIGHT_HARNESS_SHA256 = {
    "shim": "91695bbdc103345adc694882c3f9dc02dfd39929ca138265d8e7769830d7c15b",
    "runtime": "aa98681e0669136b62f940c53b7a405802a33fba6fbdd79a87efaabd6463eb44",
}
PERSISTENT_PREFLIGHT_PACKAGE_SHA256 = {
    "oracle": "d7655707035107f13a81db0e1d52d839c2bf42bf2008478fba610299cd6da149",
    "candidate": "7da9aab756d4f880e7789b652e1a3b171455eac02c58d99364157b4a44a6097b",
}
PROXY_PREFLIGHT_ORACLE_SOURCES = {
    "async_api/_generated.py": "bec1438194e51cf942b01e8425d7d7a962f4be6f14619363133a0166f66f7313",
    "_impl/_browser_type.py": "cae4b33f44b264e112aa058c08d363de6d31ea4434a309a1f56d81dae59f5801",
    "_impl/_connection.py": "bc474593abd8cf676156b3e0c16d463edd027c61cf715462bb6f3c96ffdad9cc",
    "driver/package/lib/protocol/validator.js": (
        "6643ff17117b16115867af7c647bbe5de151682d9cc8d7998db343405e465c59"
    ),
    "driver/package/lib/protocol/validatorPrimitives.js": (
        "a3abe78847253497de04ea6f329a925df66c9a2dd62a5830aacaf1461d4128b3"
    ),
    "driver/package/lib/server/dispatchers/dispatcher.js": (
        "628c39de7799325c589a6f62e642324cb61b8d6fdaeec6bc29b40b0f0a76dc58"
    ),
    "driver/package/lib/server/dispatchers/browserTypeDispatcher.js": (
        "e6765d006d98b873ea33ca91dac5f19db81ceb81a306d159d04c0be4cc2265cb"
    ),
}
PROXY_PREFLIGHT_CANDIDATE_SOURCES = {
    "async_api.py": "35ecc6c68c0da8f20bbd4a1c193531909e47b0e4a8709c7f796563675497a219",
}
PROXY_PREFLIGHT_PACKAGE_SHA256 = {
    "oracle": "d7655707035107f13a81db0e1d52d839c2bf42bf2008478fba610299cd6da149",
    "candidate": "96ec860880d03b2a09432a7ec1aec14de3598aac5327735be25b8641742cbf4f",
}


def _require_proxy_preflight_runtime_sources(mode: str, package: Path) -> None:
    """Pin the exact public/API protocol code whose rejection precedes spawn."""
    expected = (
        PROXY_PREFLIGHT_ORACLE_SOURCES
        if mode == "oracle"
        else PROXY_PREFLIGHT_CANDIDATE_SOURCES
        if mode == "candidate"
        else None
    )
    if expected is None or package.is_symlink() or not package.is_dir():
        raise ValueError("proxy preflight runtime package is invalid")
    if tree_sha256(package) != PROXY_PREFLIGHT_PACKAGE_SHA256[mode]:
        raise ValueError("proxy preflight runtime package tree drifted")
    package_root = package.resolve()
    for relative, digest in expected.items():
        path = package / relative
        if (
            path.is_symlink()
            or not path.is_file()
            or not path.resolve().is_relative_to(package_root)
            or file_sha256(path) != digest
        ):
            raise ValueError(f"proxy preflight runtime source drifted: {relative}")


def _require_persistent_preflight_harness_sources(official: Path, mode: str) -> None:
    names = ("runtime",) if mode == "oracle" else ("shim", "runtime")
    for name in names:
        if tree_sha256(official / name) != PERSISTENT_PREFLIGHT_HARNESS_SHA256[name]:
            raise ValueError(f"persistent preflight harness source drifted: {name}")


def _require_persistent_preflight_runtime_sources(mode: str, package: Path) -> None:
    """Pin the actual pre-spawn validation implementations, not just test source."""
    expected = (
        PERSISTENT_PREFLIGHT_ORACLE_SOURCES
        if mode == "oracle"
        else PERSISTENT_PREFLIGHT_CANDIDATE_SOURCES
        if mode == "candidate"
        else None
    )
    if expected is None or package.is_symlink() or not package.is_dir():
        raise ValueError("persistent preflight runtime package is invalid")
    if tree_sha256(package) != PERSISTENT_PREFLIGHT_PACKAGE_SHA256[mode]:
        raise ValueError("persistent preflight runtime package tree drifted")
    package_root = package.resolve()
    for relative, digest in expected.items():
        path = package / relative
        if (
            path.is_symlink()
            or not path.is_file()
            or not path.resolve().is_relative_to(package_root)
            or file_sha256(path) != digest
        ):
            raise ValueError(f"persistent preflight runtime source drifted: {relative}")


def _persistent_page_argument_browserless_mode(
    manifest: Manifest,
    *,
    manifest_cases: bool,
    manifest_api: str | None,
    browsers: list[str],
    browser_channel: str | None,
    launch_mode: str,
    endpoint: str | None,
    isolate_cases: bool,
    collect_only: bool,
    case_timeout: float | None,
    server_prefix: str | None,
    secure_http_fixture: bool,
) -> bool:
    """Restrict the one no-browser persistent-launch preflight before pytest starts."""
    execution = manifest.get("execution")
    selected = any(
        case.get("id") == PINNED_PERSISTENT_PAGE_ARGUMENT_CASE_ID for case in manifest["cases"]
    )
    if manifest.get("executionMode") != PERSISTENT_PAGE_ARGUMENT_MODE and not (
        selected and isinstance(execution, dict) and execution.get("requiresBrowser") is False
    ):
        return False
    try:
        validate_persistent_page_argument_manifest(manifest)
    except SystemExit as error:
        raise ValueError(str(error)) from error
    if (
        not manifest_cases
        or manifest_api != "async"
        or browsers not in (["webkit"], ["firefox"])
        or browser_channel is not None
        or launch_mode != "owned"
        or endpoint is not None
        or isolate_cases
        or collect_only
        or case_timeout is None
        or server_prefix is not None
        or secure_http_fixture
    ):
        raise ValueError(
            "persistent page-argument browserless preflight requires one WebKit or "
            "Firefox async manifest case in owned mode with timeout, no CDP, "
            "channel, isolation, secure fixture, or external server prefix"
        )
    return True


def _proxy_bad_server_browserless_mode(
    manifest: Manifest,
    *,
    manifest_cases: bool,
    manifest_api: str | None,
    browsers: list[str],
    browser_channel: str | None,
    launch_mode: str,
    endpoint: str | None,
    isolate_cases: bool,
    collect_only: bool,
    case_timeout: float | None,
    server_prefix: str | None,
    secure_http_fixture: bool,
) -> bool:
    """Restrict the exact proxy type error to an owned, no-browser run."""
    execution = manifest.get("execution")
    selected = any(case.get("id") == PINNED_PROXY_BAD_SERVER_CASE_ID for case in manifest["cases"])
    if manifest.get("executionMode") != PROXY_BAD_SERVER_MODE and not (
        selected and isinstance(execution, dict) and execution.get("requiresBrowser") is False
    ):
        return False
    try:
        validate_proxy_bad_server_manifest(manifest)
    except SystemExit as error:
        raise ValueError(str(error)) from error
    if (
        not manifest_cases
        or manifest_api != "async"
        or browsers != ["chromium", "firefox", "webkit"]
        or browser_channel is not None
        or launch_mode != "owned"
        or endpoint is not None
        or isolate_cases
        or collect_only
        or case_timeout is None
        or server_prefix is not None
        or secure_http_fixture
    ):
        raise ValueError(
            "proxy bad-server browserless preflight requires exactly the three "
            "async engines in owned mode with timeout, no CDP, channel, isolation, "
            "secure fixture, or external server prefix"
        )
    return True


def _sync_lifecycle_loop_mode(
    manifest: Manifest,
    *,
    manifest_cases: bool,
    manifest_api: str | None,
    browsers: list[str],
    browser_channel: str | None,
    isolate_cases: bool,
    collect_only: bool,
) -> bool:
    """Allow the same-thread loop only for the six pinned sync lifecycle rows."""
    if "executionMode" not in manifest:
        return False
    raw_mode = manifest["executionMode"]
    if raw_mode in (PERSISTENT_PAGE_ARGUMENT_MODE, PROXY_BAD_SERVER_MODE):
        return False
    if raw_mode != SYNC_LIFECYCLE_LOOP_MODE:
        raise ValueError(f"unsupported manifest executionMode: {raw_mode!r}")
    if not manifest_cases or manifest_api != "sync" or browsers or browser_channel:
        raise ValueError("same-thread asyncio loop requires all pinned sync manifest cases")
    if isolate_cases:
        raise ValueError("same-thread asyncio loop cannot isolate individual cases")
    if manifest.get("repository") != "https://github.com/microsoft/playwright-python.git" or (
        manifest.get("tag") != "v1.58.0" or manifest.get("commit") != SYNC_LIFECYCLE_COMMIT
    ):
        raise ValueError("same-thread asyncio loop requires pinned upstream v1.58.0")
    if manifest.get("files") != {"tests/sync/test_sync.py": SYNC_LIFECYCLE_SOURCE_SHA256}:
        raise ValueError("same-thread asyncio loop requires the pinned test source")
    if manifest.get("execution") != {
        "requiresBrowser": False,
        "requiresFixtureServers": True,
    }:
        raise ValueError("same-thread asyncio loop requires browserless fixture-server policy")
    expected_cases = [
        {"id": case_id, "capabilities": [BROWSERLESS_CAPABILITY]}
        for case_id in SYNC_LIFECYCLE_CASE_IDS
    ]
    if manifest.get("cases") != expected_cases:
        raise ValueError("same-thread asyncio loop requires exactly two pinned lifecycle cases")
    return not collect_only


# Verification imports these exact runtime decisions through public names;
# private aliases remain for local callers and historical focused tests.
persistent_page_argument_browserless_mode = _persistent_page_argument_browserless_mode
proxy_bad_server_browserless_mode = _proxy_bad_server_browserless_mode
require_persistent_preflight_runtime_sources = _require_persistent_preflight_runtime_sources
require_proxy_preflight_runtime_sources = _require_proxy_preflight_runtime_sources
sync_lifecycle_loop_mode = _sync_lifecycle_loop_mode


def _validate_loopback_endpoint(endpoint: str) -> str:
    parsed = urlsplit(endpoint)
    if parsed.scheme not in {"http", "https", "ws", "wss"}:
        raise ValueError("CDP endpoint must use http(s) or ws(s)")
    if parsed.hostname not in {"127.0.0.1", "localhost", "::1"}:
        raise ValueError("official conformance requires a loopback-only CDP endpoint")
    return endpoint


def _select_launch_endpoint(launch_mode: str, explicit_endpoint: str | None) -> str | None:
    if launch_mode == "owned":
        if explicit_endpoint is not None:
            raise ValueError("--endpoint cannot be combined with --launch-mode=owned")
        return None
    if launch_mode != "cdp-adapter":
        raise ValueError(f"unsupported launch mode: {launch_mode!r}")
    return _validate_loopback_endpoint(
        DEFAULT_CDP_ENDPOINT if explicit_endpoint is None else explicit_endpoint
    )


def _configure_launch_environment(
    environment: dict[str, str], launch_mode: str, endpoint: str | None
) -> None:
    if launch_mode == "owned" and endpoint is None:
        environment.pop("PWRIGHT_CDP_ENDPOINT", None)
    elif launch_mode == "cdp-adapter" and endpoint is not None:
        environment["PWRIGHT_CDP_ENDPOINT"] = endpoint
    else:
        raise ValueError("launch mode and CDP endpoint disagree")
    environment["PWRIGHT_OFFICIAL_LAUNCH_MODE"] = launch_mode


def _require_owned_public_launch(python: str, mode: str, *, cwd: Path, env: dict[str, str]) -> None:
    """Inspect the installed public API without starting a browser or test server."""
    package = "pwright" if mode == "candidate" else "playwright"
    script = """
import inspect
import sys
from importlib import import_module

package = sys.argv[1]
for api_name in ("async_api", "sync_api"):
    module = import_module(f"{package}.{api_name}")
    launch = inspect.getattr_static(module.BrowserType, "launch", None)
    if not callable(launch) or not getattr(launch, "__module__", "").startswith(f"{package}."):
        raise SystemExit(f"{package}.{api_name}.BrowserType.launch is not a real public API")
"""
    probe = subprocess.run(
        [python, "-c", script, package],
        cwd=cwd,
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )
    if probe.returncode:
        detail = (probe.stderr or probe.stdout).strip()
        raise ValueError(
            f"owned launch preflight failed: {detail or 'public launch API unavailable'}"
        )


def _active_cdp_pages(targets: object) -> list[dict[str, object]]:
    if not isinstance(targets, list):
        raise TypeError("CDP target inventory must be a list of objects")
    target_values = cast(list[object], targets)
    if not all(isinstance(target, dict) for target in target_values):
        raise TypeError("CDP target inventory must be a list of objects")
    pages: list[dict[str, object]] = []
    for raw_target in target_values:
        target = cast(dict[str, object], raw_target)
        if target.get("type") == "page" and target.get("url") not in {
            "about:blank",
            "chrome://newtab/",
        }:
            pages.append(target)
    return pages


def _existing_cdp_pages(endpoint: str) -> list[dict[str, object]]:
    parsed = urlsplit(_validate_loopback_endpoint(endpoint))
    scheme = {"ws": "http", "wss": "https"}.get(parsed.scheme, parsed.scheme)
    target_url = urlunsplit((scheme, parsed.netloc, "/json/list", "", ""))
    try:
        with urllib.request.urlopen(target_url, timeout=5) as response:
            return _active_cdp_pages(json.load(response))
    except (OSError, TypeError, ValueError, json.JSONDecodeError) as error:
        raise ValueError(f"could not inventory dedicated Chrome targets: {error}") from error


def _validate_tailscale_fixture(ip: str, host: str) -> tuple[str, str]:
    address = ipaddress.ip_address(ip.strip())
    if address.version != 4 or address.is_loopback or address.is_unspecified:
        raise ValueError("official fixtures require a non-loopback Tailscale IPv4")
    normalized_host = host.strip().rstrip(".")
    if not normalized_host.endswith(".ts.net") or normalized_host == str(address):
        raise ValueError("official fixtures require a full Tailscale MagicDNS hostname")
    return str(address), normalized_host


def _validate_server_prefix(prefix: str) -> str:
    parsed = urlsplit(prefix)
    if parsed.scheme not in {"http", "https"} or not (
        parsed.hostname and parsed.hostname.endswith(".ts.net")
    ):
        raise ValueError("server prefix must use a full Tailscale MagicDNS HTTP(S) URL")
    return prefix.rstrip("/")


def _execution_requirements(
    manifest: Manifest,
    manifest_cases: bool,
    collection_profile: str | None = None,
) -> tuple[bool, bool]:
    if collection_profile == "examples":
        if manifest_cases:
            raise ValueError("examples collection profile cannot use manifest cases")
        return True, False
    raw_execution = manifest.get("execution")
    if raw_execution is None:
        return True, True
    if not manifest_cases:
        raise ValueError("manifest execution policy requires --manifest-cases")
    execution = _object_mapping(raw_execution, "manifest execution policy must be an object")
    if set(execution) != {"requiresBrowser", "requiresFixtureServers"}:
        raise ValueError("manifest execution policy must declare both requirements")
    requires_browser = execution["requiresBrowser"]
    requires_fixture_servers = execution["requiresFixtureServers"]
    if not isinstance(requires_browser, bool) or not isinstance(requires_fixture_servers, bool):
        raise TypeError("manifest execution requirements must be booleans")
    if not requires_fixture_servers:
        raise ValueError("pinned upstream autouse start_server requires fixture servers")
    if not requires_browser and any(
        BROWSERLESS_CAPABILITY not in case["capabilities"] for case in manifest["cases"]
    ):
        raise ValueError("every browserless manifest case must declare its capability")
    return requires_browser, requires_fixture_servers


def _object_mapping(value: object, message: str) -> dict[str, object]:
    if not isinstance(value, dict):
        raise TypeError(message)
    return cast(dict[str, object], value)


def _object_list(value: object, message: str) -> list[object]:
    if not isinstance(value, list):
        raise TypeError(message)
    return cast(list[object], value)


def _collection_profile(manifest: Manifest, profile_name: str) -> dict[str, object]:
    raw_profiles = manifest["corpus"].get("collectionProfiles")
    profile_values = _object_list(
        raw_profiles, "manifest has no collectionProfiles for profile execution"
    )
    matches: list[dict[str, object]] = []
    for value in profile_values:
        if not isinstance(value, dict):
            continue
        profile = cast(dict[str, object], value)
        if profile.get("name") == profile_name:
            matches.append(profile)
    if len(matches) != 1:
        raise ValueError(f"manifest must define exactly one {profile_name} profile")
    profile = matches[0]
    selection = profile.get("selection")
    if not isinstance(selection, str) or not selection:
        raise ValueError(f"{profile_name} collection profile has no selection")
    expanded_tests = profile.get("expandedTests")
    if (
        isinstance(expanded_tests, bool)
        or not isinstance(expanded_tests, int)
        or expanded_tests < 1
    ):
        raise ValueError(f"{profile_name} collection profile has invalid expandedTests")
    return profile


def _selection_is_in_profile(selection: str, profile_selection: str) -> bool:
    source = selection.partition("::")[0]
    return source == profile_selection or source.startswith(f"{profile_selection}/")


def _junit_key(node_id: str) -> Key:
    source, *segments = node_id.split("::")
    if not segments or not source.endswith(".py"):
        raise SystemExit(f"unsupported isolated pytest inventory ID: {node_id}")
    module = source.removesuffix(".py").replace("/", ".")
    return ".".join((module, *segments[:-1])), segments[-1]


def _selection_matches(record: InventoryRecord, selections: list[str]) -> bool:
    if not selections:
        return True
    node_id = record["id"]
    for selection in selections:
        if "::" not in selection:
            if record["file"] == selection or record["file"].startswith(f"{selection}/"):
                return True
            continue
        if node_id == selection:
            return True
        if "[" not in selection and node_id.startswith(f"{selection}["):
            return True
    return False


def _isolated_cases(
    inventory_path: Path,
    selections: list[str],
    browsers: list[str],
) -> list[str]:
    if not inventory_path.is_file():
        raise SystemExit(f"official Python inventory is missing: {inventory_path}")
    records = [
        cast(InventoryRecord, json.loads(line)) for line in inventory_path.read_text().splitlines()
    ]
    selected: list[str] = []
    for record in records:
        node_id = record["id"]
        parameters = node_id.rsplit("[", 1)[1].removesuffix("]") if "[" in node_id else ""
        parameter_tokens = parameters.split("-")
        parameterized_browsers = [
            browser for browser in ("chromium", "firefox", "webkit") if browser in parameter_tokens
        ]
        if (
            browsers
            and parameterized_browsers
            and not any(browser in parameterized_browsers for browser in browsers)
        ):
            continue
        if _selection_matches(record, selections):
            selected.append(node_id)
    if not selected:
        raise SystemExit("isolated selection contains no expanded official tests")
    return selected


# Public tooling surface shared by the independent provenance verifier. The
# underscored implementations remain available to the focused runner tests.
isolated_cases = _isolated_cases
execution_requirements = _execution_requirements
collection_profile = _collection_profile
selection_is_in_profile = _selection_is_in_profile
validate_loopback_endpoint = _validate_loopback_endpoint
validate_server_prefix = _validate_server_prefix
validate_tailscale_fixture = _validate_tailscale_fixture


def _case_element(node_id: str, outcome: str, message: str) -> ET.Element:
    classname, name = _junit_key(node_id)
    case = ET.Element(
        "testcase",
        {"classname": classname, "name": name, "time": "0"},
    )
    detail = ET.SubElement(case, outcome, {"message": message})
    detail.text = message
    return case


def _read_case_element(path: Path, node_id: str) -> ET.Element:
    expected = _junit_key(node_id)
    cases = list(ET.parse(path).getroot().iter("testcase"))
    if len(cases) != 1 or (cases[0].get("classname"), cases[0].get("name")) != expected:
        raise ValueError(
            f"isolated report must contain exactly one {expected!r} testcase, found {len(cases)}"
        )
    return copy.deepcopy(cases[0])


def _preserve_invalid_case_report(path: Path) -> tuple[Path, str]:
    """Keep pytest's original bytes before replacing an invalid case with an error."""
    if path.is_symlink():
        raise ValueError(f"isolated report is a symlink: {path}")
    raw_path = path.with_name(f"{path.name}.invalid.raw")
    with path.open("rb") as source, raw_path.open("xb") as target:
        shutil.copyfileobj(source, target)
    return raw_path, file_sha256(raw_path)


def _write_isolated_suite(cases: list[ET.Element], output: Path) -> None:
    def count(child: str) -> int:
        return sum(case.find(child) is not None for case in cases)

    suite = ET.Element(
        "testsuite",
        {
            "name": "pytest-isolated",
            "errors": str(count("error")),
            "failures": str(count("failure")),
            "skipped": str(count("skipped")),
            "tests": str(len(cases)),
            "time": f"{sum(float(case.get('time', '0')) for case in cases):.3f}",
        },
    )
    suite.extend(copy.deepcopy(cases))
    root = ET.Element("testsuites", {"name": "pytest isolated tests"})
    root.append(suite)
    output.parent.mkdir(parents=True, exist_ok=True)
    ET.ElementTree(root).write(output, encoding="utf-8", xml_declaration=True)


def _write_single_case(case: ET.Element, output: Path) -> None:
    single_root = ET.Element("testsuites", {"name": "pytest isolated case"})
    single_suite = ET.SubElement(
        single_root,
        "testsuite",
        {
            "name": "pytest-isolated",
            "errors": str(int(case.find("error") is not None)),
            "failures": str(int(case.find("failure") is not None)),
            "skipped": str(int(case.find("skipped") is not None)),
            "tests": "1",
            "time": case.get("time", "0"),
        },
    )
    single_suite.append(copy.deepcopy(case))
    ET.ElementTree(single_root).write(output, encoding="utf-8", xml_declaration=True)


def _junit_launch_modes(root: ET.Element) -> list[str]:
    modes: list[str] = []
    for suite in root.iter("testsuite"):
        properties = suite.find("properties")
        if properties is not None:
            modes.extend(
                property_element.get("value", "")
                for property_element in properties.findall("property")
                if property_element.get("name") == JUNIT_LAUNCH_MODE_PROPERTY
            )
    return modes


def _label_junit_launch_mode(path: Path, launch_mode: str) -> None:
    try:
        tree = ET.parse(path)
    except ET.ParseError as error:
        raise ValueError(f"invalid JUnit report {path}: {error}") from error
    suites = list(tree.getroot().iter("testsuite"))
    if not suites:
        raise ValueError(f"JUnit report has no testsuite: {path}")
    for suite in suites:
        properties = suite.find("properties")
        if properties is None:
            properties = ET.Element("properties")
            suite.insert(0, properties)
        recorded = [
            property_element
            for property_element in properties.findall("property")
            if property_element.get("name") == JUNIT_LAUNCH_MODE_PROPERTY
        ]
        if recorded:
            if len(recorded) != 1 or recorded[0].get("value") != launch_mode:
                raise ValueError(f"JUnit launch mode conflicts with {launch_mode}: {path}")
        else:
            ET.SubElement(
                properties,
                "property",
                {"name": JUNIT_LAUNCH_MODE_PROPERTY, "value": launch_mode},
            )
    tree.write(path, encoding="utf-8", xml_declaration=True)


def _require_resumable_launch_mode(path: Path, launch_mode: str) -> None:
    try:
        root = ET.parse(path).getroot()
    except ET.ParseError as error:
        raise ValueError(f"invalid resumable isolated report {path}: {error}") from error
    if len(list(root.iter("testsuite"))) != 1:
        raise ValueError(f"resumable isolated report must contain one testsuite: {path}")
    recorded = _junit_launch_modes(root)
    if recorded and (len(recorded) != 1 or recorded[0] != launch_mode):
        raise ValueError(f"resumable isolated report launch mode differs: {path}")
    if not recorded and launch_mode == "owned":
        raise ValueError(f"owned launch cannot resume an unlabeled isolated report: {path}")


def _prepare_isolated_launch_receipt(case_dir: Path, launch_mode: str, resume: bool) -> None:
    receipt = case_dir / "launch-mode.json"
    if receipt.is_symlink():
        raise SystemExit(f"isolated launch receipt is a symlink: {receipt}")
    if receipt.exists():
        try:
            recorded = json.loads(receipt.read_text())
        except (OSError, ValueError) as error:
            raise SystemExit(f"invalid isolated launch receipt {receipt}: {error}") from error
        if recorded != {"launchMode": launch_mode}:
            raise SystemExit(f"isolated launch receipt differs from {launch_mode}: {receipt}")
        return
    if resume and any(case_dir.iterdir()) and launch_mode == "owned":
        raise SystemExit("owned launch cannot resume unlabeled isolated case reports")
    receipt.write_text(json.dumps({"launchMode": launch_mode}, sort_keys=True) + "\n")


def _run_isolated_cases(
    command_prefix: list[str],
    node_ids: list[str],
    upstream: Path,
    run_cwd: Path,
    env: dict[str, str],
    result_path: Path,
    case_timeout: float,
    resume: bool,
    launch_mode: str = "cdp-adapter",
) -> int:
    if not node_ids or len(set(node_ids)) != len(node_ids):
        raise SystemExit("isolated execution requires a non-empty unique case selection")
    case_dir = result_path.parent / f"{result_path.name}.cases"
    if case_dir.exists() and any(case_dir.iterdir()) and not resume:
        raise SystemExit(
            f"isolated case directory is not empty: {case_dir}; "
            "pass --resume-isolated or choose a new --result"
        )
    case_dir.mkdir(parents=True, exist_ok=True)
    _prepare_isolated_launch_receipt(case_dir, launch_mode, resume)
    cases: list[ET.Element] = []
    for index, node_id in enumerate(node_ids, start=1):
        digest = hashlib.sha256(node_id.encode()).hexdigest()[:16]
        case_path = case_dir / f"{index:05d}-{digest}.xml"
        if resume and case_path.is_file():
            try:
                _require_resumable_launch_mode(case_path, launch_mode)
                case = _read_case_element(case_path, node_id)
            except (ET.ParseError, ValueError) as error:
                raise SystemExit(
                    f"invalid resumable isolated report {case_path}: {error}"
                ) from error
            print(f"isolated case {index}/{len(node_ids)} resume {node_id}", flush=True)
        else:
            source, separator, suffix = node_id.partition("::")
            target = f"{(upstream / source).resolve()}{separator}{suffix}"
            command = [*command_prefix, target, f"--junitxml={case_path}"]
            print(f"isolated case {index}/{len(node_ids)} run {node_id}", flush=True)
            process = subprocess.Popen(
                command,
                cwd=run_cwd,
                env=env,
                start_new_session=os.name == "posix",
            )
            outer_timeout = case_timeout + 30.0
            try:
                returncode = process.wait(timeout=outer_timeout)
            except subprocess.TimeoutExpired:
                if os.name == "posix":
                    os.killpg(process.pid, signal.SIGKILL)
                else:
                    process.kill()
                process.wait()
                returncode = -signal.SIGKILL
            if case_path.is_file():
                try:
                    case = _read_case_element(case_path, node_id)
                    _label_junit_launch_mode(case_path, launch_mode)
                except (ET.ParseError, ValueError) as error:
                    raw_path, raw_digest = _preserve_invalid_case_report(case_path)
                    case = _case_element(
                        node_id,
                        "error",
                        f"isolated pytest report was invalid after exit {returncode}: {error}; "
                        f"original JUnit: {raw_path} sha256={raw_digest}",
                    )
                    _write_single_case(case, case_path)
                    _label_junit_launch_mode(case_path, launch_mode)
            else:
                case = _case_element(
                    node_id,
                    "failure" if returncode == -signal.SIGKILL else "error",
                    (
                        f"isolated pytest process exceeded {outer_timeout:g}s"
                        if returncode == -signal.SIGKILL
                        else f"isolated pytest process exited {returncode} without JUnit"
                    ),
                )
                _write_single_case(case, case_path)
                _label_junit_launch_mode(case_path, launch_mode)
        cases.append(case)
        if index % 25 == 0 and index != len(node_ids):
            _write_isolated_suite(cases, result_path)
            _label_junit_launch_mode(result_path, launch_mode)
    # A successful return is a receipt that a complete aggregate was written
    # this invocation, even when every individual case was resumed. Never let
    # an old aggregate stand in for completion before provenance is emitted.
    _write_isolated_suite(cases, result_path)
    _label_junit_launch_mode(result_path, launch_mode)
    failures = sum(
        case.find("failure") is not None or case.find("error") is not None for case in cases
    )
    print(
        f"isolated execution recorded {len(cases)}/{len(node_ids)} cases; "
        f"failures_or_errors={failures}",
        flush=True,
    )
    return int(failures != 0)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=("oracle", "candidate"))
    parser.add_argument("upstream", type=Path)
    parser.add_argument("--endpoint", help="loopback CDP endpoint for cdp-adapter mode")
    parser.add_argument(
        "--launch-mode",
        choices=("cdp-adapter", "owned"),
        default="cdp-adapter",
        help="select historical CDP attachment or the real public BrowserType.launch API",
    )
    parser.add_argument("--server-prefix")
    parser.add_argument("--browser", action="append", choices=("chromium", "firefox", "webkit"))
    parser.add_argument(
        "--browser-channel",
        choices=(
            "chrome",
            "chrome-beta",
            "chrome-dev",
            "chrome-canary",
            "chromium",
            "msedge",
        ),
        help="pass the same explicit installed-browser channel to both candidate and oracle",
    )
    parser.add_argument("--test", action="append", default=[])
    parser.add_argument(
        "--manifest-cases",
        action="store_true",
        help="run the exact cases pinned by --manifest; forbids ad-hoc --test filters",
    )
    parser.add_argument("--manifest-api", choices=("async", "sync"))
    parser.add_argument(
        "--collection-profile",
        choices=("core", "examples"),
        help="execute one collection profile recorded by the canonical manifest",
    )
    parser.add_argument("--tailscale-ip", default=os.environ.get("TAILSCALE_IP"))
    parser.add_argument("--tailscale-host", default=os.environ.get("TAILSCALE_HOST"))
    parser.add_argument(
        "--secure-http-fixture",
        action="store_true",
        help=(
            "serve the ordinary fixture over trusted Tailscale TLS; use only for "
            "upstream localhost-trust tests"
        ),
    )
    parser.add_argument("--collect-only", action="store_true")
    parser.add_argument(
        "--case-timeout",
        type=float,
        help=(
            "fail an individual executed pytest case after this many seconds so one "
            "candidate hang cannot block corpus accounting"
        ),
    )
    parser.add_argument(
        "--isolate-cases",
        action="store_true",
        help="run every expanded selected inventory row in its own pytest subprocess",
    )
    parser.add_argument(
        "--resume-isolated",
        action="store_true",
        help="reuse already validated per-case JUnit files from an isolated run",
    )
    parser.add_argument(
        "--inventory",
        type=Path,
        default=Path(__file__).parents[1]
        / ".work/official-conformance/playwright-python-v1.58.0-profiled.jsonl",
    )
    parser.add_argument("--result", type=Path)
    parser.add_argument(
        "--bootstrap-only",
        action="store_true",
        help="run the historical seven-case harness instead of the full corpus",
    )
    parser.add_argument(
        "--manifest",
        type=Path,
        default=Path(__file__).parents[1] / "packages/pwright-python/tests/official/manifest.json",
    )
    args = parser.parse_args()
    if args.bootstrap_only and not args.collect_only and not args.server_prefix:
        parser.error("--server-prefix is required for a non-collection bootstrap run")
    if args.bootstrap_only and args.test:
        parser.error("--test cannot be combined with --bootstrap-only")
    if args.bootstrap_only and args.manifest_cases:
        parser.error("--manifest-cases cannot be combined with --bootstrap-only")
    if args.manifest_cases and args.test:
        parser.error("--manifest-cases cannot be combined with --test")
    if args.manifest_api and not args.manifest_cases:
        parser.error("--manifest-api requires --manifest-cases")
    if args.collection_profile and args.bootstrap_only:
        parser.error("--collection-profile cannot be combined with --bootstrap-only")
    if args.collection_profile and args.manifest_cases:
        parser.error("--collection-profile cannot be combined with --manifest-cases")
    if args.case_timeout is not None and args.case_timeout <= 0:
        parser.error("--case-timeout must be greater than zero")
    if args.collect_only and args.case_timeout is not None:
        parser.error("--case-timeout cannot be combined with --collect-only")
    if args.isolate_cases and args.collect_only:
        parser.error("--isolate-cases cannot be combined with --collect-only")
    if args.isolate_cases and args.bootstrap_only:
        parser.error("--isolate-cases cannot be combined with --bootstrap-only")
    if args.isolate_cases and args.case_timeout is None:
        parser.error("--isolate-cases requires --case-timeout")
    if args.resume_isolated and not args.isolate_cases:
        parser.error("--resume-isolated requires --isolate-cases")
    if args.bootstrap_only and args.launch_mode == "owned":
        parser.error("--bootstrap-only supports only cdp-adapter launch mode")
    root = Path(__file__).parents[1]
    try:
        tooling_source_records = tooling_sources(root)
    except ValueError as error:
        raise SystemExit(str(error)) from error
    scope = (
        "bootstrap"
        if args.bootstrap_only
        else f"profile-{args.collection_profile}"
        if args.collection_profile is not None
        else f"manifest-cases-{args.manifest_api or 'all'}"
        if args.manifest_cases
        else "selected"
        if args.test
        else "full"
    )
    result_path = prepare_result_destination(
        args.result
        or root
        / ".work/official-conformance/results"
        / (
            f"playwright-python-{args.mode}-{scope}"
            f"{'-owned' if args.launch_mode == 'owned' else ''}"
            f"{'-collection' if args.collect_only else ''}.xml"
        ),
        resume_isolated=args.resume_isolated,
    )
    official = root / "packages/pwright-python/tests/official"
    upstream = args.upstream.resolve()
    manifest_path = args.manifest.resolve()
    verification = subprocess.run(
        [
            sys.executable,
            str(root / "scripts/check_playwright_python_official_source.py"),
            str(upstream),
            "--manifest",
            str(manifest_path),
        ],
        cwd=root,
        check=False,
    )
    if verification.returncode:
        raise SystemExit(verification.returncode)
    manifest = cast(Manifest, json.loads(manifest_path.read_text()))
    profile = (
        _collection_profile(manifest, args.collection_profile)
        if args.collection_profile is not None
        else None
    )
    if args.collection_profile == "examples":
        if args.browser not in (None, ["chromium"]):
            parser.error("examples collection profile supports only Chromium")
        if args.browser_channel is not None:
            parser.error("examples collection profile does not use a browser channel")
    try:
        persistent_page_argument_browserless = _persistent_page_argument_browserless_mode(
            manifest,
            manifest_cases=args.manifest_cases,
            manifest_api=args.manifest_api,
            browsers=args.browser or [],
            browser_channel=args.browser_channel,
            launch_mode=args.launch_mode,
            endpoint=args.endpoint,
            isolate_cases=args.isolate_cases,
            collect_only=args.collect_only,
            case_timeout=args.case_timeout,
            server_prefix=args.server_prefix,
            secure_http_fixture=args.secure_http_fixture,
        )
        proxy_bad_server_browserless = _proxy_bad_server_browserless_mode(
            manifest,
            manifest_cases=args.manifest_cases,
            manifest_api=args.manifest_api,
            browsers=args.browser or [],
            browser_channel=args.browser_channel,
            launch_mode=args.launch_mode,
            endpoint=args.endpoint,
            isolate_cases=args.isolate_cases,
            collect_only=args.collect_only,
            case_timeout=args.case_timeout,
            server_prefix=args.server_prefix,
            secure_http_fixture=args.secure_http_fixture,
        )
        exact_browserless_preflight = (
            persistent_page_argument_browserless or proxy_bad_server_browserless
        )
        requires_browser, requires_fixture_servers = _execution_requirements(
            manifest, args.manifest_cases, args.collection_profile
        )
        sync_lifecycle_loop = _sync_lifecycle_loop_mode(
            manifest,
            manifest_cases=args.manifest_cases,
            manifest_api=args.manifest_api,
            browsers=args.browser or [],
            browser_channel=args.browser_channel,
            isolate_cases=args.isolate_cases,
            collect_only=args.collect_only,
        )
        endpoint = _select_launch_endpoint(args.launch_mode, args.endpoint)
        fixture_ip: str | None = None
        fixture_host: str | None = None
        if not args.collect_only and not args.bootstrap_only and requires_fixture_servers:
            if not args.tailscale_ip or not args.tailscale_host:
                raise ValueError(
                    "official execution requires --tailscale-ip and --tailscale-host "
                    "for the upstream fixture servers"
                )
            fixture_ip, fixture_host = _validate_tailscale_fixture(
                cast(str, args.tailscale_ip), cast(str, args.tailscale_host)
            )
        server_prefix = (
            _validate_server_prefix(args.server_prefix) if args.server_prefix is not None else None
        )
    except (TypeError, ValueError) as error:
        parser.error(str(error))
    preflight_runtime_package: Path | None = None
    if exact_browserless_preflight:
        try:
            _require_persistent_preflight_harness_sources(official, args.mode)
            preflight_runtime_package = (
                Path(importlib.metadata.distribution("playwright").locate_file("playwright"))
                if args.mode == "oracle"
                else root / "packages/pwright-python/python/pwright"
            )
            if persistent_page_argument_browserless:
                _require_persistent_preflight_runtime_sources(args.mode, preflight_runtime_package)
            else:
                _require_proxy_preflight_runtime_sources(args.mode, preflight_runtime_package)
        except (OSError, ValueError, importlib.metadata.PackageNotFoundError) as error:
            parser.error(str(error))
    if args.launch_mode == "cdp-adapter" and not args.collect_only and requires_browser:
        try:
            existing_pages = _existing_cdp_pages(cast(str, endpoint))
        except ValueError as error:
            parser.error(str(error))
        if existing_pages:
            descriptions = ", ".join(
                f"{page.get('title', '')!s} ({page.get('url', '')!s})"
                for page in existing_pages[:4]
            )
            parser.error(
                "refusing official conformance against dedicated Chrome with existing "
                f"nonblank pages: {descriptions}"
            )
    selected_tests = (
        [case["id"] for case in manifest["cases"]] if args.manifest_cases else args.test
    )
    if profile is not None:
        profile_selection = cast(str, profile["selection"])
        if not selected_tests:
            selected_tests = [profile_selection]
        invalid_profile_selections = [
            selection
            for selection in selected_tests
            if not _selection_is_in_profile(selection, profile_selection)
        ]
        if invalid_profile_selections:
            parser.error(
                f"--test selection is outside {args.collection_profile} profile: "
                f"{invalid_profile_selections[0]}"
            )
    if args.manifest_api:
        prefix = f"tests/{args.manifest_api}/"
        selected_tests = [test for test in selected_tests if test.startswith(prefix)]
        if not selected_tests:
            parser.error(f"manifest has no {args.manifest_api} cases")
    python_paths: list[str] = []
    harness_inputs = [official / ("harness" if args.bootstrap_only else "runtime")]
    if not args.bootstrap_only:
        python_paths.append(str(official / "runtime"))
    if args.bootstrap_only:
        python_paths.append(str(official / "harness"))
    if args.mode == "candidate":
        python_paths.insert(0, str(official / "shim"))
        python_paths.append(str(upstream))
        harness_inputs.insert(0, official / "shim")

    env = os.environ.copy()
    if exact_browserless_preflight:
        # Ambient startup hooks and pytest plugins could extend the certified graph.
        for key in (
            "CI",
            "NODE_OPTIONS",
            "NODE_PATH",
            "PYTHONHOME",
            "PYTHONINSPECT",
            "PYTHONPATH",
            "PYTHONUSERBASE",
            "PYTEST_ADDOPTS",
            "PYTEST_PLUGINS",
        ):
            env.pop(key, None)
        env["PYTHONNOUSERSITE"] = "1"
        env["PYTEST_DISABLE_PLUGIN_AUTOLOAD"] = "1"
    existing = env.get("PYTHONPATH")
    if existing:
        python_paths.append(existing)
    env["PYTHONPATH"] = os.pathsep.join(python_paths)
    _configure_launch_environment(env, args.launch_mode, endpoint)
    env["PWRIGHT_OFFICIAL_MODE"] = args.mode
    env["PWRIGHT_OFFICIAL_BROWSER_CHANNEL"] = args.browser_channel or ""
    env["PWRIGHT_OFFICIAL_UPSTREAM_TESTS"] = str(upstream / "tests")
    if fixture_ip is not None:
        env["PWRIGHT_OFFICIAL_TAILSCALE_IP"] = fixture_ip
    if fixture_host is not None:
        env["PWRIGHT_OFFICIAL_TAILSCALE_HOST"] = fixture_host
    if args.secure_http_fixture:
        env["PWRIGHT_OFFICIAL_SECURE_HTTP_FIXTURE"] = "1"
    if server_prefix is not None:
        env["PWRIGHT_OFFICIAL_SERVER_PREFIX"] = server_prefix

    if args.launch_mode == "owned":
        try:
            _require_owned_public_launch(sys.executable, args.mode, cwd=root, env=env)
        except ValueError as error:
            parser.error(str(error))

    validated_targets: list[str] = []
    for test in selected_tests:
        if test.startswith("-"):
            parser.error("--test values must be pytest node IDs, not options")
        source, separator, suffix = test.partition("::")
        source_path = Path(source)
        if source_path.is_absolute() or ".." in source_path.parts:
            parser.error("--test values must be relative to the pinned upstream checkout")
        resolved = (upstream / source_path).resolve()
        if not resolved.is_relative_to(upstream) or not resolved.exists():
            parser.error(f"--test source does not exist in pinned upstream: {source}")
        validated_targets.append(f"{resolved}{separator}{suffix}")

    command: list[str] = (
        [sys.executable, "-c", SYNC_LIFECYCLE_LOOP_WRAPPER, "-q"]
        if sync_lifecycle_loop
        else [sys.executable, "-m", "pytest", "-q"]
    )
    run_cwd = root
    if args.bootstrap_only:
        command.extend(
            [
                "--noconftest",
                "--import-mode=importlib",
                "-c",
                str(official / "pytest.ini"),
                "-p",
                "pwright_official_harness.plugin",
            ]
        )
    else:
        command.extend(
            [
                "--import-mode=importlib",
                "-c",
                str(upstream / "pyproject.toml"),
            ]
        )
        if exact_browserless_preflight:
            command.extend(["-p", "pytest_asyncio.plugin", "-p", "pytest_timeout"])
        if not args.collect_only and requires_fixture_servers:
            command.extend(["-p", "pwright_official_server_runtime"])
        if args.mode == "candidate" and not args.collect_only and requires_fixture_servers:
            command.extend(["-p", "pwright_candidate_runtime"])
        if args.mode == "oracle" and not args.collect_only:
            command.extend(["-p", "pwright_official_oracle_runtime"])
    browsers = cast(list[str], args.browser or [])
    for browser in browsers:
        command.append(f"--browser={browser}")
    if args.browser_channel:
        command.append(f"--browser-channel={args.browser_channel}")
    if args.collect_only:
        command.append("--collect-only")
    elif args.case_timeout is not None:
        command.extend(
            [
                f"--timeout={args.case_timeout:g}",
                "--timeout-method=signal",
            ]
        )
    corpus_cases = (
        cast(int, profile["expandedTests"])
        if profile is not None
        else manifest["corpus"]["expandedTests"]
    )
    result_path.parent.mkdir(parents=True, exist_ok=True)
    inventory_path = args.inventory.resolve() if args.isolate_cases else None
    isolated_node_ids = (
        _isolated_cases(
            cast(Path, inventory_path),
            cast(list[str], selected_tests),
            browsers,
        )
        if args.isolate_cases
        else []
    )
    if args.isolate_cases:
        scope_count = f"isolated_cases={len(isolated_node_ids)}"
    elif args.bootstrap_only:
        scope_count = f"selected_cases={len(manifest['cases'])}"
    elif selected_tests:
        scope_count = f"full_manifest_cases={corpus_cases} selected_sources={len(selected_tests)}"
    else:
        scope_count = f"expected_cases={corpus_cases}"
    print(
        f"running official corpus mode={args.mode} launch_mode={args.launch_mode} scope={scope} "
        f"{scope_count} collect_only={args.collect_only} "
        f"requires_browser={requires_browser} "
        f"requires_fixture_servers={requires_fixture_servers}",
        flush=True,
    )
    print(f"result={result_path}", flush=True)
    runtime_identity = probe_runtime_identity(
        sys.executable,
        args.mode,
        cwd=run_cwd,
        env=env,
    )
    profile_identity = (
        probe_collection_profile_identity(
            sys.executable,
            profile,
            tag=manifest["tag"],
            root=root,
        )
        if profile is not None
        else None
    )
    probe = runtime_identity["playwrightPackage"]
    shim_root = (official / "shim").resolve()
    loaded_from_shim = Path(probe).is_relative_to(shim_root)
    if args.mode == "candidate" and not loaded_from_shim:
        raise SystemExit(f"candidate mode loaded the wrong playwright package: {probe}")
    if args.mode == "oracle" and loaded_from_shim:
        raise SystemExit(f"oracle mode loaded the candidate shim: {probe}")
    if exact_browserless_preflight:
        package_value = (
            runtime_identity["playwrightPackage"]
            if args.mode == "oracle"
            else runtime_identity["pwrightPackage"]
        )
        if package_value is None:
            raise SystemExit("browserless preflight has no runtime package")
        actual_runtime_package = Path(package_value)
        if (
            preflight_runtime_package is None
            or actual_runtime_package.resolve() != preflight_runtime_package.resolve()
        ):
            raise SystemExit("browserless preflight loaded a different runtime package")
        try:
            if persistent_page_argument_browserless:
                _require_persistent_preflight_runtime_sources(args.mode, actual_runtime_package)
            else:
                _require_proxy_preflight_runtime_sources(args.mode, actual_runtime_package)
        except ValueError as error:
            raise SystemExit(str(error)) from error
    print(f"loaded playwright package: {probe}", flush=True)
    registry_snapshot: dict[str, object] | None = None
    browsers_json_path: Path | None = None
    default_browsers: list[str] = []
    if browser_registry_diagnostic_selected(
        cast(dict[str, object], manifest),
        cast(list[str], selected_tests),
        manifest_cases=args.manifest_cases,
        collect_only=args.collect_only,
    ):
        try:
            browsers_json_path = probe_official_browsers_json(sys.executable, cwd=run_cwd, env=env)
            pinned_default_browsers = manifest["corpus"].get("defaultBrowsers")
            if pinned_default_browsers is None:
                raise ValueError("browser registry diagnostic requires default browsers")
            default_browsers = pinned_default_browsers
            registry_snapshot = browser_registry_snapshot(
                mode=args.mode,
                runtime=runtime_identity,
                cwd=run_cwd,
                env=env,
                engines=browsers or default_browsers,
                browsers_json_path=browsers_json_path,
            )
        except (OSError, ValueError, subprocess.CalledProcessError) as error:
            raise SystemExit(f"browser registry preflight failed: {error}") from error
    if args.isolate_cases:
        exit_code = _run_isolated_cases(
            command,
            isolated_node_ids,
            upstream,
            run_cwd,
            env,
            result_path,
            cast(float, args.case_timeout),
            cast(bool, args.resume_isolated),
            args.launch_mode,
        )
    else:
        if args.bootstrap_only:
            command.extend(str(upstream / case["id"]) for case in manifest["cases"])
        elif validated_targets:
            command.extend(validated_targets)
        else:
            command.append(str(upstream))
        command.append(f"--junitxml={result_path}")
        exit_code = subprocess.run(command, cwd=run_cwd, env=env, check=False).returncode
    if preflight_runtime_package is not None:
        try:
            if persistent_page_argument_browserless:
                _require_persistent_preflight_runtime_sources(args.mode, preflight_runtime_package)
            else:
                _require_proxy_preflight_runtime_sources(args.mode, preflight_runtime_package)
            _require_persistent_preflight_harness_sources(official, args.mode)
        except ValueError as error:
            raise SystemExit(str(error)) from error
    if not result_path.is_file():
        print("runner did not produce the requested JUnit result", file=sys.stderr)
        raise SystemExit(exit_code or 2)
    try:
        _label_junit_launch_mode(result_path, args.launch_mode)
    except ValueError as error:
        raise SystemExit(str(error)) from error
    if registry_snapshot is not None and browsers_json_path is not None:
        try:
            require_browser_registry_unchanged(
                registry_snapshot,
                mode=args.mode,
                python=sys.executable,
                runtime=runtime_identity,
                cwd=run_cwd,
                env=env,
                engines=browsers or default_browsers,
                browsers_json_path=browsers_json_path,
            )
        except (OSError, ValueError, subprocess.CalledProcessError) as error:
            raise SystemExit(str(error)) from error
    try:
        require_tooling_unchanged(root, tooling_source_records)
    except ValueError as error:
        raise SystemExit(str(error)) from error
    provenance = build_provenance(
        mode=args.mode,
        launch_mode=args.launch_mode,
        capture_runtime_libraries=True,
        manifest=cast(dict[str, object], manifest),
        manifest_path=manifest_path,
        upstream=upstream,
        scope=scope,
        browsers=browsers,
        browser_channel=args.browser_channel,
        selections=cast(list[str], selected_tests),
        manifest_cases=args.manifest_cases,
        manifest_api=args.manifest_api,
        bootstrap_only=args.bootstrap_only,
        collect_only=args.collect_only,
        endpoint=endpoint,
        fixture_bind=fixture_ip,
        fixture_host=fixture_host,
        server_prefix=server_prefix,
        secure_http_fixture=args.secure_http_fixture,
        case_timeout=args.case_timeout,
        isolate_cases=args.isolate_cases,
        resume_isolated=args.resume_isolated,
        inventory=inventory_path,
        isolated_node_ids=isolated_node_ids,
        command=command,
        runtime=runtime_identity,
        harness_inputs=harness_inputs,
        result=result_path,
        exit_code=exit_code,
        collection_profile=profile_identity,
        tooling_source_records=tooling_source_records,
        browser_registry=registry_snapshot,
    )
    if sync_lifecycle_loop:
        provenance["executionMode"] = SYNC_LIFECYCLE_LOOP_MODE
        provenance["executionWrapperSha256"] = SYNC_LIFECYCLE_LOOP_WRAPPER_SHA256
    elif persistent_page_argument_browserless:
        provenance["executionMode"] = PERSISTENT_PAGE_ARGUMENT_MODE
    elif proxy_bad_server_browserless:
        provenance["executionMode"] = PROXY_BAD_SERVER_MODE
    provenance_path = result_path.with_suffix(".provenance.json")
    try:
        require_tooling_unchanged(root, tooling_source_records)
        if registry_snapshot is not None and browsers_json_path is not None:
            require_browser_registry_unchanged(
                registry_snapshot,
                mode=args.mode,
                python=sys.executable,
                runtime=runtime_identity,
                cwd=run_cwd,
                env=env,
                engines=browsers or default_browsers,
                browsers_json_path=browsers_json_path,
            )
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        raise SystemExit(str(error)) from error
    provenance_path.write_text(json.dumps(provenance, indent=2, sort_keys=True) + "\n")
    print(f"provenance={provenance_path}", flush=True)
    raise SystemExit(exit_code)


if __name__ == "__main__":
    main()
