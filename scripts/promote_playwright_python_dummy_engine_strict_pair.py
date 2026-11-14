#!/usr/bin/env python3
"""Atomically publish eight pinned non-Chromium dummy-launch outcomes.

The four candidate failures remain failures. The four oracle passes require a
real, nonce-bound dummy Node exit; neither lane borrows the other's outcome.
Only the eight exact public-source rows may change in the 6,526-row ledgers.
"""

from __future__ import annotations

import argparse
import ctypes
import hashlib
import json
import math
import os
import shutil
import sys
import tempfile
import xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path
from typing import Any, NoReturn, cast

from verify_playwright_python_dummy_engine_strict import (
    COMMIT,
    KIND,
    POLICY,
    POLICY_SHA256,
    SCOPE,
    TEST_NAME,
    StrictEvidenceError,
    verify_provenance,
)

ROOT = Path(__file__).resolve().parents[1]
INVENTORY = ROOT / ".work/official-conformance/playwright-python-v1.58.0-profiled.jsonl"
INVENTORY_SHA256 = "3dfb11b6ad7eb4736166ee2c4f7fa32cda78063324ad8314ce12f3e2d49b9d41"
BASE_DIR = ROOT / ".work/official-conformance/outcomes/python-connect-close-abi121-strict-paired-v1"
BASES = {
    "candidate": (
        BASE_DIR / "playwright-python-candidate-browser-connect-close-strict-paired-v1.jsonl",
        "cc7329662a1ef479c250a3615b55e057cbba8e8454aa798998ded5c5102aa26f",
    ),
    "oracle": (
        BASE_DIR / "playwright-python-oracle-browser-connect-close-strict-paired-v1.jsonl",
        "7ec3fbbf7efb8c6564b19bedf09caeab4e2c16e7b2c2c26be678169db1ecb105",
    ),
}
MODES = ("candidate", "oracle")
APIS = ("async", "sync")
BROWSERS = ("firefox", "webkit")
RECORDS = 6526
TARGET_INDEXES = (2411, 2412, 5627, 5628)
GENERATED = frozenset({"attempts", "durationSeconds", "error", "outcome", "resultFile"})
RECEIPT_KIND = "playwright-python-nonchromium-dummy-node-strict-paired-promotion-v1"
RESULT_DIR = ROOT / ".work/official-conformance/results/python-dummy-engine-abi121-strict-v1"
SIDECARS = {
    (mode, api, browser): RESULT_DIR / f"{mode}-{api}-{browser}-v2.provenance.json"
    for mode in MODES
    for api in APIS
    for browser in BROWSERS
}


class PromotionError(ValueError):
    """The exact eight strict reports cannot produce a paired successor."""


def _fail(message: str) -> NoReturn:
    raise PromotionError(message)


def _require(condition: bool, message: str) -> None:
    if not condition:
        _fail(message)


def _sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _regular_bytes(path: Path, label: str) -> bytes:
    _require(path.is_file() and not path.is_symlink(), f"{label} is missing or linked: {path}")
    return path.read_bytes()


def _json_object(data: bytes, label: str) -> dict[str, Any]:
    try:
        value = json.loads(data)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise PromotionError(f"{label} is not valid JSON") from error
    _require(isinstance(value, dict), f"{label} must be a JSON object")
    return cast(dict[str, Any], value)


def _jsonl(data: bytes, label: str) -> tuple[list[bytes], list[dict[str, Any]]]:
    _require(data.endswith(b"\n"), f"{label} is not newline terminated")
    lines = data.splitlines(keepends=True)
    _require(len(lines) == RECORDS, f"{label} is not the full {RECORDS}-row corpus")
    return lines, [_json_object(line, f"{label} row {index}") for index, line in enumerate(lines)]


def _line(value: dict[str, Any]) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def _target(api: str, browser: str) -> str:
    return f"tests/{api}/test_launcher.py::{TEST_NAME}[{browser}]"


