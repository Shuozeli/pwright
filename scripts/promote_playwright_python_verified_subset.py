#!/usr/bin/env python3
"""Derive and verify the pinned 35-case BrowserContext tranche without writing a ledger.

The two original 39-case schema-5 reports and their sidecars remain authoritative.
This tool only writes new, exclusive-create filtered reports and a derivation receipt.
Run ``verify`` immediately before feeding either filtered report to the ledger summarizer.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import math
import xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path
from typing import Any

from verify_playwright_python_official_provenance import verify_provenance

SOURCE = "tests/async/test_browsercontext.py"
GENERATED_FIELDS = frozenset(
    {"attempts", "durationSeconds", "error", "outcome", "resultFile"}
)
EXCLUDED = frozenset(
    f"{SOURCE}::{name}[chromium]"
    for name in (
        "test_page_event_should_have_an_opener",
        "test_page_event_should_have_url",
        "test_page_event_should_have_url_after_domcontentloaded",
        "test_page_event_should_report_when_a_new_page_is_created_and_closed",
    )
)
PINNED_BASE_SHA256 = {
    "candidate": "51cb64812fa743773f78288c6d2b96b5bf1d1c66385b665f1002511280094e83",
    "oracle": "fc9798778600c0a2ee8bea947a0c9e48dc37f3deb42fdada5516d50f9754bb2b",
}


class SelectionError(ValueError):
    """The recorded evidence does not support the requested derivation."""


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise SelectionError(message)


def _sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _file_sha(path: Path) -> str:
    _require(
        path.is_file() and not path.is_symlink(), f"missing or symlinked file: {path}"
    )
    return _sha(path.read_bytes())


def _json_file(path: Path) -> dict[str, Any]:
    _file_sha(path)
    value = json.loads(path.read_bytes())
    _require(isinstance(value, dict), f"JSON root is not an object: {path}")
    return value


def _node_key(node_id: str) -> tuple[str, str]:
    parts = node_id.split("::")
    _require(
        len(parts) == 2 and parts[0].endswith(".py"), f"unsupported node ID: {node_id}"
    )
    return parts[0][:-3].replace("/", "."), parts[1]


def _case_key(case: ET.Element) -> tuple[str, str]:
    name, classname = case.get("name"), case.get("classname")
    _require(bool(name and classname), "JUnit testcase lacks classname/name")
    return str(classname), str(name)


def _status(case: ET.Element) -> str:
    found = [
        kind for kind in ("failure", "error", "skipped") if case.find(kind) is not None
    ]
    _require(
        len(found) <= 1, f"JUnit testcase has conflicting outcomes: {_case_key(case)}"
    )
    return {"failure": "failed", "error": "error", "skipped": "skipped"}.get(
        found[0] if found else "", "passed"
    )


def _report(
    path: Path, expected_ids: list[str]
) -> tuple[ET.Element, ET.Element, dict[str, ET.Element]]:
    _file_sha(path)
    root = ET.parse(path).getroot()
    _require(
        root.tag == "testsuites" and len(root.findall("testsuite")) == 1,
        f"expected one JUnit suite: {path}",
    )
    suite = root.find("testsuite")
    assert suite is not None
    cases = list(root.iter("testcase"))
    _require(
        cases == list(suite.findall("testcase")), f"nested/extra JUnit cases: {path}"
    )
    key_ids = {_node_key(node_id): node_id for node_id in expected_ids}
    _require(len(key_ids) == len(expected_ids), "manifest JUnit keys collide")
    by_id: dict[str, ET.Element] = {}
    for case in cases:
        key = _case_key(case)
        _require(key in key_ids, f"unexpected JUnit testcase in {path}: {key}")
        node_id = key_ids[key]
        _require(node_id not in by_id, f"duplicate JUnit testcase in {path}: {node_id}")
        by_id[node_id] = case
    _require(
        len(cases) == len(expected_ids) and set(by_id) == set(expected_ids),
        f"JUnit rows differ from the exact manifest selection: {path}",
    )
    counts = Counter(_status(case) for case in cases)
    for field, actual in (
        ("tests", len(cases)),
        ("failures", counts["failed"]),
        ("errors", counts["error"]),
        ("skipped", counts["skipped"]),
    ):
        _require(
            suite.get(field) == str(actual), f"JUnit {field} count mismatch: {path}"
        )
    return root, suite, by_id


def _isolated_reports(
    case_dir: Path, expected_ids: list[str], aggregate: dict[str, ET.Element]
) -> None:
    _require(
        case_dir.is_dir() and not case_dir.is_symlink(),
        f"missing case reports: {case_dir}",
    )
    files = sorted(case_dir.glob("*.xml"))
    _require(
        len(files) == len(expected_ids),
        "isolated report file count differs from manifest",
    )
    key_ids = {_node_key(node_id): node_id for node_id in expected_ids}
    observed: dict[str, str] = {}
    for path in files:
        _file_sha(path)
        cases = list(ET.parse(path).getroot().iter("testcase"))
        _require(len(cases) == 1, f"isolated report is not one case: {path}")
        key = _case_key(cases[0])
        _require(key in key_ids, f"isolated case outside manifest: {key}")
        node_id = key_ids[key]
        _require(node_id not in observed, f"duplicate isolated case: {node_id}")
        observed[node_id] = _status(cases[0])
    _require(set(observed) == set(expected_ids), "isolated IDs differ from manifest")
    _require(
        all(
            observed[node_id] == _status(aggregate[node_id]) for node_id in expected_ids
        ),
        "aggregate case status differs from isolated evidence",
    )


def _manifest_ids(manifest_path: Path, upstream: Path) -> list[str]:
    manifest = _json_file(manifest_path)
    _require(manifest.get("schema") == 1, "manifest must use schema 1")
    cases = manifest.get("cases")
    _require(
        isinstance(cases, list) and len(cases) == 39, "manifest must contain 39 cases"
    )
    ids: list[str] = []
    for entry in cases:
        _require(
            isinstance(entry, dict) and isinstance(entry.get("id"), str),
            "malformed manifest case",
        )
        node_id = entry["id"]
        _require(
            node_id.startswith(f"{SOURCE}::") and "[" not in node_id,
            f"unexpected manifest case: {node_id}",
        )
        ids.append(f"{node_id}[chromium]")
    _require(len(set(ids)) == 39, "manifest cases are not unique")
    files = manifest.get("files")
    _require(
        isinstance(files, dict) and bool(files), "manifest lacks source file hashes"
    )
    upstream = upstream.resolve()
    for relative, digest in files.items():
        _require(
            isinstance(relative, str) and isinstance(digest, str),
            "malformed manifest source hash",
        )
        source = upstream / relative
        _require(
            source.resolve().is_relative_to(upstream),
            "manifest source escapes checkout",
        )
        _require(_file_sha(source) == digest, f"upstream source hash drift: {relative}")
    return ids


def _ledger(
    path: Path,
    selected: dict[str, str],
    inventory: str,
    *,
    excluded_ids: frozenset[str] = EXCLUDED,
) -> str:
    digest = _file_sha(path)
    summary_path = path.with_suffix(".summary.json")
    summary = _json_file(summary_path)
    _require(
        summary.get("ledgerSha256") == digest, f"base summary digest drift: {path}"
    )
    _require(summary.get("ledger") == str(path), f"base summary path mismatch: {path}")
    _require(summary.get("inventory") == inventory, f"base inventory mismatch: {path}")
    rows = [json.loads(line) for line in path.read_text().splitlines()]
    _require(
        len(rows) == 6526 and summary.get("records") == 6526,
        f"base is not the 6526-row ledger: {path}",
    )
    inventory_rows = [
        json.loads(line) for line in Path(inventory).read_text().splitlines()
    ]
    _require(
        len(inventory_rows) == len(rows), f"base/inventory row count differs: {path}"
    )
    for index, (record, inventory_record) in enumerate(zip(rows, inventory_rows)):
        _require(
            isinstance(record, dict) and isinstance(inventory_record, dict),
            f"malformed ledger/inventory row {index}: {path}",
        )
        expected_metadata = {
            key: value
            for key, value in inventory_record.items()
            if key not in GENERATED_FIELDS
        }
        observed_metadata = {
            key: value for key, value in record.items() if key not in GENERATED_FIELDS
        }
        _require(
            observed_metadata == expected_metadata,
            f"base metadata differs from inventory at row {index}: {path}",
        )
    by_id: dict[str, dict[str, Any]] = {}
    for row in rows:
        _require(
            isinstance(row, dict) and isinstance(row.get("id"), str),
            f"malformed base row: {path}",
        )
        node_id = row["id"]
        _require(node_id not in by_id, f"duplicate base ID: {node_id}")
        by_id[node_id] = row
    _require(
        dict(sorted(Counter(row.get("outcome") for row in rows).items()))
        == summary.get("outcomes"),
        f"base summary counts drift: {path}",
    )
    for node_id, outcome in selected.items():
        _require(node_id in by_id, f"selected case absent from base: {node_id}")
        row = by_id[node_id]
        _require(
            row.get("sourceClassification") == "public-api-source"
            and row.get("privateImports") == []
            and row.get("api") == "async",
            f"selected case is not public async API: {node_id}",
        )
        _require(
            row.get("outcome") in {"not-run", outcome},
            f"base outcome collision for {node_id}: {row.get('outcome')} -> {outcome}",
        )
    for node_id in excluded_ids:
        _require(
            node_id in by_id and by_id[node_id].get("outcome") == "not-run",
            f"excluded case already has a base outcome: {node_id}",
        )
    return digest


def _filtered_xml(
    root: ET.Element,
    suite: ET.Element,
    selected_ids: list[str],
    cases: dict[str, ET.Element],
) -> bytes:
    filtered_root = copy.deepcopy(root)
    filtered_suite = filtered_root.find("testsuite")
    assert filtered_suite is not None
    for case in list(filtered_suite.findall("testcase")):
        filtered_suite.remove(case)
    chosen = [copy.deepcopy(cases[node_id]) for node_id in selected_ids]
    filtered_suite.extend(chosen)
    counts = Counter(_status(case) for case in chosen)
    filtered_suite.set("tests", str(len(chosen)))
    filtered_suite.set("failures", str(counts["failed"]))
    filtered_suite.set("errors", str(counts["error"]))
    filtered_suite.set("skipped", str(counts["skipped"]))
    seconds = [float(case.get("time", "0")) for case in chosen]
    _require(
        all(math.isfinite(value) and value >= 0 for value in seconds),
        "invalid JUnit testcase duration",
    )
    filtered_suite.set("time", f"{sum(seconds):.3f}")
    return ET.tostring(filtered_root, encoding="utf-8", xml_declaration=True)


def _prepare(spec: dict[str, Any]) -> tuple[dict[str, Any], dict[str, bytes]]:
    manifest_path = Path(spec["manifest"]).resolve()
    excluded = spec["excludedIds"]
    _require(
        isinstance(excluded, list)
        and len(excluded) == 4
        and set(excluded) == EXCLUDED
        and len(set(excluded)) == 4,
        "the four explicit exclusions must match the pinned oracle failures",
    )
    data: dict[str, Any] = {}
    manifest_sha = _file_sha(manifest_path)
    inventory: str | None = None
    for mode in ("candidate", "oracle"):
        paths = spec[mode]
        provenance_path = Path(paths["parentProvenance"]).resolve()
        raw = _json_file(provenance_path)
        payload = verify_provenance(raw, provenance_path=provenance_path)
        _require(
            payload.get("schema") == 5
            and payload.get("mode") == mode
            and payload.get("manifestCases") is True
            and payload.get("manifestApi") == "async"
            and payload.get("isolateCases") is True
            and payload.get("isolatedCaseCount") == 39
            and payload.get("browsers") == ["chromium"]
            and payload.get("manifestSha256") == manifest_sha
            and Path(str(payload.get("manifest"))).resolve() == manifest_path,
            f"{mode} is not the pinned 39-case schema-5 evidence",
        )
        _require(
            payload.get("launchMode") == "cdp-adapter"
            and payload.get("browserChannel") == "chrome",
            f"{mode} launch profile drift",
        )
        _require(
            str(Path(paths["parentReport"]).resolve())
            == str(Path(str(payload["result"])).resolve()),
            f"{mode} report path differs from sidecar",
        )
        if inventory is None:
            inventory = str(Path(str(payload["inventory"])).resolve())
        else:
            _require(
                inventory == str(Path(str(payload["inventory"])).resolve()),
                "candidate/oracle inventory mismatch",
            )
        data[mode] = {
            "paths": paths,
            "provenance": payload,
            "provenanceSha256": _file_sha(provenance_path),
            "parentSha256": _file_sha(Path(paths["parentReport"])),
        }
    candidate, oracle = data["candidate"]["provenance"], data["oracle"]["provenance"]
    for field in (
        "commit",
        "tag",
        "repository",
        "manifestSha256",
        "fixtureHost",
        "fixtureBind",
        "isolatedCaseIdsSha256",
        "caseTimeout",
        "inventorySha256",
    ):
        _require(
            candidate.get(field) == oracle.get(field),
            f"candidate/oracle {field} differs",
        )
    expected_ids = _manifest_ids(manifest_path, Path(str(candidate["upstream"])))
    _require(
        set(excluded).issubset(expected_ids), "excluded ID is absent from manifest"
    )
    selected_ids = [node_id for node_id in expected_ids if node_id not in EXCLUDED]
    _require(
        len(selected_ids) == 35 and spec.get("selectedIds") == selected_ids,
        "explicit selected 35 IDs differ from manifest minus exclusions",
    )
    reports: dict[str, bytes] = {}
    statuses: dict[str, dict[str, str]] = {}
    for mode in ("candidate", "oracle"):
        item = data[mode]
        payload = item["provenance"]
        report_path = Path(item["paths"]["parentReport"])
        root, suite, cases = _report(report_path, expected_ids)
        _isolated_reports(Path(str(payload["caseReports"])), expected_ids, cases)
        statuses[mode] = {node_id: _status(cases[node_id]) for node_id in expected_ids}
        reports[mode] = _filtered_xml(root, suite, selected_ids, cases)
    _require(
        all(statuses["oracle"][node_id] == "failed" for node_id in EXCLUDED),
        "excluded cases are not the four oracle failures",
    )
    _require(
        all(statuses["oracle"][node_id] == "passed" for node_id in selected_ids),
        "selected case has a non-passing oracle outcome",
    )
    candidate_counts = Counter(
        statuses["candidate"][node_id] for node_id in selected_ids
    )
    _require(
        candidate_counts == {"passed": 22, "failed": 13},
        f"candidate 35-case status profile drift: {dict(candidate_counts)}",
    )
    for mode in ("candidate", "oracle"):
        selected = {node_id: statuses[mode][node_id] for node_id in selected_ids}
        data[mode]["baseSha256"] = _ledger(
            Path(data[mode]["paths"]["baseLedger"]), selected, str(inventory)
        )
        _require(
            data[mode]["paths"].get("expectedBaseSha256") == PINNED_BASE_SHA256[mode],
            f"{mode} expected base SHA-256 is not the pinned tranche digest",
        )
        _require(
            data[mode]["baseSha256"] == PINNED_BASE_SHA256[mode],
            f"{mode} base SHA-256 drift from the pinned tranche digest",
        )
    receipt: dict[str, Any] = {
        "schema": 1,
        "kind": "playwright-python-browsercontext-35-of-39-source-bound-selection",
        "manifest": str(manifest_path),
        "manifestSha256": manifest_sha,
        "inventory": str(inventory),
        "inventorySha256": candidate["inventorySha256"],
        "selectedIds": selected_ids,
        "excludedIds": sorted(EXCLUDED),
        "originalStatuses": [
            {
                "id": node_id,
                "candidate": statuses["candidate"][node_id],
                "oracle": statuses["oracle"][node_id],
            }
            for node_id in expected_ids
        ],
    }
    for mode in ("candidate", "oracle"):
        paths = data[mode]["paths"]
        receipt[mode] = {
            "parentProvenance": str(Path(paths["parentProvenance"]).resolve()),
            "parentProvenanceSha256": data[mode]["provenanceSha256"],
            "parentReport": str(Path(paths["parentReport"]).resolve()),
            "parentReportSha256": data[mode]["parentSha256"],
            "baseLedger": str(Path(paths["baseLedger"]).resolve()),
            "expectedBaseSha256": PINNED_BASE_SHA256[mode],
            "baseLedgerSha256": data[mode]["baseSha256"],
            "filteredReport": str(Path(paths["filteredReport"]).absolute()),
            "filteredReportSha256": _sha(reports[mode]),
            "selectedStatuses": [
                {"id": node_id, "outcome": statuses[mode][node_id]}
                for node_id in selected_ids
            ],
        }
    return receipt, reports


def _spec_from_receipt(receipt: dict[str, Any]) -> dict[str, Any]:
    _require(
        receipt.get("schema") == 1
        and receipt.get("kind")
        == "playwright-python-browsercontext-35-of-39-source-bound-selection",
        "unsupported derivation receipt",
    )
    return {
        "manifest": receipt["manifest"],
        "selectedIds": receipt["selectedIds"],
        "excludedIds": receipt["excludedIds"],
        **{
            mode: {
                key: receipt[mode][key]
                for key in (
                    "parentProvenance",
                    "parentReport",
                    "baseLedger",
                    "expectedBaseSha256",
                    "filteredReport",
                )
            }
            for mode in ("candidate", "oracle")
        },
    }


def create(spec: dict[str, Any], receipt_path: Path) -> dict[str, Any]:
    receipt, reports = _prepare(spec)
    receipt_path = receipt_path.absolute()
    outputs = [
        Path(receipt[mode]["filteredReport"]) for mode in ("candidate", "oracle")
    ]
    outputs.append(receipt_path)
    _require(
        len({path.resolve() for path in outputs}) == 3,
        "derivation output paths collide",
    )
    _require(
        all(
            path.parent.is_dir() and not path.exists() and not path.is_symlink()
            for path in outputs
        ),
        "derivation destination exists or parent is missing",
    )
    receipt_bytes = (json.dumps(receipt, sort_keys=True, indent=2) + "\n").encode()
    for path, contents in (
        (outputs[0], reports["candidate"]),
        (outputs[1], reports["oracle"]),
        (outputs[2], receipt_bytes),
    ):
        with path.open("xb") as output:
            output.write(contents)
    return receipt


def verify(receipt_path: Path) -> dict[str, Any]:
    receipt = _json_file(receipt_path)
    expected, reports = _prepare(_spec_from_receipt(receipt))
    _require(
        receipt == expected, "derivation receipt differs from source-bound evidence"
    )
    for mode in ("candidate", "oracle"):
        report_path = Path(receipt[mode]["filteredReport"])
        _require(
            _file_sha(report_path) == receipt[mode]["filteredReportSha256"]
            and report_path.read_bytes() == reports[mode],
            f"{mode} filtered report differs from derivation",
        )
    return receipt


def _expected_selected_row(
    base_row: dict[str, Any], case: ET.Element, filtered_report: Path
) -> dict[str, Any]:
    expected = dict(base_row)
    for field in ("durationSeconds", "error", "resultFile"):
        expected.pop(field, None)
    expected["outcome"] = _status(case)
    expected["attempts"] = 1
    duration = float(case.get("time", "0"))
    _require(
        math.isfinite(duration) and duration >= 0, "invalid selected case duration"
    )
    expected["durationSeconds"] = duration
    expected["resultFile"] = str(filtered_report.resolve())
    for child_name in ("failure", "error", "skipped"):
        child = case.find(child_name)
        if child is not None:
            lines = (child.get("message") or child.text or "").splitlines()
            if lines and lines[0]:
                expected["error"] = lines[0]
            break
    return expected


def verify_ledgers(
    receipt_path: Path, candidate_ledger: Path, oracle_ledger: Path
) -> dict[str, Any]:
    """Verify strict ledger outputs without creating or mutating either ledger."""
    receipt = verify(receipt_path)
    inventory = str(receipt["inventory"])
    paths = {"candidate": candidate_ledger.resolve(), "oracle": oracle_ledger.resolve()}
    bases = {mode: Path(receipt[mode]["baseLedger"]).resolve() for mode in paths}
    _require(
        len(set(paths.values()) | set(bases.values())) == 4,
        "new strict ledger paths collide with each other or a base",
    )
    output: dict[str, Any] = {"verified": True, "receipt": str(receipt_path.resolve())}
    for mode in ("candidate", "oracle"):
        selected_ids = receipt["selectedIds"]
        selected = {
            item["id"]: item["outcome"] for item in receipt[mode]["selectedStatuses"]
        }
        _require(
            len(selected) == 35 and list(selected) == selected_ids,
            f"{mode} selected receipt statuses are not the exact 35 IDs",
        )
        ledger_path = paths[mode]
        digest = _ledger(ledger_path, selected, inventory)
        report_path = Path(receipt[mode]["filteredReport"])
        _, _, cases = _report(report_path, selected_ids)
        base_rows = [json.loads(line) for line in bases[mode].read_text().splitlines()]
        new_rows = [json.loads(line) for line in ledger_path.read_text().splitlines()]
        _require(
            len(base_rows) == len(new_rows) == 6526, f"{mode} ledger row count drift"
        )
        projected = Counter(row["outcome"] for row in base_rows)
        seen: set[str] = set()
        for index, (base_row, new_row) in enumerate(zip(base_rows, new_rows)):
            node_id = base_row["id"]
            if node_id in selected:
                _require(
                    _status(cases[node_id]) == selected[node_id],
                    f"{mode} receipt status differs from filtered report: {node_id}",
                )
                expected = _expected_selected_row(base_row, cases[node_id], report_path)
                _require(
                    new_row == expected,
                    f"{mode} selected row differs from exact merge at {index}: {node_id}",
                )
                projected[base_row["outcome"]] -= 1
                projected[selected[node_id]] += 1
                seen.add(node_id)
            else:
                _require(
                    new_row == base_row,
                    f"{mode} unrelated row differs from base at {index}: {node_id}",
                )
        _require(
            seen == set(selected_ids), f"{mode} merged ledger omits selected cases"
        )
        projected_counts = dict(
            sorted((key, value) for key, value in projected.items() if value)
        )
        summary = _json_file(ledger_path.with_suffix(".summary.json"))
        _require(
            summary.get("outcomes") == projected_counts,
            f"{mode} strict ledger counts differ from the projected 35-case merge",
        )
        output[mode] = {
            "ledger": str(ledger_path),
            "ledgerSha256": digest,
            "outcomes": projected_counts,
        }
    return output


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    create_parser = sub.add_parser("create")
    create_parser.add_argument("--manifest", type=Path, required=True)
    create_parser.add_argument("--candidate-provenance", type=Path, required=True)
    create_parser.add_argument("--oracle-provenance", type=Path, required=True)
    create_parser.add_argument("--candidate-base", type=Path, required=True)
    create_parser.add_argument("--oracle-base", type=Path, required=True)
    create_parser.add_argument("--candidate-base-sha256", required=True)
    create_parser.add_argument("--oracle-base-sha256", required=True)
    create_parser.add_argument("--candidate-output", type=Path, required=True)
    create_parser.add_argument("--oracle-output", type=Path, required=True)
    create_parser.add_argument("--exclude", action="append", required=True)
    create_parser.add_argument("--receipt", type=Path, required=True)
    verify_parser = sub.add_parser("verify")
    verify_parser.add_argument("--receipt", type=Path, required=True)
    ledger_parser = sub.add_parser("verify-ledgers")
    ledger_parser.add_argument("--receipt", type=Path, required=True)
    ledger_parser.add_argument("--candidate-ledger", type=Path, required=True)
    ledger_parser.add_argument("--oracle-ledger", type=Path, required=True)
    args = parser.parse_args()
    try:
        if args.command == "create":
            manifest = _json_file(args.manifest)
            raw_ids = [entry["id"] + "[chromium]" for entry in manifest["cases"]]
            selected = [
                node_id for node_id in raw_ids if node_id not in set(args.exclude)
            ]

            def parent_report(sidecar: Path) -> str:
                name = sidecar.name
                _require(
                    name.endswith(".provenance.json"),
                    f"not an official provenance sibling: {sidecar}",
                )
                return str(
                    sidecar.with_name(
                        name.removesuffix(".provenance.json") + ".xml"
                    ).resolve()
                )

            spec = {
                "manifest": str(args.manifest.resolve()),
                "selectedIds": selected,
                "excludedIds": args.exclude,
                "candidate": {
                    "parentProvenance": str(args.candidate_provenance.resolve()),
                    "parentReport": parent_report(args.candidate_provenance),
                    "baseLedger": str(args.candidate_base.resolve()),
                    "expectedBaseSha256": args.candidate_base_sha256,
                    "filteredReport": str(args.candidate_output.absolute()),
                },
                "oracle": {
                    "parentProvenance": str(args.oracle_provenance.resolve()),
                    "parentReport": parent_report(args.oracle_provenance),
                    "baseLedger": str(args.oracle_base.resolve()),
                    "expectedBaseSha256": args.oracle_base_sha256,
                    "filteredReport": str(args.oracle_output.absolute()),
                },
            }
            receipt = create(spec, args.receipt)
        elif args.command == "verify":
            receipt = verify(args.receipt)
        else:
            print(
                json.dumps(
                    verify_ledgers(
                        args.receipt, args.candidate_ledger, args.oracle_ledger
                    ),
                    indent=2,
                )
            )
            return
    except (
        SelectionError,
        OSError,
        ValueError,
        KeyError,
        TypeError,
        ET.ParseError,
    ) as error:
        parser.exit(2, f"source-bound selection rejected: {error}\n")
    print(
        json.dumps(
            {
                "verified": True,
                "receipt": str(args.receipt.resolve()),
                "selected": len(receipt["selectedIds"]),
                "candidateReport": receipt["candidate"]["filteredReport"],
                "oracleReport": receipt["oracle"]["filteredReport"],
                "candidateBase": receipt["candidate"]["baseLedger"],
                "oracleBase": receipt["oracle"]["baseLedger"],
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
