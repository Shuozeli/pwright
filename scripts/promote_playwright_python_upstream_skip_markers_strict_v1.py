"""Promote only 15 paired, pre-body upstream Python marker skips.

The predecessor's 6,526-row ledgers are immutable. This successor awards no
public-behavior pass or parity credit: only not-run -> skipped is permitted.
"""

from __future__ import annotations

import argparse
import json
import math
import os
import shutil
import subprocess
import tempfile
import xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path
from typing import Any

import promote_playwright_python_connect_refused_strict_v1 as prior
import verify_playwright_python_upstream_skip_markers_strict_v1 as strict

ROOT = Path(__file__).resolve().parents[1]
OUTCOMES = ROOT / ".work/official-conformance/outcomes"
BASE_DIR = OUTCOMES / "python-connect-refused-strict-paired-v1"
OUTPUT_DIR = OUTCOMES / "python-upstream-skip-markers-strict-paired-v1"
RESULT_DIR = (
    ROOT
    / ".work/official-conformance/results/python-upstream-skip-markers-strict-v1-final-v2"
)
INVENTORY = ROOT / ".work/official-conformance/playwright-python-v1.58.0-profiled.jsonl"
INVENTORY_SHA256 = "3dfb11b6ad7eb4736166ee2c4f7fa32cda78063324ad8314ce12f3e2d49b9d41"
BASE_PROMOTER_SHA256 = (
    "b890ec1d02dbf50311fb00bd5de66ef85cd0f2ddf1acd9cc966ca795341c500a"
)
BASE_RECEIPT_SHA256 = "4979cdc982cf716e5124bf00b972594c27a65fc26104c193ed12957cf5d4c060"
BASES = {
    "candidate": (
        "3095771a0928ecc076ecdec8eee5d2946d70de9dbd1ea15e00f61ad7fa79e836",
        "611e0e50464bad8d4c0835611a4d98c307daa9a8c70cc6800adf1054ac9d382d",
    ),
    "oracle": (
        "11afbc5f482a3d9cefa36c59598f1771903b74f93749dcd11e8b5b178a7a6208",
        "bc4f4df20a2b9fc9ba89fd85c16ee3b52a7b9f29f7df780e2df88ffa8e64292a",
    ),
}
MODES = ("candidate", "oracle")
ORDINALS = tuple(range(15))
KEYS = tuple((mode, ordinal) for mode in MODES for ordinal in ORDINALS)
TARGET_INDEXES = (
    860,
    861,
    863,
    864,
    866,
    867,
    869,
    870,
    872,
    873,
    4994,
    4995,
    2234,
    2235,
    2154,
)
RECORDS = 6_526
GENERATED = frozenset({"attempts", "durationSeconds", "error", "outcome", "resultFile"})
RECEIPT_KIND = "playwright-python-upstream-skip-markers-strict-paired-promotion-v1"
POLICY_SHA256 = "bb0c996863aa410ff784ad5040c3d3809ee06f18ae033f63deaf7026efe3067e"
TOOL_SHA256 = {
    "scripts/run_playwright_python_upstream_skip_markers_strict_v1.py": (
        "61027b05720ffee373ea17b26b6c7d696ea010c2f8dffee0d511c2c36b9bf550"
    ),
    "scripts/verify_playwright_python_upstream_skip_markers_strict_v1.py": (
        "168b3801f88248f11d0d628521392ce6a05615cede547a2e708492a20391feda"
    ),
    (
        "packages/pwright-python/tests/official/upstream_skip_guard_strict_v1/"
        "pwright_official_upstream_skip_guard_v1.py"
    ): "9e4b8cbddd267db02bba68b3c139abf2422c55b062deef284f98d103f63ee448",
}
SIDECARS = {key: RESULT_DIR / f"{key[0]}-{key[1]:03d}.provenance.json" for key in KEYS}
SIDECAR_SHA256: dict[tuple[str, int], str] = {
    (
        "candidate",
        0,
    ): "25a1b7cd80e5588951fdc27d87400983a3f0f0566b89ea924843240d97b4ae45",
    (
        "candidate",
        1,
    ): "0ce9f53d5a53df1a7b3e0e7b05617bfccd542768f66de4cd76ed8a4c617690d6",
    (
        "candidate",
        2,
    ): "ec69247f55928b4f2b5d2f0f9abc909ce6f89c3b0dfa1c6f269ac41331521c1e",
    (
        "candidate",
        3,
    ): "f0f51a7e14c3677f0d632a5b103b547a42b51df3a1d83546d4588baed415d0b4",
    (
        "candidate",
        4,
    ): "9e30e1bcef56eee56ed8c308d4750ebfafed47decc72aa3c6090fe11d6e4e06e",
    (
        "candidate",
        5,
    ): "a4a16ff2c513963063d2915bf93eead512fb4f6847a312b7d43ffc4806364c4d",
    (
        "candidate",
        6,
    ): "58c04edcba36aa8b723d40055722d2682f2a7e0f40f343e1e8318ad0481b6cc0",
    (
        "candidate",
        7,
    ): "8bf8bebc9d09319401a27e10a26621bfc3df6f3425190494fffa04d8d380f7b2",
    (
        "candidate",
        8,
    ): "cd6126fea8c582211954a23214817d65aec3fb2121b72a579fce0080b2a5e06b",
    (
        "candidate",
        9,
    ): "4933b88f6a98a37a34898c533b7cbbc320eef0181a600476ce045486de228d77",
    (
        "candidate",
        10,
    ): "3764bdcd21ac6fd95592b4309486ccc205bcd204fec2628182ea7bda891b5979",
    (
        "candidate",
        11,
    ): "a1734c80f08725aef70f610cb926742cc14b76d16d3ddf51b121aa35b5bd10e7",
    (
        "candidate",
        12,
    ): "7523766ffe00b53ac90e5f2d912a5b8943cd3d410d07065d125286265568ce86",
    (
        "candidate",
        13,
    ): "eafa86fa6178b72daa474cda329fad722cbc415b076abd8e632474e33e930314",
    (
        "candidate",
        14,
    ): "b955ecc27bf2874004c97d7c798fbcc00c9796203641e512bd466fc420e985b6",
    ("oracle", 0): "c0b6c9efa1f5230b29615c0b9dddd045d3fcc5838a875edfaf6a478a1f2fd05a",
    ("oracle", 1): "a51a61fdf7ee44b4c410c991b326d26ce4f85a6f2244a94653f3021cce3b87b2",
    ("oracle", 2): "af0fdb959b5c943e48a2b660a6804f43cdb3be5ae964c8e4ab0ac50eff8c9204",
    ("oracle", 3): "2c9e6aebd9d18c0589192b80e67bbcb283643664b49e95f71384c2941e23e9d8",
    ("oracle", 4): "3453502edc00efbd77c2949f428b5eacbede9fbe0273019b2ad29775f11f99a7",
    ("oracle", 5): "2c750389d1c0c47f72ac717541e81749e26c4defe80fa1ac80e48e2a3c9c5e90",
    ("oracle", 6): "144b92503e1d833517c864d04cfa71ddebfdc7a83325184058e7a520e62edf51",
    ("oracle", 7): "b85eca84d916f7bb71deaa922da3bea2901536f42dc6aeb1f76dc9c81c0ed495",
    ("oracle", 8): "e61a71c3c3c3a6427f41b3f03036705790425b1e328fbf58b5f98ca707948dfd",
    ("oracle", 9): "6121203d824938e92fcfbe09dd72d930752c10dd5659040238a6be4c8bcbc5fc",
    ("oracle", 10): "5f1064b50d54a4c5bb21aff648fb34401f03db4a2c3cf1dfa816e6f0ef8c9858",
    ("oracle", 11): "bbc886b5a38af050c2a86c578cf15510e1bc1ebca565931f4cf13c4b2924adbc",
    ("oracle", 12): "483538d186455ce3447938214a674cbf8c417475ee7b2b635184dd9f9d2dc01a",
    ("oracle", 13): "91ec5c1478f17075df8194f81f07c4bd5bd8b472ef879a2b76cc865606692823",
    ("oracle", 14): "13f619cc09bff113045da046744153fd6edba1ddcc8650528399edfcc0ceb17b",
}

