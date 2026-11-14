"""Promote only six proved official Python closed-endpoint connect cases.

Both lanes start from the immutable 6,526-row connect-close v2 paired ledgers.
All twelve fresh strict sidecars must verify before either successor is staged.
This proves only a fixture-adapted refused-connection error, not a working
Playwright Server, remote Browser, or public API parity.
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
from typing import Any, NoReturn

import verify_playwright_python_connect_refused_strict_v1 as strict

ROOT = Path(__file__).resolve().parents[1]
OUTCOMES = ROOT / ".work/official-conformance/outcomes"
BASE_DIR = OUTCOMES / "python-connect-close-post-fix-strict-paired-v2"
OUTPUT_DIR = OUTCOMES / "python-connect-refused-strict-paired-v1"
RESULT_DIR = (
    ROOT / ".work/official-conformance/results/python-connect-refused-strict-v1"
)
INVENTORY = ROOT / ".work/official-conformance/playwright-python-v1.58.0-profiled.jsonl"
INVENTORY_SHA256 = "3dfb11b6ad7eb4736166ee2c4f7fa32cda78063324ad8314ce12f3e2d49b9d41"
BASE_PROMOTER_SHA256 = (
    "29aef75a621f26d8b52a3c5c2bf92e2281cefb6cea21ce8f97a670378359c1db"
)
BASE_RECEIPT_SHA256 = "03b22f0bc8e986217e9c6d9c301139049a0d268795ba15197ed5f4716bb52b72"
BASES = {
    "candidate": (
        "1acbae02e2049b96d2556a3093b8a65cd43b12ac00f53745ee6f58c764990afb",
        "e69dd6842a946d360d7202eb10a7788c594b744e40ea3f16b144598c0db0166f",
    ),
    "oracle": (
        "7d044a471c9ee530de390d676d50478d28c3df43a4f106b60528809c7a114ec7",
        "668af9914ff36314f3c434ba8bf5538abce3db2ef28eb57a49508036a9a49423",
    ),
}
MODES = ("candidate", "oracle")
APIS = ("async", "sync")
ENGINES = ("chromium", "firefox", "webkit")
CASES = tuple((api, engine) for api in APIS for engine in ENGINES)
KEYS = tuple((mode, api, engine) for mode in MODES for api, engine in CASES)
RECORDS = 6_526
TARGET_INDEXES = (829, 830, 831, 4987, 4988, 4989)
GENERATED = frozenset({"attempts", "durationSeconds", "error", "outcome", "resultFile"})
RECEIPT_KIND = "playwright-python-browser-connect-refused-strict-paired-promotion-v1"
SIDECARS = {
    key: RESULT_DIR / f"{key[0]}-{key[1]}-{key[2]}.provenance.json" for key in KEYS
}

POLICY_SHA256 = "87f44977cc9d66b641e03b999e0178c518c50e9ef5014bd2e897afce1b42976b"
CANDIDATE_WHEEL_SHA256 = (
    "3b86f0b56c6dbc2654a3326cd91b62aed0ed2b8d22537e69476226565bfed6d1"
)
TOOL_SHA256 = {
    "scripts/run_playwright_python_connect_refused_strict_v1.py": (
        "f75bab030d3af58b74c2547294ac130f3e87f4219e0a333cbb55cc1ac7bc9303"
    ),
    "scripts/verify_playwright_python_connect_refused_strict_v1.py": (
        "3d436ed9e0ea6bfcc7196ce59e01e5b700f8a9eeba7eed748a214be1370ead56"
    ),
    (
        "packages/pwright-python/tests/official/ws_connect_refused_guard_strict_v1/"
        "pwright_official_connect_refused_guard_v1.py"
    ): "10d6ede9b5a8ecf552bb6caaab6d33474e40d23cc1ca9a00de1a988cfa9d9450",
}
SIDECAR_SHA256 = {
    (
        "candidate",
        "async",
        "chromium",
    ): "d31d2b5981be5bb56d5b8f8db8c10747c1c67584b3cfa05437350e19a120f396",
    (
        "candidate",
        "async",
        "firefox",
    ): "62693cae23b3185e95861360dee21ca4d0ce5ef4d390508eca0fdaa8750fa3fb",
    (
        "candidate",
        "async",
        "webkit",
    ): "1cd4e2e01506539e2a45115335f583034fdb1acca9b561a07df53668430180a3",
    (
        "candidate",
        "sync",
        "chromium",
    ): "4517a5dda63d7530bb14eba43d1b811eca26eb5eb9a699d140260227a22247e3",
    (
        "candidate",
        "sync",
        "firefox",
    ): "ec24bd742aca410a46b2ee0fd4344c599b32599a0d4846eb8d9fdbf6cd054888",
    (
        "candidate",
        "sync",
        "webkit",
    ): "0cfc8dd2c9f20d724fd2d30f506162f23c1372ec04453dc033ac76daab096892",
    (
        "oracle",
        "async",
        "chromium",
    ): "941596b17a1ca433f2c6005a77b464f2df4976a3f665329a5a0bb5cffa291e26",
    (
        "oracle",
        "async",
        "firefox",
    ): "e0a3f390397184c7cea600e21654ae9bc44677e45fae7baca0d35f09255df781",
    (
        "oracle",
        "async",
        "webkit",
    ): "78aa4d9c712c040d636f409cd40497b93faff199d4de010832abcacfef9f2372",
    (
        "oracle",
        "sync",
        "chromium",
    ): "502cee1737a85c9ea86c448a9a3228f81470b4ed31397ae1d52b72ca7aebf79f",
    (
        "oracle",
        "sync",
        "firefox",
    ): "dada5741793d0a6344b3a3245b8377988b9d0c03d7c1e5d5f5713c2cdcc9c839",
    (
        "oracle",
        "sync",
        "webkit",
    ): "e1855bedf142f79c7eb6ce86bfb3063ca722e48130cd31ab45a544b4463ca02a",
}


class PromotionError(ValueError):
    """The evidence does not justify a paired successor."""


def fail(message: str) -> NoReturn:
    raise PromotionError(message)


def require(condition: bool, message: str) -> None:
    if not condition:
        fail(message)


def sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def digest_pin(expected: str, label: str) -> None:
    require(
        len(expected) == 64
        and all(character in "0123456789abcdef" for character in expected),
        f"unfilled or invalid SHA-256 pin: {label}",
    )


def sealed(path: Path, expected: str, label: str) -> bytes:
    digest_pin(expected, label)
    require(
        path.is_file() and not path.is_symlink(), f"{label} missing or linked: {path}"
    )
    raw = path.read_bytes()
    require(sha256(raw) == expected, f"{label} SHA-256 differs")
    return raw


def json_object(raw: bytes, label: str) -> dict[str, Any]:
    try:
        value = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise PromotionError(f"{label} is not valid JSON") from error
    require(isinstance(value, dict), f"{label} is not an object")
    return value


def jsonl(raw: bytes, label: str) -> tuple[list[bytes], list[dict[str, Any]]]:
    require(raw.endswith(b"\n"), f"{label} is not newline terminated")
    lines = raw.splitlines(keepends=True)
    require(len(lines) == RECORDS, f"{label} is not the full {RECORDS}-row corpus")
    return lines, [
        json_object(line, f"{label} row {index}") for index, line in enumerate(lines)
    ]


def target(api: str, engine: str) -> str:
    return strict.TEST_IDS[CASES.index((api, engine))]


def base() -> dict[str, tuple[list[bytes], list[dict[str, Any]]]]:
    """Read the pinned connect-close v2 bundle without consulting newer ledgers."""
    sealed(
        ROOT / "scripts/promote_playwright_python_connect_close_strict_v2.py",
        BASE_PROMOTER_SHA256,
        "base promoter source",
    )
    _, inventory = jsonl(
        sealed(INVENTORY, INVENTORY_SHA256, "Python inventory"), "inventory"
    )
    ids = [row.get("id") for row in inventory]
    require(
        all(isinstance(node_id, str) for node_id in ids), "inventory ID is malformed"
    )
    require(len(set(ids)) == RECORDS, "inventory has duplicate IDs")
    require(
        tuple(ids[index] for index in TARGET_INDEXES) == strict.TEST_IDS,
        "six BrowserType.connect target IDs moved",
    )
    receipt = json_object(
        sealed(
            BASE_DIR / "promotion-receipt.json",
            BASE_RECEIPT_SHA256,
            "base paired receipt",
        ),
        "base paired receipt",
    )
    require(
        receipt.get("kind")
        == "playwright-python-browser-connect-close-post-fix-strict-paired-promotion-v2"
        and receipt.get("inventory") == str(INVENTORY)
        and receipt.get("inventorySha256") == INVENTORY_SHA256
        and receipt.get("promoterSha256") == BASE_PROMOTER_SHA256
        and receipt.get("commit") == strict.COMMIT
        and receipt.get("recordsPerLane") == RECORDS
        and receipt.get("focusedPublicBehaviorEvidence") is True
        and receipt.get("fullPlaywrightServerSupport") is False
        and receipt.get("publicApiParityEvidence") is False,
        "base paired receipt identity differs",
    )
    result: dict[str, tuple[list[bytes], list[dict[str, Any]]]] = {}
    for mode in MODES:
        path = (
            BASE_DIR
            / f"playwright-python-{mode}-browser-connect-close-strict-paired-v2.jsonl"
        )
        ledger_sha, summary_sha = BASES[mode]
        lines, rows = jsonl(
            sealed(path, ledger_sha, f"{mode} base ledger"), f"{mode} base ledger"
        )
        summary = json_object(
            sealed(
                path.with_suffix(".summary.json"), summary_sha, f"{mode} base summary"
            ),
            f"{mode} base summary",
        )
        require(
            receipt.get("successors", {}).get(mode) == summary
            and summary
            == {
                "schema": 1,
                "inventory": str(INVENTORY),
                "records": RECORDS,
                "outcomes": dict(
                    sorted(Counter(row.get("outcome") for row in rows).items())
                ),
                "ledgerSha256": ledger_sha,
                "ledger": str(path),
            },
            f"{mode} base summary differs from pinned ledger",
        )
        for index, (row, inventory_row) in enumerate(zip(rows, inventory)):
            require(row.get("id") == ids[index], f"{mode} row order differs at {index}")
            require(
                {key: value for key, value in row.items() if key not in GENERATED}
                == inventory_row,
                f"{mode} metadata differs from inventory at {index}",
            )
            if index in TARGET_INDEXES:
                require(
                    row == {**inventory_row, "attempts": 0, "outcome": "not-run"}
                    and row.get("sourceClassification") == "public-api-source",
                    f"{mode} target is not the exact untouched not-run baseline",
                )
        result[mode] = lines, rows
    return result


def tools_ready() -> None:
    expected_slots = {
        key: RESULT_DIR / f"{key[0]}-{key[1]}-{key[2]}.provenance.json" for key in KEYS
    }
    require(
        SIDECARS == expected_slots and len(set(SIDECARS.values())) == 12,
        "twelve sidecar slots differ",
    )
    digest_pin(POLICY_SHA256, "strict v1 policy")
    require(
        POLICY_SHA256 == strict.POLICY_SHA256,
        "strict v1 policy differs from verifier pin",
    )
    policy = strict._policy(
        json_object(
            sealed(strict.POLICY, POLICY_SHA256, "strict v1 policy"), "strict v1 policy"
        )
    )
    digest_pin(CANDIDATE_WHEEL_SHA256, "candidate wheel")
    require(
        CANDIDATE_WHEEL_SHA256 == policy["candidateWheelSha256"],
        "candidate wheel pin differs from policy",
    )
    expected_tools = set(strict._CAPTURE_PATHS) | {
        "scripts/verify_playwright_python_connect_refused_strict_v1.py"
    }
    require(set(TOOL_SHA256) == expected_tools, "strict v1 tool set differs")
    for relative, expected in TOOL_SHA256.items():
        sealed(ROOT / relative, expected, f"strict v1 tool {relative}")
        if relative in strict._CAPTURE_PATHS:
            require(
                policy["captureSourceSha256"].get(relative) == expected,
                f"strict v1 capture source pin differs: {relative}",
            )
    require(set(SIDECAR_SHA256) == set(KEYS), "twelve sidecar pins are incomplete")
    for key in KEYS:
        digest_pin(SIDECAR_SHA256[key], f"strict v1 sidecar {key}")


def report_duration(payload: dict[str, Any]) -> float:
    result = Path(payload["result"])
    raw = sealed(result, payload["resultSha256"], "strict v1 official JUnit")
    try:
        root = ET.fromstring(raw)
    except ET.ParseError as error:
        raise PromotionError("strict v1 official JUnit became malformed") from error
    suites = root.findall("testsuite") if root.tag == "testsuites" else []
    cases = list(root.iter("testcase"))
    source = payload["testId"].split("::", 1)[0]
    expected_class = (
        "tests.async.test_browsertype_connect"
        if source.startswith("tests/async/")
        else "tests.sync.test_browsertype_connect"
    )
    require(len(suites) == len(cases) == 1, "strict v1 JUnit is not one official case")
    suite, case = suites[0], cases[0]
    require(
        len(suite.findall("testcase")) == 1
        and case.get("classname") == expected_class
        and case.get("name") == payload["testId"].split("::", 1)[1]
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
        and payload["exitCode"] == 0
        and payload["outcome"] == "passed",
        "strict v1 official assertion did not pass once",
    )
    try:
        duration = float(case.get("time", ""))
    except ValueError as error:
        raise PromotionError("strict v1 JUnit duration is invalid") from error
    require(
        math.isfinite(duration) and duration >= 0, "strict v1 JUnit duration is invalid"
    )
    return duration


def reports() -> dict[tuple[str, str, str], dict[str, Any]]:
    tools_ready()
    verified: dict[tuple[str, str, str], dict[str, Any]] = {}
    for key in KEYS:
        mode, api, engine = key
        path = SIDECARS[key]
        before = sealed(path, SIDECAR_SHA256[key], f"strict v1 {key} sidecar")
        payload = strict.verify_provenance(path)
        require(
            path.read_bytes() == before,
            f"strict v1 {key} sidecar changed during verification",
        )
        require(
            payload.get("schema") == 1
            and payload.get("kind") == strict.KIND
            and payload.get("evidenceScope") == strict.SCOPE
            and (payload.get("mode"), payload.get("testId"))
            == (mode, target(api, engine))
            and payload.get("manifest") == str(strict.POLICY)
            and payload.get("manifestSha256") == POLICY_SHA256
            and payload.get("commit") == strict.COMMIT
            and payload.get("wheelSha256") == CANDIDATE_WHEEL_SHA256
            and payload.get("publicConnectPresent") is True
            and payload.get("fixtureAdaptation") == strict.ADAPTATION
            and payload.get("outcome") == "passed"
            and type(payload.get("exitCode")) is int
            and payload["exitCode"] == 0
            and payload.get("verifierSha256")
            == TOOL_SHA256[
                "scripts/verify_playwright_python_connect_refused_strict_v1.py"
            ],
            f"strict v1 {key} is not the exact passed official outcome",
        )
        require(
            path == Path(payload["result"]).with_suffix(".provenance.json")
            and Path(payload["result"]).with_suffix(".call.jsonl")
            == Path(payload["callReceipt"]),
            f"strict v1 {key} output paths differ",
        )
        report = dict(payload)
        report["_duration"] = report_duration(payload)
        report["_sidecarSha256"] = sha256(before)
        verified[key] = report
    reference = verified[KEYS[0]]
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
        "pythonRuntimeSha256",
        "wheelPath",
        "wheelSha256",
        "playwrightPackageSha256",
        "driverNodeSha256",
        "fixtureBind",
        "fixtureHost",
        "fixtureAdaptation",
        "verifierSha256",
    )
    for key, report in verified.items():
        require(
            all(report.get(field) == reference.get(field) for field in common),
            f"strict v1 {key} source, fixture, or artifact identity differs across lanes",
        )
    for mode in MODES:
        lane = [verified[(mode, api, engine)] for api, engine in CASES]
        fields = ("pythonExecutable", "playwrightPackage", "driverNode")
        if mode == "candidate":
            fields += ("pwrightPackageSha256", "nativeSha256", "runtimeLibraries")
        for field in fields:
            require(
                all(report.get(field) == lane[0].get(field) for report in lane),
                f"strict v1 {mode} {field} differs across six executions",
            )
    for field in ("result", "callReceipt", "runNonce"):
        require(
            len({report.get(field) for report in verified.values()}) == 12,
            f"strict v1 {field} was reused across twelve independent executions",
        )
    return verified


def successor(
    mode: str,
    originals: tuple[list[bytes], list[dict[str, Any]]],
    evidence: dict[tuple[str, str, str], dict[str, Any]],
    output: Path,
) -> tuple[bytes, bytes, dict[str, Any]]:
    lines, rows = originals
    require(mode in MODES, "unknown lane")
    require(len(lines) == len(rows) == RECORDS, "successor base length differs")
    updated = list(lines)
    counts = Counter(row["outcome"] for row in rows)
    for (api, engine), index in zip(CASES, TARGET_INDEXES):
        row = rows[index]
        report = evidence[(mode, api, engine)]
        require(
            row["id"] == target(api, engine)
            and row["sourceClassification"] == "public-api-source"
            and row["outcome"] == "not-run"
            and row["attempts"] == 0
            and not any(
                field in row for field in ("error", "durationSeconds", "resultFile")
            ),
            f"{mode} untouched target moved",
        )
        replacement = {
            **row,
            "outcome": "passed",
            "attempts": 1,
            "durationSeconds": report["_duration"],
            "resultFile": report["result"],
        }
        updated[index] = (
            json.dumps(replacement, sort_keys=True, separators=(",", ":")) + "\n"
        ).encode()
    counts["not-run"] -= 6
    counts["passed"] += 6
    require(counts["not-run"] >= 0, "not-run count underflow")
    require(
        all(
            updated[index] == original
            for index, original in enumerate(lines)
            if index not in TARGET_INDEXES
        ),
        f"one of the other 6,520 {mode} rows changed",
    )
    ledger = b"".join(updated)
    summary = {
        "schema": 1,
        "inventory": str(INVENTORY),
        "records": RECORDS,
        "outcomes": dict(sorted(counts.items())),
        "ledgerSha256": sha256(ledger),
        "ledger": str(output),
    }
    return ledger, (json.dumps(summary, indent=2) + "\n").encode(), summary


def prepare(output_dir: Path) -> tuple[dict[str, bytes], dict[str, Any]]:
    require(output_dir.is_absolute(), "successor directory must be absolute")
    originals = base()
    evidence = reports()
    content: dict[str, bytes] = {}
    summaries: dict[str, dict[str, Any]] = {}
    for mode in MODES:
        name = (
            f"playwright-python-{mode}-browser-connect-refused-strict-paired-v1.jsonl"
        )
        ledger, summary_bytes, summary = successor(
            mode, originals[mode], evidence, output_dir / name
        )
        content[name] = ledger
        content[Path(name).with_suffix(".summary.json").name] = summary_bytes
        summaries[mode] = summary
    receipt = {
        "schema": 2,
        "kind": RECEIPT_KIND,
        "scope": "six-unchanged-official-browser-connect-refused-cases-only",
        "fixtureAdaptation": strict.ADAPTATION,
        "sourceClassificationPreserved": "public-api-source",
        "focusedPublicBehaviorEvidence": True,
        "browserServerLaunched": False,
        "remoteBrowserConstructed": False,
        "fullBrowserServerSupport": False,
        "fullPlaywrightServerSupport": False,
        "publicApiParityEvidence": False,
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "recordsPerLane": RECORDS,
        "targetZeroBasedIndexes": list(TARGET_INDEXES),
        "targetTestIds": list(strict.TEST_IDS),
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
                    / f"playwright-python-{mode}-browser-connect-close-strict-paired-v2.jsonl"
                ),
                "ledgerSha256": BASES[mode][0],
                "oldOutcome": "not-run",
            }
            for mode in MODES
        },
        "reports": [
            {
                "mode": mode,
                "api": api,
                "engine": engine,
                "testId": target(api, engine),
                "provenance": str(SIDECARS[(mode, api, engine)]),
                "provenanceSha256": evidence[(mode, api, engine)]["_sidecarSha256"],
                "result": evidence[(mode, api, engine)]["result"],
                "resultSha256": evidence[(mode, api, engine)]["resultSha256"],
                "callReceiptSha256": evidence[(mode, api, engine)]["callReceiptSha256"],
                "outcome": "passed",
                "ledgerRowUpdated": True,
            }
            for mode, api, engine in KEYS
        ],
        "successors": summaries,
    }
    content["promotion-receipt.json"] = (
        json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    ).encode()
    return content, receipt


def _check_output(output_dir: Path) -> None:
    require(output_dir.is_absolute(), "successor directory must be absolute")
    require(
        not output_dir.exists() and not output_dir.is_symlink(),
        f"successor directory already exists or is linked: {output_dir}",
    )
    require(
        output_dir.parent.is_dir() and not output_dir.parent.is_symlink(),
        "successor parent is missing or linked",
    )
    require(
        not any(parent.is_symlink() for parent in output_dir.parents), "symlink parent"
    )


def _fsync_directory(path: Path) -> None:
    descriptor = os.open(path, os.O_RDONLY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def _publish_exclusive(staging: Path, destination: Path) -> None:
    """Publish a whole paired bundle without replacing an existing directory."""
    libc = ctypes.CDLL(None, use_errno=True)
    if sys.platform == "darwin":
        rename = libc.renameatx_np
        source_fd, target_fd, no_replace = -2, -2, 0x4
    elif sys.platform.startswith("linux"):
        rename = libc.renameat2
        source_fd, target_fd, no_replace = -100, -100, 0x1
    else:
        fail("atomic no-replace directory publication is unsupported")
    rename.argtypes = [
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_uint,
    ]
    rename.restype = ctypes.c_int
    if rename(
        source_fd, os.fsencode(staging), target_fd, os.fsencode(destination), no_replace
    ):
        error = ctypes.get_errno()
        raise PromotionError(f"atomic paired publication failed: {os.strerror(error)}")


def publish(output_dir: Path) -> dict[str, Any]:
    _check_output(output_dir)
    content, receipt = prepare(output_dir)
    staging = Path(
        tempfile.mkdtemp(prefix=f".{output_dir.name}.staging-", dir=output_dir.parent)
    )
    try:
        for name, raw in content.items():
            with (staging / name).open("xb") as stream:
                stream.write(raw)
                stream.flush()
                os.fsync(stream.fileno())
        _fsync_directory(staging)
        _check_output(output_dir)
        require(
            prepare(output_dir) == (content, receipt),
            "strict evidence changed during staging",
        )
        _publish_exclusive(staging, output_dir)
    finally:
        if staging.exists():
            shutil.rmtree(staging)
    return receipt


def verify_bundle(output_dir: Path) -> dict[str, Any]:
    require(
        output_dir.is_dir() and not output_dir.is_symlink(),
        "paired bundle is missing or linked",
    )
    content, receipt = prepare(output_dir)
    require(
        {path.name for path in output_dir.iterdir()} == set(content),
        "paired file set differs",
    )
    for name, expected in content.items():
        path = output_dir / name
        require(
            path.is_file() and not path.is_symlink(), f"paired file missing: {name}"
        )
        require(path.read_bytes() == expected, f"paired file content differs: {name}")
    return receipt


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, default=OUTPUT_DIR)
    parser.add_argument("--verify-existing", action="store_true")
    args = parser.parse_args()
    output_dir = args.output_dir.absolute()
    try:
        receipt = (
            verify_bundle(output_dir) if args.verify_existing else publish(output_dir)
        )
    except (
        OSError,
        PromotionError,
        strict.StrictConnectRefusedV1EvidenceError,
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
