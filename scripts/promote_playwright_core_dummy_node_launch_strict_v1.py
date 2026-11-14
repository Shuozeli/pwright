"""Gate one paired Core dummy-Node launch row behind verified strict evidence.

The historical dummy-Node receipts are diagnostic only. This gate accepts only
new, independently verified public-launch evidence and publishes an immutable
paired successor after both lanes prove the same pinned official assertion.
"""

from __future__ import annotations

import argparse
import ctypes
import hashlib
import json
import os
import shutil
import sys
import tempfile
from collections import Counter
from collections.abc import Iterable
from itertools import zip_longest
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
OUTCOMES = ROOT / ".work/official-conformance/outcomes"
BASE = OUTCOMES / "core-server-connect-ws-error-strict-green-paired-v2"
OUTPUT = OUTCOMES / "core-dummy-node-launch-strict-paired-v1"
STRICT_MANIFEST = (
    ROOT / "tests/official/playwright-core-dummy-node-launch-strict-v1-manifest.json"
)
STRICT_MANIFEST_SHA256 = (
    "6a6d866436c38b38a1b75c6a0e9a652cbfd7e0a4e9f3a8edb1746bef86db4b77"
)
STRICT_SIDECARS = {
    "candidate": (
        OUTCOMES / "core-dummy-strict-candidate-oct20-final-v2.strict.json",
        "1e27b6bb5047fe6aa44b1ffe7de6297367a99c22e38ace43a2c4ec47478ea4c0",
    ),
    "oracle": (
        OUTCOMES / "core-dummy-strict-oracle-oct20-final-v2.strict.json",
        "5a2ccae82a0774b072ca201c25ec29ef4b0f9ffc410c3e650e10c3596a956ba4",
    ),
}
INVENTORY = ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"
INVENTORY_SHA256 = "2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30"
BASE_RECEIPT_SHA256 = "e5921619fdfd88dd14ec71ab124ae952f34edc6678e7b914e46692ac0c754da4"
BASE_SHA256 = {
    "candidate": "0ce5ec464d804db0257707b59ca932ae29e3404dab84321700fb2260768d46eb",
    "oracle": "876335bbd6c5b1cda10740dc74d469ff23ccfdfc1b21c4af38b2f5c7e3599006",
}
BASE_SUMMARY_SHA256 = {
    "candidate": "5d68cb80fdb38e680bba88cb3f02eb5878ee73f26c8718f77075e78e11d0e3f9",
    "oracle": "067d3e96c3dc346f646f36a4302dc528dc4cc7a201fd8db40d23ffa6b2267cc6",
}
COUNT = 48_477
TARGET_INDEX = 31_613
SPEC_ID = "d1a604bdf6287c6776f3-522221011eea38945220"
SPEC_FILE = "library/browsertype-launch.spec.ts"
SOURCE_FILE = "tests/library/browsertype-launch.spec.ts"
SOURCE_SHA256 = "64fe24188ec96bb0d5d1ecca5af37986b16d82451b91e4db80593f707126bd05"
CANDIDATE_SOURCE_SHA256 = (
    "a2bb47a7c22f88b1c8cbaff72fb107de0a015eec85f6a4101d94e0cbddbfca5a"
)
CANDIDATE_ARTIFACT_SHA256 = (
    "ab6be17625aadadbfce4bbc72866c72732191572c40b951821a68f947509cf3a"
)
SUITE = "library-browsertype-launch-dummy-node-failure-chromium"
SELECTOR = "tests/library/browsertype-launch.spec.ts:64"
TITLE = "should reject if launched browser fails immediately"
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
STRICT_SCOPE = "core-dummy-node-launch-strict-v1"
SCOPE = "official-browser-free-browsertype-launch-dummy-node-strict-v1"
LANES = ("candidate", "oracle")


def verify_pair(candidate_path: Path, oracle_path: Path) -> dict[str, dict[str, Any]]:
    from verify_playwright_core_dummy_node_launch_strict_v1 import (
        verify_pair as independently_verify_pair,
    )

    return independently_verify_pair(candidate_path, oracle_path)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def pin(value: str, label: str) -> None:
    require(
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdef" for character in value),
        f"invalid or unfilled SHA-256 pin: {label}",
    )


def sealed(path: Path, expected: str, label: str) -> bytes:
    pin(expected, label)
    require(
        path.is_file() and not path.is_symlink(), f"missing or linked {label}: {path}"
    )
    data = path.read_bytes()
    require(digest(data) == expected, f"{label} SHA-256 drift")
    return data


