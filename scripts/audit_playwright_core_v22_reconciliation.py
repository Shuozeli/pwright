"""Reconcile frozen Core v22 outcomes with source-reviewed classifications.

This audit does not promote, rewrite, or reinterpret either immutable outcome
ledger. In particular, a source-only ``expected`` row is not a runtime pass.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter
from collections.abc import Iterable, Iterator
from itertools import zip_longest
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
CORPUS_SIZE = 48_477
SOURCE_ONLY = "upstream-private-source-only"
UPSTREAM_COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
BASE = ".work/official-conformance"
OUTCOME_BASE = f"{BASE}/outcomes/core-frame-hierarchy13-rust-owner-strict-paired-v22"

# These are frozen input bytes, not hashes calculated from the current worktree.
PINNED_SHA256 = {
    f"{BASE}/playwright-core-v1.58.0.jsonl": "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30",
    f"{BASE}/playwright-core-v1.58.0-invocation-classifications.jsonl": "927498c7f312c32dfcaff5ee8b58b597f5af8f879f1a5cd5bcbb19341fdb4412",
    f"{BASE}/playwright-core-v1.58.0-invocation-classifications.summary.json": "9a520ee8914a6f0256c5b824cab4c72c670731be5a67f583870cbc24dee59ab1",
    f"{OUTCOME_BASE}/candidate.jsonl": "b5e7c47d3be064f2bfa8627407a5d1417c4d54f3b54c32cff0e838297cbe7d7f",
    f"{OUTCOME_BASE}/candidate.summary.json": "742bd0b5cd6cb6a00cb3c7e8bc79f51c0333f306acae71957f425cc722393d44",
    f"{OUTCOME_BASE}/oracle.jsonl": "3918f3bd1e2f935d20b94d5c0edd8f34a6c17699b02d89715ab3252675d150cd",
    f"{OUTCOME_BASE}/oracle.summary.json": "74f60b7bc47fcc98c4d948ce12c84816ce475bb9733bbc28bd178dd89e38e77b",
    f"{OUTCOME_BASE}/promotion-receipt.json": "535148fa2b02b5af9e5519604cebef296f6e7daa1ff3c43dcba79139f933270a",
}


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _verify_pinned_file(path: Path, expected: str) -> None:
    if not path.is_file():
        raise ValueError(f"pinned input is missing: {path}")
    observed = _sha256(path)
    if observed != expected:
        raise ValueError(f"pinned input SHA-256 mismatch: {path}: {observed}")


def _json_lines(path: Path) -> Iterator[dict[str, Any]]:
    with path.open(encoding="utf-8") as stream:
        for number, line in enumerate(stream, start=1):
            record = json.loads(line)
            if not isinstance(record, dict):
                raise TypeError(f"non-object JSONL record: {path}:{number}")
            yield record


def _identity(record: dict[str, Any]) -> tuple[str, str, str, str]:
    try:
        key = (
            record["suite"],
            record["sourceFile"],
            record["projectId"],
            record["specId"],
        )
    except KeyError as error:
        raise ValueError(f"invocation lacks identity field: {error.args[0]}") from error
    # Some official example and aggregate suites have no named project.
    if not (
        all(isinstance(value, str) and value for value in (key[0], key[1], key[3]))
        and isinstance(key[2], str)
    ):
        raise ValueError(f"invalid invocation identity: {key}")
    return key


def _checked_source_only(record: dict[str, Any], lane: str) -> bool:
    if record.get("evidenceScope") != SOURCE_ONLY:
        return False
    if record.get("outcome") != "expected":
        raise ValueError(f"{lane} source-only row is not expected: {_identity(record)}")
    if record.get("candidateRuntimeRequired") is not False:
        raise ValueError(
            f"{lane} source-only row claims candidate runtime: {_identity(record)}"
        )
    return True


def reconcile(
    inventory: Iterable[dict[str, Any]],
    classifications: Iterable[dict[str, Any]],
    candidate: Iterable[dict[str, Any]],
    oracle: Iterable[dict[str, Any]],
    *,
    expected_records: int,
) -> dict[str, Any]:
    """Join every row by exact pinned order and composite identity."""
    counts: dict[str, Counter[tuple[str, str, str]]] = {
        "candidate": Counter(),
        "oracle": Counter(),
    }
    source_only_counts: dict[str, Counter[str]] = {
        "candidate": Counter(),
        "oracle": Counter(),
    }
    public_source_only: dict[str, list[dict[str, Any]]] = {
        "candidate": [],
        "oracle": [],
    }
    seen: set[tuple[str, str, str, str]] = set()
    observed = 0
    for number, bundle in enumerate(
        zip_longest(inventory, classifications, candidate, oracle), start=1
    ):
        if any(record is None for record in bundle):
            raise ValueError(f"row-count mismatch at row {number}")
        source, reviewed, candidate_row, oracle_row = bundle
        assert source is not None
        assert reviewed is not None
        assert candidate_row is not None
        assert oracle_row is not None
        key = _identity(source)
        if key in seen:
            raise ValueError(f"duplicate inventory invocation: {key}")
        seen.add(key)
        for name, row in (
            ("classification", reviewed),
            ("candidate", candidate_row),
            ("oracle", oracle_row),
        ):
            if _identity(row) != key:
                raise ValueError(
                    f"{name} identity/order mismatch at row {number}: {key}"
                )
            if row.get("sourceClassification") != source.get("sourceClassification"):
                raise ValueError(f"{name} source class mismatch at row {number}: {key}")
        if reviewed.get("sourceStartLine") != source.get("line"):
            raise ValueError(
                f"classification source line mismatch at row {number}: {key}"
            )
        if reviewed.get("sourceEndLine", 0) < reviewed["sourceStartLine"]:
            raise ValueError(f"classification source range is invalid: {key}")
        classification = reviewed.get("invocationClassification")
        if classification not in {
            "public-api-candidate",
            "public-api-with-private-test-helper",
            "public-api-with-implementation-private-hook",
            "implementation-internal",
            "product-infrastructure",
        }:
            raise ValueError(
                f"unknown reviewed class at row {number}: {classification}"
            )
        if classification in {"implementation-internal", "product-infrastructure"}:
            reasons = reviewed.get("invocationClassificationReasons")
            if not isinstance(reasons, list) or not reasons:
                raise ValueError(f"unexplained reviewed non-public row: {key}")
        for lane, row in (("candidate", candidate_row), ("oracle", oracle_row)):
            outcome = row.get("outcome")
            if outcome not in {"expected", "unexpected", "skipped", "not-run"}:
                raise ValueError(f"invalid {lane} outcome at row {number}: {outcome}")
            scope = row.get("evidenceScope")
            if scope is not None and not isinstance(scope, str):
                raise ValueError(
                    f"invalid {lane} evidence scope at row {number}: {key}"
                )
            counts[lane][(classification, outcome, scope or "unscoped")] += 1
            if _checked_source_only(row, lane):
                source_only_counts[lane][classification] += 1
                if classification.startswith("public-api-"):
                    public_source_only[lane].append(
                        {
                            "suite": key[0],
                            "sourceFile": key[1],
                            "projectId": key[2],
                            "specId": key[3],
                            "titlePath": source.get("titlePath"),
                            "sourceStartLine": reviewed["sourceStartLine"],
                            "sourceEndLine": reviewed["sourceEndLine"],
                            "reviewedClass": classification,
                            "reviewedReasons": reviewed.get(
                                "invocationClassificationReasons"
                            ),
                            "outcome": outcome,
                            "evidenceScope": SOURCE_ONLY,
                            "candidateRuntimeRequired": False,
                            "runtimeAssessment": "runtime-unproven",
                            "applicabilityAssessment": "public-behavior-still-applicable",
                        }
                    )
        observed += 1
    if observed != expected_records:
        raise ValueError(f"expected {expected_records} rows; observed {observed}")
    candidate_public = {_identity(row) for row in public_source_only["candidate"]}
    oracle_public = {_identity(row) for row in public_source_only["oracle"]}
    if candidate_public != oracle_public:
        raise ValueError("candidate and oracle source-only public ID sets differ")
    return {
        "recordsPerLane": observed,
        "crossTab": {
            lane: [
                {
                    "reviewedClass": classification,
                    "outcome": outcome,
                    "evidenceScope": scope,
                    "count": count,
                }
                for (classification, outcome, scope), count in sorted(counter.items())
            ]
            for lane, counter in counts.items()
        },
        "sourceOnlyByReviewedClass": {
            lane: dict(sorted(counter.items()))
            for lane, counter in source_only_counts.items()
        },
        "reviewedPublicSourceOnly": public_source_only,
        "reviewedPublicSourceOnlyCountPerLane": len(candidate_public),
    }


def _verify_metadata(root: Path) -> None:
    inventory_sha = PINNED_SHA256[f"{BASE}/playwright-core-v1.58.0.jsonl"]
    classification_sha = PINNED_SHA256[
        f"{BASE}/playwright-core-v1.58.0-invocation-classifications.jsonl"
    ]
    classification_summary = json.loads(
        (
            root
            / f"{BASE}/playwright-core-v1.58.0-invocation-classifications.summary.json"
        ).read_text()
    )
    receipt = json.loads((root / f"{OUTCOME_BASE}/promotion-receipt.json").read_text())
    if (
        classification_summary.get("records") != CORPUS_SIZE
        or classification_summary.get("commit") != UPSTREAM_COMMIT
        or classification_summary.get("inventorySha256") != inventory_sha
        or classification_summary.get("ledgerSha256") != classification_sha
    ):
        raise ValueError("reviewed classification metadata differs from pinned corpus")
    if (
        receipt.get("recordsPerLane") != CORPUS_SIZE
        or receipt.get("inventorySha256") != inventory_sha
        or receipt.get("fullCorpusExecution") is not False
        or receipt.get("publicApiParityEvidence") is not False
    ):
        raise ValueError("v22 receipt metadata differs from frozen incomplete scope")
    for lane in ("candidate", "oracle"):
        summary = json.loads((root / f"{OUTCOME_BASE}/{lane}.summary.json").read_text())
        if (
            summary.get("records") != CORPUS_SIZE
            or summary.get("ledgerSha256")
            != PINNED_SHA256[f"{OUTCOME_BASE}/{lane}.jsonl"]
            or summary.get("inventory") != receipt.get("inventory")
        ):
            raise ValueError(f"{lane} outcome summary differs from frozen v22")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output",
        type=Path,
        default=ROOT / f"{BASE}/audits/core-v22-reconciliation-v1.json",
    )
    args = parser.parse_args()
    for relative_path, expected in PINNED_SHA256.items():
        _verify_pinned_file(ROOT / relative_path, expected)
    _verify_metadata(ROOT)
    audit = reconcile(
        _json_lines(ROOT / f"{BASE}/playwright-core-v1.58.0.jsonl"),
        _json_lines(
            ROOT / f"{BASE}/playwright-core-v1.58.0-invocation-classifications.jsonl"
        ),
        _json_lines(ROOT / f"{OUTCOME_BASE}/candidate.jsonl"),
        _json_lines(ROOT / f"{OUTCOME_BASE}/oracle.jsonl"),
        expected_records=CORPUS_SIZE,
    )
    if audit["reviewedPublicSourceOnlyCountPerLane"] != 19:
        raise ValueError("frozen v22 reviewed-public source-only count changed")
    if any(
        sum(audit["sourceOnlyByReviewedClass"][lane].values()) != 2610
        for lane in ("candidate", "oracle")
    ):
        raise ValueError("frozen v22 source-only scope count changed")
    report = {
        "schema": 1,
        "kind": "core-v22-read-only-reconciliation",
        "upstreamCommit": UPSTREAM_COMMIT,
        "inputSha256": dict(sorted(PINNED_SHA256.items())),
        "strictOutcomeLedgersUnchanged": True,
        "fullCorpusExecution": False,
        "publicApiParityEvidence": False,
        **audit,
    }
    payload = json.dumps(report, indent=2, sort_keys=True) + "\n"
    output = args.output.resolve()
    if output.is_file():
        if output.read_text() != payload:
            raise ValueError(
                f"refusing to overwrite a different audit report: {output}"
            )
    else:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(payload)
    print(f"{output}: {_sha256(output)}")
    print(
        "reconciled 48477 IDs per lane; source-only reviewed-public "
        "runtime-unproven IDs: 19 per lane"
    )


if __name__ == "__main__":
    main()