def _inventory() -> list[dict[str, Any]]:
    raw = _regular_bytes(INVENTORY, "pinned Python inventory")
    _require(_sha(raw) == INVENTORY_SHA256, "pinned Python inventory SHA-256 differs")
    _, rows = _jsonl(raw, "pinned Python inventory")
    ids = [row.get("id") for row in rows]
    _require(all(isinstance(node_id, str) for node_id in ids), "inventory has malformed IDs")
    _require(len(set(ids)) == RECORDS, "inventory has duplicate IDs")
    expected = tuple(_target(api, browser) for api in APIS for browser in BROWSERS)
    _require(
        tuple(ids[index] for index in TARGET_INDEXES) == expected,
        "inventory target positions differ from exact non-Chromium launch IDs",
    )
    return rows


def _base(mode: str, inventory: list[dict[str, Any]]) -> tuple[list[bytes], list[dict[str, Any]]]:
    path, expected_sha = BASES[mode]
    raw = _regular_bytes(path, f"{mode} pinned base")
    _require(_sha(raw) == expected_sha, f"{mode} pinned base SHA-256 differs")
    lines, rows = _jsonl(raw, f"{mode} pinned base")
    summary = _json_object(
        _regular_bytes(path.with_suffix(".summary.json"), f"{mode} base summary"),
        f"{mode} base summary",
    )
    _require(
        summary
        == {
            "schema": 1,
            "inventory": str(INVENTORY),
            "records": RECORDS,
            "outcomes": dict(sorted(Counter(row.get("outcome") for row in rows).items())),
            "ledgerSha256": expected_sha,
            "ledger": str(path),
        },
        f"{mode} base summary differs from pinned ledger",
    )
    seen: set[str] = set()
    for index, (row, inventory_row) in enumerate(zip(rows, inventory)):
        node_id = row.get("id")
        _require(
            isinstance(node_id, str) and node_id not in seen,
            f"{mode} duplicate or malformed ID at row {index}",
        )
        seen.add(node_id)
        _require(
            {key: value for key, value in row.items() if key not in GENERATED} == inventory_row,
            f"{mode} base metadata differs from inventory at row {index}",
        )
        if index in TARGET_INDEXES:
            _require(
                inventory_row.get("sourceClassification") == "public-api-source"
                and row == {**inventory_row, "attempts": 0, "outcome": "not-run"},
                f"{mode} target is not a pristine public launch row",
            )
    return lines, rows


def _case_data(payload: dict[str, Any]) -> tuple[float, str | None]:
    raw = _regular_bytes(Path(payload["result"]), "strict official JUnit")
    _require(_sha(raw) == payload["resultSha256"], "strict JUnit changed after verification")
    try:
        root = ET.fromstring(raw)
    except ET.ParseError as error:
        raise PromotionError("strict JUnit became malformed") from error
    suites = root.findall("testsuite") if root.tag == "testsuites" else []
    cases = list(root.iter("testcase"))
    _require(len(suites) == 1 and len(cases) == 1, "strict JUnit must contain one case")
    suite, case = suites[0], cases[0]
    _require(
        len(suite.findall("testcase")) == 1
        and case.get("classname") == f"tests.{payload['api']}.test_launcher"
        and case.get("name") == f"{TEST_NAME}[{payload['browser']}]"
        and suite.get("tests") == "1"
        and suite.get("skipped") == "0"
        and suite.get("errors") == "0"
        and case.find("skipped") is None
        and case.find("error") is None,
        "strict JUnit is not the exact official launch case",
    )
    failure = case.find("failure")
    if payload["mode"] == "candidate":
        message = failure.get("message") if failure is not None else None
        _require(
            suite.get("failures") == "1"
            and failure is not None
            and message == "_native.UnsupportedError: launch is only available for Chromium"
            and payload["exitCode"] == 1,
            "candidate JUnit is not the authentic unsupported failure",
        )
    else:
        _require(
            suite.get("failures") == "0" and failure is None and payload["exitCode"] == 0,
            "oracle JUnit is not a pass",
        )
        message = None
    try:
        duration = float(case.get("time", "0"))
    except ValueError as error:
        raise PromotionError("strict JUnit duration is invalid") from error
    _require(math.isfinite(duration) and duration >= 0, "strict JUnit duration is invalid")
    return duration, message