def object_json(data: bytes, label: str) -> dict[str, Any]:
    value = json.loads(data)
    require(isinstance(value, dict), f"{label} must be a JSON object")
    return value


def check_prior_bundle() -> dict[str, dict[str, Any]]:
    sealed(INVENTORY, INVENTORY_SHA256, "Core inventory")
    receipt = object_json(
        sealed(
            BASE / "promotion-receipt.json", BASE_RECEIPT_SHA256, "predecessor receipt"
        ),
        "predecessor receipt",
    )
    require(
        receipt.get("schema") == 1
        and receipt.get("inventory") == str(INVENTORY)
        and receipt.get("inventorySha256") == INVENTORY_SHA256
        and receipt.get("recordsPerLane") == COUNT
        and receipt.get("scope")
        == "official-browser-free-server-connect-ws-error-internal-diagnostic-green-v2"
        and set(receipt.get("lanes", {})) == set(LANES),
        "predecessor receipt identity drifted",
    )
    summaries: dict[str, dict[str, Any]] = {}
    for lane in LANES:
        ledger = BASE / f"{lane}.jsonl"
        summary_path = BASE / f"{lane}.summary.json"
        sealed(ledger, BASE_SHA256[lane], f"{lane} predecessor ledger")
        summary = object_json(
            sealed(
                summary_path, BASE_SUMMARY_SHA256[lane], f"{lane} predecessor summary"
            ),
            f"{lane} predecessor summary",
        )
        lane_receipt = receipt["lanes"][lane]
        require(
            summary.get("schema") == 1
            and summary.get("inventory") == str(INVENTORY)
            and summary.get("records") == COUNT
            and summary.get("ledger") == str(ledger)
            and summary.get("ledgerSha256") == BASE_SHA256[lane]
            and lane_receipt.get("ledger") == str(ledger)
            and lane_receipt.get("ledgerSha256") == BASE_SHA256[lane]
            and lane_receipt.get("summarySha256") == BASE_SUMMARY_SHA256[lane]
            and lane_receipt.get("outcomes") == summary.get("outcomes"),
            f"{lane} predecessor ledger, summary, or receipt drifted",
        )
        summaries[lane] = summary
    return summaries


