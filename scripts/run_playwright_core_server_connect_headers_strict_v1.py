"""Capture four pinned Core BrowserType.connect header cases without a browser."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import socket
import subprocess
import tempfile
from pathlib import Path
from typing import Any

from official_candidate_package_snapshot_v1 import (
    create_snapshot,
    sha256,
    verify_snapshot,
)

ROOT = Path(__file__).parents[1].resolve()
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-v1.58.0")
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
SOURCE = "tests/library/browsertype-connect.spec.ts"
SOURCE_SHA256 = "9de4d9c8c71b8509887c81220f2eb0042e89e146c65d0311b020d191814ed2bc"
SERVER_FIXTURE = "tests/config/serverFixtures.ts"
SERVER_FIXTURE_SHA256 = (
    "4c0039a149bd61e8707adddc626b345632561bd343749293ddbe9458f6ac617b"
)
TEST_MODE_FIXTURE = "tests/config/testModeFixtures.ts"
TEST_MODE_FIXTURE_SHA256 = (
    "5560c5215cb10585ff74e827f71d229c8e45c2b7b0d663691476b9707252f3a8"
)
TEST_SERVER = "tests/config/testserver/index.ts"
TEST_SERVER_SHA256 = "4330e0b94c88fdba07401efd3f4cd30a229bb48da037d8c880cd79ba0bae7307"
USER_AGENT_JS = "packages/playwright-core/lib/server/utils/userAgent.js"
MANIFEST = (
    ROOT
    / "tests/official/playwright-core-server-connect-headers-strict-v1-manifest.json"
)
PRELOAD = (
    ROOT / "tests/official/playwright-core-server-connect-headers-strict-v1-preload.cjs"
)
OWNER_PRELOAD = (
    ROOT / "tests/official/playwright-core-browser-type-owner-v2-preload.cjs"
)
COMMON_PRELOAD = ROOT / "tests/official/playwright-core-server-runtime.cjs"
CANDIDATE_PRELOAD = ROOT / "tests/official/playwright-core-candidate-preload.cjs"
PROFILE = "playwright-server-connect-headers"
ARTIFACT_SHA256 = "30d8478b0d74acfe556b5209633045434503738c4424483ab7ea7bb952f358aa"
SUITE = "library-chromium-server-connect-headers-strict-v1"
RESULT_PARENT = (
    ROOT / ".work/official-conformance/results/core-server-connect-headers-strict-v1"
)
SOURCE_PACKAGE = ROOT / "packages/pwright"
SCOPE = "official-derived-core-server-connect-headers-fixture-host-adapted-internal-v1"
CLASSIFICATION = "official-derived-fixture-host-adapted-internal"
FIXTURE_ADAPTATION = (
    "exact-two-line238-and-line253-localhost-ws-to-tailscale-magicdns-v1"
)
SOURCE_EXTRA_BEFORE = (
    "connect(`ws://localhost:${server.PORT}/ws`, {\n"
    "          headers: {\n"
    "            'User-Agent': 'Playwright',"
)
SOURCE_DEFAULT_BEFORE = (
    "connect(`ws://localhost:${server.PORT}/ws`, {\n"
    "          headers: {\n"
    "            'foo': 'bar',"
)
SOURCE_EXTRA_AFTER = SOURCE_EXTRA_BEFORE.replace(
    "localhost", "${process.env.PWRIGHT_OFFICIAL_SERVER_HOST}"
)
SOURCE_DEFAULT_AFTER = SOURCE_DEFAULT_BEFORE.replace(
    "localhost", "${process.env.PWRIGHT_OFFICIAL_SERVER_HOST}"
)
EXPECTED_SPEC_IDS = {
    "ee6df4b3a2217bd356a7-9382fbb61fa92ab98ca5": (
        "launchServer",
        "extra",
        235,
        "should send extra headers with connect request",
    ),
    "ee6df4b3a2217bd356a7-80c13ccf5df0f8fe137d": (
        "launchServer",
        "default",
        250,
        "should send default User-Agent and X-Playwright-Browser headers with connect request",
    ),
    "ee6df4b3a2217bd356a7-604eb43298a85cc85a37": (
        "run-server",
        "extra",
        235,
        "should send extra headers with connect request",
    ),
    "ee6df4b3a2217bd356a7-07fdbb8cb8ad53bc3eaf": (
        "run-server",
        "default",
        250,
        "should send default User-Agent and X-Playwright-Browser headers with connect request",
    ),
}
INPUT_FILES = (
    MANIFEST,
    PRELOAD,
    OWNER_PRELOAD,
    COMMON_PRELOAD,
    CANDIDATE_PRELOAD,
    ROOT / "scripts/official_candidate_package_snapshot_v1.py",
    ROOT / "scripts/run_playwright_official_conformance.py",
    ROOT / "scripts/official_result_destination.py",
    ROOT / "scripts/verify_playwright_official_provenance.py",
    Path(__file__).resolve(),
    ROOT / "scripts/verify_playwright_core_server_connect_headers_strict_v1.py",
    ROOT / "scripts/test_verify_playwright_core_server_connect_headers_strict_v1.py",
)


def adapted_source_sha256() -> str:
    source = (UPSTREAM / SOURCE).read_text()
    return hashlib.sha256(
        source.replace(SOURCE_EXTRA_BEFORE, SOURCE_EXTRA_AFTER)
        .replace(SOURCE_DEFAULT_BEFORE, SOURCE_DEFAULT_AFTER)
        .encode()
    ).hexdigest()


def check_source_and_manifest() -> None:
    assert (
        subprocess.check_output(
            ["git", "-C", str(UPSTREAM), "rev-parse", "HEAD"], text=True
        ).strip()
        == COMMIT
    )
    assert not subprocess.check_output(
        ["git", "-C", str(UPSTREAM), "status", "--porcelain", "--untracked-files=no"],
        text=True,
    ).strip()
    for source, digest in (
        (SOURCE, SOURCE_SHA256),
        (SERVER_FIXTURE, SERVER_FIXTURE_SHA256),
        (TEST_MODE_FIXTURE, TEST_MODE_FIXTURE_SHA256),
        (TEST_SERVER, TEST_SERVER_SHA256),
    ):
        assert sha256(UPSTREAM / source) == digest
    assert json.loads(MANIFEST.read_text()) == {
        "schema": 1,
        "repository": "https://github.com/microsoft/playwright.git",
        "tag": "v1.58.0",
        "commit": COMMIT,
        "suites": [
            {
                "id": SUITE,
                "role": "browser-independent-fixture-adapted-diagnostic",
                "runner": "packages/playwright/cli.js",
                "config": "tests/library/playwright.config.ts",
                "args": [f"{SOURCE}:235", f"{SOURCE}:250"],
                "candidateAdapter": "test-mode-playwright-core",
                "project": "chromium-library",
                "requiresBrowser": False,
                "expectedTestsPerProject": 4,
                "expectedFiles": 1,
            }
        ],
    }
    source = (UPSTREAM / SOURCE).read_text()
    assert source.count(SOURCE_EXTRA_BEFORE) == 1
    assert source.count(SOURCE_DEFAULT_BEFORE) == 1
    assert (
        source.count("for (const kind of ['launchServer', 'run-server'] as const)") == 1
    )
    extra_marker = "test('should send extra headers with connect request', async ({ connect, server }) => {"
    default_marker = (
        "test('should send default User-Agent and X-Playwright-Browser headers "
        "with connect request', async ({ connect, browserName, server }) => {"
    )
    for marker, assertions in (
        (
            extra_marker,
            (
                SOURCE_EXTRA_BEFORE,
                "expect(request.headers['user-agent']).toBe('Playwright');",
                "expect(request.headers['foo']).toBe('bar');",
            ),
        ),
        (
            default_marker,
            (
                SOURCE_DEFAULT_BEFORE,
                "expect(request.headers['user-agent']).toBe(getUserAgent());",
                "expect(request.headers['x-playwright-browser']).toBe(browserName);",
                "expect(request.headers['foo']).toBe('bar');",
            ),
        ),
    ):
        assert source.count(marker) == 1
        start = source.index(marker)
        body = source[start : source.index("\n    test(", start + len(marker))]
        for assertion in assertions:
            assert body.count(assertion) == 1
        assert body.count("server.waitForWebSocketConnectionRequest()") == 1
        assert body.count("timeout: 100,") == 1
    fixture = (UPSTREAM / TEST_SERVER).read_text()
    assert fixture.count("waitForWebSocketConnectionRequest() {") == 1
    assert (
        fixture.count("this._server.on('upgrade', async (request, socket, head) => {")
        == 1
    )


def fixture_dns_preflight() -> dict[str, Any]:
    bind = subprocess.check_output(["tailscale", "ip", "-4"], text=True).strip()
    host = json.loads(
        subprocess.check_output(["tailscale", "status", "--json"], text=True)
    )["Self"]["DNSName"].rstrip(".")
    if not host.endswith(".ts.net") or not bind:
        raise ValueError("fixture preflight requires full Tailscale MagicDNS and IPv4")
    addresses = sorted(
        {
            item[4][0]
            for item in socket.getaddrinfo(
                host, None, family=socket.AF_INET, type=socket.SOCK_STREAM
            )
        }
    )
    if addresses != [bind]:
        raise ValueError("fixture MagicDNS IPv4 does not match Tailscale bind")
    return {"fixtureHost": host, "fixtureBind": bind, "resolvedIPv4": addresses}


def reference_user_agent() -> str:
    module = UPSTREAM / USER_AGENT_JS
    assert module.is_file() and not module.is_symlink()
    return subprocess.check_output(
        [
            "node",
            "-e",
            "process.stdout.write(require(process.argv[1]).getUserAgent())",
            str(module),
        ],
        text=True,
    )


def selected_tests(report: dict[str, Any]) -> list[dict[str, Any]]:
    selected: list[dict[str, Any]] = []

    def visit(suite: dict[str, Any]) -> None:
        for spec in suite.get("specs", []):
            spec_id = spec["id"]
            assert spec_id in EXPECTED_SPEC_IDS
            kind, header_case, line, title = EXPECTED_SPEC_IDS[spec_id]
            assert (spec["file"], spec["line"], spec["column"], spec["title"]) == (
                "library/browsertype-connect.spec.ts",
                line,
                5,
                title,
            )
            assert len(spec["tests"]) == 1
            test = spec["tests"][0]
            assert test["projectId"] == "chromium-library"
            assert test["expectedStatus"] == "passed"
            assert test["status"] in {"expected", "unexpected"}
            assert len(test["results"]) == 1
            attempt = test["results"][0]
            assert attempt["retry"] == 0
            assert attempt["status"] in {"passed", "failed"}
            passed = attempt["status"] == "passed"
            assert test["status"] == ("expected" if passed else "unexpected")
            assert spec["ok"] == passed
            errors = attempt["errors"]
            assert isinstance(errors, list) and bool(errors) != passed
            selected.append(
                {
                    "kind": kind,
                    "headerCase": header_case,
                    "specId": spec_id,
                    "line": line,
                    "title": title,
                    "status": test["status"],
                    "attempts": [
                        {
                            "retry": attempt["retry"],
                            "status": attempt["status"],
                            "workerIndex": attempt["workerIndex"],
                        }
                    ],
                    "errors": [str(error.get("message", "")) for error in errors],
                }
            )
        for child in suite.get("suites", []):
            visit(child)

    assert report["errors"] == []
    for suite in report["suites"]:
        visit(suite)
    assert len(selected) == 4
    assert {item["specId"] for item in selected} == set(EXPECTED_SPEC_IDS)
    selected.sort(key=lambda item: (item["kind"], item["headerCase"]))
    stats = report["stats"]
    assert stats["expected"] == sum(item["status"] == "expected" for item in selected)
    assert stats["unexpected"] == sum(
        item["status"] == "unexpected" for item in selected
    )
    assert stats["skipped"] == stats["flaky"] == 0
    return selected


def write_json_exclusive(destination: Path, value: dict[str, Any]) -> None:
    with tempfile.NamedTemporaryFile(
        mode="w",
        encoding="utf-8",
        dir=destination.parent,
        prefix=f".{destination.name}.",
        suffix=".tmp",
        delete=False,
    ) as stream:
        temporary = Path(stream.name)
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())
    try:
        os.link(temporary, destination)
    finally:
        temporary.unlink(missing_ok=True)


def run(mode: str, result_parent: Path = RESULT_PARENT) -> dict[str, Any]:
    forbidden = (
        "NODE_OPTIONS",
        "PWTEST_MODE",
        "PWPAGE_IMPL",
        "PWTEST_USE_BIDI_EXPECTATIONS",
        "PWRIGHT_CANDIDATE_CDP_ENDPOINT",
        "PWRIGHT_ORACLE_CDP_ENDPOINT",
        "PWRIGHT_CDP_ENDPOINT",
        "PWRIGHT_CDP_WS_URL",
        "PWRIGHT_NATIVE_PROFILE",
        "PWRIGHT_NODE_CANDIDATE",
        "PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM",
        "PWRIGHT_OFFICIAL_SERVER_IP",
        "PWRIGHT_OFFICIAL_SERVER_HOST",
    )
    if any(os.environ.get(key) for key in forbidden):
        raise ValueError(
            "strict connect headers diagnostic rejects inherited runtime overrides"
        )
    check_source_and_manifest()
    artifact = SOURCE_PACKAGE / "native" / PROFILE / "pwright.node"
    if mode == "candidate" and (
        not artifact.is_file() or sha256(artifact) != ARTIFACT_SHA256
    ):
        raise ValueError(
            "pinned connect headers candidate artifact is missing or changed"
        )
    dns_preflight = fixture_dns_preflight()
    expected_user_agent = reference_user_agent()
    user_agent_js_sha256 = sha256(UPSTREAM / USER_AGENT_JS)
    result_parent = result_parent.resolve()
    result_parent.mkdir(parents=True, exist_ok=True)
    result = result_parent / f"{mode}-v1.json"
    receipt = result.with_suffix(".guard-receipt.jsonl")
    provenance = result.with_suffix(".provenance.json")
    log_path = result.with_suffix(".log")
    strict_path = result.with_suffix(".strict.json")
    snapshot_package = result_parent / "candidate-package-snapshot-v1"
    snapshot_manifest = result_parent / "candidate-package-snapshot-v1.files.json"
    destinations = [result, receipt, provenance, log_path, strict_path]
    if mode == "candidate":
        destinations += [snapshot_package, snapshot_manifest]
    if any(path.exists() or path.is_symlink() for path in destinations):
        raise ValueError("strict connect headers destination already exists")
    inputs = {str(path.relative_to(ROOT)): sha256(path) for path in INPUT_FILES}
    snapshot = None
    if mode == "candidate":
        snapshot = create_snapshot(
            SOURCE_PACKAGE,
            snapshot_package,
            snapshot_manifest,
            profile=PROFILE,
            artifact_sha256=ARTIFACT_SHA256,
        )
    receipt.open("x").close()
    env = os.environ.copy()
    env["NODE_OPTIONS"] = f"--require={PRELOAD}"
    env["PWRIGHT_CORE_SERVER_CONNECT_HEADERS_V1_MODE"] = mode
    env["PWRIGHT_CORE_SERVER_CONNECT_HEADERS_V1_RECEIPT"] = str(receipt)
    command = [
        "uv",
        "run",
        "--with",
        "pytest",
        str(ROOT / "scripts/run_playwright_official_conformance.py"),
        mode,
        str(UPSTREAM),
        "--suite",
        SUITE,
        "--manifest",
        str(MANIFEST),
        "--workers",
        "1",
        "--result",
        str(result),
    ]
    if mode == "candidate":
        command += [
            "--candidate",
            str(snapshot_package),
            "--candidate-profile",
            PROFILE,
        ]
    completed = subprocess.run(
        command, cwd=ROOT, env=env, capture_output=True, text=True, check=False
    )
    log_path.write_text(completed.stdout + completed.stderr)
    assert all(sha256(ROOT / name) == digest for name, digest in inputs.items())
    check_source_and_manifest()
    assert sha256(UPSTREAM / USER_AGENT_JS) == user_agent_js_sha256
    if snapshot is not None:
        assert (
            verify_snapshot(
                snapshot_package,
                snapshot_manifest,
                source_package=SOURCE_PACKAGE,
                profile=PROFILE,
                artifact_sha256=ARTIFACT_SHA256,
            )
            == snapshot
        )
    if not result.is_file() or not provenance.is_file():
        raise RuntimeError(
            f"official connect headers runner produced no result/provenance (exit {completed.returncode}): "
            f"{log_path.read_text()[-3000:]}"
        )
    selected = selected_tests(json.loads(result.read_text()))
    assert completed.returncode == (
        0 if all(item["status"] == "expected" for item in selected) else 1
    )
    if mode == "oracle":
        assert all(item["status"] == "expected" for item in selected)
    assert ("[pwright-candidate]" in log_path.read_text()) == (mode == "candidate")
    records = [json.loads(line) for line in receipt.read_text().splitlines() if line]
    assert records and any(record.get("workerIndex") is not None for record in records)
    strict = {
        "schema": 1,
        "scope": SCOPE,
        "sourceClassification": CLASSIFICATION,
        "fixtureAdaptation": FIXTURE_ADAPTATION,
        "wireAttribution": "ordered-upgrades-and-header-case-counts-no-per-spec-socket-id",
        "wireReceiptScope": "asserted-header-values-and-protocol-name-counts-v1",
        "unadaptedOfficialPass": False,
        "publicApiParityEvidence": False,
        "promotedToCoreLedger": False,
        "mode": mode,
        "upstreamCommit": COMMIT,
        "upstreamSourceSha256": SOURCE_SHA256,
        "adaptedSourceSha256": adapted_source_sha256(),
        "serverFixtureSha256": SERVER_FIXTURE_SHA256,
        "testModeFixtureSha256": TEST_MODE_FIXTURE_SHA256,
        "testServerSourceSha256": TEST_SERVER_SHA256,
        "userAgentJsSha256": user_agent_js_sha256,
        "expectedDefaultUserAgent": expected_user_agent,
        "fixtureDnsPreflight": dns_preflight,
        "inputs": inputs,
        "candidateProfile": PROFILE if mode == "candidate" else None,
        "candidateArtifactSha256": ARTIFACT_SHA256 if mode == "candidate" else None,
        "candidateSnapshot": str(snapshot_package) if mode == "candidate" else None,
        "candidateSnapshotManifestSha256": sha256(snapshot_manifest)
        if mode == "candidate"
        else None,
        "candidateSourceSha256": snapshot["candidateSourceSha256"]
        if snapshot is not None
        else None,
        "result": str(result),
        "resultSha256": sha256(result),
        "provenanceSha256": sha256(provenance),
        "guardReceiptSha256": sha256(receipt),
        "logSha256": sha256(log_path),
        "exitCode": completed.returncode,
        "selected": selected,
    }
    write_json_exclusive(strict_path, strict)
    return strict


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("oracle", "candidate"))
    parser.add_argument("--result-parent", type=Path, default=RESULT_PARENT)
    args = parser.parse_args()
    strict = run(args.mode, args.result_parent)
    print(
        f"Core connect headers adapted diagnostic {args.mode}: "
        f"{[(item['kind'], item['headerCase'], item['status']) for item in strict['selected']]}"
    )


if __name__ == "__main__":
    main()
