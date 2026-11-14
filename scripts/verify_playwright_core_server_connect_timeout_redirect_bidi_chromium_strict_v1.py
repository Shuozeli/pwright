"""Independently verify the five-case Bidi Core timeout/redirect diagnostic.

The two timeout URL literals are fixture-host adapted; the redirect test body
and all five upstream assertions are unchanged. This is implementation-internal
evidence, not a browser run or public-API parity claim.
"""

from __future__ import annotations

import argparse
import hashlib
import ipaddress
import json
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path
from typing import Any

from official_candidate_package_snapshot_v1 import verify_snapshot
from verify_playwright_official_provenance import verify_provenance

ROOT = Path(__file__).resolve().parents[1]
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-v1.58.0")
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
SOURCE = "tests/library/browsertype-connect.spec.ts"
SOURCE_SHA256 = "9de4d9c8c71b8509887c81220f2eb0042e89e146c65d0311b020d191814ed2bc"
ADAPTED_SOURCE_SHA256 = "84991e12f684fc2f13d2aa915c40fbf450e6481de27eea7f9e149bbc84411cbb"
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
RESULT_DIR = (
    ROOT
    / ".work/official-conformance/results"
    / "core-server-connect-timeout-redirect-bidi-chromium-strict-v1"
    / "strict-pair-20261020-v1"
)
MANIFEST = (
    ROOT
    / "tests/official"
    / "playwright-core-server-connect-timeout-redirect-bidi-chromium-strict-v1-manifest.json"
)
SUITE = "bidi-chromium-server-connect-timeout-redirect-strict-v1"
PROJECT = "bidi-chromium-library"
PROFILE = "playwright-server-connect-session"
SCOPE = (
    "official-browserless-core-server-connect-timeout-redirect-bidi-chromium-internal-diagnostic-v1"
)
FIXTURE_ADAPTATION = "two-literal-localhost-ws-urls-plus-common-server-bind-host"
ARTIFACT_DIR = (
    ROOT
    / ".work/official-conformance/artifacts"
    / "playwright-server-connect-http-header-order-v1-20261012"
)
SOURCE_PACKAGE = ARTIFACT_DIR / "candidate-package"
SNAPSHOT = ARTIFACT_DIR / "candidate-package-snapshot"
SNAPSHOT_MANIFEST = SNAPSHOT.with_name("candidate-package-snapshot.files.json")
SNAPSHOT_MANIFEST_SHA256 = "a79c22bc1d160cc46379806820ee10d06ad4762aee3c037522a798cb129599b8"
ARTIFACT_SHA256 = "0f1c8b1c6140206bdb4692d88ff1d1b9fb235b883739f176f720c3b59361523e"
CANDIDATE_SOURCE_SHA256 = "bddad5646a880efc80a94d7bf4b975e4eb06691562ac6602a8df29c3cf97afe2"
FILTERS = [f"{SOURCE}:221", f"{SOURCE}:228", f"{SOURCE}:1056"]
TARGETS = {
    "ee6df4b3a2217bd356a7-5817a5ed954cfa532ace": (
        221,
        "launchServer",
        "should timeout in socket while connecting",
    ),
    "ee6df4b3a2217bd356a7-5b07f2a5e757a16ed468": (
        221,
        "run-server",
        "should timeout in socket while connecting",
    ),
    "ee6df4b3a2217bd356a7-30b6e0f369ffebc5d854": (
        228,
        "launchServer",
        "should timeout in connect while connecting",
    ),
    "ee6df4b3a2217bd356a7-50b361500ef280984f89": (
        228,
        "run-server",
        "should timeout in connect while connecting",
    ),
    "ee6df4b3a2217bd356a7-27eeb64a07b3673ac59b": (
        1056,
        "standalone",
        "should timeout after redirect when connecting over http",
    ),
}
STRICT_FIELDS = {
    "schema",
    "scope",
    "sourceClassification",
    "fixtureAdapted",
    "fixtureAdaptation",
    "publicApiParityEvidence",
    "promotedToCoreLedger",
    "realBrowserServer",
    "mode",
    "upstreamCommit",
    "upstreamInputs",
    "inputs",
    "candidateProfile",
    "candidateArtifactSha256",
    "candidateSnapshot",
    "candidateSnapshotManifestSha256",
    "candidateSourceSha256",
    "run",
}
RUN_FIELDS = {
    "suite",
    "project",
    "result",
    "resultSha256",
    "provenanceSha256",
    "guardReceiptSha256",
    "logSha256",
    "exitCode",
    "selected",
}
COMMON_RECEIPT_FIELDS = {
    "event",
    "pid",
    "workerIndex",
    "mode",
    "fixtureHost",
    "fixtureBind",
    "ownerBound",
    "cdpEndpointPresent",
}
HOST_RE = re.compile(r"[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*\.ts\.net\Z")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def regular(path: Path, label: str) -> None:
    require(path.is_file() and not path.is_symlink(), f"{label} missing or linked")


