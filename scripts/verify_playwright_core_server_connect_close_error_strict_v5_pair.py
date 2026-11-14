#!/usr/bin/env python3
"""Verify symmetric candidate/oracle fixture adaptation for the two Close cases."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import run_playwright_core_server_connect_close_error_strict_v5 as capture
from verify_playwright_core_server_connect_close_error_strict_v5 import verify_strict


def mapping_rows(strict: dict[str, Any]) -> list[dict[str, Any]]:
    receipt = Path(strict["result"]).with_suffix(".guard-receipt.jsonl")
    records = [json.loads(line) for line in receipt.read_text().splitlines() if line]
    upgrades = [record for record in records if record.get("kind") == "ws-upgrade"]
    assert len(upgrades) == len(strict["selected"]) == 2
    rows = []
    for item, record in zip(strict["selected"], upgrades, strict=True):
        worker = str(item["attempts"][0]["workerIndex"])
        assert record["workerIndex"] == worker
        rows.append(
            {
                "specId": item["specId"],
                "kind": item["kind"],
                "workerIndex": worker,
                "port": record["port"],
                "originalUrl": record["originalUrl"],
                "mappedUrl": record["mappedUrl"],
                "listenerAddress": record["listenerAddress"],
                "listenerPort": record["listenerPort"],
                "requestHost": record["requestHost"],
            }
        )
    return rows


def compare_verified_pair(
    candidate: dict[str, Any],
    oracle: dict[str, Any],
    candidate_mappings: list[dict[str, Any]],
    oracle_mappings: list[dict[str, Any]],
) -> dict[str, Any]:
    assert candidate["mode"] == "candidate" and oracle["mode"] == "oracle"
    assert candidate["scope"] == oracle["scope"] == capture.SCOPE
    assert (
        candidate["fixtureAdaptation"]
        == oracle["fixtureAdaptation"]
        == capture.FIXTURE_ADAPTATION
    )
    assert (
        candidate["sourceClassification"]
        == oracle["sourceClassification"]
        == ("official-derived-fixture-host-adapted-internal")
    )
    assert (
        candidate["unadaptedOfficialPass"] is oracle["unadaptedOfficialPass"] is False
    )
    assert candidate["upstreamCommit"] == oracle["upstreamCommit"] == capture.COMMIT
    assert (
        candidate["upstreamSourceSha256"]
        == oracle["upstreamSourceSha256"]
        == capture.SOURCE_SHA256
    )
    assert (
        candidate["adaptedSourceSha256"]
        == oracle["adaptedSourceSha256"]
        == capture.ADAPTED_SOURCE_SHA256
    )
    assert candidate["fixtureDnsPreflight"] == oracle["fixtureDnsPreflight"]
    assert candidate["inputs"] == oracle["inputs"]
    for strict in (candidate, oracle):
        assert [item["kind"] for item in strict["selected"]] == [
            "launchServer",
            "run-server",
        ]
        assert {item["specId"] for item in strict["selected"]} == set(
            capture.EXPECTED_SPEC_IDS
        )
        assert all(item["status"] == "expected" for item in strict["selected"])
    assert candidate["candidateProfile"] == capture.PROFILE
    assert candidate["candidateArtifactSha256"] == capture.ARTIFACT_SHA256
    assert candidate["candidateSnapshot"] is not None
    assert candidate["candidateSourceSha256"] is not None
    assert oracle["candidateProfile"] is None
    assert oracle["candidateArtifactSha256"] is None
    assert oracle["candidateSnapshot"] is None
    assert oracle["candidateSourceSha256"] is None
    for strict, mappings in (
        (candidate, candidate_mappings),
        (oracle, oracle_mappings),
    ):
        assert len(mappings) == 2
        host = strict["fixtureDnsPreflight"]["fixtureHost"]
        bind = strict["fixtureDnsPreflight"]["fixtureBind"]
        for item, row in zip(strict["selected"], mappings, strict=True):
            assert set(row) == {
                "specId",
                "kind",
                "workerIndex",
                "port",
                "originalUrl",
                "mappedUrl",
                "listenerAddress",
                "listenerPort",
                "requestHost",
            }
            worker = str(item["attempts"][0]["workerIndex"])
            port = capture.SERVER_PORT_BASE + int(worker) * capture.SERVER_PORT_STRIDE
            assert row == {
                "specId": item["specId"],
                "kind": item["kind"],
                "workerIndex": worker,
                "port": port,
                "originalUrl": f"ws://localhost:{port}/ws",
                "mappedUrl": f"ws://{host}:{port}/ws",
                "listenerAddress": bind,
                "listenerPort": port,
                "requestHost": f"{host}:{port}",
            }
    assert candidate_mappings == oracle_mappings
    assert len(candidate_mappings) == 2
    return {
        "scope": capture.SCOPE,
        "classification": "official-derived-fixture-host-adapted-internal",
        "sourceSha256": capture.SOURCE_SHA256,
        "adaptedSourceSha256": capture.ADAPTED_SOURCE_SHA256,
        "fixtureDnsPreflight": candidate["fixtureDnsPreflight"],
        "selectedSpecIds": sorted(capture.EXPECTED_SPEC_IDS),
        "mappings": candidate_mappings,
        "candidateOrigin": candidate["candidateSnapshot"],
        "oracleOrigin": "upstream-playwright-core",
        "publicApiParityEvidence": False,
    }


def verify_pair(candidate_path: Path, oracle_path: Path) -> dict[str, Any]:
    candidate = verify_strict(candidate_path)
    oracle = verify_strict(oracle_path)
    return compare_verified_pair(
        candidate,
        oracle,
        mapping_rows(candidate),
        mapping_rows(oracle),
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("candidate_strict", type=Path)
    parser.add_argument("oracle_strict", type=Path)
    args = parser.parse_args()
    try:
        pair = verify_pair(
            args.candidate_strict.absolute(), args.oracle_strict.absolute()
        )
    except (
        AssertionError,
        KeyError,
        OSError,
        TypeError,
        ValueError,
        json.JSONDecodeError,
    ) as error:
        parser.error(f"fixture-host-adapted Close pair invalid: {error}")
    print(
        "verified fixture-host-adapted Close diagnostic pair: "
        f"{len(pair['selectedSpecIds'])} cases, two isolated origins"
    )


if __name__ == "__main__":
    main()
