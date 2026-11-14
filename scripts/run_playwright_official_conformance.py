#!/usr/bin/env python3
"""Run a pinned Playwright core suite through oracle or Pwright candidate."""

from __future__ import annotations

import argparse
import hashlib
import ipaddress
import json
import os
import re
import socket
import ssl
import subprocess
import sys
import tempfile
import threading
import urllib.request
from pathlib import Path
from typing import TypedDict, cast
from urllib.parse import urlsplit, urlunsplit

from official_result_destination import prepare_result_destination


class SuiteRequired(TypedDict):
    id: str
    role: str
    runner: str


class Suite(SuiteRequired, total=False):
    config: str
    args: list[str]
    candidateAdapter: str
    oracleAdapter: str
    project: str
    requiresBrowser: bool
    expectedTestsPerProject: int
    expectedFiles: int
    expectedSpecIds: list[str]
    expectedProjectIds: list[str]
    fixtureMode: str
    upstreamTestSourceSha256: str
    expectedSpecIdsSha256: str
    upstreamFullSpecIdsSha256: str
    excludedSpecIds: list[str]
    crossOriginForward: bool
    pinnedGrep: str
    pinnedGrepInvert: str
    upstreamFixtureSourceSha256: str
    upstreamServerFixturesSourceSha256: str
    sourceOnlyCase: str


class Manifest(TypedDict):
    schema: int
    repository: str
    tag: str
    commit: str
    suites: list[Suite]


_CANDIDATE_PROFILE_PATTERN = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
_SHA256_PATTERN = re.compile(r"^[0-9a-f]{64}$")
_SECURE_FIXTURE_MODE = "trusted-tailscale-tls-dual-port"
_SOURCE_ONLY_FIXTURE_MODE = "pinned-private-source-only-no-io"
_SECURE_TEST_SOURCE = "tests/page/page-dispatchevent.spec.ts"
_SECURE_SERVER_SOURCE = "tests/config/testserver/index.ts"
_SECURE_SERVER_FIXTURES_SOURCE = "tests/config/serverFixtures.ts"
_TOOLING_SOURCE_PATHS = (
    "scripts/official_result_destination.py",
    "scripts/run_playwright_official_conformance.py",
)
_SOURCE_ONLY_GUARD_SOURCE = (
    "tests/official/playwright-core-internal-source-only-guard.cjs"
)
_ROUTE_PUBLIC_CROSS_ORIGIN_SUITE: Suite = {
    "id": "library-page-route-public-chromium",
    "role": "browser-backed-official-file",
    "runner": "packages/playwright/cli.js",
    "config": "tests/library/playwright.config.ts",
    "args": ["tests/page/page-route.spec.ts"],
    "candidateAdapter": "test-mode-playwright-core",
    "oracleAdapter": "test-mode-playwright-core-cdp",
    "project": "chromium-page",
    "requiresBrowser": True,
    "crossOriginForward": True,
    "expectedTestsPerProject": 55,
    "expectedFiles": 1,
    "upstreamTestSourceSha256": (
        "3f543741dc69d44b038024c150b6dbf789a1acc7c2c606594af9d3d442cc023b"
    ),
    "expectedSpecIdsSha256": (
        "38fab03fad14cd6a47a9684295e13ca3b7946595dfe52a3129c1d2afef7b22d3"
    ),
}
_HISTORY_PUBLIC_CROSS_ORIGIN_SUITE: Suite = {
    "id": "library-page-history-public-chromium",
    "role": "browser-backed-official-file",
    "runner": "packages/playwright/cli.js",
    "config": "tests/library/playwright.config.ts",
    "args": ["tests/page/page-history.spec.ts"],
    "candidateAdapter": "test-mode-playwright-core",
    "oracleAdapter": "test-mode-playwright-core-cdp",
    "project": "chromium-page",
    "requiresBrowser": True,
    "crossOriginForward": True,
    "expectedTestsPerProject": 16,
    "expectedFiles": 1,
    "upstreamTestSourceSha256": (
        "2e442a704a5baca8fd98b69b15007c6601d1c859ae343e6d73a7acc0b4109002"
    ),
    "expectedSpecIdsSha256": (
        "632d3bb4dd07f8e8b002cbe9a0ea0a7ac01886164e84e3c10fa7aecd10dbf7e6"
    ),
}
_SET_CONTENT_PUBLIC_SUITE = "library-page-set-content-public-chromium"
_SET_CONTENT_PUBLIC_GREP_INVERT = "should handle timeout properly(?: 2)?$"
_SET_CONTENT_SOURCE_SHA256 = (
    "3909ec532e104dc20176328f3870c1768141b2a270facbaf187169fa73c312c4"
)
_SET_CONTENT_FULL_IDS_SHA256 = (
    "8adebc2af56898da76a85c8b839bcf70d0acd9168f1616ff039e77e9bb3c8d5a"
)
_SET_CONTENT_PUBLIC_IDS_SHA256 = (
    "c29dbc52e8347553b38ba2873b593fa30df47f0b0f60c18e659914be5bf61c9d"
)
_SET_CONTENT_PRIVATE_IDS = (
    "c6caa8e9215c015092b9-9e5f7b7da96772c4c51b",
    "c6caa8e9215c015092b9-be921e910219a009586c",
)
_PAGE_CLICK_PUBLIC_SUITE = "library-page-click-public-chromium"
_PAGE_CLICK_PUBLIC_GREP_INVERT = (
    "(?:should not throw protocol error when navigating during the click|"
    "should retry when navigating during the click|"
    "should not hang when frame is detached|"
    "should not wait with noAutoWaiting(?: [23])?)$"
)
_PAGE_CLICK_SOURCE_SHA256 = (
    "30c7281301b93bf136a37164c16ffa6bc183be9bd64e0ebb359fcc380c7c0eba"
)
_PAGE_CLICK_FULL_IDS_SHA256 = (
    "eb34a5eeb76d81f30a8ec3cf4b869fd93362d85bbc0df2d51f09013845df8424"
)
_PAGE_CLICK_PUBLIC_IDS_SHA256 = (
    "57d177934f72fd878e8b76fb48ebb1392bc47936ec7aba745bc5d7bfc9f6bf4b"
)
_PAGE_CLICK_PRIVATE_HOOK_IDS = (
    "25a1508d0d9129ef6d5d-9f1fcc8080f3a1f19681",
    "25a1508d0d9129ef6d5d-24128e2bb9d111ac9101",
    "25a1508d0d9129ef6d5d-9a949f544d28d63bc462",
    "25a1508d0d9129ef6d5d-5f8bcca5c4d7e509f5ff",
    "25a1508d0d9129ef6d5d-27a6cb74a75e20eedaef",
    "25a1508d0d9129ef6d5d-564ea5fbc7810362acde",
)


def _positive_int(raw: str) -> int:
    value = int(raw)
    if value < 1:
        raise argparse.ArgumentTypeError("must be a positive integer")
    return value


def _git(upstream: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(upstream), *args],
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip()