def file_digest(path: Path, label: str) -> str:
    regular(path, label)
    return digest(path.read_bytes())


def paths(mode: str) -> dict[str, Path]:
    require(mode in {"oracle", "candidate"}, "unknown lane")
    return {
        "result": RESULT_DIR / f"{mode}-v1.json",
        "provenance": RESULT_DIR / f"{mode}-v1.provenance.json",
        "receipt": RESULT_DIR / f"{mode}-v1.guard-receipt.jsonl",
        "log": RESULT_DIR / f"{mode}-v1.log",
        "strict": RESULT_DIR / f"{mode}-v1.strict.json",
    }


def check_source_and_manifest() -> None:
    """Pin the upstream closure and prove the adaptation does not edit assertions."""
    require(
        subprocess.check_output(
            ["git", "-C", str(UPSTREAM), "rev-parse", "HEAD"], text=True
        ).strip()
        == COMMIT,
        "upstream commit drifted",
    )
    require(
        not subprocess.check_output(
            [
                "git",
                "-C",
                str(UPSTREAM),
                "status",
                "--porcelain",
                "--untracked-files=no",
            ],
            text=True,
        ).strip(),
        "upstream tracked checkout is dirty",
    )
    for relative, expected in UPSTREAM_INPUTS.items():
        require(
            file_digest(UPSTREAM / relative, relative) == expected,
            f"upstream {relative} drifted",
        )
    require(
        json.loads(MANIFEST.read_text())
        == {
            "schema": 1,
            "repository": "https://github.com/microsoft/playwright.git",
            "tag": "v1.58.0",
            "commit": COMMIT,
            "suites": [
                {
                    "id": SUITE,
                    "role": "browser-independent-fixture-host-adapted-diagnostic",
                    "runner": "packages/playwright/cli.js",
                    "config": "tests/bidi/playwright.config.ts",
                    "args": FILTERS,
                    "candidateAdapter": "test-mode-playwright-core",
                    "project": PROJECT,
                    "requiresBrowser": False,
                    "expectedTestsPerProject": 5,
                    "expectedFiles": 1,
                }
            ],
        },
        "manifest selection or diagnostic classification differs",
    )
    source = (UPSTREAM / SOURCE).read_text()
    adapted = source
    for title, path, timeout in (
        ("should timeout in socket while connecting", "/ws-slow", 1000),
        ("should timeout in connect while connecting", "/ws", 100),
    ):
        marker = f"test('{title}', async ({{ connect, server }}) => {{"
        before = f"const e = await connect(`ws://localhost:${{server.PORT}}{path}`, {{"
        after = before.replace("ws://localhost", "ws://${process.env.PWRIGHT_OFFICIAL_SERVER_HOST}")
        assertion = (
            f"expect(e.message).toContain('browserType.connect: Timeout {timeout}ms exceeded');"
        )
        require(
            source.count(marker) == source.count(before) == source.count(assertion) == 1,
            f"{title} source differs",
        )
        body = source[
            source.index(marker) : source.index("\n    test(", source.index(marker) + len(marker))
        ]
        require(body.count(before) == body.count(assertion) == 1, f"{title} body differs")
        require(
            "startRemoteServer" not in body and "browserType.launch" not in body,
            f"{title} needs a browser",
        )
        adapted = adapted.replace(before, after)
    redirect_marker = (
        "test('should timeout after redirect when connecting over http', "
        "async ({ connect, server }) => {"
    )
    require(source.count(redirect_marker) == 1, "redirect source differs")
    redirect = source[
        source.index(redirect_marker) : source.index("\n});", source.index(redirect_marker))
    ]
    for snippet in (
        "server.setRedirect('/connect/json', '/connect/slow');",
        "server.setRoute('/connect/slow', (req, res) => {",
        "req.socket.on('close', () => aborted = true);",
        "const error = await connect(`${server.PREFIX}/connect`, "
        "{ timeout: 2000, headers: { 'Connection': 'Close' } }).catch(e => e);",
        "expect(error.message).toContain('Timeout 2000ms exceeded.');",
        "await expect.poll(() => aborted).toBe(true);",
    ):
        require(
            redirect.count(snippet) == 1,
            f"redirect assertion/fixture differs: {snippet}",
        )
    require(
        "startRemoteServer" not in redirect and "browserType.launch" not in redirect,
        "redirect needs a browser",
    )
    require(
        adapted[
            adapted.index(redirect_marker) : adapted.index("\n});", adapted.index(redirect_marker))
        ]
        == redirect,
        "redirect body was adapted",
    )
    require(
        digest(adapted.encode()) == ADAPTED_SOURCE_SHA256,
        "two-literal adapted source differs",
    )
    fixture = (UPSTREAM / "tests/config/testserver/index.ts").read_text()
    for snippet in (
        "if (pathname === '/ws-slow')",
        "await new Promise(f => setTimeout(f, 2000));",
        "if (!['/ws', '/ws-slow'].includes(pathname))",
    ):
        require(fixture.count(snippet) == 1, f"timeout fixture differs: {snippet}")


