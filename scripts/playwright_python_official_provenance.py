#!/usr/bin/env python3
"""Build deterministic provenance for official Playwright Python results."""

from __future__ import annotations

import hashlib
import json
import os
import platform
import subprocess
import sys
from pathlib import Path
from typing import TypedDict, cast

TOOLING_SOURCE_PATHS = (
    "scripts/check_playwright_python_official_source.py",
    "scripts/official_result_destination.py",
    "scripts/playwright_python_official_provenance.py",
    "scripts/run_playwright_python_official_conformance.py",
)
_BROWSERS_JSON_V1_58_0_SHA256 = "6b6e0538d6e46eef71d3e1a0dcb65f8f5ea7914033c8150529d8e207afb6030c"
_BROWSER_REGISTRY_CASE_IDS = frozenset(
    {
        "tests/async/test_launcher.py::test_browser_type_executable_path_should_work",
        "tests/sync/test_launcher.py::test_browser_type_executable_path_should_work",
    }
)
_BROWSER_ENGINES = ("chromium", "firefox", "webkit")
_BROWSER_ENV_KEYS = (
    "PLAYWRIGHT_BROWSERS_PATH",
    "INIT_CWD",
    "PLAYWRIGHT_HOST_PLATFORM_OVERRIDE",
    "HOME",
    "USERPROFILE",
    "XDG_CACHE_HOME",
    "LOCALAPPDATA",
)
_EXECUTABLE_PARTS = {
    "chromium": {
        "mac-arm64": (
            "chrome-mac-arm64",
            "Google Chrome for Testing.app",
            "Contents",
            "MacOS",
            "Google Chrome for Testing",
        ),
        "mac-x64": (
            "chrome-mac-x64",
            "Google Chrome for Testing.app",
            "Contents",
            "MacOS",
            "Google Chrome for Testing",
        ),
        "linux-x64": ("chrome-linux64", "chrome"),
        "linux-arm64": ("chrome-linux", "chrome"),
        "win64": ("chrome-win64", "chrome.exe"),
    },
    "firefox": {
        "mac-arm64": ("firefox", "Nightly.app", "Contents", "MacOS", "firefox"),
        "mac-x64": ("firefox", "Nightly.app", "Contents", "MacOS", "firefox"),
        "linux-x64": ("firefox", "firefox"),
        "linux-arm64": ("firefox", "firefox"),
        "win64": ("firefox", "firefox.exe"),
    },
    "webkit": {
        "mac-arm64": ("pw_run.sh",),
        "mac-x64": ("pw_run.sh",),
        "linux-x64": ("pw_run.sh",),
        "linux-arm64": ("pw_run.sh",),
        "win64": ("Playwright.exe",),
    },
}


class RuntimeIdentity(TypedDict):
    pythonExecutable: str
    pythonVersion: str
    playwrightPackage: str
    pwrightPackage: str | None
    pwrightArtifact: str | None


class CollectionProfileIdentity(TypedDict):
    name: str
    selection: str
    expandedTests: int
    pythonVersion: str
    playwrightVersion: str
    pytestVersion: str
    pytestPlaywrightVersion: str | None
    pytestTimeoutVersion: str | None
    requirements: str | None
    requirementsSha256: str | None


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def pwright_runtime_library_paths(package: Path) -> list[Path]:
    """List the exact adjacent wheel libraries without following archive links."""
    directory = package.parent / "shuozeli_pwright.dylibs"
    if directory.is_symlink():
        raise ValueError(f"Pwright runtime library directory is a symlink: {directory}")
    if not directory.exists():
        return []
    if not directory.is_dir():
        raise ValueError(f"Pwright runtime library directory is not a directory: {directory}")
    resolved_directory = directory.resolve()
    paths = sorted(directory.iterdir(), key=lambda path: path.name)
    for path in paths:
        if (
            path.is_symlink()
            or not path.is_file()
            or path.suffix != ".dylib"
            or not path.resolve().is_relative_to(resolved_directory)
        ):
            raise ValueError(f"Pwright runtime library is not a regular adjacent dylib: {path}")
    return paths


