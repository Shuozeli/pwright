#!/usr/bin/env python3
"""Capture six Bidi-config Core HTTP-401 cases without launching a browser."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import socket
import subprocess
import sys
from pathlib import Path
from typing import Any

from official_candidate_package_snapshot_v1 import sha256, verify_snapshot
from run_playwright_core_server_connect_http_401_strict_v5 import (
    fixture_dns_preflight,
    write_json_exclusive,
)

ROOT = Path(__file__).resolve().parents[1]
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-v1.58.0")
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
SOURCE = "tests/library/browsertype-connect.spec.ts"
SOURCE_SHA256 = "9de4d9c8c71b8509887c81220f2eb0042e89e146c65d0311b020d191814ed2bc"
ADAPTED_SOURCE_SHA256 = "b2bef73c0c226ced95ccc3110e97c3c6c49aa9b3f4e96f1e0820a254e014ded6"
UPSTREAM_INPUTS = {
    SOURCE: SOURCE_SHA256,
    "tests/bidi/playwright.config.ts": (
        "9d9e2bb9df999850649c3126091a9492dd38957f581be5bc85871abbf9a0a6d8"
    ),
    "tests/bidi/expectationUtil.ts": (
        "f43f925e304f45363078f4cc8f2573dc3ebd453324de4021d20c718db6646adb"
    ),
    "tests/config/baseTest.ts": (
        "4fc41689120135a7cfbe066431ebc5b3751b1a6745e2d48e7013336da556a52c"
    ),
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
    "tests/config/testMode.ts": (
        "7c12e6bd3fa7e2e3e4ff296277f4a60656d4d8a0c536cb55d9d6c1e83016f575"
    ),
    "tests/config/testserver/index.ts": (
        "4330e0b94c88fdba07401efd3f4cd30a229bb48da037d8c880cd79ba0bae7307"
    ),
}
MANIFEST = (
    ROOT
    / "tests/official/playwright-core-server-connect-http-401-bidi-chromium-strict-v6-manifest.json"
)
GUARD = ROOT / "tests/official/playwright-core-server-connect-http-401-strict-v5-preload.cjs"
OWNER = ROOT / "tests/official/playwright-core-browser-type-owner-v2-preload.cjs"
COMMON = ROOT / "tests/official/playwright-core-server-runtime.cjs"
CANDIDATE_PRELOAD = ROOT / "tests/official/playwright-core-candidate-preload.cjs"
PINNED_HARNESS = {
    GUARD: "221c740f7ffac280d368343338ffcfec957545b03225e5275d68fbac3a1d72fe",
    OWNER: "b7dc4e3be3aa4c07ac069bc07adfa5a2e65456efc6fc438e49a84a907ffc06ce",
    COMMON: "58278e6c976da47074a1eb8d682916ebc3d0a41975fe89fe9270d1a664c396fd",
    CANDIDATE_PRELOAD: "d17be590ff9334b0d99dd16d0078e4b434a39eae21568b799314ababc6bbf924",
    ROOT / "scripts/official_candidate_package_snapshot_v1.py": (
        "20af90af081554d2102f34ca337e7ff1bff438488dfe50f3db0cf8e7d7363fbe"
    ),
    ROOT / "scripts/run_playwright_core_server_connect_http_401_strict_v5.py": (
        "662f2947eb0013ea686cb2f5c6f7ffcdbc89597fa4cc86766d769d040da59f0d"
    ),
    ROOT / "scripts/run_playwright_official_conformance.py": (
        "21f1df10e77485bf29b67b31e548a1bc5126d50dc47cdeb115a9cf6798704a7b"
    ),
    ROOT / "scripts/official_result_destination.py": (
        "61cd172cc58af3fecfe09b9d44da5ae67038ab660f7ed8a140b1ddf1784c04ab"
    ),
}
PROFILE = "playwright-server-connect-ws-error"
ARTIFACT_SHA256 = "cbdcb780b8c8f04dfb51ddc867b13a4964c8696aeac0e2a4737f113554fdc1d0"
CANDIDATE_SOURCE_SHA256 = "1bf7e4d67e226a1666af2abff31db4960781a641bf28d3b392d4c0834c0c95b4"
SNAPSHOT_MANIFEST_SHA256 = "ea5b9a1a2437575a82e5c85ae4cdcee1c1e2238a5e6c8cadd7d93e0a0550037b"
SOURCE_PACKAGE = ROOT / "packages/pwright"
SNAPSHOT_PARENT = ROOT / ".work/official-conformance/results/core-server-connect-http-401-strict-v5"
SNAPSHOT_PACKAGE = SNAPSHOT_PARENT / "candidate-package-snapshot-v1"
SNAPSHOT_MANIFEST = SNAPSHOT_PARENT / "candidate-package-snapshot-v1.files.json"
RESULT_PARENT = (
    ROOT / ".work/official-conformance/results/core-server-connect-http-401-bidi-chromium-strict-v6"
)
SCOPE = "official-derived-core-http-401-bidi-chromium-fixture-host-adapted-six-cases-v6"
CLASSIFICATION = "official-derived-fixture-host-adapted-internal"
FIXTURE_ADAPTATION = "exact-one-line116-localhost-ws-401-to-tailscale-magicdns-v6"
PROJECTS = {
    "bidi-chromium-library": {
        "suite": "bidi-chromium-server-connect-http-401-strict-v6",
        "specs": {
            "ee6df4b3a2217bd356a7-e4ef96db466ddcd9b9c7": "launchServer",
            "ee6df4b3a2217bd356a7-c029c9945e2dad716fff": "run-server",
        },
    },
    "bidi-chrome-stable-library": {
        "suite": "bidi-chrome-stable-server-connect-http-401-strict-v6",
        "specs": {
            "ee6df4b3a2217bd356a7-fcc2e9a2f05c9afd5838": "launchServer",
            "ee6df4b3a2217bd356a7-54234fd4c0513393a2e5": "run-server",
        },
    },
    "bidi-chrome-canary-library": {
        "suite": "bidi-chrome-canary-server-connect-http-401-strict-v6",
        "specs": {
            "ee6df4b3a2217bd356a7-ee9b136cffd3c3d2a689": "launchServer",
            "ee6df4b3a2217bd356a7-f9d592efdaee806ceade": "run-server",
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
    ROOT / "scripts/run_playwright_core_server_connect_http_401_strict_v5.py",
    ROOT / "scripts/run_playwright_official_conformance.py",
    ROOT / "scripts/official_result_destination.py",
    Path(__file__).resolve(),
    ROOT / "scripts/verify_playwright_core_server_connect_http_401_bidi_chromium_strict_v6.py",
    ROOT / "scripts/test_verify_playwright_core_server_connect_http_401_bidi_chromium_strict_v6.py",
)


def check_browserless_source_and_manifest() -> None:
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
            "args": [f"{SOURCE}:116"],
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
    before = "const error = await connect(`ws://localhost:${server.PORT}/ws-401`).catch(e => e);"
    after = (
        "const error = await connect(`ws://${process.env.PWRIGHT_OFFICIAL_SERVER_HOST}"
        ":${server.PORT}/ws-401`).catch(e => e);"
    )
    assert source.count(before) == 1
    marker = "test('should print HTTP error', async ({ connect, server }) => {"
    assert source.count(marker) == 1
    body = source[
        source.index(marker) : source.index("\n    test(", source.index(marker) + len(marker))
    ]
    assert "startRemoteServer" not in body and "browserType.launch" not in body
    for assertion in (
        before,
        "expect(error.message).toContain('401');",
        "expect(error.message).toContain('Unauthorized body');",
    ):
        assert body.count(assertion) == 1
    assert (
        hashlib.sha256(source.replace(before, after).encode()).hexdigest() == ADAPTED_SOURCE_SHA256
    )
    fixture = (UPSTREAM / "tests/config/testserver/index.ts").read_text()
    assert (
        fixture.count("socket.write('HTTP/1.1 401 Unauthorized\\r\\n\\r\\nUnauthorized body');")
        == 1
    )
    browser_test = (UPSTREAM / "tests/config/browserTest.ts").read_text()
    assert "await run(playwright[browserName]);" in browser_test
    test_mode = (UPSTREAM / "tests/config/testMode.ts").read_text()
    assert "return require('playwright-core');" in test_mode
    bidi_config = (UPSTREAM / "tests/bidi/playwright.config.ts").read_text()
    for project in PROJECTS:
        assert project.removesuffix("-library") in bidi_config


def checked_snapshot() -> dict[str, Any]:
    assert SNAPSHOT_MANIFEST.is_file() and not SNAPSHOT_MANIFEST.is_symlink()
    assert sha256(SNAPSHOT_MANIFEST) == SNAPSHOT_MANIFEST_SHA256
    snapshot = verify_snapshot(
        SNAPSHOT_PACKAGE,
        SNAPSHOT_MANIFEST,
        source_package=SOURCE_PACKAGE,
        profile=PROFILE,
        artifact_sha256=ARTIFACT_SHA256,
    )
    assert snapshot["candidateSourceSha256"] == CANDIDATE_SOURCE_SHA256
    return snapshot


def probe_fixture_ports(bind: str) -> None:
    """Check both possible one-worker fixture blocks before every run."""
    listeners: list[socket.socket] = []
    try:
        for port in range(8907, 8915):
            listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            listeners.append(listener)
            listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            listener.bind((bind, port))
            listener.listen(1)
    finally:
        for listener in listeners:
            listener.close()


def selected_tests(report: dict[str, Any], project: str) -> list[dict[str, Any]]:
    expected_ids = PROJECTS[project]["specs"]
    selected: list[dict[str, Any]] = []
    assert report["errors"] == []
    assert len(report["suites"]) == 1
    file_suite = report["suites"][0]
    assert file_suite["title"] == "library/browsertype-connect.spec.ts"
    assert file_suite["file"] == "library/browsertype-connect.spec.ts"
    assert file_suite["specs"] == []
    groups = file_suite["suites"]
    assert [group["title"] for group in groups] == ["launchServer", "run-server"]
    for group in groups:
        assert group["file"] == "library/browsertype-connect.spec.ts"
        assert not group.get("suites")
        assert len(group["specs"]) == 1
        for spec in group["specs"]:
            spec_id = spec["id"]
            assert spec_id in expected_ids
            assert expected_ids[spec_id] == group["title"]
            assert (spec["file"], spec["line"], spec["column"], spec["title"]) == (
                "library/browsertype-connect.spec.ts",
                116,
                5,
                "should print HTTP error",
            )
            assert len(spec["tests"]) == 1
            test = spec["tests"][0]
            assert test["projectId"] == test["projectName"] == project
            assert test["expectedStatus"] == "passed"
            assert test["status"] in {"expected", "unexpected"}
            assert len(test["results"]) == 1
            attempt = test["results"][0]
            assert attempt["retry"] == 0 and attempt["status"] in {"passed", "failed"}
            assert type(attempt["workerIndex"]) is int
            assert 0 <= attempt["workerIndex"] <= 1
            passed = attempt["status"] == "passed"
            assert test["status"] == ("expected" if passed else "unexpected")
            assert spec["ok"] == passed
            errors = attempt["errors"]
            assert isinstance(errors, list) and (not errors if passed else bool(errors))
            assert all(
                isinstance(error, dict)
                and isinstance(error.get("message"), str)
                and error["message"]
                for error in errors
            )
            selected.append(
                {
                    "project": project,
                    "kind": expected_ids[spec_id],
                    "specId": spec_id,
                    "line": 116,
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
    assert len(selected) == 2 and {item["specId"] for item in selected} == set(expected_ids)
    stats = report["stats"]
    assert stats["expected"] == sum(item["status"] == "expected" for item in selected)
    assert stats["unexpected"] == sum(item["status"] == "unexpected" for item in selected)
    assert stats["skipped"] == stats["flaky"] == 0
    return sorted(selected, key=lambda item: item["kind"])


def paths(mode: str, project: str) -> dict[str, Path]:
    result = RESULT_PARENT / f"{mode}-{project}-v6.json"
    return {
        "result": result,
        "provenance": result.with_suffix(".provenance.json"),
        "receipt": result.with_suffix(".guard-receipt.jsonl"),
        "log": result.with_suffix(".log"),
    }


def run(mode: str) -> dict[str, Any]:
    if sys.flags.optimize:
        raise RuntimeError("strict Core evidence requires Python assertions enabled")
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
        raise ValueError("strict Bidi HTTP-401 capture rejects inherited runtime overrides")
    check_browserless_source_and_manifest()
    snapshot = checked_snapshot()
    preflight = fixture_dns_preflight()
    RESULT_PARENT.mkdir(parents=True, exist_ok=True)
    strict_path = RESULT_PARENT / f"{mode}-v6.strict.json"
    destinations = [strict_path]
    for project in PROJECTS:
        destinations.extend(paths(mode, project).values())
    if any(path.exists() or path.is_symlink() for path in destinations):
        raise ValueError("strict Bidi HTTP-401 destination already exists")
    inputs = {str(path.relative_to(ROOT)): sha256(path) for path in INPUT_FILES}
    if mode == "candidate":
        from verify_playwright_core_server_connect_http_401_bidi_chromium_strict_v6 import (
            verify_strict,
        )

        oracle = verify_strict(RESULT_PARENT / "oracle-v6.strict.json")
        assert oracle["inputs"] == inputs
        assert all(
            item["status"] == "expected" for one in oracle["runs"] for item in one["selected"]
        )
    runs = []
    for project, selection in PROJECTS.items():
        probe_fixture_ports(preflight["fixtureBind"])
        output = paths(mode, project)
        output["receipt"].open("x").close()
        env = os.environ.copy()
        env["TAILSCALE_IP"] = preflight["fixtureBind"]
        env["TAILSCALE_HOST"] = preflight["fixtureHost"]
        env["NODE_OPTIONS"] = f"--require={GUARD}"
        env["PWRIGHT_CORE_SERVER_CONNECT_HTTP_401_V5_MODE"] = mode
        env["PWRIGHT_CORE_SERVER_CONNECT_HTTP_401_V5_RECEIPT"] = str(output["receipt"])
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
            command.extend(["--candidate", str(SNAPSHOT_PACKAGE), "--candidate-profile", PROFILE])
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
        check_browserless_source_and_manifest()
        assert checked_snapshot() == snapshot
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
        provenance = json.loads(output["provenance"].read_text())
        assert (provenance["fixtureHost"], provenance["fixtureBind"]) == (
            preflight["fixtureHost"],
            preflight["fixtureBind"],
        )
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
        "sourceClassification": CLASSIFICATION,
        "fixtureAdaptation": FIXTURE_ADAPTATION,
        "fixtureAdapted": True,
        "receiptAttribution": "worker-count-only",
        "unadaptedOfficialPass": False,
        "publicApiParityEvidence": False,
        "browserExecutionEvidence": False,
        "bidiProjectNamesOnly": True,
        "bidiProtocolParityEvidence": False,
        "currentLivePackageEvidence": False,
        "promotedToCoreLedger": False,
        "mode": mode,
        "upstreamCommit": COMMIT,
        "upstreamInputs": UPSTREAM_INPUTS,
        "inputs": inputs,
        "fixtureDnsPreflight": preflight,
        "candidateProfile": PROFILE if mode == "candidate" else None,
        "candidateArtifactSha256": ARTIFACT_SHA256 if mode == "candidate" else None,
        "candidateSnapshot": str(SNAPSHOT_PACKAGE) if mode == "candidate" else None,
        "candidateSnapshotManifestSha256": SNAPSHOT_MANIFEST_SHA256
        if mode == "candidate"
        else None,
        "candidateSourceSha256": CANDIDATE_SOURCE_SHA256 if mode == "candidate" else None,
        "runs": runs,
    }
    write_json_exclusive(strict_path, strict)
    return strict


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("oracle", "candidate"))
    strict = run(parser.parse_args().mode)
    print(
        f"Core Bidi HTTP-401 {strict['mode']}: "
        f"{[item['status'] for one in strict['runs'] for item in one['selected']]}"
    )


if __name__ == "__main__":
    main()