def selected_from_report(report: dict[str, Any], *, mode: str) -> list[dict[str, Any]]:
    """Reject omissions, extra tests, retries, skips, and identity substitutions."""
    require(mode in {"oracle", "candidate"}, "unknown lane")
    require(
        isinstance(report, dict) and report.get("errors") == [],
        "report has collection errors",
    )
    selected: list[dict[str, Any]] = []
    seen: set[str] = set()

    def visit(suite: dict[str, Any], kind: str | None = None) -> None:
        require(isinstance(suite, dict), "suite is malformed")
        if suite.get("title") in {"launchServer", "run-server"}:
            kind = suite["title"]
        for spec in suite.get("specs", []):
            require(isinstance(spec, dict), "spec is malformed")
            spec_id = spec.get("id")
            require(
                isinstance(spec_id, str) and spec_id in TARGETS and spec_id not in seen,
                "selected spec ID differs or repeats",
            )
            line, expected_kind, title = TARGETS[spec_id]
            actual_kind = kind or "standalone"
            expected_column = 1 if line == 1056 else 5
            require(
                (
                    spec.get("file"),
                    spec.get("line"),
                    spec.get("column"),
                    spec.get("title"),
                    actual_kind,
                )
                == (
                    "library/browsertype-connect.spec.ts",
                    line,
                    expected_column,
                    title,
                    expected_kind,
                ),
                "selected spec source/title/kind differs",
            )
            tests = spec.get("tests")
            require(
                isinstance(tests, list) and len(tests) == 1,
                "selected spec test count differs",
            )
            test = tests[0]
            require(
                isinstance(test, dict)
                and test.get("projectId") == PROJECT
                and test.get("projectName") == PROJECT
                and test.get("expectedStatus") == "passed"
                and test.get("status") in {"expected", "unexpected"},
                "selected test project/status differs",
            )
            results = test.get("results")
            require(
                isinstance(results, list) and len(results) == 1,
                "selected test attempt count differs",
            )
            attempt = results[0]
            require(
                isinstance(attempt, dict)
                and attempt.get("retry") == 0
                and attempt.get("status") in {"passed", "failed"}
                and type(attempt.get("workerIndex")) is int
                and 0 <= attempt["workerIndex"] <= 4,
                "selected test attempt is malformed",
            )
            errors = attempt.get("errors")
            passed = attempt["status"] == "passed"
            require(
                isinstance(errors, list)
                and all(
                    isinstance(error, dict) and isinstance(error.get("message"), str)
                    for error in errors
                )
                and (not errors if passed else bool(errors))
                and test["status"] == ("expected" if passed else "unexpected")
                and spec.get("ok") is passed,
                "selected test attempt/status/errors disagree",
            )
            require(mode == "candidate" or passed, "oracle has a failed official assertion")
            seen.add(spec_id)
            selected.append(
                {
                    "specId": spec_id,
                    "project": PROJECT,
                    "kind": expected_kind,
                    "line": line,
                    "title": title,
                    "status": test["status"],
                    "attempts": [
                        {
                            "retry": 0,
                            "status": attempt["status"],
                            "workerIndex": attempt["workerIndex"],
                        }
                    ],
                    "errors": [error["message"] for error in errors],
                }
            )
        for child in suite.get("suites", []):
            visit(child, kind)

    suites = report.get("suites")
    require(isinstance(suites, list), "report suites are malformed")
    for suite in suites:
        visit(suite)
    require(
        seen == set(TARGETS) and len(selected) == 5,
        "report does not contain exactly five targets",
    )
    stats = report.get("stats")
    require(
        isinstance(stats, dict)
        and stats.get("expected") == sum(item["status"] == "expected" for item in selected)
        and stats.get("unexpected") == sum(item["status"] == "unexpected" for item in selected)
        and stats.get("skipped") == stats.get("flaky") == 0,
        "report stats differ from selected tests",
    )
    return sorted(selected, key=lambda item: item["specId"])