def _existing_cdp_pages(endpoint: str) -> list[dict[str, object]]:
    parsed = urlsplit(endpoint)
    if parsed.scheme not in {"http", "https", "ws", "wss"}:
        raise SystemExit("CDP endpoint must use http(s) or ws(s)")
    if parsed.hostname not in {"127.0.0.1", "localhost", "::1"}:
        raise SystemExit("official conformance requires a loopback-only CDP endpoint")
    scheme = {"ws": "http", "wss": "https"}.get(parsed.scheme, parsed.scheme)
    target_url = urlunsplit((scheme, parsed.netloc, "/json/list", "", ""))
    try:
        with urllib.request.urlopen(target_url, timeout=5) as response:
            targets = cast(list[dict[str, object]], json.load(response))
    except Exception as error:
        raise SystemExit(
            f"could not inventory dedicated Chrome targets: {error}"
        ) from error
    return [
        target
        for target in targets
        if target.get("type") == "page"
        and target.get("url") not in {"about:blank", "chrome://newtab/"}
    ]


def _tailscale_runtime(env: dict[str, str]) -> tuple[str, str]:
    bind = env.get("TAILSCALE_IP", "").strip()
    if not bind:
        completed = subprocess.run(
            ["tailscale", "ip", "-4"],
            check=True,
            capture_output=True,
            text=True,
        )
        bind = completed.stdout.strip().splitlines()[0]
    address = ipaddress.ip_address(bind)
    if address.version != 4 or address.is_loopback or address.is_unspecified:
        raise SystemExit(
            "official fixture servers require a non-loopback Tailscale IPv4"
        )

    host = env.get("TAILSCALE_HOST", "").strip().rstrip(".")
    if not host:
        completed = subprocess.run(
            ["tailscale", "status", "--json"],
            check=True,
            capture_output=True,
            text=True,
        )
        host = cast(str, json.loads(completed.stdout)["Self"]["DNSName"]).rstrip(".")
    if not host.endswith(".ts.net") or host in {"localhost", bind}:
        raise SystemExit(
            "official fixture servers require a full Tailscale MagicDNS hostname"
        )
    return bind, host


def _resolve_candidate_profile(
    mode: str,
    collect_only: bool,
    requested: str | None,
    environment: dict[str, str],
) -> str | None:
    if mode != "candidate" or collect_only:
        return None
    selected = (requested or environment.get("PWRIGHT_NATIVE_PROFILE", "")).strip()
    if not selected:
        raise ValueError(
            "candidate execution requires --candidate-profile or "
            "PWRIGHT_NATIVE_PROFILE; use 'default' explicitly when intended"
        )
    if not _CANDIDATE_PROFILE_PATTERN.fullmatch(selected):
        raise ValueError("candidate profile must be a lowercase hyphen-separated name")
    return selected


def _file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _tooling_sources(root: Path, *, source_only: bool = False) -> list[dict[str, str]]:
    """Fingerprint the fixed local capture tooling without following source symlinks."""
    resolved_root = root.resolve()
    if (root / "scripts").is_symlink():
        raise ValueError("official Core tooling source directory is a symlink")
    records: list[dict[str, str]] = []
    paths = _TOOLING_SOURCE_PATHS + (
        (_SOURCE_ONLY_GUARD_SOURCE,) if source_only else ()
    )
    for relative in paths:
        path = root / relative
        if (
            path.is_symlink()
            or not path.is_file()
            or not path.resolve().is_relative_to(resolved_root)
        ):
            raise ValueError(
                f"official Core tooling source is missing or a symlink: {relative}"
            )
        records.append({"path": relative, "sha256": _file_sha256(path)})
    return records


def _require_tooling_unchanged(root: Path, expected: list[dict[str, str]]) -> None:
    if (
        _tooling_sources(
            root, source_only=len(expected) == len(_TOOLING_SOURCE_PATHS) + 1
        )
        != expected
    ):
        raise ValueError("official Core tooling sources changed during the run")


def _candidate_source_sha256(candidate: Path) -> str:
    package = candidate / "package.json"
    source = candidate / "src"
    if not package.is_file() or not source.is_dir():
        raise ValueError(
            "candidate source digest requires package.json and a src directory"
        )
    files = [package, *(path for path in source.rglob("*") if path.is_file())]
    digest = hashlib.sha256()
    for path in sorted(files, key=lambda item: item.relative_to(candidate).as_posix()):
        relative = path.relative_to(candidate).as_posix().encode()
        digest.update(len(relative).to_bytes(8, "big"))
        digest.update(relative)
        size = path.stat().st_size
        digest.update(size.to_bytes(8, "big"))
        with path.open("rb") as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(chunk)
    return digest.hexdigest()


def _candidate_artifact_path(candidate: Path, profile: str) -> Path:
    if profile == "default":
        return candidate / "native/pwright.node"
    return candidate / "native" / profile / "pwright.node"


def _secure_fixture_source_digests(suite: Suite, upstream: Path) -> dict[str, str]:
    if (
        suite.get("fixtureMode") != _SECURE_FIXTURE_MODE
        or suite.get("role") != "browser-backed-official-file"
        or suite.get("requiresBrowser") is not True
        or suite.get("project") != "chromium-page"
        or suite.get("args") != [_SECURE_TEST_SOURCE]
        or suite.get("expectedTestsPerProject") != 17
        or isinstance(suite.get("expectedFiles"), bool)
        or suite.get("expectedFiles") != 1
    ):
        raise ValueError(
            "secure Core fixture is limited to the 17 pinned page-dispatchevent cases"
        )
    _secure_spec_ids(suite)
    expected = {
        _SECURE_TEST_SOURCE: suite.get("upstreamTestSourceSha256"),
        _SECURE_SERVER_SOURCE: suite.get("upstreamFixtureSourceSha256"),
        _SECURE_SERVER_FIXTURES_SOURCE: suite.get("upstreamServerFixturesSourceSha256"),
    }
    verified: dict[str, str] = {}
    for relative, digest in expected.items():
        if not isinstance(digest, str) or not _SHA256_PATTERN.fullmatch(digest):
            raise ValueError(
                f"secure Core fixture lacks a pinned SHA-256 for {relative}"
            )
        source = upstream / relative
        if not source.is_file() or _file_sha256(source) != digest:
            raise ValueError(f"secure Core fixture pinned source drifted: {relative}")
        verified[relative] = digest
    return verified


def _probe_trusted_tls(
    bind: str, host: str, certificate: Path, private_key: Path
) -> None:
    server_context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    server_context.load_cert_chain(str(certificate), str(private_key))
    client_context = ssl.create_default_context()
    _probe_trusted_tls_hostname(bind, host, server_context, client_context)


def _probe_trusted_tls_hostname(
    bind: str,
    hostname: str,
    server_context: ssl.SSLContext,
    client_context: ssl.SSLContext,
) -> None:
    if socket.gethostbyname(hostname) != bind:
        raise RuntimeError(
            f"secure Core fixture DNS for {hostname} does not resolve to {bind}"
        )
    errors: list[OSError] = []
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        listener.bind((bind, 0))
        listener.listen(1)
        listener.settimeout(5)
        port = cast(int, listener.getsockname()[1])

        def answer_once() -> None:
            try:
                connection, _ = listener.accept()
                with server_context.wrap_socket(connection, server_side=True) as stream:
                    stream.settimeout(5)
                    stream.recv(4096)
                    stream.sendall(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    )
            except OSError as error:
                errors.append(error)

        server = threading.Thread(target=answer_once, daemon=True)
        server.start()
        try:
            with (
                socket.create_connection((bind, port), timeout=5) as connection,
                client_context.wrap_socket(
                    connection, server_hostname=hostname
                ) as stream,
            ):
                stream.settimeout(5)
                stream.sendall(
                    f"HEAD / HTTP/1.1\r\nHost: {hostname}\r\nConnection: close\r\n\r\n".encode()
                )
                if not stream.recv(4096).startswith(b"HTTP/1.1 200 OK"):
                    raise RuntimeError(
                        f"secure Core fixture TLS probe returned no HTTP 200 for {hostname}"
                    )
        except (OSError, RuntimeError) as error:
            raise RuntimeError(
                f"secure Core fixture TLS is not trusted for {hostname}: {error}"
            ) from error
        finally:
            server.join(timeout=5)
        if errors:
            raise RuntimeError(
                f"secure Core fixture TLS server failed for {hostname}: {errors[0]}"
            )


