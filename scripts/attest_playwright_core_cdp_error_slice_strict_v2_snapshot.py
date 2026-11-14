#!/usr/bin/env python3
"""Reverify the sealed CDP error pair against its capture-time candidate source.

The original strict verifier intentionally compares provenance with the live
package. This successor attestation supplies a source snapshot whose digest is
exactly the digest captured before the next opt-in feature was added. It does
not change the original sidecars, official reports, ledgers, or receipt.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import promote_playwright_core_cdp_error_slice_strict_v2 as promotion
import run_playwright_core_cdp_error_slice_strict_v1 as source
import run_playwright_core_cdp_error_slice_strict_v2 as capture
import verify_playwright_core_cdp_error_slice_strict_v2 as original
from run_playwright_official_conformance import _candidate_source_sha256
from verify_playwright_official_provenance import verify_provenance

ROOT = source.ROOT
SNAPSHOT = ROOT / ".work/official-conformance/snapshots/core-cdp-error-slice-strict-v2-candidate"
SOURCE_SHA256 = "041a4d60f4901b5f89ffbff492fa13c33e41f6c1ee695c1c81d120e6dba4f03a"
ARTIFACT = SNAPSHOT / "native" / source.PROFILE / "pwright.node"
RECEIPT_SHA256 = "92c6720100cfbecdef6ad36e3bd8fdaf3f327251b4200f83dafeb0ef893e4cb4"
ORIGINAL_VERIFIER_SHA256 = "d059b55943d5a4506f5499e48d4ea9bd06eee40a244a84b620c394e4a08bb7a0"
PROMOTER_SHA256 = "1a41f80e08d476c803d6bd76d351bba943e5216ca7312dedb9340434faedbe99"
GENERIC_VERIFIER_SHA256 = "9b1c6c4edee469c3c7a56866444fb5ad1bafe31443164f497c562a8eda1e0672"
ATTESTATION = (
    ROOT
    / ".work/official-conformance/outcomes/core-cdp-error-slice-strict-paired-v2-seal-v1"
    / "attestation.json"
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def verify_snapshot() -> None:
    require(SNAPSHOT.is_dir() and not SNAPSHOT.is_symlink(), "snapshot directory missing")
    require((SNAPSHOT / "package.json").is_file(), "snapshot package.json missing")
    require((SNAPSHOT / "src").is_dir(), "snapshot src missing")
    require(ARTIFACT.is_file() and not ARTIFACT.is_symlink(), "snapshot artifact missing")
    for path in SNAPSHOT.rglob("*"):
        require(not path.is_symlink(), f"snapshot symlink: {path}")
        require(path.is_file() or path.is_dir(), f"non-regular snapshot entry: {path}")
        if path.is_file():
            relative = path.relative_to(SNAPSHOT)
            require(
                relative == Path("package.json")
                or relative == ARTIFACT.relative_to(SNAPSHOT)
                or relative.is_relative_to("src"),
                f"unexpected snapshot file: {relative}",
            )
    require(_candidate_source_sha256(SNAPSHOT) == SOURCE_SHA256, "snapshot source SHA drift")
    require(source.sha256(ARTIFACT) == source.ARTIFACT_SHA256, "snapshot artifact SHA drift")
    require(
        source.sha256(promotion.OUTPUT / "promotion-receipt.json") == RECEIPT_SHA256,
        "sealed promotion receipt drift",
    )
    for name, digest in (
        ("scripts/verify_playwright_core_cdp_error_slice_strict_v2.py", ORIGINAL_VERIFIER_SHA256),
        ("scripts/promote_playwright_core_cdp_error_slice_strict_v2.py", PROMOTER_SHA256),
        ("scripts/verify_playwright_official_provenance.py", GENERIC_VERIFIER_SHA256),
    ):
        require(source.sha256(ROOT / name) == digest, f"historical verifier source drift: {name}")


def verify_attestation() -> None:
    require(ATTESTATION.is_file() and not ATTESTATION.is_symlink(), "attestation missing")
    seal = json.loads(ATTESTATION.read_text())
    expected = {
        "schema": 1,
        "scope": "historical-capture-time-candidate-source-snapshot",
        "upstreamCommit": source.COMMIT,
        "snapshotPackage": str(SNAPSHOT),
        "snapshotSourceFiles": 64,
        "candidateSourceSha256": SOURCE_SHA256,
        "candidateProfile": source.PROFILE,
        "candidateArtifactSha256": source.ARTIFACT_SHA256,
        "priorPairedReceiptSha256": RECEIPT_SHA256,
        "attestorSha256": source.sha256(Path(__file__)),
        "attestorTestsSha256": source.sha256(
            ROOT / "scripts/test_attest_playwright_core_cdp_error_slice_strict_v2_snapshot.py"
        ),
        "originalVerifierSha256": ORIGINAL_VERIFIER_SHA256,
        "originalPromoterSha256": PROMOTER_SHA256,
        "genericProvenanceVerifierSha256": GENERIC_VERIFIER_SHA256,
        "publicApiParityEvidence": False,
    }
    require(seal == expected, "attestation fields or source pins drift")
    require(
        sum(path.is_file() for path in (SNAPSHOT / "src").rglob("*"))
        == seal["snapshotSourceFiles"],
        "snapshot source file count drift",
    )


def verify_strict_from_snapshot(strict_path: Path) -> dict[str, Any]:
    require(strict_path.is_file() and not strict_path.is_symlink(), "strict sidecar missing")
    strict = json.loads(strict_path.read_text())
    require(set(strict) == original.EXPECTED_FIELDS, "strict sidecar fields drift")
    require(strict["schema"] == 3 and strict["scope"] == capture.SCOPE, "strict scope drift")
    mode = strict["mode"]
    require(mode in {"candidate", "oracle"}, "strict mode drift")
    require(strict["upstreamCommit"] == source.COMMIT, "upstream commit drift")
    require(strict["upstreamSourceSha256"] == source.SOURCE_SHA256, "upstream source drift")
    require(strict["serverFixtureSha256"] == source.SERVER_FIXTURE_SHA256, "fixture source drift")
    require(strict["commonPreloadSha256"] == source.COMMON_PRELOAD_SHA256, "preload drift")
    source.check_manifest()
    inputs = strict["inputs"]
    require(
        isinstance(inputs, dict)
        and set(inputs)
        == {
            str(source.MANIFEST.relative_to(ROOT)),
            str(source.PRELOAD.relative_to(ROOT)),
            str(source.COMMON_PRELOAD.relative_to(ROOT)),
            str(capture.BASE_RUNNER.relative_to(ROOT)),
            "scripts/run_playwright_core_cdp_error_slice_strict_v2.py",
        },
        "capture input set drift",
    )
    for name, digest in inputs.items():
        require(source.sha256(ROOT / name) == digest, f"capture input drift: {name}")
    result = Path(strict["result"])
    require(
        result.is_absolute()
        and result.parent == capture.RESULT_PARENT
        and result.with_suffix(".strict.json") == strict_path
        and result.is_file()
        and not result.is_symlink()
        and source.sha256(result) == strict["resultSha256"],
        "official report drift",
    )
    provenance_path = result.with_suffix(".provenance.json")
    receipt_path = result.with_suffix(".guard-receipt.jsonl")
    log_path = result.with_suffix(".log")
    for path, key in (
        (provenance_path, "provenanceSha256"),
        (receipt_path, "guardReceiptSha256"),
        (log_path, "logSha256"),
    ):
        require(
            path.is_file() and not path.is_symlink() and source.sha256(path) == strict[key],
            f"capture output drift: {path.name}",
        )
    provenance = verify_provenance(
        json.loads(provenance_path.read_text()),
        provenance_path=provenance_path,
        manifest_override=source.MANIFEST,
        result_override=result,
        upstream_override=source.UPSTREAM,
        candidate_override=SNAPSHOT if mode == "candidate" else None,
        tooling_root=ROOT,
    )
    require(
        provenance["mode"] == mode
        and provenance["suite"] == source.SUITE
        and provenance["project"] == "chromium-library"
        and provenance["workers"] == 1
        and provenance["filters"] == [f"{source.SOURCE}:{line}" for line in source.SELECTED_LINES]
        and provenance["resultSha256"] == strict["resultSha256"]
        and provenance["exitCode"] == strict["exitCode"],
        "official provenance drift",
    )
    host = provenance["fixtureHost"]
    require(isinstance(host, str) and host.endswith(".ts.net"), "fixture host drift")
    log = log_path.read_text()
    if mode == "candidate":
        require(
            strict["candidateProfile"] == source.PROFILE
            and strict["candidateArtifactSha256"] == source.ARTIFACT_SHA256
            and provenance["candidateProfile"] == source.PROFILE
            and provenance["candidateArtifactSha256"] == source.ARTIFACT_SHA256
            and provenance["candidateSourceSha256"] == SOURCE_SHA256
            and provenance["candidate"] is not None
            and "[pwright-candidate]" in log,
            "candidate identity drift",
        )
    else:
        require(
            strict["candidateProfile"] is None
            and strict["candidateArtifactSha256"] is None
            and provenance["candidate"] is None
            and "[pwright-candidate]" not in log,
            "oracle identity drift",
        )
    selected = capture.selected_tests(json.loads(result.read_text()))
    require(selected == strict["selected"], "selected tests drift")
    require(
        strict["exitCode"] == (0 if all(item["status"] == "expected" for item in selected) else 1),
        "exit/status mismatch",
    )
    capture.checked_receipts(receipt_path, selected, host)
    return strict


def verify() -> dict[str, Any]:
    verify_snapshot()
    verify_attestation()
    old = promotion.verify_strict
    promotion.verify_strict = verify_strict_from_snapshot
    try:
        receipt = promotion.verify_existing()
    finally:
        promotion.verify_strict = old
    require(receipt["publicApiParityEvidence"] is False, "internal slice misclassified")
    return receipt


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args()
    try:
        receipt = verify()
    except (AssertionError, OSError, ValueError, KeyError, TypeError) as error:
        parser.error(f"historical CDP error pair attestation rejected: {error}")
    print(
        f"verified archived Core CDP error pair candidateSourceSha256={SOURCE_SHA256} "
        f"targetRows={receipt['targetZeroBasedIndexes']}"
    )


if __name__ == "__main__":
    main()
