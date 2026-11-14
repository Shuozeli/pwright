#!/usr/bin/env python3
"""Publish only three proved post-fix BrowserType.connect close-error rows.

Both lanes require three fresh official, real-fixture sidecars. The candidate
may change failed to passed at the exact three IDs; the already-passed oracle
ledger is copied byte-for-byte so its prior proof remains the ledger source.
This does not establish a working Playwright Server or full API compatibility.
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

import verify_playwright_python_connect_close_strict_v2 as strict

ROOT = Path(__file__).resolve().parents[1]
OUTCOMES = ROOT / ".work/official-conformance/outcomes"
BASE_DIR = OUTCOMES / "python-dummy-engine-post-fix-strict-paired-v2"
OUTPUT_DIR = OUTCOMES / "python-connect-close-post-fix-strict-paired-v2"
RESULT_DIR = ROOT / ".work/official-conformance/results/python-connect-close-post-fix-strict-v2"
INVENTORY = ROOT / ".work/official-conformance/playwright-python-v1.58.0-profiled.jsonl"
INVENTORY_SHA256 = "3dfb11b6ad7eb4736166ee2c4f7fa32cda78063324ad8314ce12f3e2d49b9d41"
BASE_RECEIPT_SHA256 = "009556238f14410c229cad349760bef962183fca33cf82569dc6a5c74d70a5bf"
BASE_PROMOTER_SHA256 = "c3d3becc1c7e6a29b70d039d7b48b84f3b65e51a96f4dde5024601d4ccc11318"
BASES = {
    "candidate": (
        "5c88ae0e2cad7a0b5c10228e1435d220fb6263311b5d85cbd0f4c323e7eb32a3",
        "db47e94f9dde5d2897a69e37cceef53ceb34b4d88ed9adddf8fb3dbf6f56cd92",
    ),
    "oracle": (
        "7d044a471c9ee530de390d676d50478d28c3df43a4f106b60528809c7a114ec7",
        "e0f357b193761aa5f3dee506962206d29b0c30834c21bd06b90501fa4c3d32cb",
    ),
}
MODES = ("candidate", "oracle")
ENGINES = ("chromium", "firefox", "webkit")
KEYS = tuple((mode, engine) for mode in MODES for engine in ENGINES)
RECORDS = 6_526
TARGET_INDEXES = (844, 845, 846)
GENERATED = frozenset({"attempts", "durationSeconds", "error", "outcome", "resultFile"})
RECEIPT_KIND = "playwright-python-browser-connect-close-post-fix-strict-paired-promotion-v2"
SIDECARS = {key: RESULT_DIR / f"{key[0]}-{key[1]}-v1.provenance.json" for key in KEYS}

POLICY_SHA256 = "2d96647c6d06f6ce4db7ff043c6d8cd1561c0486d1d5414b88890c9695699a2c"
CANDIDATE_WHEEL_SHA256 = "2ad72324788ed7ae24eeb6f265e035f64a654e75ebc342ddb2537730f659bc1d"
TOOL_SHA256 = {
    "scripts/run_playwright_python_connect_close_strict_v2.py": (
        "ddbb66d36b145de668e7bd6a669f9d4399b68eb691f6385f94e499ad0411a2ca"
    ),
    "scripts/verify_playwright_python_connect_close_strict_v2.py": (
        "a138a0f0d17e64b942d42fdcfc03de3af4dafd96ec4cca68149ed854d7e548c4"
    ),
    (
        "packages/pwright-python/tests/official/ws_connect_guard_strict_v2/"
        "pwright_official_connect_guard_v2.py"
    ): "f1cc02ce09be1688d2a5882afbc36ee22987871a517cab134a52e55374083d66",
}
SIDECAR_SHA256 = {
    ("candidate", "chromium"): "ea75815df15aec7ca4e0c32caf436fa1a8ec99a787f97633868be3a00f9f2c48",
    ("candidate", "firefox"): "5fe3fd2b8ae0f7d1e0d394e225a4bba44844023a656c10f0ea85c3124946b99b",
    ("candidate", "webkit"): "dc803b38a6094aede20354d816e6ab60b4a5dfbaaa3ab417b64990434fab2ec6",
    ("oracle", "chromium"): "13123518108345a3c3404817c322a6fcc6f53765aeb3b3597dc0a0d92c989f0f",
    ("oracle", "firefox"): "aa0c8ce695692ba61fef185432dbdea8b23c8486077c2b22bb67d058f436f7b8",
    ("oracle", "webkit"): "f60591b3e686c4ae1a4c48f7cf9411df428be247a19becc29f0bdae15d90f4e8",
}


class PromotionError(ValueError):
    """The six official results do not justify a paired successor."""


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
    require(isinstance(value, dict), f"{label} is not an object")
    return value


def jsonl(raw: bytes, label: str) -> tuple[list[bytes], list[dict[str, Any]]]:
    require(raw.endswith(b"\n"), f"{label} is not newline terminated")
    lines = raw.splitlines(keepends=True)
    require(len(lines) == RECORDS, f"{label} is not the full {RECORDS}-row corpus")
    return lines, [json_object(line, f"{label} row {index}") for index, line in enumerate(lines)]


def target(engine: str) -> str:
    return strict.TEST_IDS[ENGINES.index(engine)]


def base() -> dict[str, tuple[list[bytes], list[dict[str, Any]]]]:
    """Read and validate the immutable dummy-engine post-fix paired baseline."""
    sealed(
        ROOT / "scripts/promote_playwright_python_dummy_engine_strict_v2.py",
        BASE_PROMOTER_SHA256,
        "base promoter source",
    )
    _, inventory = jsonl(sealed(INVENTORY, INVENTORY_SHA256, "Python inventory"), "inventory")
    ids = [row.get("id") for row in inventory]
    require(all(isinstance(node_id, str) for node_id in ids), "inventory ID is malformed")
    require(len(set(ids)) == RECORDS, "inventory has duplicate IDs")
    require(
        tuple(ids[index] for index in TARGET_INDEXES) == strict.TEST_IDS,
        "three BrowserType.connect target IDs moved",
    )
    receipt = json_object(
        sealed(BASE_DIR / "promotion-receipt.json", BASE_RECEIPT_SHA256, "base paired receipt"),
        "base paired receipt",
    )
    require(
        receipt.get("kind")
        == "playwright-python-nonchromium-dummy-node-post-fix-strict-paired-promotion-v2"
        and receipt.get("inventory") == str(INVENTORY)
        and receipt.get("inventorySha256") == INVENTORY_SHA256
        and receipt.get("promoterSha256") == BASE_PROMOTER_SHA256
        and receipt.get("commit") == strict.COMMIT
        and receipt.get("recordsPerLane") == RECORDS
        and receipt.get("publicApiParityEvidence") is False,
        "base paired receipt identity differs",
    )
    result: dict[str, tuple[list[bytes], list[dict[str, Any]]]] = {}
    for mode in MODES:
        path = BASE_DIR / f"playwright-python-{mode}-dummy-engine-launch-strict-paired-v2.jsonl"
        ledger_sha, summary_sha = BASES[mode]
        lines, rows = jsonl(sealed(path, ledger_sha, f"{mode} base ledger"), f"{mode} base ledger")
        summary = json_object(
            sealed(path.with_suffix(".summary.json"), summary_sha, f"{mode} base summary"),
            f"{mode} base summary",
        )
        require(
            receipt.get("successors", {}).get(mode) == summary
            and summary
            == {
                "schema": 1,
                "inventory": str(INVENTORY),
                "records": RECORDS,
                "outcomes": dict(sorted(Counter(row.get("outcome") for row in rows).items())),
                "ledgerSha256": ledger_sha,
                "ledger": str(path),
            },
            f"{mode} base summary differs from pinned ledger",
        )
        for index, (row, inventory_row) in enumerate(zip(rows, inventory)):
            require(row.get("id") == ids[index], f"{mode} row order differs at {index}")
            require(
                {key: value for key, value in row.items() if key not in GENERATED} == inventory_row,
                f"{mode} metadata differs from inventory at {index}",
            )
            if index in TARGET_INDEXES:
                require(
                    row.get("sourceClassification") == "public-api-source"
                    and row.get("attempts") == 1
                    and row.get("outcome") == ("failed" if mode == "candidate" else "passed")
                    and ("error" in row) == (mode == "candidate"),
                    f"{mode} target is not the pinned red/oracle baseline",
                )
                if mode == "candidate":
                    require(
                        row.get("error")
                        == "AttributeError: 'BrowserType' object has no attribute 'connect'",
                        "candidate red predecessor error differs",
                    )
        result[mode] = lines, rows
    return result


def tools_ready() -> None:
    expected_sidecars = {key: RESULT_DIR / f"{key[0]}-{key[1]}-v1.provenance.json" for key in KEYS}
    require(
        SIDECARS == expected_sidecars and len(set(SIDECARS.values())) == 6,
        "six sidecar slots differ",
    )
    digest_pin(POLICY_SHA256, "v2 policy")
    require(POLICY_SHA256 == strict.POLICY_SHA256, "v2 policy differs from verifier pin")
    sealed(strict.POLICY, POLICY_SHA256, "v2 strict policy")
    policy = strict._policy(json.loads(strict.POLICY.read_text()))
    digest_pin(CANDIDATE_WHEEL_SHA256, "v2 candidate wheel")
    require(CANDIDATE_WHEEL_SHA256 == policy["candidateWheelSha256"], "candidate wheel pin differs")
    expected_tools = set(strict._CAPTURE_PATHS) | {
        "scripts/verify_playwright_python_connect_close_strict_v2.py"
    }
    require(set(TOOL_SHA256) == expected_tools, "v2 capture/verifier tool set differs")
    for relative, expected in TOOL_SHA256.items():
        sealed(ROOT / relative, expected, f"v2 tool {relative}")
    require(set(SIDECAR_SHA256) == set(KEYS), "six v2 sidecar pins are incomplete")
    for key, digest in SIDECAR_SHA256.items():
        digest_pin(digest, f"v2 sidecar {key}")


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
        and case.get("classname") == "tests.async.test_browsertype_connect"
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
        "v2 official assertion did not pass once",
    )
    try:
        duration = float(case.get("time", ""))
    except ValueError as error:
        raise PromotionError("v2 JUnit duration is invalid") from error
    require(math.isfinite(duration) and duration >= 0, "v2 JUnit duration is invalid")
    return duration


def reports() -> dict[tuple[str, str], dict[str, Any]]:
    tools_ready()
    verified: dict[tuple[str, str], dict[str, Any]] = {}
    for key in KEYS:
        mode, engine = key
        path = SIDECARS[key]
        before = sealed(path, SIDECAR_SHA256[key], f"v2 {key} sidecar")
        payload = strict.verify_provenance(path)
        require(path.read_bytes() == before, f"v2 {key} sidecar changed during verification")
        require(
            payload.get("schema") == 2
            and payload.get("kind") == strict.KIND
            and payload.get("evidenceScope") == strict.SCOPE
            and (payload.get("mode"), payload.get("testId")) == (mode, target(engine))
            and payload.get("manifest") == str(strict.POLICY)
            and payload.get("manifestSha256") == POLICY_SHA256
            and payload.get("commit") == strict.COMMIT
            and payload.get("wheelSha256") == CANDIDATE_WHEEL_SHA256
            and payload.get("publicConnectPresent") is True
            and payload.get("fixtureTransportAdaptation") == strict.ADAPTATION
            and payload.get("outcome") == "passed"
            and payload.get("exitCode") == 0
            and payload.get("verifierSha256")
            == TOOL_SHA256["scripts/verify_playwright_python_connect_close_strict_v2.py"],
            f"v2 {key} is not the exact passed official outcome",
        )
        require(
            path == Path(payload["result"]).with_suffix(".provenance.json"),
            f"v2 {key} sidecar is not its JUnit sibling",
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
        "fixtureTransportAdaptation",
        "verifierSha256",
    )
    for key, report in verified.items():
        require(
            all(report.get(field) == reference.get(field) for field in common),
            f"v2 {key} source, fixture, or artifact identity differs across lanes",
        )
    for mode in MODES:
        lane = [verified[(mode, engine)] for engine in ENGINES]
        fields = ("pythonExecutable", "playwrightPackage", "driverNode")
        if mode == "candidate":
            fields += ("pwrightPackageSha256", "nativeSha256", "runtimeLibraries")
        for field in fields:
            require(
                all(report.get(field) == lane[0].get(field) for report in lane),
                f"v2 {mode} {field} differs across engines",
            )
    for field in ("result", "callReceipt", "runNonce"):
        require(
            len({report.get(field) for report in verified.values()}) == 6,
            f"v2 {field} was reused across six independent executions",
        )
    return verified


def successor(
    mode: str,
    originals: tuple[list[bytes], list[dict[str, Any]]],
    evidence: dict[tuple[str, str], dict[str, Any]],
    output: Path,
) -> tuple[bytes, bytes, dict[str, Any]]:
    lines, rows = originals
    require(len(lines) == len(rows) == RECORDS, "successor base length differs")
    updated = list(lines)
    counts = Counter(row["outcome"] for row in rows)
    if mode == "candidate":
        for engine, index in zip(ENGINES, TARGET_INDEXES):
            row = rows[index]
            report = evidence[(mode, engine)]
            require(
                row["id"] == target(engine) and row["outcome"] == "failed", "candidate target moved"
            )
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
        counts["failed"] -= 3
        counts["passed"] += 3
        require(counts["failed"] >= 0, "candidate failed-count underflow")
        require(
            all(
                updated[index] == original
                for index, original in enumerate(lines)
                if index not in TARGET_INDEXES
            ),
            "one of the other 6,523 candidate rows changed",
        )
    elif mode == "oracle":
        require(
            all(
                rows[index]["id"] == target(engine) and rows[index]["outcome"] == "passed"
                for engine, index in zip(ENGINES, TARGET_INDEXES)
            ),
            "oracle target proof differs",
        )
        require(updated == lines, "oracle prior proof must remain byte-identical")
    else:
        fail("unknown lane")
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
        name = f"playwright-python-{mode}-browser-connect-close-strict-paired-v2.jsonl"
        ledger, summary_bytes, summary = successor(
            mode, originals[mode], evidence, output_dir / name
        )
        content[name] = ledger
        content[Path(name).with_suffix(".summary.json").name] = summary_bytes
        summaries[mode] = summary
    receipt = {
        "schema": 2,
        "kind": RECEIPT_KIND,
        "scope": "three-unchanged-official-browser-connect-early-ws-close-cases-only",
        "sourceClassificationPreserved": "public-api-source",
        "focusedPublicBehaviorEvidence": True,
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
                    / f"playwright-python-{mode}-dummy-engine-launch-strict-paired-v2.jsonl"
                ),
                "ledgerSha256": BASES[mode][0],
                "oldOutcome": "failed" if mode == "candidate" else "passed",
            }
            for mode in MODES
        },
        "reports": [
            {
                "mode": mode,
                "engine": engine,
                "testId": target(engine),
                "provenance": str(SIDECARS[(mode, engine)]),
                "provenanceSha256": evidence[(mode, engine)]["_sidecarSha256"],
                "result": evidence[(mode, engine)]["result"],
                "resultSha256": evidence[(mode, engine)]["resultSha256"],
                "callReceiptSha256": evidence[(mode, engine)]["callReceiptSha256"],
                "outcome": "passed",
                "ledgerRowUpdated": mode == "candidate",
            }
            for mode, engine in KEYS
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
    require(not any(parent.is_symlink() for parent in output_dir.parents), "symlink parent")


def _fsync_directory(path: Path) -> None:
    descriptor = os.open(path, os.O_RDONLY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def _publish_exclusive(staging: Path, destination: Path) -> None:
    """Atomically publish the whole bundle without replacing an existing one."""
    libc = ctypes.CDLL(None, use_errno=True)
    if sys.platform == "darwin":
        rename = libc.renameatx_np
        source_fd, target_fd, no_replace = -2, -2, 0x4
    elif sys.platform.startswith("linux"):
        rename = libc.renameat2
        source_fd, target_fd, no_replace = -100, -100, 0x1
    else:
        fail("atomic no-replace directory publication is unsupported")
    rename.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p, ctypes.c_uint]
    rename.restype = ctypes.c_int
    if rename(source_fd, os.fsencode(staging), target_fd, os.fsencode(destination), no_replace):
        error = ctypes.get_errno()
        raise PromotionError(f"atomic paired publication failed: {os.strerror(error)}")


def publish(output_dir: Path) -> dict[str, Any]:
    _check_output(output_dir)
    content, receipt = prepare(output_dir)
    staging = Path(tempfile.mkdtemp(prefix=f".{output_dir.name}.staging-", dir=output_dir.parent))
    try:
        for name, raw in content.items():
            with (staging / name).open("xb") as stream:
                stream.write(raw)
                stream.flush()
                os.fsync(stream.fileno())
        _fsync_directory(staging)
        _check_output(output_dir)
        require(prepare(output_dir) == (content, receipt), "strict evidence changed during staging")
        _publish_exclusive(staging, output_dir)
    finally:
        if staging.exists():
            shutil.rmtree(staging)
    return receipt


def verify_bundle(output_dir: Path) -> dict[str, Any]:
    require(
        output_dir.is_dir() and not output_dir.is_symlink(), "paired bundle is missing or linked"
    )
    content, receipt = prepare(output_dir)
    require(
        {path.name for path in output_dir.iterdir()} == set(content),
        "paired bundle file set differs",
    )
    for name, expected in content.items():
        path = output_dir / name
        require(path.is_file() and not path.is_symlink(), f"paired bundle file missing: {name}")
        require(path.read_bytes() == expected, f"paired bundle file content differs: {name}")
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
        strict.StrictWebSocketEvidenceError,
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
