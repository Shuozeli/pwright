"""Independently verify the immutable 80-case Chromium navigation promotion.

The verifier rechecks the complete predecessor receipt chain, frozen official
reports and source pins, then reconstructs every promoted ledger row. It does
not run browser cases or publish another bundle.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import subprocess
import xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path
from typing import Any, NoReturn

import promote_playwright_python_upstream_skip_callable_fixtures_strict_v2 as prior
import verify_playwright_python_official_provenance as official

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / ".work/official-conformance"
BUILD = WORK / "builds/python-navigation-full-abi127-oct20.bB5H3w"
RESULTS = BUILD / "results"
BASE_DIR = WORK / "outcomes/python-upstream-skip-callable-fixtures-strict-paired-v2"
OUTPUT_DIR = WORK / "outcomes/python-navigation-full-chromium-strict-paired-v1"
INVENTORY = WORK / "playwright-python-v1.58.0-profiled.jsonl"
MANIFEST = ROOT / "packages/pwright-python/tests/official/manifest.json"
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-python")
NAVIGATION_SOURCE = UPSTREAM / "tests/async/test_navigation.py"
SOURCE_PINS = BUILD / "source-pins.json"
FROZEN_WHEEL = BUILD / "frozen/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl"
PROMOTER = (
    ROOT / "scripts/promote_playwright_python_navigation_full_strict_paired_v1.py"
)
BASE_PROMOTER = (
    ROOT
    / "scripts/promote_playwright_python_upstream_skip_callable_fixtures_strict_v2.py"
)

MODES = ("candidate", "oracle")
RECORDS = 6_526
TARGET_INDEXES = tuple(range(2680, 2918, 3))
GENERATED = frozenset({"attempts", "durationSeconds", "error", "outcome", "resultFile"})
KIND = "playwright-python-navigation-full-chromium-strict-paired-promotion-v1"
SCOPE = "tests/async/test_navigation.py-chromium-full-file-80"
COMMIT = "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
INVENTORY_SHA256 = "3dfb11b6ad7eb4736166ee2c4f7fa32cda78063324ad8314ce12f3e2d49b9d41"
MANIFEST_SHA256 = "6d4f86fee517e300319cfa87014a54302980bdf8cf538fa21c1daff4219ea724"
NAVIGATION_SHA256 = "f04af8cbdee3dffb2c578d61d7685f02e5186ca565906323d71b6f92f96cd144"
SOURCE_PINS_SHA256 = "cd5f3360d8b25a5546162600ed71bb63bcc73c997e1bbb4f71e0a91bf2a063a3"
FROZEN_WHEEL_SHA256 = "05142f1065ec5e3de608da0ebad697d8888131f480099b7bb12a3472b2b5eeb4"
BASE_RECEIPT_SHA256 = "1b8ce858fa3ab643fb10e8f12142c12b32e3164b8719e2c627d2f6e4a66dbb4a"
BASE_PROMOTER_SHA256 = (
    "891543d38877631763806b68a4449d0e7689e7269f3473ebfe27d2ecd7401bd1"
)
BASE_HASHES = {
    "candidate": (
        "bc150cd75730d0c0a0fa8fab6fc0743d7fa81209304474e917dc70e13194a81d",
        "a36899c92dc5aa9b1ad9b19b26a61155bc5fa81b17f700c032e2ce8247093cc9",
    ),
    "oracle": (
        "3b4c8e43d1a0398e1e2ec6bfdde636a5ce3be0e5d7c3fe8e110016a721bb6934",
        "8057b5670e705c208f28b71d318c6452fe1cd9d00e2e3dc9022dc6c0772be27e",
    ),
}
REPORT_HASHES = {
    "candidate": (
        "b54a1408045342de9249a80b8dc0e77c51bb5261048854f67387f610f15c637e",
        "0a6bb9528dbf49ec7f761e37e8aaaf4813cc2c82385f9c8dc3e7081267be8591",
        "f40f4cfaa98b555b4645e04a90e85f294577bc7163c1bcbab821dd7ff1833c04",
        {"passed": 57, "failed": 23},
    ),
    "oracle": (
        "9d2cb134fcd79ab56142b8f8cb63d5b5d88aac43c7564877039fdf706fd308da",
        "3d0bc87ff1615ff4d6c2488e5e07107df1d5d6d4da3945312f175edcbd3114e0",
        "68c219ce12e9875a2a5267980b25864ad242d809096e5dc55ddc134c27e717cd",
        {"passed": 77, "failed": 3},
    ),
}
ISOLATED_IDS_SHA256 = "4b9f37f3a3857ddf59e9b5766255f84dfe83d3d5af76285208e176088f656696"


class NavigationPromotionValidationError(ValueError):
    """Raised when an immutable promotion or its evidence differs."""


def fail(message: str) -> NoReturn:
    raise NavigationPromotionValidationError(message)


def require(condition: bool, message: str) -> None:
    if not condition:
        fail(message)


def sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def sealed(path: Path, expected_sha: str, label: str) -> bytes:
    require(
        len(expected_sha) == 64
        and all(character in "0123456789abcdef" for character in expected_sha),
        f"{label} has an invalid SHA-256 pin",
    )
    require(
        path.is_file() and not path.is_symlink(), f"{label} missing or linked: {path}"
    )
    raw = path.read_bytes()
    require(sha256(raw) == expected_sha, f"{label} SHA-256 differs")
    return raw


def json_object(raw: bytes, label: str) -> dict[str, Any]:
    try:
        value = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise NavigationPromotionValidationError(f"{label} is invalid JSON") from error
    require(isinstance(value, dict), f"{label} is not an object")
    return value


def jsonl(
    raw: bytes, label: str, expected_rows: int = RECORDS
) -> tuple[list[bytes], list[dict[str, Any]]]:
    require(raw.endswith(b"\n"), f"{label} is not newline terminated")
    lines = raw.splitlines(keepends=True)
    require(len(lines) == expected_rows, f"{label} row count differs")
    return lines, [
        json_object(line, f"{label} row {index}") for index, line in enumerate(lines)
    ]


def base_name(mode: str) -> str:
    return f"playwright-python-{mode}-upstream-skip-callable-fixtures-strict-paired-v2.jsonl"


def output_name(mode: str) -> str:
    return f"playwright-python-{mode}-navigation-full-chromium-strict-paired-v1.jsonl"


def report_paths(mode: str) -> tuple[Path, Path]:
    result = RESULTS / f"navigation-{mode}-full-v1.xml"
    return result.with_suffix(".provenance.json"), result


def check_receipt_scope(receipt: dict[str, object]) -> None:
    require(
        type(receipt.get("schema")) is int and receipt["schema"] == 2,
        "receipt schema differs",
    )
    require(
        receipt.get("kind") == KIND and receipt.get("scope") == SCOPE,
        "receipt kind or scope differs",
    )
    for field, expected in (
        ("publicApiParityEvidence", False),
        ("fullCorpusExecution", False),
        ("completeFileExecution", True),
        ("fixtureAdapted", True),
    ):
        require(receipt.get(field) is expected, f"receipt {field} differs")
    require(receipt.get("launchMode") == "cdp-adapter", "receipt launch mode differs")
    require(
        receipt.get("chromeVersionObserved") == "155.0.8059.85",
        "receipt Chrome version differs",
    )


def _metadata(row: dict[str, Any]) -> dict[str, Any]:
    return {key: value for key, value in row.items() if key not in GENERATED}


def check_reconciliation(
    base_lines: list[bytes],
    successor_lines: list[bytes],
    inventory_rows: list[dict[str, object]],
    target_indexes: list[int],
    expected_cases: dict[str, dict[str, object]],
    result_file: str,
    mode: str,
) -> Counter[str]:
    """Rebuild target rows and demand byte-identical off-target rows.

    This helper accepts a small synthetic inventory so mutation tests do not
    have to rehash the full upstream wheel and receipt chain.
    """
    require(mode in MODES, "ledger mode differs")
    require(
        len(base_lines) == len(successor_lines) == len(inventory_rows),
        f"{mode} ledger length differs",
    )
    require(
        len(target_indexes) == len(set(target_indexes))
        and all(0 <= index < len(inventory_rows) for index in target_indexes),
        "target indexes are missing, duplicated or out of bounds",
    )
    ids = [row.get("id") for row in inventory_rows]
    require(
        all(isinstance(test_id, str) for test_id in ids), "inventory ID is malformed"
    )
    require(len(set(ids)) == len(ids), "inventory has duplicate IDs")
    target_ids = [ids[index] for index in target_indexes]
    require(set(expected_cases) == set(target_ids), "report target ID set differs")
    target_set = set(target_indexes)
    counts: Counter[str] = Counter()
    new_ids: list[str] = []
    for index, (before_raw, after_raw, source) in enumerate(
        zip(base_lines, successor_lines, inventory_rows)
    ):
        before = json_object(before_raw, f"{mode} base row {index}")
        after = json_object(after_raw, f"{mode} successor row {index}")
        require(before.get("id") == ids[index], f"{mode} base ID moved at {index}")
        require(
            _metadata(before) == source,
            f"{mode} base source metadata differs at {index}",
        )
        require(after.get("id") == ids[index], f"{mode} successor ID moved at {index}")
        require(
            _metadata(after) == source,
            f"{mode} successor source metadata differs at {index}",
        )
        new_ids.append(after["id"])
        if index not in target_set:
            require(
                after_raw == before_raw, f"{mode} off-target raw row changed at {index}"
            )
        else:
            require(
                source.get("sourceClassification") == "public-api-source",
                "target source classification differs",
            )
            require(
                before.get("outcome") in {"passed", "not-run"},
                f"{mode} prior target status differs",
            )
            case = expected_cases[ids[index]]
            outcome = case.get("outcome")
            duration = case.get("durationSeconds")
            require(
                outcome in {"passed", "failed"},
                f"{mode} report status differs at {index}",
            )
            require(
                type(duration) is float and math.isfinite(duration) and duration >= 0,
                f"{mode} report duration differs at {index}",
            )
            expected: dict[str, object] = {
                **source,
                "attempts": 1,
                "durationSeconds": duration,
                "outcome": outcome,
                "resultFile": result_file,
            }
            if outcome == "failed":
                error = case.get("error")
                require(
                    isinstance(error, str) and bool(error),
                    f"{mode} failure message missing at {index}",
                )
                expected["error"] = error
            require(after == expected, f"{mode} promoted row differs at {index}")
            canonical = (
                json.dumps(expected, sort_keys=True, separators=(",", ":")) + "\n"
            ).encode()
            require(
                after_raw == canonical, f"{mode} promoted row bytes differ at {index}"
            )
        status = after.get("outcome")
        require(isinstance(status, str), f"{mode} row outcome malformed at {index}")
        counts[status] += 1
    require(len(set(new_ids)) == len(new_ids), f"{mode} successor has duplicate IDs")
    return counts


def _verify_base() -> tuple[
    dict[str, Any], list[dict[str, Any]], dict[str, list[bytes]]
]:
    sealed(BASE_PROMOTER, BASE_PROMOTER_SHA256, "base promoter source")
    prior_receipt = json_object(
        sealed(
            BASE_DIR / "promotion-receipt.json", BASE_RECEIPT_SHA256, "base receipt"
        ),
        "base receipt",
    )
    require(
        prior.verify_bundle(BASE_DIR) == prior_receipt,
        "base chain verification differs",
    )
    require(
        prior_receipt.get("kind") == prior.RECEIPT_KIND
        and prior_receipt.get("commit") == COMMIT
        and prior_receipt.get("promoterSha256") == BASE_PROMOTER_SHA256
        and prior_receipt.get("recordsPerLane") == RECORDS
        and prior_receipt.get("inventory") == str(INVENTORY)
        and prior_receipt.get("inventorySha256") == INVENTORY_SHA256,
        "base receipt identity differs",
    )
    _, inventory = jsonl(
        sealed(INVENTORY, INVENTORY_SHA256, "profiled inventory"), "inventory"
    )
    ids = [row.get("id") for row in inventory]
    require(len(set(ids)) == RECORDS, "profiled inventory has duplicate IDs")
    expected_indexes = tuple(
        index
        for index, row in enumerate(inventory)
        if row.get("file") == "tests/async/test_navigation.py"
        and isinstance(row.get("id"), str)
        and row["id"].endswith("[chromium]")
    )
    require(expected_indexes == TARGET_INDEXES, "80 navigation indexes moved")
    require(
        all(
            inventory[index].get("sourceClassification") == "public-api-source"
            for index in TARGET_INDEXES
        ),
        "navigation source classification differs",
    )
    ledgers: dict[str, list[bytes]] = {}
    for mode in MODES:
        ledger = BASE_DIR / base_name(mode)
        ledger_sha, summary_sha = BASE_HASHES[mode]
        lines, rows = jsonl(
            sealed(ledger, ledger_sha, f"{mode} base ledger"), f"{mode} base ledger"
        )
        summary = json_object(
            sealed(
                ledger.with_suffix(".summary.json"), summary_sha, f"{mode} base summary"
            ),
            f"{mode} base summary",
        )
        require(
            prior_receipt.get("successors", {}).get(mode) == summary,
            f"{mode} base receipt summary differs",
        )
        require(
            summary.get("ledgerSha256") == ledger_sha
            and summary.get("records") == RECORDS,
            f"{mode} base summary differs",
        )
        statuses = Counter(rows[index].get("outcome") for index in TARGET_INDEXES)
        expected = (
            {"passed": 2, "not-run": 78} if mode == "candidate" else {"not-run": 80}
        )
        require(statuses == expected, f"{mode} prior navigation statuses differ")
        if mode == "candidate":
            require(
                [
                    index
                    for index in TARGET_INDEXES
                    if rows[index].get("outcome") == "passed"
                ]
                == [2710, 2713],
                "candidate's two prior navigation passes moved",
            )
        ledgers[mode] = lines
    return prior_receipt, inventory, ledgers


def _verify_source_pins() -> dict[str, Any]:
    pins = json_object(
        sealed(SOURCE_PINS, SOURCE_PINS_SHA256, "navigation source pins"),
        "navigation source pins",
    )
    require(
        pins.get("kind") == "playwright-python-navigation-full-file-candidate"
        and pins.get("upstreamCommit") == COMMIT
        and pins.get("upstreamNavigationSha256") == NAVIGATION_SHA256
        and pins.get("officialManifestSha256") == MANIFEST_SHA256
        and pins.get("profiledInventorySha256") == INVENTORY_SHA256
        and pins.get("chromiumExpandedCases") == 80
        and pins.get("installedNativeAbi") == 127
        and pins.get("installedSelectedFeatures") == 109
        and pins.get("wheelName") == FROZEN_WHEEL.name
        and pins.get("frozenWheelSha256") == FROZEN_WHEEL_SHA256,
        "navigation source-pin identity differs",
    )
    sealed(MANIFEST, MANIFEST_SHA256, "official manifest")
    manifest = json_object(MANIFEST.read_bytes(), "official manifest")
    require(
        manifest.get("commit") == COMMIT
        and manifest.get("repository")
        == "https://github.com/microsoft/playwright-python.git"
        and manifest.get("tag") == "v1.58.0"
        and manifest.get("files", {}).get("tests/async/test_navigation.py")
        == NAVIGATION_SHA256,
        "official manifest or source-file pin differs",
    )
    sealed(NAVIGATION_SOURCE, NAVIGATION_SHA256, "upstream navigation source")
    sealed(FROZEN_WHEEL, FROZEN_WHEEL_SHA256, "frozen ABI127 wheel")
    return pins


def _junit_cases(mode: str, result_raw: bytes) -> dict[str, dict[str, object]]:
    try:
        tree = ET.fromstring(result_raw)
    except ET.ParseError as error:
        raise NavigationPromotionValidationError(
            f"{mode} result is invalid XML"
        ) from error
    cases = list(tree.iter("testcase"))
    require(len(cases) == 80, f"{mode} XML does not have exactly 80 cases")
    observed: dict[str, dict[str, object]] = {}
    for case in cases:
        require(
            case.get("classname") == "tests.async.test_navigation",
            f"{mode} XML case is outside the pinned navigation file",
        )
        name = case.get("name")
        require(
            isinstance(name, str) and name.endswith("[chromium]"),
            f"{mode} XML case name differs",
        )
        test_id = f"tests/async/test_navigation.py::{name}"
        require(test_id not in observed, f"{mode} XML has duplicate ID {test_id}")
        try:
            duration = float(case.get("time", ""))
        except ValueError as error:
            raise NavigationPromotionValidationError(
                f"{mode} XML duration is invalid"
            ) from error
        require(
            math.isfinite(duration) and duration >= 0, f"{mode} XML duration is invalid"
        )
        failures = case.findall("failure")
        require(
            len(failures) <= 1
            and not case.findall("error")
            and not case.findall("skipped"),
            f"{mode} XML status is ambiguous",
        )
        row: dict[str, object] = {
            "outcome": "failed" if failures else "passed",
            "durationSeconds": duration,
        }
        if failures:
            message = failures[0].get("message") or failures[0].text or ""
            first_line = message.splitlines()
            require(
                bool(first_line and first_line[0]), f"{mode} failure message missing"
            )
            row["error"] = first_line[0]
        observed[test_id] = row
    require(
        Counter(case["outcome"] for case in observed.values())
        == REPORT_HASHES[mode][3],
        f"{mode} XML pass/fail count differs from pinned result",
    )
    return observed


def _verify_report(
    mode: str, target_ids: list[str]
) -> tuple[dict[str, Any], dict[str, dict[str, object]]]:
    sidecar, result = report_paths(mode)
    sidecar_sha, result_sha, case_reports_sha, _ = REPORT_HASHES[mode]
    payload = json_object(
        sealed(sidecar, sidecar_sha, f"{mode} sidecar"), f"{mode} sidecar"
    )
    sealed(result, result_sha, f"{mode} JUnit")
    try:
        official.verify_provenance(payload, provenance_path=sidecar)
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        raise NavigationPromotionValidationError(
            f"{mode} official provenance failed: {error}"
        ) from error
    require(
        payload.get("schema") == 5
        and payload.get("mode") == mode
        and payload.get("commit") == COMMIT
        and payload.get("manifest") == str(MANIFEST)
        and payload.get("manifestSha256") == MANIFEST_SHA256
        and payload.get("upstream") == str(UPSTREAM)
        and payload.get("inventory") == str(INVENTORY)
        and payload.get("inventorySha256") == INVENTORY_SHA256
        and payload.get("result") == str(result)
        and payload.get("resultSha256") == result_sha
        and payload.get("caseReportsSha256") == case_reports_sha
        and payload.get("isolatedCaseIdsSha256") == ISOLATED_IDS_SHA256
        and payload.get("isolatedCaseCount") == 80
        and payload.get("selections") == ["tests/async/test_navigation.py"]
        and payload.get("browsers") == ["chromium"]
        and payload.get("launchMode") == "cdp-adapter"
        and payload.get("isolateCases") is True
        and payload.get("collectOnly") is False
        and payload.get("bootstrapOnly") is False
        and payload.get("manifestCases") is False
        and payload.get("exitCode") == 1,
        f"{mode} sidecar identity or execution scope differs",
    )
    cases = _junit_cases(mode, result.read_bytes())
    require(list(cases) == target_ids, f"{mode} XML IDs/order differ from inventory")
    return payload, cases


def verify_bundle(output_dir: Path = OUTPUT_DIR) -> dict[str, Any]:
    """Verify the one fixed successor and all of its immutable inputs."""
    require(
        output_dir == OUTPUT_DIR
        and output_dir.is_dir()
        and not output_dir.is_symlink(),
        "successor directory missing or moved",
    )
    expected_files = {"promotion-receipt.json"} | {
        name
        for mode in MODES
        for name in (
            output_name(mode),
            Path(output_name(mode)).with_suffix(".summary.json").name,
        )
    }
    require(
        {path.name for path in output_dir.iterdir()} == expected_files,
        "successor file set differs",
    )
    require(
        all(
            (output_dir / name).is_file() and not (output_dir / name).is_symlink()
            for name in expected_files
        ),
        "successor contains a missing or linked file",
    )
    receipt = json_object(
        (output_dir / "promotion-receipt.json").read_bytes(), "promotion receipt"
    )
    check_receipt_scope(receipt)
    base_receipt, inventory, base_ledgers = _verify_base()
    _verify_source_pins()
    target_ids = [inventory[index]["id"] for index in TARGET_INDEXES]
    require(
        receipt.get("inventory") == str(INVENTORY)
        and receipt.get("inventorySha256") == INVENTORY_SHA256
        and receipt.get("recordsPerLane") == RECORDS
        and receipt.get("upstreamCommit") == COMMIT
        and receipt.get("manifest") == str(MANIFEST)
        and receipt.get("manifestSha256") == MANIFEST_SHA256
        and receipt.get("upstream") == str(UPSTREAM)
        and receipt.get("upstreamNavigationSha256") == NAVIGATION_SHA256
        and receipt.get("sourcePins") == str(SOURCE_PINS)
        and receipt.get("sourcePinsSha256") == SOURCE_PINS_SHA256
        and receipt.get("frozenWheel") == str(FROZEN_WHEEL)
        and receipt.get("frozenWheelSha256") == FROZEN_WHEEL_SHA256
        and receipt.get("baseReceipt") == str(BASE_DIR / "promotion-receipt.json")
        and receipt.get("baseReceiptSha256") == BASE_RECEIPT_SHA256
        and receipt.get("basePromoterSha256") == BASE_PROMOTER_SHA256
        and receipt.get("targetZeroBasedIndexes") == list(TARGET_INDEXES)
        and receipt.get("targetTestIds") == target_ids,
        "promotion receipt source or target identity differs",
    )
    require(
        base_receipt.get("promoterSha256") == receipt["basePromoterSha256"],
        "base promoter receipt SHA differs",
    )
    promoter_sha = receipt.get("promoterSha256")
    require(isinstance(promoter_sha, str), "promoter source SHA missing")
    sealed(PROMOTER, promoter_sha, "navigation promoter source")
    require(isinstance(receipt.get("bases"), dict), "receipt bases missing")
    require(isinstance(receipt.get("reports"), dict), "receipt reports missing")
    require(isinstance(receipt.get("successors"), dict), "receipt successors missing")
    require(set(receipt["bases"]) == set(MODES), "receipt base lanes differ")
    require(set(receipt["reports"]) == set(MODES), "receipt report lanes differ")
    require(set(receipt["successors"]) == set(MODES), "receipt successor lanes differ")
    for mode in MODES:
        ledger_sha, summary_sha = BASE_HASHES[mode]
        base_ledger = BASE_DIR / base_name(mode)
        require(
            receipt["bases"][mode]
            == {
                "ledger": str(base_ledger),
                "ledgerSha256": ledger_sha,
                "summary": str(base_ledger.with_suffix(".summary.json")),
                "summarySha256": summary_sha,
            },
            f"{mode} receipt base differs",
        )
        payload, cases = _verify_report(mode, target_ids)
        sidecar, result = report_paths(mode)
        sidecar_sha, result_sha, case_reports_sha, expected_outcomes = REPORT_HASHES[
            mode
        ]
        require(
            receipt["reports"][mode]
            == {
                "provenance": str(sidecar),
                "provenanceSha256": sidecar_sha,
                "result": str(result),
                "resultSha256": result_sha,
                "observedOutcomes": expected_outcomes,
                "isolatedCaseIdsSha256": ISOLATED_IDS_SHA256,
                "caseReportsSha256": case_reports_sha,
            },
            f"{mode} receipt report differs",
        )
        require(
            payload.get("resultSha256") == receipt["reports"][mode]["resultSha256"],
            f"{mode} source-bound result SHA differs",
        )
        output_ledger = output_dir / output_name(mode)
        successor_lines, _ = jsonl(
            output_ledger.read_bytes(), f"{mode} successor ledger"
        )
        counts = check_reconciliation(
            base_ledgers[mode],
            successor_lines,
            inventory,
            list(TARGET_INDEXES),
            cases,
            str(result),
            mode,
        )
        summary_path = output_ledger.with_suffix(".summary.json")
        summary = json_object(summary_path.read_bytes(), f"{mode} successor summary")
        expected_summary = {
            "schema": 1,
            "inventory": str(INVENTORY),
            "records": RECORDS,
            "outcomes": dict(sorted(counts.items())),
            "ledgerSha256": sha256(output_ledger.read_bytes()),
            "ledger": str(output_ledger),
        }
        require(summary == expected_summary, f"{mode} successor summary differs")
        require(
            receipt["successors"][mode] == summary, f"{mode} receipt successor differs"
        )
    return receipt


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify-existing", action="store_true")
    args = parser.parse_args()
    if not args.verify_existing:
        parser.error("pass --verify-existing for read-only verification")
    try:
        receipt = verify_bundle()
    except (OSError, NavigationPromotionValidationError, ET.ParseError) as error:
        parser.error(str(error))
    print(
        json.dumps(
            {"verified": str(OUTPUT_DIR), "successors": receipt["successors"]},
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
