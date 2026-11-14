"""Freeze a candidate package for source-stable official Core evidence."""

from __future__ import annotations

import ctypes
import hashlib
import json
import os
import re
import shutil
import sys
import tempfile
from pathlib import Path
from typing import Any

from run_playwright_official_conformance import _candidate_source_sha256


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _regular(path: Path) -> None:
    if not path.is_file() or path.is_symlink():
        raise ValueError(f"candidate snapshot file is missing or a symlink: {path}")


def _relative_files(package: Path, profile: str) -> list[Path]:
    if package.is_symlink() or not package.is_dir():
        raise ValueError("candidate package is missing or a symlink")
    source = package / "src"
    if source.is_symlink() or not source.is_dir():
        raise ValueError("candidate src is missing or a symlink")
    package_json = package / "package.json"
    artifact = package / "native" / profile / "pwright.node"
    _regular(package_json)
    _regular(artifact)
    files = [Path("package.json")]
    for path in source.rglob("*"):
        if path.is_symlink():
            raise ValueError(f"candidate src contains a symlink: {path}")
        if path.is_file():
            files.append(path.relative_to(package))
        elif not path.is_dir():
            raise ValueError(f"candidate src contains a non-file: {path}")
    if len(files) == 1:
        raise ValueError("candidate src is empty")
    files.append(Path("native") / profile / "pwright.node")
    return sorted(files, key=Path.as_posix)


def _record(package: Path, relative: Path) -> dict[str, Any]:
    path = package / relative
    _regular(path)
    return {"path": relative.as_posix(), "size": path.stat().st_size, "sha256": sha256(path)}


def _publish_directory_exclusive(temporary: Path, snapshot: Path) -> None:
    if sys.platform == "darwin":
        libc = ctypes.CDLL(None, use_errno=True)
        renamex_np = libc.renamex_np
        renamex_np.argtypes = (ctypes.c_char_p, ctypes.c_char_p, ctypes.c_uint)
        renamex_np.restype = ctypes.c_int
        if renamex_np(os.fsencode(temporary), os.fsencode(snapshot), 0x00000004) != 0:
            error_number = ctypes.get_errno()
            raise OSError(error_number, os.strerror(error_number), str(snapshot))
        return
    # Non-Darwin fallback claims the destination exclusively. The manifest is
    # published last, so a partial directory is never accepted as evidence.
    snapshot.mkdir(exist_ok=False)
    for child in temporary.iterdir():
        shutil.move(str(child), str(snapshot / child.name))
    temporary.rmdir()


def _publish_manifest_exclusive(manifest_path: Path, manifest: dict[str, Any]) -> None:
    with tempfile.NamedTemporaryFile(
        mode="w",
        encoding="utf-8",
        dir=manifest_path.parent,
        prefix=f".{manifest_path.name}.",
        suffix=".tmp",
        delete=False,
    ) as stream:
        temporary = Path(stream.name)
        json.dump(manifest, stream, indent=2, sort_keys=True)
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())
    try:
        os.link(temporary, manifest_path)
    finally:
        temporary.unlink(missing_ok=True)


