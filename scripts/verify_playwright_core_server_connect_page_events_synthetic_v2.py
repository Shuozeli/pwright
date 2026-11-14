#!/usr/bin/env python3
"""Verify unchanged Core remote-waiter assertions and positive Page delivery.

The run-server launcher alone is adapted to a browser-free protocol fixture.
Passing this diagnostic does not credit the unadapted official Core ledger.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from collections import Counter
from pathlib import Path
from typing import Any

from official_candidate_package_snapshot_v1 import sha256, verify_snapshot
from run_playwright_official_conformance import _candidate_source_sha256
from verify_playwright_core_server_connect_waiters_synthetic_v1 import (
    COMMIT,
    EXPECTED_IDS,
    SOURCE_HASHES,
    UPSTREAM,
    _selected_tests,
)

ROOT = Path(__file__).parents[1].resolve()
SUITE = "library-chromium-server-connect-page-events-synthetic-v2"
PROFILE = "playwright-server-connect-page-events"
SOURCE_PACKAGE = ROOT / "packages/pwright"
ARTIFACT_SOURCE = SOURCE_PACKAGE / "native" / PROFILE / "pwright.node"
MANIFEST = ROOT / "tests/official/playwright-core-server-connect-page-events-synthetic-v2-manifest.json"
PRELOAD = ROOT / "tests/official/playwright-core-server-connect-page-events-synthetic-v2-preload.cjs"
SERVER = ROOT / "tests/official/playwright-core-server-connect-page-events-synthetic-v2-server.cjs"
POSITIVE = ROOT / "tests/official/playwright-core-server-connect-page-events-synthetic-v2-positive.cjs"
OFFICIAL_RUNNER = ROOT / "scripts/run_playwright_official_conformance.py"
SNAPSHOT_HELPER = ROOT / "scripts/official_candidate_package_snapshot_v1.py"
RUNNER = ROOT / "scripts/run_playwright_core_server_connect_page_events_synthetic_v2.py"
RESULT_PARENT = ROOT / ".work/official-conformance/results"
INPUTS = (
    MANIFEST,
    PRELOAD,
    SERVER,
    POSITIVE,
    ROOT / "tests/official/playwright-core-browser-type-owner-v2-preload.cjs",
    ROOT / "tests/official/playwright-core-server-runtime.cjs",
    ROOT / "tests/official/playwright-core-candidate-preload.cjs",
    OFFICIAL_RUNNER,
    ROOT / "scripts/official_result_destination.py",
    SNAPSHOT_HELPER,
    ROOT / "scripts/verify_playwright_core_server_connect_waiters_synthetic_v1.py",
    RUNNER,
    Path(__file__).resolve(),
)
EVENT_URL = "https://example.test/report"
DOWNLOAD_URL = "https://example.test/report.csv"
ENDPOINT_PATH = "/browser-synthetic-page-events-v2"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def read_json(path: Path) -> dict[str, Any]:
    require(path.is_file() and not path.is_symlink(), f"missing evidence file: {path}")
    value = json.loads(path.read_text())
    require(isinstance(value, dict), f"evidence is not an object: {path}")
    return value


def read_receipts(path: Path, mode: str, host: str, bind: str) -> list[dict[str, Any]]:
    require(path.is_file() and not path.is_symlink(), f"missing receipt: {path}")
    records = [json.loads(line) for line in path.read_text().splitlines() if line]
    require(records and all(isinstance(item, dict) for item in records), "invalid receipt")
    require(
        all(item.get("mode") == mode and item.get("fixtureHost") == host
            and item.get("fixtureBind") == bind for item in records),
        "receipt lane or Tailscale address differs",
    )
    return records


def verify_upstream() -> None:
    commit = subprocess.check_output(
        ["git", "-C", str(UPSTREAM), "rev-parse", "HEAD"], text=True
    ).strip()
    require(commit == COMMIT, "pinned Playwright commit changed")
    tracked = subprocess.check_output(
        ["git", "-C", str(UPSTREAM), "status", "--porcelain", "--untracked-files=no"],
        text=True,
    ).strip()
    require(not tracked, "pinned Playwright checkout has tracked changes")
    for relative, digest in SOURCE_HASHES.items():
        require(sha256(UPSTREAM / relative) == digest, f"upstream source drifted: {relative}")
    source = (UPSTREAM / "tests/library/browsertype-connect.spec.ts").read_text()
    for assertion in (
        "newPage.waitForRequest(server.EMPTY_PAGE).catch(e => e)",
        "newPage.waitForResponse(server.EMPTY_PAGE).catch(e => e)",
        "expect(message).toContain(kTargetClosedErrorMessage);",
        "expect(message).not.toContain('Timeout');",
        "newPage.waitForEvent('download').catch(() => rejected = true)",
        "expect(rejected).toBe(true);",
        "await expect.poll(() => log).toEqual(['rejected', 'disconnected']);",
    ):
        require(assertion in source, f"selected upstream assertion drifted: {assertion}")
    manifest = read_json(MANIFEST)
    require(
        manifest == {
            "schema": 1,
            "repository": "https://github.com/microsoft/playwright.git",
            "tag": "v1.58.0",
            "commit": COMMIT,
            "suites": [{
                "id": SUITE,
                "role": "browser-independent-fixture-adapted-diagnostic",
                "runner": "packages/playwright/cli.js",
                "config": "tests/library/playwright.config.ts",
                "args": [
                    f"tests/library/browsertype-connect.spec.ts:{line}"
                    for line in EXPECTED_IDS
                ],
                "candidateAdapter": "test-mode-playwright-core",
                "project": "chromium-library",
                "requiresBrowser": False,
                "expectedTestsPerProject": 3,
                "expectedFiles": 1,
            }],
        },
        "synthetic Page-events manifest changed",
    )


def _verify_official_receipt(
    path: Path, mode: str, host: str, bind: str
) -> dict[str, int]:
    records = read_receipts(path, mode, host, bind)
    preloads = [item for item in records if item.get("kind") == "preload"]
    require(preloads and any(item.get("workerIndex") == "0" for item in preloads),
            "selected test worker preload missing")
    require(all(
        item.get("upstreamSourceSha256") == SOURCE_HASHES["tests/library/browsertype-connect.spec.ts"]
        and item.get("upstreamFixtureSha256") == SOURCE_HASHES["tests/config/remoteServer.ts"]
        and item.get("testSourceAdapted") is False
        and item.get("fixtureAdapted") is True
        and item.get("cdpEndpointPresent") is False
        and item.get("ownerBound") is (True if mode == "candidate" else None)
        for item in preloads
    ), "selected test source or adapter identity changed")
    require(any(item.get("kind") == "fixture-source-read" and
                item.get("workerIndex") == "0" for item in records),
            "run-server launcher substitution did not reach worker")
    synthetic = [item for item in records if item.get("synthetic") is True]
    require(synthetic and all(item.get("browserProcessLaunched") is False
                              for item in synthetic), "fixture launched or claimed a browser")
    require(not any(item.get("kind") in {"start-error", "protocol-error", "websocket-error",
                                              "positive-trigger", "positive-event-sent"}
                    for item in synthetic), "official teardown fixture saw protocol or positive-event traffic")
    required = ("listening", "upgrade", "root-initialize", "bootstrap-sent",
                "new-context-replied", "new-page-replied", "close", "stop-requested", "stop")
    by_kind = {kind: [item for item in synthetic if item.get("kind") == kind]
               for kind in required}
    for kind, items in by_kind.items():
        require(len(items) == 3 and len({item.get("pid") for item in items}) == 3,
                f"selected cases did not use three isolated synthetic {kind} instances")
    require(all(item.get("endpointPath") == ENDPOINT_PATH and
                item.get("positiveProbeEnabled") is False
                for item in by_kind["listening"]), "upstream cases enabled event injection")
    for pid in {item["pid"] for item in by_kind["listening"]}:
        sequence = [item["kind"] for item in synthetic if item.get("pid") == pid]
        positions = [sequence.index(kind) for kind in required[:6]]
        require(positions == sorted(positions) and
                sequence.index("new-page-replied") < sequence.index("close") < sequence.index("stop")
                and sequence.index("stop-requested") < sequence.index("stop"),
                "synthetic Page creation or teardown order changed")
        close = next(item for item in by_kind["close"] if item["pid"] == pid)
        require(close.get("phase") == 3, "WebSocket closed before Page publication")
    if mode == "oracle":
        info = [item for item in synthetic if item.get("kind") == "wait-event-info-replied"]
        subscriptions = [item for item in synthetic if item.get("kind") == "subscription-replied"]
        require(Counter((item.get("phase"), item.get("event")) for item in info) == Counter({
            ("before", "request"): 1, ("before", "response"): 1,
            ("before", "download"): 2, ("log", None): 4,
        }), "oracle did not arm the selected network and download waiters")
        require(Counter((item.get("event"), item.get("enabled"))
                        for item in subscriptions) == Counter({
                            ("request", True): 1, ("response", True): 1,
                        }), "oracle request/response subscriptions were not acknowledged")
    return {kind: len(items) for kind, items in by_kind.items()}


def _verify_positive_receipt(
    path: Path, mode: str, host: str, bind: str
) -> dict[str, Any]:
    records = read_receipts(path, mode, host, bind)
    require(all(item.get("synthetic") is True and item.get("browserProcessLaunched") is False
                for item in records), "positive probe receipt contains foreign process")
    require(len({item.get("pid") for item in records}) == 1
            and all(item.get("connection", 1) == 1 for item in records),
            "positive probe mixed fixture processes or connections")
    require(not any(item.get("kind") in {"start-error", "protocol-error", "websocket-error"}
                    for item in records), "positive fixture had a protocol error")
    kinds = [item.get("kind") for item in records]
    for kind in ("listening", "upgrade", "root-initialize", "bootstrap-sent",
                 "new-context-replied", "new-page-replied", "positive-trigger",
                 "close", "stop-requested", "stop"):
        require(kinds.count(kind) == 1, f"positive probe missing one {kind}")
    require(kinds.index("new-page-replied") < kinds.index("positive-trigger")
            < kinds.index("close") < kinds.index("stop"),
            "positive event publication escaped Page lifetime")
    close = next(item for item in records if item.get("kind") == "close")
    require(close.get("phase") == 3, "positive fixture closed before Page publication")
    listening = next(item for item in records if item.get("kind") == "listening")
    require(listening.get("endpointPath") == ENDPOINT_PATH and
            listening.get("positiveProbeEnabled") is True,
            "positive fixture endpoint or injection gate changed")
    trigger = next(item for item in records if item.get("kind") == "positive-trigger")
    before_trigger = records[:kinds.index("positive-trigger")]
    acknowledgements = [item for item in before_trigger
                        if item.get("kind") == "subscription-replied"]
    require(Counter((item.get("guid"), item.get("event"), item.get("enabled"))
                    for item in acknowledgements) == Counter({
                        ("page@1", "request", True): 1,
                        ("page@1", "response", True): 1,
                    }), "Page request/response subscriptions were not acknowledged before publication")
    require(trigger.get("subscriptionsAcknowledged") == ["request", "response"]
            and kinds.index("subscription-replied") < kinds.index("positive-trigger"),
            "positive events were published before request/response subscription acknowledgements")
    if mode == "oracle":
        require(trigger.get("downloadWaitArmed") is True,
                "oracle positive events were published before download waiter arm")
        require(any(item.get("kind") == "wait-event-info-replied"
                    and item.get("phase") == "before" and item.get("event") == "download"
                    and item.get("guid") == "page@1" for item in before_trigger),
                "oracle download waiter arm receipt missing")
    events = [item for item in records if item.get("kind") == "positive-event-sent"]
    require([item.get("event") for item in events] == ["request", "response", "download"],
            "positive wire events are missing or out of order")
    request, response, download = events
    require(request.get("url") == EVENT_URL and request.get("method") == "POST"
            and request.get("requestGuid") == response.get("requestGuid")
            and response.get("url") == EVENT_URL and response.get("status") == 201
            and download.get("url") == DOWNLOAD_URL
            and download.get("suggestedFilename") == "report.csv"
            and all(item.get("pageGuid") == "page@1" for item in events),
            "positive request/response/download wire identities changed")
    return {
        "eventOrder": [item["event"] for item in events],
        "subscriptionsAcknowledgedBeforePublication": True,
        "oracleDownloadWaitArmedBeforePublication": mode == "oracle",
        "receiptSha256": sha256(path),
    }


def verify_lane(
    result_dir: Path, mode: str, snapshot_data: dict[str, Any]
) -> dict[str, Any]:
    result = result_dir / f"{mode}.json"
    provenance = result_dir / f"{mode}.provenance.json"
    receipt = result_dir / f"{mode}.official.receipt.jsonl"
    log = result_dir / f"{mode}.log"
    report = read_json(result)
    source = read_json(provenance)
    require(log.is_file() and not log.is_symlink(), "official lane log missing")
    expected_command = [
        "node", str(UPSTREAM / "packages/playwright/cli.js"), "test",
        "--config=tests/library/playwright.config.ts", "--reporter=line,json",
        "--workers=1", "--project=chromium-library", "--grep=run-server",
        *[f"tests/library/browsertype-connect.spec.ts:{line}" for line in EXPECTED_IDS],
    ]
    require(source.get("mode") == mode and source.get("suite") == SUITE
            and source.get("commit") == COMMIT
            and source.get("suiteRole") == "browser-independent-fixture-adapted-diagnostic"
            and source.get("manifestSha256") == sha256(MANIFEST)
            and source.get("resultSha256") == sha256(result)
            and source.get("command") == expected_command
            and source.get("project") == "chromium-library"
            and source.get("grep") == "run-server" and source.get("workers") == 1
            and source.get("exitCode") == 0,
            f"{mode} official runner provenance changed or did not pass")
    host = source.get("fixtureHost")
    bind = source.get("fixtureBind")
    require(isinstance(host, str) and re.fullmatch(r"[a-zA-Z0-9.-]+\.ts\.net", host)
            and isinstance(bind, str) and bind.startswith("100."),
            "official fixture did not bind to Tailscale")
    snapshot = result_dir / "candidate-package-snapshot"
    if mode == "candidate":
        require(source.get("candidate") == str(snapshot)
                and source.get("candidateProfile") == PROFILE
                and source.get("candidateArtifactSha256") == snapshot_data["artifactSha256"]
                and source.get("candidateSourceSha256") == snapshot_data["candidateSourceSha256"]
                and "[pwright-candidate]" in log.read_text(),
                "candidate lane did not load the selected frozen addon")
    else:
        require(source.get("candidate") is None and source.get("candidateProfile") is None
                and "[pwright-candidate]" not in log.read_text(),
                "oracle lane loaded Pwright")
    selected = _selected_tests(report, "oracle")
    official_wire = _verify_official_receipt(receipt, mode, host, bind)
    positive_result = read_json(result_dir / f"{mode}.positive.json")
    require(positive_result == {
        "mode": mode,
        "positiveEventDelivery": True,
        "waiterResolved": ["request", "response", "download"],
        "emitterOrder": ["request", "response", "download"],
        "request": {"url": EVENT_URL, "method": "POST"},
        "response": {"url": EVENT_URL, "status": 201, "requestIdentity": True},
        "download": {"url": DOWNLOAD_URL, "suggestedFilename": "report.csv"},
        "syntheticServerExitCode": 0,
    }, f"{mode} positive Page waiter and emitter observations changed")
    positive_log = result_dir / f"{mode}.positive.log"
    require(positive_log.is_file() and not positive_log.is_symlink()
            and not positive_log.read_text().strip(),
            "positive probe logged an error")
    positive_receipt = result_dir / f"{mode}.positive.receipt.jsonl"
    positive_wire = _verify_positive_receipt(positive_receipt, mode, host, bind)
    return {
        "selected": selected,
        "passed": 3,
        "failed": 0,
        "skipped": 0,
        "fixtureHost": host,
        "fixtureBind": bind,
        "officialResultSha256": sha256(result),
        "officialProvenanceSha256": sha256(provenance),
        "officialReceiptSha256": sha256(receipt),
        "officialLogSha256": sha256(log),
        "officialWireCounts": official_wire,
        "positiveResultSha256": sha256(result_dir / f"{mode}.positive.json"),
        "positiveLogSha256": sha256(positive_log),
        "positiveWire": positive_wire,
    }


def verify_directory(result_dir: Path) -> dict[str, Any]:
    result_dir = result_dir.absolute()
    require(result_dir.parent.resolve() == RESULT_PARENT.resolve()
            and result_dir.is_dir() and not result_dir.is_symlink()
            and result_dir.name.startswith("core-connect-page-events-synthetic-v2-green-"),
            "result directory is not a versioned Page-events capture")
    verify_upstream()
    snapshot = result_dir / "candidate-package-snapshot"
    snapshot_manifest = result_dir / "candidate-package-snapshot.files.json"
    snapshot_data = read_json(snapshot_manifest)
    artifact_digest = snapshot_data.get("artifactSha256")
    require(isinstance(artifact_digest, str) and re.fullmatch(r"[0-9a-f]{64}", artifact_digest),
            "snapshot addon digest missing")
    verify_snapshot(snapshot, snapshot_manifest, source_package=SOURCE_PACKAGE,
                    profile=PROFILE, artifact_sha256=artifact_digest)
    oracle = verify_lane(result_dir, "oracle", snapshot_data)
    candidate = verify_lane(result_dir, "candidate", snapshot_data)
    require(oracle["fixtureHost"] == candidate["fixtureHost"]
            and oracle["fixtureBind"] == candidate["fixtureBind"],
            "paired fixture addresses differ")
    inputs = {str(path.relative_to(ROOT)): sha256(path) for path in INPUTS}
    return {
        "schema": 2,
        "scope": "three-unchanged-core-connect-waiter-assertions-plus-positive-page-event-probe",
        "classification": "official-derived-fixture-adapted-internal-diagnostic",
        "publicApiParityEvidence": False,
        "unadaptedOfficialPass": False,
        "fullPlaywrightServerSupport": False,
        "positiveEventDeliveryEvidence": True,
        "candidateAssertionReached": True,
        "candidateWaitForResponseInvocationReached": True,
        "candidateTerminalOrderingEvidence": True,
        "promotedToCoreLedger": False,
        "upstreamCommit": COMMIT,
        "upstreamSourceSha256": SOURCE_HASHES,
        "manifestSha256": sha256(MANIFEST),
        "preloadSha256": sha256(PRELOAD),
        "syntheticServerSha256": sha256(SERVER),
        "positiveProbeSha256": sha256(POSITIVE),
        "candidateSnapshotManifestSha256": sha256(snapshot_manifest),
        "candidateSourceSha256": snapshot_data["candidateSourceSha256"],
        "candidateArtifactSource": str(ARTIFACT_SOURCE.relative_to(ROOT)),
        "candidateArtifactSha256": artifact_digest,
        "fixtureHost": oracle["fixtureHost"],
        "fixtureBind": oracle["fixtureBind"],
        "harnessInputs": inputs,
        "lanes": {"oracle": oracle, "candidate": candidate},
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("result_dir", type=Path)
    args = parser.parse_args()
    actual = verify_directory(args.result_dir)
    stored = read_json(args.result_dir / "attestation.json")
    require(stored == actual, "stored attestation differs from independently verified evidence")
    print("verified fixture-adapted Core Page events: oracle 3/3 + positive; candidate 3/3 + positive")
    print(f"selected addon SHA256: {actual['candidateArtifactSha256']}")


if __name__ == "__main__":
    main()
