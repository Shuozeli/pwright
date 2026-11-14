#!/usr/bin/env python3
"""Atomically publish only the four verified guarded dummy-launch outcomes.

The two immutable 6,526-row Python ledgers are pinned here. Four distinct,
fresh strict sidecars (candidate/oracle x async/sync) must all verify before a
new pair is staged. No generic schema-1..5 result promotion is involved.
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

from verify_playwright_python_dummy_node_strict import (
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
BASES = {
    "candidate": (
        ROOT / ".work/official-conformance/outcomes/"
        "playwright-python-candidate-abi109-elementhandle-async59-20261011-v11.jsonl",
        "5c0f4078382dfcdbc0b9ed5b6e90ee7e3b0c4922755eff23b54f69f3b9023dfa",
    ),
    "oracle": (
        ROOT / ".work/official-conformance/outcomes/"
        "playwright-python-oracle-evaluate-handle-preview2-20261011-v8.jsonl",
        "ed44f66be349a527e682aa2648fc7bf61a8a527af839411f45bbc606460f2a72",
    ),
}
APIS = ("async", "sync")
MODES = ("candidate", "oracle")
RECORDS = 6526
GENERATED = frozenset({"attempts", "durationSeconds", "error", "outcome", "resultFile"})
RECEIPT_KIND = "playwright-python-dummy-node-strict-paired-promotion-v1"


class PromotionError(ValueError):
    """The four strict reports cannot produce a paired ledger successor."""


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
        f"{label} is missing or a symlink: {path}",
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
    records = [
        _json_object(line, f"{label} row {index}") for index, line in enumerate(lines)
    ]
    return lines, records


def _line(value: dict[str, Any]) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def _target(api: str) -> str:
    return f"tests/{api}/test_launcher.py::{TEST_NAME}[chromium]"


def _inventory() -> list[dict[str, Any]]:
    raw = _regular_bytes(INVENTORY, "pinned Python inventory")
    _require(_sha(raw) == INVENTORY_SHA256, "pinned Python inventory SHA-256 differs")
    _, rows = _jsonl(raw, "pinned Python inventory")
    ids = [row.get("id") for row in rows]
    _require(
        all(isinstance(node_id, str) for node_id in ids),
        "inventory contains a malformed ID",
    )
    _require(len(set(ids)) == RECORDS, "inventory contains duplicate IDs")
    _require(
        {_target(api) for api in APIS}.issubset(ids),
        "inventory omits a guarded dummy-launch ID",
    )
    return rows


def _base(
    mode: str, inventory: list[dict[str, Any]]
) -> tuple[list[bytes], list[dict[str, Any]], dict[str, Any]]:
    path, expected_sha = BASES[mode]
    raw = _regular_bytes(path, f"{mode} pinned base")
    _require(_sha(raw) == expected_sha, f"{mode} pinned base SHA-256 differs")
    summary = _json_object(
        _regular_bytes(path.with_suffix(".summary.json"), f"{mode} base summary"),
        f"{mode} base summary",
    )
    lines, rows = _jsonl(raw, f"{mode} pinned base")
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
        f"{mode} base summary differs from the actual pinned ledger",
    )
    seen: set[str] = set()
    targets: set[str] = set()
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
        if node_id in {_target(api) for api in APIS}:
            api = "async" if node_id == _target("async") else "sync"
            _require(
                row
                == {
                    "api": api,
                    "attempts": 0,
                    "file": f"tests/{api}/test_launcher.py",
                    "id": node_id,
                    "outcome": "not-run",
                    "privateImports": [],
                    "sourceClassification": "public-api-source",
                },
                f"{mode} target is not a pristine public Chromium row: {node_id}",
            )
            targets.add(node_id)
    _require(
        targets == {_target(api) for api in APIS}, f"{mode} base is missing a target"
    )
    return lines, rows, summary


def _duration(payload: dict[str, Any]) -> float:
    report_path = Path(str(payload["result"]))
    raw = _regular_bytes(report_path, "strict JUnit report")
    _require(
        _sha(raw) == payload["resultSha256"],
        "strict JUnit changed after provenance verification",
    )
    try:
        root = ET.fromstring(raw)
    except ET.ParseError as error:
        raise PromotionError("strict JUnit became malformed") from error
    cases = list(root.iter("testcase"))
    _require(len(cases) == 1, "strict JUnit must contain one case")
    case = cases[0]
    _require(
        case.get("classname") == f"tests.{payload['api']}.test_launcher"
        and case.get("name") == f"{TEST_NAME}[chromium]"
        and all(case.find(kind) is None for kind in ("failure", "error", "skipped")),
        "strict JUnit does not contain the exact passing case",
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
        set(sidecars) == {(mode, api) for mode in MODES for api in APIS},
        "all four lane/API sidecars are required",
    )
    _require(len(set(sidecars.values())) == 4, "strict sidecar paths must be distinct")
    verified: dict[tuple[str, str], dict[str, Any]] = {}
    for (mode, api), sidecar in sidecars.items():
        source = _regular_bytes(sidecar, f"{mode}/{api} strict provenance")
        payload = verify_provenance(sidecar)
        _require(
            isinstance(payload, dict), f"{mode}/{api} verifier did not return an object"
        )
        _require(
            _sha(_regular_bytes(sidecar, f"{mode}/{api} strict provenance"))
            == _sha(source),
            f"{mode}/{api} strict provenance changed after verification",
        )
        _require(
            payload.get("schema") == 2
            and payload.get("kind") == KIND
            and payload.get("evidenceScope") == SCOPE
            and payload.get("mode") == mode
            and payload.get("api") == api
            and payload.get("testId") == _target(api)
            and payload.get("manifest") == str(POLICY)
            and payload.get("manifestSha256") == POLICY_SHA256
            and payload.get("commit") == COMMIT
            and payload.get("launchMode") == "owned"
            and payload.get("endpoint") is None
            and payload.get("collectOnly") is False
            and payload.get("bootstrapOnly") is False
            and payload.get("exitCode") == 0,
            f"{mode}/{api} is not the exact strict executed Chromium case",
        )
        _require(
            sidecar == Path(str(payload.get("result"))).with_suffix(".provenance.json"),
            f"{mode}/{api} sidecar is not its JUnit sibling",
        )
        payload = dict(payload)
        payload["_duration"] = _duration(payload)
        payload["_sidecarSha256"] = _sha(source)
        verified[(mode, api)] = payload
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
    reference = verified[("candidate", "async")]
    for key, payload in verified.items():
        _require(
            all(payload.get(field) == reference.get(field) for field in common),
            f"{key} manifest, source or artifact identity differs across lanes",
        )
    for mode in MODES:
        left, right = verified[(mode, "async")], verified[(mode, "sync")]
        for field in ("pythonExecutable", "playwrightPackage", "driverNode"):
            _require(
                left.get(field) == right.get(field),
                f"{mode} runtime path differs across APIs",
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
                    left.get(field) == right.get(field),
                    f"candidate {field} differs across APIs",
                )
    for field in ("result", "publicReceipt", "processReceipt", "runNonce"):
        values = [payload.get(field) for payload in verified.values()]
        _require(
            len(set(values)) == 4, f"strict {field} was reused across four executions"
        )
    return verified


def _successor(
    mode: str,
    base_lines: list[bytes],
    base_rows: list[dict[str, Any]],
    reports: dict[tuple[str, str], dict[str, Any]],
    output: Path,
) -> tuple[bytes, bytes, dict[str, Any]]:
    chosen = {_target(api): reports[(mode, api)] for api in APIS}
    lines: list[bytes] = []
    observed: set[str] = set()
    counts = Counter(row["outcome"] for row in base_rows)
    for original, row in zip(base_lines, base_rows):
        node_id = row["id"]
        if node_id in chosen:
            report = chosen[node_id]
            changed = {
                **row,
                "outcome": "passed",
                "attempts": 1,
                "durationSeconds": report["_duration"],
                "resultFile": report["result"],
            }
            lines.append(_line(changed))
            observed.add(node_id)
            counts["not-run"] -= 1
            counts["passed"] += 1
        else:
            lines.append(original)
    _require(observed == set(chosen), f"{mode} successor omitted one of two targets")
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
    summary_bytes = (json.dumps(summary, indent=2) + "\n").encode()
    return ledger, summary_bytes, summary


def _check_output(output_dir: Path) -> None:
    _require(output_dir.is_absolute(), "successor directory must be absolute")
    _require(
        not output_dir.exists() and not output_dir.is_symlink(),
        f"successor directory already exists or is a symlink: {output_dir}",
    )
    _require(
        output_dir.parent.is_dir() and not output_dir.parent.is_symlink(),
        "successor parent is missing or a symlink",
    )
    _require(
        not any(parent.is_symlink() for parent in output_dir.parents),
        "successor path contains a symlink parent",
    )


def prepare(
    sidecars: dict[tuple[str, str], Path], output_dir: Path
) -> tuple[dict[str, bytes], dict[str, Any]]:
    """Validate all source evidence and prepare both immutable successors in memory."""
    _require(output_dir.is_absolute(), "successor directory must be absolute")
    inventory = _inventory()
    reports = _reports(sidecars)
    content: dict[str, bytes] = {}
    summaries: dict[str, dict[str, Any]] = {}
    bases: dict[str, dict[str, str]] = {}
    for mode in MODES:
        lines, rows, _ = _base(mode, inventory)
        name = f"playwright-python-{mode}-dummy-node-abi119-strict-paired-v1.jsonl"
        ledger, summary_data, summary = _successor(
            mode, lines, rows, reports, output_dir / name
        )
        content[name] = ledger
        content[Path(name).with_suffix(".summary.json").name] = summary_data
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
        "candidateWheelSha256": reports[("candidate", "async")]["wheelSha256"],
        "bases": bases,
        "reports": [
            {
                "mode": mode,
                "api": api,
                "testId": _target(api),
                "provenance": str(sidecars[(mode, api)]),
                "provenanceSha256": reports[(mode, api)]["_sidecarSha256"],
                "result": reports[(mode, api)]["result"],
                "resultSha256": reports[(mode, api)]["resultSha256"],
                "outcome": "passed",
            }
            for mode in MODES
            for api in APIS
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
    """Use the platform no-replace rename primitive for the whole paired bundle."""
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
        _fail("atomic no-replace directory publication is unsupported on this platform")
    if result != 0:
        error = ctypes.get_errno()
        raise PromotionError(
            f"atomic paired successor publication failed: {os.strerror(error)}"
        )


def promote(sidecars: dict[tuple[str, str], Path], output_dir: Path) -> dict[str, Any]:
    """Publish a complete paired bundle or leave the target absent."""
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
    """Read-only recheck of both successors against all four strict sources."""
    _require(
        output_dir.is_dir() and not output_dir.is_symlink(),
        "paired successor directory is missing or a symlink",
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
        for api in APIS:
            parser.add_argument(f"--{mode}-{api}-provenance", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--verify-existing", action="store_true")
    args = parser.parse_args()
    sidecars = {
        (mode, api): getattr(args, f"{mode}_{api}_provenance").absolute()
        for mode in MODES
        for api in APIS
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
        StrictEvidenceError,
        KeyError,
        TypeError,
        AttributeError,
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