def tooling_sources(root: Path) -> list[dict[str, str]]:
    """Fingerprint the fixed Python capture tooling at one point in time."""
    resolved_root = root.resolve()
    if (root / "scripts").is_symlink():
        raise ValueError("official Python tooling source directory is a symlink")
    records: list[dict[str, str]] = []
    for relative in TOOLING_SOURCE_PATHS:
        path = root / relative
        if (
            path.is_symlink()
            or not path.is_file()
            or not path.resolve().is_relative_to(resolved_root)
        ):
            raise ValueError(f"official Python tooling source is missing or a symlink: {relative}")
        records.append({"path": relative, "sha256": file_sha256(path)})
    return records


def require_tooling_unchanged(root: Path, expected: list[dict[str, str]]) -> None:
    if tooling_sources(root) != expected:
        raise ValueError("official Python tooling sources changed during the run")


def browser_registry_diagnostic_selected(
    manifest: dict[str, object],
    selections: list[str],
    *,
    manifest_cases: bool,
    collect_only: bool,
) -> bool:
    """Scope schema 3 to pinned executable_path execution, never collection."""
    return (
        manifest.get("tag") == "v1.58.0"
        and manifest_cases
        and not collect_only
        and any(selection in _BROWSER_REGISTRY_CASE_IDS for selection in selections)
    )


def probe_official_browsers_json(python: str, *, cwd: Path, env: dict[str, str]) -> Path:
    """Locate the interpreter's installed wheel, never upstream PYTHONPATH metadata."""
    script = f"""
import hashlib
import sysconfig
from importlib.metadata import distributions
from pathlib import Path

sites = sorted({{
    Path(location).resolve()
    for key in ("purelib", "platlib")
    if (location := sysconfig.get_path(key)) is not None
}})
installed_wheels = [
    distribution
    for site in sites
    for distribution in distributions(path=[str(site)])
    if (distribution.metadata.get("Name") or "").casefold() == "playwright"
    and distribution.read_text("WHEEL") is not None
]
if len(installed_wheels) != 1:
    raise RuntimeError("expected exactly one installed Playwright wheel")
installed = installed_wheels[0]
if installed.version != "1.58.0":
    raise RuntimeError("wrong Playwright version for browser registry provenance")
member = Path("playwright/driver/package/browsers.json")
if installed.files is None or member not in installed.files:
    raise RuntimeError("official Playwright wheel does not record browsers.json")
path = Path(installed.locate_file(member)).resolve()
if not path.is_file() or not any(path.is_relative_to(site) for site in sites):
    raise RuntimeError("official Playwright browsers.json escapes the installed wheel")
if hashlib.sha256(path.read_bytes()).hexdigest() != {_BROWSERS_JSON_V1_58_0_SHA256!r}:
    raise RuntimeError("official Playwright browsers.json SHA-256 differs")
print(path)
"""
    completed = subprocess.run(
        [python, "-I", "-c", script],
        cwd=cwd,
        env=env,
        check=True,
        capture_output=True,
        text=True,
    )
    path = Path(completed.stdout.strip())
    if not path.is_absolute() or not path.is_file():
        raise ValueError("official v1.58.0 browsers.json is missing")
    return path


def _short_platform(os_name: str, architecture: str) -> str:
    if os_name == "darwin" and architecture in {"arm64", "aarch64"}:
        return "mac-arm64"
    if os_name == "darwin" and architecture in {"x86_64", "AMD64"}:
        return "mac-x64"
    if os_name == "linux" and architecture in {"aarch64", "arm64"}:
        return "linux-arm64"
    if os_name == "linux" and architecture in {"x86_64", "AMD64"}:
        return "linux-x64"
    if os_name == "win32" and architecture in {"x86_64", "AMD64"}:
        return "win64"
    raise ValueError(f"unsupported browser registry platform: {os_name}/{architecture}")


