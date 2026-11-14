#!/usr/bin/env python3
"""Run two unchanged Core Close tests with a guarded fixture endpoint map."""

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
MANIFEST = (
    ROOT
    / "tests/official/playwright-core-server-connect-close-error-strict-v4-manifest.json"
)
PRELOAD = (
    ROOT
    / "tests/official/playwright-core-server-connect-close-error-strict-v4-preload.cjs"
)
OWNER_PRELOAD = (
    ROOT / "tests/official/playwright-core-browser-type-owner-v2-preload.cjs"
)
COMMON_PRELOAD = ROOT / "tests/official/playwright-core-server-runtime.cjs"
CANDIDATE_PRELOAD = ROOT / "tests/official/playwright-core-candidate-preload.cjs"
PROFILE = "playwright-server-connect-close-error"
ARTIFACT_SHA256 = "3f3fd557a053d8b115c0ea349b3a32bda54242fea3b8a4896ab70311090348ec"
SUITE = "library-chromium-server-connect-close-error-strict-v4"
RESULT_PARENT = (
    ROOT
    / ".work/official-conformance/results/core-server-connect-close-error-strict-v4"
)
SOURCE_PACKAGE = ROOT / "packages/pwright"
SNAPSHOT_PACKAGE = RESULT_PARENT / "candidate-package-snapshot-v1"
SNAPSHOT_MANIFEST = RESULT_PARENT / "candidate-package-snapshot-v1.files.json"
SCOPE = "official-derived-core-server-connect-ws-close-fixture-host-adapted-two-cases-internal-diagnostic-v4"
FIXTURE_ADAPTATION = "exact-line130-localhost-ws-to-tailscale-magicdns-v4"
ADAPTED_SOURCE_SHA256 = (
    "923655fa0381086cd6702fd1aa3c81535e0bf4c06c5d5df826c392bc79aa6a25"
)
SOURCE_CONNECT_BEFORE = (
    "const error = await connect(`ws://localhost:${server.PORT}/ws`).catch(e => e);"
)
SOURCE_CONNECT_AFTER = "const error = await connect(`ws://${process.env.PWRIGHT_OFFICIAL_SERVER_HOST}:${server.PORT}/ws`).catch(e => e);"
SERVER_PORT_BASE = 8907
SERVER_PORT_STRIDE = 4
EXPECTED_SPEC_IDS = {
    "ee6df4b3a2217bd356a7-5806a41224caa4a156e8": "launchServer",
    "ee6df4b3a2217bd356a7-2604d04b7d7623f5b601": "run-server",
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
    Path(__file__).resolve(),
    ROOT / "scripts/verify_playwright_core_server_connect_close_error_strict_v4.py",
    ROOT
    / "scripts/verify_playwright_core_server_connect_close_error_strict_v4_pair.py",
)


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
    assert sha256(UPSTREAM / SOURCE) == SOURCE_SHA256
    assert sha256(UPSTREAM / SERVER_FIXTURE) == SERVER_FIXTURE_SHA256
    assert sha256(UPSTREAM / TEST_MODE_FIXTURE) == TEST_MODE_FIXTURE_SHA256
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
                "args": [f"{SOURCE}:130"],
                "candidateAdapter": "test-mode-playwright-core",
                "project": "chromium-library",
                "requiresBrowser": False,
                "expectedTestsPerProject": 2,
                "expectedFiles": 1,
            }
        ],
    }
    source = (UPSTREAM / SOURCE).read_text()
    assert source.count(SOURCE_CONNECT_BEFORE) == 1
    assert (
        hashlib.sha256(
            source.replace(SOURCE_CONNECT_BEFORE, SOURCE_CONNECT_AFTER).encode()
        ).hexdigest()
        == ADAPTED_SOURCE_SHA256
    )
    assert (UPSTREAM / SERVER_FIXTURE).read_text().count(
        "const port = 8907 + workerInfo.workerIndex * 4;"
    ) == 1
    marker = (
        "test('should print custom ws close error', async ({ connect, server }) => {"
    )
    assert source.count(marker) == 1
    body = source[
        source.index(marker) : source.index(
            "\n    test(", source.index(marker) + len(marker)
        )
    ]
    for assertion in (
        "server.onceWebSocketConnection((ws, request) => {",
        "ws.on('message', message => {",
        "ws.close(4123, 'Oh my!');",
        "const error = await connect(`ws://localhost:${server.PORT}/ws`).catch(e => e);",
        "expect(error.message).toContain('browserType.connect: Oh my!');",
    ):
        assert body.count(assertion) == 1


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


