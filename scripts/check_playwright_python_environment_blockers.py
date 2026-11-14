#!/usr/bin/env python3
"""Check four historical fixture-host failures without changing conformance outcomes.

This is an evidence annotation, not an alternative test runner or a source of
pass credit. It never starts a fixture server, browser, or CDP connection.
"""

from __future__ import annotations

import argparse
import hashlib
import ipaddress
import json
import subprocess
import xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path
from typing import cast

EXPECTED_CASES = {
    (
        "tests/async/test_browsercontext_add_cookies.py::"
        "test_should_set_cookie_with_reasonable_defaults[chromium]"
    ): (
        "add-cookies",
        "cookie-default-domain",
    ),
    (
        "tests/async/test_browsercontext_add_cookies.py::"
        "test_should_set_a_cookie_with_a_path[chromium]"
    ): (
        "add-cookies",
        "cookie-path-domain",
    ),
    (
        "tests/async/test_browsercontext_storage_state.py::"
        "test_should_serialise_indexed_db[chromium]"
    ): (
        "storage-state",
        "indexeddb-origin",
    ),
    "tests/sync/test_browsercontext_storage_state.py::test_should_serialise_indexed_db[chromium]": (
        "storage-state",
        "indexeddb-origin",
    ),
}


class EvidenceError(ValueError):
    """The frozen evidence no longer supports the environment-blocker claim."""


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise EvidenceError(message)


def _object(value: object, label: str) -> dict[str, object]:
    _require(isinstance(value, dict), f"{label} must be an object")
    result = cast(dict[object, object], value)
    _require(all(isinstance(key, str) for key in result), f"{label} has non-string keys")
    return cast(dict[str, object], result)


def _string(value: object, label: str) -> str:
    _require(isinstance(value, str) and bool(value), f"{label} must be a string")
    return cast(str, value)


def _read_json(path: Path) -> dict[str, object]:
    return _object(json.loads(path.read_text()), str(path))


def _relative_file(root: Path, value: object, label: str) -> Path:
    relative = Path(_string(value, label))
    _require(
        not relative.is_absolute() and ".." not in relative.parts,
        f"{label} escapes root",
    )
    target = root / relative
    _require(target.is_file() and not target.is_symlink(), f"{label} is missing or a symlink")
    _require(target.resolve().is_relative_to(root.resolve()), f"{label} escapes root")
    return target


def _digest(path: Path) -> str:
    sha = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            sha.update(chunk)
    return sha.hexdigest()


def _pinned_file(root: Path, record: dict[str, object], label: str) -> Path:
    path = _relative_file(root, record.get("path"), label)
    _require(_digest(path) == record.get("sha256"), f"{label} SHA-256 changed")
    return path


def _ledger_rows(
    root: Path, record: dict[str, object], expected_count: int, lane: str
) -> tuple[Path, dict[str, dict[str, object]]]:
    path = _pinned_file(root, record, f"{lane} ledger")
    summary = _read_json(path.with_suffix(".summary.json"))
    rows = [
        _object(json.loads(line), f"{lane} ledger row") for line in path.read_text().splitlines()
    ]
    _require(len(rows) == expected_count, f"{lane} ledger row count changed")
    _require(summary.get("records") == expected_count, f"{lane} summary count changed")
    _require(summary.get("ledgerSha256") == _digest(path), f"{lane} summary digest changed")
    outcomes = [_string(row.get("outcome"), f"{lane} outcome") for row in rows]
    _require(
        set(outcomes).issubset({"passed", "failed", "skipped", "not-run"}),
        f"{lane} ledger has an unsupported outcome",
    )
    actual_counts = dict(sorted(Counter(outcomes).items()))
    _require(summary.get("outcomes") == actual_counts, f"{lane} summary outcomes changed")
    by_id: dict[str, dict[str, object]] = {}
    for row in rows:
        node_id = _string(row.get("id"), f"{lane} ledger ID")
        _require(node_id not in by_id, f"{lane} ledger repeats {node_id}")
        by_id[node_id] = row
    return path, by_id


def verify_shared_raw_failures(
    candidate: dict[str, dict[str, object]],
    oracle: dict[str, dict[str, object]],
    expected_ids: set[str],
) -> None:
    """A shared infrastructure failure must remain failed and public in both lanes."""
    for lane, rows in (("candidate", candidate), ("oracle", oracle)):
        failed = {node_id for node_id, row in rows.items() if row.get("outcome") == "failed"}
        _require(failed == expected_ids, f"{lane} failed IDs differ from exact blocker set")
        for node_id in expected_ids:
            row = rows[node_id]
            _require(
                row.get("sourceClassification") == "public-api-source",
                f"{lane} {node_id} lost public classification",
            )
            _require(row.get("privateImports") == [], f"{lane} {node_id} has private imports")
            _require(row.get("attempts") == 1, f"{lane} {node_id} was not executed once")
            _require(
                _string(row.get("error"), f"{lane} {node_id} error").startswith("AssertionError"),
                f"{lane} {node_id} is not an assertion failure",
            )