def _host_platform(short_platform: str, override: str | None, os_release: str) -> str:
    if override:
        return override
    if short_platform.startswith("mac-"):
        return "mac15-arm64" if short_platform == "mac-arm64" else "mac15"
    if short_platform == "win64":
        return "win64"
    architecture = "arm64" if short_platform.endswith("arm64") else "x64"
    values = dict(
        line.split("=", 1)
        for line in os_release.splitlines()
        if "=" in line and line.split("=", 1)[0] in {"ID", "VERSION_ID"}
    )
    distro = values.get("ID", "").strip('"')
    version = values.get("VERSION_ID", "").strip('"')
    major_text = version.split(".")[0]
    major = int(major_text) if major_text.isdigit() else None
    if distro in {"ubuntu", "pop", "neon", "tuxedo"} and major is not None:
        if major < 20:
            return f"ubuntu18.04-{architecture}"
        if major < 22:
            return f"ubuntu20.04-{architecture}"
        if major < 24:
            return f"ubuntu22.04-{architecture}"
        if major < 26:
            return f"ubuntu24.04-{architecture}"
        return f"ubuntu{version}-{architecture}"
    if distro == "linuxmint" and major is not None:
        if major <= 20:
            return f"ubuntu20.04-{architecture}"
        if major == 21:
            return f"ubuntu22.04-{architecture}"
        return f"ubuntu24.04-{architecture}"
    if distro in {"debian", "raspbian"}:
        if version in {"11", "12", "13"}:
            return f"debian{version}-{architecture}"
        if version == "":
            return f"debian13-{architecture}"
    return f"ubuntu24.04-{architecture}"


def _executable_platform(host_platform: str) -> str:
    if host_platform == "<unknown>":
        raise ValueError("browser executable metadata is unavailable for an unknown host platform")
    if host_platform == "win64":
        return "win64"
    if host_platform.startswith("mac"):
        return "mac-arm64" if host_platform.endswith("arm64") else "mac-x64"
    return "linux-arm64" if host_platform.endswith("arm64") else "linux-x64"