PromotionError = prior.PromotionError
require = prior.require
sha256 = prior.sha256
sealed = prior.sealed
json_object = prior.json_object
jsonl = prior.jsonl
digest_pin = prior.digest_pin


def base_name(mode: str) -> str:
    return f"playwright-python-{mode}-browser-connect-refused-strict-paired-v1.jsonl"


def output_name(mode: str) -> str:
    return f"playwright-python-{mode}-upstream-skip-markers-strict-paired-v1.jsonl"


def target(ordinal: int) -> str:
    return strict.TEST_IDS[ordinal]


def base() -> dict[str, tuple[list[bytes], list[dict[str, Any]]]]:
    """Verify the entire predecessor bundle and pin every input byte."""
    prior_source = (
        ROOT / "scripts/promote_playwright_python_connect_refused_strict_v1.py"
    )
    require(
        Path(prior.__file__).resolve() == prior_source,
        "predecessor module path differs",
    )
    sealed(prior_source, BASE_PROMOTER_SHA256, "base promoter source")
    predecessor = prior.verify_bundle(BASE_DIR)
    receipt = json_object(
        sealed(
            BASE_DIR / "promotion-receipt.json", BASE_RECEIPT_SHA256, "base receipt"
        ),
        "base receipt",
    )
    require(
        predecessor == receipt, "predecessor verification returned a different receipt"
    )
    require(
        receipt.get("kind") == prior.RECEIPT_KIND
        and receipt.get("inventory") == str(INVENTORY)
        and receipt.get("inventorySha256") == INVENTORY_SHA256
        and receipt.get("recordsPerLane") == RECORDS
        and receipt.get("commit") == strict.COMMIT
        and receipt.get("promoterSha256") == BASE_PROMOTER_SHA256
        and receipt.get("publicApiParityEvidence") is False,
        "base receipt identity or scope differs",
    )
    _, inventory = jsonl(
        sealed(INVENTORY, INVENTORY_SHA256, "Python inventory"), "inventory"
    )
    ids = [row.get("id") for row in inventory]
    require(len(ids) == RECORDS and len(set(ids)) == RECORDS, "inventory IDs differ")
    require(
        len(TARGET_INDEXES) == len(set(TARGET_INDEXES)) == len(strict.TEST_IDS) == 15
        and tuple(ids[index] for index in TARGET_INDEXES) == strict.TEST_IDS,
        "15 skip-only target IDs moved",
    )
    originals: dict[str, tuple[list[bytes], list[dict[str, Any]]]] = {}
    for mode in MODES:
        ledger_path = BASE_DIR / base_name(mode)
        ledger_sha, summary_sha = BASES[mode]
        lines, rows = jsonl(
            sealed(ledger_path, ledger_sha, f"{mode} base ledger"),
            f"{mode} base ledger",
        )
        summary = json_object(
            sealed(
                ledger_path.with_suffix(".summary.json"),
                summary_sha,
                f"{mode} base summary",
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
                "ledger": str(ledger_path),
            },
            f"{mode} base summary differs",
        )
        for index, (row, source) in enumerate(zip(rows, inventory)):
            require(row.get("id") == ids[index], f"{mode} row order differs at {index}")
            require(
                {key: value for key, value in row.items() if key not in GENERATED}
                == source,
                f"{mode} source metadata differs at {index}",
            )
            if index in TARGET_INDEXES:
                require(
                    row == {**source, "attempts": 0, "outcome": "not-run"}
                    and source.get("sourceClassification") == "public-api-source",
                    f"{mode} target is not an untouched public-source not-run row",
                )
        originals[mode] = lines, rows
    return originals


