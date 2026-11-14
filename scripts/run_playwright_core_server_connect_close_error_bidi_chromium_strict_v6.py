#!/usr/bin/env python3
"""Capture six fixture-adapted, browserless official Bidi-config Close cases."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import socket
import subprocess
import sys
import tempfile
from datetime import datetime
from pathlib import Path
from typing import Any

from official_candidate_package_snapshot_v1 import sha256, verify_snapshot

ROOT = Path(__file__).resolve().parents[1]
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-v1.58.0")
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
SOURCE = "tests/library/browsertype-connect.spec.ts"
SOURCE_SHA256 = "9de4d9c8c71b8509887c81220f2eb0042e89e146c65d0311b020d191814ed2bc"
UPSTREAM_INPUTS = {
    SOURCE: SOURCE_SHA256,
    "tests/bidi/playwright.config.ts": (
        "9d9e2bb9df999850649c3126091a9492dd38957f581be5bc85871abbf9a0a6d8"
    ),
    "tests/bidi/expectationUtil.ts": (
        "f43f925e304f45363078f4cc8f2573dc3ebd453324de4021d20c718db6646adb"
    ),
    "tests/config/baseTest.ts": "4fc41689120135a7cfbe066431ebc5b3751b1a6745e2d48e7013336da556a52c",
    "tests/config/browserTest.ts": (
        "0ef5440e164d262e7c3d01523448b5d3c4d50b99060604c20f168f04419a13d5"
    ),
    "tests/config/commonFixtures.ts": (
        "cc8aad21b0684cc4958a9d76fa86090db77e93ba855c655773033da9025b9fcc"
    ),
    "tests/config/platformFixtures.ts": (
        "bb414c74d5013fbc5900434fa1bfb0b58210899862265ec9a701372a4a27b1d3"
    ),
    "tests/config/serverFixtures.ts": (
        "4c0039a149bd61e8707adddc626b345632561bd343749293ddbe9458f6ac617b"
    ),
    "tests/config/testModeFixtures.ts": (
        "5560c5215cb10585ff74e827f71d229c8e45c2b7b0d663691476b9707252f3a8"
    ),
    "tests/config/testMode.ts": "7c12e6bd3fa7e2e3e4ff296277f4a60656d4d8a0c536cb55d9d6c1e83016f575",
    "tests/config/testserver/index.ts": (
        "4330e0b94c88fdba07401efd3f4cd30a229bb48da037d8c880cd79ba0bae7307"
    ),
}
MANIFEST = (
    ROOT
    / "tests/official"
    / "playwright-core-server-connect-close-error-bidi-chromium-strict-v6-manifest.json"
)
GUARD = ROOT / "tests/official/playwright-core-server-connect-close-error-strict-v5-preload.cjs"
OWNER = ROOT / "tests/official/playwright-core-browser-type-owner-v2-preload.cjs"
COMMON = ROOT / "tests/official/playwright-core-server-runtime.cjs"
CANDIDATE_PRELOAD = ROOT / "tests/official/playwright-core-candidate-preload.cjs"
PINNED_HARNESS = {
    GUARD: "9b4c33fe52e084c5bc92ef2f777537f815ac12d0b5fa3ed208092e43ad5aaedd",
    OWNER: "b7dc4e3be3aa4c07ac069bc07adfa5a2e65456efc6fc438e49a84a907ffc06ce",
    COMMON: "58278e6c976da47074a1eb8d682916ebc3d0a41975fe89fe9270d1a664c396fd",
    CANDIDATE_PRELOAD: "d17be590ff9334b0d99dd16d0078e4b434a39eae21568b799314ababc6bbf924",
}
PROFILE = "playwright-server-connect-close-error"
ARTIFACT_SHA256 = "39a8ecfdac8db8d0cf1eb0e100c3c17aaf59fdf9c4dbb4f9739602202f0843f1"
SOURCE_PACKAGE = ROOT / "packages/pwright"
FROZEN_PARENT = (
    ROOT / ".work/official-conformance/results/core-server-connect-close-error-strict-v5"
)
SNAPSHOT_PACKAGE = FROZEN_PARENT / "candidate-package-snapshot-v1"
SNAPSHOT_MANIFEST = FROZEN_PARENT / "candidate-package-snapshot-v1.files.json"
SNAPSHOT_MANIFEST_SHA256 = "bb901e4bac312364d6f1af762053e63e8a3dccf3fc7e1785c5d7c2c7d3524c5b"
SNAPSHOT_SOURCE_SHA256 = "1bf7e4d67e226a1666af2abff31db4960781a641bf28d3b392d4c0834c0c95b4"
FROZEN_SIDECAR = FROZEN_PARENT / "candidate-v5.strict.json"
FROZEN_SIDECAR_SHA256 = "be0859d36bf4a82302ed8124e324c69d1a94d10ab48fab00791445c32cd4fa62"
RESULT_PARENT = (
    ROOT
    / ".work/official-conformance/results/core-server-connect-close-error-bidi-chromium-strict-v6"
)
SCOPE = "official-derived-bidi-chromium-close-error-fixture-host-adapted-six-cases-internal-v6"
FIXTURE_ADAPTATION = "exact-line130-localhost-ws-to-tailscale-magicdns-v5-guard"
ADAPTED_SOURCE_SHA256 = "923655fa0381086cd6702fd1aa3c81535e0bf4c06c5d5df826c392bc79aa6a25"
SOURCE_CONNECT_BEFORE = (
    "const error = await connect(`ws://localhost:${server.PORT}/ws`).catch(e => e);"
)
SOURCE_CONNECT_AFTER = (
    "const error = await connect(`ws://${process.env.PWRIGHT_OFFICIAL_SERVER_HOST}:"
    "${server.PORT}/ws`).catch(e => e);"
)
PROJECTS = {
    "bidi-chromium-library": {
        "suite": "bidi-chromium-server-connect-close-error-strict-v6",
        "specs": {
            "ee6df4b3a2217bd356a7-5977d0c5bef040f1bf03": "launchServer",
            "ee6df4b3a2217bd356a7-9c774e5f960e4e3f4a21": "run-server",
        },
    },
    "bidi-chrome-stable-library": {
        "suite": "bidi-chrome-stable-server-connect-close-error-strict-v6",
        "specs": {
            "ee6df4b3a2217bd356a7-2b2c19e6a3745a17907d": "launchServer",
            "ee6df4b3a2217bd356a7-f0142f43a589fa23adf3": "run-server",
        },
    },
    "bidi-chrome-canary-library": {
        "suite": "bidi-chrome-canary-server-connect-close-error-strict-v6",
        "specs": {
            "ee6df4b3a2217bd356a7-511a6ea814b661f15f2e": "launchServer",
            "ee6df4b3a2217bd356a7-82f4b2b4a00d4a3c199b": "run-server",
        },
    },
}
INPUT_FILES = (
    MANIFEST,
    GUARD,
    OWNER,
    COMMON,
    CANDIDATE_PRELOAD,
    ROOT / "scripts/official_candidate_package_snapshot_v1.py",
    ROOT / "scripts/run_playwright_official_conformance.py",
    ROOT / "scripts/official_result_destination.py",
    Path(__file__).resolve(),
    ROOT / "scripts/verify_playwright_core_server_connect_close_error_bidi_chromium_strict_v6.py",
    ROOT
    / "scripts/test_verify_playwright_core_server_connect_close_error_bidi_chromium_strict_v6.py",
)


def check_source_and_manifest() -> None:
    assert not sys.flags.optimize
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
    assert all(sha256(UPSTREAM / name) == digest for name, digest in UPSTREAM_INPUTS.items())
    assert all(sha256(path) == digest for path, digest in PINNED_HARNESS.items())
    suites = [
        {
            "id": selection["suite"],
            "role": "browser-independent-fixture-adapted-diagnostic",
            "runner": "packages/playwright/cli.js",
            "config": "tests/bidi/playwright.config.ts",
            "args": [f"{SOURCE}:130"],
            "candidateAdapter": "test-mode-playwright-core",
            "project": project,
            "requiresBrowser": False,
            "expectedTestsPerProject": 2,
            "expectedFiles": 1,
        }
        for project, selection in PROJECTS.items()
    ]
    assert json.loads(MANIFEST.read_text()) == {
        "schema": 1,
        "repository": "https://github.com/microsoft/playwright.git",
        "tag": "v1.58.0",
        "commit": COMMIT,
        "suites": suites,
    }
    source = (UPSTREAM / SOURCE).read_text()
    assert source.count(SOURCE_CONNECT_BEFORE) == 1
    assert (
        hashlib.sha256(
            source.replace(SOURCE_CONNECT_BEFORE, SOURCE_CONNECT_AFTER).encode()
        ).hexdigest()
        == ADAPTED_SOURCE_SHA256
    )
    marker = "test('should print custom ws close error', async ({ connect, server }) => {"
    assert source.count(marker) == 1
    body = source[
        source.index(marker) : source.index("\n    test(", source.index(marker) + len(marker))
    ]
    for assertion in (
        "server.onceWebSocketConnection((ws, request) => {",
        "ws.on('message', message => {",
        "ws.close(4123, 'Oh my!');",
        SOURCE_CONNECT_BEFORE,
        "expect(error.message).toContain('browserType.connect: Oh my!');",
    ):
        assert body.count(assertion) == 1
    assert "startRemoteServer" not in body
    assert "browserType.launch" not in body
    assert "await browser." not in body
    assert (UPSTREAM / "tests/config/serverFixtures.ts").read_text().count(
        "const port = 8907 + workerInfo.workerIndex * 4;"
    ) == 1
    assert (
        "await run(playwright[browserName]);"
        in (UPSTREAM / "tests/config/browserTest.ts").read_text()
    )
    bidi_config = (UPSTREAM / "tests/bidi/playwright.config.ts").read_text()
    for project in PROJECTS:
        assert project.removesuffix("-library") in bidi_config


def check_frozen_candidate() -> dict[str, Any]:
    assert sha256(SNAPSHOT_MANIFEST) == SNAPSHOT_MANIFEST_SHA256
    assert sha256(FROZEN_SIDECAR) == FROZEN_SIDECAR_SHA256
    previous = json.loads(FROZEN_SIDECAR.read_text())
    assert previous["mode"] == "candidate"
    assert previous["candidateProfile"] == PROFILE
    assert previous["candidateArtifactSha256"] == ARTIFACT_SHA256
    assert previous["candidateSnapshot"] == str(SNAPSHOT_PACKAGE)
    assert previous["candidateSnapshotManifestSha256"] == SNAPSHOT_MANIFEST_SHA256
    assert previous["candidateSourceSha256"] == SNAPSHOT_SOURCE_SHA256
    snapshot = verify_snapshot(
        SNAPSHOT_PACKAGE,
        SNAPSHOT_MANIFEST,
        source_package=SOURCE_PACKAGE,
        profile=PROFILE,
        artifact_sha256=ARTIFACT_SHA256,
    )
    assert snapshot["candidateSourceSha256"] == SNAPSHOT_SOURCE_SHA256
    return snapshot


def fixture_dns_preflight() -> dict[str, Any]:
    bind = subprocess.check_output(["tailscale", "ip", "-4"], text=True).strip()
    host = json.loads(subprocess.check_output(["tailscale", "status", "--json"], text=True))[
        "Self"
    ]["DNSName"].rstrip(".")
    if not host.endswith(".ts.net") or not bind:
        raise ValueError("Close fixture requires full Tailscale MagicDNS and IPv4")
    addresses = sorted(
        {
            item[4][0]
            for item in socket.getaddrinfo(
                host, None, family=socket.AF_INET, type=socket.SOCK_STREAM
            )
        }
    )
    if addresses != [bind]:
        raise ValueError("Close fixture MagicDNS IPv4 does not match Tailscale bind")
    return {"fixtureHost": host, "fixtureBind": bind, "resolvedIPv4": addresses}


def selected_tests(report: dict[str, Any], project: str) -> list[dict[str, Any]]:
    expected_ids = PROJECTS[project]["specs"]
    selected: list[dict[str, Any]] = []

    def visit(suite: dict[str, Any]) -> None:
        for spec in suite.get("specs", []):
            spec_id = spec["id"]
            assert spec_id in expected_ids
            assert (spec["file"], spec["line"], spec["column"], spec["title"]) == (
                "library/browsertype-connect.spec.ts",
                130,
                5,
                "should print custom ws close error",
            )
            assert len(spec["tests"]) == 1
            test = spec["tests"][0]
            assert test["projectId"] == project
            assert test["expectedStatus"] == "passed"
            assert test["status"] in {"expected", "unexpected"}
            assert len(test["results"]) == 1
            attempt = test["results"][0]
            assert attempt["retry"] == 0
            assert attempt["status"] in {"passed", "failed"}
            assert isinstance(attempt["workerIndex"], int)
            assert 0 <= attempt["workerIndex"] <= 16
            started = datetime.fromisoformat(attempt["startTime"].replace("Z", "+00:00"))
            assert started.tzinfo is not None
            assert isinstance(attempt["duration"], int)
            assert attempt["duration"] >= 0
            passed = attempt["status"] == "passed"
            assert test["status"] == ("expected" if passed else "unexpected")
            assert spec["ok"] == passed
            errors = attempt["errors"]
            assert isinstance(errors, list)
            assert not errors if passed else bool(errors)
            selected.append(
                {
                    "project": project,
                    "kind": expected_ids[spec_id],
                    "specId": spec_id,
                    "line": 130,
                    "title": spec["title"],
                    "status": test["status"],
                    "attempts": [
                        {
                            "retry": attempt["retry"],
                            "status": attempt["status"],
                            "workerIndex": attempt["workerIndex"],
                            "startTime": attempt["startTime"],
                            "durationMs": attempt["duration"],
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
    assert {item["specId"] for item in selected} == set(expected_ids)
    assert [item["kind"] for item in selected] == ["launchServer", "run-server"]
    starts = [
        datetime.fromisoformat(item["attempts"][0]["startTime"].replace("Z", "+00:00"))
        for item in selected
    ]
    assert starts[0] <= starts[1]
    stats = report["stats"]
    assert stats["expected"] == sum(item["status"] == "expected" for item in selected)
    assert stats["unexpected"] == sum(item["status"] == "unexpected" for item in selected)
    assert stats["skipped"] == stats["flaky"] == 0
    return selected


def paths(mode: str, project: str) -> dict[str, Path]:
    result = RESULT_PARENT / f"{mode}-{project}-v6.json"
    return {
        "result": result,
        "provenance": result.with_suffix(".provenance.json"),
        "receipt": result.with_suffix(".guard-receipt.jsonl"),
        "log": result.with_suffix(".log"),
    }


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
    if sys.flags.optimize:
        raise RuntimeError("strict Close evidence requires Python assertions enabled")
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
        "PWRIGHT_CORE_SERVER_CONNECT_CLOSE_ERROR_V5_MODE",
        "PWRIGHT_CORE_SERVER_CONNECT_CLOSE_ERROR_V5_RECEIPT",
    )
    if any(os.environ.get(key) for key in forbidden):
        raise ValueError("strict Bidi Close capture rejects inherited runtime overrides")
    check_source_and_manifest()
    snapshot = check_frozen_candidate()
    dns_preflight = fixture_dns_preflight()
    RESULT_PARENT.mkdir(parents=True, exist_ok=True)
    strict_path = RESULT_PARENT / f"{mode}-v6.strict.json"
    destinations = [strict_path]
    for project in PROJECTS:
        destinations.extend(paths(mode, project).values())
    if any(path.exists() or path.is_symlink() for path in destinations):
        raise ValueError("strict Bidi Close destination already exists")
    inputs = {str(path.relative_to(ROOT)): sha256(path) for path in INPUT_FILES}
    if mode == "candidate":
        from verify_playwright_core_server_connect_close_error_bidi_chromium_strict_v6 import (
            verify_strict,
        )

        oracle = verify_strict(RESULT_PARENT / "oracle-v6.strict.json")
        assert oracle["inputs"] == inputs
        assert all(
            item["status"] == "expected" for one in oracle["runs"] for item in one["selected"]
        )
    runs = []
    for project, selection in PROJECTS.items():
        output = paths(mode, project)
        output["receipt"].open("x").close()
        env = os.environ.copy()
        env["TAILSCALE_IP"] = dns_preflight["fixtureBind"]
        env["TAILSCALE_HOST"] = dns_preflight["fixtureHost"]
        env["NODE_OPTIONS"] = f"--require={GUARD}"
        env["PWRIGHT_CORE_SERVER_CONNECT_CLOSE_ERROR_V5_MODE"] = mode
        env["PWRIGHT_CORE_SERVER_CONNECT_CLOSE_ERROR_V5_RECEIPT"] = str(output["receipt"])
        command = [
            "uv",
            "run",
            "--no-project",
            "--with",
            "pytest==8.4.2",
            str(ROOT / "scripts/run_playwright_official_conformance.py"),
            mode,
            str(UPSTREAM),
            "--suite",
            selection["suite"],
            "--manifest",
            str(MANIFEST),
            "--workers",
            "1",
            "--result",
            str(output["result"]),
        ]
        if mode == "candidate":
            command.extend(
                [
                    "--candidate",
                    str(SNAPSHOT_PACKAGE),
                    "--candidate-profile",
                    PROFILE,
                ]
            )
        completed = subprocess.run(
            command,
            cwd=ROOT,
            env=env,
            capture_output=True,
            text=True,
            check=False,
            timeout=180,
        )
        output["log"].write_text(completed.stdout + completed.stderr)
        assert all(sha256(ROOT / name) == digest for name, digest in inputs.items())
        check_source_and_manifest()
        assert check_frozen_candidate() == snapshot
        if not output["result"].is_file() or not output["provenance"].is_file():
            raise RuntimeError(
                f"official {project} runner produced no result/provenance "
                f"(exit {completed.returncode}): {output['log'].read_text()[-3000:]}"
            )
        selected = selected_tests(json.loads(output["result"].read_text()), project)
        assert completed.returncode == (
            0 if all(item["status"] == "expected" for item in selected) else 1
        )
        if mode == "oracle":
            assert all(item["status"] == "expected" for item in selected)
        assert ("[pwright-candidate]" in output["log"].read_text()) == (mode == "candidate")
        runs.append(
            {
                "project": project,
                "suite": selection["suite"],
                "result": str(output["result"]),
                "resultSha256": sha256(output["result"]),
                "provenanceSha256": sha256(output["provenance"]),
                "guardReceiptSha256": sha256(output["receipt"]),
                "logSha256": sha256(output["log"]),
                "exitCode": completed.returncode,
                "selected": selected,
            }
        )
    strict = {
        "schema": 1,
        "scope": SCOPE,
        "sourceClassification": "official-derived-fixture-host-adapted-internal",
        "fixtureAdaptation": FIXTURE_ADAPTATION,
        "unadaptedOfficialPass": False,
        "adaptedSourceSha256": ADAPTED_SOURCE_SHA256,
        "fixtureDnsPreflight": dns_preflight,
        "publicApiParityEvidence": False,
        "browserExecutionEvidence": False,
        "bidiProtocolParityEvidence": False,
        "currentLivePackageEvidence": False,
        "promotedToCoreLedger": False,
        "perSpecTransportAttribution": "ordered-project-receipts-only-not-proven-per-spec",
        "mode": mode,
        "upstreamCommit": COMMIT,
        "upstreamInputs": UPSTREAM_INPUTS,
        "inputs": inputs,
        "candidateProfile": PROFILE if mode == "candidate" else None,
        "candidateArtifactSha256": ARTIFACT_SHA256 if mode == "candidate" else None,
        "candidateSnapshot": str(SNAPSHOT_PACKAGE) if mode == "candidate" else None,
        "candidateSnapshotManifestSha256": (
            SNAPSHOT_MANIFEST_SHA256 if mode == "candidate" else None
        ),
        "candidateSourceSha256": (SNAPSHOT_SOURCE_SHA256 if mode == "candidate" else None),
        "frozenCandidateSidecarSha256": FROZEN_SIDECAR_SHA256,
        "runs": runs,
    }
    write_json_exclusive(strict_path, strict)
    return strict


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("oracle", "candidate"))
    strict = run(parser.parse_args().mode)
    print(
        f"Core Bidi Close {strict['mode']}: "
        f"{[item['status'] for one in strict['runs'] for item in one['selected']]}"
    )


if __name__ == "__main__":
    main()
