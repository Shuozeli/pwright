#!/usr/bin/env python3
"""Verify the pinned complete 29-case keyboard oracle merge without changing sources.

The receipt binds the unchanged upstream selection, independent schema-5
execution, all isolated cases, and the exact prior strict oracle ledger. The
resulting successor must change only those 29 invocation rows.
"""

from __future__ import annotations

import argparse
import json
import xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path
from typing import Any

import promote_playwright_python_verified_subset as common
from verify_playwright_python_official_provenance import verify_provenance

SOURCE = "tests/async/test_keyboard.py"
PINNED_SHA256 = {
    "manifest": "35c8c97c20affae0129c14b7e7f0917d592825b235cd3612013ced7cfd776197",
    "oracleBase": "63b2962367ce219c0d174514f21fcd5c3a7b18940b6d829b3e55fbd9ba11cd68",
    "provenance": "8f4af08d2e3c9cc691ccea2fde90b6d7e5418f106bd7a6f6d69028a58d3050a6",
    "report": "e5862c438f7faa375b300c0ab2b5018132e2ef001b68d2803025e4e316e572b5",
    "oraclePackage": "d7655707035107f13a81db0e1d52d839c2bf42bf2008478fba610299cd6da149",
}
KIND = "playwright-python-keyboard-29-case-source-bound-oracle-merge"


def _pinned_file(path: Path, key: str) -> str:
    digest = common._file_sha(path)
    common._require(digest == PINNED_SHA256[key], f"{key} differs from pinned SHA-256")
    return digest


def _manifest_ids(path: Path, upstream: Path) -> list[str]:
    manifest = common._json_file(path)
    cases = manifest.get("cases")
    common._require(
        manifest.get("schema") == 1 and isinstance(cases, list) and len(cases) == 29,
        "keyboard manifest does not select exactly 29 cases",
    )
    ids: list[str] = []
    for entry in cases:
        common._require(isinstance(entry, dict), "malformed keyboard manifest case")
        node_id = entry.get("id")
        common._require(
            isinstance(node_id, str)
            and node_id.startswith(f"{SOURCE}::")
            and "[" not in node_id,
            f"unexpected keyboard case: {node_id}",
        )
        ids.append(f"{node_id}[chromium]")
    common._require(len(set(ids)) == 29, "keyboard manifest case IDs are not unique")
    files = manifest.get("files")
    common._require(
        isinstance(files, dict) and SOURCE in files, "keyboard source hash missing"
    )
    upstream = upstream.resolve()
    for relative, digest in files.items():
        common._require(
            isinstance(relative, str) and isinstance(digest, str),
            "malformed keyboard source hash",
        )
        source = upstream / relative
        common._require(
            source.resolve().is_relative_to(upstream), "source escapes checkout"
        )
        common._require(
            common._file_sha(source) == digest, f"upstream source drift: {relative}"
        )
    return ids


def _prepare(spec: dict[str, str]) -> dict[str, Any]:
    paths = {key: Path(value).resolve() for key, value in spec.items()}
    manifest_sha = _pinned_file(paths["manifest"], "manifest")
    provenance_sha = _pinned_file(paths["provenance"], "provenance")
    sidecar = common._json_file(paths["provenance"])
    payload = verify_provenance(sidecar, provenance_path=paths["provenance"])
    report = paths["provenance"].with_name(
        paths["provenance"].name.removesuffix(".provenance.json") + ".xml"
    )
    common._require(
        paths["provenance"].name.endswith(".provenance.json")
        and Path(str(payload.get("result"))).resolve() == report,
        "oracle report is not provenance sibling",
    )
    report_sha = _pinned_file(report, "report")
    common._require(
        payload.get("schema") == 5
        and payload.get("mode") == "oracle"
        and payload.get("manifestCases") is True
        and payload.get("manifestApi") == "async"
        and payload.get("isolateCases") is True
        and payload.get("isolatedCaseCount") == 29
        and payload.get("browsers") == ["chromium"]
        and payload.get("launchMode") == "cdp-adapter"
        and payload.get("browserChannel") == "chrome"
        and payload.get("exitCode") == 0
        and payload.get("manifestSha256") == manifest_sha
        and Path(str(payload.get("manifest"))).resolve() == paths["manifest"]
        and payload.get("playwrightPackageSha256") == PINNED_SHA256["oraclePackage"],
        "oracle keyboard schema-5 execution profile drift",
    )
    ids = _manifest_ids(paths["manifest"], Path(str(payload["upstream"])))
    _, _, cases = common._report(report, ids)
    common._isolated_reports(Path(str(payload["caseReports"])), ids, cases)
    statuses = {node_id: common._status(cases[node_id]) for node_id in ids}
    common._require(
        Counter(statuses.values()) == {"passed": 28, "skipped": 1},
        f"oracle keyboard result differs from observed 28/1: {dict(Counter(statuses.values()))}",
    )
    inventory = str(Path(str(payload["inventory"])).resolve())
    base_sha = _pinned_file(paths["oracleBase"], "oracleBase")
    common._ledger(
        paths["oracleBase"],
        {node_id: "not-run" for node_id in ids},
        inventory,
        excluded_ids=frozenset(),
    )
    return {
        "schema": 1,
        "kind": KIND,
        "manifest": str(paths["manifest"]),
        "manifestSha256": manifest_sha,
        "inventory": inventory,
        "inventorySha256": payload["inventorySha256"],
        "provenance": str(paths["provenance"]),
        "provenanceSha256": provenance_sha,
        "report": str(report),
        "reportSha256": report_sha,
        "oraclePackageSha256": PINNED_SHA256["oraclePackage"],
        "oracleBase": str(paths["oracleBase"]),
        "oracleBaseSha256": base_sha,
        "ids": ids,
        "statuses": [{"id": node_id, "outcome": statuses[node_id]} for node_id in ids],
        "expectedTransitions": {"not-run->passed": 28, "not-run->skipped": 1},
    }


