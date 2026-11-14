#!/usr/bin/env python3
"""Verify a Playwright Python conformance result provenance sidecar."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import NoReturn, cast


class ProvenanceValidationError(ValueError):
    """Raised when a Python result sidecar fails closed."""


_SHA256_PATTERN = re.compile(r"^[0-9a-f]{64}$")
_EXPECTED_KEYS = frozenset(
    {
        "schema",
        "mode",
        "repository",
        "tag",
        "commit",
        "manifest",
        "manifestSha256",
        "upstream",
        "scope",
        "browsers",
        "browserChannel",
        "selections",
        "manifestCases",
        "manifestApi",
        "bootstrapOnly",
        "collectOnly",
        "endpoint",
        "fixtureBind",
        "fixtureHost",
        "serverPrefix",
        "secureHttpFixture",
        "caseTimeout",
        "isolateCases",
        "resumeIsolated",
        "inventory",
        "inventorySha256",
        "isolatedCaseCount",
        "isolatedCaseIdsSha256",
        "pythonExecutable",
        "pythonVersion",
        "playwrightPackage",
        "playwrightPackageSha256",
        "pwrightPackage",
        "pwrightPackageSha256",
        "pwrightArtifact",
        "pwrightArtifactSha256",
        "harnessInputs",
        "command",
        "exitCode",
        "result",
        "resultSha256",
        "caseReports",
        "caseReportsSha256",
    }
)
_PROFILE_KEY = "collectionProfile"
_LOOP_KEYS = frozenset({"executionMode", "executionWrapperSha256"})
_PERSISTENT_PREFLIGHT_KEYS = frozenset({"executionMode"})
_TOOLING_KEY = "toolingSources"
_BROWSER_REGISTRY_KEY = "browserRegistry"
_LAUNCH_MODE_KEY = "launchMode"
_RUNTIME_LIBRARIES_KEY = "pwrightRuntimeLibraries"
_JUNIT_LAUNCH_MODE_PROPERTY = "pwright.official.launchMode"
_TOOLING_SOURCE_PATHS = (
    "scripts/check_playwright_python_official_source.py",
    "scripts/official_result_destination.py",
    "scripts/playwright_python_official_provenance.py",
    "scripts/run_playwright_python_official_conformance.py",
)


def _fail(message: str) -> NoReturn:
    raise ProvenanceValidationError(message)


def _string(payload: dict[str, object], key: str) -> str:
    value = payload[key]
    if not isinstance(value, str) or not value:
        _fail(f"{key} must be a non-empty string")
    return value


def _nullable_string(payload: dict[str, object], key: str) -> str | None:
    value = payload[key]
    if value is not None and (not isinstance(value, str) or not value):
        _fail(f"{key} must be null or a non-empty string")
    return value if isinstance(value, str) else None


def _boolean(payload: dict[str, object], key: str) -> bool:
    value = payload[key]
    if not isinstance(value, bool):
        _fail(f"{key} must be a boolean")
    return value


def _sha(payload: dict[str, object], key: str, *, nullable: bool = False) -> str | None:
    value = payload[key]
    if nullable and value is None:
        return None
    if not isinstance(value, str) or not _SHA256_PATTERN.fullmatch(value):
        _fail(f"{key} must be a lowercase SHA-256 digest")
    return value


def _verify_file(path: Path, expected: str, label: str) -> None:
    from playwright_python_official_provenance import file_sha256

    if not path.is_file():
        _fail(f"{label} is missing: {path}")
    actual = file_sha256(path)
    if actual != expected:
        _fail(f"{label} SHA-256 mismatch: expected {expected}, got {actual}")


def _verify_tree(path: Path, expected: str, label: str) -> None:
    from playwright_python_official_provenance import tree_sha256

    try:
        actual = tree_sha256(path)
    except ValueError as error:
        raise ProvenanceValidationError(str(error)) from error
    if actual != expected:
        _fail(f"{label} SHA-256 mismatch: expected {expected}, got {actual}")


def _verify_archived_package_root(path: Path) -> None:
    if path.is_symlink() or not path.is_dir():
        _fail(f"archived Pwright package is missing or a symlink: {path}")
    resolved_root = path.resolve()
    for entry in path.rglob("*"):
        if entry.is_symlink() or not entry.resolve().is_relative_to(resolved_root):
            _fail(f"archived Pwright package contains a symlink or escape: {entry}")


def _verify_runtime_libraries(raw: object, recorded_package: Path, verified_package: Path) -> None:
    from playwright_python_official_provenance import pwright_runtime_library_paths

    if not isinstance(raw, list):
        _fail("pwrightRuntimeLibraries must be a list")
    records = cast(list[object], raw)
    if not recorded_package.is_absolute():
        _fail("recorded Pwright package must be absolute for runtime libraries")
    recorded_directory = recorded_package.parent / "shuozeli_pwright.dylibs"
    try:
        verified_paths = pwright_runtime_library_paths(verified_package)
    except ValueError as error:
        raise ProvenanceValidationError(str(error)) from error
    if len(records) != len(verified_paths):
        _fail("Pwright runtime library set differs from provenance")
    for index, (record, actual_path) in enumerate(zip(records, verified_paths)):
        if not isinstance(record, dict):
            _fail(f"pwrightRuntimeLibraries[{index}] must contain only path and sha256")
        record_fields = cast(dict[object, object], record)
        if set(record_fields) != {"path", "sha256"}:
            _fail(f"pwrightRuntimeLibraries[{index}] must contain only path and sha256")
        if record_fields["path"] != str(recorded_directory / actual_path.name):
            _fail(f"pwrightRuntimeLibraries[{index}] has the wrong adjacent path")
        digest = record_fields["sha256"]
        if not isinstance(digest, str) or not _SHA256_PATTERN.fullmatch(digest):
            _fail(f"pwrightRuntimeLibraries[{index}].sha256 is invalid")
        _verify_file(actual_path, digest, f"pwrightRuntimeLibraries[{index}]")


def _verify_junit_launch_mode(
    root: ET.Element, expected: str, label: str, *, require: bool = True
) -> None:
    suites = list(root.iter("testsuite"))
    if not suites:
        _fail(f"{label} has no testsuite")
    for suite in suites:
        properties = suite.find("properties")
        matches = (
            []
            if properties is None
            else [
                property_element
                for property_element in properties.findall("property")
                if property_element.get("name") == _JUNIT_LAUNCH_MODE_PROPERTY
            ]
        )
        if not matches and not require:
            continue
        if len(matches) != 1 or matches[0].get("value") != expected:
            _fail(f"{label} launch mode differs from provenance")


def _verify_tooling_sources(raw: object, root: Path) -> None:
    if not isinstance(raw, list):
        _fail("toolingSources must contain exactly the fixed Python capture sources")
    records = cast(list[object], raw)
    if len(records) != len(_TOOLING_SOURCE_PATHS):
        _fail("toolingSources must contain exactly the fixed Python capture sources")
    if (root / "scripts").is_symlink():
        _fail("toolingSources source directory is a symlink")
    for index, relative in enumerate(_TOOLING_SOURCE_PATHS):
        record = records[index]
        if not isinstance(record, dict):
            _fail(f"toolingSources[{index}] must contain only path and sha256")
        record_fields = cast(dict[object, object], record)
        if set(record_fields) != {"path", "sha256"}:
            _fail(f"toolingSources[{index}] must contain only path and sha256")
        if record_fields["path"] != relative:
            _fail(f"toolingSources[{index}] has the wrong fixed source path")
        digest = record_fields["sha256"]
        if not isinstance(digest, str) or not _SHA256_PATTERN.fullmatch(digest):
            _fail(f"toolingSources[{index}].sha256 is invalid")
        path = root / relative
        if path.is_symlink() or not path.is_file() or not path.resolve().is_relative_to(root):
            _fail(f"toolingSources[{index}] source is missing or a symlink: {relative}")
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
        if actual != digest:
            _fail(f"toolingSources[{index}] SHA-256 mismatch: expected {digest}, got {actual}")


def _git(upstream: Path, *args: str) -> str:
    completed = subprocess.run(
        ["git", "-C", str(upstream), *args],
        check=True,
        capture_output=True,
        text=True,
    )
    return completed.stdout.strip()


def _manifest_cases(manifest: dict[str, object]) -> list[dict[str, object]]:
    raw_cases = manifest.get("cases")
    if not isinstance(raw_cases, list):
        _fail("manifest cases must be a list")
    cases: list[dict[str, object]] = []
    for index, raw_case in enumerate(cast(list[object], raw_cases)):
        if not isinstance(raw_case, dict):
            _fail(f"manifest cases[{index}] must be an object")
        case = cast(dict[str, object], raw_case)
        if not isinstance(case.get("id"), str) or not case["id"]:
            _fail(f"manifest cases[{index}].id must be a non-empty string")
        cases.append(case)
    return cases


def _expected_command(
    payload: dict[str, object],
    manifest: dict[str, object],
    upstream: Path,
    *,
    requires_fixture_servers: bool,
    sync_lifecycle_loop: bool,
    exact_browserless_preflight: bool,
) -> list[str]:
    from run_playwright_python_official_conformance import SYNC_LIFECYCLE_LOOP_WRAPPER

    command = (
        [_string(payload, "pythonExecutable"), "-c", SYNC_LIFECYCLE_LOOP_WRAPPER, "-q"]
        if sync_lifecycle_loop
        else [_string(payload, "pythonExecutable"), "-m", "pytest", "-q"]
    )
    bootstrap_only = _boolean(payload, "bootstrapOnly")
    collect_only = _boolean(payload, "collectOnly")
    mode = _string(payload, "mode")
    manifest_path = Path(_string(payload, "manifest"))
    root = manifest_path.parents[4]
    official = root / "packages/pwright-python/tests/official"
    if bootstrap_only:
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
        command.extend(["--import-mode=importlib", "-c", str(upstream / "pyproject.toml")])
        if exact_browserless_preflight:
            command.extend(["-p", "pytest_asyncio.plugin", "-p", "pytest_timeout"])
        if not collect_only:
            if requires_fixture_servers:
                command.extend(["-p", "pwright_official_server_runtime"])
            if mode == "candidate":
                if requires_fixture_servers:
                    command.extend(["-p", "pwright_candidate_runtime"])
            else:
                command.extend(["-p", "pwright_official_oracle_runtime"])
    for browser in cast(list[str], payload["browsers"]):
        command.append(f"--browser={browser}")
    browser_channel = payload["browserChannel"]
    if browser_channel is not None:
        command.append(f"--browser-channel={browser_channel}")
    if collect_only:
        command.append("--collect-only")
    elif payload["caseTimeout"] is not None:
        command.extend([f"--timeout={payload['caseTimeout']:g}", "--timeout-method=signal"])
    if _boolean(payload, "isolateCases"):
        return command
    selections = cast(list[str], payload["selections"])
    if bootstrap_only:
        cases = _manifest_cases(manifest)
        command.extend(str(upstream / cast(str, case["id"])) for case in cases)
    elif selections:
        for selection in selections:
            source, separator, suffix = selection.partition("::")
            command.append(f"{(upstream / source).resolve()}{separator}{suffix}")
    else:
        command.append(str(upstream))
    command.append(f"--junitxml={payload['result']}")
    return command


def verify_provenance(
    raw: object,
    *,
    provenance_path: Path,
    manifest_override: Path | None = None,
    result_override: Path | None = None,
    upstream_override: Path | None = None,
    tooling_root: Path | None = None,
    harness_root: Path | None = None,
    package_root: Path | None = None,
    check_checkout: bool = True,
) -> dict[str, object]:
    if not isinstance(raw, dict):
        _fail("provenance root must be a JSON object with string keys")
    raw_object = cast(dict[object, object], raw)
    if not all(isinstance(key, str) for key in raw_object):
        _fail("provenance root must be a JSON object with string keys")
    payload = cast(dict[str, object], raw)
    actual_keys = frozenset(payload)
    schema = payload.get("schema")
    if isinstance(schema, bool) or schema not in {1, 2, 3, 4, 5}:
        _fail(f"unsupported provenance schema: {schema!r}")
    tooling_key: frozenset[str] = (
        frozenset({_TOOLING_KEY}) if schema in {2, 3, 4, 5} else frozenset()
    )
    launch_key: frozenset[str] = frozenset({_LAUNCH_MODE_KEY}) if schema in {4, 5} else frozenset()
    runtime_libraries_key: frozenset[str] = (
        frozenset({_RUNTIME_LIBRARIES_KEY}) if schema == 5 else frozenset()
    )
    registry_key: frozenset[str] = (
        frozenset({_BROWSER_REGISTRY_KEY})
        if schema == 3 or (schema in {4, 5} and _BROWSER_REGISTRY_KEY in actual_keys)
        else frozenset()
    )
    required_keys: frozenset[str] = (
        _EXPECTED_KEYS | tooling_key | registry_key | launch_key | runtime_libraries_key
    )
    valid_key_sets: set[frozenset[str]] = (
        {required_keys}
        if schema == 3
        else {
            required_keys,
            required_keys | {_PROFILE_KEY},
            required_keys | _LOOP_KEYS,
        }
    )
    if schema == 5:
        valid_key_sets.add(required_keys | _PERSISTENT_PREFLIGHT_KEYS)
    if actual_keys not in valid_key_sets:
        expected_keys = required_keys
        if _PROFILE_KEY in actual_keys:
            expected_keys |= {_PROFILE_KEY}
        if actual_keys & _LOOP_KEYS:
            expected_keys |= (
                _LOOP_KEYS
                if "executionWrapperSha256" in actual_keys
                else _PERSISTENT_PREFLIGHT_KEYS
            )
        _fail(
            "provenance fields differ: "
            f"missing={sorted(expected_keys - actual_keys)}, "
            f"extra={sorted(actual_keys - expected_keys)}"
        )
    if schema in {2, 3, 4, 5}:
        _verify_tooling_sources(
            payload[_TOOLING_KEY],
            (tooling_root or Path(__file__).parents[1]).resolve(),
        )
    # Delay capture-tool imports until source-bound tooling identity is checked.
    from playwright_python_official_provenance import (
        browser_registry_diagnostic_selected,
        browser_registry_snapshot,
        probe_collection_profile_identity,
        probe_official_browsers_json,
        string_list_sha256,
    )
    from run_playwright_python_official_conformance import (
        PERSISTENT_PAGE_ARGUMENT_MODE,
        PERSISTENT_PREFLIGHT_HARNESS_SHA256,
        PROXY_BAD_SERVER_MODE,
        SYNC_LIFECYCLE_CASE_IDS,
        SYNC_LIFECYCLE_LOOP_MODE,
        SYNC_LIFECYCLE_LOOP_WRAPPER_SHA256,
        Manifest,
        collection_profile,
        execution_requirements,
        isolated_cases,
        persistent_page_argument_browserless_mode,
        proxy_bad_server_browserless_mode,
        require_persistent_preflight_runtime_sources,
        require_proxy_preflight_runtime_sources,
        selection_is_in_profile,
        sync_lifecycle_loop_mode,
        validate_loopback_endpoint,
        validate_server_prefix,
        validate_tailscale_fixture,
    )

    mode = _string(payload, "mode")
    if mode not in {"candidate", "oracle"}:
        _fail("mode must be candidate or oracle")
    if package_root is not None and mode != "candidate":
        _fail("archived Pwright package is only valid for candidate provenance")
    launch_mode = _string(payload, _LAUNCH_MODE_KEY) if schema in {4, 5} else "cdp-adapter"
    if launch_mode not in {"cdp-adapter", "owned"}:
        _fail("launchMode must be cdp-adapter or owned")

    manifest_digest = cast(str, _sha(payload, "manifestSha256"))
    recorded_manifest = Path(_string(payload, "manifest"))
    actual_manifest = (manifest_override or recorded_manifest).resolve()
    _verify_file(actual_manifest, manifest_digest, "manifest")
    parsed_manifest = json.loads(actual_manifest.read_text())
    if not isinstance(parsed_manifest, dict):
        _fail("manifest root must be a JSON object")
    manifest = cast(dict[str, object], parsed_manifest)
    if manifest.get("schema") != 1:
        _fail("manifest schema must be 1")
    for key in ("repository", "tag", "commit"):
        if manifest.get(key) != payload[key]:
            _fail(f"manifest {key} differs from provenance")

    recorded_upstream = Path(_string(payload, "upstream"))
    actual_upstream = (upstream_override or recorded_upstream).resolve()
    if check_checkout:
        if _git(actual_upstream, "rev-parse", "HEAD") != payload["commit"]:
            _fail("upstream checkout no longer matches the pinned commit")
        if _git(actual_upstream, "status", "--porcelain"):
            _fail("upstream checkout is dirty")
    if not (actual_upstream / "pyproject.toml").is_file():
        _fail("upstream pyproject.toml is missing")

    raw_browsers = payload["browsers"]
    if not isinstance(raw_browsers, list):
        _fail("browsers contain an unsupported value")
    browser_values = cast(list[object], raw_browsers)
    if not all(browser in {"chromium", "firefox", "webkit"} for browser in browser_values):
        _fail("browsers contain an unsupported value")
    browsers = cast(list[str], browser_values)
    if len(browsers) != len(set(browsers)):
        _fail("browsers must not contain duplicates")
    _nullable_string(payload, "browserChannel")
    raw_selections = payload["selections"]
    if not isinstance(raw_selections, list):
        _fail("selections must be a list of non-empty strings")
    selection_values = cast(list[object], raw_selections)
    if not all(isinstance(selection, str) and selection for selection in selection_values):
        _fail("selections must be a list of non-empty strings")
    selections = cast(list[str], selection_values)
    profile_name: str | None = None
    profile_manifest: dict[str, object] | None = None
    if _PROFILE_KEY in payload:
        raw_profile = payload[_PROFILE_KEY]
        if not isinstance(raw_profile, dict):
            _fail("collectionProfile must be an object")
        profile_record = cast(dict[str, object], raw_profile)
        raw_profile_name = profile_record.get("name")
        if not isinstance(raw_profile_name, str) or not raw_profile_name:
            _fail("collectionProfile.name must be a non-empty string")
        profile_name = raw_profile_name
        try:
            profile_manifest = collection_profile(cast(Manifest, manifest), profile_name)
        except ValueError as error:
            raise ProvenanceValidationError(str(error)) from error
        profile_selection = cast(str, profile_manifest["selection"])
        if not selections or any(
            not selection_is_in_profile(selection, profile_selection) for selection in selections
        ):
            _fail("selections differ from the recorded collection profile")
    manifest_cases = _boolean(payload, "manifestCases")
    manifest_api = _nullable_string(payload, "manifestApi")
    if manifest_api not in {None, "async", "sync"}:
        _fail("manifestApi must be async, sync or null")
    cases = _manifest_cases(manifest)
    if manifest_cases:
        expected_selections = [
            cast(str, case["id"])
            for case in cases
            if manifest_api is None or cast(str, case["id"]).startswith(f"tests/{manifest_api}/")
        ]
        if selections != expected_selections:
            _fail("manifest-case selections differ from the pinned manifest")
    elif manifest_api is not None:
        _fail("manifestApi requires manifestCases")
    try:
        _, requires_fixture_servers = execution_requirements(
            cast(Manifest, manifest), manifest_cases, profile_name
        )
    except (TypeError, ValueError) as error:
        raise ProvenanceValidationError(str(error)) from error

    bootstrap_only = _boolean(payload, "bootstrapOnly")
    collect_only = _boolean(payload, "collectOnly")
    registry_selected = browser_registry_diagnostic_selected(
        manifest, selections, manifest_cases=manifest_cases, collect_only=collect_only
    )
    if schema == 3 and not registry_selected:
        _fail("browserRegistry is limited to pinned executable_path execution")
    if schema in {4, 5} and registry_selected != bool(registry_key):
        _fail("browserRegistry presence differs from pinned executable_path selection")
    secure_http_fixture = _boolean(payload, "secureHttpFixture")
    isolate_cases = _boolean(payload, "isolateCases")
    resume_isolated = _boolean(payload, "resumeIsolated")
    expected_scope = (
        "bootstrap"
        if bootstrap_only
        else f"profile-{profile_name}"
        if profile_name is not None
        else f"manifest-cases-{manifest_api or 'all'}"
        if manifest_cases
        else "selected"
        if selections
        else "full"
    )
    if payload["scope"] != expected_scope:
        _fail("scope does not match the recorded selection")
    if launch_mode == "owned":
        if payload["endpoint"] is not None:
            _fail("owned launch provenance must not contain a CDP endpoint")
    else:
        try:
            validate_loopback_endpoint(_string(payload, "endpoint"))
        except ValueError as error:
            raise ProvenanceValidationError(str(error)) from error
    fixture_bind = _nullable_string(payload, "fixtureBind")
    fixture_host = _nullable_string(payload, "fixtureHost")
    server_prefix = _nullable_string(payload, "serverPrefix")
    if collect_only:
        if fixture_bind is not None or fixture_host is not None:
            _fail("collection provenance must not contain fixture bind or host")
    elif bootstrap_only:
        if fixture_bind is not None or fixture_host is not None or server_prefix is None:
            _fail("bootstrap provenance requires only a server prefix")
    else:
        if requires_fixture_servers:
            if fixture_bind is None or fixture_host is None:
                _fail("execution provenance requires fixture bind and host")
            try:
                validate_tailscale_fixture(fixture_bind, fixture_host)
            except ValueError as error:
                raise ProvenanceValidationError(str(error)) from error
        elif (
            fixture_bind is not None
            or fixture_host is not None
            or server_prefix is not None
            or secure_http_fixture
        ):
            _fail("server-free execution provenance contains fixture configuration")
    if server_prefix is not None:
        try:
            validate_server_prefix(server_prefix)
        except ValueError as error:
            raise ProvenanceValidationError(str(error)) from error
    case_timeout = payload["caseTimeout"]
    if case_timeout is not None and (
        isinstance(case_timeout, bool)
        or not isinstance(case_timeout, (int, float))
        or case_timeout <= 0
    ):
        _fail("caseTimeout must be null or a positive number")
    inventory = _nullable_string(payload, "inventory")
    inventory_digest = _sha(payload, "inventorySha256", nullable=True)
    isolated_count = payload["isolatedCaseCount"]
    isolated_digest = _sha(payload, "isolatedCaseIdsSha256", nullable=True)
    if isinstance(isolated_count, bool) or not isinstance(isolated_count, int):
        _fail("isolatedCaseCount must be an integer")
    case_reports = _nullable_string(payload, "caseReports")
    case_reports_digest = _sha(payload, "caseReportsSha256", nullable=True)
    if isolate_cases:
        if collect_only or bootstrap_only or case_timeout is None:
            _fail("isolated execution requires a timeout and ordinary execution mode")
        if (
            inventory is None
            or inventory_digest is None
            or isolated_count < 1
            or isolated_digest is None
            or case_reports is None
            or case_reports_digest is None
        ):
            _fail("isolated provenance is missing inventory or case evidence")
        _verify_file(Path(inventory), inventory_digest, "inventory")
        _verify_tree(Path(case_reports), case_reports_digest, "case reports")
        if schema in {4, 5}:
            case_root = Path(case_reports)
            receipt = case_root / "launch-mode.json"
            if receipt.is_symlink() or not receipt.is_file():
                _fail("launch-bound isolated case reports require a launch-mode receipt")
            try:
                recorded_mode = json.loads(receipt.read_text())
            except (OSError, ValueError) as error:
                raise ProvenanceValidationError(f"invalid launch-mode receipt: {error}") from error
            if recorded_mode != {"launchMode": launch_mode}:
                _fail("isolated launch-mode receipt differs from provenance")
            case_xml_files = sorted(case_root.glob("*.xml"))
            if len(case_xml_files) != isolated_count:
                _fail("isolated case report count differs from provenance")
            for case_xml in case_xml_files:
                try:
                    case_root_element = ET.parse(case_xml).getroot()
                except ET.ParseError as error:
                    raise ProvenanceValidationError(
                        f"isolated case report is not valid JUnit XML: {error}"
                    ) from error
                _verify_junit_launch_mode(
                    case_root_element,
                    launch_mode,
                    f"isolated case report {case_xml.name}",
                    require=launch_mode == "owned",
                )
        expected_isolated_ids = isolated_cases(Path(inventory), selections, browsers)
        if isolated_count != len(expected_isolated_ids) or isolated_digest != string_list_sha256(
            expected_isolated_ids
        ):
            _fail("isolated case count or digest differs from the inventory selection")
    elif (
        any(
            value is not None
            for value in (
                inventory,
                inventory_digest,
                isolated_digest,
                case_reports,
                case_reports_digest,
            )
        )
        or isolated_count != 0
    ):
        _fail("non-isolated provenance contains isolated-run evidence")
    if resume_isolated and not isolate_cases:
        _fail("resumeIsolated requires isolateCases")

    try:
        persistent_page_argument_browserless = persistent_page_argument_browserless_mode(
            cast(Manifest, manifest),
            manifest_cases=manifest_cases,
            manifest_api=manifest_api,
            browsers=browsers,
            browser_channel=cast("str | None", payload["browserChannel"]),
            launch_mode=launch_mode,
            endpoint=cast("str | None", payload["endpoint"]),
            isolate_cases=isolate_cases,
            collect_only=collect_only,
            case_timeout=cast("float | None", case_timeout),
            server_prefix=server_prefix,
            secure_http_fixture=secure_http_fixture,
        )
        proxy_bad_server_browserless = proxy_bad_server_browserless_mode(
            cast(Manifest, manifest),
            manifest_cases=manifest_cases,
            manifest_api=manifest_api,
            browsers=browsers,
            browser_channel=cast("str | None", payload["browserChannel"]),
            launch_mode=launch_mode,
            endpoint=cast("str | None", payload["endpoint"]),
            isolate_cases=isolate_cases,
            collect_only=collect_only,
            case_timeout=cast("float | None", case_timeout),
            server_prefix=server_prefix,
            secure_http_fixture=secure_http_fixture,
        )
        exact_browserless_preflight = (
            persistent_page_argument_browserless or proxy_bad_server_browserless
        )
        sync_lifecycle_loop = sync_lifecycle_loop_mode(
            cast(Manifest, manifest),
            manifest_cases=manifest_cases,
            manifest_api=manifest_api,
            browsers=browsers,
            browser_channel=cast("str | None", payload["browserChannel"]),
            isolate_cases=isolate_cases,
            collect_only=collect_only,
        )
    except (TypeError, ValueError) as error:
        raise ProvenanceValidationError(str(error)) from error
    if sync_lifecycle_loop:
        if actual_keys != required_keys | _LOOP_KEYS:
            _fail("same-thread asyncio loop requires execution mode and wrapper digest")
        if payload["executionMode"] != SYNC_LIFECYCLE_LOOP_MODE:
            _fail("executionMode differs from pinned same-thread asyncio loop")
        if _sha(payload, "executionWrapperSha256") != SYNC_LIFECYCLE_LOOP_WRAPPER_SHA256:
            _fail("executionWrapperSha256 differs from pinned wrapper source")
    elif persistent_page_argument_browserless:
        if schema != 5:
            _fail("persistent preflight requires schema-5 runtime evidence")
        if actual_keys != required_keys | _PERSISTENT_PREFLIGHT_KEYS:
            _fail("persistent preflight requires its exact execution mode evidence")
        if payload["executionMode"] != PERSISTENT_PAGE_ARGUMENT_MODE:
            _fail("executionMode differs from pinned persistent preflight")
    elif proxy_bad_server_browserless:
        if schema != 5:
            _fail("proxy preflight requires schema-5 runtime evidence")
        if actual_keys != required_keys | _PERSISTENT_PREFLIGHT_KEYS:
            _fail("proxy preflight requires its exact execution mode evidence")
        if payload["executionMode"] != PROXY_BAD_SERVER_MODE:
            _fail("executionMode differs from pinned proxy preflight")
    elif actual_keys & _LOOP_KEYS:
        _fail("execution wrapper evidence is forbidden outside loop execution")

    _string(payload, "scope")
    python_executable = Path(_string(payload, "pythonExecutable"))
    if not python_executable.is_file():
        _fail(f"pythonExecutable is missing: {python_executable}")
    python_version = _string(payload, "pythonVersion")
    observed_python_version = subprocess.run(
        [
            str(python_executable),
            "-c",
            "import platform; print(platform.python_version())",
        ],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    if observed_python_version != python_version:
        _fail(
            "pythonVersion differs from the recorded interpreter: "
            f"expected {python_version}, got {observed_python_version}"
        )
    if profile_manifest is not None:
        try:
            observed_profile = probe_collection_profile_identity(
                str(python_executable),
                profile_manifest,
                tag=cast(str, manifest["tag"]),
                root=actual_manifest.parents[4],
            )
        except (OSError, TypeError, ValueError, subprocess.CalledProcessError) as error:
            raise ProvenanceValidationError(str(error)) from error
        if payload[_PROFILE_KEY] != observed_profile:
            _fail("collectionProfile differs from the pinned runtime")
    playwright_package = Path(_string(payload, "playwrightPackage"))
    playwright_digest = cast(str, _sha(payload, "playwrightPackageSha256"))
    verified_playwright_package = playwright_package
    if harness_root is not None and mode == "candidate":
        recorded_shim = recorded_manifest.parents[4] / (
            "packages/pwright-python/tests/official/shim/playwright"
        )
        if playwright_package.resolve() == recorded_shim.resolve():
            verified_playwright_package = harness_root / "shim/playwright"
            if (
                harness_root.is_symlink()
                or verified_playwright_package.is_symlink()
                or not verified_playwright_package.resolve().is_relative_to(harness_root.resolve())
            ):
                _fail("archived playwright package is a symlink or escapes its root")
    _verify_tree(verified_playwright_package, playwright_digest, "playwright package")
    pwright_package = _nullable_string(payload, "pwrightPackage")
    pwright_digest = _sha(payload, "pwrightPackageSha256", nullable=True)
    pwright_artifact = _nullable_string(payload, "pwrightArtifact")
    artifact_digest = _sha(payload, "pwrightArtifactSha256", nullable=True)
    if mode == "oracle":
        if schema == 5 and payload[_RUNTIME_LIBRARIES_KEY] != []:
            _fail("oracle provenance must not contain Pwright runtime libraries")
        if any(
            value is not None
            for value in (
                pwright_package,
                pwright_digest,
                pwright_artifact,
                artifact_digest,
            )
        ):
            _fail("oracle provenance must not contain Pwright package inputs")
        if exact_browserless_preflight:
            try:
                if persistent_page_argument_browserless:
                    require_persistent_preflight_runtime_sources(mode, verified_playwright_package)
                else:
                    require_proxy_preflight_runtime_sources(mode, verified_playwright_package)
            except ValueError as error:
                raise ProvenanceValidationError(str(error)) from error
    else:
        if (
            pwright_package is None
            or pwright_digest is None
            or pwright_artifact is None
            or artifact_digest is None
        ):
            _fail("candidate provenance must contain Pwright package inputs")
        verified_package = Path(pwright_package)
        verified_artifact = Path(pwright_artifact)
        if package_root is not None:
            recorded_source_package = recorded_manifest.parents[4] / (
                "packages/pwright-python/python/pwright"
            )
            if verified_package != recorded_source_package:
                _fail("archived Pwright package requires the exact recorded source package")
            if verified_artifact.parent != verified_package:
                _fail("recorded Pwright artifact must be inside the recorded package")
            _verify_archived_package_root(package_root)
            verified_package = package_root
            verified_artifact = package_root / verified_artifact.name
        _verify_tree(verified_package, pwright_digest, "pwright package")
        _verify_file(verified_artifact, artifact_digest, "pwright artifact")
        if exact_browserless_preflight:
            try:
                if persistent_page_argument_browserless:
                    require_persistent_preflight_runtime_sources(mode, verified_package)
                else:
                    require_proxy_preflight_runtime_sources(mode, verified_package)
            except ValueError as error:
                raise ProvenanceValidationError(str(error)) from error
        if schema == 5:
            _verify_runtime_libraries(
                payload[_RUNTIME_LIBRARIES_KEY], Path(pwright_package), verified_package
            )

    if registry_key:
        raw_registry = payload[_BROWSER_REGISTRY_KEY]
        if not isinstance(raw_registry, dict):
            _fail("browserRegistry must be an object")
        raw_corpus = manifest.get("corpus")
        default_browsers: object = (
            cast(dict[str, object], raw_corpus).get("defaultBrowsers")
            if isinstance(raw_corpus, dict)
            else None
        )
        registry_engines = browsers
        if not registry_engines:
            if not isinstance(default_browsers, list) or not default_browsers:
                _fail("browserRegistry requires pinned default browsers")
            default_browser_values = cast(list[object], default_browsers)
            if any(
                engine not in {"chromium", "firefox", "webkit"} for engine in default_browser_values
            ):
                _fail("browserRegistry requires pinned default browsers")
            registry_engines = cast(list[str], default_browser_values)
        run_cwd = recorded_manifest.parents[4]
        try:
            browsers_json = probe_official_browsers_json(
                str(python_executable), cwd=run_cwd, env=os.environ.copy()
            )
            expected_registry = browser_registry_snapshot(
                mode=mode,
                runtime={
                    "pythonExecutable": str(python_executable),
                    "pythonVersion": python_version,
                    "playwrightPackage": str(playwright_package),
                    "pwrightPackage": pwright_package,
                    "pwrightArtifact": pwright_artifact,
                },
                cwd=run_cwd,
                env=os.environ.copy(),
                engines=registry_engines,
                browsers_json_path=browsers_json,
            )
        except (OSError, ValueError, subprocess.CalledProcessError) as error:
            raise ProvenanceValidationError(
                f"browserRegistry validation failed: {error}"
            ) from error
        if raw_registry != expected_registry:
            _fail(
                "browserRegistry differs from pinned metadata, environment or executable existence"
            )

    raw_harness_inputs = payload["harnessInputs"]
    expected_harness_count = 2 if mode == "candidate" else 1
    if not isinstance(raw_harness_inputs, list):
        _fail(f"harnessInputs must contain exactly {expected_harness_count} records")
    if len(cast(list[object], raw_harness_inputs)) != expected_harness_count:
        _fail(f"harnessInputs must contain exactly {expected_harness_count} records")
    harness_inputs = cast(list[object], raw_harness_inputs)
    expected_harness_names = (
        ["shim", "harness" if bootstrap_only else "runtime"]
        if mode == "candidate"
        else ["harness" if bootstrap_only else "runtime"]
    )
    for index, expected_name in enumerate(expected_harness_names):
        raw_record = harness_inputs[index]
        if not isinstance(raw_record, dict):
            _fail(f"harnessInputs[{index}] must contain only path and sha256")
        record = cast(dict[object, object], raw_record)
        if set(record) != {"path", "sha256"}:
            _fail(f"harnessInputs[{index}] must contain only path and sha256")
        path_value = record["path"]
        digest_value = record["sha256"]
        if not isinstance(path_value, str) or Path(path_value).name != expected_name:
            _fail(f"harnessInputs[{index}] has the wrong path")
        if not isinstance(digest_value, str) or not _SHA256_PATTERN.fullmatch(digest_value):
            _fail(f"harnessInputs[{index}].sha256 is invalid")
        if exact_browserless_preflight and (
            digest_value != PERSISTENT_PREFLIGHT_HARNESS_SHA256[expected_name]
        ):
            _fail(f"persistent preflight harness digest changed: {expected_name}")
        if harness_root is None:
            harness_path = Path(path_value)
        else:
            resolved_harness_root = harness_root.resolve()
            harness_path = harness_root / expected_name
            if (
                harness_root.is_symlink()
                or harness_path.is_symlink()
                or not harness_path.resolve().is_relative_to(resolved_harness_root)
            ):
                _fail(f"harnessInputs[{index}] archive path is a symlink or escapes its root")
        _verify_tree(harness_path, digest_value, f"harnessInputs[{index}]")

    raw_command = payload["command"]
    if not isinstance(raw_command, list) or not raw_command:
        _fail("command must be a non-empty list of strings")
    command_values = cast(list[object], raw_command)
    if not all(isinstance(part, str) and part for part in command_values):
        _fail("command must be a non-empty list of strings")
    command = cast(list[str], command_values)
    if Path(command[0]).resolve() != python_executable.resolve():
        _fail("command interpreter differs from pythonExecutable")
    expected_command = _expected_command(
        payload,
        manifest,
        recorded_upstream,
        requires_fixture_servers=requires_fixture_servers,
        sync_lifecycle_loop=sync_lifecycle_loop,
        exact_browserless_preflight=exact_browserless_preflight,
    )
    expected_command[0] = command[0]
    if command != expected_command:
        _fail("command does not match the pinned Python selection")
    exit_code = payload["exitCode"]
    if isinstance(exit_code, bool) or not isinstance(exit_code, int):
        _fail("exitCode must be an integer")
    if registry_key and exit_code == 0 and payload["browserChannel"] is None:
        registry = cast(dict[str, object], payload[_BROWSER_REGISTRY_KEY])
        executables = cast(list[dict[str, object]], registry["executables"])
        if any(record["exists"] is False for record in executables):
            _fail("successful executable_path execution cannot have a missing executable")

    result_digest = cast(str, _sha(payload, "resultSha256"))
    recorded_result = Path(_string(payload, "result"))
    actual_result = (result_override or recorded_result).resolve()
    _verify_file(actual_result, result_digest, "result")
    try:
        result_tree = ET.parse(actual_result)
    except ET.ParseError as error:
        raise ProvenanceValidationError(f"result is not valid JUnit XML: {error}") from error
    if schema in {4, 5}:
        _verify_junit_launch_mode(result_tree.getroot(), launch_mode, "result")
    if sync_lifecycle_loop:
        expected_rows = {
            ("tests.sync.test_sync", f"{case_id.rsplit('::', 1)[1]}[{browser}]")
            for case_id in SYNC_LIFECYCLE_CASE_IDS
            for browser in ("chromium", "firefox", "webkit")
        }
        observed_rows = [
            (node.get("classname"), node.get("name")) for node in result_tree.iter("testcase")
        ]
        if len(observed_rows) != len(expected_rows) or set(observed_rows) != expected_rows:
            _fail("same-thread asyncio loop result must contain exactly six pinned rows")
    if persistent_page_argument_browserless:
        rows = list(result_tree.iter("testcase"))
        engine = browsers[0]
        if len(rows) != 1 or (rows[0].get("classname"), rows[0].get("name")) != (
            "tests.async.test_defaultbrowsercontext",
            f"test_should_throw_if_page_argument_is_passed[{engine}]",
        ):
            _fail("persistent preflight result must contain its one pinned row")
        skipped = rows[0].find("skipped") is not None
        if skipped != (engine == "firefox"):
            _fail("persistent preflight skip status differs from pinned browser")
    if result_override is None:
        expected_sidecar = recorded_result.with_suffix(".provenance.json").resolve()
        if provenance_path.resolve() != expected_sidecar:
            _fail("provenance sidecar is not the recorded result's sibling")
    return payload


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("provenance", type=Path)
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--result", type=Path)
    parser.add_argument("--upstream", type=Path)
    parser.add_argument("--tooling-root", type=Path)
    parser.add_argument("--harness-root", type=Path)
    parser.add_argument("--package-root", type=Path)
    args = parser.parse_args()
    try:
        raw = json.loads(args.provenance.read_text())
        payload = verify_provenance(
            raw,
            provenance_path=args.provenance,
            manifest_override=args.manifest,
            result_override=args.result,
            upstream_override=args.upstream,
            tooling_root=args.tooling_root,
            harness_root=args.harness_root,
            package_root=args.package_root,
        )
    except (
        OSError,
        json.JSONDecodeError,
        ProvenanceValidationError,
        subprocess.CalledProcessError,
    ) as error:
        parser.error(str(error))
    print(
        "verified Playwright Python provenance "
        f"mode={payload['mode']} scope={payload['scope']} "
        f"tooling={'source-bound' if cast(int, payload['schema']) >= 2 else 'legacy-unbound'} "
        f"resultSha256={payload['resultSha256']}"
    )


if __name__ == "__main__":
    main()
