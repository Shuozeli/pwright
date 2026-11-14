#!/usr/bin/env python3
"""Source-bound refresh of the 35 oracle-passing BrowserContext candidate rows.

The new complete 39-case candidate report is immutable. Its four cases that
failed in the independent oracle stay not-run in the strict ledgers. This tool
derives one candidate-only filtered report and verifies the exact successor;
it never changes the independent oracle ledger or the prior 35-case receipt.
"""

from __future__ import annotations

import argparse
import json
import xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path
from typing import Any

import promote_playwright_python_verified_subset as prior
from verify_playwright_python_official_provenance import verify_provenance

PINNED_SHA256 = {
    "priorReceipt": "2858b98b3b8ca5be99496367c1746f54b782a946fe9acc01a26e101e83c4f77b",
    "candidateBase": "a44a39ac38f15afb07bd249f7a9f748cc1c47fd5b1c1d068dce574b5b74c21f6",
    "oracleLedger": "63b2962367ce219c0d174514f21fcd5c3a7b18940b6d829b3e55fbd9ba11cd68",
    "candidateProvenance": "d9c82dd0527bb25504d6020c8f93dd67b4cc73de2776fc22617f828de39f3638",
    "candidateReport": "98fcd60681bdf26b38a075cc3526e0fc3e89fecfb02b2e89773e4ac0bac7e7d4",
    "candidateArtifact": "9b89957193a8a1c0aef2075eb25698b60e62a7dd61f57359cbc8e417fc6bb4ff",
}
KIND = "playwright-python-browsercontext-35-of-39-candidate-source-bound-refresh"


def _require_pinned(path: Path, key: str) -> str:
    digest = prior._file_sha(path)
    prior._require(
        digest == PINNED_SHA256[key], f"{key} SHA-256 differs from pinned evidence"
    )
    return digest


def _parent_report(sidecar: Path) -> Path:
    name = sidecar.name
    prior._require(
        name.endswith(".provenance.json"), "not an official provenance sibling"
    )
    return sidecar.with_name(name.removesuffix(".provenance.json") + ".xml")


