#!/usr/bin/env python3
"""Independently verify six browserless Bidi-config custom Close captures."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from datetime import datetime
from pathlib import Path
from typing import Any

from official_candidate_package_snapshot_v1 import sha256, verify_snapshot
from verify_playwright_official_provenance import verify_provenance

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
    ROOT / "scripts/run_playwright_core_server_connect_close_error_bidi_chromium_strict_v6.py",
    Path(__file__).resolve(),
    ROOT
    / "scripts/test_verify_playwright_core_server_connect_close_error_bidi_chromium_strict_v6.py",
)
STRICT_FIELDS = {
    "schema",
    "scope",
    "sourceClassification",
    "fixtureAdaptation",
    "unadaptedOfficialPass",
    "adaptedSourceSha256",
    "fixtureDnsPreflight",
    "publicApiParityEvidence",
    "browserExecutionEvidence",
    "bidiProtocolParityEvidence",
    "currentLivePackageEvidence",
    "promotedToCoreLedger",
    "perSpecTransportAttribution",
    "mode",
    "upstreamCommit",
    "upstreamInputs",
    "inputs",
    "candidateProfile",
    "candidateArtifactSha256",
    "candidateSnapshot",
    "candidateSnapshotManifestSha256",
    "candidateSourceSha256",
    "frozenCandidateSidecarSha256",
    "runs",
}
RUN_FIELDS = {
    "project",
    "suite",
    "result",
    "resultSha256",
    "provenanceSha256",
    "guardReceiptSha256",
    "logSha256",
    "exitCode",
    "selected",
}
SERVER_PORT_BASE = 8907
SERVER_PORT_STRIDE = 4


def regular(path: Path) -> None:
    assert path.is_file() and not path.is_symlink()


def check_inputs() -> None:
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
    for snippet in (
        "server.onceWebSocketConnection((ws, request) => {",
        "ws.on('message', message => {",
        "ws.close(4123, 'Oh my!');",
        SOURCE_CONNECT_BEFORE,
        "expect(error.message).toContain('browserType.connect: Oh my!');",
    ):
        assert body.count(snippet) == 1
    assert "startRemoteServer" not in body
    assert "browserType.launch" not in body
    assert (UPSTREAM / "tests/config/serverFixtures.ts").read_text().count(
        "const port = 8907 + workerInfo.workerIndex * 4;"
    ) == 1
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


def checked_receipts(
    path: Path, mode: str, selected: list[dict[str, Any]], host: str, bind: str
) -> int:
    regular(path)
    records = [json.loads(line) for line in path.read_text().splitlines() if line]
    assert records and all(isinstance(record, dict) for record in records)
    selected_workers = [str(item["attempts"][0]["workerIndex"]) for item in selected]
    allowed_workers = set(selected_workers)
    common = {
        "kind",
        "pid",
        "workerIndex",
        "mode",
        "sourceSha256",
        "adaptedSourceSha256",
        "fixtureHost",
        "fixtureBind",
        "ownerBound",
        "cdpEndpointPresent",
        "sourceAdapted",
        "fixtureAdapted",
    }
    endpoint = {
        "originalUrl",
        "mappedUrl",
        "port",
        "listenerAddress",
        "listenerPort",
        "listenerFamily",
        "requestHost",
    }
    details = {
        "preload": {"replacements"},
        "source-read": {"replacements"},
        "ws-upgrade": endpoint,
        "ws-message": endpoint | {"messageBytes"},
        "ws-close": endpoint | {"code", "reason"},
    }
    preload_positions: dict[int, int] = {}
    source_positions: dict[tuple[int, str], int] = {}
    ws_events: list[tuple[int, dict[str, Any]]] = []
    for position, record in enumerate(records):
        kind = record["kind"]
        assert kind in details
        assert set(record) == common | details[kind]
        assert isinstance(record["pid"], int) and record["pid"] > 0
        assert record["mode"] == mode
        assert record["sourceSha256"] == SOURCE_SHA256
        assert record["adaptedSourceSha256"] == ADAPTED_SOURCE_SHA256
        assert record["fixtureHost"] == host and record["fixtureBind"] == bind
        assert record["ownerBound"] == (True if mode == "candidate" else None)
        assert record["cdpEndpointPresent"] is False
        assert record["sourceAdapted"] is True
        assert record["fixtureAdapted"] is True
        worker = record["workerIndex"]
        assert worker is None or (
            isinstance(worker, str)
            and re.fullmatch(r"(?:0|[1-9][0-9]*)", worker)
            and int(worker) <= 16
        )
        if kind == "preload":
            assert record["replacements"] == 1
            preload_positions.setdefault(record["pid"], position)
            continue
        assert record["pid"] in preload_positions
        assert preload_positions[record["pid"]] < position
        if kind == "source-read":
            assert record["replacements"] == 1
            assert worker is None or worker in allowed_workers
            if worker is not None:
                source_positions[(record["pid"], worker)] = position
            continue
        assert worker in allowed_workers
        assert (record["pid"], worker) in source_positions
        assert source_positions[(record["pid"], worker)] < position
        port = SERVER_PORT_BASE + int(worker) * SERVER_PORT_STRIDE
        assert record["port"] == record["listenerPort"] == port
        assert record["originalUrl"] == f"ws://localhost:{port}/ws"
        assert record["mappedUrl"] == f"ws://{host}:{port}/ws"
        assert record["listenerAddress"] == bind
        assert record["listenerFamily"] == "IPv4"
        assert record["requestHost"] == f"{host}:{port}"
        if kind == "ws-message":
            assert isinstance(record["messageBytes"], int)
            assert record["messageBytes"] > 0
        if kind == "ws-close":
            assert (record["code"], record["reason"]) == (4123, "Oh my!")
        ws_events.append((position, record))
    assert preload_positions
    assert source_positions
    assert len(ws_events) % 3 == 0
    triplets = [ws_events[index : index + 3] for index in range(0, len(ws_events), 3)]
    assert len(triplets) <= 2
    observed_workers = []
    for triplet in triplets:
        assert [record["kind"] for _, record in triplet] == [
            "ws-upgrade",
            "ws-message",
            "ws-close",
        ]
        assert triplet[0][0] < triplet[1][0] < triplet[2][0]
        pid = triplet[0][1]["pid"]
        worker = triplet[0][1]["workerIndex"]
        assert all(record["pid"] == pid for _, record in triplet)
        assert all(record["workerIndex"] == worker for _, record in triplet)
        observed_workers.append(worker)
    remaining = iter(selected_workers)
    for observed in observed_workers:
        assert any(worker == observed for worker in remaining)
    if mode == "oracle":
        assert len(triplets) == 2
    else:
        # A failed candidate may reject before the fixture sees a WebSocket.
        # The guard does not record a spec ID, so receipts prove ordered project
        # transport, not a cryptographic per-spec assignment. If Playwright
        # restarts a worker, though, a passing test must have transport on its
        # reported worker rather than borrowing a failed worker's receipt.
        for worker in allowed_workers:
            expected = sum(
                item["status"] == "expected" and str(item["attempts"][0]["workerIndex"]) == worker
                for item in selected
            )
            assert observed_workers.count(worker) >= expected
    return len(triplets)


def verify_strict(strict_path: Path) -> dict[str, Any]:
    if sys.flags.optimize:
        raise RuntimeError("strict Close verification requires assertions enabled")
    regular(strict_path)
    strict = json.loads(strict_path.read_text())
    assert isinstance(strict, dict) and set(strict) == STRICT_FIELDS
    assert strict["schema"] == 1 and strict["scope"] == SCOPE
    assert strict["sourceClassification"] == "official-derived-fixture-host-adapted-internal"
    assert strict["fixtureAdaptation"] == FIXTURE_ADAPTATION
    assert strict["unadaptedOfficialPass"] is False
    assert strict["adaptedSourceSha256"] == ADAPTED_SOURCE_SHA256
    assert strict["publicApiParityEvidence"] is False
    assert strict["browserExecutionEvidence"] is False
    assert strict["bidiProtocolParityEvidence"] is False
    assert strict["currentLivePackageEvidence"] is False
    assert strict["promotedToCoreLedger"] is False
    assert (
        strict["perSpecTransportAttribution"] == "ordered-project-receipts-only-not-proven-per-spec"
    )
    mode = strict["mode"]
    assert mode in {"oracle", "candidate"}
    assert strict_path == RESULT_PARENT / f"{mode}-v6.strict.json"
    assert strict["upstreamCommit"] == COMMIT
    assert strict["upstreamInputs"] == UPSTREAM_INPUTS
    assert strict["frozenCandidateSidecarSha256"] == FROZEN_SIDECAR_SHA256
    dns = strict["fixtureDnsPreflight"]
    assert isinstance(dns, dict) and set(dns) == {
        "fixtureHost",
        "fixtureBind",
        "resolvedIPv4",
    }
    host, bind = dns["fixtureHost"], dns["fixtureBind"]
    assert isinstance(host, str) and isinstance(bind, str)
    assert re.fullmatch(r"[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*\.ts\.net", host)
    assert re.fullmatch(
        r"100\.(?:6[4-9]|[7-9][0-9]|1[01][0-9]|12[0-7])\."
        r"(?:[0-9]{1,3})\.(?:[0-9]{1,3})",
        bind,
    )
    assert dns["resolvedIPv4"] == [bind]
    check_inputs()
    inputs = strict["inputs"]
    assert isinstance(inputs, dict)
    assert set(inputs) == {str(path.relative_to(ROOT)) for path in INPUT_FILES}
    assert all(sha256(ROOT / name) == digest for name, digest in inputs.items())
    if mode == "candidate":
        assert strict["candidateProfile"] == PROFILE
        assert strict["candidateArtifactSha256"] == ARTIFACT_SHA256
        assert strict["candidateSnapshot"] == str(SNAPSHOT_PACKAGE)
        assert strict["candidateSnapshotManifestSha256"] == SNAPSHOT_MANIFEST_SHA256
        assert strict["candidateSourceSha256"] == SNAPSHOT_SOURCE_SHA256
    else:
        for key in (
            "candidateProfile",
            "candidateArtifactSha256",
            "candidateSnapshot",
            "candidateSnapshotManifestSha256",
            "candidateSourceSha256",
        ):
            assert strict[key] is None
    runs = strict["runs"]
    assert isinstance(runs, list) and len(runs) == 3
    assert [run["project"] for run in runs] == list(PROJECTS)
    all_ids = set()
    for run in runs:
        assert isinstance(run, dict) and set(run) == RUN_FIELDS
        project = run["project"]
        assert run["suite"] == PROJECTS[project]["suite"]
        output = paths(mode, project)
        assert run["result"] == str(output["result"])
        for key, path in (
            ("resultSha256", output["result"]),
            ("provenanceSha256", output["provenance"]),
            ("guardReceiptSha256", output["receipt"]),
            ("logSha256", output["log"]),
        ):
            regular(path)
            assert run[key] == sha256(path)
        provenance = verify_provenance(
            json.loads(output["provenance"].read_text()),
            provenance_path=output["provenance"],
            manifest_override=MANIFEST,
            result_override=output["result"],
            upstream_override=UPSTREAM,
            candidate_override=SNAPSHOT_PACKAGE if mode == "candidate" else None,
            tooling_root=ROOT,
        )
        assert provenance["mode"] == mode
        assert provenance["suite"] == run["suite"]
        assert provenance["project"] == project
        assert provenance["adapter"] == (
            "test-mode-playwright-core" if mode == "candidate" else None
        )
        assert provenance["workers"] == 1 and provenance["headed"] is False
        assert provenance["filters"] == [f"{SOURCE}:130"]
        assert provenance["fixtureHost"] == host
        assert provenance["fixtureBind"] == bind
        assert provenance["resultSha256"] == run["resultSha256"]
        assert provenance["exitCode"] == run["exitCode"]
        command = provenance["command"]
        assert command.count("--config=tests/bidi/playwright.config.ts") == 1
        assert command.count(f"--project={project}") == 1
        assert command.count(f"{SOURCE}:130") == 1
        if mode == "candidate":
            assert provenance["candidate"] == str(SNAPSHOT_PACKAGE)
            assert provenance["candidateProfile"] == PROFILE
            assert provenance["candidateArtifactSha256"] == ARTIFACT_SHA256
            assert provenance["candidateSourceSha256"] == SNAPSHOT_SOURCE_SHA256
        else:
            assert provenance["candidate"] is None
        selected = selected_tests(json.loads(output["result"].read_text()), project)
        assert selected == run["selected"]
        assert run["exitCode"] == (
            0 if all(item["status"] == "expected" for item in selected) else 1
        )
        if mode == "oracle":
            assert all(item["status"] == "expected" for item in selected)
        assert ("[pwright-candidate]" in output["log"].read_text()) == (mode == "candidate")
        checked_receipts(output["receipt"], mode, selected, host, bind)
        for item in selected:
            assert item["specId"] not in all_ids
            all_ids.add(item["specId"])
    assert all_ids == {spec_id for selection in PROJECTS.values() for spec_id in selection["specs"]}
    assert len(all_ids) == 6
    return strict


def verify_pair(oracle_path: Path, candidate_path: Path) -> tuple[dict[str, Any], dict[str, Any]]:
    oracle = verify_strict(oracle_path)
    candidate = verify_strict(candidate_path)
    assert oracle["mode"] == "oracle" and candidate["mode"] == "candidate"
    assert oracle["upstreamInputs"] == candidate["upstreamInputs"]
    assert oracle["inputs"] == candidate["inputs"]
    assert oracle["fixtureDnsPreflight"] == candidate["fixtureDnsPreflight"]
    for oracle_run, candidate_run in zip(oracle["runs"], candidate["runs"], strict=True):
        assert oracle_run["project"] == candidate_run["project"]
        assert oracle_run["suite"] == candidate_run["suite"]
        assert [item["specId"] for item in oracle_run["selected"]] == [
            item["specId"] for item in candidate_run["selected"]
        ]
    return oracle, candidate


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("strict_sidecar", type=Path)
    parser.add_argument("candidate_sidecar", type=Path, nargs="?")
    args = parser.parse_args()
    try:
        if args.candidate_sidecar is None:
            strict = verify_strict(args.strict_sidecar.absolute())
            print(
                f"verified Core Bidi Close {strict['mode']}: "
                f"{[item['status'] for run in strict['runs'] for item in run['selected']]}"
            )
        else:
            oracle, candidate = verify_pair(
                args.strict_sidecar.absolute(), args.candidate_sidecar.absolute()
            )
            oracle_statuses = [item["status"] for run in oracle["runs"] for item in run["selected"]]
            candidate_statuses = [
                item["status"] for run in candidate["runs"] for item in run["selected"]
            ]
            print(
                "verified Core Bidi Close pair: "
                f"oracle={oracle_statuses}, candidate={candidate_statuses}"
            )
    except (
        AssertionError,
        KeyError,
        OSError,
        TypeError,
        ValueError,
        json.JSONDecodeError,
    ) as error:
        parser.error(f"strict Core Bidi Close evidence invalid: {error}")


if __name__ == "__main__":
    main()
