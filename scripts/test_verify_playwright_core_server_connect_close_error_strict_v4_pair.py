"""Negative controls for the fixture-host-adapted Core Close pair proof."""

from __future__ import annotations

import pytest
import run_playwright_core_server_connect_close_error_strict_v4 as capture
import verify_playwright_core_server_connect_close_error_strict_v4_pair as pair


def sidecars() -> tuple[dict, dict, list[dict]]:
    selected = [
        {
            "kind": kind,
            "specId": spec_id,
            "status": "expected",
            "attempts": [{"workerIndex": index}],
        }
        for index, (spec_id, kind) in enumerate(capture.EXPECTED_SPEC_IDS.items())
    ]
    common = {
        "scope": capture.SCOPE,
        "fixtureAdaptation": capture.FIXTURE_ADAPTATION,
        "sourceClassification": "official-derived-fixture-host-adapted-internal",
        "unadaptedOfficialPass": False,
        "upstreamCommit": capture.COMMIT,
        "upstreamSourceSha256": capture.SOURCE_SHA256,
        "adaptedSourceSha256": capture.ADAPTED_SOURCE_SHA256,
        "fixtureDnsPreflight": {
            "fixtureHost": "example-yuacx.tail8f3b66.ts.net",
            "fixtureBind": "100.78.97.63",
            "resolvedIPv4": ["100.78.97.63"],
        },
        "inputs": {"v4-guard": "pinned"},
        "selected": selected,
    }
    candidate = {
        **common,
        "mode": "candidate",
        "candidateProfile": capture.PROFILE,
        "candidateArtifactSha256": capture.ARTIFACT_SHA256,
        "candidateSnapshot": "/candidate-package-snapshot-v1",
        "candidateSourceSha256": "a" * 64,
    }
    oracle = {
        **common,
        "mode": "oracle",
        "candidateProfile": None,
        "candidateArtifactSha256": None,
        "candidateSnapshot": None,
        "candidateSourceSha256": None,
    }
    mappings = [
        {"workerIndex": str(index), "port": capture.SERVER_PORT_BASE + 4 * index}
        for index in range(2)
    ]
    return candidate, oracle, mappings


def test_exact_two_isolated_mappings_are_compared() -> None:
    # Arrange
    candidate, oracle, mappings = sidecars()

    # Act
    result = pair.compare_verified_pair(candidate, oracle, mappings, mappings)

    # Assert
    assert result["publicApiParityEvidence"] is False
    assert result["mappings"] == mappings


@pytest.mark.parametrize(
    "tamper",
    ["adapted_sha", "fixture_dns", "mapping", "oracle_origin", "candidate_status"],
)
def test_asymmetric_or_unproven_pair_is_rejected(tamper: str) -> None:
    # Arrange
    candidate, oracle, mappings = sidecars()
    oracle_mappings = [dict(row) for row in mappings]
    if tamper == "adapted_sha":
        oracle["adaptedSourceSha256"] = "0" * 64
    elif tamper == "fixture_dns":
        oracle["fixtureDnsPreflight"] = {
            **oracle["fixtureDnsPreflight"],
            "fixtureBind": "100.78.97.64",
        }
    elif tamper == "mapping":
        oracle_mappings[0]["port"] += 4
    elif tamper == "oracle_origin":
        oracle["candidateSnapshot"] = "/candidate-package-snapshot-v1"
    else:
        oracle["selected"] = [dict(item) for item in oracle["selected"]]
        oracle["selected"][0]["status"] = "unexpected"

    # Act / Assert
    with pytest.raises(AssertionError):
        pair.compare_verified_pair(candidate, oracle, mappings, oracle_mappings)
