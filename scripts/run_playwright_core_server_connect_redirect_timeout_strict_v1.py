"""Capture the unchanged official Core redirect-timeout test in isolated lanes.

The only fixture change is the existing common Tailscale host/bind policy.
No browser, CDP, BrowserServer, or test-body rewrite is part of this diagnostic.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import socket
import subprocess
from collections import Counter
from pathlib import Path
from typing import Any

from official_candidate_package_snapshot_v1 import sha256, verify_snapshot
from run_playwright_core_server_connect_timeouts_strict_v1 import _write_json_exclusive

ROOT = Path(__file__).parents[1].resolve()
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-v1.58.0")
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
SOURCE = "tests/library/browsertype-connect.spec.ts"
SOURCE_SHA256 = "9de4d9c8c71b8509887c81220f2eb0042e89e146c65d0311b020d191814ed2bc"
SERVER_FIXTURE = "tests/config/serverFixtures.ts"
SERVER_FIXTURE_SHA256 = (
    "4c0039a149bd61e8707adddc626b345632561bd343749293ddbe9458f6ac617b"
)
TEST_SERVER = "tests/config/testserver/index.ts"
TEST_SERVER_SHA256 = "4330e0b94c88fdba07401efd3f4cd30a229bb48da037d8c880cd79ba0bae7307"
MANIFEST = (
    ROOT
    / "tests/official/playwright-core-server-connect-redirect-timeout-strict-v1-manifest.json"
)
PRELOAD = (
    ROOT
    / "tests/official/playwright-core-server-connect-redirect-timeout-strict-v1-preload.cjs"
)
PROFILE = "playwright-server-connect-session"
ARTIFACT_SHA256 = "a2e668d626749e6799f4888ee089bf318925c6929d5bc032adc4e3577f85448c"
CANDIDATE_SOURCE_SHA256 = (
    "5abfa96c18a5e312aa45f8451650f339dfcf55ddd06814f3bdadf98a6e2234e5"
)
SOURCE_PACKAGE = (
    ROOT
    / ".work/official-conformance/artifacts/playwright-server-connect-session-timeout-map-v2-20261012/candidate-package"
)
PREDECESSOR = (
    ROOT
    / ".work/official-conformance/results/core-server-connect-timeouts-strict-v2/core-server-connect-timeouts-strict-v1-fixed-v2-20261012/attestation.json"
)
PREDECESSOR_SHA256 = "c30d9b193697a3be4a7d5705885363fa8a7ed6a66181d6ce7a69499fbc965f88"
SNAPSHOT = PREDECESSOR.parent / "candidate-package-snapshot"
SNAPSHOT_MANIFEST = PREDECESSOR.parent / "candidate-package-snapshot.files.json"
SNAPSHOT_MANIFEST_SHA256 = (
    "bb1ac0fdc5db562035996826919b7238096a3f1ec5e13a5fdbfe8a189296e397"
)
RESULT_PARENT = (
    ROOT
    / ".work/official-conformance/results/core-server-connect-redirect-timeout-strict-v1"
)
SCOPE = "official-derived-core-server-connect-redirect-timeout-fixture-host-adapted-internal-v1"
LIBRARY_SUITE = "library-server-connect-redirect-timeout-strict-v1"
BIDI_SUITE = "bidi-server-connect-redirect-timeout-strict-v1"
PROJECTS_BY_SUITE = {
    LIBRARY_SUITE: frozenset({"chromium-library", "firefox-library", "webkit-library"}),
    BIDI_SUITE: frozenset(
        {
            "bidi-chrome-canary-library",
            "bidi-chrome-stable-library",
            "bidi-chromium-library",
            "moz-firefox-beta-library",
            "moz-firefox-library",
            "moz-firefox-nightly-library",
        }
    ),
}
PROJECTS = frozenset().union(*PROJECTS_BY_SUITE.values())
IDENTITIES_SHA256_BY_SUITE = {
    LIBRARY_SUITE: "5d5f7de334e37e76996300d8c5f1ea2ffa022d3ac6ae032a25bea0b22ee084ad",
    BIDI_SUITE: "f8963177af64ce9f2fb0422170d89f4b12104e66b74e84ee2b17ce32a9eaa654",
}
IDENTITIES_SHA256 = "57e535b678e74f3283583c400d1cfa949d5a913fdcfc44a8c161c5fa8ed29d5e"
LINE = 1056
TITLE = "should timeout after redirect when connecting over http"
INPUT_FILES = (
    MANIFEST,
    PRELOAD,
    ROOT / "tests/official/playwright-core-browser-type-owner-v2-preload.cjs",
    ROOT / "tests/official/playwright-core-server-runtime.cjs",
    ROOT / "tests/official/playwright-core-candidate-preload.cjs",
    ROOT / "scripts/official_candidate_package_snapshot_v1.py",
    ROOT / "scripts/run_playwright_official_conformance.py",
    ROOT / "scripts/official_result_destination.py",
    ROOT / "scripts/verify_playwright_official_provenance.py",
    ROOT / "scripts/run_playwright_core_server_connect_timeouts_strict_v1.py",
    Path(__file__).resolve(),
    ROOT
    / "scripts/verify_playwright_core_server_connect_redirect_timeout_strict_v1.py",
    ROOT
    / "scripts/test_verify_playwright_core_server_connect_redirect_timeout_strict_v1.py",
    PREDECESSOR,
    SNAPSHOT_MANIFEST,
)
FORBIDDEN_OVERRIDES = (
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
    "PWRIGHT_OFFICIAL_CROSS_ORIGIN_FORWARD",
    "PWRIGHT_CORE_SERVER_REDIRECT_TIMEOUT_V1_MODE",
    "PWRIGHT_CORE_SERVER_REDIRECT_TIMEOUT_V1_RECEIPT",
)


def check_sources() -> dict[str, Any]:
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
    for relative, digest in (
        (SOURCE, SOURCE_SHA256),
        (SERVER_FIXTURE, SERVER_FIXTURE_SHA256),
        (TEST_SERVER, TEST_SERVER_SHA256),
    ):
        assert sha256(UPSTREAM / relative) == digest
    source = (UPSTREAM / SOURCE).read_text()
    marker = f"test('{TITLE}', async ({{ connect, server }}) => {{"
    body = source[source.index(marker) : source.index("\n});", source.index(marker))]
    for snippet in (
        "server.setRedirect('/connect/json', '/connect/slow');",
        "server.setRoute('/connect/slow', (req, res) => {",
        "req.socket.on('close', () => aborted = true);",
        "const error = await connect(`${server.PREFIX}/connect`, { timeout: 2000, headers: { 'Connection': 'Close' } }).catch(e => e);",
        "expect(error.message).toContain('Timeout 2000ms exceeded.');",
        "await expect.poll(() => aborted).toBe(true);",
    ):
        assert source.count(snippet) == body.count(snippet) == 1
    assert source.count(marker) == 1
    assert sha256(PREDECESSOR) == PREDECESSOR_SHA256
    assert sha256(SNAPSHOT_MANIFEST) == SNAPSHOT_MANIFEST_SHA256
    snapshot = verify_snapshot(
        SNAPSHOT,
        SNAPSHOT_MANIFEST,
        source_package=SOURCE_PACKAGE,
        profile=PROFILE,
        artifact_sha256=ARTIFACT_SHA256,
    )
    assert snapshot["candidateSourceSha256"] == CANDIDATE_SOURCE_SHA256
    assert json.loads(MANIFEST.read_text()) == {
        "schema": 1,
        "repository": "https://github.com/microsoft/playwright.git",
        "tag": "v1.58.0",
        "commit": COMMIT,
        "suites": [
            {
                "id": suite,
                "role": "browser-independent-fixture-host-adapted-diagnostic",
                "runner": "packages/playwright/cli.js",
                "config": f"tests/{config}/playwright.config.ts",
                "args": [f"{SOURCE}:{LINE}"],
                "candidateAdapter": "test-mode-playwright-core",
                "requiresBrowser": False,
                "expectedTestsPerProject": 1,
                "expectedFiles": 1,
            }
            for suite, config in ((LIBRARY_SUITE, "library"), (BIDI_SUITE, "bidi"))
        ],
    }
    return snapshot


def fixture_dns_preflight() -> dict[str, object]:
    bind = subprocess.check_output(["tailscale", "ip", "-4"], text=True).strip()
    host = json.loads(
        subprocess.check_output(["tailscale", "status", "--json"], text=True)
    )["Self"]["DNSName"].rstrip(".")
    assert re.fullmatch(
        r"100\.(?:6[4-9]|[7-9][0-9]|1[01][0-9]|12[0-7])\.(?:\d{1,3})\.(?:\d{1,3})", bind
    )
    assert re.fullmatch(r"[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*\.ts\.net", host)
    addresses = sorted(
        {
            item[4][0]
            for item in socket.getaddrinfo(
                host, None, family=socket.AF_INET, type=socket.SOCK_STREAM
            )
        }
    )
    assert addresses == [bind]
    return {"fixtureHost": host, "fixtureBind": bind, "resolvedIPv4": addresses}


def selected_tests(report: dict[str, Any], suite_id: str) -> list[dict[str, Any]]:
    assert suite_id in PROJECTS_BY_SUITE
    selected: list[dict[str, Any]] = []
    identities: list[str] = []

    def visit(suite: dict[str, Any]) -> None:
        for spec in suite.get("specs", []):
            assert (spec["file"], spec["line"], spec["column"], spec["title"]) == (
                "library/browsertype-connect.spec.ts",
                LINE,
                1,
                TITLE,
            )
            for test in spec["tests"]:
                project = test["projectId"]
                assert project in PROJECTS_BY_SUITE[suite_id]
                assert test["expectedStatus"] == "passed"
                assert test["status"] in {"expected", "unexpected"}
                assert len(test["results"]) == 1
                attempt = test["results"][0]
                assert attempt["retry"] == 0 and attempt["status"] in {
                    "passed",
                    "failed",
                }
                passed = attempt["status"] == "passed"
                assert test["status"] == ("expected" if passed else "unexpected")
                assert bool(attempt["errors"]) != passed
                identities.append(f"{spec['id']}\t{project}\t{LINE}")
                selected.append(
                    {
                        "specId": spec["id"],
                        "projectId": project,
                        "line": LINE,
                        "title": spec["title"],
                        "status": test["status"],
                        "attempt": {
                            "retry": attempt["retry"],
                            "status": attempt["status"],
                            "workerIndex": attempt["workerIndex"],
                        },
                        "errors": [
                            str(error.get("message", "")) for error in attempt["errors"]
                        ],
                    }
                )
            assert spec["ok"] == all(
                item["status"] == "expected"
                for item in selected
                if item["specId"] == spec["id"]
            )
        for child in suite.get("suites", []):
            visit(child)

    assert report["errors"] == []
    for suite in report["suites"]:
        visit(suite)
    expected = PROJECTS_BY_SUITE[suite_id]
    assert len(selected) == len(expected)
    assert Counter(item["projectId"] for item in selected) == Counter(
        {project: 1 for project in expected}
    )
    assert len(set(identities)) == len(identities)
    digest = hashlib.sha256(("\n".join(sorted(identities)) + "\n").encode()).hexdigest()
    assert digest == IDENTITIES_SHA256_BY_SUITE[suite_id]
    stats = report["stats"]
    assert stats["expected"] == sum(item["status"] == "expected" for item in selected)
    assert stats["unexpected"] == sum(
        item["status"] == "unexpected" for item in selected
    )
    assert stats["skipped"] == stats["flaky"] == 0
    selected.sort(key=lambda item: item["projectId"])
    return selected


def capture(result_dir: Path) -> dict[str, object]:
    if any(os.environ.get(key) for key in FORBIDDEN_OVERRIDES):
        raise ValueError(
            "strict redirect timeout diagnostic rejects inherited runtime overrides"
        )
    snapshot = check_sources()
    fixture_dns_preflight()
    result_dir = result_dir.absolute()
    if (
        result_dir.parent.resolve() != RESULT_PARENT.resolve()
        or not result_dir.name.startswith(
            "core-server-connect-redirect-timeout-strict-v1-"
        )
        or result_dir.exists()
        or result_dir.is_symlink()
    ):
        raise ValueError("redirect timeout destination must be fresh and versioned")
    inputs = {str(path.relative_to(ROOT)): sha256(path) for path in INPUT_FILES}
    result_dir.mkdir(parents=True, exist_ok=False)
    for mode in ("oracle", "candidate"):
        for suite_id in PROJECTS_BY_SUITE:
            result = result_dir / f"{mode}-{suite_id}.json"
            receipt = result_dir / f"{mode}-{suite_id}.receipt.jsonl"
            log = result_dir / f"{mode}-{suite_id}.log"
            receipt.open("x").close()
            env = os.environ.copy()
            env["NODE_OPTIONS"] = f"--require={PRELOAD}"
            env["PWRIGHT_CORE_SERVER_REDIRECT_TIMEOUT_V1_MODE"] = mode
            env["PWRIGHT_CORE_SERVER_REDIRECT_TIMEOUT_V1_RECEIPT"] = str(receipt)
            command = [
                "uv",
                "run",
                "--with",
                "pytest",
                str(ROOT / "scripts/run_playwright_official_conformance.py"),
                mode,
                str(UPSTREAM),
                "--suite",
                suite_id,
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
                    str(SNAPSHOT),
                    "--candidate-profile",
                    PROFILE,
                ]
            completed = subprocess.run(
                command,
                cwd=ROOT,
                env=env,
                capture_output=True,
                text=True,
                check=False,
                timeout=240,
            )
            log.write_text(completed.stdout + completed.stderr)
            if (
                not result.is_file()
                or not result.with_suffix(".provenance.json").is_file()
            ):
                raise RuntimeError(
                    f"{mode}/{suite_id} produced no JSON/provenance (exit {completed.returncode}): "
                    f"{log.read_text()[-3000:]}"
                )
            selected = selected_tests(json.loads(result.read_text()), suite_id)
            assert completed.returncode == (
                0 if all(item["status"] == "expected" for item in selected) else 1
            )
    assert inputs == {str(path.relative_to(ROOT)): sha256(path) for path in INPUT_FILES}
    assert check_sources() == snapshot
    from verify_playwright_core_server_connect_redirect_timeout_strict_v1 import (
        verify_directory,
    )

    attestation = verify_directory(result_dir, require_attestation=False)
    attestation["inputs"] = inputs
    _write_json_exclusive(result_dir / "attestation.json", attestation)
    verify_directory(result_dir)
    return attestation


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("result_dir", type=Path)
    args = parser.parse_args()
    attestation = capture(args.result_dir)
    for mode in ("oracle", "candidate"):
        lane = attestation["lanes"][mode]
        print(f"{mode}: {lane['passed']}/9 expected, {lane['unexpected']} unexpected")
    print(f"diagnostic only: {args.result_dir / 'attestation.json'}")


if __name__ == "__main__":
    main()