def failure_matches_signature(text: str, signature: str, host: str) -> bool:
    """Match the observed URL/domain conflict, not merely any shared failure."""
    if signature == "cookie-default-domain":
        return f"'domain': '{host}'" in text and "'domain': 'localhost'" in text
    if signature == "cookie-path-domain":
        return f"Page url='http://{host}:" in text and "assert '' == 'gridcookie=GRID'" in text
    if signature == "indexeddb-origin":
        return f"'origin': 'http://{host}:" in text and "'origin': 'http://localhost:" in text
    raise EvidenceError(f"unknown failure signature: {signature}")


def _case_id(testcase: ET.Element) -> str:
    module = _string(testcase.get("classname"), "JUnit classname")
    name = _string(testcase.get("name"), "JUnit name")
    return f"{module.replace('.', '/')}.py::{name}"


def _verify_report(
    root: Path,
    record: dict[str, object],
    *,
    manifest_path: Path,
    manifest_sha: str,
    commit: str,
    fixture_host: str,
    fixture_bind: str,
    lane: str,
    expected_count: int,
    expected_cases: dict[str, str],
    ledger: dict[str, dict[str, object]],
) -> None:
    report_path = _pinned_file(root, record, f"{lane} JUnit")
    provenance_path = _relative_file(root, record.get("provenance"), f"{lane} provenance")
    _require(
        _digest(provenance_path) == record.get("provenanceSha256"),
        f"{lane} provenance SHA-256 changed",
    )
    provenance = _read_json(provenance_path)
    expected_metadata: dict[str, object] = {
        "schema": 1,
        "mode": lane,
        "commit": commit,
        "fixtureHost": fixture_host,
        "fixtureBind": fixture_bind,
        "manifestSha256": manifest_sha,
        "resultSha256": _digest(report_path),
        "scope": "manifest-cases-all",
        "browsers": ["chromium"],
        "collectOnly": False,
        "exitCode": 1,
    }
    for key, expected in expected_metadata.items():
        _require(provenance.get(key) == expected, f"{lane} provenance {key} changed")
    _require(
        Path(_string(provenance.get("manifest"), "provenance manifest")).resolve()
        == manifest_path.resolve(),
        f"{lane} provenance manifest path changed",
    )
    _require(
        Path(_string(provenance.get("result"), "provenance result")).resolve()
        == report_path.resolve(),
        f"{lane} provenance result path changed",
    )

    suite = ET.parse(report_path).getroot().find("testsuite")
    if suite is None:
        raise EvidenceError(f"{lane} JUnit lacks a testsuite")
    _require(suite.get("tests") == str(expected_count), f"{lane} JUnit case count changed")
    _require(
        suite.get("failures") == str(len(expected_cases)),
        f"{lane} JUnit failure count changed",
    )
    _require(
        suite.get("errors") == "0" and suite.get("skipped") == "0",
        f"{lane} JUnit setup or skip changed",
    )
    testcases = suite.findall("testcase")
    _require(len(testcases) == expected_count, f"{lane} JUnit testcase count changed")
    failures: dict[str, ET.Element] = {}
    for testcase in testcases:
        failure = testcase.find("failure")
        if failure is not None:
            node_id = _case_id(testcase)
            _require(node_id not in failures, f"{lane} JUnit repeats {node_id}")
            failures[node_id] = failure
    _require(set(failures) == set(expected_cases), f"{lane} JUnit failure IDs changed")
    for node_id, signature in expected_cases.items():
        failure = failures[node_id]
        detail = (failure.text or "") + "\n" + (failure.get("message") or "")
        _require(
            failure_matches_signature(detail, signature, fixture_host),
            f"{lane} {node_id} lacks fixture-host signature",
        )
        _require(
            Path(
                _string(ledger[node_id].get("resultFile"), f"{lane} {node_id} resultFile")
            ).resolve()
            == report_path.resolve(),
            f"{lane} {node_id} points to another JUnit",
        )


