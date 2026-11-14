#!/usr/bin/env python3
"""Publish only an independently proved post-fix four-case Python dummy slice.

This does not assert general Firefox/WebKit support or full public API parity.
The candidate and oracle each need four fresh, unchanged official assertions,
including a real exit-one dummy child, before any v1 outcome is replaced.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import shutil
import tempfile
import xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path
from typing import Any, NoReturn

import promote_playwright_python_dummy_engine_strict_pair as previous
import verify_playwright_python_dummy_engine_strict_v2 as strict

ROOT = Path(__file__).resolve().parents[1]
OUTCOMES = ROOT / ".work/official-conformance/outcomes"
BASE_DIR = OUTCOMES / "python-dummy-engine-abi121-strict-paired-v1"
OUTPUT_DIR = OUTCOMES / "python-dummy-engine-post-fix-strict-paired-v2"
RESULT_DIR = ROOT / ".work/official-conformance/results/python-dummy-engine-post-fix-strict-v2"
INVENTORY = previous.INVENTORY
INVENTORY_SHA256 = previous.INVENTORY_SHA256
BASE_RECEIPT_SHA256 = "c01148322f9733743716cc86da1749497d1bee45766f93cc240c388d0c8b3b5a"
BASE_PROMOTER_SHA256 = "8417eac2a2068697c405a84211700f2c42746d1a0a6e0d3cb8d73e32600382cc"
BASES = {
    "candidate": (
        "dd0609e85a704f97c55762587126c0790d80a444bc116cc9f7e3f55c8d0efb9d",
        "df1c7841a53cf325eb39cf6f16fe08a1d30d312695d2ef57353acd23f3e10dec",
    ),
    "oracle": (
        "f15b5292a64e27607bc632074fcac26457f879decf569ff8eb616435588e9b95",
        "4e5ce6a0578eda9de346cfb623a200534ff70d12c225ad093929585975894b5c",
    ),
}
MODES = ("candidate", "oracle")
APIS = ("async", "sync")
BROWSERS = ("firefox", "webkit")
RECORDS = 6_526
TARGET_INDEXES = (2411, 2412, 5627, 5628)
TARGET_KEYS = tuple((mode, api, browser) for mode in MODES for api in APIS for browser in BROWSERS)
SIDECARS = {
    key: RESULT_DIR / f"{key[0]}-{key[1]}-{key[2]}-v1.provenance.json" for key in TARGET_KEYS
}
RECEIPT_KIND = "playwright-python-nonchromium-dummy-node-post-fix-strict-paired-promotion-v2"
GENERATED = frozenset({"attempts", "durationSeconds", "error", "outcome", "resultFile"})

# Fill these only from independently verified, completed eight-case capture.
# Empty values deliberately disable promotion and --verify-existing.
POLICY_SHA256 = "faa4a608951f661d26342aeeed21d76b976f8170fe606bdbe81f46b3145e9c63"
CANDIDATE_WHEEL_SHA256 = "904db7f0af2082451edfbe589afcedd9fdc77295ce627cbc391d9e60b9780367"
TOOL_SHA256 = {
    "scripts/run_playwright_python_dummy_engine_strict_v2.py": (
        "a4f0e0ef31fc5770eb4525710c61886e209b10c45794490aa24168a976304ba4"
    ),
    "scripts/verify_playwright_python_dummy_engine_strict_v2.py": (
        "b6b71cceca4563ddc3d49dca0cc58ff6f961801bc470f86ad324f10b180f7abb"
    ),
    (
        "packages/pwright-python/tests/official/dummy_engine_guard_strict_v2/"
        "pwright_official_dummy_engine_guard_v2.py"
    ): "014a3f78d4087622ae96acb6ba92fe15d9c4f185aa66203bb2e5759bd805eadf",
    "packages/pwright-python/tests/official/dummy_engine_guard_strict_v2/node": (
        "7a41a91c7d26b363d7097eff5e28b0e47b1a46fe0d6a5c55496452b1ff31a766"
    ),
}
SIDECAR_SHA256 = {
    (
        "candidate",
        "async",
        "firefox",
    ): "53a2fa2bb6924219ea865d8dc3ce650b03f04052a454611defbc50ed6733ee16",
    (
        "candidate",
        "async",
        "webkit",
    ): "7f92847f2064ada45a3b13598816dc3a7c5c9a2562fa76bb37140b54fc0b351c",
    (
        "candidate",
        "sync",
        "firefox",
    ): "385048722b9140a1275ba9eeab7ec2af2e367eb03f29b1f95b6bd2a3bcf0f762",
    (
        "candidate",
        "sync",
        "webkit",
    ): "d89d64de13ddd67e03bc8f2393af3b9d12a2375657f3805b1bc2580ba8f46a1b",
    (
        "oracle",
        "async",
        "firefox",
    ): "69d4a4c08aa819af2f95513c87aa4bc06e5a8d93db8b1da27f66a3cb21e0020d",
    (
        "oracle",
        "async",
        "webkit",
    ): "94a670c08f996912e9a5a62cb214119fa572b6d9cfb5582a32aee6dee688e347",
    (
        "oracle",
        "sync",
        "firefox",
    ): "c4d25f1e10a375b3504381c1aba5dd886606f139777f5ecd04189dee53632ef1",
    (
        "oracle",
        "sync",
        "webkit",
    ): "10f49fe5d457b5850844f0937b79a22c5550181d4d9d6c7e438ced8bdb6a6ec5",
}


class PromotionError(ValueError):
    """Strict paired evidence cannot justify a four-row successor."""


def fail(message: str) -> NoReturn:
    raise PromotionError(message)


def require(condition: bool, message: str) -> None:
    if not condition:
        fail(message)


def sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def digest_pin(expected: str, label: str) -> None:
    require(
        len(expected) == 64 and all(character in "0123456789abcdef" for character in expected),
        f"unfilled or invalid SHA-256 pin: {label}",
    )


def sealed(path: Path, expected: str, label: str) -> bytes:
    digest_pin(expected, label)
    require(path.is_file() and not path.is_symlink(), f"{label} is missing or linked: {path}")
    raw = path.read_bytes()
    require(sha256(raw) == expected, f"{label} SHA-256 differs")
    return raw


def json_object(raw: bytes, label: str) -> dict[str, Any]:
    try:
        value = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise PromotionError(f"{label} is not valid JSON") from error
    require(isinstance(value, dict), f"{label} is not a JSON object")
    return value


def jsonl(raw: bytes, label: str) -> tuple[list[bytes], list[dict[str, Any]]]:
    require(raw.endswith(b"\n"), f"{label} is not newline terminated")
    lines = raw.splitlines(keepends=True)
    require(len(lines) == RECORDS, f"{label} is not the full {RECORDS}-row corpus")
    return lines, [json_object(line, f"{label} row {index}") for index, line in enumerate(lines)]


def target(api: str, browser: str) -> str:
    return f"tests/{api}/test_launcher.py::{strict.TEST_NAME}[{browser}]"


def base() -> dict[str, tuple[list[bytes], list[dict[str, Any]]]]:
    """Validate both immutable red baselines against the 6,526-ID inventory."""
    sealed(Path(previous.__file__).resolve(), BASE_PROMOTER_SHA256, "v1 promoter source")
    inventory_raw = sealed(INVENTORY, INVENTORY_SHA256, "pinned Python inventory")
    _, inventory = jsonl(inventory_raw, "pinned Python inventory")
    ids = [row.get("id") for row in inventory]
    require(all(isinstance(node_id, str) for node_id in ids), "inventory ID is malformed")
    require(len(set(ids)) == RECORDS, "inventory has duplicate IDs")
    expected = tuple(target(api, browser) for api in APIS for browser in BROWSERS)
    require(tuple(ids[index] for index in TARGET_INDEXES) == expected, "four target IDs moved")
    receipt = json_object(
        sealed(BASE_DIR / "promotion-receipt.json", BASE_RECEIPT_SHA256, "v1 paired receipt"),
        "v1 paired receipt",
    )
    require(
        receipt.get("kind") == previous.RECEIPT_KIND
        and receipt.get("inventory") == str(INVENTORY)
        and receipt.get("inventorySha256") == INVENTORY_SHA256
        and receipt.get("promoterSha256") == BASE_PROMOTER_SHA256
        and receipt.get("commit") == strict.COMMIT,
        "v1 paired receipt identity differs",
    )
    result: dict[str, tuple[list[bytes], list[dict[str, Any]]]] = {}
    for mode in MODES:
        path = BASE_DIR / f"playwright-python-{mode}-dummy-engine-launch-strict-paired-v1.jsonl"
        ledger_sha, summary_sha = BASES[mode]
        ledger_raw = sealed(path, ledger_sha, f"{mode} v1 ledger")
        lines, rows = jsonl(ledger_raw, f"{mode} v1 ledger")
        summary_raw = sealed(path.with_suffix(".summary.json"), summary_sha, f"{mode} v1 summary")
        summary = json_object(summary_raw, f"{mode} v1 summary")
        require(
            receipt["successors"][mode] == summary
            and summary
            == {
                "schema": 1,
                "inventory": str(INVENTORY),
                "records": RECORDS,
                "outcomes": dict(sorted(Counter(row.get("outcome") for row in rows).items())),
                "ledgerSha256": ledger_sha,
                "ledger": str(path),
            },
            f"{mode} v1 summary differs from pinned ledger",
        )
        require(
            receipt["reports"] and receipt["successors"][mode]["ledgerSha256"] == ledger_sha,
            f"{mode} v1 receipt does not cite pinned ledger",
        )
        for index, (row, inventory_row) in enumerate(zip(rows, inventory)):
            require(row.get("id") == ids[index], f"{mode} v1 ledger row order differs")
            require(
                {key: value for key, value in row.items() if key not in GENERATED} == inventory_row,
                f"{mode} v1 row metadata differs from inventory at {index}",
            )
            if index in TARGET_INDEXES:
                require(
                    row.get("sourceClassification") == "public-api-source"
                    and row.get("attempts") == 1
                    and row.get("outcome") == ("failed" if mode == "candidate" else "passed")
                    and (
                        row.get("error")
                        == "_native.UnsupportedError: launch is only available for Chromium"
                    )
                    == (mode == "candidate"),
                    f"{mode} v1 target is not the pinned red/oracle baseline",
                )
        result[mode] = lines, rows
    return result


def tools_ready() -> None:
    expected_sidecars = {
        key: RESULT_DIR / f"{key[0]}-{key[1]}-{key[2]}-v1.provenance.json" for key in TARGET_KEYS
    }
    require(SIDECARS == expected_sidecars, "v2 exact eight sidecar slots differ")
    require(len(set(SIDECARS.values())) == len(TARGET_KEYS), "v2 sidecar paths were reused")
    require(POLICY_SHA256 == strict.POLICY_SHA256, "v2 policy pin differs from verifier")
    sealed(strict.POLICY, POLICY_SHA256, "v2 strict policy")
    digest_pin(CANDIDATE_WHEEL_SHA256, "candidate wheel")
    require(
        set(TOOL_SHA256)
        == set(strict._CAPTURE_PATHS)
        | {"scripts/verify_playwright_python_dummy_engine_strict_v2.py"},
        "v2 capture tool set differs",
    )
    for relative, expected in TOOL_SHA256.items():
        sealed(ROOT / relative, expected, f"v2 tool {relative}")
    require(set(SIDECAR_SHA256) == set(TARGET_KEYS), "v2 sidecar pin set differs")


def report_duration(payload: dict[str, Any]) -> float:
    result = Path(payload["result"])
    raw = sealed(result, payload["resultSha256"], "v2 official JUnit")
    try:
        root = ET.fromstring(raw)
    except ET.ParseError as error:
        raise PromotionError("v2 official JUnit became malformed") from error
    suites = root.findall("testsuite") if root.tag == "testsuites" else []
    cases = list(root.iter("testcase"))
    require(len(suites) == len(cases) == 1, "v2 JUnit is not one official case")
    suite, case = suites[0], cases[0]
    require(
        len(suite.findall("testcase")) == 1
        and case.get("classname") == f"tests.{payload['api']}.test_launcher"
        and case.get("name") == f"{strict.TEST_NAME}[{payload['browser']}]"
        and all(
            suite.get(field) == value
            for field, value in (
                ("tests", "1"),
                ("failures", "0"),
                ("errors", "0"),
                ("skipped", "0"),
            )
        )
        and all(case.find(field) is None for field in ("failure", "error", "skipped"))
        and payload["exitCode"] == 0,
        "v2 official assertion did not pass once",
    )
    try:
        duration = float(case.get("time", ""))
    except ValueError as error:
        raise PromotionError("v2 JUnit duration is invalid") from error
    require(math.isfinite(duration) and duration >= 0, "v2 JUnit duration is invalid")
    return duration


def reports() -> dict[tuple[str, str, str], dict[str, Any]]:
    tools_ready()
    verified: dict[tuple[str, str, str], dict[str, Any]] = {}
    for key in TARGET_KEYS:
        mode, api, browser = key
        path = SIDECARS[key]
        before = sealed(path, SIDECAR_SHA256[key], f"v2 {key} strict sidecar")
        payload = strict.verify_provenance(path)
        require(path.read_bytes() == before, f"v2 {key} strict sidecar changed during verification")
        require(
            payload.get("schema") == 3
            and payload.get("kind") == strict.KIND
            and payload.get("evidenceScope") == strict.SCOPE
            and (payload.get("mode"), payload.get("api"), payload.get("browser")) == key
            and payload.get("testId") == target(api, browser)
            and payload.get("manifest") == str(strict.POLICY)
            and payload.get("manifestSha256") == POLICY_SHA256
            and payload.get("commit") == strict.COMMIT
            and payload.get("launchMode") == "owned"
            and payload.get("endpoint") is None
            and payload.get("collectOnly") is False
            and payload.get("bootstrapOnly") is False
            and payload.get("exitCode") == 0
            and payload.get("verifierSha256")
            == TOOL_SHA256["scripts/verify_playwright_python_dummy_engine_strict_v2.py"],
            f"v2 {key} strict run identity differs",
        )
        require(
            path == Path(payload["result"]).with_suffix(".provenance.json"),
            f"v2 {key} sidecar is not its JUnit sibling",
        )
        if mode == "candidate":
            require(
                payload.get("wheelSha256") == CANDIDATE_WHEEL_SHA256,
                f"v2 {key} candidate wheel differs",
            )
        else:
            require(
                payload.get("wheelSha256") is None,
                f"v2 {key} oracle carried a candidate wheel",
            )
        process = sealed(
            Path(payload["processReceipt"]),
            payload["processReceiptSha256"],
            f"v2 {key} dummy process receipt",
        )
        process_rows = process.splitlines()
        require(len(process_rows) == 1, f"v2 {key} has not exactly one dummy process")
        child = json_object(process_rows[0], f"v2 {key} dummy process")
        require(
            child.get("event") == "dummy-node-exited"
            and child.get("exitCode") == 1
            and child.get("runNonce") == payload.get("runNonce")
            and child.get("mode") == mode
            and child.get("api") == api
            and child.get("browser") == browser,
            f"v2 {key} lacks an independent exit-one child",
        )
        public = sealed(
            Path(payload["publicReceipt"]),
            payload["publicReceiptSha256"],
            f"v2 {key} public launch receipt",
        )
        public_rows = public.splitlines()
        require(len(public_rows) == 2, f"v2 {key} lacks two public launch events")
        invoked = json_object(public_rows[0], f"v2 {key} public invocation")
        failed = json_object(public_rows[1], f"v2 {key} public failure")
        require(
            invoked.get("event") == "public-launch-invoked"
            and invoked.get("runNonce") == payload.get("runNonce")
            and failed.get("event") == "public-launch-failed"
            and failed.get("runNonce") == payload.get("runNonce")
            and failed.get("processReceiptSha256") == payload["processReceiptSha256"]
            and failed.get("errorClass")
            == {"firefox": "Error", "webkit": "TargetClosedError"}[browser],
            f"v2 {key} public failure is not the expected browser-specific error",
        )
        payload = dict(payload)
        payload["_duration"] = report_duration(payload)
        payload["_sidecarSha256"] = sha256(before)
        verified[key] = payload
    reference = verified[TARGET_KEYS[0]]
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
    for key, payload in verified.items():
        require(
            all(payload.get(field) == reference.get(field) for field in common),
            f"v2 {key} source, fixture, or tool identity differs across lanes",
        )
    for mode in MODES:
        lane = [verified[(mode, api, browser)] for api in APIS for browser in BROWSERS]
        fields = ("pythonExecutable", "playwrightPackage", "driverNode")
        if mode == "candidate":
            fields += ("wheelSha256", "pwrightPackageSha256", "nativeSha256", "runtimeLibraries")
        for field in fields:
            require(
                all(payload.get(field) == lane[0].get(field) for payload in lane),
                f"v2 {mode} {field} differs across four cases",
            )
    for field in ("result", "publicReceipt", "processReceipt", "runNonce"):
        require(
            len({payload.get(field) for payload in verified.values()}) == len(TARGET_KEYS),
            f"v2 strict {field} was reused across independent executions",
        )
    return verified


def successor(
    mode: str,
    originals: tuple[list[bytes], list[dict[str, Any]]],
    evidence: dict[tuple[str, str, str], dict[str, Any]],
    output: Path,
) -> tuple[bytes, bytes, dict[str, Any]]:
    lines, rows = originals
    selected = {
        target(api, browser): evidence[(mode, api, browser)] for api in APIS for browser in BROWSERS
    }
    updated = list(lines)
    changes: list[int] = []
    for index, row in enumerate(rows):
        report = selected.get(row["id"])
        if report is None:
            continue
        replacement = {key: value for key, value in row.items() if key != "error"}
        replacement.update(
            {
                "outcome": "passed",
                "attempts": 1,
                "durationSeconds": report["_duration"],
                "resultFile": report["result"],
            }
        )
        updated[index] = (
            json.dumps(replacement, sort_keys=True, separators=(",", ":")) + "\n"
        ).encode()
        changes.append(index)
    require(tuple(changes) == TARGET_INDEXES, f"{mode} successor changed the wrong four rows")
    require(
        all(
            updated[index] == original
            for index, original in enumerate(lines)
            if index not in TARGET_INDEXES
        ),
        f"{mode} successor changed one of the other 6,522 rows",
    )
    ledger = b"".join(updated)
    outcomes = Counter(row["outcome"] for row in rows)
    if mode == "candidate":
        outcomes["failed"] -= 4
        outcomes["passed"] += 4
    require(all(count >= 0 for count in outcomes.values()), f"{mode} outcome count underflow")
    summary = {
        "schema": 1,
        "inventory": str(INVENTORY),
        "records": RECORDS,
        "outcomes": dict(sorted(outcomes.items())),
        "ledgerSha256": sha256(ledger),
        "ledger": str(output),
    }
    return ledger, (json.dumps(summary, indent=2) + "\n").encode(), summary


def prepare(output_dir: Path) -> tuple[dict[str, bytes], dict[str, Any]]:
    require(output_dir.is_absolute(), "successor directory must be absolute")
    originals = base()
    evidence = reports()
    contents: dict[str, bytes] = {}
    summaries: dict[str, dict[str, Any]] = {}
    for mode in MODES:
        name = f"playwright-python-{mode}-dummy-engine-launch-strict-paired-v2.jsonl"
        ledger, summary_bytes, summary = successor(
            mode, originals[mode], evidence, output_dir / name
        )
        contents[name] = ledger
        contents[Path(name).with_suffix(".summary.json").name] = summary_bytes
        summaries[mode] = summary
    receipt = {
        "schema": 2,
        "kind": RECEIPT_KIND,
        "scope": "four-unchanged-official-dummy-engine-launch-failure-cases-only",
        "sourceClassificationPreserved": "public-api-source",
        "focusedPublicBehaviorEvidence": True,
        "fullFirefoxWebkitSupport": False,
        "publicApiParityEvidence": False,
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "recordsPerLane": RECORDS,
        "targetZeroBasedIndexes": list(TARGET_INDEXES),
        "targetTestIds": [target(api, browser) for api in APIS for browser in BROWSERS],
        "manifest": str(strict.POLICY),
        "manifestSha256": POLICY_SHA256,
        "commit": strict.COMMIT,
        "promoterSha256": sha256(Path(__file__).read_bytes()),
        "candidateWheelSha256": CANDIDATE_WHEEL_SHA256,
        "toolSha256": TOOL_SHA256,
        "baseReceiptSha256": BASE_RECEIPT_SHA256,
        "bases": {
            mode: {
                "ledger": str(
                    BASE_DIR
                    / f"playwright-python-{mode}-dummy-engine-launch-strict-paired-v1.jsonl"
                ),
                "ledgerSha256": BASES[mode][0],
                "oldOutcome": "failed" if mode == "candidate" else "passed",
            }
            for mode in MODES
        },
        "reports": [
            {
                "mode": mode,
                "api": api,
                "browser": browser,
                "testId": target(api, browser),
                "provenance": str(SIDECARS[(mode, api, browser)]),
                "provenanceSha256": evidence[(mode, api, browser)]["_sidecarSha256"],
                "result": evidence[(mode, api, browser)]["result"],
                "resultSha256": evidence[(mode, api, browser)]["resultSha256"],
                "publicReceiptSha256": evidence[(mode, api, browser)]["publicReceiptSha256"],
                "processReceiptSha256": evidence[(mode, api, browser)]["processReceiptSha256"],
                "dummyProcessExitCode": 1,
                "outcome": "passed",
            }
            for mode, api, browser in TARGET_KEYS
        ],
        "successors": summaries,
    }
    contents["promotion-receipt.json"] = (
        json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    ).encode()
    return contents, receipt


def publish(output_dir: Path) -> dict[str, Any]:
    previous._check_output(output_dir)
    contents, receipt = prepare(output_dir)
    staging = Path(tempfile.mkdtemp(prefix=f".{output_dir.name}.staging-", dir=output_dir.parent))
    try:
        for name, raw in contents.items():
            with (staging / name).open("xb") as stream:
                stream.write(raw)
                stream.flush()
                os.fsync(stream.fileno())
        previous._fsync_directory(staging)
        previous._check_output(output_dir)
        require(
            prepare(output_dir) == (contents, receipt), "strict evidence changed during staging"
        )
        previous._publish_exclusive(staging, output_dir)
    finally:
        if staging.exists():
            shutil.rmtree(staging)
    return receipt


def verify_bundle(output_dir: Path) -> dict[str, Any]:
    require(
        output_dir.is_dir() and not output_dir.is_symlink(), "v2 paired bundle is missing or linked"
    )
    contents, receipt = prepare(output_dir)
    require(
        {path.name for path in output_dir.iterdir()} == set(contents),
        "v2 paired bundle file set differs",
    )
    for name, expected in contents.items():
        path = output_dir / name
        require(
            path.is_file() and not path.is_symlink(),
            f"v2 paired bundle file is missing or linked: {name}",
        )
        require(path.read_bytes() == expected, f"v2 paired bundle content differs: {name}")
    return receipt


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, default=OUTPUT_DIR)
    parser.add_argument("--verify-existing", action="store_true")
    args = parser.parse_args()
    output_dir = args.output_dir.absolute()
    try:
        receipt = verify_bundle(output_dir) if args.verify_existing else publish(output_dir)
    except (
        OSError,
        PromotionError,
        previous.PromotionError,
        strict.StrictEvidenceError,
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