def check_pair(
    sidecars: dict[str, tuple[Path, str]],
) -> tuple[dict[str, dict[str, Any]], dict[str, str]]:
    require(
        set(sidecars) == set(LANES) and sidecars == STRICT_SIDECARS,
        "strict pair must be the exact fresh candidate/oracle evidence pins",
    )
    require(
        sidecars["candidate"][0] != sidecars["oracle"][0],
        "strict lanes share one sidecar",
    )
    require(
        all(path.is_absolute() for path, _ in sidecars.values()),
        "strict sidecar paths must be absolute",
    )
    manifest = object_json(
        sealed(STRICT_MANIFEST, STRICT_MANIFEST_SHA256, "strict manifest"),
        "strict manifest",
    )
    suite = manifest.get("suite")
    require(
        manifest.get("schema") == 2
        and manifest.get("commit") == COMMIT
        and isinstance(suite, dict)
        and suite.get("id") == SUITE
        and suite.get("selector") == SELECTOR
        and suite.get("specId") == SPEC_ID
        and suite.get("candidateProfile") == "playwright-trace"
        and suite.get("candidateSourceSha256") == CANDIDATE_SOURCE_SHA256
        and suite.get("candidateArtifactSha256") == CANDIDATE_ARTIFACT_SHA256
        and isinstance(suite.get("upstreamSources"), dict)
        and suite["upstreamSources"].get(SOURCE_FILE) == SOURCE_SHA256,
        "strict manifest is not the pinned public dummy-launch case",
    )
    hashes = {
        lane: digest(sealed(path, expected, f"{lane} strict sidecar"))
        for lane, (path, expected) in sidecars.items()
    }
    pair = verify_pair(sidecars["candidate"][0], sidecars["oracle"][0])
    require(
        isinstance(pair, dict) and set(pair) == set(LANES),
        "pair verifier returned wrong lanes",
    )
    verified: dict[str, dict[str, Any]] = {}
    for lane in LANES:
        strict = pair[lane]
        require(isinstance(strict, dict), f"{lane} strict verifier returned non-object")
        sources = strict.get("upstreamSources")
        require(
            strict.get("schema") == 2
            and strict.get("scope") == STRICT_SCOPE
            and strict.get("mode") == lane
            and strict.get("suite") == SUITE
            and strict.get("specId") == SPEC_ID
            and strict.get("selector") == SELECTOR
            and strict.get("upstreamCommit") == COMMIT
            and isinstance(sources, dict)
            and sources == suite["upstreamSources"]
            and strict.get("manifest") == str(STRICT_MANIFEST)
            and strict.get("manifestSha256") == STRICT_MANIFEST_SHA256
            and strict.get("testStatus") == "expected"
            and strict.get("exitCode") == 0,
            f"{lane} strict evidence is not the pinned passing official case",
        )
        require(
            strict.get("publicApiParityEvidence") is True
            and strict.get("diagnosticOnly") is False
            and strict.get("upstreamAssertionsModified") is False
            and strict.get("fixtureAdapted") is True
            and strict.get("fixtureAdaptationEligible") is True
            and strict.get("unadaptedOfficialPass") is False
            and strict.get("ledgerEligible") is True
            and isinstance(strict.get("processOwnershipProof"), dict)
            and bool(strict["processOwnershipProof"]),
            f"{lane} strict evidence is diagnostic or ineligible for Core credit",
        )
        require(
            isinstance(strict.get("runId"), str)
            and bool(strict["runId"])
            and isinstance(strict.get("manifestSha256"), str)
            and isinstance(strict.get("result"), str)
            and Path(strict["result"]).is_absolute(),
            f"{lane} strict run identity or result missing",
        )
        pin(strict["manifestSha256"], f"{lane} manifest")
        result = Path(strict["result"])
        require(
            result.is_file() and not result.is_symlink(),
            f"{lane} strict official result missing or linked",
        )
        sealed(result, strict["resultSha256"], f"{lane} strict official result")
        verified[lane] = strict
    candidate, oracle = verified["candidate"], verified["oracle"]
    require(
        candidate["runId"] != oracle["runId"]
        and candidate["result"] != oracle["result"]
        and candidate["manifestSha256"] == oracle["manifestSha256"]
        and candidate["upstreamSources"] == oracle["upstreamSources"]
        and candidate["testStatus"] == oracle["testStatus"] == "expected",
        "strict oracle/candidate case, source, or outcome parity failed",
    )
    require(
        candidate.get("candidateArtifactSha256") == CANDIDATE_ARTIFACT_SHA256
        and candidate.get("candidateSourceSha256") == CANDIDATE_SOURCE_SHA256
        and candidate.get("candidateProfile") == "playwright-trace"
        and oracle.get("candidateArtifactSha256") is None
        and oracle.get("candidateSourceSha256") is None
        and oracle.get("candidateProfile") is None,
        "strict candidate artifact/oracle isolation failed",
    )
    return verified, hashes


