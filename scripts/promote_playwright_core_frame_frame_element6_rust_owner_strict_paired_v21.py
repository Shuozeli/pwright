"""Plan or exclusively publish the strict Core frameElement v21 successor.

The v20 bundle is immutable historical evidence. Four complete candidate
reruns are required: an observed first-run frameset navigation failure, then
three complete GREEN runs. Only the six v4 rows are promoted; Oracle bytes and
all off-target candidate rows are inherited unchanged.
"""

from __future__ import annotations

import argparse
import ctypes
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

import verify_playwright_core_frame_frame_element6_rust_owner_strict_paired_v21 as independent


def prepare_pair(inputs: dict[str, Any] | None = None) -> dict[str, bytes]:
    checked = inputs if inputs is not None else independent.verify_inputs()
    candidate_lines = list(checked["oldLines"]["candidate"])
    for index, inventory_row in checked["selected"].items():
        candidate_lines[index] = independent.canonical_line(
            independent.expected_candidate_row(
                independent.object_from(
                    candidate_lines[index], "v20 candidate frame RED row"
                ),
                inventory_row["specId"],
            )
        )
    candidate_ledger = b"".join(candidate_lines)
    files = {
        "candidate.jsonl": candidate_ledger,
        "oracle.jsonl": checked["oldFiles"]["oracle.jsonl"],
        "oracle.summary.json": checked["oldFiles"]["oracle.summary.json"],
    }
    candidate_summary = independent.expected_candidate_summary(
        checked["oldSummaries"]["candidate"], candidate_ledger
    )
    files["candidate.summary.json"] = (
        json.dumps(candidate_summary, indent=2) + "\n"
    ).encode()
    receipt = independent.expected_receipt(files, checked)
    files["promotion-receipt.json"] = (
        json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    ).encode()
    independent.verify_payloads(files, checked)
    return files


def _publish_exclusive(staging: Path, destination: Path) -> None:
    libc = ctypes.CDLL(None, use_errno=True)
    if sys.platform == "darwin":
        rename = libc.renameatx_np
        flag, at_fdcwd = 0x4, -2
    elif sys.platform.startswith("linux"):
        rename = libc.renameat2
        flag, at_fdcwd = 0x1, -100
    else:
        raise ValueError("atomic no-replace directory publication is unavailable")
    rename.argtypes = [
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_uint,
    ]
    rename.restype = ctypes.c_int
    if (
        rename(at_fdcwd, os.fsencode(staging), at_fdcwd, os.fsencode(destination), flag)
        != 0
    ):
        raise ValueError(
            f"atomic no-replace publication failed: {os.strerror(ctypes.get_errno())}"
        )


def publish(files: dict[str, bytes]) -> None:
    independent.require(
        not independent.V21.exists() and not independent.V21.is_symlink(),
        "immutable v21 successor already exists",
    )
    independent.verify_payloads(files)
    independent.V21.parent.mkdir(parents=True, exist_ok=True)
    staging = Path(
        tempfile.mkdtemp(
            prefix=".core-frame-frame-element6-rust-owner-v21-",
            dir=independent.V21.parent,
        )
    )
    try:
        for name, payload in files.items():
            with (staging / name).open("xb") as stream:
                stream.write(payload)
                stream.flush()
                os.fsync(stream.fileno())
        directory_fd = os.open(staging, os.O_RDONLY)
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)
        _publish_exclusive(staging, independent.V21)
        parent_fd = os.open(independent.V21.parent, os.O_RDONLY)
        try:
            os.fsync(parent_fd)
        finally:
            os.close(parent_fd)
    finally:
        if staging.exists():
            shutil.rmtree(staging)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--plan", action="store_true")
    action.add_argument("--publish", action="store_true")
    action.add_argument("--verify-existing", action="store_true")
    args = parser.parse_args()
    try:
        if args.verify_existing:
            receipt = independent.verify_existing()
            result = {
                "published": True,
                "scope": receipt["scope"],
                "destination": str(independent.V21),
            }
        else:
            independent.require(
                not independent.V21.exists() and not independent.V21.is_symlink(),
                "immutable v21 successor already exists",
            )
            files = prepare_pair()
            result = {
                "published": False,
                "scope": independent.SCOPE,
                "destination": str(independent.V21),
                "sha256": {
                    name: independent.digest(payload)
                    for name, payload in sorted(files.items())
                },
                "outcomes": {
                    lane: json.loads(files[f"{lane}.summary.json"])["outcomes"]
                    for lane in independent.LANES
                },
            }
            if args.publish:
                independent.require(
                    files == prepare_pair(), "evidence changed before publication"
                )
                publish(files)
                independent.verify_existing()
                result["published"] = True
    except (
        OSError,
        ValueError,
        KeyError,
        TypeError,
        StopIteration,
        json.JSONDecodeError,
        subprocess.CalledProcessError,
    ) as error:
        parser.exit(
            2, f"strict paired Core frameElement v21 promotion rejected: {error}\n"
        )
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
