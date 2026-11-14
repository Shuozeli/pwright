#!/usr/bin/env python3
"""Publish exactly two passing official CDP WebSocket rows per Python lane.

The unchanged upstream tests run with an explicitly disclosed fixture-only
loopback-to-MagicDNS transport adapter. All four independently verified strict
reports must pass before a new immutable candidate/oracle ledger pair exists.
An authentic failed capture is useful diagnostic evidence but cannot promote.
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

from verify_playwright_python_ws_transport_strict import (
    COMMIT,
    KIND,
    POLICY,
    POLICY_SHA256,
    TEST_IDS,
    StrictWebSocketEvidenceError,
    verify_provenance,
)

ROOT = Path(__file__).resolve().parents[1]
INVENTORY = ROOT / ".work/official-conformance/playwright-python-v1.58.0-profiled.jsonl"
INVENTORY_SHA256 = "3dfb11b6ad7eb4736166ee2c4f7fa32cda78063324ad8314ce12f3e2d49b9d41"
BASE_DIR = (
    ROOT
    / ".work/official-conformance/outcomes/python-dummy-node-abi119-strict-paired-v1"
)
BASES = {
    "candidate": (
        BASE_DIR
        / "playwright-python-candidate-dummy-node-abi119-strict-paired-v1.jsonl",
        "9c057de3aff4b3c2141d150999e12adcbd162ba1f7876cdc6d2a949fddda5f80",
    ),
    "oracle": (
        BASE_DIR / "playwright-python-oracle-dummy-node-abi119-strict-paired-v1.jsonl",
        "a95e15e4cfb4708a6e8a98832fea7e4e2759ca8ad034f13a3bfc8933f21913f1",
    ),
}
MODES = ("candidate", "oracle")
CASES = ("header", "close")
RECORDS = 6526
TARGET_INDEXES = (862, 871)
GENERATED = frozenset({"attempts", "durationSeconds", "error", "outcome", "resultFile"})
RECEIPT_KIND = "playwright-python-cdp-ws-transport-strict-paired-promotion-v1"
_ADAPTATION = "exact-127.0.0.1-ws-to-tailscale-magicdns-only"


class PromotionError(ValueError):
    """The four strict reports cannot form an immutable paired successor."""


def _fail(message: str) -> NoReturn:
    raise PromotionError(message)


def _require(condition: bool, message: str) -> None:
    if not condition:
        _fail(message)


def _sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _regular_bytes(path: Path, label: str) -> bytes:
    _require(
        path.is_file() and not path.is_symlink(),
        f"{label} is missing or linked: {path}",
    )
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
    rows = [
        _json_object(line, f"{label} row {index}") for index, line in enumerate(lines)
    ]
    return lines, rows


def _line(value: dict[str, Any]) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def _target(case: str) -> str:
    return TEST_IDS[CASES.index(case)]


def _inventory() -> list[dict[str, Any]]:
    raw = _regular_bytes(INVENTORY, "pinned Python inventory")
    _require(_sha(raw) == INVENTORY_SHA256, "pinned Python inventory SHA-256 differs")
    _, rows = _jsonl(raw, "pinned Python inventory")
    ids = [row.get("id") for row in rows]
    _require(
        all(isinstance(node_id, str) for node_id in ids), "inventory has a malformed ID"
    )
    _require(len(set(ids)) == RECORDS, "inventory has duplicate IDs")
    _require(
        tuple(ids[index] for index in TARGET_INDEXES) == TEST_IDS,
        "inventory target positions differ from the exact two Chromium IDs",
    )
    return rows


def _base(
    mode: str, inventory: list[dict[str, Any]]
) -> tuple[list[bytes], list[dict[str, Any]]]:
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
            "outcomes": dict(
                sorted(Counter(row.get("outcome") for row in rows).items())
            ),
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
            {key: value for key, value in row.items() if key not in GENERATED}
            == inventory_row,
            f"{mode} base metadata differs from inventory at row {index}",
        )
        if index in TARGET_INDEXES:
            _require(
                row
                == {
                    "api": "async",
                    "attempts": 0,
                    "file": "tests/async/test_browsertype_connect_cdp.py",
                    "id": TEST_IDS[TARGET_INDEXES.index(index)],
                    "outcome": "not-run",
                    "privateImports": [],
                    "sourceClassification": "public-api-source",
                },
                f"{mode} target is not a pristine public Chromium row",
            )
    return lines, rows


def _duration(payload: dict[str, Any]) -> float:
    result = Path(str(payload["result"]))
    raw = _regular_bytes(result, "strict official JUnit")
    _require(
        _sha(raw) == payload["resultSha256"], "strict JUnit changed after verification"
    )
    try:
        root = ET.fromstring(raw)
    except ET.ParseError as error:
        raise PromotionError("strict JUnit became malformed") from error
    suites = root.findall("testsuite") if root.tag == "testsuites" else []
    cases = list(root.iter("testcase"))
    _require(len(suites) == 1 and len(cases) == 1, "strict JUnit must contain one case")
    case = cases[0]
    _require(
        case.get("classname") == "tests.async.test_browsertype_connect_cdp"
        and case.get("name") == payload["testId"].split("::", 1)[1]
        and all(case.find(kind) is None for kind in ("failure", "error", "skipped"))
        and payload["exitCode"] == 0
        and payload["outcome"] == "passed",
        "strict JUnit does not contain the exact passing official case",
    )
    try:
        duration = float(case.get("time", "0"))
    except ValueError as error:
        raise PromotionError("strict JUnit duration is invalid") from error
    _require(
        math.isfinite(duration) and duration >= 0, "strict JUnit duration is invalid"
    )
    return duration


def _reports(
    sidecars: dict[tuple[str, str], Path],
) -> dict[tuple[str, str], dict[str, Any]]:
    _require(
        set(sidecars) == {(mode, case) for mode in MODES for case in CASES},
        "all four lane/case sidecars are required",
    )
    _require(len(set(sidecars.values())) == 4, "strict sidecar paths must be distinct")
    verified: dict[tuple[str, str], dict[str, Any]] = {}
    for (mode, case), path in sidecars.items():
        original = _regular_bytes(path, f"{mode}/{case} strict sidecar")
        payload = verify_provenance(path)
        _require(
            _sha(_regular_bytes(path, f"{mode}/{case} strict sidecar"))
            == _sha(original),
            f"{mode}/{case} sidecar changed during verification",
        )
        _require(
            payload.get("schema") == 1
            and payload.get("kind") == KIND
            and payload.get("mode") == mode
            and payload.get("testId") == _target(case)
            and payload.get("manifest") == str(POLICY)
            and payload.get("manifestSha256") == POLICY_SHA256
            and payload.get("commit") == COMMIT
            and payload.get("fixtureTransportAdaptation") == _ADAPTATION
            and payload.get("outcome") == "passed"
            and payload.get("exitCode") == 0,
            f"{mode}/{case} is not a verified passing exact official case",
        )
        _require(
            path == Path(str(payload["result"])).with_suffix(".provenance.json"),
            f"{mode}/{case} sidecar is not its JUnit sibling",
        )
        report = dict(payload)
        report["_duration"] = _duration(payload)
        report["_sidecarSha256"] = _sha(original)
        verified[(mode, case)] = report
    reference = verified[("candidate", "header")]
    common = (
        "kind",
        "manifest",
        "manifestSha256",
        "upstream",
        "commit",
        "toolingRoot",
        "toolingSources",
        "pythonVersion",
        "pythonRuntimeSha256",
        "wheelPath",
        "wheelSha256",
        "playwrightPackageSha256",
        "driverNodeSha256",
        "fixtureBind",
        "fixtureHost",
        "fixtureTransportAdaptation",
        "verifierSha256",
    )
    for key, report in verified.items():
        _require(
            all(report.get(field) == reference.get(field) for field in common),
            f"{key} manifest, upstream, fixture or artifact identity differs across lanes",
        )
    for mode in MODES:
        left, right = verified[(mode, "header")], verified[(mode, "close")]
        for field in ("pythonExecutable", "playwrightPackage", "driverNode"):
            _require(
                left.get(field) == right.get(field), f"{mode} runtime path differs"
            )
        if mode == "candidate":
            for field in (
                "pwrightPackage",
                "pwrightPackageSha256",
                "nativePath",
                "nativeSha256",
                "runtimeLibraries",
            ):
                _require(
                    left.get(field) == right.get(field), f"candidate {field} differs"
                )
    for field in ("result", "callReceipt", "runNonce"):
        values = [report.get(field) for report in verified.values()]
        _require(len(set(values)) == 4, f"strict {field} was reused across executions")
    return verified


def _successor(
    mode: str,
    base_lines: list[bytes],
    base_rows: list[dict[str, Any]],
    reports: dict[tuple[str, str], dict[str, Any]],
    output: Path,
) -> tuple[bytes, bytes, dict[str, Any]]:
    chosen = {_target(case): reports[(mode, case)] for case in CASES}
    lines: list[bytes] = []
    changed: list[int] = []
    counts = Counter(row["outcome"] for row in base_rows)
    for index, (original, row) in enumerate(zip(base_lines, base_rows)):
        report = chosen.get(row["id"])
        if report is None:
            lines.append(original)
            continue
        replacement = {
            **row,
            "outcome": "passed",
            "attempts": 1,
            "durationSeconds": report["_duration"],
            "resultFile": report["result"],
        }
        lines.append(_line(replacement))
        changed.append(index)
        counts["not-run"] -= 1
        counts["passed"] += 1
    _require(
        tuple(changed) == TARGET_INDEXES, f"{mode} successor changed the wrong rows"
    )
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


def prepare(
    sidecars: dict[tuple[str, str], Path], output_dir: Path
) -> tuple[dict[str, bytes], dict[str, Any]]:
    """Read and verify everything; create no output and give no pass credit yet."""
    _require(output_dir.is_absolute(), "successor directory must be absolute")
    inventory = _inventory()
    reports = _reports(sidecars)
    content: dict[str, bytes] = {}
    summaries: dict[str, dict[str, Any]] = {}
    bases: dict[str, dict[str, str]] = {}
    for mode in MODES:
        lines, rows = _base(mode, inventory)
        name = f"playwright-python-{mode}-cdp-ws-transport-strict-paired-v1.jsonl"
        ledger, summary_bytes, summary = _successor(
            mode, lines, rows, reports, output_dir / name
        )
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
        "candidateWheelSha256": reports[("candidate", "header")]["wheelSha256"],
        "bases": bases,
        "reports": [
            {
                "mode": mode,
                "case": case,
                "testId": _target(case),
                "provenance": str(sidecars[(mode, case)]),
                "provenanceSha256": reports[(mode, case)]["_sidecarSha256"],
                "result": reports[(mode, case)]["result"],
                "resultSha256": reports[(mode, case)]["resultSha256"],
                "outcome": "passed",
            }
            for mode in MODES
            for case in CASES
        ],
        "successors": summaries,
    }
    content["promotion-receipt.json"] = (
        json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    ).encode()
    return content, receipt


def _fsync_directory(path: Path) -> None:
    descriptor = os.open(path, os.O_RDONLY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def _publish_exclusive(staging: Path, destination: Path) -> None:
    """No-replace rename publishes both ledgers and receipt as one directory."""
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
        error = ctypes.get_errno()
        raise PromotionError(f"atomic paired publication failed: {os.strerror(error)}")


def promote(sidecars: dict[tuple[str, str], Path], output_dir: Path) -> dict[str, Any]:
    """Publish complete paired successors or leave the target absent."""
    _check_output(output_dir)
    content, receipt = prepare(sidecars, output_dir)
    staging = Path(
        tempfile.mkdtemp(prefix=f".{output_dir.name}.staging-", dir=output_dir.parent)
    )
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


def verify_bundle(
    sidecars: dict[tuple[str, str], Path], output_dir: Path
) -> dict[str, Any]:
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
            f"paired successor changed from exact four-report merge: {name}",
        )
    return receipt


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    for mode in MODES:
        for case in CASES:
            parser.add_argument(f"--{mode}-{case}-provenance", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--verify-existing", action="store_true")
    args = parser.parse_args()
    sidecars = {
        (mode, case): getattr(args, f"{mode}_{case}_provenance").absolute()
        for mode in MODES
        for case in CASES
    }
    try:
        receipt = (
            verify_bundle(sidecars, args.output_dir.absolute())
            if args.verify_existing
            else promote(sidecars, args.output_dir.absolute())
        )
    except (
        OSError,
        PromotionError,
        StrictWebSocketEvidenceError,
        KeyError,
        TypeError,
        AttributeError,
        IndexError,
    ) as error:
        parser.error(str(error))
    print(
        json.dumps(
            {
                "verified" if args.verify_existing else "published": str(
                    args.output_dir.absolute()
                ),
                "successors": receipt["successors"],
            },
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