def _reports(
    sidecars: dict[tuple[str, str, str], Path],
) -> dict[tuple[str, str, str], dict[str, Any]]:
    _require(sidecars == SIDECARS, "the exact eight pinned sidecars are required")
    _require(len(set(sidecars.values())) == 8, "strict sidecar paths must be distinct")
    verified: dict[tuple[str, str, str], dict[str, Any]] = {}
    for (mode, api, browser), path in sidecars.items():
        original = _regular_bytes(path, f"{mode}/{api}/{browser} strict sidecar")
        payload = verify_provenance(path)
        _require(
            _sha(_regular_bytes(path, f"{mode}/{api}/{browser} strict sidecar")) == _sha(original),
            f"{mode}/{api}/{browser} sidecar changed during verification",
        )
        _require(
            payload.get("schema") == 2
            and payload.get("kind") == KIND
            and payload.get("evidenceScope") == SCOPE
            and payload.get("mode") == mode
            and payload.get("api") == api
            and payload.get("browser") == browser
            and payload.get("testId") == _target(api, browser)
            and payload.get("manifest") == str(POLICY)
            and payload.get("manifestSha256") == POLICY_SHA256
            and payload.get("commit") == COMMIT
            and payload.get("launchMode") == "owned"
            and payload.get("endpoint") is None
            and payload.get("collectOnly") is False
            and payload.get("bootstrapOnly") is False
            and payload.get("exitCode") == (1 if mode == "candidate" else 0),
            f"{mode}/{api}/{browser} is not the exact executed official outcome",
        )
        _require(
            path == Path(payload["result"]).with_suffix(".provenance.json"),
            f"{mode}/{api}/{browser} sidecar is not its JUnit sibling",
        )
        report = dict(payload)
        report["_duration"], report["_error"] = _case_data(payload)
        report["_sidecarSha256"] = _sha(original)
        verified[(mode, api, browser)] = report
    reference = verified[("candidate", "async", "firefox")]
    common = (
        "kind",
        "evidenceScope",
        "manifest",
        "manifestSha256",
        "upstream",
        "commit",
        "toolingRoot",
        "toolingSources",
        "pythonVersion",
        "wheelPath",
        "wheelSha256",
        "playwrightPackageSha256",
        "driverNodeSha256",
        "systemNode",
        "systemNodeSha256",
        "fixtureBind",
        "fixtureHost",
        "launchMode",
        "endpoint",
        "verifierSha256",
    )
    for key, report in verified.items():
        _require(
            all(report.get(field) == reference.get(field) for field in common),
            f"{key} manifest, source, fixture or artifact identity differs across lanes",
        )
    for mode in MODES:
        lane = [verified[(mode, api, browser)] for api in APIS for browser in BROWSERS]
        for field in ("pythonExecutable", "playwrightPackage", "driverNode"):
            _require(
                len({report.get(field) for report in lane}) == 1,
                f"{mode} runtime {field} differs across cases",
            )
        if mode == "candidate":
            for field in ("pwrightPackage", "pwrightPackageSha256", "nativePath", "nativeSha256"):
                _require(
                    len({report.get(field) for report in lane}) == 1,
                    f"candidate {field} differs across cases",
                )
            _require(
                all(report["runtimeLibraries"] == lane[0]["runtimeLibraries"] for report in lane),
                "candidate runtime libraries differ across cases",
            )
    for field in ("result", "publicReceipt", "processReceipt", "runNonce"):
        values = [report.get(field) for report in verified.values()]
        _require(len(set(values)) == 8, f"strict {field} was reused across executions")
    return verified


