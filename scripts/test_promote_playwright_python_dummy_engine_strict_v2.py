"""Synthetic negative gates for the future v2 paired dummy-engine successor."""

from __future__ import annotations

import copy
import hashlib
import json
from collections import Counter
from pathlib import Path
from typing import Any

import promote_playwright_python_dummy_engine_strict_v2 as promotion
import pytest


def _write(path: Path, raw: bytes) -> str:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(raw)
    return hashlib.sha256(raw).hexdigest()


def _jsonl(rows: list[dict[str, Any]]) -> bytes:
    return b"".join((json.dumps(row, sort_keys=True) + "\n").encode() for row in rows)


@pytest.fixture
def red_baselines(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> dict[str, tuple[list[bytes], list[dict[str, Any]]]]:
    """A full-size but small synthetic ledger; never depend on ignored .work."""
    # Arrange
    inventory_path = tmp_path / "inventory.jsonl"
    base_dir = tmp_path / "paired-v1"
    inventory: list[dict[str, Any]] = []
    positions = dict(
        zip(
            promotion.TARGET_INDEXES,
            (
                promotion.target(api, browser)
                for api in promotion.APIS
                for browser in promotion.BROWSERS
            ),
        )
    )
    for index in range(promotion.RECORDS):
        inventory.append(
            {
                "id": positions.get(index, f"tests/synthetic.py::test_case_{index}"),
                "sourceClassification": "public-api-source"
                if index in positions
                else "unclassified",
            }
        )
    inventory_sha = _write(inventory_path, _jsonl(inventory))
    bases: dict[str, tuple[str, str]] = {}
    summaries: dict[str, dict[str, Any]] = {}
    for mode in promotion.MODES:
        rows: list[dict[str, Any]] = []
        for index, item in enumerate(inventory):
            row = {**item, "attempts": 0, "outcome": "not-run"}
            if index in positions:
                row.update(
                    {
                        "attempts": 1,
                        "outcome": "failed" if mode == "candidate" else "passed",
                        "durationSeconds": 1.0,
                        "resultFile": str(tmp_path / f"old-{mode}-{index}.xml"),
                    }
                )
                if mode == "candidate":
                    row["error"] = "_native.UnsupportedError: launch is only available for Chromium"
            rows.append(row)
        ledger = base_dir / f"playwright-python-{mode}-dummy-engine-launch-strict-paired-v1.jsonl"
        ledger_sha = _write(ledger, _jsonl(rows))
        summary = {
            "schema": 1,
            "inventory": str(inventory_path),
            "records": promotion.RECORDS,
            "outcomes": dict(sorted(Counter(row["outcome"] for row in rows).items())),
            "ledgerSha256": ledger_sha,
            "ledger": str(ledger),
        }
        summary_sha = _write(
            ledger.with_suffix(".summary.json"),
            (json.dumps(summary, indent=2) + "\n").encode(),
        )
        bases[mode] = ledger_sha, summary_sha
        summaries[mode] = summary
    promoter_sha = hashlib.sha256(Path(promotion.previous.__file__).read_bytes()).hexdigest()
    receipt = {
        "kind": promotion.previous.RECEIPT_KIND,
        "inventory": str(inventory_path),
        "inventorySha256": inventory_sha,
        "promoterSha256": promoter_sha,
        "commit": promotion.strict.COMMIT,
        "successors": summaries,
        "reports": [{"synthetic": True}],
    }
    receipt_sha = _write(
        base_dir / "promotion-receipt.json",
        (json.dumps(receipt, sort_keys=True) + "\n").encode(),
    )
    monkeypatch.setattr(promotion, "INVENTORY", inventory_path)
    monkeypatch.setattr(promotion, "INVENTORY_SHA256", inventory_sha)
    monkeypatch.setattr(promotion, "BASE_DIR", base_dir)
    monkeypatch.setattr(promotion, "BASES", bases)
    monkeypatch.setattr(promotion, "BASE_PROMOTER_SHA256", promoter_sha)
    monkeypatch.setattr(promotion, "BASE_RECEIPT_SHA256", receipt_sha)

    # Act
    baselines = promotion.base()

    # Assert
    assert set(baselines) == {"candidate", "oracle"}
    return baselines


@pytest.fixture
def synthetic_reports(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> dict[tuple[str, str, str], dict[str, Any]]:
    # Arrange
    sidecars: dict[tuple[str, str, str], Path] = {}
    pins: dict[tuple[str, str, str], str] = {}
    payloads: dict[tuple[str, str, str], dict[str, Any]] = {}
    wheel_sha = "a" * 64
    verifier_sha = "b" * 64
    for mode, api, browser in promotion.TARGET_KEYS:
        key = (mode, api, browser)
        stem = tmp_path / f"{mode}-{api}-{browser}-v1"
        result = stem.with_suffix(".xml")
        sidecar = stem.with_suffix(".provenance.json")
        process = stem.with_suffix(".dummy-process.jsonl")
        public = stem.with_suffix(".public-launch.jsonl")
        nonce = f"{len(payloads) + 1:032x}"
        case = (
            f'<testsuites><testsuite tests="1" failures="0" errors="0" skipped="0">'
            f'<testcase classname="tests.{api}.test_launcher" '
            f'name="{promotion.strict.TEST_NAME}[{browser}]" time="1.25"/>'
            f"</testsuite></testsuites>"
        ).encode()
        result_sha = _write(result, case)
        process_sha = _write(
            process,
            _jsonl(
                [
                    {
                        "event": "dummy-node-exited",
                        "exitCode": 1,
                        "runNonce": nonce,
                        "mode": mode,
                        "api": api,
                        "browser": browser,
                    }
                ]
            ),
        )
        public_sha = _write(
            public,
            _jsonl(
                [
                    {"event": "public-launch-invoked", "runNonce": nonce},
                    {
                        "event": "public-launch-failed",
                        "runNonce": nonce,
                        "processReceiptSha256": process_sha,
                        "errorClass": {
                            "firefox": "Error",
                            "webkit": "TargetClosedError",
                        }[browser],
                    },
                ]
            ),
        )
        payload = {
            "schema": 3,
            "kind": promotion.strict.KIND,
            "evidenceScope": promotion.strict.SCOPE,
            "mode": mode,
            "api": api,
            "browser": browser,
            "testId": promotion.target(api, browser),
            "manifest": str(promotion.strict.POLICY),
            "manifestSha256": "c" * 64,
            "commit": promotion.strict.COMMIT,
            "launchMode": "owned",
            "endpoint": None,
            "collectOnly": False,
            "bootstrapOnly": False,
            "exitCode": 0,
            "verifierSha256": verifier_sha,
            "result": str(result),
            "resultSha256": result_sha,
            "wheelSha256": wheel_sha if mode == "candidate" else None,
            "processReceipt": str(process),
            "processReceiptSha256": process_sha,
            "publicReceipt": str(public),
            "publicReceiptSha256": public_sha,
            "runNonce": nonce,
            "upstream": "/pinned/upstream",
            "toolingRoot": str(promotion.ROOT),
            "toolingSources": {"capture.py": "d" * 64},
            "pythonVersion": "3.12.14",
            "playwrightPackageSha256": "e" * 64,
            "driverNodeSha256": "f" * 64,
            "systemNode": "/pinned/node",
            "systemNodeSha256": "1" * 64,
            "fixtureBind": "100.100.100.100",
            "fixtureHost": "host.example.ts.net",
            "pythonExecutable": f"/pinned/{mode}/python",
            "playwrightPackage": f"/pinned/{mode}/playwright",
            "driverNode": f"/pinned/{mode}/playwright/driver/node",
            "pwrightPackageSha256": "2" * 64 if mode == "candidate" else None,
            "nativeSha256": "3" * 64 if mode == "candidate" else None,
            "runtimeLibraries": {"native.dylib": "4" * 64} if mode == "candidate" else {},
        }
        pins[key] = _write(sidecar, (json.dumps(payload, sort_keys=True) + "\n").encode())
        payloads[key] = payload
        sidecars[key] = sidecar
    monkeypatch.setattr(promotion, "SIDECARS", sidecars)
    monkeypatch.setattr(promotion, "RESULT_DIR", tmp_path)
    monkeypatch.setattr(promotion, "SIDECAR_SHA256", pins)
    monkeypatch.setattr(promotion, "POLICY_SHA256", "c" * 64)
    monkeypatch.setattr(promotion, "CANDIDATE_WHEEL_SHA256", wheel_sha)
    monkeypatch.setattr(
        promotion,
        "TOOL_SHA256",
        {"scripts/verify_playwright_python_dummy_engine_strict_v2.py": verifier_sha},
    )
    monkeypatch.setattr(promotion, "tools_ready", lambda: None)
    monkeypatch.setattr(
        promotion.strict,
        "verify_provenance",
        lambda path: copy.deepcopy(
            next(payload for key, payload in payloads.items() if sidecars[key] == path)
        ),
    )

    # Act
    reports = promotion.reports()

    # Assert
    assert len(reports) == 8
    return payloads


def test_unfilled_hash_pins_prevent_publication(
    red_baselines: dict[str, tuple[list[bytes], list[dict[str, Any]]]],
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    output = tmp_path / "successor"
    monkeypatch.setattr(promotion, "POLICY_SHA256", "")
    monkeypatch.setattr(promotion.strict, "POLICY_SHA256", "")

    # Act, Assert
    with pytest.raises(promotion.PromotionError, match="unfilled|pin differs"):
        promotion.publish(output)
    assert not output.exists()


def test_pinned_v1_four_rows_are_red_only_in_candidate(
    red_baselines: dict[str, tuple[list[bytes], list[dict[str, Any]]]],
) -> None:
    # Arrange
    candidate = red_baselines["candidate"][1]
    oracle = red_baselines["oracle"][1]

    # Act, Assert
    assert [candidate[index]["outcome"] for index in promotion.TARGET_INDEXES] == ["failed"] * 4
    assert [oracle[index]["outcome"] for index in promotion.TARGET_INDEXES] == ["passed"] * 4
    assert all(
        candidate[index]["sourceClassification"] == "public-api-source"
        for index in promotion.TARGET_INDEXES
    )


@pytest.mark.parametrize("mode", promotion.MODES)
def test_successor_changes_only_four_rows_and_preserves_classification(
    red_baselines: dict[str, tuple[list[bytes], list[dict[str, Any]]]],
    synthetic_reports: dict[tuple[str, str, str], dict[str, Any]],
    mode: str,
    tmp_path: Path,
) -> None:
    # Arrange
    reports = promotion.reports()
    old_lines, _ = red_baselines[mode]

    # Act
    ledger, _, summary = promotion.successor(
        mode, red_baselines[mode], reports, tmp_path / "out.jsonl"
    )
    new_lines = ledger.splitlines(keepends=True)

    # Assert
    assert len(new_lines) == promotion.RECORDS
    assert [
        index for index, (old, new) in enumerate(zip(old_lines, new_lines)) if old != new
    ] == list(promotion.TARGET_INDEXES)
    assert all(
        json.loads(new_lines[index])["sourceClassification"] == "public-api-source"
        for index in promotion.TARGET_INDEXES
    )
    assert all(
        json.loads(new_lines[index])["outcome"] == "passed" for index in promotion.TARGET_INDEXES
    )
    assert all("error" not in json.loads(new_lines[index]) for index in promotion.TARGET_INDEXES)
    assert summary["outcomes"]["passed"] == 4


def test_successor_rejects_missing_target(
    red_baselines: dict[str, tuple[list[bytes], list[dict[str, Any]]]],
    synthetic_reports: dict[tuple[str, str, str], dict[str, Any]],
    tmp_path: Path,
) -> None:
    # Arrange
    lines, rows = red_baselines["candidate"]
    altered = [dict(row) for row in rows]
    altered[promotion.TARGET_INDEXES[0]]["id"] = "wrong target"

    # Act, Assert
    with pytest.raises(promotion.PromotionError, match="wrong four rows"):
        promotion.successor(
            "candidate", (lines, altered), promotion.reports(), tmp_path / "out.jsonl"
        )


def test_prepare_limits_claim_to_four_focused_public_cases(
    red_baselines: dict[str, tuple[list[bytes], list[dict[str, Any]]]],
    synthetic_reports: dict[tuple[str, str, str], dict[str, Any]],
    tmp_path: Path,
) -> None:
    # Arrange
    output = tmp_path / "paired-v2"

    # Act
    contents, receipt = promotion.prepare(output)

    # Assert
    assert not output.exists()
    assert len(contents) == 5
    assert len(receipt["reports"]) == 8
    assert receipt["targetZeroBasedIndexes"] == list(promotion.TARGET_INDEXES)
    assert receipt["sourceClassificationPreserved"] == "public-api-source"
    assert receipt["focusedPublicBehaviorEvidence"] is True
    assert receipt["fullFirefoxWebkitSupport"] is False
    assert receipt["publicApiParityEvidence"] is False
    assert receipt["successors"]["candidate"]["outcomes"]["failed"] == 0
    assert receipt["successors"]["candidate"]["outcomes"]["passed"] == 4
    assert receipt["successors"]["oracle"]["outcomes"]["passed"] == 4


@pytest.mark.parametrize(
    ("field", "value", "message"),
    [
        ("exitCode", 1, "identity differs"),
        ("schema", 2, "identity differs"),
        ("wheelSha256", "0" * 64, "wheel differs"),
        ("runNonce", "00000000000000000000000000000000", "exit-one child"),
    ],
)
def test_report_rejects_unproved_candidate_pass(
    synthetic_reports: dict[tuple[str, str, str], dict[str, Any]],
    monkeypatch: pytest.MonkeyPatch,
    field: str,
    value: Any,
    message: str,
) -> None:
    # Arrange
    key = ("candidate", "async", "firefox")
    old = synthetic_reports[key][field]
    synthetic_reports[key][field] = value

    # Act, Assert
    try:
        with pytest.raises(promotion.PromotionError, match=message):
            promotion.reports()
    finally:
        synthetic_reports[key][field] = old


def test_report_rejects_wrong_dummy_exit(
    synthetic_reports: dict[tuple[str, str, str], dict[str, Any]],
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    key = ("oracle", "sync", "webkit")
    process = Path(synthetic_reports[key]["processReceipt"])
    old = process.read_bytes()
    rows = [json.loads(line) for line in old.splitlines()]
    rows[0]["exitCode"] = 0
    synthetic_reports[key]["processReceiptSha256"] = _write(process, _jsonl(rows))

    # Act, Assert
    try:
        with pytest.raises(promotion.PromotionError, match="exit-one child"):
            promotion.reports()
    finally:
        synthetic_reports[key]["processReceiptSha256"] = hashlib.sha256(old).hexdigest()
        process.write_bytes(old)


def test_report_rejects_wrong_browser_exception_class(
    synthetic_reports: dict[tuple[str, str, str], dict[str, Any]],
) -> None:
    # Arrange
    key = ("candidate", "sync", "webkit")
    public = Path(synthetic_reports[key]["publicReceipt"])
    old = public.read_bytes()
    rows = [json.loads(line) for line in old.splitlines()]
    rows[1]["errorClass"] = "Error"
    synthetic_reports[key]["publicReceiptSha256"] = _write(public, _jsonl(rows))

    # Act, Assert
    try:
        with pytest.raises(promotion.PromotionError, match="browser-specific error"):
            promotion.reports()
    finally:
        synthetic_reports[key]["publicReceiptSha256"] = hashlib.sha256(old).hexdigest()
        public.write_bytes(old)


def test_report_rejects_duplicate_run_nonce(
    synthetic_reports: dict[tuple[str, str, str], dict[str, Any]],
) -> None:
    # Arrange
    source = ("candidate", "async", "firefox")
    target = ("oracle", "async", "firefox")
    target_payload = synthetic_reports[target]
    old_nonce = target_payload["runNonce"]
    target_payload["runNonce"] = synthetic_reports[source]["runNonce"]
    for field in ("processReceipt", "publicReceipt"):
        path = Path(target_payload[field])
        rows = [json.loads(line) for line in path.read_bytes().splitlines()]
        for row in rows:
            row["runNonce"] = target_payload["runNonce"]
        if field == "publicReceipt":
            rows[1]["processReceiptSha256"] = target_payload["processReceiptSha256"]
        target_payload[f"{field}Sha256"] = _write(path, _jsonl(rows))

    # Act, Assert
    with pytest.raises(promotion.PromotionError, match="runNonce was reused"):
        promotion.reports()
    target_payload["runNonce"] = old_nonce


def test_report_rejects_strict_verifier_failure(
    synthetic_reports: dict[tuple[str, str, str], dict[str, Any]],
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    def rejected(_: Path) -> dict[str, Any]:
        raise promotion.strict.StrictEvidenceError("unverified official assertion")

    monkeypatch.setattr(promotion.strict, "verify_provenance", rejected)

    # Act, Assert
    with pytest.raises(promotion.strict.StrictEvidenceError, match="unverified"):
        promotion.reports()


def test_report_rejects_failed_official_junit(
    synthetic_reports: dict[tuple[str, str, str], dict[str, Any]],
) -> None:
    # Arrange
    key = ("candidate", "async", "webkit")
    result = Path(synthetic_reports[key]["result"])
    old = result.read_bytes()
    failed = old.replace(b'failures="0"', b'failures="1"')
    synthetic_reports[key]["resultSha256"] = _write(result, failed)

    # Act, Assert
    with pytest.raises(promotion.PromotionError, match="did not pass once"):
        promotion.reports()


def test_report_rejects_sidecar_hash_drift(
    synthetic_reports: dict[tuple[str, str, str], dict[str, Any]],
) -> None:
    # Arrange
    key = ("candidate", "sync", "firefox")
    sidecar = promotion.SIDECARS[key]
    _write(sidecar, sidecar.read_bytes() + b" ")

    # Act, Assert
    with pytest.raises(promotion.PromotionError, match="sidecar SHA-256 differs"):
        promotion.reports()


def test_fixed_sidecar_slots_cannot_be_repointed(monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    wrong = dict(promotion.SIDECARS)
    wrong[("candidate", "async", "firefox")] = Path("/unrelated/fake.provenance.json")
    monkeypatch.setattr(promotion, "SIDECARS", wrong)

    # Act, Assert
    with pytest.raises(promotion.PromotionError, match="exact eight sidecar slots differ"):
        promotion.tools_ready()
