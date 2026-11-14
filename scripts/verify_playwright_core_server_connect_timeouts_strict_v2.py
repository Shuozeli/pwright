"""Independently reverify the isolated-addon v2 Core timeout diagnostic."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import run_playwright_core_server_connect_timeouts_strict_v2 as v2
from verify_playwright_core_server_connect_timeouts_strict_v1 import verify_directory


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("result_dir", type=Path)
    args = parser.parse_args()
    try:
        v2.configure()
        attestation = verify_directory(args.result_dir)
    except (
        AssertionError,
        KeyError,
        OSError,
        TypeError,
        ValueError,
        json.JSONDecodeError,
    ) as error:
        parser.error(f"strict v2 timeout evidence invalid: {error}")
    for mode in ("oracle", "candidate"):
        lane = attestation["lanes"][mode]
        print(
            f"verified {mode}: {lane['passed']}/36 expected, {lane['unexpected']} unexpected"
        )


if __name__ == "__main__":
    main()