def _successor(
    mode: str,
    base_lines: list[bytes],
    base_rows: list[dict[str, Any]],
    reports: dict[tuple[str, str, str], dict[str, Any]],
    output: Path,
) -> tuple[bytes, bytes, dict[str, Any]]:
    selected = {
        _target(api, browser): reports[(mode, api, browser)] for api in APIS for browser in BROWSERS
    }
    lines: list[bytes] = []
    changed: list[int] = []
    counts = Counter(row["outcome"] for row in base_rows)
    outcome = "failed" if mode == "candidate" else "passed"
    for index, (original, row) in enumerate(zip(base_lines, base_rows)):
        report = selected.get(row["id"])
        if report is None:
            lines.append(original)
            continue
        replacement = {
            **row,
            "outcome": outcome,
            "attempts": 1,
            "durationSeconds": report["_duration"],
            "resultFile": report["result"],
        }
        if outcome == "failed":
            replacement["error"] = report["_error"]
        lines.append(_line(replacement))
        changed.append(index)
        counts["not-run"] -= 1
        counts[outcome] += 1
    _require(tuple(changed) == TARGET_INDEXES, f"{mode} successor changed the wrong rows")
    _require(counts["not-run"] >= 0, f"{mode} not-run count underflow")
    ledger = b"".join(lines)
    summary = {
        "schema": 1,
        "inventory": str(INVENTORY),
        "records": RECORDS,
        "outcomes": dict(sorted(counts.items())),
        "ledgerSha256": _sha(ledger),
        "ledger": str(output),
    }
    return ledger, (json.dumps(summary, indent=2) + "\n").encode(), summary


def prepare(
    sidecars: dict[tuple[str, str, str], Path], output_dir: Path
) -> tuple[dict[str, bytes], dict[str, Any]]:
    """Verify all eight inputs and calculate both successors without writing output."""
    _require(output_dir.is_absolute(), "successor directory must be absolute")
    inventory = _inventory()
    reports = _reports(sidecars)
    content: dict[str, bytes] = {}
    summaries: dict[str, dict[str, Any]] = {}
    bases: dict[str, dict[str, str]] = {}
    for mode in MODES:
        lines, rows = _base(mode, inventory)
        name = f"playwright-python-{mode}-dummy-engine-launch-strict-paired-v1.jsonl"
        ledger, summary_bytes, summary = _successor(mode, lines, rows, reports, output_dir / name)
        content[name] = ledger
        content[Path(name).with_suffix(".summary.json").name] = summary_bytes
        summaries[mode] = summary
        base_path, base_sha = BASES[mode]
        bases[mode] = {"ledger": str(base_path), "ledgerSha256": base_sha}
    receipt = {
        "schema": 1,
        "kind": RECEIPT_KIND,
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "manifest": str(POLICY),
        "manifestSha256": POLICY_SHA256,
        "commit": COMMIT,
        "promoterSha256": _sha(_regular_bytes(Path(__file__), "promoter source")),
        "candidateWheelSha256": reports[("candidate", "async", "firefox")]["wheelSha256"],
        "bases": bases,
        "reports": [
            {
                "mode": mode,
                "api": api,
                "browser": browser,
                "testId": _target(api, browser),
                "provenance": str(sidecars[(mode, api, browser)]),
                "provenanceSha256": reports[(mode, api, browser)]["_sidecarSha256"],
                "result": reports[(mode, api, browser)]["result"],
                "resultSha256": reports[(mode, api, browser)]["resultSha256"],
                "outcome": "failed" if mode == "candidate" else "passed",
            }
            for mode in MODES
            for api in APIS
            for browser in BROWSERS
        ],
        "successors": summaries,
    }
    content["promotion-receipt.json"] = (
        json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    ).encode()
    return content, receipt