def tools_ready() -> None:
    require(
        strict.POLICY
        == ROOT
        / "packages/pwright-python/tests/official/upstream-skip-markers-strict-v1-manifest.json",
        "policy path differs",
    )
    sealed(strict.POLICY, POLICY_SHA256, "skip policy")
    policy = strict.load_policy()
    require(policy["tests"] == list(strict.TEST_IDS), "policy targets differ")
    expected_tools = {
        str(strict.RUNNER.relative_to(ROOT)),
        str(Path(strict.__file__).relative_to(ROOT)),
        str(
            (strict.GUARD / "pwright_official_upstream_skip_guard_v1.py").relative_to(
                ROOT
            )
        ),
    }
    require(set(TOOL_SHA256) == expected_tools, "skip tool set differs")
    for relative, expected in TOOL_SHA256.items():
        sealed(ROOT / relative, expected, f"skip tool {relative}")
    require(
        set(SIDECARS) == set(KEYS) and set(SIDECAR_SHA256) == set(KEYS),
        "30 final-v2 sidecar pins are incomplete",
    )
    require(len(set(SIDECARS.values())) == 30, "sidecar slots are not unique")
    for key in KEYS:
        require(
            SIDECARS[key] == RESULT_DIR / f"{key[0]}-{key[1]:03d}.provenance.json",
            f"sidecar slot moved: {key}",
        )
        digest_pin(SIDECAR_SHA256[key], f"final-v2 sidecar {key}")
    require(
        RESULT_DIR.is_dir() and not RESULT_DIR.is_symlink(),
        "final-v2 evidence directory is absent or linked",
    )
    expected_names = {
        f"{mode}-{ordinal:03d}{suffix}"
        for mode, ordinal in KEYS
        for suffix in (".provenance.json", ".call.jsonl", ".xml")
    }
    require(
        {path.name for path in RESULT_DIR.iterdir()} == expected_names,
        "final-v2 evidence directory does not contain exactly 90 artifacts",
    )


