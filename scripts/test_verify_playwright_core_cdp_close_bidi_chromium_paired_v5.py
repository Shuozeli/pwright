"""Mutation controls for the three-row Bidi CDP Close paired successor."""

from __future__ import annotations

import json
from copy import deepcopy

import promote_playwright_core_cdp_close_bidi_chromium_strict_v5 as promoter
import pytest
import verify_playwright_core_cdp_close_bidi_chromium_paired_v5 as verifier


def encoded(record: dict) -> bytes:
    return (json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n").encode()


@pytest.fixture
def three_rows():
    targets = {index: value for index, value in enumerate(verifier.TARGETS.values())}
    source = [
        verifier.target_source(spec_id, project)
        for spec_id, project in targets.values()
    ]
    old = [{**record, "attempts": 0, "outcome": "not-run"} for record in source]
    results = {
        record["specId"]: f"/official/{record['specId']}.json" for record in source
    }
    new = [
        {
            **record,
            "attempts": 1,
            "candidateRuntimeRequired": True,
            "evidenceScope": verifier.SCOPE,
            "outcome": "expected",
            "resultFile": results[record["specId"]],
        }
        for record in old
    ]
    return targets, source, old, new, results


def check_rows(
    three_rows, *, source=None, old=None, new=None, targets=None, lane="candidate"
):
    base_targets, base_source, base_old, base_new, results = three_rows
    return verifier.verify_successor_lines(
        [encoded(item) for item in (base_source if source is None else source)],
        [encoded(item) for item in (base_old if old is None else old)],
        [encoded(item) for item in (base_new if new is None else new)],
        lane=lane,
        results_by_id=results,
        prior_outcomes={"not-run": 3},
        prior_scopes={},
        expected_count=3,
        targets=base_targets if targets is None else targets,
    )


def test_exact_three_row_candidate_transition_accepted(three_rows):
    outcomes, scopes, changed = check_rows(three_rows)
    assert outcomes == {"expected": 3}
    assert scopes == {verifier.SCOPE: 3}
    assert [item["index"] for item in changed] == [0, 1, 2]


def test_exact_three_row_oracle_transition_accepted(three_rows):
    targets, source, old, new, _ = three_rows
    oracle = deepcopy(new)
    for row in oracle:
        row["candidateRuntimeRequired"] = False
    outcomes, scopes, changed = check_rows(
        three_rows, source=source, old=old, new=oracle, targets=targets, lane="oracle"
    )
    assert outcomes == {"expected": 3}
    assert scopes == {verifier.SCOPE: 3}
    assert len(changed) == 3


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("specId", "wrong-id"),
        ("sourceClassification", "public-api"),
        ("classificationReasons", ["private-client-access"]),
        ("projectId", "chromium-library"),
        ("line", 573),
        ("titlePath", ["wrong", "should print custom ws close error"]),
    ],
)
def test_target_source_mutations_rejected(three_rows, field, value):
    _, source, old, new, _ = three_rows
    source, old, new = deepcopy((source, old, new))
    for record in (source[0], old[0], new[0]):
        record[field] = value
    with pytest.raises(ValueError, match="target identity/classification"):
        check_rows(three_rows, source=source, old=old, new=new)


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
def test_target_successor_mutations_rejected(three_rows, field, value):
    changed = deepcopy(three_rows[3])
    changed[0][field] = value
    with pytest.raises(ValueError, match="exact allowed delta"):
        check_rows(three_rows, new=changed)


def test_wrong_target_position_rejected(three_rows):
    targets = dict(three_rows[0])
    targets[0], targets[1] = targets[1], targets[0]
    with pytest.raises(ValueError, match="target identity/classification"):
        check_rows(three_rows, targets=targets)


def test_non_target_raw_byte_change_rejected(three_rows):
    targets, source, old, new, results = three_rows
    extra = {"specId": "unrelated-case", "suite": "other"}
    source_raw = [encoded(item) for item in [*source, extra]]
    old_raw = [encoded(item) for item in old] + [
        encoded({**extra, "attempts": 0, "outcome": "not-run"})
    ]
    changed_raw = [encoded(item) for item in new] + [
        b'{"suite":"other","specId":"unrelated-case","attempts":0,'
        b'"outcome":"not-run"}\n'
    ]
    with pytest.raises(ValueError, match="non-target row"):
        verifier.verify_successor_lines(
            source_raw,
            old_raw,
            changed_raw,
            lane="candidate",
            results_by_id=results,
            prior_outcomes={"not-run": 4},
            prior_scopes={},
            expected_count=4,
            targets=targets,
        )


