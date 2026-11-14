"""Pure mutation and archive-safety controls for the v8 ledger-only snapshot."""

from __future__ import annotations

import gzip
import io
import json
import tarfile
import tempfile
import unittest
from pathlib import Path

from scripts import package_playwright_core_v8_ledger_snapshot as snapshot


def encoded(value: dict[str, object]) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def synthetic_rows(
    lane: str,
) -> tuple[list[bytes], list[bytes], list[bytes], dict[int, tuple[str, str, str]], dict[str, str]]:
    """Six real target identities in a seven-row miniature inventory."""
    source = {"specId": "untouched", "sourceClassification": "public"}
    old = {**source, "attempts": 0, "outcome": "not-run"}
    inventory = [encoded(source)]
    predecessor = [encoded(old)]
    successor = [encoded(old)]
    targets = {index: value for index, value in enumerate(snapshot.TARGETS.values(), 1)}
    results = {value[1]: f"/historical/{lane}/{value[1]}.json" for value in targets.values()}
    for spec_id, project, kind in targets.values():
        source = snapshot.target_source(spec_id, project, kind)
        old = {**source, "attempts": 0, "outcome": "not-run"}
        new = {
            **old,
            "attempts": 1,
            "candidateRuntimeRequired": lane == "candidate",
            "evidenceScope": snapshot.V8_SCOPE,
            "outcome": "expected",
            "resultFile": results[project],
        }
        inventory.append(encoded(source))
        predecessor.append(encoded(old))
        successor.append(encoded(new))
    return inventory, predecessor, successor, targets, results


class LedgerReplayTests(unittest.TestCase):
    def test_pinned_local_source_replays_all_rows_in_both_lanes(self) -> None:
        # Arrange
        root = snapshot.ROOT

        # Act
        report = snapshot.verify_source(root)

        # Assert
        self.assertEqual(len(snapshot.INPUTS), 11)
        self.assertEqual(report["inventoryRows"], 48_477)
        self.assertEqual(report["targetZeroBasedIndexes"], list(range(5360, 5366)))
        self.assertEqual(report["outcomes"]["candidate"]["expected"], 4575)
        self.assertEqual(report["outcomes"]["oracle"]["expected"], 4324)
        self.assertFalse(report["publicApiParityEvidence"])

    def test_synthetic_six_row_transition_replays_in_both_lanes(self) -> None:
        for lane in snapshot.LANES:
            with self.subTest(lane=lane):
                # Arrange
                inventory, old, new, targets, results = synthetic_rows(lane)

                # Act
                report = snapshot.replay_lane(
                    inventory,
                    old,
                    new,
                    lane=lane,
                    results_by_project=results,
                    expected_count=7,
                    targets=targets,
                )

                # Assert
                self.assertEqual(report["rows"], 7)
                self.assertEqual(report["oldOutcomes"], {"not-run": 7})
                self.assertEqual(report["newOutcomes"], {"expected": 6, "not-run": 1})
                self.assertEqual(len(report["changedRows"]), 6)
                self.assertEqual(report["newScopes"], {snapshot.V8_SCOPE: 6})

    def test_rejects_non_target_raw_byte_change(self) -> None:
        # Arrange
        inventory, old, new, targets, results = synthetic_rows("candidate")
        new[0] = new[0].replace(b"not-run", b"expected")

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "non-target row 0 changed"):
            snapshot.replay_lane(
                inventory,
                old,
                new,
                lane="candidate",
                results_by_project=results,
                expected_count=7,
                targets=targets,
            )

    def test_rejects_target_classification_change_even_if_old_row_agrees(self) -> None:
        # Arrange
        inventory, old, new, targets, results = synthetic_rows("candidate")
        source = json.loads(inventory[1])
        source["sourceClassification"] = "public"
        inventory[1] = encoded(source)
        old[1] = encoded({**source, "attempts": 0, "outcome": "not-run"})

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "target identity or classification"):
            snapshot.replay_lane(
                inventory,
                old,
                new,
                lane="candidate",
                results_by_project=results,
                expected_count=7,
                targets=targets,
            )

    def test_rejects_target_result_path_change(self) -> None:
        # Arrange
        inventory, old, new, targets, results = synthetic_rows("candidate")
        value = json.loads(new[1])
        value["resultFile"] = "/other/result.json"
        new[1] = encoded(value)

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "canonical transition"):
            snapshot.replay_lane(
                inventory,
                old,
                new,
                lane="candidate",
                results_by_project=results,
                expected_count=7,
                targets=targets,
            )

    def test_rejects_missing_or_unterminated_row(self) -> None:
        # Arrange
        inventory, old, new, targets, results = synthetic_rows("oracle")

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "row count differs"):
            snapshot.replay_lane(
                inventory,
                old,
                new[:-1],
                lane="oracle",
                results_by_project=results,
                expected_count=7,
                targets=targets,
            )
        new[-1] = new[-1].rstrip(b"\n")
        with self.assertRaisesRegex(ValueError, "unterminated row"):
            snapshot.replay_lane(
                inventory,
                old,
                new,
                lane="oracle",
                results_by_project=results,
                expected_count=7,
                targets=targets,
            )

    def test_rejects_summary_outcome_drift(self) -> None:
        # Arrange
        expected = {
            "schema": 1,
            "inventory": "/historical/inventory.jsonl",
            "ledger": "/historical/candidate.jsonl",
            "ledgerSha256": "a" * 64,
            "records": snapshot.COUNT,
            "outcomes": {"expected": 6},
            "explicitEvidenceScopes": {snapshot.V8_SCOPE: 6},
        }
        changed = {**expected, "outcomes": {"expected": 7}}

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "summary disagrees"):
            snapshot.verify_summary(
                changed,
                inventory=expected["inventory"],
                ledger=expected["ledger"],
                ledger_sha256=expected["ledgerSha256"],
                outcomes=expected["outcomes"],
                scopes=expected["explicitEvidenceScopes"],
            )

    def test_rejects_parity_claim_in_internal_receipt(self) -> None:
        # Arrange
        receipt = {
            "schema": 1,
            "scope": snapshot.V8_SCOPE,
            "promotionKind": "implementation-internal-diagnostic",
            "fixtureAdapted": True,
            "sourceClassificationPreserved": "implementation-internal",
            "diagnosticOnly": True,
            "publicApiParityEvidence": True,
            "browserExecutionEvidence": False,
            "bidiProtocolParityEvidence": False,
            "currentLivePackageEvidence": False,
            "recordsPerLane": snapshot.COUNT,
            "promotedRowsPerLane": 6,
            "lanes": {lane: {} for lane in snapshot.LANES},
        }

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "no-parity flags"):
            snapshot.require_no_parity(receipt, snapshot.V8_SCOPE)


class ArchiveSafetyTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="pwright-core-v8-safety-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def archive(self, members: list[tuple[str, bytes]]) -> Path:
        path = self.root / "fixture.tar.gz"
        with tarfile.open(path, "w:gz", format=tarfile.USTAR_FORMAT) as archive:
            for name, member_type in members:
                member = tarfile.TarInfo(name)
                member.type = member_type
                if member_type == tarfile.REGTYPE:
                    member.size = 1
                    archive.addfile(member, io.BytesIO(b"x"))
                else:
                    member.linkname = "../outside"
                    archive.addfile(member)
        return path

    def extract(
        self,
        archive: Path,
        allowed: frozenset[str],
        *,
        size_bounds: dict[str, tuple[int, int]] | None = None,
    ) -> Path:
        destination = self.root / "out"
        destination.mkdir()
        snapshot.extract_exact_archive(archive, destination, allowed, size_bounds=size_bounds)
        return destination

    def test_extracts_only_allowlisted_regular_file(self) -> None:
        # Arrange
        archive = self.archive([("nested/data.txt", tarfile.REGTYPE)])

        # Act
        destination = self.extract(archive, frozenset({"nested/data.txt"}))

        # Assert
        self.assertEqual((destination / "nested/data.txt").read_bytes(), b"x")

    def test_rejects_parent_traversal(self) -> None:
        # Arrange
        archive = self.archive([("../outside", tarfile.REGTYPE)])

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "unsafe archive member path"):
            self.extract(archive, frozenset({"../outside"}))
        self.assertFalse((self.root / "outside").exists())

    def test_rejects_absolute_member_path(self) -> None:
        # Arrange
        archive = self.archive([("/outside", tarfile.REGTYPE)])

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "unsafe archive member path"):
            self.extract(archive, frozenset({"/outside"}))

    def test_rejects_symlink_member(self) -> None:
        # Arrange
        archive = self.archive([("data.txt", tarfile.SYMTYPE)])

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "non-regular member"):
            self.extract(archive, frozenset({"data.txt"}))

    def test_rejects_hardlink_member(self) -> None:
        # Arrange
        archive = self.archive([("data.txt", tarfile.LNKTYPE)])

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "non-regular member"):
            self.extract(archive, frozenset({"data.txt"}))

    def test_rejects_directory_member(self) -> None:
        # Arrange
        archive = self.archive([("nested/", tarfile.DIRTYPE)])

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "unsafe archive member path"):
            self.extract(archive, frozenset({"nested/"}))

    def test_rejects_duplicate_member(self) -> None:
        # Arrange
        archive = self.archive([("data.txt", tarfile.REGTYPE)] * 2)

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "duplicate archive member"):
            self.extract(archive, frozenset({"data.txt"}))

    def test_rejects_extra_member(self) -> None:
        # Arrange
        archive = self.archive([("data.txt", tarfile.REGTYPE), ("extra.txt", tarfile.REGTYPE)])

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "unexpected archive member"):
            self.extract(archive, frozenset({"data.txt"}))

    def test_rejects_missing_member(self) -> None:
        # Arrange
        archive = self.archive([("data.txt", tarfile.REGTYPE)])

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "file set differs"):
            self.extract(archive, frozenset({"data.txt", "other.txt"}))

    def test_rejects_wrong_declared_sealed_size_before_writing(self) -> None:
        # Arrange
        archive = self.archive([("data.txt", tarfile.REGTYPE)])

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "declared size differs"):
            self.extract(
                archive,
                frozenset({"data.txt"}),
                size_bounds={"data.txt": (2, 2)},
            )
        self.assertFalse((self.root / "out/data.txt").exists())

    def test_rejects_oversized_sidecar_declaration_before_writing(self) -> None:
        # Arrange
        archive = self.root / "oversized-sidecar.tar.gz"
        member = tarfile.TarInfo("snapshot.json")
        member.size = snapshot.MAX_SIDECAR_BYTES + 1
        with gzip.open(archive, "wb") as stream:
            stream.write(member.tobuf(format=tarfile.USTAR_FORMAT))
            stream.write(bytes(2 * snapshot.TAR_BLOCK_BYTES))

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "declared size differs"):
            self.extract(
                archive,
                frozenset({"snapshot.json"}),
                size_bounds={"snapshot.json": (1, snapshot.MAX_SIDECAR_BYTES)},
            )
        self.assertFalse((self.root / "out/snapshot.json").exists())

    def test_rejects_declared_expansion_before_reading_payload(self) -> None:
        # Arrange
        archive = self.root / "large-declaration.tar.gz"
        member = tarfile.TarInfo("data.txt")
        member.size = snapshot.MAX_EXPANDED_BYTES + 1
        with gzip.open(archive, "wb") as stream:
            stream.write(member.tobuf(format=tarfile.USTAR_FORMAT))
            stream.write(bytes(2 * snapshot.TAR_BLOCK_BYTES))

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "expanded archive exceeds size limit"):
            self.extract(archive, frozenset({"data.txt"}))

    def test_rejects_corrupt_gzip_trailer(self) -> None:
        # Arrange
        archive = self.archive([("data.txt", tarfile.REGTYPE)])
        damaged = bytearray(archive.read_bytes())
        damaged[-1] ^= 1
        archive.write_bytes(damaged)

        # Act / Assert
        with self.assertRaises((gzip.BadGzipFile, EOFError)):
            self.extract(archive, frozenset({"data.txt"}))

    def test_rejects_nonzero_ustar_trailer(self) -> None:
        # Arrange
        payload = io.BytesIO()
        with tarfile.open(fileobj=payload, mode="w", format=tarfile.USTAR_FORMAT) as tar:
            member = tarfile.TarInfo("data.txt")
            member.size = 1
            tar.addfile(member, io.BytesIO(b"x"))
        archive = self.root / "nonzero-trailer.tar.gz"
        with gzip.open(archive, "wb") as stream:
            stream.write(payload.getvalue() + b"x")

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "non-zero or oversized USTAR trailer"):
            self.extract(archive, frozenset({"data.txt"}))

    def test_rejects_symlinked_source_component(self) -> None:
        # Arrange
        outside = self.root / "outside"
        outside.mkdir()
        (outside / "data.txt").write_text("x", encoding="utf-8")
        package = self.root / "package"
        package.mkdir()
        (package / "nested").symlink_to(outside, target_is_directory=True)

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "symlinked package input"):
            snapshot.regular_input(package, Path("nested/data.txt"))

    def test_rejects_symlinked_archive(self) -> None:
        # Arrange
        archive = self.archive([("data.txt", tarfile.REGTYPE)])
        alias = self.root / "alias.tar.gz"
        alias.symlink_to(archive)

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "archive must be a regular file"):
            self.extract(alias, frozenset({"data.txt"}))

    def test_rejects_wrong_archive_sha_before_extraction(self) -> None:
        # Arrange
        archive = self.archive([("data.txt", tarfile.REGTYPE)])

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "archive SHA-256 mismatch"):
            snapshot.verify_archive(archive, "0" * 64)


if __name__ == "__main__":
    unittest.main()