def checked_receipts(
    records: list[dict[str, Any]],
    *,
    mode: str,
    selected: list[dict[str, Any]],
    host: str,
    bind: str,
) -> dict[str, int]:
    """Verify the combined preloads' event vocabulary and causal constraints."""
    require(bool(records), "guard receipt is empty")
    selected_workers = {str(item["attempts"][0]["workerIndex"]) for item in selected}
    verified_workers: set[str] = set()
    adapted_workers: set[str] = set()
    requests: dict[tuple[int, int], tuple[str, str]] = {}
    redirected: set[tuple[int, int]] = set()
    closed: set[tuple[int, int]] = set()
    counts: Counter[str] = Counter()
    for record in records:
        require(isinstance(record, dict), "guard receipt event is malformed")
        require(
            type(record.get("pid")) is int
            and record["pid"] > 0
            and record.get("mode") == mode
            and record.get("fixtureHost") == host
            and record.get("fixtureBind") == bind
            and record.get("ownerBound") is (True if mode == "candidate" else None)
            and record.get("cdpEndpointPresent") is False,
            "guard receipt runtime/fixture identity differs",
        )
        worker = record.get("workerIndex")
        require(
            worker is None or (isinstance(worker, str) and worker.isdigit()),
            "guard worker is malformed",
        )
        require(
            worker is None or worker in selected_workers,
            "unselected worker emitted a guard receipt",
        )
        event = record.get("event")
        if event == "source-verified":
            require(
                set(record) == COMMON_RECEIPT_FIELDS | {"sourceSha256", "fixtureSha256"},
                "source verification fields differ",
            )
            require(
                record["sourceSha256"] == SOURCE_SHA256
                and record["fixtureSha256"] == UPSTREAM_INPUTS["tests/config/testserver/index.ts"],
                "source verification hashes differ",
            )
            if worker is not None:
                verified_workers.add(worker)
        elif event == "source-adapted":
            require(
                set(record)
                == COMMON_RECEIPT_FIELDS | {"sourceSha256", "adaptedSha256", "replacements"},
                "source adaptation fields differ",
            )
            require(
                record["sourceSha256"] == SOURCE_SHA256
                and record["adaptedSha256"] == ADAPTED_SOURCE_SHA256
                and record["replacements"] == 2,
                "source adaptation hashes/count differ",
            )
            if worker is not None:
                require(worker not in adapted_workers, "worker source adaptation repeated")
                adapted_workers.add(worker)
        elif event == "fixture-upgrade-request":
            require(
                set(record) == COMMON_RECEIPT_FIELDS | {"requestPath", "requestHost"},
                "upgrade receipt fields differ",
            )
            require(
                worker is not None and record["requestPath"] in {"/ws", "/ws-slow"},
                "upgrade request path/worker differs",
            )
            require(
                record["requestHost"] == f"{host}:{8907 + 4 * int(worker)}",
                "upgrade request host differs",
            )
            counts[record["requestPath"]] += 1
        elif event in {"fixture-request", "redirect-response", "slow-socket-closed"}:
            require(
                worker is not None
                and type(record.get("requestId")) is int
                and record["requestId"] > 0,
                "redirect request worker/ID differs",
            )
            key = (record["pid"], record["requestId"])
            if event == "fixture-request":
                require(
                    set(record)
                    == COMMON_RECEIPT_FIELDS
                    | {"requestId", "requestPath", "requestHost", "requestMethod"},
                    "redirect request fields differ",
                )
                path = record["requestPath"]
                require(
                    path in {"/connect/json", "/connect/slow"}
                    and record["requestMethod"] == "GET"
                    and key not in requests,
                    "redirect request path/method/ID differs",
                )
                require(
                    record["requestHost"] == f"{host}:{8907 + 4 * int(worker)}",
                    "redirect request host differs",
                )
                requests[key] = (path, worker)
                counts[path] += 1
            elif event == "redirect-response":
                require(
                    set(record) == COMMON_RECEIPT_FIELDS | {"requestId", "status", "location"},
                    "redirect response fields differ",
                )
                require(
                    key in requests
                    and requests[key] == ("/connect/json", worker)
                    and key not in redirected
                    and record["status"] == 302
                    and record["location"] == "/connect/slow",
                    "redirect response chain differs",
                )
                redirected.add(key)
                counts[event] += 1
            else:
                require(
                    set(record) == COMMON_RECEIPT_FIELDS | {"requestId"},
                    "slow close fields differ",
                )
                require(
                    key in requests
                    and requests[key] == ("/connect/slow", worker)
                    and key not in closed,
                    "slow socket close chain differs",
                )
                closed.add(key)
                counts[event] += 1
        else:
            raise ValueError(f"unrecognized guard receipt event: {event}")
    require(
        selected_workers <= verified_workers and selected_workers <= adapted_workers,
        "selected worker lacks both source guards",
    )
    passed_by_line = Counter(item["line"] for item in selected if item["status"] == "expected")
    require(passed_by_line[221] <= counts["/ws-slow"] <= 2, "slow WS receipt count differs")
    require(passed_by_line[228] <= counts["/ws"] <= 2, "WS receipt count differs")
    for event in (
        "/connect/json",
        "redirect-response",
        "/connect/slow",
        "slow-socket-closed",
    ):
        require(
            passed_by_line[1056] <= counts[event] <= 1,
            f"redirect {event} receipt count differs",
        )
    require(
        counts["slow-socket-closed"]
        <= counts["/connect/slow"]
        <= counts["redirect-response"]
        <= counts["/connect/json"],
        "redirect receipt causal counts differ",
    )
    return dict(sorted(counts.items()))