def transform_lines(
    inventory_lines: Iterable[bytes],
    baseline_lines: Iterable[bytes],
    *,
    lane: str,
    result_file: str,
    count: int,
    target_index: int,
    prior_outcomes: dict[str, int],
    prior_scopes: dict[str, int],
) -> tuple[bytes, dict[str, int], dict[str, int]]:
    require(lane in LANES, "unknown ledger lane")
    before: Counter[str] = Counter()
    after: Counter[str] = Counter()
    before_scopes: Counter[str] = Counter()
    after_scopes: Counter[str] = Counter()
    output = bytearray()
    seen = False
    rows = 0
    for index, (inventory_line, line) in enumerate(
        zip_longest(inventory_lines, baseline_lines)
    ):
        require(
            inventory_line is not None and line is not None, f"{lane} row count drifted"
        )
        require(
            inventory_line.endswith(b"\n") and line.endswith(b"\n"),
            f"{lane} row {index} is unterminated",
        )
        source = object_json(inventory_line, f"inventory row {index}")
        row = object_json(line, f"{lane} base row {index}")
        require(
            all(row.get(key) == value for key, value in source.items()),
            f"{lane} row {index} inventory metadata drifted",
        )
        before[row["outcome"]] += 1
        if "evidenceScope" in row:
            before_scopes[row["evidenceScope"]] += 1
        if index == target_index:
            require(not seen, "target row was duplicated")
            seen = True
            require(
                source.get("specId") == SPEC_ID
                and source.get("projectId")
                == source.get("projectName")
                == "chromium-library"
                and source.get("sourceFile") == SOURCE_FILE
                and source.get("file") == SPEC_FILE
                and source.get("line") == 64
                and source.get("column") == 3
                and source.get("titlePath") == [SPEC_FILE, TITLE]
                and source.get("sourceClassification") == "public-api-candidate"
                and source.get("classificationReasons") == []
                and source.get("role") == "official-entrypoint"
                and source.get("suite") == "library"
                and source.get("expectedStatus") == "passed"
                and row == {**source, "attempts": 0, "outcome": "not-run"},
                f"{lane} target is not the exact pinned unrun public API row",
            )
            promoted = {
                **row,
                "attempts": 1,
                "outcome": "expected",
                "resultFile": result_file,
                "evidenceScope": SCOPE,
                "candidateRuntimeRequired": lane == "candidate",
            }
            output.extend(
                (
                    json.dumps(promoted, sort_keys=True, separators=(",", ":")) + "\n"
                ).encode()
            )
            after["expected"] += 1
            after_scopes[SCOPE] += 1
        else:
            require(
                row.get("specId") != SPEC_ID, f"{lane} duplicate target at row {index}"
            )
            output.extend(line)
            after[row["outcome"]] += 1
            if "evidenceScope" in row:
                after_scopes[row["evidenceScope"]] += 1
        rows += 1
    require(rows == count and seen, f"{lane} target count or position drifted")
    require(
        dict(before) == prior_outcomes and dict(before_scopes) == prior_scopes,
        f"{lane} predecessor summary differs from ledger",
    )
    expected_outcomes = before.copy()
    expected_outcomes["not-run"] -= 1
    expected_outcomes["expected"] += 1
    expected_scopes = before_scopes.copy()
    expected_scopes[SCOPE] += 1
    require(
        after == expected_outcomes and after_scopes == expected_scopes,
        f"{lane} successor changed more than one outcome or scope",
    )
    return (
        bytes(output),
        dict(sorted(after.items())),
        dict(sorted(after_scopes.items())),
    )


def prepare_pair(sidecars: dict[str, tuple[Path, str]]) -> dict[str, bytes]:
    prior = check_prior_bundle()
    verified, hashes = check_pair(sidecars)
    files: dict[str, bytes] = {}
    lane_receipts: dict[str, dict[str, Any]] = {}
    for lane in LANES:
        with (
            INVENTORY.open("rb") as inventory,
            (BASE / f"{lane}.jsonl").open("rb") as base,
        ):
            ledger, outcomes, scopes = transform_lines(
                inventory,
                base,
                lane=lane,
                result_file=verified[lane]["result"],
                count=COUNT,
                target_index=TARGET_INDEX,
                prior_outcomes=prior[lane]["outcomes"],
                prior_scopes=prior[lane]["explicitEvidenceScopes"],
            )
        destination = OUTPUT / f"{lane}.jsonl"
        summary = {
            "schema": 1,
            "inventory": str(INVENTORY),
            "records": COUNT,
            "outcomes": outcomes,
            "explicitEvidenceScopes": scopes,
            "ledgerSha256": digest(ledger),
            "ledger": str(destination),
        }
        summary_bytes = (json.dumps(summary, indent=2) + "\n").encode()
        files[f"{lane}.jsonl"] = ledger
        files[f"{lane}.summary.json"] = summary_bytes
        lane_receipts[lane] = {
            "base": str(BASE / f"{lane}.jsonl"),
            "baseSha256": BASE_SHA256[lane],
            "strictSidecar": str(sidecars[lane][0]),
            "strictSidecarSha256": hashes[lane],
            "runId": verified[lane]["runId"],
            "officialResult": verified[lane]["result"],
            "officialResultSha256": verified[lane]["resultSha256"],
            "candidateArtifactSha256": verified[lane]["candidateArtifactSha256"],
            "candidateSourceSha256": verified[lane]["candidateSourceSha256"],
            "ledger": str(destination),
            "ledgerSha256": summary["ledgerSha256"],
            "summarySha256": digest(summary_bytes),
            "oldOutcome": "not-run",
            "newOutcome": "expected",
            "outcomes": outcomes,
        }
    receipt = {
        "schema": 1,
        "scope": SCOPE,
        "publicApiParityEvidence": True,
        "diagnosticOnly": False,
        "upstreamAssertionsModified": False,
        "fixtureAdapted": True,
        "fixtureAdaptationEligible": True,
        "unadaptedOfficialPass": False,
        "candidateRuntimeAttested": True,
        "promoterSourceSha256": digest(Path(__file__).read_bytes()),
        "strictVerifierSha256": digest(
            (
                ROOT / "scripts/verify_playwright_core_dummy_node_launch_strict_v1.py"
            ).read_bytes()
        ),
        "priorBundle": str(BASE),
        "priorBundleReceiptSha256": BASE_RECEIPT_SHA256,
        "inventory": str(INVENTORY),
        "inventorySha256": INVENTORY_SHA256,
        "recordsPerLane": COUNT,
        "targetZeroBasedIndexes": [TARGET_INDEX],
        "targetSpecIds": [SPEC_ID],
        "upstreamCommit": COMMIT,
        "selectedUpstreamSpecSha256": SOURCE_SHA256,
        "manifestSha256": verified["candidate"]["manifestSha256"],
        "lanes": lane_receipts,
    }
    files["promotion-receipt.json"] = (
        json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    ).encode()
    return files


