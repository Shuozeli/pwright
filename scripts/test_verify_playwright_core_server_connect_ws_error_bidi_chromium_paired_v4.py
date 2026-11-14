"""Mutation controls for the independent six-row Bidi Core paired verifier."""

from __future__ import annotations

import json
from collections import Counter
from copy import deepcopy

import pytest

import promote_playwright_core_server_connect_ws_error_bidi_chromium_strict_v4 as promoter
import verify_playwright_core_server_connect_ws_error_bidi_chromium_paired_v4 as verifier


def encoded(record: dict) -> bytes:
    return (json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n").encode()


@pytest.fixture
def six_rows():
    targets = {index: value for index, value in enumerate(verifier.TARGETS.values())}
    source = [
        verifier.target_source(spec_id, project, kind)
        for spec_id, project, kind in targets.values()
    ]
    old = [{**record, "attempts": 0, "outcome": "not-run"} for record in source]
    result = {
        record["specId"]: f"/official/{record['specId']}.json" for record in source
    }
    new = [
        {
            **record,
            "attempts": 1,
            "candidateRuntimeRequired": True,
            "evidenceScope": verifier.SCOPE,
            "outcome": "expected",
            "resultFile": result[record["specId"]],
        }
        for record in old
    ]
    return targets, source, old, new, result


def check(
    six_rows, *, source=None, old=None, new=None, target_override=None, result=None
):
    targets, original_source, original_old, original_new, original_result = six_rows
    source = original_source if source is None else source
    old = original_old if old is None else old
    new = original_new if new is None else new
    results = original_result if result is None else result
    return verifier.verify_successor_lines(
        [encoded(item) for item in source],
        [encoded(item) for item in old],
        [encoded(item) for item in new],
        lane="candidate",
        results_by_id=results,
        prior_outcomes={"not-run": 6},
        prior_scopes={},
        expected_count=6,
        targets=targets if target_override is None else target_override,
    )


def test_exact_six_row_transition_is_accepted(six_rows):
    outcomes, scopes, changed = check(six_rows)
    assert outcomes == {"expected": 6}
    assert scopes == {verifier.SCOPE: 6}
    assert [item["index"] for item in changed] == list(range(6))


@pytest.mark.parametrize(
    "mutation", ["id", "classification", "project", "line", "kind"]
)
def test_source_identity_mutations_rejected(six_rows, mutation):
    _, source, old, new, _ = six_rows
    changed_source, changed_old, changed_new = deepcopy((source, old, new))
    key, value = {
        "id": ("specId", "wrong-id"),
        "classification": ("sourceClassification", "public-api"),
        "project": ("projectId", "chromium-library"),
        "line": ("line", 123),
        "kind": (
            "titlePath",
            ["library/browsertype-connect.spec.ts", "wrong", "should print ws error"],
        ),
    }[mutation]
    for record in (changed_source[0], changed_old[0], changed_new[0]):
        record[key] = value
    with pytest.raises(ValueError, match="target identity/classification"):
        check(six_rows, source=changed_source, old=changed_old, new=changed_new)


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("attempts", 2),
        ("outcome", "unexpected"),
        ("candidateRuntimeRequired", False),
        ("evidenceScope", "wrong-scope"),
        ("resultFile", "/official/wrong.json"),
    ],
)
def test_successor_field_mutations_rejected(six_rows, field, value):
    changed = deepcopy(six_rows[3])
    changed[0][field] = value
    with pytest.raises(ValueError, match="exact allowed delta"):
        check(six_rows, new=changed)


def test_wrong_target_position_rejected(six_rows):
    targets = dict(six_rows[0])
    targets[0], targets[1] = targets[1], targets[0]
    with pytest.raises(ValueError, match="target identity/classification"):
        check(six_rows, target_override=targets)


def test_non_target_raw_byte_change_rejected(six_rows):
    targets, source, old, new, result = six_rows
    extra = {"specId": "unrelated-case", "suite": "other"}
    source_raw = [encoded(item) for item in [*source, extra]]
    old_raw = [
        encoded(item) for item in [*old, {**extra, "attempts": 0, "outcome": "not-run"}]
    ]
    new_raw = [encoded(item) for item in new] + [
        b'{"suite":"other","specId":"unrelated-case","attempts":0,"outcome":"not-run"}\n'
    ]
    with pytest.raises(ValueError, match="non-target row"):
        verifier.verify_successor_lines(
            source_raw,
            old_raw,
            new_raw,
            lane="candidate",
            results_by_id=result,
            prior_outcomes=dict(Counter({"not-run": 7})),
            prior_scopes={},
            expected_count=7,
            targets=targets,
        )


@pytest.fixture(scope="module")
def planned_bundle():
    files = promoter.prepare_pair()
    checked = verifier.verify_inputs()
    verifier.verify_payloads(files, checked_inputs=checked)
    return files, checked


def changed_receipt(files, transform):
    mutated = dict(files)
    receipt = json.loads(files["promotion-receipt.json"])
    transform(receipt)
    mutated["promotion-receipt.json"] = (
        json.dumps(receipt, sort_keys=True) + "\n"
    ).encode()
    return mutated


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("publicApiParityEvidence", True),
        ("targetZeroBasedIndexes", [5349, 5350, 5351, 5352, 5353, 5354]),
        ("promotedRowsPerLane", 7),
        ("promoterSourceSha256", "0" * 64),
        ("pairedVerifierSha256", "0" * 64),
    ],
)
def test_receipt_header_mutations_rejected(planned_bundle, field, value):
    files, checked = planned_bundle
    mutated = changed_receipt(files, lambda receipt: receipt.__setitem__(field, value))
    with pytest.raises(ValueError, match="receipt header"):
        verifier.verify_payloads(mutated, checked_inputs=checked)


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("promotedRows", 7),
        ("candidateArtifactSha256", "0" * 64),
        ("strictSidecarSha256", "0" * 64),
        ("candidateSnapshotManifestSha256", "0" * 64),
    ],
)
def test_lane_receipt_mutations_rejected(planned_bundle, field, value):
    files, checked = planned_bundle
    mutated = changed_receipt(
        files, lambda receipt: receipt["lanes"]["candidate"].__setitem__(field, value)
    )
    with pytest.raises(ValueError, match="candidate receipt"):
        verifier.verify_payloads(mutated, checked_inputs=checked)


def test_non_target_ledger_mutation_rejected(planned_bundle):
    files, checked = planned_bundle
    mutated = dict(files)
    old = mutated["candidate.jsonl"]
    first, rest = old.split(b"\n", 1)
    mutated["candidate.jsonl"] = first + b" " + b"\n" + rest
    with pytest.raises(ValueError, match="non-target row"):
        verifier.verify_payloads(mutated, checked_inputs=checked)


def test_raw_provenance_receipt_mutation_rejected(planned_bundle):
    files, checked = planned_bundle
    mutated = changed_receipt(
        files,
        lambda receipt: receipt["lanes"]["candidate"]["officialRuns"][0].__setitem__(
            "rawProvenanceSha256", "0" * 64
        ),
    )
    with pytest.raises(ValueError, match="candidate receipt"):
        verifier.verify_payloads(mutated, checked_inputs=checked)


def test_summary_count_mutation_rejected(planned_bundle):
    files, checked = planned_bundle
    mutated = dict(files)
    summary = json.loads(files["oracle.summary.json"])
    summary["outcomes"]["expected"] += 1
    mutated["oracle.summary.json"] = (json.dumps(summary) + "\n").encode()
    with pytest.raises(ValueError, match="summary differs"):
        verifier.verify_payloads(mutated, checked_inputs=checked)