def verify_strict_payload(
    strict: dict[str, Any], *, mode: str, selected: list[dict[str, Any]]
) -> None:
    """Pure envelope check used by mutation controls and filesystem replay."""
    require(
        isinstance(strict, dict) and set(strict) == STRICT_FIELDS,
        "strict envelope fields differ",
    )
    require(
        strict["schema"] == 1
        and strict["scope"] == SCOPE
        and strict["sourceClassification"] == "implementation-internal"
        and strict["fixtureAdapted"] is True
        and strict["fixtureAdaptation"] == FIXTURE_ADAPTATION
        and strict["publicApiParityEvidence"] is False
        and strict["promotedToCoreLedger"] is False
        and strict["realBrowserServer"] is False
        and strict["mode"] == mode
        and strict["upstreamCommit"] == COMMIT
        and strict["upstreamInputs"] == UPSTREAM_INPUTS,
        "strict diagnostic identity/no-parity flags differ",
    )
    expected_candidate = mode == "candidate"
    require(
        strict["candidateProfile"] == (PROFILE if expected_candidate else None)
        and strict["candidateArtifactSha256"] == (ARTIFACT_SHA256 if expected_candidate else None)
        and strict["candidateSnapshot"] == (str(SNAPSHOT) if expected_candidate else None)
        and strict["candidateSnapshotManifestSha256"]
        == (SNAPSHOT_MANIFEST_SHA256 if expected_candidate else None)
        and strict["candidateSourceSha256"]
        == (CANDIDATE_SOURCE_SHA256 if expected_candidate else None),
        "strict candidate runtime identity differs",
    )
    run = strict["run"]
    require(isinstance(run, dict) and set(run) == RUN_FIELDS, "strict run fields differ")
    output = paths(mode)
    require(
        run["suite"] == SUITE
        and run["project"] == PROJECT
        and run["result"] == str(output["result"])
        and run["selected"] == selected
        and run["exitCode"] == (0 if all(item["status"] == "expected" for item in selected) else 1),
        "strict run identity/results differ",
    )