def report_duration(payload: dict[str, Any]) -> tuple[float, str]:
    result = Path(payload["result"])
    raw = sealed(result, payload["resultSha256"], "final-v2 JUnit")
    try:
        root = ET.fromstring(raw)
        cases = list(root.iter("testcase"))
        require(len(cases) == 1, "skip JUnit case count differs")
        duration = float(cases[0].get("time", ""))
    except (ET.ParseError, ValueError) as error:
        raise PromotionError("skip JUnit duration or XML is invalid") from error
    require(math.isfinite(duration) and duration >= 0, "skip JUnit duration is invalid")
    reason = cases[0].find("skipped")
    require(reason is not None, "skip JUnit lacks marker reason")
    message = reason.get("message")
    engine = payload["testId"].rsplit("[", 1)[1].removesuffix("]")
    require(
        message == f"skipped for this browser: {engine}", "skip JUnit reason differs"
    )
    return duration, message


def reports() -> dict[tuple[str, int], dict[str, Any]]:
    """Verify each paired skip and the 30 immutable final-v2 sidecar pins."""
    tools_ready()
    verified: dict[tuple[str, int], dict[str, Any]] = {}
    for ordinal in ORDINALS:
        candidate_path = SIDECARS[("candidate", ordinal)]
        oracle_path = SIDECARS[("oracle", ordinal)]
        candidate_raw = sealed(
            candidate_path, SIDECAR_SHA256[("candidate", ordinal)], "candidate sidecar"
        )
        oracle_raw = sealed(
            oracle_path, SIDECAR_SHA256[("oracle", ordinal)], "oracle sidecar"
        )
        candidate, oracle = strict.verify_pair(candidate_path, oracle_path)
        for mode, path, raw, payload in (
            ("candidate", candidate_path, candidate_raw, candidate),
            ("oracle", oracle_path, oracle_raw, oracle),
        ):
            require(
                path.read_bytes() == raw, "sidecar changed during paired verification"
            )
            expected_result = RESULT_DIR / f"{mode}-{ordinal:03d}.xml"
            require(
                payload.get("mode") == mode
                and payload.get("testId") == target(ordinal)
                and payload.get("kind") == strict.KIND
                and payload.get("evidenceScope") == strict.SCOPE
                and payload.get("manifestSha256") == POLICY_SHA256
                and payload.get("commit") == strict.COMMIT
                and payload.get("outcome") == "skipped"
                and type(payload.get("exitCode")) is int
                and payload["exitCode"] == 0
                and payload.get("runnerSha256")
                == TOOL_SHA256[str(strict.RUNNER.relative_to(ROOT))]
                and payload.get("verifierSha256")
                == TOOL_SHA256[str(Path(strict.__file__).relative_to(ROOT))]
                and payload.get("guardSha256")
                == TOOL_SHA256[
                    str(
                        (
                            strict.GUARD / "pwright_official_upstream_skip_guard_v1.py"
                        ).relative_to(ROOT)
                    )
                ]
                and payload.get("result") == str(expected_result)
                and payload.get("callReceipt")
                == str(expected_result.with_suffix(".call.jsonl")),
                f"final-v2 {mode} case {ordinal} identity differs",
            )
            sealed(
                Path(payload["callReceipt"]),
                payload["callReceiptSha256"],
                "skip guard receipt",
            )
            duration, reason = report_duration(payload)
            report = dict(payload)
            report["_duration"] = duration
            report["_reason"] = reason
            report["_sidecarSha256"] = sha256(raw)
            verified[(mode, ordinal)] = report
    reference = verified[KEYS[0]]
    for key, report in verified.items():
        require(
            all(
                report[field] == reference[field]
                for field in (
                    "upstream",
                    "commit",
                    "manifest",
                    "manifestSha256",
                    "fixtureBind",
                    "fixtureHost",
                    "runnerSha256",
                    "verifierSha256",
                    "guardSha256",
                )
            ),
            f"final-v2 shared identity differs at {key}",
        )
    for mode in MODES:
        lane = [verified[(mode, ordinal)] for ordinal in ORDINALS]
        require(
            all(report["artifact"] == lane[0]["artifact"] for report in lane),
            f"final-v2 {mode} runtime identity differs",
        )
    for field in ("result", "callReceipt", "runNonce"):
        require(
            len({report[field] for report in verified.values()}) == 30,
            f"final-v2 {field} was reused across independent runs",
        )
    return verified