def browser_registry_snapshot(
    *,
    mode: str,
    runtime: RuntimeIdentity,
    cwd: Path,
    env: dict[str, str],
    engines: list[str],
    browsers_json_path: Path,
    os_name: str | None = None,
    architecture: str | None = None,
    os_release: str | None = None,
) -> dict[str, object]:
    """Record exact expected paths and stat-only existence for the pinned registry."""
    if mode not in {"candidate", "oracle"}:
        raise ValueError("browser registry mode must be candidate or oracle")
    if not cwd.is_absolute() or not engines or len(set(engines)) != len(engines):
        raise ValueError("browser registry needs an absolute cwd and unique engines")
    if any(engine not in _BROWSER_ENGINES for engine in engines):
        raise ValueError("browser registry contains an unsupported engine")
    browsers_json_path = browsers_json_path.resolve()
    if not browsers_json_path.is_file() or browsers_json_path.stat().st_size > 64 * 1024:
        raise ValueError("official v1.58.0 browsers.json is missing or unexpectedly large")
    digest = file_sha256(browsers_json_path)
    if digest != _BROWSERS_JSON_V1_58_0_SHA256:
        raise ValueError("official v1.58.0 browsers.json SHA-256 differs")
    parsed_metadata: object = json.loads(browsers_json_path.read_text())
    if not isinstance(parsed_metadata, dict):
        raise ValueError("official v1.58.0 browsers.json is malformed")
    raw_metadata = cast(dict[str, object], parsed_metadata)
    raw_browsers = raw_metadata.get("browsers")
    if not isinstance(raw_browsers, list):
        raise ValueError("official v1.58.0 browsers.json is malformed")
    browser_entries = cast(list[object], raw_browsers)
    metadata: dict[str, dict[str, object]] = {}
    for entry in browser_entries:
        if not isinstance(entry, dict):
            continue
        browser = cast(dict[str, object], entry)
        name = browser.get("name")
        if isinstance(name, str):
            metadata[name] = browser
    if (
        len(metadata) != len(browser_entries)
        or any(
            metadata.get(engine, {}).get("revision") != revision
            for engine, revision in (
                ("chromium", "1208"),
                ("firefox", "1509"),
                ("webkit", "2248"),
            )
        )
        or metadata["webkit"].get("revisionOverrides")
        != {
            "debian11-x64": "2105",
            "debian11-arm64": "2105",
            "ubuntu20.04-x64": "2092",
            "ubuntu20.04-arm64": "2092",
        }
    ):
        raise ValueError("official v1.58.0 browser revisions differ")
    cache_platform = _short_platform(os_name or sys.platform, architecture or platform.machine())
    release = (
        os_release
        if os_release is not None
        else (Path("/etc/os-release").read_text() if cache_platform.startswith("linux-") else "")
    )
    host_platform = _host_platform(
        cache_platform, env.get("PLAYWRIGHT_HOST_PLATFORM_OVERRIDE"), release
    )
    short_platform = _executable_platform(host_platform)
    raw_browser_path = env.get("PLAYWRIGHT_BROWSERS_PATH")
    package_root_value = (
        runtime["pwrightPackage"] if mode == "candidate" else str(browsers_json_path.parent)
    )
    if package_root_value is None:
        raise ValueError("candidate browser registry lacks Pwright package root")
    package_root = Path(package_root_value)
    if not package_root.is_absolute():
        raise ValueError("browser registry package root must be absolute")
    if raw_browser_path == "0":
        requested = package_root / ".local-browsers"
    elif raw_browser_path:
        requested = Path(raw_browser_path)
    else:
        home = (
            env.get("USERPROFILE") or env.get("HOME")
            if cache_platform == "win64"
            else env.get("HOME")
        )
        if cache_platform.startswith("mac-"):
            if not home:
                raise ValueError("macOS browser registry requires HOME")
            requested = Path(home) / "Library/Caches/ms-playwright"
        elif cache_platform.startswith("linux-"):
            xdg_cache_home = env.get("XDG_CACHE_HOME")
            if xdg_cache_home:
                cache_base = Path(xdg_cache_home)
            elif home:
                cache_base = Path(home) / ".cache"
            else:
                raise ValueError("Linux browser registry requires XDG_CACHE_HOME or HOME")
            requested = cache_base / "ms-playwright"
        else:
            local_appdata = env.get("LOCALAPPDATA")
            if local_appdata:
                cache_base = Path(local_appdata)
            elif home:
                cache_base = Path(home) / "AppData/Local"
            else:
                raise ValueError("Windows browser registry requires LOCALAPPDATA or home")
            requested = cache_base / "ms-playwright"
    base = Path(env.get("INIT_CWD") or cwd)
    if not base.is_absolute():
        raise ValueError("INIT_CWD must be absolute for browser registry provenance")
    registry_root = Path(
        os.path.normpath(str(requested if requested.is_absolute() else base / requested))
    )
    records: list[dict[str, object]] = []
    for engine in _BROWSER_ENGINES:
        if engine not in engines:
            continue
        browser = metadata[engine]
        raw_overrides = browser.get("revisionOverrides", {})
        default_revision = browser.get("revision")
        if not isinstance(raw_overrides, dict) or not isinstance(default_revision, str):
            raise ValueError("official v1.58.0 browser revisions differ")
        overrides = cast(dict[str, object], raw_overrides)
        revision = overrides.get(host_platform, default_revision)
        if not isinstance(revision, str):
            raise ValueError("official v1.58.0 browser revisions differ")
        directory = f"{engine}-{revision}"
        if engine == "webkit" and host_platform in overrides:
            directory = f"webkit_{host_platform.replace('-', '_')}_special-{revision}"
        executable = registry_root.joinpath(directory, *_EXECUTABLE_PARTS[engine][short_platform])
        records.append(
            {
                "engine": engine,
                "revision": revision,
                "path": str(executable),
                "exists": os.path.exists(executable),
            }
        )
    return {
        "policy": "playwright-v1.58.0-executable-path",
        "resolver": "pwright-python" if mode == "candidate" else "official-node-driver",
        "environment": {key: env.get(key) for key in _BROWSER_ENV_KEYS},
        "workingDirectory": str(cwd),
        "cachePlatform": cache_platform,
        "shortPlatform": short_platform,
        "hostPlatform": host_platform,
        "linuxOsReleaseSha256": hashlib.sha256(release.encode()).hexdigest()
        if cache_platform.startswith("linux-")
        else None,
        "packageRoot": str(package_root),
        "registryRoot": str(registry_root),
        "browsersJsonPath": str(browsers_json_path),
        "browsersJsonSha256": digest,
        "executables": records,
    }