def verify_strict(strict_path: Path) -> dict[str, Any]:
    if sys.flags.optimize:
        raise RuntimeError("strict Core verification requires assertions enabled")
    require(
        RESULT_DIR.is_dir() and not RESULT_DIR.is_symlink(),
        "strict result directory missing or linked",
    )
    strict_path = strict_path.absolute()
    mode = strict_path.name.removesuffix("-v1.strict.json")
    require(
        mode in {"oracle", "candidate"} and strict_path == paths(mode)["strict"],
        "strict sidecar path differs",
    )
    regular(strict_path, "strict sidecar")
    check_source_and_manifest()
    require(
        file_digest(SNAPSHOT_MANIFEST, "candidate snapshot manifest") == SNAPSHOT_MANIFEST_SHA256,
        "snapshot manifest drifted",
    )
    snapshot = verify_snapshot(
        SNAPSHOT,
        SNAPSHOT_MANIFEST,
        source_package=SOURCE_PACKAGE,
        profile=PROFILE,
        artifact_sha256=ARTIFACT_SHA256,
    )
    require(
        snapshot["candidateSourceSha256"] == CANDIDATE_SOURCE_SHA256,
        "snapshot source drifted",
    )
    strict = json.loads(strict_path.read_text())
    output = paths(mode)
    for name in ("result", "provenance", "receipt", "log"):
        regular(output[name], f"{mode} {name}")
    selected = selected_from_report(json.loads(output["result"].read_text()), mode=mode)
    verify_strict_payload(strict, mode=mode, selected=selected)
    inputs = strict["inputs"]
    require(
        isinstance(inputs, dict)
        and all(isinstance(name, str) and isinstance(value, str) for name, value in inputs.items()),
        "strict harness inputs malformed",
    )
    required_inputs = {
        "tests/official/playwright-core-server-connect-timeout-redirect-bidi-chromium-strict-v1-manifest.json",
        "tests/official/playwright-core-server-connect-redirect-timeout-strict-v1-preload.cjs",
        "tests/official/playwright-core-server-connect-timeouts-strict-v1-preload.cjs",
        "tests/official/playwright-core-browser-type-owner-v2-preload.cjs",
        "tests/official/playwright-core-server-runtime.cjs",
        "tests/official/playwright-core-candidate-preload.cjs",
        "scripts/official_candidate_package_snapshot_v1.py",
        "scripts/run_playwright_official_conformance.py",
        "scripts/official_result_destination.py",
        "scripts/verify_playwright_official_provenance.py",
        "scripts/run_playwright_core_server_connect_timeout_redirect_bidi_chromium_strict_v1.py",
        "scripts/verify_playwright_core_server_connect_timeout_redirect_bidi_chromium_strict_v1.py",
        "scripts/test_verify_playwright_core_server_connect_timeout_redirect_bidi_chromium_strict_v1.py",
    }
    require(set(inputs) == required_inputs, "strict harness input set differs")
    for relative, expected in inputs.items():
        require(
            file_digest(ROOT / relative, relative) == expected,
            f"strict input changed: {relative}",
        )
    run = strict["run"]
    for field, name in (
        ("resultSha256", "result"),
        ("provenanceSha256", "provenance"),
        ("guardReceiptSha256", "receipt"),
        ("logSha256", "log"),
    ):
        require(
            run[field] == file_digest(output[name], f"{mode} {name}"),
            f"strict {field} differs",
        )
    provenance = verify_provenance(
        json.loads(output["provenance"].read_text()),
        provenance_path=output["provenance"],
        manifest_override=MANIFEST,
        result_override=output["result"],
        upstream_override=UPSTREAM,
        candidate_override=SNAPSHOT if mode == "candidate" else None,
        tooling_root=ROOT,
    )
    require(
        provenance["mode"] == mode
        and provenance["suite"] == SUITE
        and provenance["suiteRole"] == "browser-independent-fixture-host-adapted-diagnostic"
        and provenance["project"] == PROJECT
        and provenance["filters"] == FILTERS
        and provenance["workers"] == 1
        and provenance["headed"] is False
        and provenance["grep"] is None
        and provenance["adapter"] == ("test-mode-playwright-core" if mode == "candidate" else None)
        and provenance["exitCode"] == run["exitCode"]
        and provenance["resultSha256"] == run["resultSha256"],
        "official provenance route/result differs",
    )
    command = provenance["command"]
    require(
        isinstance(command, list)
        and command.count("--config=tests/bidi/playwright.config.ts") == 1
        and command.count(f"--project={PROJECT}") == 1
        and all(command.count(item) == 1 for item in FILTERS),
        "official invocation route differs",
    )
    if mode == "candidate":
        require(
            provenance["candidate"] == str(SNAPSHOT)
            and provenance["candidateProfile"] == PROFILE
            and provenance["candidateArtifactSha256"] == ARTIFACT_SHA256
            and provenance["candidateSourceSha256"] == CANDIDATE_SOURCE_SHA256,
            "candidate provenance runtime differs",
        )
    else:
        require(
            provenance["candidate"] is None and provenance["candidateProfile"] is None,
            "oracle imported candidate runtime",
        )
    host = provenance["fixtureHost"]
    bind = provenance["fixtureBind"]
    require(
        isinstance(host, str) and HOST_RE.fullmatch(host) is not None,
        "fixture host is not full MagicDNS",
    )
    require(
        isinstance(bind, str)
        and ipaddress.ip_address(bind) in ipaddress.ip_network("100.64.0.0/10"),
        "fixture bind is not a Tailscale IPv4",
    )
    records = [json.loads(line) for line in output["receipt"].read_text().splitlines() if line]
    checked_receipts(records, mode=mode, selected=selected, host=host, bind=bind)
    log = output["log"].read_text()
    require(
        ("[pwright-candidate]" in log) == (mode == "candidate"),
        "candidate import log marker differs",
    )
    return strict