def _prepare(spec: dict[str, str]) -> tuple[dict[str, Any], bytes]:
    paths = {key: Path(value).resolve() for key, value in spec.items()}
    previous_path = paths["priorReceipt"]
    previous_sha = _require_pinned(previous_path, "priorReceipt")
    previous = prior.verify(previous_path)
    candidate_base = paths["candidateBase"]
    oracle_ledger = paths["oracleLedger"]
    _require_pinned(candidate_base, "candidateBase")
    _require_pinned(oracle_ledger, "oracleLedger")
    prior.verify_ledgers(previous_path, candidate_base, oracle_ledger)

    manifest = paths["manifest"]
    prior._require(
        manifest == Path(previous["manifest"]).resolve(),
        "refresh manifest differs from prior receipt",
    )
    parent_sidecar = paths["candidateProvenance"]
    sidecar_sha = _require_pinned(parent_sidecar, "candidateProvenance")
    raw = prior._json_file(parent_sidecar)
    payload = verify_provenance(raw, provenance_path=parent_sidecar)
    parent_report = _parent_report(parent_sidecar)
    report_sha = _require_pinned(parent_report, "candidateReport")
    prior._require(
        Path(str(payload.get("result"))).resolve() == parent_report,
        "candidate report differs from sidecar",
    )
    prior._require(
        payload.get("schema") == 5
        and payload.get("mode") == "candidate"
        and payload.get("manifestCases") is True
        and payload.get("manifestApi") == "async"
        and payload.get("isolateCases") is True
        and payload.get("isolatedCaseCount") == 39
        and payload.get("browsers") == ["chromium"]
        and payload.get("launchMode") == "cdp-adapter"
        and payload.get("browserChannel") == "chrome"
        and payload.get("manifestSha256") == previous["manifestSha256"]
        and Path(str(payload.get("manifest"))).resolve() == manifest
        and payload.get("exitCode") == 1
        and payload.get("pwrightArtifactSha256") == PINNED_SHA256["candidateArtifact"],
        "candidate 39-case execution profile or artifact drift",
    )
    original_oracle = prior._json_file(Path(previous["oracle"]["parentProvenance"]))
    for field in (
        "commit",
        "tag",
        "repository",
        "manifestSha256",
        "fixtureHost",
        "fixtureBind",
        "isolatedCaseIdsSha256",
        "caseTimeout",
        "inventory",
        "inventorySha256",
    ):
        prior._require(
            payload.get(field) == original_oracle.get(field),
            f"candidate/oracle {field} differs",
        )

    ids = prior._manifest_ids(manifest, Path(str(payload["upstream"])))
    selected_ids = previous["selectedIds"]
    prior._require(
        len(ids) == 39
        and len(selected_ids) == 35
        and selected_ids
        == [node_id for node_id in ids if node_id not in prior.EXCLUDED],
        "refresh selection differs from original oracle-passing 35",
    )
    root, suite, cases = prior._report(parent_report, ids)
    prior._isolated_reports(Path(str(payload["caseReports"])), ids, cases)
    statuses = {node_id: prior._status(cases[node_id]) for node_id in ids}
    selected_counts = Counter(statuses[node_id] for node_id in selected_ids)
    prior._require(
        selected_counts == {"passed": 29, "failed": 6},
        f"new candidate profile differs from observed 29/6: {dict(selected_counts)}",
    )
    excluded_counts = Counter(statuses[node_id] for node_id in prior.EXCLUDED)
    prior._require(
        excluded_counts == {"passed": 3, "failed": 1},
        f"excluded popup profile differs from observed 3/1: {dict(excluded_counts)}",
    )
    previous_statuses = {
        entry["id"]: entry["outcome"]
        for entry in previous["candidate"]["selectedStatuses"]
    }
    prior._require(
        len(previous_statuses) == 35 and set(previous_statuses) == set(selected_ids),
        "prior receipt does not account for the same 35 candidate cases",
    )
    base_sha = prior._ledger(
        candidate_base, previous_statuses, str(Path(previous["inventory"]).resolve())
    )
    filtered = prior._filtered_xml(root, suite, selected_ids, cases)
    transitions = dict(
        sorted(
            Counter(
                f"{previous_statuses[node_id]}->{statuses[node_id]}"
                for node_id in selected_ids
            ).items()
        )
    )
    receipt = {
        "schema": 2,
        "kind": KIND,
        "manifest": str(manifest),
        "manifestSha256": previous["manifestSha256"],
        "inventory": str(Path(previous["inventory"]).resolve()),
        "inventorySha256": previous["inventorySha256"],
        "priorReceipt": str(previous_path),
        "priorReceiptSha256": previous_sha,
        "candidateBase": str(candidate_base),
        "candidateBaseSha256": base_sha,
        "oracleLedger": str(oracle_ledger),
        "oracleLedgerSha256": PINNED_SHA256["oracleLedger"],
        "candidateProvenance": str(parent_sidecar),
        "candidateProvenanceSha256": sidecar_sha,
        "candidateParentReport": str(parent_report),
        "candidateParentReportSha256": report_sha,
        "candidateArtifactSha256": PINNED_SHA256["candidateArtifact"],
        "selectedIds": selected_ids,
        "excludedIds": sorted(prior.EXCLUDED),
        "originalStatuses": [
            {"id": node_id, "candidate": statuses[node_id]} for node_id in ids
        ],
        "selectedStatuses": [
            {"id": node_id, "outcome": statuses[node_id]} for node_id in selected_ids
        ],
        "transitions": transitions,
        "filteredReport": str(paths["filteredReport"]),
        "filteredReportSha256": prior._sha(filtered),
    }
    return receipt, filtered


def create(spec: dict[str, str], receipt_path: Path) -> dict[str, Any]:
    receipt, filtered = _prepare(spec)
    receipt_path = receipt_path.resolve()
    report_path = Path(receipt["filteredReport"])
    prior._require(receipt_path != report_path, "refresh output paths collide")
    prior._require(
        all(
            path.parent.is_dir() and not path.exists() and not path.is_symlink()
            for path in (report_path, receipt_path)
        ),
        "refresh destination exists or parent is missing",
    )
    with report_path.open("xb") as output:
        output.write(filtered)
    with receipt_path.open("xb") as output:
        output.write((json.dumps(receipt, sort_keys=True, indent=2) + "\n").encode())
    return receipt