def _install_secure_certificate(
    bind: str, host: str
) -> tuple[tempfile.TemporaryDirectory[str], Path, Path]:
    directory = tempfile.TemporaryDirectory(prefix="pwright-core-official-tls-")
    certificate = Path(directory.name) / "certificate.pem"
    private_key = Path(directory.name) / "private-key.pem"
    try:
        completed = subprocess.run(
            [
                "tailscale",
                "cert",
                "--cert-file",
                str(certificate),
                "--key-file",
                str(private_key),
                host,
            ],
            check=False,
            capture_output=True,
            text=True,
            timeout=120,
        )
        if completed.returncode:
            detail = (completed.stderr or completed.stdout or "unknown error").strip()
            raise RuntimeError(f"Tailscale certificate issuance failed: {detail}")
        certificate.chmod(0o600)
        private_key.chmod(0o600)
        _probe_trusted_tls(bind, host, certificate, private_key)
    except (OSError, RuntimeError, subprocess.TimeoutExpired):
        directory.cleanup()
        raise
    return directory, certificate, private_key


def _secure_spec_ids(suite: Suite) -> set[str]:
    raw_ids = cast(dict[str, object], suite).get("expectedSpecIds")
    if not isinstance(raw_ids, list):
        raise ValueError("secure Core fixture requires 17 unique pinned spec IDs")
    values = cast(list[object], raw_ids)
    spec_ids: set[str] = set()
    for value in values:
        if not isinstance(value, str) or not value:
            raise ValueError("secure Core fixture requires 17 unique pinned spec IDs")
        spec_ids.add(value)
    if len(values) != 17 or len(spec_ids) != 17:
        raise ValueError("secure Core fixture requires 17 unique pinned spec IDs")
    return spec_ids


def _verify_secure_result_selection(result: Path, project: str, suite: Suite) -> int:
    expected_ids = _secure_spec_ids(suite)
    if project != "chromium-page":
        raise ValueError("secure Core fixture is pinned to project chromium-page")
    report = json.loads(result.read_text())
    suites = (
        cast(dict[str, object], report).get("suites")
        if isinstance(report, dict)
        else None
    )
    if not isinstance(suites, list):
        raise TypeError("secure Core fixture JSON result has no suites")
    reported_ids: list[str] = []

    def visit(raw_suite: object) -> None:
        if not isinstance(raw_suite, dict):
            raise TypeError("secure Core fixture JSON result has a malformed suite")
        report_suite = cast(dict[str, object], raw_suite)
        specs = report_suite.get("specs", [])
        children = report_suite.get("suites", [])
        if not isinstance(specs, list) or not isinstance(children, list):
            raise TypeError("secure Core fixture JSON result has malformed suite lists")
        for raw_spec in cast(list[object], specs):
            if not isinstance(raw_spec, dict):
                raise TypeError("secure Core fixture JSON result has a malformed spec")
            spec = cast(dict[str, object], raw_spec)
            if spec.get("file") != "page/page-dispatchevent.spec.ts":
                raise ValueError(
                    "secure Core fixture result includes an unexpected spec file"
                )
            spec_id = spec.get("id")
            raw_tests = spec.get("tests")
            tests = cast(list[object], raw_tests) if isinstance(raw_tests, list) else []
            if (
                not isinstance(spec_id, str)
                or len(tests) != 1
                or not isinstance(tests[0], dict)
                or cast(dict[str, object], tests[0]).get("projectId") != project
            ):
                raise ValueError(
                    "secure Core fixture must report one pinned chromium-page test per spec"
                )
            reported_ids.append(spec_id)
        for child in cast(list[object], children):
            visit(child)

    for report_suite in cast(list[object], suites):
        visit(report_suite)
    if len(reported_ids) != 17 or set(reported_ids) != expected_ids:
        raise ValueError(
            "secure Core fixture must report exactly 17 unique pinned "
            f"{project} cases; got {len(reported_ids)}"
        )
    return len(reported_ids)


def _pinned_full_file_source(suite: Suite, upstream: Path) -> tuple[str, int, str, str]:
    """Check one opt-in public file's source and exact selection metadata."""

    args = suite.get("args")
    count = suite.get("expectedTestsPerProject")
    project = suite.get("project")
    source_digest = suite.get("upstreamTestSourceSha256")
    ids_digest = suite.get("expectedSpecIdsSha256")
    if (
        suite.get("role") != "browser-backed-official-file"
        or suite.get("requiresBrowser") is not True
        or not isinstance(args, list)
        or len(args) != 1
        or not isinstance(args[0], str)
        or not args[0].startswith("tests/")
        or not args[0].endswith(".spec.ts")
        or ".." in Path(args[0]).parts
        or isinstance(suite.get("expectedFiles"), bool)
        or suite.get("expectedFiles") != 1
        or isinstance(count, bool)
        or not isinstance(count, int)
        or count < 1
        or not isinstance(project, str)
        or not project
        or not isinstance(source_digest, str)
        or not _SHA256_PATTERN.fullmatch(source_digest)
        or not isinstance(ids_digest, str)
        or not _SHA256_PATTERN.fullmatch(ids_digest)
    ):
        raise ValueError(
            "pinned full-file Core suite lacks exact source and selection metadata"
        )
    source = upstream / args[0]
    if (
        source.is_symlink()
        or not source.is_file()
        or not source.resolve().is_relative_to(upstream.resolve())
        or _file_sha256(source) != source_digest
    ):
        raise ValueError(f"pinned full-file Core source drifted: {args[0]}")
    return args[0].removeprefix("tests/"), count, project, ids_digest


def _pinned_grep(suite: Suite) -> str | None:
    """Allow only a manifest-owned one-row filter, never an ad-hoc override."""
    raw = cast(dict[str, object], suite).get("pinnedGrep")
    if raw is None:
        return None
    if (
        not isinstance(raw, str)
        or not raw
        or len(raw) > 200
        or "\n" in raw
        or "\r" in raw
        or suite.get("expectedTestsPerProject") != 1
    ):
        raise ValueError("pinned Core grep requires one exact expected test")
    return raw