def verify_pair(oracle_path: Path, candidate_path: Path) -> tuple[dict[str, Any], dict[str, Any]]:
    oracle = verify_strict(oracle_path)
    candidate = verify_strict(candidate_path)
    require(
        oracle["mode"] == "oracle" and candidate["mode"] == "candidate",
        "lane order differs",
    )
    require(
        oracle["upstreamInputs"] == candidate["upstreamInputs"]
        and oracle["inputs"] == candidate["inputs"],
        "paired harness/source differs",
    )
    oracle_cases = oracle["run"]["selected"]
    candidate_cases = candidate["run"]["selected"]
    require(len(oracle_cases) == len(candidate_cases) == 5, "paired case count differs")
    for before, after in zip(oracle_cases, candidate_cases):
        for field in ("specId", "project", "kind", "line", "title"):
            require(before[field] == after[field], f"paired {field} differs")
    return oracle, candidate


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("strict_sidecar", type=Path)
    parser.add_argument("candidate_sidecar", type=Path, nargs="?")
    args = parser.parse_args()
    try:
        if args.candidate_sidecar is None:
            strict = verify_strict(args.strict_sidecar)
            result = {
                "mode": strict["mode"],
                "statuses": [item["status"] for item in strict["run"]["selected"]],
            }
        else:
            oracle, candidate = verify_pair(args.strict_sidecar, args.candidate_sidecar)
            result = {
                "scope": SCOPE,
                "oracle": [item["status"] for item in oracle["run"]["selected"]],
                "candidate": [item["status"] for item in candidate["run"]["selected"]],
            }
    except (
        AssertionError,
        OSError,
        ValueError,
        KeyError,
        TypeError,
        json.JSONDecodeError,
    ) as error:
        parser.exit(2, f"Bidi timeout/redirect strict evidence invalid: {error}\n")
    print(json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    main()
