"""Package and replay the sealed v7-to-v8 Core ledger delta without raw captures.

This is a portable historical row-integrity snapshot, not an official test
re-execution or current-package parity result. Historical absolute paths in
receipts and summaries are compared as opaque strings; they are never opened.
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import os
import re
import stat
import tarfile
import tempfile
from collections import Counter
from collections.abc import Iterable, Mapping
from itertools import zip_longest
from pathlib import Path
from typing import Any, cast

ROOT = Path(__file__).resolve().parents[1]
WORK = Path(".work/official-conformance")
OUTCOMES = WORK / "outcomes"
INVENTORY = WORK / "playwright-core-v1.58.0.jsonl"
V7 = OUTCOMES / "core-server-connect-http-401-bidi-chromium-strict-paired-v7"
V8 = OUTCOMES / "core-server-connect-close-error-bidi-chromium-strict-paired-v8"
SIDECAR = Path("snapshot.json")
KIND = "playwright-core-v1.58.0-v8-ledger-only-historical-row-integrity"
SCHEMA = 1
COUNT = 48_477
LANES = ("candidate", "oracle")
V7_SCOPE = (
    "official-browser-free-server-connect-http-401-bidi-chromium-"
    "fixture-host-adapted-internal-diagnostic-green-v7"
)
V8_SCOPE = (
    "official-browser-free-server-connect-close-error-bidi-chromium-"
    "fixture-host-adapted-internal-diagnostic-green-v8"
)
CLASSIFICATION_REASONS = [
    "private-client-access",
    "runtime-source-import:packages/playwright-core/lib/server/utils/network",
    "runtime-source-import:packages/playwright-core/lib/server/utils/userAgent",
]
TARGETS = {
    5360: (
        "ee6df4b3a2217bd356a7-511a6ea814b661f15f2e",
        "bidi-chrome-canary-library",
        "launchServer",
    ),
    5361: (
        "ee6df4b3a2217bd356a7-82f4b2b4a00d4a3c199b",
        "bidi-chrome-canary-library",
        "run-server",
    ),
    5362: (
        "ee6df4b3a2217bd356a7-2b2c19e6a3745a17907d",
        "bidi-chrome-stable-library",
        "launchServer",
    ),
    5363: (
        "ee6df4b3a2217bd356a7-f0142f43a589fa23adf3",
        "bidi-chrome-stable-library",
        "run-server",
    ),
    5364: (
        "ee6df4b3a2217bd356a7-5977d0c5bef040f1bf03",
        "bidi-chromium-library",
        "launchServer",
    ),
    5365: (
        "ee6df4b3a2217bd356a7-9c774e5f960e4e3f4a21",
        "bidi-chromium-library",
        "run-server",
    ),
}

# Immutable local seals, not values learned from the archive being verified.
PINNED: dict[Path, tuple[int, str]] = {
    INVENTORY: (23611514, "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"),
    V7 / "candidate.jsonl": (
        26204457,
        "6f8abbddeb28c1666afd6b7a28eaccfad2f4b611834fb39a2d9e09a125c4a2ab",
    ),
    V7 / "candidate.summary.json": (
        1814,
        "a66b8aa2a2e723d097652b68c2e17c7fbe557219fc5e81b1eafe5b92b188c182",
    ),
    V7 / "oracle.jsonl": (
        26137848,
        "d3cf5ba7b73d00fcc0e2d38ced08c482c1542a5927b8ea3b298a23fc58e50db1",
    ),
    V7 / "oracle.summary.json": (
        1810,
        "90a10cd49d2b675d4f086701f97fd24f6e9f26528e8cbc7c4518da1725478fb9",
    ),
    V7 / "promotion-receipt.json": (
        17880,
        "ef2b8a1d16fd3758c784071e50777df0b35935e04b9c1a3eaa8bf3a60a339f9a",
    ),
    V8 / "candidate.jsonl": (
        26206649,
        "d41e853872aaed73e303184909b9323e7229e1e5068307ac831419bf7b5ab1d6",
    ),
    V8 / "candidate.summary.json": (
        1940,
        "fb81213cf7674ba26ba170f5216afc1143c72456ba5873da61132d652cb15339",
    ),
    V8 / "oracle.jsonl": (
        26140028,
        "c6fd28c4db0184f5821489907e5b489f500f2752c34a5d097ade1b68f5f565b4",
    ),
    V8 / "oracle.summary.json": (
        1936,
        "10b68d822d275d0b2a916ee559048ce44fb646a388a82ef40f97ef40a3805f86",
    ),
    V8 / "promotion-receipt.json": (
        17830,
        "4c0a72a233440263a38bf718fa003fa2e8f8c36ef0243aaac9811c314e14dd68",
    ),
}
INPUTS = tuple(sorted(PINNED, key=lambda path: path.as_posix()))
ARCHIVE_FILES = tuple(sorted((*INPUTS, SIDECAR), key=lambda path: path.as_posix()))
ARCHIVE_NAMES = frozenset(path.as_posix() for path in ARCHIVE_FILES)
MAX_ARCHIVE_BYTES = 32 * 1024 * 1024
MAX_EXPANDED_BYTES = 256 * 1024 * 1024
MAX_SIDECAR_BYTES = 64 * 1024
MAX_TRAILER_BYTES = 10 * 1024
TAR_BLOCK_BYTES = 512
ARCHIVE_SIZE_BOUNDS = {
    **{relative.as_posix(): (size, size) for relative, (size, _) in PINNED.items()},
    SIDECAR.as_posix(): (1, MAX_SIDECAR_BYTES),
}

if len(INPUTS) != 11 or len(ARCHIVE_NAMES) != 12:
    raise RuntimeError("v8 ledger snapshot requires exactly 11 inputs and one sidecar")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def file_digest(path: Path) -> str:
    result = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            result.update(chunk)
    return result.hexdigest()


def regular_input(root: Path, relative: Path) -> Path:
    """Reject symlinks in every component beneath the package root."""
    require(root.is_dir() and not root.is_symlink(), f"unsafe package root: {root}")
    require(
        not relative.is_absolute()
        and relative.parts
        and all(part not in ("", ".", "..") for part in relative.parts),
        f"unsafe relative input: {relative}",
    )
    current = root
    for index, part in enumerate(relative.parts):
        current /= part
        try:
            mode = current.lstat().st_mode
        except OSError as error:
            raise ValueError(f"missing package input: {current}") from error
        require(not stat.S_ISLNK(mode), f"symlinked package input: {current}")
        expected = stat.S_ISREG if index == len(relative.parts) - 1 else stat.S_ISDIR
        require(expected(mode), f"non-regular package input: {current}")
    return current


def input_entries(root: Path) -> list[dict[str, object]]:
    entries = []
    for relative in INPUTS:
        path = regular_input(root, relative)
        size, sha256 = PINNED[relative]
        require(path.stat().st_size == size, f"sealed input size drifted: {relative}")
        require(file_digest(path) == sha256, f"sealed input SHA-256 drifted: {relative}")
        entries.append({"path": relative.as_posix(), "bytes": size, "sha256": sha256})
    return entries


def json_object(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_bytes())
    require(isinstance(value, dict), f"JSON object required: {path}")
    return value


def target_source(spec_id: str, project: str, kind: str) -> dict[str, Any]:
    return {
        "classificationReasons": CLASSIFICATION_REASONS,
        "column": 5,
        "expectedStatus": "passed",
        "file": "library/browsertype-connect.spec.ts",
        "line": 130,
        "projectId": project,
        "projectName": project,
        "role": "official-entrypoint",
        "sourceClassification": "implementation-internal",
        "sourceFile": "tests/library/browsertype-connect.spec.ts",
        "specId": spec_id,
        "suite": "bidi",
        "titlePath": [
            "library/browsertype-connect.spec.ts",
            kind,
            "should print custom ws close error",
        ],
    }


def replay_lane(
    inventory_lines: Iterable[bytes],
    old_lines: Iterable[bytes],
    new_lines: Iterable[bytes],
    *,
    lane: str,
    results_by_project: Mapping[str, str],
    expected_count: int = COUNT,
    targets: Mapping[int, tuple[str, str, str]] = TARGETS,
) -> dict[str, object]:
    """Recount all rows and prove only the six specified canonical deltas."""
    require(lane in LANES, "unknown lane")
    require(len(targets) == 6, "target count differs")
    require(
        set(results_by_project) == {item[1] for item in targets.values()}
        and all(isinstance(value, str) and value for value in results_by_project.values()),
        "result paths differ from target projects",
    )
    old_outcomes: Counter[str] = Counter()
    new_outcomes: Counter[str] = Counter()
    old_scopes: Counter[str] = Counter()
    new_scopes: Counter[str] = Counter()
    changed: list[dict[str, object]] = []
    count = 0
    for index, (inventory_raw, old_raw, new_raw) in enumerate(
        zip_longest(inventory_lines, old_lines, new_lines)
    ):
        require(
            inventory_raw is not None and old_raw is not None and new_raw is not None,
            f"{lane} row count differs at {index}",
        )
        require(
            inventory_raw.endswith(b"\n") and old_raw.endswith(b"\n") and new_raw.endswith(b"\n"),
            f"{lane} unterminated row {index}",
        )
        source = json.loads(inventory_raw)
        old = json.loads(old_raw)
        require(
            isinstance(source, dict)
            and isinstance(old, dict)
            and all(old.get(key) == value for key, value in source.items())
            and old.get("outcome") in {"expected", "unexpected", "skipped", "not-run"}
            and type(old.get("attempts")) is int
            and old["attempts"] >= 0,
            f"{lane} predecessor identity or outcome differs at {index}",
        )
        old_outcomes[old["outcome"]] += 1
        if "evidenceScope" in old:
            require(isinstance(old["evidenceScope"], str), "old scope malformed")
            old_scopes[old["evidenceScope"]] += 1
        if index in targets:
            spec_id, project, kind = targets[index]
            require(
                source == target_source(spec_id, project, kind)
                and old == {**source, "attempts": 0, "outcome": "not-run"},
                f"{lane} target identity or classification differs at {index}",
            )
            promoted = {
                **old,
                "attempts": 1,
                "candidateRuntimeRequired": lane == "candidate",
                "evidenceScope": V8_SCOPE,
                "outcome": "expected",
                "resultFile": results_by_project[project],
            }
            canonical = (
                json.dumps(promoted, sort_keys=True, separators=(",", ":")) + "\n"
            ).encode()
            require(
                new_raw == canonical and new_raw != old_raw,
                f"{lane} target row {index} differs from canonical transition",
            )
            changed.append(
                {
                    "index": index,
                    "specId": spec_id,
                    "oldRowSha256": digest(old_raw),
                    "newRowSha256": digest(new_raw),
                }
            )
            new_outcomes["expected"] += 1
            new_scopes[V8_SCOPE] += 1
        else:
            require(old_raw == new_raw, f"{lane} non-target row {index} changed")
            require(
                old.get("specId") not in {item[0] for item in targets.values()},
                f"{lane} target ID at wrong row {index}",
            )
            new_outcomes[old["outcome"]] += 1
            if "evidenceScope" in old:
                new_scopes[old["evidenceScope"]] += 1
        count += 1
    require(
        count == expected_count and [entry["index"] for entry in changed] == sorted(targets),
        f"{lane} row count or target positions differ",
    )
    require(
        old_outcomes["not-run"] >= 6
        and new_outcomes["expected"] == old_outcomes["expected"] + 6
        and new_outcomes["not-run"] == old_outcomes["not-run"] - 6
        and all(
            new_outcomes[status] == old_outcomes[status] for status in ("unexpected", "skipped")
        )
        and new_scopes[V8_SCOPE] == 6
        and V8_SCOPE not in old_scopes,
        f"{lane} outcome or scope arithmetic differs",
    )
    return {
        "rows": count,
        "oldOutcomes": dict(sorted(old_outcomes.items())),
        "newOutcomes": dict(sorted(new_outcomes.items())),
        "oldScopes": dict(sorted(old_scopes.items())),
        "newScopes": dict(sorted(new_scopes.items())),
        "changedRows": changed,
    }


def require_no_parity(receipt: Mapping[str, Any], scope: str) -> None:
    require(
        receipt.get("schema") == 1
        and receipt.get("scope") == scope
        and receipt.get("promotionKind") == "implementation-internal-diagnostic"
        and receipt.get("fixtureAdapted") is True
        and receipt.get("sourceClassificationPreserved") == "implementation-internal"
        and receipt.get("diagnosticOnly") is True
        and receipt.get("publicApiParityEvidence") is False
        and receipt.get("browserExecutionEvidence") is False
        and receipt.get("bidiProtocolParityEvidence") is False
        and receipt.get("currentLivePackageEvidence") is False
        and receipt.get("recordsPerLane") == COUNT
        and receipt.get("promotedRowsPerLane") == 6
        and isinstance(receipt.get("lanes"), dict)
        and set(receipt["lanes"]) == set(LANES),
        f"{scope} identity or no-parity flags differ",
    )


def verify_summary(
    summary: Mapping[str, Any],
    *,
    inventory: str,
    ledger: str,
    ledger_sha256: str,
    outcomes: Mapping[str, int],
    scopes: Mapping[str, int],
) -> None:
    require(
        summary
        == {
            "schema": 1,
            "inventory": inventory,
            "ledger": ledger,
            "ledgerSha256": ledger_sha256,
            "records": COUNT,
            "outcomes": outcomes,
            "explicitEvidenceScopes": scopes,
        },
        f"ledger summary disagrees with replay: {ledger}",
    )


def replay_snapshot(root: Path) -> dict[str, object]:
    """Verify the pinned v7/v8 receipt chain and both full 48,477-row lanes."""
    v7 = json_object(regular_input(root, V7 / "promotion-receipt.json"))
    v8 = json_object(regular_input(root, V8 / "promotion-receipt.json"))
    require_no_parity(v7, V7_SCOPE)
    require_no_parity(v8, V8_SCOPE)
    require(
        v8.get("priorBundleReceiptSha256") == PINNED[V7 / "promotion-receipt.json"][1]
        and v7.get("inventorySha256") == PINNED[INVENTORY][1]
        and v8.get("inventorySha256") == PINNED[INVENTORY][1]
        and v8.get("targetZeroBasedIndexes") == list(TARGETS)
        and v8.get("targetSpecIds") == [item[0] for item in TARGETS.values()]
        and v7.get("targetZeroBasedIndexes") == list(range(5336, 5342))
        and isinstance(v8.get("inventory"), str)
        and v8.get("inventory") == v7.get("inventory"),
        "receipt ancestry, inventory, or target identities differ",
    )
    summaries: dict[str, dict[str, int]] = {}
    for lane in LANES:
        prior = v7["lanes"][lane]
        current = v8["lanes"][lane]
        require(isinstance(prior, dict) and isinstance(current, dict), "lane receipt malformed")
        old_path = V7 / f"{lane}.jsonl"
        new_path = V8 / f"{lane}.jsonl"
        old_summary_path = V7 / f"{lane}.summary.json"
        new_summary_path = V8 / f"{lane}.summary.json"
        require(
            prior.get("ledgerSha256") == PINNED[old_path][1]
            and prior.get("summarySha256") == PINNED[old_summary_path][1]
            and current.get("baseSha256") == PINNED[old_path][1]
            and current.get("baseSummarySha256") == PINNED[old_summary_path][1]
            and current.get("ledgerSha256") == PINNED[new_path][1]
            and current.get("summarySha256") == PINNED[new_summary_path][1]
            and current.get("oldOutcome") == "not-run"
            and current.get("newOutcome") == "expected"
            and current.get("promotedRows") == 6,
            f"{lane} receipt ledger chain differs",
        )
        official_runs = current.get("officialRuns")
        require(
            isinstance(official_runs, list)
            and [run.get("project") for run in official_runs]
            == [
                "bidi-chromium-library",
                "bidi-chrome-stable-library",
                "bidi-chrome-canary-library",
            ]
            and all(isinstance(run.get("result"), str) for run in official_runs),
            f"{lane} receipt result paths differ",
        )
        results = {run["project"]: run["result"] for run in official_runs}
        with (
            regular_input(root, INVENTORY).open("rb") as inventory_stream,
            regular_input(root, old_path).open("rb") as old_stream,
            regular_input(root, new_path).open("rb") as new_stream,
        ):
            replay = replay_lane(
                inventory_stream,
                old_stream,
                new_stream,
                lane=lane,
                results_by_project=results,
            )
        prior_summary = json_object(regular_input(root, old_summary_path))
        current_summary = json_object(regular_input(root, new_summary_path))
        verify_summary(
            prior_summary,
            inventory=v8["inventory"],
            ledger=prior["ledger"],
            ledger_sha256=PINNED[old_path][1],
            outcomes=replay["oldOutcomes"],
            scopes=replay["oldScopes"],
        )
        verify_summary(
            current_summary,
            inventory=v8["inventory"],
            ledger=current["ledger"],
            ledger_sha256=PINNED[new_path][1],
            outcomes=replay["newOutcomes"],
            scopes=replay["newScopes"],
        )
        require(
            prior.get("outcomes") == replay["oldOutcomes"]
            and current.get("outcomes") == replay["newOutcomes"]
            and current.get("changedRows") == replay["changedRows"]
            and current.get("ledger") == current_summary["ledger"],
            f"{lane} receipt rows or totals differ",
        )
        summaries[lane] = cast(dict[str, int], replay["newOutcomes"])
    return {
        "kind": KIND,
        "inventoryRows": COUNT,
        "promotedRowsPerLane": 6,
        "targetZeroBasedIndexes": list(TARGETS),
        "outcomes": summaries,
        "v8ReceiptSha256": PINNED[V8 / "promotion-receipt.json"][1],
        "diagnosticOnly": True,
        "publicApiParityEvidence": False,
        "browserExecutionEvidence": False,
        "bidiProtocolParityEvidence": False,
        "currentLivePackageEvidence": False,
    }


def report_sha256(report: Mapping[str, object]) -> str:
    return digest(json.dumps(report, sort_keys=True, separators=(",", ":")).encode())


def verify_source(root: Path) -> dict[str, object]:
    input_entries(root)
    return replay_snapshot(root)


def expected_tree() -> set[str]:
    expected = {SIDECAR.as_posix()}
    for relative in INPUTS:
        expected.add(relative.as_posix())
        expected.update(parent.as_posix() for parent in relative.parents if parent != Path("."))
    return expected


def actual_tree(root: Path) -> set[str]:
    require(root.is_dir() and not root.is_symlink(), f"unsafe package root: {root}")
    observed: set[str] = set()
    for parent, directories, files in os.walk(root, followlinks=False):
        for name in (*directories, *files):
            path = Path(parent) / name
            require(not path.is_symlink(), f"symlinked package entry: {path}")
            observed.add(path.relative_to(root).as_posix())
    return observed


def verify_bundle(root: Path) -> dict[str, object]:
    require(actual_tree(root) == expected_tree(), "package tree differs from 12-file closure")
    sidecar = json_object(regular_input(root, SIDECAR))
    require(
        set(sidecar) == {"schema", "kind", "inputs", "reportSha256"}
        and sidecar.get("schema") == SCHEMA
        and sidecar.get("kind") == KIND
        and sidecar.get("inputs") == input_entries(root),
        "snapshot sidecar identity or input seals differ",
    )
    report = replay_snapshot(root)
    require(
        sidecar.get("reportSha256") == report_sha256(report),
        "snapshot report digest differs",
    )
    return report


def archive_member_name(name: str, allowed_names: frozenset[str]) -> Path:
    parts = name.split("/")
    require(
        not name.startswith("/")
        and "\\" not in name
        and all(part not in ("", ".", "..") for part in parts),
        f"unsafe archive member path: {name}",
    )
    require(name in allowed_names, f"unexpected archive member: {name}")
    return Path(*parts)


def tar_octal(field: bytes, label: str) -> int:
    value = field.strip(b"\0 ")
    require(
        bool(value) and all(digit in b"01234567" for digit in value),
        f"invalid USTAR {label}",
    )
    return int(value, 8)


def read_exact(source: gzip.GzipFile, count: int, label: str) -> bytes:
    chunks: list[bytes] = []
    remaining = count
    while remaining:
        chunk = source.read(remaining)
        require(bool(chunk), f"truncated USTAR {label}")
        chunks.append(chunk)
        remaining -= len(chunk)
    return b"".join(chunks)


def extract_exact_archive(
    archive: Path,
    destination: Path,
    allowed_names: frozenset[str],
    *,
    max_expanded_bytes: int = MAX_EXPANDED_BYTES,
    size_bounds: Mapping[str, tuple[int, int]] | None = None,
) -> None:
    """Manually extract only regular USTAR files from an exact allowlist."""
    require(destination.is_dir() and not destination.is_symlink(), "unsafe destination")
    require(not any(destination.iterdir()), "archive destination must be empty")
    try:
        archive_mode = archive.lstat().st_mode
    except OSError as error:
        raise ValueError(f"missing archive: {archive}") from error
    require(stat.S_ISREG(archive_mode), "archive must be a regular file")
    require(archive.stat().st_size <= MAX_ARCHIVE_BYTES, "archive exceeds size limit")
    if size_bounds is not None:
        require(set(size_bounds) == allowed_names, "archive size-bound closure differs")
    seen: set[str] = set()
    expanded = 0
    with gzip.open(archive, mode="rb") as source:
        while True:
            header = read_exact(source, TAR_BLOCK_BYTES, "header")
            expanded += TAR_BLOCK_BYTES
            require(expanded <= max_expanded_bytes, "expanded archive exceeds size limit")
            if header == bytes(TAR_BLOCK_BYTES):
                second = read_exact(source, TAR_BLOCK_BYTES, "end marker")
                require(second == bytes(TAR_BLOCK_BYTES), "invalid USTAR end marker")
                trailer = source.read(MAX_TRAILER_BYTES + 1)
                require(
                    len(trailer) <= MAX_TRAILER_BYTES and not any(trailer) and not source.read(1),
                    "non-zero or oversized USTAR trailer",
                )
                break
            require(
                header[257:263] == b"ustar\0" and header[263:265] == b"00",
                "archive requires USTAR-only members",
            )
            expected_checksum = tar_octal(header[148:156], "checksum")
            observed_checksum = sum(header[:148]) + 8 * 32 + sum(header[156:])
            require(expected_checksum == observed_checksum, "invalid USTAR checksum")
            prefix = header[345:500].split(b"\0", 1)[0]
            basename = header[:100].split(b"\0", 1)[0]
            member_name = ((prefix + b"/") if prefix else b"") + basename
            try:
                name = member_name.decode("utf-8")
            except UnicodeDecodeError as error:
                raise ValueError("non-UTF-8 USTAR member name") from error
            relative = archive_member_name(name, allowed_names)
            require(header[156:157] == tarfile.REGTYPE, f"non-regular member: {name}")
            require(name not in seen, f"duplicate archive member: {name}")
            seen.add(name)
            size = tar_octal(header[124:136], "size")
            if size_bounds is not None:
                minimum, maximum = size_bounds[name]
                require(
                    minimum <= size <= maximum,
                    f"archive member declared size differs from sealed input: {name}",
                )
            padding = (-size) % TAR_BLOCK_BYTES
            expanded += size + padding
            require(expanded <= max_expanded_bytes, "expanded archive exceeds size limit")
            target = destination / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            with target.open("xb") as output:
                remaining = size
                while remaining:
                    chunk = source.read(min(remaining, 1024 * 1024))
                    require(bool(chunk), f"truncated USTAR member: {name}")
                    output.write(chunk)
                    remaining -= len(chunk)
            if padding:
                require(
                    read_exact(source, padding, "padding") == bytes(padding),
                    f"non-zero USTAR padding: {name}",
                )
        require(seen == allowed_names, "archive file set differs from exact closure")


def verify_archive(archive: Path, expected_sha256: str) -> dict[str, object]:
    require(
        re.fullmatch(r"[a-f0-9]{64}", expected_sha256) is not None,
        "archive pin must be a lowercase SHA-256 digest",
    )
    try:
        mode = archive.lstat().st_mode
    except OSError as error:
        raise ValueError(f"missing archive: {archive}") from error
    require(stat.S_ISREG(mode), "archive must be a regular file")
    require(archive.stat().st_size <= MAX_ARCHIVE_BYTES, "archive exceeds size limit")
    with tempfile.TemporaryDirectory(prefix="pwright-core-v8-ledger-verify-") as temp:
        temporary = Path(temp)
        copied = temporary / "pinned.tar.gz"
        copied_bytes = 0
        with archive.open("rb") as source, copied.open("xb") as output:
            for chunk in iter(lambda: source.read(1024 * 1024), b""):
                copied_bytes += len(chunk)
                require(copied_bytes <= MAX_ARCHIVE_BYTES, "archive exceeds size limit")
                output.write(chunk)
        require(file_digest(copied) == expected_sha256, "archive SHA-256 mismatch")
        bundle = temporary / "bundle"
        bundle.mkdir()
        extract_exact_archive(copied, bundle, ARCHIVE_NAMES, size_bounds=ARCHIVE_SIZE_BOUNDS)
        return verify_bundle(bundle)


def pack_archive(source_root: Path, output: Path) -> tuple[dict[str, object], str, int]:
    """Create a deterministic no-replace archive after full source replay."""
    report = verify_source(source_root)
    require(not output.exists() and not output.is_symlink(), "output already exists")
    require(
        output.parent.is_dir() and not output.parent.is_symlink(),
        "output parent is missing or symlinked",
    )
    sidecar = (
        json.dumps(
            {
                "schema": SCHEMA,
                "kind": KIND,
                "inputs": input_entries(source_root),
                "reportSha256": report_sha256(report),
            },
            sort_keys=True,
            indent=2,
        )
        + "\n"
    ).encode()
    with tempfile.TemporaryDirectory(prefix=f".{output.name}-", dir=output.parent) as temp:
        staged = Path(temp) / "snapshot.tar.gz"
        with (
            staged.open("xb") as destination,
            gzip.GzipFile(
                filename="", fileobj=destination, mode="wb", compresslevel=9, mtime=0
            ) as compressed,
            tarfile.open(fileobj=compressed, mode="w", format=tarfile.USTAR_FORMAT) as tar,
        ):
            for relative in ARCHIVE_FILES:
                source = None if relative == SIDECAR else regular_input(source_root, relative)
                member = tarfile.TarInfo(relative.as_posix())
                member.size = len(sidecar) if source is None else source.stat().st_size
                member.mode = 0o644
                member.mtime = 0
                member.uid = 0
                member.gid = 0
                member.uname = ""
                member.gname = ""
                if source is None:
                    tar.addfile(member, io.BytesIO(sidecar))
                else:
                    with source.open("rb") as stream:
                        tar.addfile(member, stream)
        require(staged.stat().st_size <= MAX_ARCHIVE_BYTES, "archive exceeds size limit")
        archive_sha256 = file_digest(staged)
        require(verify_archive(staged, archive_sha256) == report, "archive replay differs")
        require(input_entries(source_root) == json.loads(sidecar)["inputs"], "source drifted")
        try:
            os.link(staged, output)
        except FileExistsError as error:
            raise ValueError("output appeared while packaging") from error
    return report, archive_sha256, output.stat().st_size


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    actions = parser.add_subparsers(dest="command", required=True)
    source = actions.add_parser("verify-source", help="replay sealed local inputs")
    source.add_argument("--source-root", type=Path, default=ROOT)
    pack = actions.add_parser("pack-archive", help="package the sealed ledger-only snapshot")
    pack.add_argument("--source-root", type=Path, default=ROOT)
    pack.add_argument("--output", type=Path, required=True)
    verify = actions.add_parser("verify-archive", help="verify a relocated pinned archive")
    verify.add_argument("--archive", type=Path, required=True)
    verify.add_argument("--expected-sha256", required=True)
    args = parser.parse_args()
    try:
        if args.command == "verify-source":
            report = verify_source(args.source_root)
            archive_sha256 = None
            archive_bytes = None
        elif args.command == "pack-archive":
            report, archive_sha256, archive_bytes = pack_archive(args.source_root, args.output)
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
        parser.exit(2, f"Core v8 ledger snapshot rejected: {error}\n")
    result = {**report, "reportSha256": report_sha256(report)}
    if archive_sha256 is not None:
        result["archiveSha256"] = archive_sha256
        result["archiveBytes"] = archive_bytes
    print(json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    main()