def _pinned_grep_invert(suite: Suite) -> str | None:
    """Permit only source-pinned public subsets with fixed private exclusions."""

    raw = cast(dict[str, object], suite).get("pinnedGrepInvert")
    if raw is None:
        return None
    selected = cast(dict[str, object], suite).get("expectedSpecIds")
    excluded = cast(dict[str, object], suite).get("excludedSpecIds")
    if suite.get("id") == _PAGE_CLICK_PUBLIC_SUITE:
        if (
            suite.get("role") != "browser-backed-official-file"
            or suite.get("project") != "chromium-page"
            or suite.get("requiresBrowser") is not True
            or suite.get("runner") != "packages/playwright/cli.js"
            or suite.get("config") != "tests/library/playwright.config.ts"
            or suite.get("candidateAdapter") != "test-mode-playwright-core"
            or suite.get("oracleAdapter") != "test-mode-playwright-core-cdp"
            or suite.get("args") != ["tests/page/page-click.spec.ts"]
            or suite.get("expectedFiles") != 1
            or suite.get("expectedTestsPerProject") != 81
            or suite.get("upstreamTestSourceSha256") != _PAGE_CLICK_SOURCE_SHA256
            or suite.get("expectedSpecIdsSha256") != _PAGE_CLICK_PUBLIC_IDS_SHA256
            or suite.get("upstreamFullSpecIdsSha256") != _PAGE_CLICK_FULL_IDS_SHA256
            or cast(dict[str, object], suite).get("pinnedGrep") is not None
            or raw != _PAGE_CLICK_PUBLIC_GREP_INVERT
            or not isinstance(selected, list)
            or len(selected) != 81
            or not all(isinstance(spec_id, str) and spec_id for spec_id in selected)
            or len(set(selected)) != 81
            or excluded != list(_PAGE_CLICK_PRIVATE_HOOK_IDS)
        ):
            raise ValueError(
                "pinned Core page-click public selection differs from its fixed 81+6 rows"
            )
        public_digest = hashlib.sha256(
            "".join(f"{spec_id}\n" for spec_id in sorted(selected)).encode()
        ).hexdigest()
        full_digest = hashlib.sha256(
            "".join(
                f"{spec_id}\n" for spec_id in sorted([*selected, *excluded])
            ).encode()
        ).hexdigest()
        if (
            public_digest != _PAGE_CLICK_PUBLIC_IDS_SHA256
            or full_digest != _PAGE_CLICK_FULL_IDS_SHA256
        ):
            raise ValueError(
                "pinned Core page-click public IDs differ from its fixed 81+6 rows"
            )
        return _PAGE_CLICK_PUBLIC_GREP_INVERT
    if (
        suite.get("id") != _SET_CONTENT_PUBLIC_SUITE
        or suite.get("role") != "browser-backed-official-file"
        or suite.get("project") != "chromium-page"
        or suite.get("requiresBrowser") is not True
        or suite.get("args") != ["tests/page/page-set-content.spec.ts"]
        or suite.get("expectedFiles") != 1
        or suite.get("expectedTestsPerProject") != 15
        or suite.get("upstreamTestSourceSha256") != _SET_CONTENT_SOURCE_SHA256
        or suite.get("expectedSpecIdsSha256") != _SET_CONTENT_PUBLIC_IDS_SHA256
        or suite.get("upstreamFullSpecIdsSha256") != _SET_CONTENT_FULL_IDS_SHA256
        or cast(dict[str, object], suite).get("pinnedGrep") is not None
        or raw != _SET_CONTENT_PUBLIC_GREP_INVERT
        or not isinstance(selected, list)
        or len(selected) != 15
        or not all(isinstance(spec_id, str) and spec_id for spec_id in selected)
        or len(set(selected)) != 15
        or excluded != list(_SET_CONTENT_PRIVATE_IDS)
    ):
        raise ValueError(
            "pinned Core set-content public selection differs from its fixed 15+2 rows"
        )
    public_digest = hashlib.sha256(
        "".join(f"{spec_id}\n" for spec_id in sorted(selected)).encode()
    ).hexdigest()
    full_digest = hashlib.sha256(
        "".join(f"{spec_id}\n" for spec_id in sorted([*selected, *excluded])).encode()
    ).hexdigest()
    if (
        public_digest != _SET_CONTENT_PUBLIC_IDS_SHA256
        or full_digest != _SET_CONTENT_FULL_IDS_SHA256
    ):
        raise ValueError(
            "pinned Core set-content public IDs differ from its fixed 15+2 rows"
        )
    return raw


def _verify_pinned_full_file_result_selection(
    result: Path, suite: Suite, upstream: Path
) -> int:
    expected_file, expected_count, expected_project, expected_ids_digest = (
        _pinned_full_file_source(suite, upstream)
    )
    report = json.loads(result.read_text())
    suites = (
        cast(dict[str, object], report).get("suites")
        if isinstance(report, dict)
        else None
    )
    if not isinstance(suites, list):
        raise TypeError("pinned full-file Core JSON result has no suites")
    reported_ids: list[str] = []

    def visit(raw_suite: object) -> None:
        if not isinstance(raw_suite, dict):
            raise TypeError("pinned full-file Core JSON result has a malformed suite")
        specs = raw_suite.get("specs", [])
        children = raw_suite.get("suites", [])
        if not isinstance(specs, list) or not isinstance(children, list):
            raise TypeError(
                "pinned full-file Core JSON result has malformed suite lists"
            )
        for raw_spec in specs:
            if not isinstance(raw_spec, dict):
                raise TypeError(
                    "pinned full-file Core JSON result has a malformed spec"
                )
            spec_id = raw_spec.get("id")
            tests = raw_spec.get("tests")
            if (
                raw_spec.get("file") != expected_file
                or not isinstance(spec_id, str)
                or not spec_id
                or not isinstance(tests, list)
                or len(tests) != 1
                or not isinstance(tests[0], dict)
                or tests[0].get("projectId") != expected_project
            ):
                raise ValueError(
                    "pinned full-file Core result has a wrong file, ID or project"
                )
            test = tests[0]
            attempts = test.get("results")
            if (
                test.get("status") not in {"expected", "unexpected", "flaky", "skipped"}
                or not isinstance(attempts, list)
                or not attempts
                or any(
                    not isinstance(attempt, dict)
                    or attempt.get("status")
                    not in {"passed", "failed", "timedOut", "skipped", "interrupted"}
                    for attempt in attempts
                )
            ):
                raise ValueError(
                    "pinned full-file Core result requires an executed attempt for every case"
                )
            reported_ids.append(spec_id)
        for child in children:
            visit(child)

    for report_suite in suites:
        visit(report_suite)
    digest = hashlib.sha256(
        "".join(f"{spec_id}\n" for spec_id in sorted(reported_ids)).encode()
    ).hexdigest()
    if (
        len(reported_ids) != expected_count
        or len(set(reported_ids)) != expected_count
        or digest != expected_ids_digest
    ):
        raise ValueError(
            f"pinned full-file Core result expected {expected_count} unique "
            f"{expected_project} IDs; got {len(reported_ids)} (SHA-256 {digest})"
        )
    return len(reported_ids)


def _cross_origin_forward_enabled(suite: Suite) -> bool:
    """Limit the different-origin HTTP fixture to exact pinned file shards."""

    value = suite.get("crossOriginForward")
    if suite.get("id") == _HISTORY_PUBLIC_CROSS_ORIGIN_SUITE["id"]:
        if suite != _HISTORY_PUBLIC_CROSS_ORIGIN_SUITE:
            raise ValueError(
                "cross-origin forward requires the exact pinned page-history suite"
            )
        return True
    if value is None:
        return False
    if suite.get("id") == _ROUTE_PUBLIC_CROSS_ORIGIN_SUITE["id"]:
        if suite != _ROUTE_PUBLIC_CROSS_ORIGIN_SUITE:
            raise ValueError(
                "cross-origin forward is limited to the exact pinned page-route public shard"
            )
        return True
    if (
        value is not True
        or suite.get("id") != "library-page-goto-chromium"
        or suite.get("role") != "browser-backed-official-file"
        or suite.get("requiresBrowser") is not True
        or suite.get("args") != ["tests/page/page-goto.spec.ts"]
        or suite.get("project") != "chromium-page"
        or suite.get("expectedTestsPerProject") != 64
    ):
        raise ValueError(
            "cross-origin forward is limited to the pinned page-goto shard"
        )
    return True