def _check_output(output_dir: Path) -> None:
    _require(output_dir.is_absolute(), "successor directory must be absolute")
    _require(
        not output_dir.exists() and not output_dir.is_symlink(),
        f"successor directory already exists or is linked: {output_dir}",
    )
    _require(
        output_dir.parent.is_dir() and not output_dir.parent.is_symlink(),
        "successor parent is missing or linked",
    )
    _require(
        not any(parent.is_symlink() for parent in output_dir.parents),
        "successor path contains a symlink parent",
    )


def _fsync_directory(path: Path) -> None:
    descriptor = os.open(path, os.O_RDONLY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def _publish_exclusive(staging: Path, destination: Path) -> None:
    """No-replace rename publishes both ledgers and the receipt as one directory."""
    libc = ctypes.CDLL(None, use_errno=True)
    if sys.platform == "darwin":
        rename = libc.renameatx_np
        rename.argtypes = [
            ctypes.c_int,
            ctypes.c_char_p,
            ctypes.c_int,
            ctypes.c_char_p,
            ctypes.c_uint,
        ]
        rename.restype = ctypes.c_int
        result = rename(-2, os.fsencode(staging), -2, os.fsencode(destination), 0x4)
    elif sys.platform.startswith("linux"):
        rename = libc.renameat2
        rename.argtypes = [
            ctypes.c_int,
            ctypes.c_char_p,
            ctypes.c_int,
            ctypes.c_char_p,
            ctypes.c_uint,
        ]
        rename.restype = ctypes.c_int
        result = rename(-100, os.fsencode(staging), -100, os.fsencode(destination), 0x1)
    else:
        _fail("atomic no-replace directory publication is unsupported here")
    if result != 0:
        raise PromotionError(f"atomic paired publication failed: {os.strerror(ctypes.get_errno())}")


def promote(sidecars: dict[tuple[str, str, str], Path], output_dir: Path) -> dict[str, Any]:
    """Publish complete paired successors or leave the target absent."""
    _check_output(output_dir)
    content, receipt = prepare(sidecars, output_dir)
    staging = Path(tempfile.mkdtemp(prefix=f".{output_dir.name}.staging-", dir=output_dir.parent))
    try:
        for name, data in content.items():
            target = staging / name
            with target.open("xb") as stream:
                stream.write(data)
                stream.flush()
                os.fsync(stream.fileno())
        _fsync_directory(staging)
        _check_output(output_dir)
        fresh_content, fresh_receipt = prepare(sidecars, output_dir)
        _require(
            fresh_content == content and fresh_receipt == receipt,
            "strict evidence changed while paired successors were staged",
        )
        _publish_exclusive(staging, output_dir)
    finally:
        if staging.exists():
            shutil.rmtree(staging)
    return receipt


def verify_bundle(sidecars: dict[tuple[str, str, str], Path], output_dir: Path) -> dict[str, Any]:
    """Read-only exact-content audit of a published paired successor."""
    _require(
        output_dir.is_dir() and not output_dir.is_symlink(),
        "paired successor directory is missing or linked",
    )
    content, receipt = prepare(sidecars, output_dir)
    _require(
        {path.name for path in output_dir.iterdir()} == set(content),
        "paired successor directory contains missing or extra files",
    )
    for name, expected in content.items():
        _require(
            _regular_bytes(output_dir / name, f"paired successor {name}") == expected,
            f"paired successor changed from exact eight-report merge: {name}",
        )
    return receipt


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--verify-existing", action="store_true")
    args = parser.parse_args()
    output_dir = args.output_dir.absolute()
    try:
        receipt = (
            verify_bundle(SIDECARS, output_dir)
            if args.verify_existing
            else promote(SIDECARS, output_dir)
        )
    except (
        OSError,
        PromotionError,
        StrictEvidenceError,
        KeyError,
        TypeError,
        AttributeError,
        IndexError,
    ) as error:
        parser.error(str(error))
    print(
        json.dumps(
            {
                "verified" if args.verify_existing else "published": str(output_dir),
                "successors": receipt["successors"],
            },
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