@pytest.fixture(scope="module")
def planned_bundle():
    files = promoter.prepare_pair()
    checked = verifier.verify_inputs()
    verifier.verify_payloads(files, checked_inputs=checked)
    return files, checked


def changed_receipt(files, transform):
    changed = dict(files)
    receipt = json.loads(files["promotion-receipt.json"])
    transform(receipt)
    changed["promotion-receipt.json"] = (
        json.dumps(receipt, sort_keys=True) + "\n"
    ).encode()
    return changed


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("publicApiParityEvidence", True),
        ("bidiProtocolParityEvidence", True),
        ("currentLivePackageEvidence", True),
        ("browserExecutionEvidence", True),
        ("fixtureAdapted", False),
        ("promotedRowsPerLane", 4),
        ("targetZeroBasedIndexes", [6527, 6528, 6530]),
        ("promoterSourceSha256", "0" * 64),
        ("pairedVerifierSha256", "0" * 64),
    ],
)
def test_receipt_header_mutations_rejected(planned_bundle, field, value):
    files, checked = planned_bundle
    changed = changed_receipt(files, lambda receipt: receipt.__setitem__(field, value))
    with pytest.raises(ValueError, match="receipt header"):
        verifier.verify_payloads(changed, checked_inputs=checked)


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("promotedRows", 4),
        ("candidateArtifactSha256", "0" * 64),
        ("strictSidecarSha256", "0" * 64),
        ("candidateSnapshotManifestSha256", "0" * 64),
        ("candidateSourceSha256", "0" * 64),
    ],
)
def test_receipt_lane_mutations_rejected(planned_bundle, field, value):
    files, checked = planned_bundle
    changed = changed_receipt(
        files,
        lambda receipt: receipt["lanes"]["candidate"].__setitem__(field, value),
    )
    with pytest.raises(ValueError, match="candidate receipt"):
        verifier.verify_payloads(changed, checked_inputs=checked)


def test_raw_provenance_receipt_mutation_rejected(planned_bundle):
    files, checked = planned_bundle
    changed = changed_receipt(
        files,
        lambda receipt: receipt["lanes"]["oracle"]["officialRuns"][0].__setitem__(
            "rawProvenanceSha256", "0" * 64
        ),
    )
    with pytest.raises(ValueError, match="oracle receipt"):
        verifier.verify_payloads(changed, checked_inputs=checked)


def test_non_target_ledger_byte_mutation_rejected(planned_bundle):
    files, checked = planned_bundle
    changed = dict(files)
    first, rest = files["candidate.jsonl"].split(b"\n", 1)
    changed["candidate.jsonl"] = first + b" " + b"\n" + rest
    with pytest.raises(ValueError, match="non-target row"):
        verifier.verify_payloads(changed, checked_inputs=checked)


def test_target_ledger_mutation_rejected(planned_bundle):
    files, checked = planned_bundle
    changed = dict(files)
    lines = changed["oracle.jsonl"].splitlines(keepends=True)
    row = json.loads(lines[6527])
    row["candidateRuntimeRequired"] = True
    lines[6527] = encoded(row)
    changed["oracle.jsonl"] = b"".join(lines)
    with pytest.raises(ValueError, match="exact allowed delta"):
        verifier.verify_payloads(changed, checked_inputs=checked)


def test_target_ledger_noncanonical_whitespace_rejected(planned_bundle):
    files, checked = planned_bundle
    changed = dict(files)
    lines = changed["candidate.jsonl"].splitlines(keepends=True)
    lines[6527] = lines[6527][:-1] + b" \n"
    changed["candidate.jsonl"] = b"".join(lines)
    with pytest.raises(ValueError, match="exact allowed delta"):
        verifier.verify_payloads(changed, checked_inputs=checked)


def test_summary_count_mutation_rejected(planned_bundle):
    files, checked = planned_bundle
    changed = dict(files)
    summary = json.loads(files["oracle.summary.json"])
    summary["outcomes"]["expected"] += 1
    changed["oracle.summary.json"] = (json.dumps(summary) + "\n").encode()
    with pytest.raises(ValueError, match="summary differs"):
        verifier.verify_payloads(changed, checked_inputs=checked)


def test_extra_successor_file_rejected(planned_bundle):
    files, checked = planned_bundle
    changed = {**files, "unexpected.txt": b"extra"}
    with pytest.raises(ValueError, match="file set differs"):
        verifier.verify_payloads(changed, checked_inputs=checked)


def test_missing_successor_file_rejected(planned_bundle):
    files, checked = planned_bundle
    changed = dict(files)
    del changed["oracle.summary.json"]
    with pytest.raises(ValueError, match="file set differs"):
        verifier.verify_payloads(changed, checked_inputs=checked)
