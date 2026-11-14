"""Promote only 11 paired, pre-body upstream Python marker skips.

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

import promote_playwright_python_upstream_skip_markers_strict_v1 as prior
import verify_playwright_python_upstream_skip_callable_fixtures_strict_v2 as strict

ROOT = Path(__file__).resolve().parents[1]
OUTCOMES = ROOT / ".work/official-conformance/outcomes"
BASE_DIR = OUTCOMES / "python-upstream-skip-markers-strict-paired-v1"
OUTPUT_DIR = OUTCOMES / "python-upstream-skip-callable-fixtures-strict-paired-v2"
RESULT_DIR = (
    ROOT
    / ".work/official-conformance/results/python-upstream-skip-callable-fixtures-strict-v2"
)
INVENTORY = ROOT / ".work/official-conformance/playwright-python-v1.58.0-profiled.jsonl"
INVENTORY_SHA256 = "3dfb11b6ad7eb4736166ee2c4f7fa32cda78063324ad8314ce12f3e2d49b9d41"
BASE_PROMOTER_SHA256 = (
    "7a2c6431a0027ad53eaeca4852fd413f6e025f6ad535e2fbec40e4e41ea4818f"
)
BASE_RECEIPT_SHA256 = "21c65650e7f3b8c35c91e4d08b317227fe9ba94b06c0cc83c4fba0b2390287f1"
BASES = {
    "candidate": (
        "42edaee19df8ab7f88cbe1e9952389319c0d7affaa2d7c34bfd2c5b0c692d876",
        "90c0803489cc647b6e3c3563fc4df6d8c64973b26e48662b7fac715b20caab0c",
    ),
    "oracle": (
        "349dd43949b61fa3aec58edb8ff612960e43d8043354f5d75c379d74fcd7deaa",
        "47ab18cdffe320c860b8aec45ac815ab445f6bc7f65486ec1b9e62c0c65611b3",
    ),
}
MODES = ("candidate", "oracle")
ORDINALS = tuple(range(11))
KEYS = tuple((mode, ordinal) for mode in MODES for ordinal in ORDINALS)
TARGET_INDEXES = (
    1259,
    1625,
    1626,
    1628,
    1629,
    1631,
    1632,
    5273,
    5274,
    5276,
    5277,
)
RECORDS = 6_526
GENERATED = frozenset({"attempts", "durationSeconds", "error", "outcome", "resultFile"})
RECEIPT_KIND = (
    "playwright-python-upstream-skip-callable-fixtures-strict-paired-promotion-v2"
)
POLICY_SHA256 = "929225455a1f371ff671e5800455746f7c03d18c40f709d68d6f6a7ec077eb69"
TOOL_SHA256 = {
    "scripts/run_playwright_python_upstream_skip_callable_fixtures_strict_v2.py": (
        "ccd4032b0d2706008477b3c418e7b208766d3ca893610ecdb4d7516bff0fcdbd"
    ),
    "scripts/verify_playwright_python_upstream_skip_callable_fixtures_strict_v2.py": (
        "95a49c1d391a55b792828ecd9bbae38a41b741c94d34e4209502c69dcfbc4526"
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
    ): "e36a917caf6eb668b9543ea466348c98a29839a3766c8ab27b2ea6135ebc145a",
    (
        "candidate",
        1,
    ): "9a211d356b98f6f53a668639bbff6b443470d14f27b0d203c473c262b3dadfa5",
    (
        "candidate",
        2,
    ): "838cfa4eecc39ae06ee30806529b72c2c1db7d5ba554b52eca0f60206fa901e5",
    (
        "candidate",
        3,
    ): "111d8575091da6e17588269ab97407472d7b2c3e9a3a643cebd4fd30617d1649",
    (
        "candidate",
        4,
    ): "b1030c8898a3cde5b7b0e2c0994566d1b8b5d75831230f4277491684abda0165",
    (
        "candidate",
        5,
    ): "a970c6d980f567a540869ee4c40c0a0e832bc7f301d1c7ab853c59b4755f0a14",
    (
        "candidate",
        6,
    ): "0efc6753d57162e6c87c5de2bda499e53556da76e55afe34ee4181f491fc5dc5",
    (
        "candidate",
        7,
    ): "0cb25fefe42318aa3fe2f17888f02ad81b5b64eef6acd64723473d0ec9744592",
    (
        "candidate",
        8,
    ): "4e5133e7cfb7c893a1c789213935f0e9e1bfd458f62999a1055c621e6a1ea791",
    (
        "candidate",
        9,
    ): "4f0be560802e7e05fd26e27c50c3e893f696008c656cb1782bb3988b1ca1da9f",
    (
        "candidate",
        10,
    ): "c8df27669718edc971871652a2b83fcd7e3107efef2bae804067fb3b56426a7c",
    ("oracle", 0): "f9dc4d7c377b8946f90c9426933c926f00cee0e93f04c1e57c7b853bb7703d09",
    ("oracle", 1): "b6fb7fce4a22e102b8ea0b69b1ef8d57d89a2bf36aac7e62db8fb47c2391e2b8",
    ("oracle", 2): "9a4df651ea17ddbe94d771911c384bd629f28997d56f013edd5a060cd1b17819",
    ("oracle", 3): "2a28b6cd134486e18021848cd566230855c7a950a7f75d56a44260e4f8de1da4",
    ("oracle", 4): "42b5ce9a58b70f2222151ee9d30eba0170421b66502fd268b59bf44cf72a63b3",
    ("oracle", 5): "0fd5764c779088c1351f0aa8aa1cf37b3d715a5abe988927b55e45bf96719bfe",
    ("oracle", 6): "f12f73926f2f72d8e13e264bf0d6c520a54b0a4bab09b4128f84ba29ec660213",
    ("oracle", 7): "f4169bdee4ea70914ebf2089198e99f0d7621a49f26da1933030cac78944fc07",
    ("oracle", 8): "f0f3c40dc0736fffe9335aefa965f2fabe42ab552f3a0ccee84cac4a34e68409",
    ("oracle", 9): "3a9b277598034b44f2f1467b95143ea5ecf5d548c506590a3d97ac2b61da94e8",
    ("oracle", 10): "378f218707e3646c63c43972ffc9e5637b0604b879e4f65b8c5d22e771562802",
}

PromotionError = prior.PromotionError
require = prior.require
sha256 = prior.sha256
sealed = prior.sealed
json_object = prior.json_object
jsonl = prior.jsonl
digest_pin = prior.digest_pin


def base_name(mode: str) -> str:
    return f"playwright-python-{mode}-upstream-skip-markers-strict-paired-v1.jsonl"


def output_name(mode: str) -> str:
    return f"playwright-python-{mode}-upstream-skip-callable-fixtures-strict-paired-v2.jsonl"


def target(ordinal: int) -> str:
    return strict.TEST_IDS[ordinal]


def base() -> dict[str, tuple[list[bytes], list[dict[str, Any]]]]:
    """Verify the entire predecessor bundle and pin every input byte."""
    prior_source = (
        ROOT / "scripts/promote_playwright_python_upstream_skip_markers_strict_v1.py"
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
        len(TARGET_INDEXES) == len(set(TARGET_INDEXES)) == len(strict.TEST_IDS) == 11
        and tuple(ids[index] for index in TARGET_INDEXES) == strict.TEST_IDS,
        "11 skip-only target IDs moved",
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
        / "packages/pwright-python/tests/official/upstream-skip-callable-fixtures-strict-v2-manifest.json",
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
        "22 callable-v2 sidecar pins are incomplete",
    )
    require(len(set(SIDECARS.values())) == 22, "sidecar slots are not unique")
    for key in KEYS:
        require(
            SIDECARS[key] == RESULT_DIR / f"{key[0]}-{key[1]:03d}.provenance.json",
            f"sidecar slot moved: {key}",
        )
        digest_pin(SIDECAR_SHA256[key], f"callable-v2 sidecar {key}")
    require(
        RESULT_DIR.is_dir() and not RESULT_DIR.is_symlink(),
        "callable-v2 evidence directory is absent or linked",
    )
    expected_names = {
        f"{mode}-{ordinal:03d}{suffix}"
        for mode, ordinal in KEYS
        for suffix in (".provenance.json", ".call.jsonl", ".xml")
    }
    require(
        {path.name for path in RESULT_DIR.iterdir()} == expected_names,
        "callable-v2 evidence directory does not contain exactly 66 artifacts",
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
    """Verify each paired skip and the 22 immutable callable-v2 sidecar pins."""
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
            len({report[field] for report in verified.values()}) == 22,
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
    counts["not-run"] -= 11
    counts["skipped"] += 11
    require(counts["not-run"] >= 0, "not-run count underflow")
    require(
        all(
            updated[index] == original
            for index, original in enumerate(lines)
            if index not in TARGET_INDEXES
        ),
        f"one of the other 6,515 {mode} row bytes changed",
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
        "scope": "11-upstream-pre-body-marker-skips-only",
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
    prior.prior._check_output(output_dir)


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
        prior.prior._fsync_directory(staging)
        _check_output(output_dir)
        require(
            prepare(output_dir) == (content, receipt),
            "strict evidence changed during staging",
        )
        prior.prior._publish_exclusive(staging, output_dir)
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