def verify_environment_blockers(
    root: Path, upstream: Path, registry_path: Path
) -> dict[str, object]:
    registry = _read_json(registry_path)
    _require(registry.get("schema") == 1, "unsupported blocker registry schema")
    _require(
        registry.get("repository") == "https://github.com/microsoft/playwright-python.git",
        "unexpected upstream repository",
    )
    commit = _string(registry.get("commit"), "upstream commit")
    actual_commit = subprocess.check_output(
        ["git", "-C", str(upstream), "rev-parse", "HEAD"], text=True
    ).strip()
    _require(actual_commit == commit, "upstream checkout is not the pinned commit")
    host = _string(registry.get("fixtureHost"), "fixtureHost")
    _require(
        host.endswith(".ts.net") and "-yuacx." in host,
        "fixtureHost is not full Tailscale MagicDNS",
    )
    fixture_bind = _string(registry.get("fixtureBind"), "fixtureBind")
    bind_address = ipaddress.ip_address(fixture_bind)
    _require(
        isinstance(bind_address, ipaddress.IPv4Address)
        and bind_address in ipaddress.IPv4Network("100.64.0.0/10"),
        "fixtureBind is not a Tailscale IPv4 address",
    )
    expected_count = registry.get("ledgerRows")
    if expected_count != 6526:
        raise EvidenceError("full Python corpus row count changed")
    expected_count = cast(int, expected_count)

    raw_case_values = registry.get("cases")
    if not isinstance(raw_case_values, list):
        raise EvidenceError("registry must contain exactly four cases")
    case_values = cast(list[object], raw_case_values)
    if len(case_values) != 4:
        raise EvidenceError("registry must contain exactly four cases")
    cases = [_object(value, "blocker case") for value in case_values]
    keyed_cases = {cast(str, case.get("id")): case for case in cases}
    _require(set(keyed_cases) == set(EXPECTED_CASES), "blocker IDs changed")
    for node_id, case in keyed_cases.items():
        _require(
            (case.get("run"), case.get("signature")) == EXPECTED_CASES[node_id],
            f"{node_id} blocker definition changed",
        )

    ledger_records = _object(registry.get("ledgers"), "ledgers")
    _require(set(ledger_records) == {"candidate", "oracle"}, "ledger lanes changed")
    ledgers: dict[str, dict[str, dict[str, object]]] = {}
    for lane in ("candidate", "oracle"):
        _, ledgers[lane] = _ledger_rows(
            root, _object(ledger_records[lane], f"{lane} ledger"), expected_count, lane
        )
    verify_shared_raw_failures(ledgers["candidate"], ledgers["oracle"], set(EXPECTED_CASES))

    runs = _object(registry.get("runs"), "runs")
    _require(set(runs) == {"add-cookies", "storage-state"}, "registered runs changed")
    for run_name, raw_run in runs.items():
        run = _object(raw_run, run_name)
        manifest_path = _relative_file(root, run.get("manifest"), f"{run_name} manifest")
        manifest_sha = _string(run.get("manifestSha256"), f"{run_name} manifest SHA-256")
        _require(_digest(manifest_path) == manifest_sha, f"{run_name} manifest changed")
        manifest = _read_json(manifest_path)
        _require(manifest.get("commit") == commit, f"{run_name} manifest commit changed")
        source_files = _object(run.get("sourceFiles"), f"{run_name} sourceFiles")
        _require(
            manifest.get("files") == source_files,
            f"{run_name} source pin differs from manifest",
        )
        for source, expected_sha in source_files.items():
            _require(
                _digest(_relative_file(upstream, source, source)) == expected_sha,
                f"{source} changed",
            )
        raw_manifest_cases = manifest.get("cases")
        if not isinstance(raw_manifest_cases, list):
            raise EvidenceError(f"{run_name} manifest has no cases")
        manifest_cases = cast(list[object], raw_manifest_cases)
        report_count = run.get("reportCases")
        _require(
            isinstance(report_count, int) and report_count == len(manifest_cases),
            f"{run_name} manifest case count changed",
        )
        report_count = cast(int, report_count)
        manifest_ids = {
            _object(case, f"{run_name} manifest case").get("id") for case in manifest_cases
        }
        selected = {
            node_id: cast(str, case["signature"])
            for node_id, case in keyed_cases.items()
            if case["run"] == run_name
        }
        _require(
            {node_id.split("[", 1)[0] for node_id in selected}.issubset(manifest_ids),
            f"{run_name} manifest lacks selected cases",
        )
        report_records = _object(run.get("reports"), f"{run_name} reports")
        _require(
            set(report_records) == {"candidate", "oracle"},
            f"{run_name} report lanes changed",
        )
        for lane in ("candidate", "oracle"):
            _verify_report(
                root,
                _object(report_records[lane], f"{run_name} {lane} report"),
                manifest_path=manifest_path,
                manifest_sha=manifest_sha,
                commit=commit,
                fixture_host=host,
                fixture_bind=fixture_bind,
                lane=lane,
                expected_count=report_count,
                expected_cases=selected,
                ledger=ledgers[lane],
            )

    return {
        "schema": 1,
        "publicFailures": len(EXPECTED_CASES),
        "sharedEnvironmentBlockers": sorted(EXPECTED_CASES),
        "conformancePassCredit": 0,
        "rawOutcome": "failed-in-both-lanes",
        "registrySha256": _digest(registry_path),
    }


def main() -> None:
    root = Path(__file__).parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=root)
    parser.add_argument(
        "--upstream",
        type=Path,
        default=root.parents[2] / "thirdparty/playwright-python",
    )
    parser.add_argument(
        "--registry",
        type=Path,
        default=root
        / "packages/pwright-python/tests/official/python-fixture-environment-blockers.json",
    )
    args = parser.parse_args()
    try:
        result = verify_environment_blockers(args.root, args.upstream, args.registry)
    except (OSError, ValueError, subprocess.CalledProcessError, ET.ParseError) as error:
        parser.error(str(error))
    print(json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    main()