def verify(receipt_path: Path) -> dict[str, Any]:
    observed = prior._json_file(receipt_path)
    prior._require(
        observed.get("schema") == 2 and observed.get("kind") == KIND,
        "unsupported candidate refresh receipt",
    )
    spec = {
        key: observed[key]
        for key in (
            "manifest",
            "priorReceipt",
            "candidateBase",
            "oracleLedger",
            "candidateProvenance",
            "filteredReport",
        )
    }
    expected, filtered = _prepare(spec)
    prior._require(
        observed == expected, "candidate refresh receipt differs from source evidence"
    )
    path = Path(observed["filteredReport"])
    prior._require(
        prior._file_sha(path) == observed["filteredReportSha256"]
        and path.read_bytes() == filtered,
        "candidate refresh filtered report differs from source evidence",
    )
    return observed


def verify_ledger(receipt_path: Path, output_path: Path) -> dict[str, Any]:
    receipt = verify(receipt_path)
    output_path = output_path.resolve()
    base_path = Path(receipt["candidateBase"])
    oracle_path = Path(receipt["oracleLedger"])
    prior._require(
        output_path not in {base_path, oracle_path}, "refresh ledger overwrote base"
    )
    selected_ids = receipt["selectedIds"]
    selected = {entry["id"]: entry["outcome"] for entry in receipt["selectedStatuses"]}
    digest = prior._ledger(output_path, selected, receipt["inventory"])
    report_path = Path(receipt["filteredReport"])
    _, _, cases = prior._report(report_path, selected_ids)
    before = [json.loads(line) for line in base_path.read_text().splitlines()]
    after = [json.loads(line) for line in output_path.read_text().splitlines()]
    prior._require(len(before) == len(after) == 6526, "refresh ledger row count drift")
    changes: Counter[str] = Counter()
    for index, (base_row, row) in enumerate(zip(before, after)):
        node_id = base_row["id"]
        if node_id in selected:
            expected = prior._expected_selected_row(
                base_row, cases[node_id], report_path
            )
            prior._require(
                row == expected, f"selected refresh row differs at {index}: {node_id}"
            )
            changes[f"{base_row['outcome']}->{row['outcome']}"] += 1
        else:
            prior._require(
                row == base_row, f"unrelated refresh row differs at {index}: {node_id}"
            )
    prior._require(
        dict(sorted(changes.items())) == receipt["transitions"]
        and sum(changes.values()) == 35,
        "refresh ledger transitions differ from receipt",
    )
    return {
        "verified": True,
        "receipt": str(receipt_path.resolve()),
        "ledger": str(output_path),
        "ledgerSha256": digest,
        "transitions": dict(sorted(changes.items())),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    create_parser = sub.add_parser("create")
    for name in (
        "manifest",
        "prior-receipt",
        "candidate-base",
        "oracle-ledger",
        "candidate-provenance",
        "filtered-report",
        "receipt",
    ):
        create_parser.add_argument(f"--{name}", type=Path, required=True)
    verify_parser = sub.add_parser("verify")
    verify_parser.add_argument("--receipt", type=Path, required=True)
    ledger_parser = sub.add_parser("verify-ledger")
    ledger_parser.add_argument("--receipt", type=Path, required=True)
    ledger_parser.add_argument("--ledger", type=Path, required=True)
    args = parser.parse_args()
    try:
        if args.command == "create":
            spec = {
                "manifest": str(args.manifest.resolve()),
                "priorReceipt": str(args.prior_receipt.resolve()),
                "candidateBase": str(args.candidate_base.resolve()),
                "oracleLedger": str(args.oracle_ledger.resolve()),
                "candidateProvenance": str(args.candidate_provenance.resolve()),
                "filteredReport": str(args.filtered_report.resolve()),
            }
            receipt = create(spec, args.receipt)
            result = {
                "verified": True,
                "receipt": str(args.receipt.resolve()),
                "transitions": receipt["transitions"],
            }
        elif args.command == "verify":
            receipt = verify(args.receipt)
            result = {
                "verified": True,
                "receipt": str(args.receipt.resolve()),
                "transitions": receipt["transitions"],
            }
        else:
            result = verify_ledger(args.receipt, args.ledger)
    except (
        prior.SelectionError,
        OSError,
        ValueError,
        KeyError,
        TypeError,
        ET.ParseError,
    ) as error:
        parser.exit(2, f"source-bound candidate refresh rejected: {error}\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