def successor(
    mode: str,
    originals: tuple[list[bytes], list[dict[str, Any]]],
    evidence: dict[tuple[str, int], dict[str, Any]],
    output: Path,
) -> tuple[bytes, bytes, dict[str, Any]]:
    lines, rows = originals
    require(
        mode in MODES and len(lines) == len(rows) == RECORDS,
        "successor baseline differs",
    )
    updated = list(lines)
    counts = Counter(row["outcome"] for row in rows)
    for ordinal, index in enumerate(TARGET_INDEXES):
        row = rows[index]
        report = evidence[(mode, ordinal)]
        require(
            row.get("id") == target(ordinal)
            and row.get("sourceClassification") == "public-api-source"
            and row.get("outcome") == "not-run"
            and row.get("attempts") == 0
            and not any(
                field in row for field in ("error", "durationSeconds", "resultFile")
            ),
            f"{mode} target {ordinal} moved",
        )
        require(
            report.get("mode") == mode
            and report.get("testId") == target(ordinal)
            and report.get("outcome") == "skipped"
            and report.get("_reason")
            == f"skipped for this browser: {target(ordinal).rsplit('[', 1)[1].removesuffix(']')}"
            and isinstance(report.get("_duration"), (float, int))
            and math.isfinite(report["_duration"])
            and report["_duration"] >= 0,
            f"{mode} skip report {ordinal} differs",
        )
        replacement = {
            **row,
            "attempts": 1,
            "durationSeconds": report["_duration"],
            "error": report["_reason"],
            "outcome": "skipped",
            "resultFile": report["result"],
        }
        updated[index] = (
            json.dumps(replacement, sort_keys=True, separators=(",", ":")) + "\n"
        ).encode()
    counts["not-run"] -= 15
    counts["skipped"] += 15
    require(counts["not-run"] >= 0, "not-run count underflow")
    require(
        all(
            updated[index] == original
            for index, original in enumerate(lines)
            if index not in TARGET_INDEXES
        ),
        f"one of the other 6,511 {mode} row bytes changed",
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
    require(
        output_dir == OUTPUT_DIR and output_dir.is_absolute(), "successor path differs"
    )
    originals = base()
    evidence = reports()
    content: dict[str, bytes] = {}
    summaries: dict[str, dict[str, Any]] = {}
    for mode in MODES:
        name = output_name(mode)
        ledger, summary_bytes, summary = successor(
            mode, originals[mode], evidence, output_dir / name
        )
        content[name] = ledger
        content[Path(name).with_suffix(".summary.json").name] = summary_bytes
        summaries[mode] = summary
    receipt = {
        "schema": 2,
        "kind": RECEIPT_KIND,
        "scope": "15-upstream-pre-body-marker-skips-only",
        "evidenceScope": strict.SCOPE,
        "sourceClassificationPreserved": "public-api-source",
        "newPublicBehaviorPasses": 0,
        "newPublicBehaviorParityCredit": 0,
        "focusedPublicBehaviorEvidenceThisPromotion": False,
        "publicApiParityEvidence": False,
        "browserServerLaunched": False,
        "remoteBrowserConstructed": False,
        "fullBrowserServerSupport": False,
        "fullPlaywrightServerSupport": False,
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "recordsPerLane": RECORDS,
        "targetZeroBasedIndexes": list(TARGET_INDEXES),
        "targetTestIds": list(strict.TEST_IDS),
        "manifest": str(strict.POLICY),
        "manifestSha256": POLICY_SHA256,
        "commit": strict.COMMIT,
        "promoterSha256": sha256(Path(__file__).read_bytes()),
        "toolSha256": TOOL_SHA256,
        "baseReceipt": str(BASE_DIR / "promotion-receipt.json"),
        "baseReceiptSha256": BASE_RECEIPT_SHA256,
        "basePromoterSha256": BASE_PROMOTER_SHA256,
        "bases": {
            mode: {
                "ledger": str(BASE_DIR / base_name(mode)),
                "ledgerSha256": BASES[mode][0],
                "summary": str(
                    (BASE_DIR / base_name(mode)).with_suffix(".summary.json")
                ),
                "summarySha256": BASES[mode][1],
                "oldOutcome": "not-run",
            }
            for mode in MODES
        },
        "reports": [
            {
                "mode": mode,
                "ordinal": ordinal,
                "testId": target(ordinal),
                "provenance": str(SIDECARS[(mode, ordinal)]),
                "provenanceSha256": evidence[(mode, ordinal)]["_sidecarSha256"],
                "result": evidence[(mode, ordinal)]["result"],
                "resultSha256": evidence[(mode, ordinal)]["resultSha256"],
                "callReceipt": evidence[(mode, ordinal)]["callReceipt"],
                "callReceiptSha256": evidence[(mode, ordinal)]["callReceiptSha256"],
                "outcome": "skipped",
                "ledgerRowUpdated": True,
                "publicBehaviorCredit": 0,
            }
            for mode, ordinal in KEYS
        ],
        "successors": summaries,
    }
    content["promotion-receipt.json"] = (
        json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    ).encode()
    return content, receipt


def _check_output(output_dir: Path) -> None:
    require(output_dir == OUTPUT_DIR, "successor path differs")
    prior._check_output(output_dir)


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
        prior._fsync_directory(staging)
        _check_output(output_dir)
        require(
            prepare(output_dir) == (content, receipt),
            "strict evidence changed during staging",
        )
        prior._publish_exclusive(staging, output_dir)
    finally:
        if staging.exists():
            shutil.rmtree(staging)
    return receipt


def verify_bundle(output_dir: Path) -> dict[str, Any]:
    require(
        output_dir == OUTPUT_DIR
        and output_dir.is_dir()
        and not output_dir.is_symlink(),
        "successor bundle is missing or moved",
    )
    content, receipt = prepare(output_dir)
    require(
        {path.name for path in output_dir.iterdir()} == set(content),
        "successor file set differs",
    )
    for name, expected in content.items():
        path = output_dir / name
        require(
            path.is_file() and not path.is_symlink(),
            f"successor file missing or linked: {name}",
        )
        require(
            path.read_bytes() == expected, f"successor file content differs: {name}"
        )
    return receipt


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify-existing", action="store_true")
    args = parser.parse_args()
    try:
        receipt = (
            verify_bundle(OUTPUT_DIR) if args.verify_existing else publish(OUTPUT_DIR)
        )
    except (
        OSError,
        PromotionError,
        strict.StrictUpstreamSkipEvidenceError,
        subprocess.CalledProcessError,
        subprocess.TimeoutExpired,
        ET.ParseError,
        KeyError,
        TypeError,
        AttributeError,
        IndexError,
    ) as error:
        parser.error(str(error))
    print(
        json.dumps(
            {
                "verified" if args.verify_existing else "published": str(OUTPUT_DIR),
                "successors": receipt["successors"],
            },
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
