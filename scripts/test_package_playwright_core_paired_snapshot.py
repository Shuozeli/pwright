"""Mutation controls for the relocatable 16-input Core outcome snapshot."""

from __future__ import annotations

import gzip
import hashlib
import io
import json
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path
from typing import cast

from scripts import package_playwright_core_paired_snapshot as snapshot


class PairedSnapshotTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.temporary = tempfile.TemporaryDirectory(
            prefix="pwright-core-snapshot-test-"
        )
        cls.bundle = Path(cls.temporary.name) / "bundle"
        cls.report = snapshot.pack_bundle(snapshot.coverage.ROOT, cls.bundle)
        cls.archive = Path(cls.temporary.name) / "snapshot.tar.gz"
        archived_report, cls.archive_sha256, cls.archive_bytes = snapshot.pack_archive(
            cls.bundle, cls.archive
        )
        assert archived_report == cls.report

    @classmethod
    def tearDownClass(cls) -> None:
        cls.temporary.cleanup()

    def test_new_bundle_replays_pinned_full_row_audit(self) -> None:
        # Arrange
        bundle = self.bundle

        # Act
        report = snapshot.verify_bundle(bundle)

        # Assert
        self.assertEqual(
            cast(dict[str, object], report["summary"])["inventoryRows"], 48_477
        )
        self.assertEqual(report, self.report)
        self.assertEqual(len(snapshot.INPUTS), 16)

    def test_copied_bundle_verifies_at_another_absolute_path(self) -> None:
        # Arrange
        with tempfile.TemporaryDirectory(prefix="pwright-core-relocated-") as parent:
            relocated = Path(parent) / "relocated"
            shutil.copytree(self.bundle, relocated)

            # Act
            report = snapshot.verify_bundle(relocated)

            # Assert
            self.assertEqual(report, self.report)

    def test_verifier_runs_from_clean_directory_without_original_repo(self) -> None:
        # Arrange
        with tempfile.TemporaryDirectory(prefix="pwright-core-clean-") as parent:
            clean = Path(parent)
            scripts = clean / "scripts"
            scripts.mkdir()
            verifier = scripts / "package_playwright_core_paired_snapshot.py"
            auditor = scripts / "report_playwright_core_adapter_coverage.py"
            shutil.copyfile(Path(snapshot.__file__), verifier)
            shutil.copyfile(Path(snapshot.coverage.__file__), auditor)
            relocated = clean / "relocated"
            shutil.copytree(self.bundle, relocated)

            # Act
            completed = subprocess.run(
                [sys.executable, str(verifier), "verify", "--bundle", str(relocated)],
                cwd=clean,
                capture_output=True,
                text=True,
                check=False,
            )

            # Assert
            self.assertEqual(completed.returncode, 0, completed.stderr)
            self.assertEqual(json.loads(completed.stdout)["inventoryRows"], 48_477)

    def test_rejects_extra_package_entry(self) -> None:
        # Arrange
        extra = self.bundle / "extra.txt"
        extra.write_text("unaccounted", encoding="utf-8")
        try:
            # Act / Assert
            with self.assertRaisesRegex(ValueError, "file set differs"):
                snapshot.verify_bundle(self.bundle)
        finally:
            extra.unlink()

    def test_rejects_symlinked_input(self) -> None:
        # Arrange
        relative = snapshot.OFFICIAL / snapshot.coverage.DEFAULT_MANIFEST.name
        target = self.bundle / relative
        with tempfile.TemporaryDirectory(prefix="pwright-core-link-") as parent:
            saved = Path(parent) / "manifest.json"
            shutil.copyfile(target, saved)
            target.unlink()
            target.symlink_to(saved)
            try:
                # Act / Assert
                with self.assertRaisesRegex(ValueError, "symlinked package entry"):
                    snapshot.verify_bundle(self.bundle)
            finally:
                target.unlink()
                shutil.copyfile(saved, target)

    def test_rejects_rehashed_successor_manifest_drift(self) -> None:
        # Arrange
        relative = snapshot.OFFICIAL / snapshot.coverage.SUCCESSOR_MANIFEST.name
        target = self.bundle / relative
        sidecar = self.bundle / snapshot.SIDECAR
        original = target.read_bytes()
        original_sidecar = sidecar.read_bytes()
        changed = original.replace(
            snapshot.coverage.EXPECTED_COMMIT.encode(), b"0" * 40
        )
        self.assertNotEqual(changed, original)
        target.write_bytes(changed)
        metadata = json.loads(original_sidecar)
        entry = next(
            entry
            for entry in metadata["inputs"]
            if entry["path"] == relative.as_posix()
        )
        entry["bytes"] = len(changed)
        entry["sha256"] = hashlib.sha256(changed).hexdigest()
        sidecar.write_text(json.dumps(metadata), encoding="utf-8")
        try:
            # Act / Assert
            with self.assertRaisesRegex(ValueError, "provenance drifted"):
                snapshot.verify_bundle(self.bundle)
        finally:
            target.write_bytes(original)
            sidecar.write_bytes(original_sidecar)

    def test_rejects_rehashed_candidate_ledger_drift(self) -> None:
        # Arrange
        relative = (
            snapshot.OUTCOMES / snapshot.coverage.DEFAULT_PAIR.name / "candidate.jsonl"
        )
        target = self.bundle / relative
        sidecar = self.bundle / snapshot.SIDECAR
        original_sidecar = sidecar.read_bytes()
        with target.open("r+b") as ledger:
            first = ledger.read(1)
            self.assertEqual(first, b"{")
            ledger.seek(0)
            ledger.write(b"[")
        metadata = json.loads(original_sidecar)
        entry = next(
            entry
            for entry in metadata["inputs"]
            if entry["path"] == relative.as_posix()
        )
        entry["sha256"] = snapshot._sha256(target)
        sidecar.write_text(json.dumps(metadata), encoding="utf-8")
        try:
            # Act / Assert
            with self.assertRaisesRegex(
                ValueError, "ledger does not match paired receipt"
            ):
                snapshot.verify_bundle(self.bundle)
        finally:
            with target.open("r+b") as ledger:
                ledger.write(first)
            sidecar.write_bytes(original_sidecar)

    def _mutant_archive(self, members: list[tuple[str, bytes]]) -> Path:
        """Write a small hostile archive with no payload from the real ledger."""
        path = Path(self.temporary.name) / f"{self._testMethodName}.tar.gz"
        with tarfile.open(path, "w:gz") as archive:
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

    def test_archive_is_reproducible_from_same_payload(self) -> None:
        # Arrange
        second = Path(self.temporary.name) / "second-snapshot.tar.gz"

        # Act
        report, digest, size = snapshot.pack_archive(self.bundle, second)

        # Assert
        self.assertEqual(report, self.report)
        self.assertEqual(digest, self.archive_sha256)
        self.assertEqual(size, self.archive_bytes)
        self.assertEqual(second.read_bytes(), self.archive.read_bytes())

    def test_archive_verifies_when_relocated(self) -> None:
        # Arrange
        with tempfile.TemporaryDirectory(
            prefix="pwright-core-relocated-archive-"
        ) as parent:
            relocated = Path(parent) / "relocated.tar.gz"
            shutil.copyfile(self.archive, relocated)

            # Act
            report = snapshot.verify_archive(relocated, self.archive_sha256)

            # Assert
            self.assertEqual(report, self.report)

    def test_archive_verifier_runs_without_original_repo(self) -> None:
        # Arrange
        with tempfile.TemporaryDirectory(
            prefix="pwright-core-archive-clean-"
        ) as parent:
            clean = Path(parent)
            scripts = clean / "scripts"
            scripts.mkdir()
            verifier = scripts / "package_playwright_core_paired_snapshot.py"
            auditor = scripts / "report_playwright_core_adapter_coverage.py"
            shutil.copyfile(Path(snapshot.__file__), verifier)
            shutil.copyfile(Path(snapshot.coverage.__file__), auditor)
            relocated = clean / "snapshot.tar.gz"
            shutil.copyfile(self.archive, relocated)

            # Act
            completed = subprocess.run(
                [
                    sys.executable,
                    str(verifier),
                    "verify-archive",
                    "--archive",
                    str(relocated),
                    "--expected-sha256",
                    self.archive_sha256,
                ],
                cwd=clean,
                capture_output=True,
                text=True,
                check=False,
            )

            # Assert
            self.assertEqual(completed.returncode, 0, completed.stderr)
            result = json.loads(completed.stdout)
            self.assertEqual(result["archiveSha256"], self.archive_sha256)
            self.assertEqual(result["inventoryRows"], 48_477)

    def test_archive_rejects_wrong_external_digest(self) -> None:
        # Arrange
        wrong_digest = "0" * 64
        self.assertNotEqual(wrong_digest, self.archive_sha256)

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "archive SHA-256 mismatch"):
            snapshot.verify_archive(self.archive, wrong_digest)

    def test_archive_rejects_duplicate_member(self) -> None:
        # Arrange
        archive = self._mutant_archive([(snapshot.SIDECAR, tarfile.REGTYPE)] * 2)

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "duplicate archive member"):
            snapshot.verify_archive(archive, snapshot._sha256(archive))

    def test_archive_rejects_parent_traversal(self) -> None:
        # Arrange
        archive = self._mutant_archive([("../outside", tarfile.REGTYPE)])

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "unsafe archive member path"):
            snapshot.verify_archive(archive, snapshot._sha256(archive))

    def test_archive_rejects_absolute_path(self) -> None:
        # Arrange
        archive = self._mutant_archive([("/outside", tarfile.REGTYPE)])

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "unsafe archive member path"):
            snapshot.verify_archive(archive, snapshot._sha256(archive))

    def test_archive_rejects_symlink_member(self) -> None:
        # Arrange
        archive = self._mutant_archive([(snapshot.SIDECAR, tarfile.SYMTYPE)])

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "non-regular archive member"):
            snapshot.verify_archive(archive, snapshot._sha256(archive))

    def test_archive_rejects_hardlink_member(self) -> None:
        # Arrange
        archive = self._mutant_archive([(snapshot.SIDECAR, tarfile.LNKTYPE)])

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "non-regular archive member"):
            snapshot.verify_archive(archive, snapshot._sha256(archive))

    def test_archive_rejects_directory_member(self) -> None:
        # Arrange
        archive = self._mutant_archive([(snapshot.SIDECAR, tarfile.DIRTYPE)])

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "unsafe archive member path"):
            snapshot.verify_archive(archive, snapshot._sha256(archive))

    def test_archive_rejects_pax_metadata_member(self) -> None:
        # Arrange
        archive = self._mutant_archive([(snapshot.SIDECAR, tarfile.XHDTYPE)])

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "non-regular archive member"):
            snapshot.verify_archive(archive, snapshot._sha256(archive))

    def test_archive_rejects_gnu_metadata_member(self) -> None:
        # Arrange
        archive = Path(self.temporary.name) / "gnu-metadata.tar.gz"
        member = tarfile.TarInfo(snapshot.SIDECAR)
        member.type = tarfile.GNUTYPE_LONGNAME
        with gzip.open(archive, "wb") as destination:
            destination.write(member.tobuf(format=tarfile.GNU_FORMAT))
            destination.write(bytes(2 * snapshot.TAR_BLOCK_BYTES))

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "archive requires USTAR-only members"):
            snapshot.verify_archive(archive, snapshot._sha256(archive))

    def test_archive_rejects_oversized_declared_member_before_reading(self) -> None:
        # Arrange
        archive = Path(self.temporary.name) / "oversized-declared.tar.gz"
        member = tarfile.TarInfo(snapshot.SIDECAR)
        member.size = snapshot.MAX_EXPANDED_BYTES + 1
        with gzip.open(archive, "wb") as destination:
            destination.write(member.tobuf(format=tarfile.USTAR_FORMAT))
            destination.write(bytes(2 * snapshot.TAR_BLOCK_BYTES))

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "expanded archive exceeds size limit"):
            snapshot.verify_archive(archive, snapshot._sha256(archive))


if __name__ == "__main__":
    unittest.main()
