"""Plan or exclusively publish the strict Core page-dialog v15 pair.

Both unchanged official full-file reports are retained as evidence. Replace
only the ten source-pinned Chromium dialog rows in each lane, preserving every
off-target v14 JSONL line byte for byte. The independent verifier owns all
evidence and payload acceptance checks.
"""

from __future__ import annotations

import argparse
import ctypes
import json
import os
import shutil
import sys
import tempfile
from pathlib import Path
from typing import Any

import verify_playwright_core_page_dialog10_strict_paired_v15 as independent


def prepare_pair(inputs: dict[str, Any] | None = None) -> dict[str, bytes]:
    checked = inputs if inputs is not None else independent.verify_inputs()
    files: dict[str, bytes] = {}
    for lane in independent.LANES:
        old_lines = checked["oldLines"][lane]
        replacements = checked["replacementRows"][lane]
        independent.require(
            len(replacements) == 10,
            f"{lane} must replace exactly 10 source-pinned dialog rows",
        )
        new_lines = list(old_lines)
        for index, row in replacements.items():
            independent.require(
                isinstance(index, int) and 0 <= index < len(new_lines),
                f"{lane} replacement index is outside the ledger",
            )
            new_lines[index] = independent.canonical_line(row)
        files[f"{lane}.jsonl"] = b"".join(new_lines)
        summary = independent.expected_summary(
            lane, checked["oldSummaries"][lane], files[f"{lane}.jsonl"]
        )
        files[f"{lane}.summary.json"] = (json.dumps(summary, indent=2) + "\n").encode()
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
        not independent.V15.exists() and not independent.V15.is_symlink(),
        "immutable v15 successor already exists",
    )
    independent.verify_payloads(files)
    independent.V15.parent.mkdir(parents=True, exist_ok=True)
    staging = Path(
        tempfile.mkdtemp(prefix=".core-page-dialog10-v15-", dir=independent.V15.parent)
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
        _publish_exclusive(staging, independent.V15)
        parent_fd = os.open(independent.V15.parent, os.O_RDONLY)
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
                "destination": str(independent.V15),
            }
        else:
            independent.require(
                not independent.V15.exists() and not independent.V15.is_symlink(),
                "immutable v15 successor already exists",
            )
            files = prepare_pair()
            result = {
                "published": False,
                "scope": independent.SCOPE,
                "destination": str(independent.V15),
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
    ) as error:
        parser.exit(2, f"strict paired Core dialog v15 promotion rejected: {error}\n")
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
