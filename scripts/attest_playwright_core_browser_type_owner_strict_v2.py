#!/usr/bin/env python3
"""Seal the verifier behind an already-immutable paired Core ledger bundle."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).parents[1].resolve()
OUTCOMES = ROOT / ".work/official-conformance/outcomes"
BUNDLE = OUTCOMES / "core-browser-type-owner-strict-paired-v2"
OUTPUT = OUTCOMES / "core-browser-type-owner-strict-paired-v2-seal-v1"
CLASSIFICATION = (
    ROOT
    / ".work/official-conformance/playwright-core-v1.58.0-invocation-classifications-trace-private-candidate-v1.jsonl"
)
VERIFIER = ROOT / "scripts/verify_playwright_core_browser_type_owner_strict_v2.py"
PROMOTER = ROOT / "scripts/promote_playwright_core_browser_type_owner_strict_v2.py"
PINS = {
    CLASSIFICATION: "3e260fd6dca8c2c20a3b16c39c196c2bc34c191e1f1dbd59567f89056547a458",
    VERIFIER: "49166047957d83a7aa62c68c43acae0d70090642bb8576cea7c82a0a90c13660",
    PROMOTER: "be64451343d6e4ae79be6792d46f40e6135e1d14ad5386d107b739ff39aa7ecb",
    BUNDLE
    / "promotion-receipt.json": "26e45506688e5bf994393a6d7555cbfd3adc5dffeec1de9370f98b38d504cd0d",
    BUNDLE / "candidate.jsonl": "40625a1de093799e3ad19b0059721c95fd84f3fa04654f373d273ff80abd2e71",
    BUNDLE / "oracle.jsonl": "a03b9b2f9cf1ec06e58db2ce08bad2c770b8eb8620f73ee1aec82d0415f10fd5",
    ROOT
    / ".work/official-conformance/results/playwright-core-candidate-browsertype-owner-strict-v2.strict.json": "8d420350c1e0354c1294868e206e6010c7da792aae90cb908fd6787d128d6e45",
    ROOT
    / ".work/official-conformance/results/playwright-core-oracle-browsertype-owner-strict-v2.strict.json": "84a16f6210e7d4cc5eac0424f92e10a202764d8d174d0d9787ec8c05b8050ec1",
}
PROJECT_IDS = {
    "chromium-library": "6b03a5a0308bccda8853-2cb507360f36bef87bf6",
    "firefox-library": "6b03a5a0308bccda8853-3692f14db41d17bf119c",
    "webkit-library": "6b03a5a0308bccda8853-c7ac17524d35be6489fc",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def sealed(path: Path, expected: str) -> None:
    require(path.is_file() and not path.is_symlink(), f"missing or symlinked evidence: {path}")
    require(sha256(path) == expected, f"SHA-256 drift: {path}")


def check_invocation_classification() -> None:
    matches = []
    with CLASSIFICATION.open() as source:
        for line in source:
            row = json.loads(line)
            if row["specId"] in PROJECT_IDS.values():
                matches.append(row)
    require(len(matches) == 3, "corrected invocation classification count drifted")
    require(
        all(
            row["specId"] == PROJECT_IDS[row["projectId"]]
            and row["sourceClassification"] == "public-api-candidate"
            and row["invocationClassification"] == "implementation-internal"
            and row["invocationClassificationReasons"] == ["private-client-access"]
            for row in matches
        ),
        "corrected invocation classification drifted",
    )


def check_report_assertions() -> None:
    for lane in ("candidate", "oracle"):
        result = (
            ROOT
            / f".work/official-conformance/results/playwright-core-{lane}-browsertype-owner-strict-v2.json"
        )
        report = json.loads(result.read_text())
        require(report["errors"] == [], f"{lane} top-level reporter errors")
        require(
            all(
                report["stats"][name] == value
                for name, value in {
                    "expected": 3,
                    "unexpected": 0,
                    "skipped": 0,
                    "flaky": 0,
                }.items()
            ),
            f"{lane} reporter statistics drifted",
        )
        selected = []

        def visit(suite: dict) -> None:
            for spec in suite.get("specs", []):
                require(
                    spec["file"] == "library/browsertype-basic.spec.ts"
                    and spec["line"] == 21
                    and spec["column"] == 5
                    and spec["title"] == "browserType.executablePath should work"
                    and spec["ok"] is True
                    and len(spec["tests"]) == 1,
                    f"{lane} official spec metadata drifted",
                )
                test = spec["tests"][0]
                require(
                    test["projectId"] in PROJECT_IDS
                    and spec["id"] == PROJECT_IDS[test["projectId"]]
                    and test["expectedStatus"] == "passed"
                    and test["status"] == "expected"
                    and len(test["results"]) == 1,
                    f"{lane} selected project/outcome drifted",
                )
                attempt = test["results"][0]
                require(
                    attempt["retry"] == 0
                    and attempt["status"] == "passed"
                    and attempt["errors"] == []
                    and isinstance(attempt["workerIndex"], int),
                    f"{lane} official attempt drifted",
                )
                selected.append(test["projectId"])
            for child in suite.get("suites", []):
                visit(child)

        for suite in report["suites"]:
            visit(suite)
        require(
            len(selected) == 3 and set(selected) == set(PROJECT_IDS),
            f"{lane} three-engine selection drifted",
        )


def prepare() -> bytes:
    # Verify the executable verifier/promoter before importing or executing either.
    for path, expected in PINS.items():
        sealed(path, expected)
    check_invocation_classification()
    check_report_assertions()
    completed = subprocess.run(
        [sys.executable, str(PROMOTER), "--verify-existing"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    require(completed.returncode == 0, f"paired reconstruction failed: {completed.stderr[-1000:]}")
    reconstructed = json.loads(completed.stdout)
    require(
        reconstructed
        == {
            "scope": "official-browser-free-browsertype-executable-path-owner-v2",
            "lanes": {
                "candidate": {
                    "expected": 4537,
                    "not-run": 43875,
                    "skipped": 11,
                    "unexpected": 54,
                },
                "oracle": {
                    "expected": 4285,
                    "not-run": 44174,
                    "skipped": 10,
                    "unexpected": 8,
                },
            },
        },
        "paired reconstruction counts drifted",
    )
    receipt = {
        "schema": 1,
        "scope": "verifier-source-seal-for-core-browser-type-owner-strict-paired-v2",
        "bundle": str(BUNDLE),
        "bundleReceiptSha256": PINS[BUNDLE / "promotion-receipt.json"],
        "ledgerSha256": {
            "candidate": PINS[BUNDLE / "candidate.jsonl"],
            "oracle": PINS[BUNDLE / "oracle.jsonl"],
        },
        "strictSidecarSha256": {
            "candidate": PINS[
                ROOT
                / ".work/official-conformance/results/playwright-core-candidate-browsertype-owner-strict-v2.strict.json"
            ],
            "oracle": PINS[
                ROOT
                / ".work/official-conformance/results/playwright-core-oracle-browsertype-owner-strict-v2.strict.json"
            ],
        },
        "strictVerifierSha256": PINS[VERIFIER],
        "promoterSha256": PINS[PROMOTER],
        "correctedInvocationClassificationSha256": PINS[CLASSIFICATION],
        "invocationClassification": "implementation-internal/private-client-access",
        "publicApiParityEvidence": False,
        "scopeClarification": (
            "The unchanged public executablePath call and path-exists assertion pass only "
            "after a private fixture backpointer adapter; this does not establish unadapted "
            "public API, exact path equality, or browser launch parity."
        ),
        "attestorSha256": sha256(Path(__file__)),
        "reconstructed": reconstructed,
    }
    return (json.dumps(receipt, indent=2, sort_keys=True) + "\n").encode()


def verify_existing() -> dict:
    expected = prepare()
    require(OUTPUT.is_dir() and not OUTPUT.is_symlink(), "immutable seal directory missing")
    require(
        {path.name for path in OUTPUT.iterdir()} == {"attestation.json"},
        "immutable seal directory has missing or extra files",
    )
    path = OUTPUT / "attestation.json"
    require(
        path.is_file() and not path.is_symlink() and path.read_bytes() == expected,
        "immutable seal attestation drifted",
    )
    return json.loads(expected)


def publish(payload: bytes) -> None:
    require(not OUTPUT.exists() and not OUTPUT.is_symlink(), "immutable seal already exists")
    temporary = Path(tempfile.mkdtemp(prefix=".core-browser-owner-v2-seal-", dir=OUTCOMES))
    try:
        with (temporary / "attestation.json").open("xb") as stream:
            stream.write(payload)
            stream.flush()
            os.fsync(stream.fileno())
        directory_fd = os.open(temporary, os.O_RDONLY)
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)
        require(
            not OUTPUT.exists() and not OUTPUT.is_symlink(),
            "immutable seal appeared during publication",
        )
        os.rename(temporary, OUTPUT)
    finally:
        if temporary.exists():
            shutil.rmtree(temporary)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--verify-existing", action="store_true")
    args = parser.parse_args()
    try:
        if args.verify_existing:
            receipt = verify_existing()
        else:
            payload = prepare()
            require(payload == prepare(), "seal inputs changed before publication")
            publish(payload)
            receipt = verify_existing()
    except (AssertionError, OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        parser.error(f"Core BrowserType evidence seal rejected: {error}")
    print(
        json.dumps({"scope": receipt["scope"], "reconstructed": receipt["reconstructed"]}, indent=2)
    )


if __name__ == "__main__":
    main()
