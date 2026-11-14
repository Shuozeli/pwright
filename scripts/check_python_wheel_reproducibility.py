#!/usr/bin/env python3
"""Compare two wheels while normalizing the two CycloneDX identity fields."""

from __future__ import annotations

import csv
import io
import json
import sys
import zipfile
from pathlib import Path


def _entries(path: Path) -> dict[str, bytes]:
    with zipfile.ZipFile(path) as archive:
        return {name: archive.read(name) for name in archive.namelist()}


def _sbom_name(entries: dict[str, bytes]) -> str:
    matches = [name for name in entries if ".dist-info/sboms/" in name]
    if len(matches) != 1:
        raise AssertionError(f"expected one wheel SBOM, observed {matches!r}")
    return matches[0]


def _record_name(entries: dict[str, bytes]) -> str:
    matches = [name for name in entries if name.endswith(".dist-info/RECORD")]
    if len(matches) != 1:
        raise AssertionError(f"expected one wheel RECORD, observed {matches!r}")
    return matches[0]


def _normalized_sbom(value: bytes) -> object:
    document = json.loads(value)
    document["serialNumber"] = "<build-identity>"
    document["metadata"]["timestamp"] = "<build-time>"
    return document


def _normalized_record(value: bytes, sbom: str) -> list[tuple[str, str, str]]:
    rows = list(csv.reader(io.StringIO(value.decode("utf-8"))))
    return [
        (name, "<normalized-sbom-hash>" if name == sbom else digest, size)
        for name, digest, size in rows
    ]


def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit("usage: check_python_wheel_reproducibility.py FIRST.whl SECOND.whl")
    first_path, second_path = map(Path, sys.argv[1:])
    first = _entries(first_path)
    second = _entries(second_path)
    if first.keys() != second.keys():
        raise AssertionError("wheel entry sets differ")

    sbom = _sbom_name(first)
    record = _record_name(first)
    for name in first:
        if name in {sbom, record}:
            continue
        if first[name] != second[name]:
            raise AssertionError(f"wheel payload differs: {name}")
    if _normalized_sbom(first[sbom]) != _normalized_sbom(second[sbom]):
        raise AssertionError("wheel SBOM differs outside serialNumber/timestamp")
    if _normalized_record(first[record], sbom) != _normalized_record(second[record], sbom):
        raise AssertionError("wheel RECORD differs outside the normalized SBOM hash")
    print("wheel payloads are reproducible; only CycloneDX serialNumber/timestamp differ")


if __name__ == "__main__":
    main()
