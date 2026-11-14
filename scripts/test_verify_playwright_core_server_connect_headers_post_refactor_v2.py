"""Clone-free controls for the post-refactor connect-header evidence gate."""

from __future__ import annotations

import json
import re

import pytest
import run_playwright_core_server_connect_headers_post_refactor_v2 as capture
import run_playwright_core_server_connect_headers_strict_v1 as core
import verify_playwright_core_server_connect_headers_post_refactor_v2 as verify
from test_verify_playwright_core_server_connect_headers_strict_v1 import (
    ADAPTED_DIGEST,
    BIND,
    DEFAULT_UA,
    HOST,
    records,
    report,
)


def receipts_with_browser_header() -> list[dict]:
    value = records("candidate")
    for record in value:
        if record["event"] == "fixture-upgrade-received":
            record["assertedHeaders"]["x-playwright-browser"] = "chromium"
            record["rawAssertedHeaders"]["x-playwright-browser"] = ["chromium"]
    return value


def check_receipts(tmp_path, value: list[dict]) -> None:
    path = tmp_path / "receipts.jsonl"
    path.write_text("".join(json.dumps(record) + "\n" for record in value))
    verify.checked_receipts_v2(
        path,
        "candidate",
        core.selected_tests(report()),
        HOST,
        BIND,
        DEFAULT_UA,
        ADAPTED_DIGEST,
    )


def test_v2_accepts_all_four_browser_headers(tmp_path) -> None:
    # Arrange
    value = receipts_with_browser_header()

    # Act
    check_receipts(tmp_path, value)

    # Assert
    assert (
        len([item for item in value if item["event"] == "fixture-upgrade-received"])
        == 4
    )


@pytest.mark.parametrize("change", ["missing-extra", "duplicate-raw", "wrong-extra"])
def test_v2_rejects_missing_or_mismatched_extra_browser_header(
    change, tmp_path
) -> None:
    # Arrange
    value = receipts_with_browser_header()
    extra = value[1]
    if change == "missing-extra":
        extra["assertedHeaders"]["x-playwright-browser"] = None
        extra["rawAssertedHeaders"]["x-playwright-browser"] = []
    elif change == "duplicate-raw":
        extra["rawAssertedHeaders"]["x-playwright-browser"] = ["chromium", "chromium"]
    else:
        extra["assertedHeaders"]["x-playwright-browser"] = "webkit"
        extra["rawAssertedHeaders"]["x-playwright-browser"] = ["webkit"]

    # Act / Assert
    with pytest.raises(AssertionError):
        check_receipts(tmp_path, value)


@pytest.mark.parametrize(
    "probe",
    [
        {"enabled": True, "shape": None, "port": "function"},
        {"enabled": True, "shape": 1, "port": "function"},
        {"enabled": False, "shape": 2, "port": "function"},
        {"enabled": True, "shape": 2, "port": "undefined"},
    ],
)
def test_v2_rejects_old_or_wrong_native_abi(probe: dict) -> None:
    # Arrange
    old_or_wrong = probe

    # Act / Assert
    with pytest.raises(AssertionError):
        verify.require_abi_shape(old_or_wrong)


def test_v2_abi_attestation_is_pinned_without_upstream_clone() -> None:
    # Arrange
    attestation = capture.expected_abi_shape()

    # Act
    artifact_sha = attestation["artifactSha256"]

    # Assert
    assert attestation["markerExport"] == "playwrightServerConnectHeadersAbiShape"
    assert attestation["markerValue"] == 2
    assert attestation["runtimeIdentityFields"] == ["platform", "arch", "release"]
    assert attestation["runtimeIdentityMaxUtf8Bytes"] == [64, 64, 128]
    assert re.fullmatch(r"[0-9a-f]{64}", artifact_sha)
    assert artifact_sha == capture.ARTIFACT_SHA256
    assert capture.RESULT_PARENT.name == "post-refactor-v2"
    assert capture.VERIFIER in capture.INPUT_FILES
    assert capture.TESTS in capture.INPUT_FILES