def create(spec: dict[str, str], receipt_path: Path) -> dict[str, Any]:
    receipt = _prepare(spec)
    receipt_path = receipt_path.resolve()
    common._require(
        receipt_path.parent.is_dir()
        and not receipt_path.exists()
        and not receipt_path.is_symlink(),
        "receipt destination exists or parent is missing",
    )
    with receipt_path.open("xb") as output:
        output.write((json.dumps(receipt, sort_keys=True, indent=2) + "\n").encode())
    return receipt


def verify(receipt_path: Path) -> dict[str, Any]:
    observed = common._json_file(receipt_path)
    common._require(
        observed.get("schema") == 1 and observed.get("kind") == KIND,
        "unsupported keyboard oracle receipt",
    )
    spec = {key: observed[key] for key in ("manifest", "provenance", "oracleBase")}
    expected = _prepare(spec)
    common._require(
        observed == expected, "keyboard receipt differs from source evidence"
    )
    return observed


def verify_ledger(receipt_path: Path, ledger_path: Path) -> dict[str, Any]:
    receipt = verify(receipt_path)
    ledger_path = ledger_path.resolve()
    base_path = Path(receipt["oracleBase"])
    common._require(ledger_path != base_path, "oracle successor overwrote base")
    statuses = {entry["id"]: entry["outcome"] for entry in receipt["statuses"]}
    digest = common._ledger(
        ledger_path, statuses, receipt["inventory"], excluded_ids=frozenset()
    )
    _, _, cases = common._report(Path(receipt["report"]), receipt["ids"])
    before = [json.loads(line) for line in base_path.read_text().splitlines()]
    after = [json.loads(line) for line in ledger_path.read_text().splitlines()]
    common._require(
        len(before) == len(after) == 6526, "keyboard ledger row count drift"
    )
    transitions: Counter[str] = Counter()
    for index, (base_row, row) in enumerate(zip(before, after)):
        node_id = base_row["id"]
        if node_id in statuses:
            expected = common._expected_selected_row(
                base_row, cases[node_id], Path(receipt["report"])
            )
            common._require(
                row == expected, f"keyboard selected row differs at {index}: {node_id}"
            )
            transitions[f"{base_row['outcome']}->{row['outcome']}"] += 1
        else:
            common._require(
                row == base_row, f"unrelated oracle row differs at {index}: {node_id}"
            )
    common._require(
        dict(sorted(transitions.items())) == receipt["expectedTransitions"],
        "keyboard successor transitions differ from receipt",
    )
    return {
        "verified": True,
        "receipt": str(receipt_path.resolve()),
        "ledger": str(ledger_path),
        "ledgerSha256": digest,
        "transitions": dict(sorted(transitions.items())),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    create_parser = sub.add_parser("create")
    for name in ("manifest", "provenance", "oracle-base", "receipt"):
        create_parser.add_argument(f"--{name}", type=Path, required=True)
    verify_parser = sub.add_parser("verify")
    verify_parser.add_argument("--receipt", type=Path, required=True)
    ledger_parser = sub.add_parser("verify-ledger")
    ledger_parser.add_argument("--receipt", type=Path, required=True)
    ledger_parser.add_argument("--ledger", type=Path, required=True)
    args = parser.parse_args()
    try:
        if args.command == "create":
            receipt = create(
                {
                    "manifest": str(args.manifest),
                    "provenance": str(args.provenance),
                    "oracleBase": str(args.oracle_base),
                },
                args.receipt,
            )
            result = {
                "verified": True,
                "receipt": str(args.receipt.resolve()),
                "selected": len(receipt["ids"]),
            }
        elif args.command == "verify":
            receipt = verify(args.receipt)
            result = {
                "verified": True,
                "receipt": str(args.receipt.resolve()),
                "selected": len(receipt["ids"]),
            }
        else:
            result = verify_ledger(args.receipt, args.ledger)
    except (
        common.SelectionError,
        OSError,
        ValueError,
        KeyError,
        TypeError,
        ET.ParseError,
    ) as error:
        parser.exit(2, f"source-bound keyboard oracle merge rejected: {error}\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