def require_browser_registry_unchanged(
    expected: dict[str, object],
    *,
    mode: str,
    python: str,
    runtime: RuntimeIdentity,
    cwd: Path,
    env: dict[str, str],
    engines: list[str],
    browsers_json_path: Path,
    os_name: str | None = None,
    architecture: str | None = None,
    os_release: str | None = None,
) -> None:
    if probe_official_browsers_json(python, cwd=cwd, env=env) != browsers_json_path:
        raise ValueError("official browser metadata path changed during the run")
    if (
        browser_registry_snapshot(
            mode=mode,
            runtime=runtime,
            cwd=cwd,
            env=env,
            engines=engines,
            browsers_json_path=browsers_json_path,
            os_name=os_name,
            architecture=architecture,
            os_release=os_release,
        )
        != expected
    ):
        raise ValueError("browser registry cache or executable existence changed during the run")


def tree_sha256(root: Path) -> str:
    if not root.is_dir():
        raise ValueError(f"tree digest root is missing: {root}")
    files = [
        path
        for path in root.rglob("*")
        if path.is_file()
        and "__pycache__" not in path.relative_to(root).parts
        and path.suffix != ".pyc"
    ]
    digest = hashlib.sha256()
    for path in sorted(files, key=lambda item: item.relative_to(root).as_posix()):
        relative = path.relative_to(root).as_posix().encode()
        digest.update(len(relative).to_bytes(8, "big"))
        digest.update(relative)
        size = path.stat().st_size
        digest.update(size.to_bytes(8, "big"))
        with path.open("rb") as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(chunk)
    return digest.hexdigest()


def string_list_sha256(values: list[str]) -> str:
    digest = hashlib.sha256()
    for value in values:
        encoded = value.encode()
        digest.update(len(encoded).to_bytes(8, "big"))
        digest.update(encoded)
    return digest.hexdigest()


def probe_runtime_identity(
    python: str,
    mode: str,
    *,
    cwd: Path,
    env: dict[str, str],
) -> RuntimeIdentity:
    script = """
import json
import pathlib
import platform
import sys

import playwright

mode = sys.argv[1]
payload = {
    "pythonExecutable": str(pathlib.Path(sys.executable).absolute()),
    "pythonVersion": platform.python_version(),
    "playwrightPackage": str(pathlib.Path(playwright.__file__).resolve().parent),
    "pwrightPackage": None,
    "pwrightArtifact": None,
}
if mode == "candidate":
    import pwright
    import pwright._native
    payload["pwrightPackage"] = str(pathlib.Path(pwright.__file__).resolve().parent)
    payload["pwrightArtifact"] = str(pathlib.Path(pwright._native.__file__).resolve())
print(json.dumps(payload, sort_keys=True))
"""
    completed = subprocess.run(
        [python, "-c", script, mode],
        cwd=cwd,
        env=env,
        check=True,
        capture_output=True,
        text=True,
    )
    lines = completed.stdout.strip().splitlines()
    if not lines:
        raise ValueError("runtime identity probe produced no JSON")
    parsed = json.loads(lines[-1])
    if not isinstance(parsed, dict):
        raise TypeError("runtime identity probe must produce a JSON object")
    parsed_object = cast(dict[object, object], parsed)
    if not all(isinstance(key, str) for key in parsed_object):
        raise TypeError("runtime identity probe keys must be strings")
    raw = cast(dict[str, object], parsed)
    required = {
        "pythonExecutable",
        "pythonVersion",
        "playwrightPackage",
        "pwrightPackage",
        "pwrightArtifact",
    }
    if set(raw) != required:
        raise ValueError("runtime identity probe fields differ from the schema")
    return cast(RuntimeIdentity, raw)


