"""Copy and verify the pinned Core paired-ledger snapshot without rerunning tests.

The package contains the exact 16 inputs to the full Core adapter-coverage
audit. Historical absolute paths inside receipts remain opaque provenance;
only these fixed, relative package paths are opened. Verification replays the
48,477-row audit and its pinned receipt hash chain, not upstream test execution.
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import os
import re
import shutil
import stat
import tarfile
import tempfile
from pathlib import Path
from typing import cast

if __package__:
    from . import report_playwright_core_adapter_coverage as coverage
else:
    import report_playwright_core_adapter_coverage as coverage

SCHEMA = 1
KIND = "playwright-core-v1.58.0-paired-outcome-snapshot-not-reexecution"
SIDECAR = "snapshot.json"
OFFICIAL = Path("tests/official")
WORK = Path(".work/official-conformance")
OUTCOMES = WORK / "outcomes"
INPUTS = tuple(
    sorted(
        (
            OFFICIAL / coverage.DEFAULT_MANIFEST.name,
            OFFICIAL / coverage.STRICT_MANIFEST.name,
            OFFICIAL / coverage.SUCCESSOR_MANIFEST.name,
            *(OFFICIAL / path.name for path in coverage.COHORT_MANIFESTS.values()),
            WORK / coverage.DEFAULT_INVENTORY.name,
            *(
                OUTCOMES / pair / name
                for pair in (
                    coverage.BASE_PAIR_NAME,
                    coverage.PRIOR_PAIR_NAME,
                    coverage.DEFAULT_PAIR.name,
                )
                for name in (
                    "candidate.jsonl",
                    "oracle.jsonl",
                    "promotion-receipt.json",
                )
            ),
        ),
        key=lambda path: path.as_posix(),
    )
)
if len(INPUTS) != 16 or len(set(INPUTS)) != 16:
    raise RuntimeError("Core snapshot input closure must contain 16 unique files")
ARCHIVE_FILES = tuple(
    sorted((*INPUTS, Path(SIDECAR)), key=lambda path: path.as_posix())
)
ARCHIVE_NAMES = frozenset(path.as_posix() for path in ARCHIVE_FILES)
MAX_ARCHIVE_BYTES = 32 * 1024 * 1024
MAX_EXPANDED_BYTES = 256 * 1024 * 1024
MAX_TRAILER_BYTES = 10 * 1024
TAR_BLOCK_BYTES = 512


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _report_sha256(report: coverage.JsonObject) -> str:
    encoded = json.dumps(report, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(encoded).hexdigest()


def _regular_input(root: Path, relative: Path) -> Path:
    """Reject symlinks in every component, not only the final input file."""
    _require(root.is_dir() and not root.is_symlink(), f"unsafe package root: {root}")
    current = root
    parts = relative.parts
    _require(
        bool(parts) and not relative.is_absolute() and ".." not in parts,
        f"unsafe relative input: {relative}",
    )
    for index, part in enumerate(parts):
        current /= part
        try:
            mode = current.lstat().st_mode
        except OSError as error:
            raise ValueError(f"missing package input: {current}") from error
        _require(not stat.S_ISLNK(mode), f"symlinked package input: {current}")
        expected = stat.S_ISREG if index == len(parts) - 1 else stat.S_ISDIR
        _require(expected(mode), f"non-regular package input: {current}")
    return current


def _audit(root: Path) -> coverage.JsonObject:
    pair = root / OUTCOMES / coverage.DEFAULT_PAIR.name
    return coverage.audit(
        root / OFFICIAL / coverage.DEFAULT_MANIFEST.name,
        root / WORK / coverage.DEFAULT_INVENTORY.name,
        pair / "candidate.jsonl",
        pair / "oracle.jsonl",
        pair / "promotion-receipt.json",
        auxiliary_manifest_root=root / OFFICIAL,
    )


def _entry(path: Path, relative: Path) -> dict[str, object]:
    return {
        "path": relative.as_posix(),
        "bytes": path.stat().st_size,
        "sha256": _sha256(path),
    }


def _actual_tree(root: Path) -> set[str]:
    """Inventory every package entry, including hidden .work and symlinks."""
    actual: set[str] = set()
    for parent, directories, files in os.walk(root, followlinks=False):
        for name in (*directories, *files):
            path = Path(parent) / name
            _require(not path.is_symlink(), f"symlinked package entry: {path}")
            actual.add(path.relative_to(root).as_posix())
    return actual


def _expected_tree() -> set[str]:
    expected = {SIDECAR}
    for relative in INPUTS:
        expected.add(relative.as_posix())
        expected.update(
            parent.as_posix() for parent in relative.parents if parent != Path(".")
        )
    return expected


def verify_bundle(root: Path) -> coverage.JsonObject:
    """Fail closed on layout, bytes, receipt provenance, and every ledger row."""
    _require(root.is_dir() and not root.is_symlink(), f"unsafe package root: {root}")
    _require(
        _actual_tree(root) == _expected_tree(),
        "package file set differs from the 16-input closure",
    )
    sidecar = _regular_input(root, Path(SIDECAR))
    metadata = json.loads(sidecar.read_text(encoding="utf-8"))
    _require(isinstance(metadata, dict), "snapshot metadata must be an object")
    _require(
        set(metadata) == {"schema", "kind", "upstreamCommit", "inputs", "reportSha256"}
        and metadata.get("schema") == SCHEMA
        and metadata.get("kind") == KIND
        and metadata.get("upstreamCommit") == coverage.EXPECTED_COMMIT,
        "snapshot metadata identity drifted",
    )
    declared = metadata.get("inputs")
    _require(
        isinstance(declared, list) and len(declared) == len(INPUTS),
        "snapshot input index is incomplete",
    )
    observed = [_entry(_regular_input(root, relative), relative) for relative in INPUTS]
    _require(declared == observed, "snapshot input bytes or hashes drifted")
    report = _audit(root)
    _require(
        metadata.get("reportSha256") == _report_sha256(report),
        "snapshot audit report digest drifted",
    )
    return report


def pack_bundle(source_root: Path, output: Path) -> coverage.JsonObject:
    """Copy unchanged inputs to a new directory, then verify the copy."""
    _require(
        not output.exists() and not output.is_symlink(),
        f"output already exists: {output}",
    )
    _require(
        output.parent.is_dir() and not output.parent.is_symlink(),
        "output parent is missing or symlinked",
    )
    sources = [_regular_input(source_root, relative) for relative in INPUTS]
    report = _audit(source_root)
    source_entries = [
        _entry(path, relative) for path, relative in zip(sources, INPUTS, strict=True)
    ]
    with tempfile.TemporaryDirectory(
        prefix=f".{output.name}-", dir=output.parent
    ) as temporary:
        staged = Path(temporary) / "snapshot"
        staged.mkdir()
        for source, relative in zip(sources, INPUTS, strict=True):
            target = staged / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target, follow_symlinks=False)
        metadata = {
            "schema": SCHEMA,
            "kind": KIND,
            "upstreamCommit": coverage.EXPECTED_COMMIT,
            "inputs": source_entries,
            "reportSha256": _report_sha256(report),
        }
        (staged / SIDECAR).write_text(
            json.dumps(metadata, sort_keys=True, indent=2) + "\n", encoding="utf-8"
        )
        verified = verify_bundle(staged)
        _require(
            all(
                _entry(source, relative) == entry
                for source, relative, entry in zip(
                    sources, INPUTS, source_entries, strict=True
                )
            ),
            "source changed while packaging",
        )
        _require(
            not output.exists() and not output.is_symlink(),
            f"output appeared during packaging: {output}",
        )
        staged.rename(output)
    return verified


def _archive_member_name(name: str) -> Path:
    """Accept only canonical POSIX names from the fixed snapshot file closure."""
    parts = name.split("/")
    _require(
        not name.startswith("/")
        and "\\" not in name
        and all(part not in ("", ".", "..") for part in parts),
        f"unsafe archive member path: {name}",
    )
    _require(name in ARCHIVE_NAMES, f"unexpected archive member: {name}")
    return Path(*parts)


def _tar_octal(field: bytes, label: str) -> int:
    value = field.strip(b"\0 ")
    _require(
        bool(value) and all(digit in b"01234567" for digit in value),
        f"invalid USTAR {label}",
    )
    return int(value, 8)


def _read_exact(source: gzip.GzipFile, count: int, label: str) -> bytes:
    chunks: list[bytes] = []
    remaining = count
    while remaining:
        chunk = source.read(remaining)
        _require(bool(chunk), f"truncated USTAR {label}")
        chunks.append(chunk)
        remaining -= len(chunk)
    return b"".join(chunks)


def verify_archive(archive_path: Path, expected_sha256: str) -> coverage.JsonObject:
    """Verify pinned gzip/USTAR with bounded, manual extraction only."""
    _require(
        re.fullmatch(r"[a-f0-9]{64}", expected_sha256) is not None,
        "archive SHA-256 pin must be 64 lowercase hexadecimal characters",
    )
    try:
        mode = archive_path.lstat().st_mode
    except OSError as error:
        raise ValueError(f"missing archive: {archive_path}") from error
    _require(stat.S_ISREG(mode), f"archive is not a regular file: {archive_path}")
    _require(
        archive_path.stat().st_size <= MAX_ARCHIVE_BYTES,
        "compressed archive exceeds size limit",
    )
    with tempfile.TemporaryDirectory(prefix="pwright-core-archive-verify-") as temp:
        root = Path(temp) / "bundle"
        root.mkdir()
        archive_copy = Path(temp) / "pinned.tar.gz"
        copied = 0
        with archive_path.open("rb") as source, archive_copy.open("wb") as target:
            while chunk := source.read(1024 * 1024):
                copied += len(chunk)
                _require(
                    copied <= MAX_ARCHIVE_BYTES,
                    "compressed archive exceeds size limit",
                )
                target.write(chunk)
        _require(_sha256(archive_copy) == expected_sha256, "archive SHA-256 mismatch")
        seen: set[str] = set()
        expanded = 0
        with gzip.open(archive_copy, mode="rb") as source:
            while True:
                header = _read_exact(source, TAR_BLOCK_BYTES, "header")
                expanded += TAR_BLOCK_BYTES
                _require(
                    expanded <= MAX_EXPANDED_BYTES,
                    "expanded archive exceeds size limit",
                )
                if header == bytes(TAR_BLOCK_BYTES):
                    second = _read_exact(source, TAR_BLOCK_BYTES, "end marker")
                    _require(
                        second == bytes(TAR_BLOCK_BYTES),
                        "invalid USTAR end marker",
                    )
                    trailer = source.read(MAX_TRAILER_BYTES + 1)
                    _require(
                        len(trailer) <= MAX_TRAILER_BYTES
                        and not any(trailer)
                        and not source.read(1),
                        "non-zero or oversized USTAR trailer",
                    )
                    break
                _require(
                    header[257:263] == b"ustar\0" and header[263:265] == b"00",
                    "archive requires USTAR-only members",
                )
                expected_checksum = _tar_octal(header[148:156], "checksum")
                observed_checksum = sum(header[:148]) + 8 * 32 + sum(header[156:])
                _require(
                    expected_checksum == observed_checksum, "invalid USTAR checksum"
                )
                prefix = header[345:500].split(b"\0", 1)[0]
                basename = header[:100].split(b"\0", 1)[0]
                name = (prefix + b"/" if prefix else b"") + basename
                try:
                    member_name = name.decode("utf-8")
                except UnicodeDecodeError as error:
                    raise ValueError("non-UTF-8 USTAR member name") from error
                relative = _archive_member_name(member_name)
                _require(
                    header[156:157] == tarfile.REGTYPE,
                    f"non-regular archive member: {member_name}",
                )
                _require(
                    member_name not in seen, f"duplicate archive member: {member_name}"
                )
                seen.add(member_name)
                member_bytes = _tar_octal(header[124:136], "size")
                padding = (-member_bytes) % TAR_BLOCK_BYTES
                expanded += member_bytes + padding
                _require(
                    expanded <= MAX_EXPANDED_BYTES,
                    "expanded archive exceeds size limit",
                )
                target = root / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                with target.open("wb") as destination:
                    remaining = member_bytes
                    while remaining:
                        chunk = source.read(min(remaining, 1024 * 1024))
                        _require(bool(chunk), f"truncated USTAR member: {member_name}")
                        destination.write(chunk)
                        remaining -= len(chunk)
                if padding:
                    _require(
                        _read_exact(source, padding, "padding") == bytes(padding),
                        f"non-zero USTAR padding: {member_name}",
                    )
                _require(
                    target.stat().st_size == member_bytes,
                    f"archive member size drifted: {member_name}",
                )
        _require(
            seen == ARCHIVE_NAMES, "archive file set differs from the 17-file closure"
        )
        return verify_bundle(root)


def pack_archive(
    bundle_root: Path, output: Path
) -> tuple[coverage.JsonObject, str, int]:
    """Write a byte-stable gzip/USTAR archive of an already-verified bundle."""
    report = verify_bundle(bundle_root)
    _require(
        not output.exists() and not output.is_symlink(),
        f"output already exists: {output}",
    )
    _require(
        output.parent.is_dir() and not output.parent.is_symlink(),
        "output parent is missing or symlinked",
    )
    with tempfile.TemporaryDirectory(
        prefix=f".{output.name}-", dir=output.parent
    ) as temporary:
        staged = Path(temporary) / "snapshot.tar.gz"
        with (
            staged.open("wb") as destination,
            gzip.GzipFile(
                filename="", fileobj=destination, mode="wb", compresslevel=9, mtime=0
            ) as compressed,
            tarfile.open(
                fileobj=compressed, mode="w", format=tarfile.USTAR_FORMAT
            ) as tar,
        ):
            for relative in ARCHIVE_FILES:
                path = _regular_input(bundle_root, relative)
                member = tarfile.TarInfo(relative.as_posix())
                member.size = path.stat().st_size
                member.mode = 0o644
                member.mtime = 0
                member.uid = 0
                member.gid = 0
                member.uname = ""
                member.gname = ""
                with path.open("rb") as source:
                    tar.addfile(member, source)
        digest = _sha256(staged)
        verified = verify_archive(staged, digest)
        _require(verified == report, "archive report differs from source bundle")
        _require(
            not output.exists() and not output.is_symlink(),
            f"output appeared during packaging: {output}",
        )
        os.link(staged, output)
    return verified, digest, output.stat().st_size


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    pack = subparsers.add_parser("pack", help="copy the current pinned Core snapshot")
    pack.add_argument("--source-root", type=Path, default=coverage.ROOT)
    pack.add_argument("--output", type=Path, required=True)
    verify = subparsers.add_parser("verify", help="verify a relocated snapshot")
    verify.add_argument("--bundle", type=Path, required=True)
    archive_pack = subparsers.add_parser(
        "pack-archive", help="create a deterministic compressed snapshot archive"
    )
    archive_pack.add_argument("--bundle", type=Path, required=True)
    archive_pack.add_argument("--output", type=Path, required=True)
    archive_verify = subparsers.add_parser(
        "verify-archive", help="verify a hash-pinned compressed snapshot archive"
    )
    archive_verify.add_argument("--archive", type=Path, required=True)
    archive_verify.add_argument("--expected-sha256", required=True)
    args = parser.parse_args()
    archive_sha256 = None
    archive_bytes = None
    try:
        if args.command == "pack":
            report = pack_bundle(args.source_root, args.output)
        elif args.command == "verify":
            report = verify_bundle(args.bundle)
        elif args.command == "pack-archive":
            report, archive_sha256, archive_bytes = pack_archive(
                args.bundle, args.output
            )
        else:
            report = verify_archive(args.archive, args.expected_sha256)
            archive_sha256 = args.expected_sha256
            archive_bytes = args.archive.stat().st_size
    except (
        OSError,
        ValueError,
        TypeError,
        KeyError,
        EOFError,
        tarfile.TarError,
        json.JSONDecodeError,
    ) as error:
        parser.exit(2, f"Core paired snapshot rejected: {error}\n")
    result: dict[str, object] = {
        "kind": KIND,
        "upstreamCommit": coverage.EXPECTED_COMMIT,
        "inventoryRows": cast(dict[str, object], report["summary"])["inventoryRows"],
        "reportSha256": _report_sha256(report),
    }
    if archive_sha256 is not None:
        result["archiveSha256"] = archive_sha256
        result["archiveBytes"] = archive_bytes
    print(json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    main()
