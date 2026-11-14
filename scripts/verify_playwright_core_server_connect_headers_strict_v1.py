"""Independently verify the pinned Core connect-header diagnostic lanes."""

from __future__ import annotations

import argparse
import json
import re
from collections import Counter
from pathlib import Path
from typing import Any

import run_playwright_core_server_connect_headers_strict_v1 as capture
from official_candidate_package_snapshot_v1 import sha256, verify_snapshot
from verify_playwright_official_provenance import verify_provenance

EXPECTED_FIELDS = {
    "schema",
    "scope",
    "sourceClassification",
    "fixtureAdaptation",
    "wireAttribution",
    "wireReceiptScope",
    "unadaptedOfficialPass",
    "publicApiParityEvidence",
    "promotedToCoreLedger",
    "mode",
    "upstreamCommit",
    "upstreamSourceSha256",
    "adaptedSourceSha256",
    "serverFixtureSha256",
    "testModeFixtureSha256",
    "testServerSourceSha256",
    "userAgentJsSha256",
    "expectedDefaultUserAgent",
    "fixtureDnsPreflight",
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
COMMON_RECEIPT_FIELDS = {
    "pid",
    "workerIndex",
    "mode",
    "fixtureHost",
    "fixtureBind",
    "ownerBound",
    "cdpEndpointPresent",
    "event",
}


def _regular(path: Path) -> None:
    assert path.is_file() and not path.is_symlink()


def checked_receipts(
    path: Path,
    mode: str,
    selected: list[dict[str, Any]],
    host: str,
    bind: str,
    expected_user_agent: str,
    adapted_sha256: str,
) -> None:
    _regular(path)
    records = [json.loads(line) for line in path.read_text().splitlines() if line]
    assert records and all(isinstance(record, dict) for record in records)
    selected_workers = Counter(
        (str(item["attempts"][0]["workerIndex"]), item["headerCase"])
        for item in selected
    )
    source_workers: set[str] = set()
    upgrade_workers: Counter[tuple[str, str]] = Counter()
    upgrade_order: list[str] = []
    request_ordinals: Counter[str] = Counter()
    for record in records:
        assert isinstance(record["pid"], int) and record["pid"] > 0
        assert record["mode"] == mode
        assert record["fixtureHost"] == host and record["fixtureBind"] == bind
        assert record["ownerBound"] == (True if mode == "candidate" else None)
        assert record["cdpEndpointPresent"] is False
        if record["event"] == "source-adapted":
            assert set(record) == COMMON_RECEIPT_FIELDS | {
                "sourceSha256",
                "adaptedSha256",
                "replacements",
            }
            assert record["sourceSha256"] == capture.SOURCE_SHA256
            assert record["adaptedSha256"] == adapted_sha256
            assert record["replacements"] == 2
            if record["workerIndex"] is not None:
                source_workers.add(str(record["workerIndex"]))
            continue
        assert record["event"] == "fixture-upgrade-received"
        assert set(record) == COMMON_RECEIPT_FIELDS | {
            "method",
            "upgradeOrdinal",
            "url",
            "localAddress",
            "localPort",
            "assertedHeaders",
            "rawAssertedHeaders",
            "protocolHeaderCounts",
        }
        worker = record["workerIndex"]
        assert isinstance(worker, str) and worker.isdecimal()
        request_ordinals[worker] += 1
        assert record["upgradeOrdinal"] == request_ordinals[worker]
        assert record["method"] == "GET" and record["url"] == "/ws"
        assert record["localAddress"] == bind
        assert record["localPort"] == 8907 + int(worker) * 4
        headers = record["assertedHeaders"]
        raw_values = record["rawAssertedHeaders"]
        protocol_counts = record["protocolHeaderCounts"]
        asserted_names = {"host", "user-agent", "foo", "x-playwright-browser"}
        assert isinstance(headers, dict) and set(headers) == asserted_names
        assert isinstance(raw_values, dict) and set(raw_values) == asserted_names
        assert protocol_counts == {
            "connection": 1,
            "upgrade": 1,
            "sec-websocket-version": 1,
            "sec-websocket-key": 1,
        }
        for name in ("host", "user-agent", "foo"):
            assert isinstance(headers[name], str)
            assert raw_values[name] == [headers[name]]
        assert headers["x-playwright-browser"] is None or isinstance(
            headers["x-playwright-browser"], str
        )
        assert raw_values["x-playwright-browser"] == (
            []
            if headers["x-playwright-browser"] is None
            else [headers["x-playwright-browser"]]
        )
        assert headers["host"] == f"{host}:{record['localPort']}"
        assert headers["foo"] == "bar"
        header_case = "extra" if headers["user-agent"] == "Playwright" else "default"
        if header_case == "default":
            assert headers["user-agent"] == expected_user_agent
            assert headers.get("x-playwright-browser") == "chromium"
            assert raw_values["x-playwright-browser"] == ["chromium"]
        upgrade_workers[(worker, header_case)] += 1
        upgrade_order.append(header_case)
    assert {worker for worker, _ in selected_workers} <= source_workers
    assert upgrade_workers == selected_workers
    assert upgrade_order == ["extra", "default", "extra", "default"]


def verify_strict(strict_path: Path) -> dict[str, Any]:
    _regular(strict_path)
    strict = json.loads(strict_path.read_text())
    assert isinstance(strict, dict) and set(strict) == EXPECTED_FIELDS
    assert strict["schema"] == 1 and strict["scope"] == capture.SCOPE
    assert strict["sourceClassification"] == capture.CLASSIFICATION
    assert strict["fixtureAdaptation"] == capture.FIXTURE_ADAPTATION
    assert strict["wireAttribution"] == (
        "ordered-upgrades-and-header-case-counts-no-per-spec-socket-id"
    )
    assert (
        strict["wireReceiptScope"]
        == "asserted-header-values-and-protocol-name-counts-v1"
    )
    assert strict["unadaptedOfficialPass"] is False
    assert strict["publicApiParityEvidence"] is False
    assert strict["promotedToCoreLedger"] is False
    mode = strict["mode"]
    assert mode in {"oracle", "candidate"}
    assert strict["upstreamCommit"] == capture.COMMIT
    assert strict["upstreamSourceSha256"] == capture.SOURCE_SHA256
    assert strict["adaptedSourceSha256"] == capture.adapted_source_sha256()
    assert strict["serverFixtureSha256"] == capture.SERVER_FIXTURE_SHA256
    assert strict["testModeFixtureSha256"] == capture.TEST_MODE_FIXTURE_SHA256
    assert strict["testServerSourceSha256"] == capture.TEST_SERVER_SHA256
    capture.check_source_and_manifest()
    assert strict["userAgentJsSha256"] == sha256(
        capture.UPSTREAM / capture.USER_AGENT_JS
    )
    assert strict["expectedDefaultUserAgent"] == capture.reference_user_agent()
    inputs = strict["inputs"]
    assert isinstance(inputs, dict) and set(inputs) == {
        str(path.relative_to(capture.ROOT)) for path in capture.INPUT_FILES
    }
    assert all(
        capture.sha256(capture.ROOT / name) == digest for name, digest in inputs.items()
    )
    result = strict_path.parent / f"{mode}-v1.json"
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
    snapshot_package = result.parent / "candidate-package-snapshot-v1"
    snapshot_manifest = result.parent / "candidate-package-snapshot-v1.files.json"
    if mode == "candidate":
        assert strict["candidateProfile"] == capture.PROFILE
        assert strict["candidateArtifactSha256"] == capture.ARTIFACT_SHA256
        assert strict["candidateSnapshot"] == str(snapshot_package)
        _regular(snapshot_manifest)
        assert strict["candidateSnapshotManifestSha256"] == sha256(snapshot_manifest)
        snapshot = verify_snapshot(
            snapshot_package,
            snapshot_manifest,
            source_package=capture.SOURCE_PACKAGE,
            profile=capture.PROFILE,
            artifact_sha256=capture.ARTIFACT_SHA256,
        )
        assert strict["candidateSourceSha256"] == snapshot["candidateSourceSha256"]
    else:
        for key in (
            "candidateProfile",
            "candidateArtifactSha256",
            "candidateSnapshot",
            "candidateSnapshotManifestSha256",
            "candidateSourceSha256",
        ):
            assert strict[key] is None
    provenance = verify_provenance(
        json.loads(provenance_path.read_text()),
        provenance_path=provenance_path,
        manifest_override=capture.MANIFEST,
        result_override=result,
        upstream_override=capture.UPSTREAM,
        candidate_override=snapshot_package if mode == "candidate" else None,
        tooling_root=capture.ROOT,
    )
    assert provenance["mode"] == mode and provenance["suite"] == capture.SUITE
    assert provenance["project"] == "chromium-library" and provenance["workers"] == 1
    assert provenance["filters"] == [f"{capture.SOURCE}:235", f"{capture.SOURCE}:250"]
    assert provenance["headed"] is False
    assert provenance["suiteRole"] == "browser-independent-fixture-adapted-diagnostic"
    dns = strict["fixtureDnsPreflight"]
    assert isinstance(dns, dict) and set(dns) == {
        "fixtureHost",
        "fixtureBind",
        "resolvedIPv4",
    }
    assert dns["fixtureHost"] == provenance["fixtureHost"]
    assert dns["fixtureBind"] == provenance["fixtureBind"]
    assert dns["resolvedIPv4"] == [dns["fixtureBind"]]
    assert re.fullmatch(
        r"[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*\.ts\.net", dns["fixtureHost"]
    )
    assert re.fullmatch(
        r"100\.(?:6[4-9]|[7-9][0-9]|1[01][0-9]|12[0-7])\.(?:[0-9]{1,3})\.(?:[0-9]{1,3})",
        dns["fixtureBind"],
    )
    assert provenance["resultSha256"] == strict["resultSha256"]
    assert provenance["exitCode"] == strict["exitCode"]
    if mode == "candidate":
        assert provenance["candidate"] == str(snapshot_package)
        assert provenance["candidateProfile"] == capture.PROFILE
        assert provenance["candidateArtifactSha256"] == capture.ARTIFACT_SHA256
        assert provenance["candidateSourceSha256"] == strict["candidateSourceSha256"]
    else:
        assert provenance["candidate"] is None
    selected = capture.selected_tests(json.loads(result.read_text()))
    assert selected == strict["selected"]
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
        dns["fixtureHost"],
        dns["fixtureBind"],
        strict["expectedDefaultUserAgent"],
        strict["adaptedSourceSha256"],
    )
    return strict