def probe_collection_profile_identity(
    python: str,
    profile: dict[str, object],
    *,
    tag: str,
    root: Path,
) -> CollectionProfileIdentity:
    """Verify and record the interpreter/dependencies for one collection profile."""

    script = """
import json
import platform
from importlib.metadata import PackageNotFoundError, version

def package_version(name):
    try:
        return version(name)
    except PackageNotFoundError:
        return None

print(json.dumps({
    "pythonVersion": platform.python_version(),
    "playwrightVersion": package_version("playwright"),
    "pytestVersion": package_version("pytest"),
    "pytestPlaywrightVersion": package_version("pytest-playwright"),
    "pytestTimeoutVersion": package_version("pytest-timeout"),
}, sort_keys=True))
"""
    completed = subprocess.run(
        [python, "-c", script],
        check=True,
        capture_output=True,
        text=True,
    )
    observed = cast(dict[str, object], json.loads(completed.stdout.strip()))
    name = profile.get("name")
    selection = profile.get("selection")
    expanded_tests = profile.get("expandedTests")
    if not isinstance(name, str) or not name:
        raise ValueError("collection profile name must be a non-empty string")
    if not isinstance(selection, str) or not selection:
        raise ValueError("collection profile selection must be a non-empty string")
    if (
        isinstance(expanded_tests, bool)
        or not isinstance(expanded_tests, int)
        or expanded_tests < 1
    ):
        raise ValueError("collection profile expandedTests must be a positive integer")
    python_version = observed.get("pythonVersion")
    expected_python = profile.get("pythonVersion")
    if not isinstance(python_version, str) or (
        isinstance(expected_python, str)
        and python_version != expected_python
        and not python_version.startswith(f"{expected_python}.")
    ):
        raise ValueError(f"{name} Python runtime differs from the pinned profile")
    expected_playwright = tag.removeprefix("v")
    if observed.get("playwrightVersion") != expected_playwright:
        raise ValueError(f"{name} Playwright version differs from the pinned tag")
    expected_pytest = profile.get("pytestVersion")
    observed_pytest = observed.get("pytestVersion")
    if not isinstance(observed_pytest, str) or (
        isinstance(expected_pytest, str) and observed_pytest != expected_pytest
    ):
        raise ValueError(f"{name} pytest version differs from the pinned profile")
    expected_plugin = profile.get("pytestPlaywrightVersion")
    if observed.get("pytestPlaywrightVersion") != expected_plugin:
        raise ValueError(f"{name} pytest-playwright version differs from the pinned profile")
    requirements_value = profile.get("requirements")
    requirements_digest = profile.get("requirementsSha256")
    requirements: str | None = None
    if requirements_value is not None or requirements_digest is not None:
        if not isinstance(requirements_value, str) or not isinstance(requirements_digest, str):
            raise ValueError(f"{name} profile requirements path and digest must both be strings")
        requirements_path = (root / requirements_value).resolve()
        if file_sha256(requirements_path) != requirements_digest:
            raise ValueError(f"{name} profile requirements digest differs")
        requirements = str(requirements_path)
    return {
        "name": name,
        "selection": selection,
        "expandedTests": expanded_tests,
        "pythonVersion": python_version,
        "playwrightVersion": expected_playwright,
        "pytestVersion": observed_pytest,
        "pytestPlaywrightVersion": cast("str | None", observed["pytestPlaywrightVersion"]),
        "pytestTimeoutVersion": cast("str | None", observed["pytestTimeoutVersion"]),
        "requirements": requirements,
        "requirementsSha256": (
            cast(str, requirements_digest) if requirements is not None else None
        ),
    }


