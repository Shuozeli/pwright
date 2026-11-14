"""Verify the immutable post-refactor Core connect-header evidence pair."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import run_playwright_core_server_connect_headers_post_refactor_v2 as capture
import verify_playwright_core_server_connect_headers_strict_v1 as base

ORIGINAL_CHECKED_RECEIPTS = base.checked_receipts


def require_abi_shape(probe: dict) -> None:
    assert probe == {"enabled": True, "shape": 2, "port": "function"}


def checked_receipts_v2(
    path: Path,
    mode: str,
    selected: list[dict],
    host: str,
    bind: str,
    expected_user_agent: str,
    adapted_sha256: str,
) -> None:
    ORIGINAL_CHECKED_RECEIPTS(
        path, mode, selected, host, bind, expected_user_agent, adapted_sha256
    )
    records = [json.loads(line) for line in path.read_text().splitlines() if line]
    upgrades = [
        record for record in records if record["event"] == "fixture-upgrade-received"
    ]
    assert len(upgrades) == 4
    for record in upgrades:
        assert record["assertedHeaders"]["x-playwright-browser"] == "chromium"
        assert record["rawAssertedHeaders"]["x-playwright-browser"] == ["chromium"]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("oracle_sidecar", type=Path)
    parser.add_argument("candidate_sidecar", type=Path)
    args = parser.parse_args()
    capture.configure(check_live=False)
    base.EXPECTED_FIELDS = base.EXPECTED_FIELDS | {"abiShape"}
    base.checked_receipts = checked_receipts_v2
    try:
        oracle, candidate = base.verify_pair(
            args.oracle_sidecar, args.candidate_sidecar
        )
    except (
        AssertionError,
        KeyError,
        OSError,
        TypeError,
        ValueError,
        json.JSONDecodeError,
    ) as error:
        parser.error(f"post-refactor Core connect headers evidence invalid: {error}")
    assert oracle["scope"] == candidate["scope"] == capture.SCOPE
    assert oracle["abiShape"] == candidate["abiShape"] == capture.expected_abi_shape()
    assert candidate["candidateArtifactSha256"] == capture.ARTIFACT_SHA256
    snapshot = Path(candidate["candidateSnapshot"])
    assert base.sha256(snapshot / "src/index.cjs") == capture.CANDIDATE_INDEX_SHA256
    assert (
        base.sha256(snapshot / "src/native-artifact.cjs")
        == capture.CANDIDATE_NATIVE_ARTIFACT_SHA256
    )
    artifact = snapshot / "native" / capture.base.PROFILE / "pwright.node"
    assert base.sha256(artifact) == capture.ARTIFACT_SHA256
    require_abi_shape(capture.probe_artifact_abi(artifact))
    print(
        "verified post-refactor Core connect headers pair: "
        f"oracle={len(oracle['selected'])} candidate={len(candidate['selected'])}"
    )


if __name__ == "__main__":
    main()