def verify_pair(
    oracle_path: Path, candidate_path: Path
) -> tuple[dict[str, Any], dict[str, Any]]:
    oracle = verify_strict(oracle_path.absolute())
    candidate = verify_strict(candidate_path.absolute())
    assert oracle["mode"] == "oracle" and candidate["mode"] == "candidate"
    assert oracle_path.absolute().parent == candidate_path.absolute().parent
    for key in (
        "scope",
        "sourceClassification",
        "fixtureAdaptation",
        "wireAttribution",
        "wireReceiptScope",
        "upstreamCommit",
        "upstreamSourceSha256",
        "adaptedSourceSha256",
        "serverFixtureSha256",
        "testModeFixtureSha256",
        "testServerSourceSha256",
        "userAgentJsSha256",
        "expectedDefaultUserAgent",
        "fixtureDnsPreflight",
        "inputs",
    ):
        assert oracle[key] == candidate[key]
    assert all(item["status"] == "expected" for item in oracle["selected"])
    assert all(item["status"] == "expected" for item in candidate["selected"])
    assert [item["specId"] for item in oracle["selected"]] == [
        item["specId"] for item in candidate["selected"]
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
                f"verified Core connect headers adapted diagnostic {strict['mode']}: "
                f"{[item['status'] for item in strict['selected']]}"
            )
        else:
            oracle, candidate = verify_pair(args.strict_sidecar, args.candidate_sidecar)
            print(
                "verified Core connect headers adapted diagnostic pair: "
                f"oracle={len(oracle['selected'])} candidate={len(candidate['selected'])}"
            )
    except (
        AssertionError,
        KeyError,
        OSError,
        TypeError,
        ValueError,
        json.JSONDecodeError,
    ) as error:
        parser.error(f"strict Core connect headers adapted evidence invalid: {error}")


if __name__ == "__main__":
    main()