def verify_existing() -> dict[str, Any]:
    require(OUTPUT.is_dir() and not OUTPUT.is_symlink(), "immutable successor missing")
    receipt_path = OUTPUT / "promotion-receipt.json"
    require(
        receipt_path.is_file() and not receipt_path.is_symlink(),
        "successor receipt missing or linked",
    )
    receipt = object_json(receipt_path.read_bytes(), "successor receipt")
    require(
        set(receipt.get("lanes", {})) == set(LANES),
        "successor receipt lane set drifted",
    )
    sidecars = {
        lane: (
            Path(receipt["lanes"][lane]["strictSidecar"]),
            receipt["lanes"][lane]["strictSidecarSha256"],
        )
        for lane in LANES
    }
    expected = prepare_pair(sidecars)
    require(
        {path.name for path in OUTPUT.iterdir()} == set(expected),
        "successor file set drifted",
    )
    for name, payload in expected.items():
        path = OUTPUT / name
        require(
            path.is_file() and not path.is_symlink() and path.read_bytes() == payload,
            f"immutable successor drifted: {name}",
        )
    return receipt


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
    require(
        not OUTPUT.exists() and not OUTPUT.is_symlink(),
        "immutable successor already exists",
    )
    OUTCOMES.mkdir(parents=True, exist_ok=True)
    staging = Path(
        tempfile.mkdtemp(prefix=".core-dummy-launch-strict-v1-", dir=OUTCOMES)
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
        _publish_exclusive(staging, OUTPUT)
        parent_fd = os.open(OUTCOMES, os.O_RDONLY)
        try:
            os.fsync(parent_fd)
        finally:
            os.close(parent_fd)
    finally:
        if staging.exists():
            shutil.rmtree(staging)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify-existing", action="store_true")
    parser.add_argument("--candidate-sidecar", type=Path)
    parser.add_argument("--candidate-sha256")
    parser.add_argument("--oracle-sidecar", type=Path)
    parser.add_argument("--oracle-sha256")
    args = parser.parse_args()
    try:
        if args.verify_existing:
            require(
                all(
                    getattr(args, key) is None
                    for key in (
                        "candidate_sidecar",
                        "candidate_sha256",
                        "oracle_sidecar",
                        "oracle_sha256",
                    )
                ),
                "--verify-existing takes no evidence overrides",
            )
            receipt = verify_existing()
        else:
            require(
                all(
                    getattr(args, key) is not None
                    for key in (
                        "candidate_sidecar",
                        "candidate_sha256",
                        "oracle_sidecar",
                        "oracle_sha256",
                    )
                ),
                "both strict sidecar paths and SHA-256 pins are required",
            )
            sidecars = {
                "candidate": (args.candidate_sidecar, args.candidate_sha256),
                "oracle": (args.oracle_sidecar, args.oracle_sha256),
            }
            files = prepare_pair(sidecars)
            require(
                files == prepare_pair(sidecars),
                "paired evidence changed before publication",
            )
            publish(files)
            receipt = verify_existing()
    except (
        AssertionError,
        OSError,
        ValueError,
        KeyError,
        TypeError,
        json.JSONDecodeError,
    ) as error:
        parser.error(f"strict paired Core dummy-launch promotion rejected: {error}")
    print(json.dumps({"scope": receipt["scope"], "lanes": receipt["lanes"]}, indent=2))


if __name__ == "__main__":
    main()