def selected_tests(report: dict[str, Any]) -> list[dict[str, Any]]:
    selected: list[dict[str, Any]] = []

    def visit(suite: dict[str, Any]) -> None:
        for spec in suite.get("specs", []):
            spec_id = spec["id"]
            assert spec_id in EXPECTED_SPEC_IDS
            assert (spec["file"], spec["line"], spec["column"], spec["title"]) == (
                "library/browsertype-connect.spec.ts",
                130,
                5,
                "should print custom ws close error",
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
                    "kind": EXPECTED_SPEC_IDS[spec_id],
                    "specId": spec_id,
                    "line": 130,
                    "title": spec["title"],
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
    assert len(selected) == 2
    assert {item["specId"] for item in selected} == set(EXPECTED_SPEC_IDS)
    selected.sort(key=lambda item: item["kind"])
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


def run(mode: str) -> dict[str, Any]:
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
            "strict Server-connect Close diagnostic rejects inherited runtime overrides"
        )
    check_source_and_manifest()
    artifact = SOURCE_PACKAGE / "native" / PROFILE / "pwright.node"
    if mode == "candidate" and (
        not artifact.is_file() or sha256(artifact) != ARTIFACT_SHA256
    ):
        raise ValueError(
            "pinned server-connect-close-error candidate artifact is missing or changed"
        )
    dns_preflight = fixture_dns_preflight()
    RESULT_PARENT.mkdir(parents=True, exist_ok=True)
    result = RESULT_PARENT / f"{mode}-v4.json"
    receipt = result.with_suffix(".guard-receipt.jsonl")
    provenance = result.with_suffix(".provenance.json")
    log_path = result.with_suffix(".log")
    strict_path = result.with_suffix(".strict.json")
    destinations = [result, receipt, provenance, log_path, strict_path]
    if mode == "candidate":
        destinations += [SNAPSHOT_PACKAGE, SNAPSHOT_MANIFEST]
    if any(path.exists() or path.is_symlink() for path in destinations):
        raise ValueError(
            "strict Server-connect Close diagnostic destination already exists"
        )
    inputs = {str(path.relative_to(ROOT)): sha256(path) for path in INPUT_FILES}
    snapshot = None
    if mode == "candidate":
        snapshot = create_snapshot(
            SOURCE_PACKAGE,
            SNAPSHOT_PACKAGE,
            SNAPSHOT_MANIFEST,
            profile=PROFILE,
            artifact_sha256=ARTIFACT_SHA256,
        )
    receipt.open("x").close()
    env = os.environ.copy()
    env["TAILSCALE_IP"] = dns_preflight["fixtureBind"]
    env["TAILSCALE_HOST"] = dns_preflight["fixtureHost"]
    env["NODE_OPTIONS"] = f"--require={PRELOAD}"
    env["PWRIGHT_CORE_SERVER_CONNECT_CLOSE_ERROR_V4_MODE"] = mode
    env["PWRIGHT_CORE_SERVER_CONNECT_CLOSE_ERROR_V4_RECEIPT"] = str(receipt)
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
            str(SNAPSHOT_PACKAGE),
            "--candidate-profile",
            PROFILE,
        ]
    completed = subprocess.run(
        command, cwd=ROOT, env=env, capture_output=True, text=True, check=False
    )
    log_path.write_text(completed.stdout + completed.stderr)
    assert all(sha256(ROOT / name) == digest for name, digest in inputs.items())
    check_source_and_manifest()
    if snapshot is not None:
        assert (
            verify_snapshot(
                SNAPSHOT_PACKAGE,
                SNAPSHOT_MANIFEST,
                source_package=SOURCE_PACKAGE,
                profile=PROFILE,
                artifact_sha256=ARTIFACT_SHA256,
            )
            == snapshot
        )
    if not result.is_file() or not provenance.is_file():
        raise RuntimeError(
            f"official Close runner produced no result/provenance (exit {completed.returncode}): "
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
        "sourceClassification": "official-derived-fixture-host-adapted-internal",
        "fixtureAdaptation": FIXTURE_ADAPTATION,
        "unadaptedOfficialPass": False,
        "adaptedSourceSha256": ADAPTED_SOURCE_SHA256,
        "fixtureDnsPreflight": dns_preflight,
        "publicApiParityEvidence": False,
        "promotedToCoreLedger": False,
        "mode": mode,
        "upstreamCommit": COMMIT,
        "upstreamSourceSha256": SOURCE_SHA256,
        "serverFixtureSha256": SERVER_FIXTURE_SHA256,
        "testModeFixtureSha256": TEST_MODE_FIXTURE_SHA256,
        "inputs": inputs,
        "candidateProfile": PROFILE if mode == "candidate" else None,
        "candidateArtifactSha256": ARTIFACT_SHA256 if mode == "candidate" else None,
        "candidateSnapshot": str(SNAPSHOT_PACKAGE) if mode == "candidate" else None,
        "candidateSnapshotManifestSha256": (
            sha256(SNAPSHOT_MANIFEST) if mode == "candidate" else None
        ),
        "candidateSourceSha256": (
            snapshot["candidateSourceSha256"] if snapshot is not None else None
        ),
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
    mode = parser.parse_args().mode
    strict = run(mode)
    print(
        f"Core Server-connect Close diagnostic {mode}: "
        f"{[item['status'] for item in strict['selected']]}"
    )


if __name__ == "__main__":
    main()
