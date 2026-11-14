#!/usr/bin/env python3
"""Verify a source-pinned, diagnostic-only Core Server-connect Close run."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any

import run_playwright_core_server_connect_close_error_strict_v5 as capture
from official_candidate_package_snapshot_v1 import sha256, verify_snapshot
from verify_playwright_official_provenance import verify_provenance

EXPECTED_FIELDS = {
    "schema",
    "scope",
    "sourceClassification",
    "fixtureAdaptation",
    "unadaptedOfficialPass",
    "adaptedSourceSha256",
    "fixtureDnsPreflight",
    "publicApiParityEvidence",
    "promotedToCoreLedger",
    "mode",
    "upstreamCommit",
    "upstreamSourceSha256",
    "serverFixtureSha256",
    "testModeFixtureSha256",
    "inputs",
    "candidateProfile",
    "candidateArtifactSha256",
    "candidateSnapshot",
    "candidateSnapshotManifestSha256",
    "candidateSourceSha256",
    "result",
    "resultSha256",
    "provenanceSha256",
    "guardReceiptSha256",
    "logSha256",
    "exitCode",
    "selected",
}


def _regular(path: Path) -> None:
    assert path.is_file() and not path.is_symlink()


def checked_receipts(
    path: Path, mode: str, selected: list[dict[str, Any]], host: str, bind: str
) -> None:
    _regular(path)
    records = [json.loads(line) for line in path.read_text().splitlines() if line]
    assert records and all(isinstance(record, dict) for record in records)
    assert [item["kind"] for item in selected] == ["launchServer", "run-server"]
    workers = [str(item["attempts"][0]["workerIndex"]) for item in selected]
    assert all(re.fullmatch(r"\d+", worker) and int(worker) <= 16 for worker in workers)
    allowed_workers = set(workers)
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
    for record in records:
        assert record["kind"] in details
        assert set(record) == common | details[record["kind"]]
        assert isinstance(record["pid"], int) and record["pid"] > 0
        assert record["mode"] == mode
        assert record["sourceSha256"] == capture.SOURCE_SHA256
        assert record["adaptedSourceSha256"] == capture.ADAPTED_SOURCE_SHA256
        assert record["fixtureHost"] == host and record["fixtureBind"] == bind
        assert record["ownerBound"] == (True if mode == "candidate" else None)
        assert record["cdpEndpointPresent"] is False
        assert record["sourceAdapted"] is True
        assert record["fixtureAdapted"] is True
        assert record["workerIndex"] is None or record["workerIndex"] in allowed_workers
        if record["kind"] in {"preload", "source-read"}:
            assert record["replacements"] == 1
            continue
        worker = record["workerIndex"]
        assert worker in allowed_workers
        port = capture.SERVER_PORT_BASE + int(worker) * capture.SERVER_PORT_STRIDE
        assert record["port"] == record["listenerPort"] == port
        assert record["originalUrl"] == f"ws://localhost:{port}/ws"
        assert record["mappedUrl"] == f"ws://{host}:{port}/ws"
        assert record["listenerAddress"] == bind
        assert record["listenerFamily"] == "IPv4"
        assert record["requestHost"] == f"{host}:{port}"
        if record["kind"] == "ws-message":
            assert isinstance(record["messageBytes"], int)
            assert record["messageBytes"] > 0
        if record["kind"] == "ws-close":
            assert (record["code"], record["reason"]) == (4123, "Oh my!")

    ws_events = [record for record in records if record["kind"].startswith("ws-")]
    assert len(ws_events) == 6
    for index, item in enumerate(selected):
        triplet = ws_events[index * 3 : (index + 1) * 3]
        assert [record["kind"] for record in triplet] == [
            "ws-upgrade",
            "ws-message",
            "ws-close",
        ]
        worker = workers[index]
        assert all(record["workerIndex"] == worker for record in triplet)
        pid = triplet[0]["pid"]
        assert all(record["pid"] == pid for record in triplet)
        assert any(
            record["kind"] == "preload" and record["pid"] == pid for record in records
        )
        assert any(
            record["kind"] == "source-read"
            and record["pid"] == pid
            and record["workerIndex"] == worker
            for record in records
        )


def verify_strict(strict_path: Path) -> dict[str, Any]:
    _regular(strict_path)
    strict = json.loads(strict_path.read_text())
    assert isinstance(strict, dict) and set(strict) == EXPECTED_FIELDS
    assert strict["schema"] == 1 and strict["scope"] == capture.SCOPE
    assert (
        strict["sourceClassification"]
        == "official-derived-fixture-host-adapted-internal"
    )
    assert strict["fixtureAdaptation"] == capture.FIXTURE_ADAPTATION
    assert strict["unadaptedOfficialPass"] is False
    assert strict["adaptedSourceSha256"] == capture.ADAPTED_SOURCE_SHA256
    dns_preflight = strict["fixtureDnsPreflight"]
    assert isinstance(dns_preflight, dict) and set(dns_preflight) == {
        "fixtureHost",
        "fixtureBind",
        "resolvedIPv4",
    }
    assert dns_preflight["resolvedIPv4"] == [dns_preflight["fixtureBind"]]
    assert strict["publicApiParityEvidence"] is False
    assert strict["promotedToCoreLedger"] is False
    mode = strict["mode"]
    assert mode in {"oracle", "candidate"}
    assert strict["upstreamCommit"] == capture.COMMIT
    assert strict["upstreamSourceSha256"] == capture.SOURCE_SHA256
    assert strict["serverFixtureSha256"] == capture.SERVER_FIXTURE_SHA256
    assert strict["testModeFixtureSha256"] == capture.TEST_MODE_FIXTURE_SHA256
    capture.check_source_and_manifest()
    inputs = strict["inputs"]
    assert isinstance(inputs, dict) and set(inputs) == {
        str(path.relative_to(capture.ROOT)) for path in capture.INPUT_FILES
    }
    assert all(
        capture.sha256(capture.ROOT / name) == digest for name, digest in inputs.items()
    )
    result = capture.RESULT_PARENT / f"{mode}-v5.json"
    assert strict["result"] == str(result)
    assert strict_path == result.with_suffix(".strict.json")
    provenance_path = result.with_suffix(".provenance.json")
    receipt_path = result.with_suffix(".guard-receipt.jsonl")
    log_path = result.with_suffix(".log")
    for path in (result, provenance_path, receipt_path, log_path):
        _regular(path)
    for key, path in (
        ("resultSha256", result),
        ("provenanceSha256", provenance_path),
        ("guardReceiptSha256", receipt_path),
        ("logSha256", log_path),
    ):
        assert strict[key] == sha256(path)
    if mode == "candidate":
        assert strict["candidateProfile"] == capture.PROFILE
        assert strict["candidateArtifactSha256"] == capture.ARTIFACT_SHA256
        assert strict["candidateSnapshot"] == str(capture.SNAPSHOT_PACKAGE)
        _regular(capture.SNAPSHOT_MANIFEST)
        assert strict["candidateSnapshotManifestSha256"] == sha256(
            capture.SNAPSHOT_MANIFEST
        )
        snapshot = verify_snapshot(
            capture.SNAPSHOT_PACKAGE,
            capture.SNAPSHOT_MANIFEST,
            source_package=capture.SOURCE_PACKAGE,
            profile=capture.PROFILE,
            artifact_sha256=capture.ARTIFACT_SHA256,
        )
        assert strict["candidateSourceSha256"] == snapshot["candidateSourceSha256"]
    else:
        assert strict["candidateProfile"] is None
        assert strict["candidateArtifactSha256"] is None
        assert strict["candidateSnapshot"] is None
        assert strict["candidateSnapshotManifestSha256"] is None
        assert strict["candidateSourceSha256"] is None
    provenance = verify_provenance(
        json.loads(provenance_path.read_text()),
        provenance_path=provenance_path,
        manifest_override=capture.MANIFEST,
        result_override=result,
        upstream_override=capture.UPSTREAM,
        candidate_override=capture.SNAPSHOT_PACKAGE if mode == "candidate" else None,
        tooling_root=capture.ROOT,
    )
    assert provenance["mode"] == mode and provenance["suite"] == capture.SUITE
    assert provenance["project"] == "chromium-library" and provenance["workers"] == 1
    assert provenance["filters"] == [f"{capture.SOURCE}:130"]
    assert provenance["fixtureHost"] == dns_preflight["fixtureHost"]
    assert provenance["fixtureBind"] == dns_preflight["fixtureBind"]
    assert provenance["headed"] is False
    assert provenance["fixtureHost"] and re.fullmatch(
        r"[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*\.ts\.net",
        provenance["fixtureHost"],
    )
    assert provenance["resultSha256"] == strict["resultSha256"]
    assert provenance["exitCode"] == strict["exitCode"]
    if mode == "candidate":
        assert provenance["candidate"] == str(capture.SNAPSHOT_PACKAGE)
        assert provenance["candidateProfile"] == capture.PROFILE
        assert provenance["candidateArtifactSha256"] == capture.ARTIFACT_SHA256
        assert provenance["candidateSourceSha256"] == strict["candidateSourceSha256"]
    else:
        assert provenance["candidate"] is None
    selected = capture.selected_tests(json.loads(result.read_text()))
    assert selected == strict["selected"]
    assert all(item["status"] == "expected" for item in selected)
    assert strict["exitCode"] == (
        0 if all(item["status"] == "expected" for item in selected) else 1
    )
    if mode == "oracle":
        assert all(item["status"] == "expected" for item in selected)
    assert ("[pwright-candidate]" in log_path.read_text()) == (mode == "candidate")
    checked_receipts(
        receipt_path,
        mode,
        selected,
        provenance["fixtureHost"],
        provenance["fixtureBind"],
    )
    return strict


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("strict_sidecar", type=Path)
    args = parser.parse_args()
    try:
        strict = verify_strict(args.strict_sidecar.absolute())
    except (
        AssertionError,
        KeyError,
        OSError,
        TypeError,
        ValueError,
        json.JSONDecodeError,
    ) as error:
        parser.error(f"strict Core Server-connect Close evidence invalid: {error}")
    print(
        f"verified Core Server-connect Close diagnostic {strict['mode']}: "
        f"{[item['status'] for item in strict['selected']]}"
    )


if __name__ == "__main__":
    main()
