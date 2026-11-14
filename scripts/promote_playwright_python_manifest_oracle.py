#!/usr/bin/env python3
"""Source-bound promotion of a complete pinned Python oracle manifest.

The receipt binds every selected case to the unchanged official report and
isolated execution. A successor is valid only when selected rows match the
report exactly and every other inventory row remains byte-for-byte identical.
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

KIND = "playwright-python-complete-manifest-oracle-source-bound-merge"
PINNED_REPOSITORY = "https://github.com/microsoft/playwright-python.git"
PINNED_TAG = "v1.58.0"
PINNED_COMMIT = "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
PINNED_INVOCATION_SHA256 = (
    "31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090"
)


def _manifest_ids(path: Path, upstream: Path) -> list[str]:
    manifest = common._json_file(path)
    common._require(
        manifest.get("schema") == 1
        and manifest.get("repository") == PINNED_REPOSITORY
        and manifest.get("tag") == PINNED_TAG
        and manifest.get("commit") == PINNED_COMMIT,
        "oracle manifest does not pin the selected official source",
    )
    cases, files = manifest.get("cases"), manifest.get("files")
    common._require(
        isinstance(cases, list)
        and bool(cases)
        and isinstance(files, dict)
        and bool(files),
        "oracle manifest has no explicit cases or source hashes",
    )
    ids: list[str] = []
    for entry in cases:
        common._require(isinstance(entry, dict), "malformed oracle manifest case")
        node_id = entry.get("id")
        common._require(
            isinstance(node_id, str)
            and node_id.count("::") == 1
            and node_id.startswith("tests/async/")
            and "[" not in node_id
            and node_id.split("::", 1)[0] in files,
            f"unsupported oracle manifest case: {node_id}",
        )
        ids.append(f"{node_id}[chromium]")
    common._require(len(set(ids)) == len(ids), "duplicate oracle manifest case")
    upstream = upstream.resolve()
    for relative, digest in files.items():
        common._require(
            isinstance(relative, str) and isinstance(digest, str),
            "malformed oracle source hash",
        )
        source = upstream / relative
        common._require(
            source.resolve().is_relative_to(upstream), "oracle source escapes checkout"
        )
        common._require(
            common._file_sha(source) == digest, f"upstream source drift: {relative}"
        )
    return ids


def _audit_selection(
    path: Path, base: Path, ids: list[str]
) -> tuple[str, list[str], list[dict[str, str]]]:
    audit_sha = common._file_sha(path)
    audit_summary = common._json_file(path.with_suffix(".summary.json"))
    common._require(
        audit_sha == PINNED_INVOCATION_SHA256
        and audit_summary.get("ledgerSha256") == audit_sha
        and audit_summary.get("outcomeLedger") == str(base)
        and audit_summary.get("outcomeLedgerSha256") == common._file_sha(base)
        and audit_summary.get("records") == 6526,
        "invocation audit is not pinned to the exact oracle base",
    )
    base_rows = [json.loads(line) for line in base.read_text().splitlines()]
    audit_rows = [json.loads(line) for line in path.read_text().splitlines()]
    common._require(
        len(base_rows) == len(audit_rows) == 6526,
        "invocation audit is not aligned to the complete Python corpus",
    )
    wanted = set(ids)
    classifications: dict[str, dict[str, str]] = {}
    for index, (base_row, audit_row) in enumerate(zip(base_rows, audit_rows)):
        node_id = base_row.get("id")
        common._require(
            node_id == audit_row.get("id"), f"audit ID differs at row {index}"
        )
        if node_id in wanted:
            common._require(
                base_row.get("api") == "async", f"not an async case: {node_id}"
            )
            classifications[node_id] = {
                "id": node_id,
                "applicability": str(audit_row.get("applicability")),
                "invocationClassification": str(
                    audit_row.get("invocationClassification")
                ),
            }
    common._require(set(classifications) == wanted, "audit lacks manifest IDs")
    selected: list[str] = []
    excluded: list[dict[str, str]] = []
    for node_id in ids:
        classification = classifications[node_id]
        applicability = classification["applicability"]
        invocation = classification["invocationClassification"]
        if applicability == "applicable" and invocation.startswith("public-api-"):
            selected.append(node_id)
        elif (
            applicability == "non-applicable"
            and invocation == "implementation-internal"
        ):
            excluded.append(classification)
        else:
            raise common.SelectionError(
                f"unclassified oracle manifest applicability: {node_id}"
            )
    common._require(bool(selected), "manifest has no applicable public cases")
    return audit_sha, selected, excluded


def _prepare(spec: dict[str, str]) -> dict[str, Any]:
    paths = {key: Path(value).resolve() for key, value in spec.items()}
    provenance_path = paths["provenance"]
    provenance_sha = common._file_sha(provenance_path)
    payload = verify_provenance(
        common._json_file(provenance_path), provenance_path=provenance_path
    )
    manifest = paths["manifest"]
    manifest_sha = common._file_sha(manifest)
    ids = _manifest_ids(manifest, Path(str(payload["upstream"])))
    report = provenance_path.with_name(
        provenance_path.name.removesuffix(".provenance.json") + ".xml"
    )
    common._require(
        provenance_path.name.endswith(".provenance.json")
        and payload.get("schema") == 5
        and payload.get("mode") == "oracle"
        and payload.get("manifestCases") is True
        and payload.get("manifestApi") == "async"
        and payload.get("isolateCases") is True
        and payload.get("isolatedCaseCount") == len(ids)
        and payload.get("browsers") == ["chromium"]
        and payload.get("launchMode") == "cdp-adapter"
        and payload.get("browserChannel") == "chrome"
        and payload.get("exitCode") == 0
        and payload.get("manifestSha256") == manifest_sha
        and Path(str(payload.get("manifest"))).resolve() == manifest
        and Path(str(payload.get("result"))).resolve() == report
        and payload.get("repository") == PINNED_REPOSITORY
        and payload.get("tag") == PINNED_TAG
        and payload.get("commit") == PINNED_COMMIT,
        "oracle evidence differs from complete pinned manifest profile",
    )
    report_sha = common._file_sha(report)
    root, suite, cases = common._report(report, ids)
    common._isolated_reports(Path(str(payload["caseReports"])), ids, cases)
    statuses = {node_id: common._status(cases[node_id]) for node_id in ids}
    inventory = str(Path(str(payload["inventory"])).resolve())
    base = paths["oracleBase"]
    base_sha = common._ledger(base, {}, inventory, excluded_ids=frozenset())
    audit_sha, selected_ids, excluded = _audit_selection(
        paths["invocationAudit"], base, ids
    )
    filtered = common._filtered_xml(root, suite, selected_ids, cases)
    base_rows = {
        row["id"]: row
        for row in (json.loads(line) for line in base.read_text().splitlines())
    }
    transitions: Counter[str] = Counter()
    for node_id in selected_ids:
        before = base_rows[node_id]
        prior = before["outcome"]
        common._require(
            prior != "not-run" or before.get("attempts") == 0,
            f"pristine oracle row has an attempt: {node_id}",
        )
        transitions[f"{prior}->{statuses[node_id]}"] += 1
    common._require(
        sum(
            count
            for label, count in transitions.items()
            if label.startswith("not-run->")
        )
        > 0,
        "oracle manifest does not add any previously unrun cases",
    )
    return {
        "schema": 1,
        "kind": KIND,
        "manifest": str(manifest),
        "manifestSha256": manifest_sha,
        "provenance": str(provenance_path),
        "provenanceSha256": provenance_sha,
        "report": str(report),
        "reportSha256": report_sha,
        "filteredReport": str(paths["filteredReport"]),
        "filteredReportSha256": common._sha(filtered),
        "oraclePackageSha256": payload["playwrightPackageSha256"],
        "inventory": inventory,
        "inventorySha256": payload["inventorySha256"],
        "oracleBase": str(base),
        "oracleBaseSha256": base_sha,
        "invocationAudit": str(paths["invocationAudit"]),
        "invocationAuditSha256": audit_sha,
        "ids": ids,
        "selectedIds": selected_ids,
        "excludedIds": excluded,
        "statuses": [{"id": node_id, "outcome": statuses[node_id]} for node_id in ids],
        "expectedTransitions": dict(sorted(transitions.items())),
    }


def _filtered_bytes(receipt: dict[str, Any]) -> bytes:
    root, suite, cases = common._report(Path(receipt["report"]), receipt["ids"])
    return common._filtered_xml(root, suite, receipt["selectedIds"], cases)


def create(spec: dict[str, str], receipt_path: Path) -> dict[str, Any]:
    receipt = _prepare(spec)
    receipt_path = receipt_path.resolve()
    filtered_path = Path(receipt["filteredReport"])
    common._require(
        receipt_path.parent.is_dir()
        and not receipt_path.exists()
        and not receipt_path.is_symlink(),
        "receipt destination exists or parent is missing",
    )
    common._require(
        filtered_path.parent.is_dir()
        and not filtered_path.exists()
        and not filtered_path.is_symlink()
        and filtered_path not in {receipt_path, Path(receipt["report"])},
        "filtered report destination exists or overlaps evidence",
    )
    filtered = _filtered_bytes(receipt)
    common._require(
        common._sha(filtered) == receipt["filteredReportSha256"],
        "filtered report differs from evidence",
    )
    with filtered_path.open("xb") as output:
        output.write(filtered)
    with receipt_path.open("xb") as output:
        output.write((json.dumps(receipt, indent=2, sort_keys=True) + "\n").encode())
    return receipt


def verify(receipt_path: Path) -> dict[str, Any]:
    observed = common._json_file(receipt_path)
    common._require(
        observed.get("schema") == 1 and observed.get("kind") == KIND,
        "unsupported complete-manifest oracle receipt",
    )
    spec = {
        key: observed[key]
        for key in (
            "manifest",
            "provenance",
            "oracleBase",
            "invocationAudit",
            "filteredReport",
        )
    }
    common._require(
        observed == _prepare(spec), "oracle receipt differs from source evidence"
    )
    common._require(
        common._file_sha(Path(observed["filteredReport"]))
        == observed["filteredReportSha256"]
        and Path(observed["filteredReport"]).read_bytes() == _filtered_bytes(observed),
        "filtered report differs from complete oracle evidence",
    )
    return observed


def verify_ledger(receipt_path: Path, ledger_path: Path) -> dict[str, Any]:
    receipt = verify(receipt_path)
    ledger_path = ledger_path.resolve()
    base = Path(receipt["oracleBase"])
    common._require(ledger_path != base, "oracle successor overwrote its base")
    digest = common._ledger(
        ledger_path, {}, receipt["inventory"], excluded_ids=frozenset()
    )
    ids = receipt["selectedIds"]
    filtered = Path(receipt["filteredReport"])
    _, _, cases = common._report(filtered, ids)
    selected = set(ids)
    before = [json.loads(line) for line in base.read_text().splitlines()]
    after = [json.loads(line) for line in ledger_path.read_text().splitlines()]
    common._require(len(before) == len(after) == 6526, "oracle ledger row count drift")
    transitions: Counter[str] = Counter()
    for index, (base_row, row) in enumerate(zip(before, after)):
        node_id = base_row["id"]
        if node_id in selected:
            expected = common._expected_selected_row(base_row, cases[node_id], filtered)
            common._require(row == expected, f"selected oracle row differs at {index}")
            transitions[f"{base_row['outcome']}->{row['outcome']}"] += 1
        else:
            common._require(row == base_row, f"unrelated oracle row differs at {index}")
    common._require(
        dict(sorted(transitions.items())) == receipt["expectedTransitions"],
        "oracle transitions differ from source-bound receipt",
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
    for name in (
        "manifest",
        "provenance",
        "oracle-base",
        "invocation-audit",
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
            result = create(
                {
                    "manifest": str(args.manifest),
                    "provenance": str(args.provenance),
                    "oracleBase": str(args.oracle_base),
                    "invocationAudit": str(args.invocation_audit),
                    "filteredReport": str(args.filtered_report),
                },
                args.receipt,
            )
            output = {
                "verified": True,
                "receipt": str(args.receipt.resolve()),
                "selected": len(result["selectedIds"]),
            }
        elif args.command == "verify":
            result = verify(args.receipt)
            output = {
                "verified": True,
                "receipt": str(args.receipt.resolve()),
                "selected": len(result["selectedIds"]),
            }
        else:
            output = verify_ledger(args.receipt, args.ledger)
    except (
        common.SelectionError,
        OSError,
        ValueError,
        KeyError,
        TypeError,
        ET.ParseError,
    ) as error:
        parser.exit(
            2, f"source-bound complete-manifest oracle merge rejected: {error}\n"
        )
    print(json.dumps(output, indent=2))


if __name__ == "__main__":
    main()