def build_provenance(
    *,
    mode: str,
    manifest: dict[str, object],
    manifest_path: Path,
    upstream: Path,
    scope: str,
    browsers: list[str],
    browser_channel: str | None,
    selections: list[str],
    manifest_cases: bool,
    manifest_api: str | None,
    bootstrap_only: bool,
    collect_only: bool,
    endpoint: str | None,
    fixture_bind: str | None,
    fixture_host: str | None,
    server_prefix: str | None,
    secure_http_fixture: bool,
    case_timeout: float | None,
    isolate_cases: bool,
    resume_isolated: bool,
    inventory: Path | None,
    isolated_node_ids: list[str],
    command: list[str],
    runtime: RuntimeIdentity,
    harness_inputs: list[Path],
    result: Path,
    exit_code: int,
    collection_profile: CollectionProfileIdentity | None = None,
    tooling_source_records: list[dict[str, str]] | None = None,
    browser_registry: dict[str, object] | None = None,
    launch_mode: str | None = None,
    capture_runtime_libraries: bool = False,
) -> dict[str, object]:
    if browser_registry is not None and tooling_source_records is None:
        raise ValueError("browser registry provenance requires tooling-source binding")
    if launch_mode is None:
        if endpoint is None:
            raise ValueError("legacy launch provenance requires a CDP endpoint")
    else:
        if launch_mode not in {"cdp-adapter", "owned"}:
            raise ValueError("launch mode must be cdp-adapter or owned")
        if tooling_source_records is None:
            raise ValueError("launch-mode provenance requires tooling-source binding")
        if launch_mode == "owned" and endpoint is not None:
            raise ValueError("owned launch provenance must not contain a CDP endpoint")
        if launch_mode == "cdp-adapter" and endpoint is None:
            raise ValueError("CDP-adapter launch provenance requires an endpoint")
    if capture_runtime_libraries and launch_mode is None:
        raise ValueError("runtime-library provenance requires launch-mode binding")
    playwright_package = Path(runtime["playwrightPackage"])
    pwright_package = (
        Path(runtime["pwrightPackage"]) if runtime["pwrightPackage"] is not None else None
    )
    pwright_artifact = (
        Path(runtime["pwrightArtifact"]) if runtime["pwrightArtifact"] is not None else None
    )
    case_reports = result.parent / f"{result.name}.cases"
    provenance: dict[str, object] = {
        "schema": 5
        if capture_runtime_libraries
        else 4
        if launch_mode is not None
        else 3
        if browser_registry is not None
        else 2
        if tooling_source_records is not None
        else 1,
        "mode": mode,
        "repository": manifest["repository"],
        "tag": manifest["tag"],
        "commit": manifest["commit"],
        "manifest": str(manifest_path),
        "manifestSha256": file_sha256(manifest_path),
        "upstream": str(upstream),
        "scope": scope,
        "browsers": browsers,
        "browserChannel": browser_channel,
        "selections": selections,
        "manifestCases": manifest_cases,
        "manifestApi": manifest_api,
        "bootstrapOnly": bootstrap_only,
        "collectOnly": collect_only,
        "endpoint": endpoint,
        "fixtureBind": fixture_bind,
        "fixtureHost": fixture_host,
        "serverPrefix": server_prefix,
        "secureHttpFixture": secure_http_fixture,
        "caseTimeout": case_timeout,
        "isolateCases": isolate_cases,
        "resumeIsolated": resume_isolated,
        "inventory": str(inventory) if inventory is not None else None,
        "inventorySha256": (file_sha256(inventory) if inventory is not None else None),
        "isolatedCaseCount": len(isolated_node_ids),
        "isolatedCaseIdsSha256": (
            string_list_sha256(isolated_node_ids) if isolated_node_ids else None
        ),
        "pythonExecutable": runtime["pythonExecutable"],
        "pythonVersion": runtime["pythonVersion"],
        "playwrightPackage": str(playwright_package),
        "playwrightPackageSha256": tree_sha256(playwright_package),
        "pwrightPackage": str(pwright_package) if pwright_package is not None else None,
        "pwrightPackageSha256": (
            tree_sha256(pwright_package) if pwright_package is not None else None
        ),
        "pwrightArtifact": (str(pwright_artifact) if pwright_artifact is not None else None),
        "pwrightArtifactSha256": (
            file_sha256(pwright_artifact) if pwright_artifact is not None else None
        ),
        "harnessInputs": [
            {"path": str(path), "sha256": tree_sha256(path)} for path in harness_inputs
        ],
        "command": command,
        "exitCode": exit_code,
        "result": str(result),
        "resultSha256": file_sha256(result),
        "caseReports": str(case_reports) if isolate_cases else None,
        "caseReportsSha256": tree_sha256(case_reports) if isolate_cases else None,
    }
    if collection_profile is not None:
        provenance["collectionProfile"] = collection_profile
    if tooling_source_records is not None:
        provenance["toolingSources"] = tooling_source_records
    if browser_registry is not None:
        provenance["browserRegistry"] = browser_registry
    if launch_mode is not None:
        provenance["launchMode"] = launch_mode
    if capture_runtime_libraries:
        if mode == "candidate" and pwright_package is None:
            raise ValueError("candidate runtime-library provenance requires Pwright")
        provenance["pwrightRuntimeLibraries"] = [
            {"path": str(path), "sha256": file_sha256(path)}
            for path in (
                pwright_runtime_library_paths(pwright_package)
                if pwright_package is not None
                else []
            )
        ]
    return provenance