def verify_snapshot(
    snapshot: Path,
    manifest_path: Path,
    *,
    source_package: Path,
    profile: str,
    artifact_sha256: str,
) -> dict[str, Any]:
    if not re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", profile):
        raise ValueError("candidate snapshot profile is invalid")
    if not re.fullmatch(r"[0-9a-f]{64}", artifact_sha256):
        raise ValueError("candidate snapshot artifact SHA256 is invalid")
    if snapshot.is_symlink() or not snapshot.is_dir():
        raise ValueError("candidate snapshot package is missing or a symlink")
    _regular(manifest_path)
    manifest = json.loads(manifest_path.read_text())
    if not isinstance(manifest, dict) or set(manifest) != {
        "schema",
        "sourcePackage",
        "snapshotPackage",
        "profile",
        "artifactSha256",
        "candidateSourceSha256",
        "files",
    }:
        raise ValueError("candidate snapshot manifest has unexpected fields")
    if (
        manifest["schema"] != 1
        or manifest["sourcePackage"] != str(source_package)
        or manifest["snapshotPackage"] != str(snapshot)
        or manifest["profile"] != profile
        or manifest["artifactSha256"] != artifact_sha256
    ):
        raise ValueError("candidate snapshot manifest identity mismatch")
    files = manifest["files"]
    if not isinstance(files, list) or not files:
        raise ValueError("candidate snapshot manifest files are missing")
    paths = []
    for record in files:
        if not isinstance(record, dict) or set(record) != {"path", "size", "sha256"}:
            raise ValueError("candidate snapshot manifest file record is invalid")
        relative = record["path"]
        if (
            not isinstance(relative, str)
            or not relative
            or Path(relative).is_absolute()
            or any(part in ("", ".", "..") for part in relative.split("/"))
            or Path(relative).as_posix() != relative
        ):
            raise ValueError("candidate snapshot manifest path is unsafe")
        if not isinstance(record["size"], int) or record["size"] < 0:
            raise ValueError("candidate snapshot manifest size is invalid")
        if not isinstance(record["sha256"], str) or not re.fullmatch(
            r"[0-9a-f]{64}", record["sha256"]
        ):
            raise ValueError("candidate snapshot manifest digest is invalid")
        path = snapshot / relative
        _regular(path)
        if path.stat().st_size != record["size"] or sha256(path) != record["sha256"]:
            raise ValueError(f"candidate snapshot file changed: {relative}")
        paths.append(relative)
    if paths != sorted(set(paths)):
        raise ValueError("candidate snapshot files are not uniquely sorted")
    actual = []
    for path in snapshot.rglob("*"):
        if path.is_symlink():
            raise ValueError(f"candidate snapshot contains a symlink: {path}")
        if path.is_file():
            actual.append(path.relative_to(snapshot).as_posix())
        elif not path.is_dir():
            raise ValueError(f"candidate snapshot contains a non-file: {path}")
    if sorted(actual) != paths:
        raise ValueError("candidate snapshot has missing or extra files")
    expected_layout = [relative.as_posix() for relative in _relative_files(snapshot, profile)]
    if expected_layout != paths:
        raise ValueError(
            "candidate snapshot includes files outside package.json, src, and artifact"
        )
    digest = _candidate_source_sha256(snapshot)
    if digest != manifest["candidateSourceSha256"]:
        raise ValueError("candidate snapshot source digest changed")
    if sha256(snapshot / "native" / profile / "pwright.node") != artifact_sha256:
        raise ValueError("candidate snapshot artifact digest changed")
    return manifest


def create_snapshot(
    source_package: Path,
    snapshot: Path,
    manifest_path: Path,
    *,
    profile: str,
    artifact_sha256: str,
) -> dict[str, Any]:
    if not re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", profile):
        raise ValueError("candidate snapshot profile is invalid")
    if not re.fullmatch(r"[0-9a-f]{64}", artifact_sha256):
        raise ValueError("candidate snapshot artifact SHA256 is invalid")
    if (
        snapshot.exists()
        or snapshot.is_symlink()
        or manifest_path.exists()
        or manifest_path.is_symlink()
    ):
        raise ValueError("candidate snapshot destination is not empty")
    source_files = _relative_files(source_package, profile)
    before_source = _candidate_source_sha256(source_package)
    before_artifact = sha256(source_package / "native" / profile / "pwright.node")
    if before_artifact != artifact_sha256:
        raise ValueError("candidate source artifact differs from pinned SHA256")
    snapshot.parent.mkdir(parents=True, exist_ok=True)
    temporary = Path(tempfile.mkdtemp(prefix=f".{snapshot.name}.", dir=snapshot.parent))
    published = False
    try:
        for relative in source_files:
            destination = temporary / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source_package / relative, destination)
        after_source = _candidate_source_sha256(source_package)
        after_artifact = sha256(source_package / "native" / profile / "pwright.node")
        if (before_source, before_artifact) != (after_source, after_artifact):
            raise ValueError("candidate source changed during snapshot copy")
        if _candidate_source_sha256(temporary) != before_source:
            raise ValueError("candidate snapshot source differs from copied source")
        if sha256(temporary / "native" / profile / "pwright.node") != artifact_sha256:
            raise ValueError("candidate snapshot artifact differs from copied source")
        manifest = {
            "schema": 1,
            "sourcePackage": str(source_package),
            "snapshotPackage": str(snapshot),
            "profile": profile,
            "artifactSha256": artifact_sha256,
            "candidateSourceSha256": before_source,
            "files": [_record(temporary, relative) for relative in source_files],
        }
        if snapshot.exists() or snapshot.is_symlink():
            raise ValueError("candidate snapshot destination appeared during copy")
        _publish_directory_exclusive(temporary, snapshot)
        published = True
        # If manifest publication fails, the orphaned snapshot remains visible
        # but is never accepted as evidence; the fixed v3 destination cannot be
        # reused without an explicit new evidence version.
        _publish_manifest_exclusive(manifest_path, manifest)
        verify_snapshot(
            snapshot,
            manifest_path,
            source_package=source_package,
            profile=profile,
            artifact_sha256=artifact_sha256,
        )
        return manifest
    finally:
        if not published:
            shutil.rmtree(temporary)