def _verify_source_only_suite(
    suite: Suite, upstream: Path, manifest_path: Path, root: Path
) -> dict[str, object]:
    if suite.get("fixtureMode") != _SOURCE_ONLY_FIXTURE_MODE:
        raise ValueError("source-only guard requires its distinct fixture mode")
    guard = root / _SOURCE_ONLY_GUARD_SOURCE
    if not guard.is_file() or guard.is_symlink():
        raise ValueError("source-only guard source is missing or a symlink")
    completed = subprocess.run(
        ["node", str(guard), "--check", str(manifest_path), suite["id"], str(upstream)],
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode:
        raise ValueError(
            "source-only AST guard rejected the pinned selection: "
            + (completed.stderr.strip() or completed.stdout.strip())[-1000:]
        )
    try:
        source = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise ValueError("source-only AST guard emitted invalid JSON") from error
    legacy_fields = {
        "case", "sourceFile", "sourceLine", "sourceSha256",
        "expectedSpecIds", "expectedProjectIds",
    }
    batch_fields = {
        "case", "sourceFiles", "sourceSha256", "expectedInvocationIdsSha256",
        "expectedProjectIds", "expectedTestsPerProject",
    }
    if not isinstance(source, dict) or set(source) not in (legacy_fields, batch_fields):
        raise ValueError("source-only AST guard emitted an incomplete source record")
    if (set(source) == batch_fields) != (
        source.get("case") == "private-internal-batch-v1"
    ):
        raise ValueError("source-only AST guard emitted a mismatched batch record")
    return cast(dict[str, object], source)


def _verify_source_only_batch_result_selection(
    result: Path, suite: Suite, source: dict[str, object]
) -> int:
    report = json.loads(result.read_text())
    if (
        not isinstance(report, dict)
        or not isinstance(report.get("suites"), list)
        or report.get("errors")
    ):
        raise ValueError("source-only batch result lacks suites or has runner errors")
    files = source["sourceFiles"]
    projects = source["expectedProjectIds"]
    per_project = source["expectedTestsPerProject"]
    if (
        not isinstance(files, list)
        or len(files) != 26
        or not all(isinstance(value, str) for value in files)
        or not isinstance(projects, list)
        or len(projects) not in (3, 6)
        or not all(isinstance(value, str) for value in projects)
        or per_project != 284
    ):
        raise ValueError("source-only batch source record is malformed")
    expected_files = {value.removeprefix("tests/") for value in files}
    found_files: set[str] = set()
    found: list[tuple[str, str]] = []

    def visit(raw_suite: object) -> None:
        if not isinstance(raw_suite, dict):
            raise ValueError("source-only batch result contains a malformed suite")
        specs = raw_suite.get("specs", [])
        children = raw_suite.get("suites", [])
        if not isinstance(specs, list) or not isinstance(children, list):
            raise ValueError("source-only batch result contains malformed suite lists")
        for raw_spec in specs:
            if not isinstance(raw_spec, dict):
                raise ValueError("source-only batch result contains a malformed spec")
            file = raw_spec.get("file")
            spec_id = raw_spec.get("id")
            tests = raw_spec.get("tests")
            if (
                not isinstance(file, str)
                or file not in expected_files
                or not isinstance(spec_id, str)
                or not re.fullmatch(r"[a-f0-9]{20}-[a-f0-9]{20}", spec_id)
                or not isinstance(tests, list)
                or len(tests) != 1
            ):
                raise ValueError("source-only batch result contains an unpinned spec")
            found_files.add(file)
            test = tests[0]
            if not isinstance(test, dict):
                raise ValueError("source-only batch result contains a malformed test")
            project_id = test.get("projectId")
            results = test.get("results")
            if (
                project_id not in projects
                or test.get("expectedStatus") != "passed"
                or test.get("status") not in {"expected", "unexpected", "flaky", "skipped"}
                or not isinstance(results, list)
                or not results
                or not all(isinstance(item, dict) and item.get("status") in {
                    "passed", "failed", "timedOut", "interrupted", "skipped"
                } for item in results)
            ):
                raise ValueError("source-only batch result has a missing or malformed attempt")
            found.append((spec_id, project_id))
        for child in children:
            visit(child)

    for raw_suite in report["suites"]:
        visit(raw_suite)
    if found_files != expected_files or len(found) != len(projects) * per_project:
        raise ValueError("source-only batch result changed its file or invocation count")
    if len(set(found)) != len(found):
        raise ValueError("source-only batch result duplicated an invocation")
    source_suite = (
        "library" if suite["config"] == "tests/library/playwright.config.ts" else "bidi"
    )
    digest = hashlib.sha256(
        "".join(
            f"{source_suite}\t{spec_id}\t{project_id}\n"
            for spec_id, project_id in sorted(found)
        ).encode()
    ).hexdigest()
    if digest != source["expectedInvocationIdsSha256"]:
        raise ValueError("source-only batch result changed exact invocation IDs")
    return len(found)


def _verify_source_only_result_selection(
    result: Path, suite: Suite, source: dict[str, object]
) -> int:
    if source.get("case") == "private-internal-batch-v1":
        return _verify_source_only_batch_result_selection(result, suite, source)
    report = json.loads(result.read_text())
    if not isinstance(report, dict) or not isinstance(report.get("suites"), list):
        raise ValueError("source-only result lacks suites")
    expected = source["expectedSpecIds"]
    if not isinstance(expected, list) or not all(
        isinstance(value, str) for value in expected
    ):
        raise ValueError("source-only expected spec IDs are malformed")
    expected_projects = source["expectedProjectIds"]
    if (
        not isinstance(expected_projects, list)
        or len(expected_projects) != len(expected)
        or not all(isinstance(value, str) for value in expected_projects)
    ):
        raise ValueError("source-only expected project IDs are malformed")
    expected_pairs = set(zip(expected, expected_projects, strict=True))
    found: list[tuple[str, str]] = []
    expected_file = cast(str, source["sourceFile"]).removeprefix("tests/")

    def visit(raw_suite: object) -> None:
        if not isinstance(raw_suite, dict):
            raise ValueError("source-only result contains a malformed suite")
        specs = raw_suite.get("specs", [])
        children = raw_suite.get("suites", [])
        if not isinstance(specs, list) or not isinstance(children, list):
            raise ValueError("source-only result contains malformed suite lists")
        for raw_spec in specs:
            if not isinstance(raw_spec, dict) or raw_spec.get("file") != expected_file:
                raise ValueError("source-only result contains a different spec file")
            tests = raw_spec.get("tests")
            if (
                not isinstance(tests, list)
                or len(tests) != 1
                or not isinstance(tests[0], dict)
            ):
                raise ValueError("source-only result must have one test per spec")
            spec_id = raw_spec.get("id")
            project_id = tests[0].get("projectId")
            if (
                not isinstance(spec_id, str)
                or not isinstance(project_id, str)
                or (spec_id, project_id) not in expected_pairs
            ):
                raise ValueError(
                    "source-only result contains an unpinned spec or project"
                )
            results = tests[0].get("results")
            if (
                tests[0].get("status") != "expected"
                or not isinstance(results, list)
                or len(results) != 1
                or not isinstance(results[0], dict)
                or results[0].get("status") != "passed"
            ):
                raise ValueError(
                    "source-only result has a skipped, failed, or retried test"
                )
            found.append((spec_id, project_id))
        for child in children:
            visit(child)

    for raw_suite in report["suites"]:
        visit(raw_suite)
    if len(found) != len(expected) or set(found) != expected_pairs:
        raise ValueError(
            f"source-only result changed the exact selection: {len(found)} != {len(expected)}"
        )
    return len(found)


def _verify_source_only_guard_receipt(
    receipt: Path, suite: Suite, source: dict[str, object]
) -> int:
    if not receipt.is_file():
        raise ValueError("source-only worker guard produced no receipt")
    lines = receipt.read_text().splitlines()
    if len(lines) < 2:
        raise ValueError("source-only worker guard never activated")
    thread_keys: set[tuple[int, int]] = set()
    kinds: list[str] = []
    for line in lines:
        try:
            record = json.loads(line)
        except json.JSONDecodeError as error:
            raise ValueError("source-only worker guard receipt is not JSONL") from error
        if (
            not isinstance(record, dict)
            or set(record) != {"kind", "pid", "threadId", "suite", "case"}
            or record["kind"]
            not in {"runner-no-io", "worker-no-io", "loader-thread-no-io"}
            or record["suite"] != suite["id"]
            or record["case"] != source["case"]
            or isinstance(record["pid"], bool)
            or not isinstance(record["pid"], int)
            or record["pid"] < 1
            or isinstance(record["threadId"], bool)
            or not isinstance(record["threadId"], int)
            or record["threadId"] < 0
            or (record["pid"], record["threadId"]) in thread_keys
            or (record["kind"] == "loader-thread-no-io") != (record["threadId"] > 0)
        ):
            raise ValueError("source-only worker guard receipt is invalid")
        thread_keys.add((record["pid"], record["threadId"]))
        kinds.append(record["kind"])
    if kinds.count("runner-no-io") != 1 or kinds.count("worker-no-io") < 1:
        raise ValueError("source-only guard must cover runner and worker processes")
    return kinds.count("worker-no-io")


def _result_provenance(
    *,
    mode: str,
    manifest: Manifest,
    manifest_path: Path,
    suite: Suite,
    upstream: Path,
    project: str | None,
    filters: list[str],
    grep: str | None,
    grep_invert: str | None = None,
    workers: int,
    headed: bool,
    endpoint: str,
    fixture_bind: str,
    fixture_host: str,
    candidate: Path | None,
    candidate_profile: str | None,
    harness_inputs: list[Path],
    command: list[str],
    result: Path,
    exit_code: int,
    secure_fixture_sources: dict[str, str] | None = None,
    tooling_sources: list[dict[str, str]] | None = None,
    source_only_execution: dict[str, object] | None = None,
) -> dict[str, object]:
    adapter_key = "candidateAdapter" if mode == "candidate" else "oracleAdapter"
    candidate_artifact = (
        _candidate_artifact_path(candidate, candidate_profile)
        if candidate is not None and candidate_profile is not None
        else None
    )
    provenance: dict[str, object] = {
        "schema": 3
        if source_only_execution is not None
        else (2 if tooling_sources is not None else 1),
        "mode": mode,
        "repository": manifest["repository"],
        "tag": manifest["tag"],
        "commit": manifest["commit"],
        "manifest": str(manifest_path),
        "manifestSha256": _file_sha256(manifest_path),
        "upstream": str(upstream),
        "suite": suite["id"],
        "suiteRole": suite["role"],
        "adapter": suite.get(adapter_key),
        "project": project,
        "filters": filters,
        "grep": grep,
        "workers": workers,
        "headed": headed,
        "endpoint": endpoint,
        "fixtureBind": fixture_bind,
        "fixtureHost": fixture_host,
        "candidate": str(candidate) if candidate is not None else None,
        "candidateProfile": candidate_profile,
        "candidateSourceSha256": (
            _candidate_source_sha256(candidate) if candidate is not None else None
        ),
        "candidateArtifact": (
            str(candidate_artifact) if candidate_artifact is not None else None
        ),
        "candidateArtifactSha256": (
            _file_sha256(candidate_artifact)
            if candidate_artifact is not None and candidate_artifact.is_file()
            else None
        ),
        "harnessInputs": [
            {"path": str(path), "sha256": _file_sha256(path)} for path in harness_inputs
        ],
        "command": command,
        "exitCode": exit_code,
        "result": str(result),
        "resultSha256": _file_sha256(result),
    }
    if secure_fixture_sources is not None:
        provenance["secureFixture"] = {
            "mode": _SECURE_FIXTURE_MODE,
            "fixtureHost": fixture_host,
            "ordinaryScheme": "https",
            "crossProcessScheme": "https",
            "crossProcessPortOffset": 1,
            "isolationGuarantee": "distinct-origin-same-site-possible",
            "reportedTests": 17,
            "upstreamSources": secure_fixture_sources,
            "trustedTlsPreflight": True,
        }
    if tooling_sources is not None:
        provenance["toolingSources"] = tooling_sources
    if source_only_execution is not None:
        provenance["sourceOnlyExecution"] = source_only_execution
    if grep_invert is not None:
        provenance["grepInvert"] = grep_invert
    return provenance


def main() -> None:
    root = Path(__file__).parents[1]
    try:
        tooling_sources = _tooling_sources(root)
    except ValueError as error:
        raise SystemExit(str(error)) from error
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=("oracle", "candidate"))
    parser.add_argument("upstream", type=Path)
    parser.add_argument("--suite", default="library")
    parser.add_argument("--project")
    parser.add_argument("--test", action="append", default=[])
    parser.add_argument("--grep")
    parser.add_argument("--workers", type=_positive_int, default=1)
    parser.add_argument("--collect-only", action="store_true")
    parser.add_argument("--headed", action="store_true")
    parser.add_argument("--endpoint", default="http://127.0.0.1:9222")
    parser.add_argument("--candidate", type=Path, default=root / "packages/pwright")
    parser.add_argument("--candidate-profile")
    parser.add_argument(
        "--secure-http-fixture",
        action="store_true",
        help="opt into the manifest-pinned trusted dual-port Tailscale TLS dispatch-event fixture",
    )
    parser.add_argument("--result", type=Path)
    parser.add_argument(
        "--manifest",
        type=Path,
        default=root / "tests/official/playwright-core-manifest.json",
    )
    args = parser.parse_args()

    result_path: Path | None = None
    if not args.collect_only:
        result_path = prepare_result_destination(
            args.result
            if args.result is not None
            else root
            / ".work/official-conformance/results"
            / f"playwright-core-{args.mode}-{args.suite}.json"
        )

    upstream = args.upstream.resolve()
    manifest_path = args.manifest.resolve()
    manifest = cast(Manifest, json.loads(manifest_path.read_text()))
    if manifest["schema"] != 1:
        raise SystemExit(f"unsupported manifest schema: {manifest['schema']}")
    if _git(upstream, "rev-parse", "HEAD") != manifest["commit"]:
        raise SystemExit("Playwright core checkout does not match the pinned commit")
    if _git(upstream, "status", "--porcelain", "--untracked-files=no"):
        raise SystemExit("Playwright core checkout has tracked source changes")

    suites = {suite["id"]: suite for suite in manifest["suites"]}
    if args.suite not in suites:
        parser.error(f"unknown suite {args.suite!r}; choose from {sorted(suites)}")
    suite = suites[args.suite]
    secure_fixture = suite.get("fixtureMode") == _SECURE_FIXTURE_MODE
    source_only = suite.get("fixtureMode") == _SOURCE_ONLY_FIXTURE_MODE
    if secure_fixture != args.secure_http_fixture:
        parser.error(
            "secure Core fixture requires both its dedicated manifest and "
            "--secure-http-fixture; historical HTTP manifests cannot enable it"
        )
    if suite.get("fixtureMode") not in (
        None,
        _SECURE_FIXTURE_MODE,
        _SOURCE_ONLY_FIXTURE_MODE,
    ):
        parser.error("unknown official Core fixture mode")
    source_only_source: dict[str, object] | None = None
    if source_only:
        if (
            args.project
            or args.test
            or args.grep
            or args.headed
            or args.workers != 1
            or suite.get("requiresBrowser") is not False
        ):
            parser.error(
                "source-only Core suite forbids project, filter, headed, worker, and browser overrides"
            )
        for key in (
            "NODE_OPTIONS",
            "PWTEST_MODE",
            "PWPAGE_IMPL",
            "PWTEST_USE_BIDI_EXPECTATIONS",
            "PWRIGHT_CORE_SOURCE_ONLY_TEST_WORKER",
        ):
            if os.environ.get(key):
                parser.error(f"source-only Core suite forbids inherited {key}")
        try:
            source_only_source = _verify_source_only_suite(
                suite, upstream, manifest_path, root
            )
            tooling_sources = _tooling_sources(root, source_only=True)
        except ValueError as error:
            parser.error(str(error))
    secure_fixture_sources: dict[str, str] | None = None
    if secure_fixture:
        if args.test or args.grep:
            parser.error("secure Core fixture forbids ad-hoc --test and --grep filters")
        try:
            secure_fixture_sources = _secure_fixture_source_digests(suite, upstream)
        except ValueError as error:
            parser.error(str(error))
    pinned_full_file = (
        not secure_fixture
        and not source_only
        and suite.get("upstreamTestSourceSha256") is not None
    )
    if not pinned_full_file and (
        cast(dict[str, object], suite).get("pinnedGrep") is not None
        or cast(dict[str, object], suite).get("pinnedGrepInvert") is not None
    ):
        parser.error("pinned Core grep requires a source-bound file selection")
    if pinned_full_file:
        if args.test or args.grep:
            parser.error(
                "pinned full-file Core suite forbids ad-hoc --test and --grep filters"
            )
        try:
            _pinned_full_file_source(suite, upstream)
            pinned_grep = _pinned_grep(suite)
            pinned_grep_invert = _pinned_grep_invert(suite)
        except ValueError as error:
            parser.error(str(error))
    else:
        pinned_grep = None
        pinned_grep_invert = None
    try:
        cross_origin_forward = _cross_origin_forward_enabled(suite)
    except ValueError as error:
        parser.error(str(error))
    if cross_origin_forward and not pinned_full_file:
        parser.error(
            "cross-origin forward requires an exact pinned full-file selection"
        )
    fixed_project = suite.get("project")
    if args.project and fixed_project and args.project != fixed_project:
        parser.error(
            f"suite {args.suite!r} is fixed to project {fixed_project!r}; "
            f"cannot override it with {args.project!r}"
        )
    project = fixed_project or args.project
    requires_browser = suite.get("requiresBrowser", True)
    if not requires_browser and args.test:
        parser.error(
            "browser-independent suite filters are fixed by its manifest; "
            "--test overrides are forbidden"
        )
    if args.mode == "candidate" and args.collect_only:
        parser.error(
            "candidate --collect-only cannot prove that the runtime candidate was loaded; "
            "use the inventory script for source collection"
        )
    if args.mode == "candidate" and not suite.get("candidateAdapter"):
        parser.error(
            f"suite {args.suite!r} has no candidate package adapter yet; "
            "refusing an official-implementation false green"
        )
    try:
        candidate_profile = _resolve_candidate_profile(
            args.mode,
            args.collect_only,
            args.candidate_profile,
            os.environ,
        )
    except ValueError as error:
        parser.error(str(error))
    if (
        args.mode == "oracle"
        and not args.collect_only
        and requires_browser
        and not suite.get("oracleAdapter")
    ):
        parser.error(
            f"suite {args.suite!r} has no dedicated-CDP oracle adapter yet; "
            "refusing to launch an unmanaged browser"
        )
    if not args.collect_only and requires_browser:
        existing_pages = _existing_cdp_pages(args.endpoint)
        if existing_pages:
            descriptions = ", ".join(
                f"{page.get('title', '')!s} ({page.get('url', '')!s})"
                for page in existing_pages[:4]
            )
            raise SystemExit(
                "refusing official conformance against dedicated Chrome with existing "
                f"nonblank pages: {descriptions}"
            )
    runner = upstream / suite["runner"]
    if not runner.is_file():
        raise SystemExit(f"suite runner is missing: {runner}")

    command = ["node", str(runner), "test"]
    if config := suite.get("config"):
        command.append(f"--config={config}")
    if args.collect_only:
        command.extend(["--list", "--reporter=line"])
    else:
        command.extend(["--reporter=line,json", f"--workers={args.workers}"])
    if project:
        command.append(f"--project={project}")
    if args.headed:
        command.append("--headed")
    if args.grep or pinned_grep:
        command.append(f"--grep={args.grep or pinned_grep}")
    if pinned_grep_invert:
        command.append(f"--grep-invert={pinned_grep_invert}")
    command.extend(args.test or suite.get("args", []))

    env = os.environ.copy()
    env.pop("PWRIGHT_OFFICIAL_CROSS_ORIGIN_FORWARD", None)
    if cross_origin_forward and not args.collect_only:
        env["PWRIGHT_OFFICIAL_CROSS_ORIGIN_FORWARD"] = "1"
    if result_path is not None:
        result_path.parent.mkdir(parents=True, exist_ok=True)
        env["PLAYWRIGHT_JSON_OUTPUT_FILE"] = str(result_path)

    fixture_bind = ""
    fixture_host = ""
    harness_inputs: list[Path] = []
    secure_certificate_directory: tempfile.TemporaryDirectory[str] | None = None
    if not args.collect_only:
        if not source_only:
            fixture_bind, fixture_host = _tailscale_runtime(env)
            env["PWRIGHT_OFFICIAL_SERVER_IP"] = fixture_bind
            env["PWRIGHT_OFFICIAL_SERVER_HOST"] = fixture_host
        env["PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM"] = str(upstream)
        if not source_only:
            common_preload = (
                root / "tests/official/playwright-core-server-runtime.cjs"
            ).resolve()
            harness_inputs.append(common_preload)
        existing_node_options = env.get("NODE_OPTIONS", "").strip()
        if not source_only:
            require_option = f"--require={common_preload}"
            env["NODE_OPTIONS"] = " ".join(
                part for part in (existing_node_options, require_option) if part
            )
        if secure_fixture:
            try:
                secure_certificate_directory, certificate, private_key = (
                    _install_secure_certificate(fixture_bind, fixture_host)
                )
            except (OSError, RuntimeError, subprocess.TimeoutExpired) as error:
                raise SystemExit(f"secure Core fixture unavailable: {error}") from error
            env["PWRIGHT_OFFICIAL_SECURE_TLS_CERT"] = str(certificate)
            env["PWRIGHT_OFFICIAL_SECURE_TLS_KEY"] = str(private_key)
            secure_preload = (
                root / "tests/official/playwright-core-secure-server-runtime.cjs"
            ).resolve()
            harness_inputs.append(secure_preload)
            existing_node_options = env.get("NODE_OPTIONS", "").strip()
            env["NODE_OPTIONS"] = f"{existing_node_options} --require={secure_preload}"

    guard_receipt: Path | None = None
    if source_only:
        guard = (root / _SOURCE_ONLY_GUARD_SOURCE).resolve()
        harness_inputs.insert(0, guard)
        env["PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM"] = str(upstream)
        env["PWRIGHT_CORE_SOURCE_ONLY_ACTIVE"] = "1"
        env["PWRIGHT_CORE_SOURCE_ONLY_MANIFEST"] = str(manifest_path)
        env["PWRIGHT_CORE_SOURCE_ONLY_SUITE"] = suite["id"]
        env["NODE_OPTIONS"] = f"--require={guard}"
        if result_path is not None:
            guard_receipt = result_path.with_suffix(".guard-receipt.jsonl")
            if guard_receipt.is_symlink() or guard_receipt.exists():
                raise SystemExit(
                    f"source-only guard receipt already exists: {guard_receipt}"
                )
            guard_receipt.open("x").close()
            env["PWRIGHT_CORE_SOURCE_ONLY_GUARD_RECEIPT"] = str(guard_receipt)
        else:
            # Collection loads pinned source in the CLI process, but never forks
            # test workers. Its no-I/O guard still runs before any test module.
            env["PWRIGHT_CORE_SOURCE_ONLY_GUARD_RECEIPT"] = os.devnull

    candidate: Path | None = None
    if args.mode == "candidate":
        candidate = args.candidate.resolve()
        if not (candidate / "package.json").is_file():
            raise SystemExit(f"candidate package is missing: {candidate}")
        preload = (
            root / "tests/official/playwright-core-candidate-preload.cjs"
        ).resolve()
        harness_inputs.append(preload)
        existing_node_options = env.get("NODE_OPTIONS", "").strip()
        require_option = f"--require={preload}"
        env["NODE_OPTIONS"] = " ".join(
            part for part in (existing_node_options, require_option) if part
        )
        env["PWRIGHT_NODE_CANDIDATE"] = str(candidate)
        env["PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM"] = str(upstream)
        if candidate_profile is not None:
            env["PWRIGHT_NATIVE_PROFILE"] = candidate_profile
        if requires_browser:
            env["PWRIGHT_CANDIDATE_CDP_ENDPOINT"] = args.endpoint
    elif not args.collect_only and requires_browser:
        preload = (root / "tests/official/playwright-core-oracle-preload.cjs").resolve()
        harness_inputs.append(preload)
        existing_node_options = env.get("NODE_OPTIONS", "").strip()
        require_option = f"--require={preload}"
        env["NODE_OPTIONS"] = " ".join(
            part for part in (existing_node_options, require_option) if part
        )
        env["PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM"] = str(upstream)
        env["PWRIGHT_ORACLE_CDP_ENDPOINT"] = args.endpoint

    print(
        f"running Playwright core mode={args.mode} suite={args.suite} "
        f"collect_only={args.collect_only} browser={requires_browser} "
        f"profile={candidate_profile!r} "
        f"filters={len(args.test or suite.get('args', []))} grep={(args.grep or pinned_grep)!r} "
        f"grep_invert={pinned_grep_invert!r}",
        flush=True,
    )
    if result_path:
        print(f"result={result_path}", flush=True)
    try:
        completed = subprocess.run(command, cwd=upstream, env=env, check=False)
    finally:
        if secure_certificate_directory is not None:
            secure_certificate_directory.cleanup()
    if result_path and not result_path.is_file():
        print("runner did not produce the requested JSON result", file=sys.stderr)
        raise SystemExit(completed.returncode or 2)
    try:
        _require_tooling_unchanged(root, tooling_sources)
    except ValueError as error:
        raise SystemExit(str(error)) from error
    if result_path:
        source_only_execution: dict[str, object] | None = None
        if secure_fixture:
            try:
                _verify_secure_result_selection(result_path, cast(str, project), suite)
            except (OSError, TypeError, ValueError, json.JSONDecodeError) as error:
                raise SystemExit(
                    f"secure Core result selection invalid: {error}"
                ) from error
        if pinned_full_file:
            try:
                _verify_pinned_full_file_result_selection(result_path, suite, upstream)
            except (OSError, TypeError, ValueError, json.JSONDecodeError) as error:
                raise SystemExit(
                    f"pinned full-file Core result selection invalid: {error}"
                ) from error
        if source_only:
            assert source_only_source is not None and guard_receipt is not None
            try:
                selected_count = _verify_source_only_result_selection(
                    result_path, suite, source_only_source
                )
                worker_count = _verify_source_only_guard_receipt(
                    guard_receipt, suite, source_only_source
                )
            except (OSError, TypeError, ValueError, json.JSONDecodeError) as error:
                raise SystemExit(
                    f"source-only Core evidence invalid: {error}"
                ) from error
            source_only_execution = {
                "scope": "upstream-private-source-only",
                "candidateRuntimeAttested": False,
                "publicApiParityEvidence": False,
                "source": source_only_source,
                "reportedTests": selected_count,
                "guardReceipt": str(guard_receipt),
                "guardReceiptSha256": _file_sha256(guard_receipt),
                "guardedWorkerProcesses": worker_count,
            }
        provenance = _result_provenance(
            mode=args.mode,
            manifest=manifest,
            manifest_path=manifest_path,
            suite=suite,
            upstream=upstream,
            project=project,
            filters=args.test or suite.get("args", []),
            grep=args.grep or pinned_grep,
            grep_invert=pinned_grep_invert,
            workers=args.workers,
            headed=args.headed,
            endpoint=args.endpoint,
            fixture_bind=fixture_bind,
            fixture_host=fixture_host,
            candidate=candidate,
            candidate_profile=candidate_profile,
            harness_inputs=harness_inputs,
            command=command,
            result=result_path,
            exit_code=completed.returncode,
            secure_fixture_sources=secure_fixture_sources,
            tooling_sources=tooling_sources,
            source_only_execution=source_only_execution,
        )
        provenance_path = result_path.with_suffix(".provenance.json")
        try:
            _require_tooling_unchanged(root, tooling_sources)
        except ValueError as error:
            raise SystemExit(str(error)) from error
        provenance_path.write_text(
            json.dumps(provenance, indent=2, sort_keys=True) + "\n"
        )
        print(f"provenance={provenance_path}", flush=True)
    raise SystemExit(completed.returncode)


if __name__ == "__main__":
    main()
