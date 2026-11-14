"""Capture browser-free official Core connect timeout diagnostics in both lanes.

Only two literal localhost fixture URLs are adapted to the full Tailscale
MagicDNS host. The pinned test assertions are unchanged. This is not a real
BrowserServer run and must never be promoted as public API parity.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import socket
import subprocess
import tempfile
from collections import Counter
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
ADAPTED_SOURCE_SHA256 = (
    "84991e12f684fc2f13d2aa915c40fbf450e6481de27eea7f9e149bbc84411cbb"
)
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
MANIFEST = (
    ROOT
    / "tests/official/playwright-core-server-connect-timeouts-strict-v1-manifest.json"
)
PRELOAD = (
    ROOT
    / "tests/official/playwright-core-server-connect-timeouts-strict-v1-preload.cjs"
)
PROFILE = "playwright-server-connect-session"
ARTIFACT_SHA256 = "cc27493d9006fc1a4a2cb1375fb08861ff93758d3f937522b8ae615c5a0f94f7"
SOURCE_PACKAGE = ROOT / "packages/pwright"
LIBRARY_SUITE = "library-server-connect-timeouts-strict-v1"
BIDI_SUITE = "bidi-server-connect-timeouts-strict-v1"
RESULT_PARENT = (
    ROOT / ".work/official-conformance/results/core-server-connect-timeouts-strict-v1"
)
SCOPE = "official-derived-core-server-connect-timeouts-fixture-host-adapted-internal-v1"
PROJECTS = frozenset(
    {
        "bidi-chrome-canary-library",
        "bidi-chrome-stable-library",
        "bidi-chromium-library",
        "chromium-library",
        "firefox-library",
        "moz-firefox-beta-library",
        "moz-firefox-library",
        "moz-firefox-nightly-library",
        "webkit-library",
    }
)
PROJECTS_BY_SUITE = {
    LIBRARY_SUITE: frozenset({"chromium-library", "firefox-library", "webkit-library"}),
    BIDI_SUITE: PROJECTS - {"chromium-library", "firefox-library", "webkit-library"},
}
EXPECTED_IDENTITIES_SHA256 = (
    "7e4180e3e9fb70df3e3460553fd770e8b3e944b9bd47f1292240310a2ec4d9f3"
)
IDENTITIES_SHA256_BY_SUITE = {
    LIBRARY_SUITE: "dbdafcfdbebad17d203faf4b7a33e72ddfd9d76e73ae5798cafd40ca11a766ab",
    BIDI_SUITE: "6d863b16bfc068b289e8f317d213398b7dfd1258facac4bca4fe8f16388301a8",
}
CASE_TITLES = {
    221: "should timeout in socket while connecting",
    228: "should timeout in connect while connecting",
}
PATH_BY_LINE = {221: "/ws-slow", 228: "/ws"}
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
    Path(__file__).resolve(),
    ROOT / "scripts/verify_playwright_core_server_connect_timeouts_strict_v1.py",
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
    "PWRIGHT_CORE_SERVER_CONNECT_TIMEOUTS_V1_MODE",
    "PWRIGHT_CORE_SERVER_CONNECT_TIMEOUTS_V1_RECEIPT",
)


def check_sources() -> None:
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
        (TEST_MODE_FIXTURE, TEST_MODE_FIXTURE_SHA256),
        (TEST_SERVER, TEST_SERVER_SHA256),
    ):
        assert sha256(UPSTREAM / relative) == digest
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
                "args": [f"{SOURCE}:221", f"{SOURCE}:228"],
                "candidateAdapter": "test-mode-playwright-core",
                "requiresBrowser": False,
                "expectedTestsPerProject": 4,
                "expectedFiles": 1,
            }
            for suite, config in ((LIBRARY_SUITE, "library"), (BIDI_SUITE, "bidi"))
        ],
    }
    source = (UPSTREAM / SOURCE).read_text()
    adapted = source
    for line, timeout in ((221, 1000), (228, 100)):
        marker = f"test('{CASE_TITLES[line]}', async ({{ connect, server }}) => {{"
        before = f"const e = await connect(`ws://localhost:${{server.PORT}}{PATH_BY_LINE[line]}`, {{"
        after = before.replace(
            "ws://localhost", "ws://${process.env.PWRIGHT_OFFICIAL_SERVER_HOST}"
        )
        assertion = f"expect(e.message).toContain('browserType.connect: Timeout {timeout}ms exceeded');"
        assert (
            source.count(marker) == source.count(before) == source.count(assertion) == 1
        )
        body = source[
            source.index(marker) : source.index("\n    test(", source.index(marker) + 1)
        ]
        assert body.count(before) == body.count(assertion) == 1
        adapted = adapted.replace(before, after)
    assert hashlib.sha256(adapted.encode()).hexdigest() == ADAPTED_SOURCE_SHA256
    server_source = (UPSTREAM / TEST_SERVER).read_text()
    for snippet in (
        "if (pathname === '/ws-slow')",
        "await new Promise(f => setTimeout(f, 2000));",
        "if (!['/ws', '/ws-slow'].includes(pathname))",
    ):
        assert server_source.count(snippet) == 1


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


def selected_tests(
    report: dict[str, Any], suite_id: str | None = None
) -> list[dict[str, Any]]:
    assert suite_id is None or suite_id in PROJECTS_BY_SUITE
    expected_projects = PROJECTS if suite_id is None else PROJECTS_BY_SUITE[suite_id]
    expected_digest = (
        EXPECTED_IDENTITIES_SHA256
        if suite_id is None
        else IDENTITIES_SHA256_BY_SUITE[suite_id]
    )
    selected: list[dict[str, Any]] = []
    identities: list[str] = []

    def visit(suite: dict[str, Any], kind: str | None = None) -> None:
        name = suite.get("title")
        if name in {"launchServer", "run-server"}:
            kind = name
        for spec in suite.get("specs", []):
            assert kind in {"launchServer", "run-server"}
            line = spec["line"]
            assert line in CASE_TITLES
            assert (spec["file"], spec["column"], spec["title"]) == (
                "library/browsertype-connect.spec.ts",
                5,
                CASE_TITLES[line],
            )
            tests = spec["tests"]
            assert len(tests) >= 1
            for test in tests:
                project = test["projectId"]
                assert (
                    project in expected_projects and test["expectedStatus"] == "passed"
                )
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
                identities.append(f"{spec['id']}\t{project}\t{line}\t{kind}")
                selected.append(
                    {
                        "specId": spec["id"],
                        "projectId": project,
                        "kind": kind,
                        "line": line,
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
            visit(child, kind)

    assert report["errors"] == []
    for suite in report["suites"]:
        visit(suite)
    assert len(selected) == len(expected_projects) * 4
    assert len(set(identities)) == len(identities)
    identity_bytes = ("\n".join(sorted(identities)) + "\n").encode()
    assert hashlib.sha256(identity_bytes).hexdigest() == expected_digest
    assert Counter(item["projectId"] for item in selected) == Counter(
        {project: 4 for project in expected_projects}
    )
    per_line = len(expected_projects) * 2
    assert Counter(item["line"] for item in selected) == Counter(
        {221: per_line, 228: per_line}
    )
    stats = report["stats"]
    assert stats["expected"] == sum(item["status"] == "expected" for item in selected)
    assert stats["unexpected"] == sum(
        item["status"] == "unexpected" for item in selected
    )
    assert stats["skipped"] == stats["flaky"] == 0
    selected.sort(key=lambda item: (item["projectId"], item["line"], item["kind"]))
    return selected


def _write_json_exclusive(path: Path, value: dict[str, object]) -> None:
    with tempfile.NamedTemporaryFile(
        mode="w",
        encoding="utf-8",
        dir=path.parent,
        prefix=f".{path.name}.",
        suffix=".tmp",
        delete=False,
    ) as stream:
        temporary = Path(stream.name)
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())
    try:
        os.link(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def capture(result_dir: Path) -> dict[str, object]:
    if any(os.environ.get(key) for key in FORBIDDEN_OVERRIDES):
        raise ValueError(
            "strict timeout diagnostic rejects inherited runtime overrides"
        )
    check_sources()
    artifact = SOURCE_PACKAGE / "native" / PROFILE / "pwright.node"
    assert artifact.is_file() and not artifact.is_symlink()
    assert sha256(artifact) == ARTIFACT_SHA256
    fixture_dns_preflight()
    result_dir = result_dir.absolute()
    if (
        result_dir.parent.resolve() != RESULT_PARENT.resolve()
        or not result_dir.name.startswith("core-server-connect-timeouts-strict-v1-")
        or result_dir.exists()
        or result_dir.is_symlink()
    ):
        raise ValueError(
            "timeout destination must be fresh and versioned under its result parent"
        )
    inputs = {str(path.relative_to(ROOT)): sha256(path) for path in INPUT_FILES}
    result_dir.mkdir(parents=True, exist_ok=False)
    snapshot_path = result_dir / "candidate-package-snapshot"
    snapshot_manifest = result_dir / "candidate-package-snapshot.files.json"
    snapshot = create_snapshot(
        SOURCE_PACKAGE,
        snapshot_path,
        snapshot_manifest,
        profile=PROFILE,
        artifact_sha256=ARTIFACT_SHA256,
    )
    for mode in ("oracle", "candidate"):
        for suite_id in PROJECTS_BY_SUITE:
            result = result_dir / f"{mode}-{suite_id}.json"
            receipt = result_dir / f"{mode}-{suite_id}.receipt.jsonl"
            log = result_dir / f"{mode}-{suite_id}.log"
            receipt.open("x").close()
            env = os.environ.copy()
            env["NODE_OPTIONS"] = f"--require={PRELOAD}"
            env["PWRIGHT_CORE_SERVER_CONNECT_TIMEOUTS_V1_MODE"] = mode
            env["PWRIGHT_CORE_SERVER_CONNECT_TIMEOUTS_V1_RECEIPT"] = str(receipt)
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
                    str(snapshot_path),
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
    check_sources()
    assert (
        verify_snapshot(
            snapshot_path,
            snapshot_manifest,
            source_package=SOURCE_PACKAGE,
            profile=PROFILE,
            artifact_sha256=ARTIFACT_SHA256,
        )
        == snapshot
    )
    from verify_playwright_core_server_connect_timeouts_strict_v1 import (
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
        print(f"{mode}: {lane['passed']}/36 expected, {lane['unexpected']} unexpected")
    print(f"diagnostic only: {args.result_dir / 'attestation.json'}")


if __name__ == "__main__":
    main()
