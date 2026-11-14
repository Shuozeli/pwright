#!/usr/bin/env python3
"""Freeze the input bytes needed to replay the v1/v2 remote-kill attestations.

The original result directories are intentionally not rewritten.  This seal is
an additional, versioned input archive, not official-test ledger credit.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import tempfile
from pathlib import Path, PurePosixPath
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-python")
BASE = ROOT / ".work/official-conformance"
RESULTS = BASE / "results"
SEAL = BASE / "seals/python-remote-kill-v1-v2-inputs-v1"
RESULT_DIRS = {
    "v1": RESULTS / "python-connect-remote-kill-synthetic-v1-oct20-final",
    "v2": RESULTS / "python-connect-remote-kill-synthetic-v2-terminal-fix-oct20",
}
ATTESTATION_SHA256 = {
    "v1": "6d75f73094c46e7127ee87cc7319bd4e1edadde2986dc4505601aadb37590a59",
    "v2": "bff60e731976671cd1af6eb7e2484884aace48e8a7c2be7cdea0bb3696e2a5ab",
}
LEDGERS = {
    "candidate": BASE
    / "outcomes/python-connect-refused-strict-paired-v1/playwright-python-candidate-browser-connect-refused-strict-paired-v1.jsonl",
    "oracle": BASE
    / "outcomes/python-connect-refused-strict-paired-v1/playwright-python-oracle-browser-connect-refused-strict-paired-v1.jsonl",
}
BUILD_WHEEL = (
    BASE
    / "builds/python-remote-terminal-fix-oct20/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl"
)
WHEEL_NAME = BUILD_WHEEL.name


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def regular(path: Path) -> Path:
    require(path.is_file() and not path.is_symlink(), f"missing regular file: {path}")
    return path


def relative_key(raw: str) -> PurePosixPath:
    path = PurePosixPath(raw)
    require(
        not path.is_absolute()
        and path.parts
        and all(part not in (".", "..") for part in path.parts),
        f"unsafe path: {raw}",
    )
    require(str(path) == raw, f"noncanonical path: {raw}")
    return path


def checked_attestations() -> dict[str, dict[str, Any]]:
    attestations: dict[str, dict[str, Any]] = {}
    for version, directory in RESULT_DIRS.items():
        path = regular(directory / "attestation.json")
        require(
            sha256(path) == ATTESTATION_SHA256[version],
            f"{version} attestation changed",
        )
        value = json.loads(path.read_text())
        require(
            isinstance(value, dict) and value.get("schema") == 1,
            f"{version} attestation schema changed",
        )
        require(
            value.get("classification")
            == "official-derived-fixture-adapted-internal-diagnostic",
            f"{version} classification changed",
        )
        require(
            value.get("publicApiParityEvidence") is False
            and value.get("promotedToPythonLedger") is False,
            f"{version} diagnostic was misclassified",
        )
        attestations[version] = value
    require(
        attestations["v2"].get("predecessorRedAttestationSha256")
        == ATTESTATION_SHA256["v1"],
        "v2 predecessor link changed",
    )
    return attestations


def inputs() -> dict[str, tuple[Path, str]]:
    attestations = checked_attestations()
    selected: dict[str, tuple[Path, str]] = {}

    def add(key: str, source: Path, expected: str) -> None:
        relative_key(key)
        require(
            len(expected) == 64
            and all(char in "0123456789abcdef" for char in expected),
            f"invalid digest: {key}",
        )
        old = selected.get(key)
        require(
            old is None or old == (source, expected),
            f"conflicting source identity: {key}",
        )
        selected[key] = (regular(source), expected)

    for version, attestation in attestations.items():
        add(
            f"attestations/{version}.json",
            RESULT_DIRS[version] / "attestation.json",
            ATTESTATION_SHA256[version],
        )
        for relative, digest in attestation["harnessInputs"].items():
            add(f"repo/{relative}", ROOT / relative_key(relative), digest)
        for relative, digest in attestation.get("candidateSourceSha256", {}).items():
            add(f"repo/{relative}", ROOT / relative_key(relative), digest)
        for relative, digest in attestation.get("upstreamSourceSha256", {}).items():
            add(f"upstream/{relative}", UPSTREAM / relative_key(relative), digest)
        add(
            f"result-wheels/{version}/{WHEEL_NAME}",
            RESULT_DIRS[version] / WHEEL_NAME,
            attestation["candidateWheelSha256"],
        )
    for mode, path in LEDGERS.items():
        require(
            attestations["v1"]["predecessorLedgerSha256"][mode]
            == attestations["v2"]["predecessorLedgerSha256"][mode],
            f"{mode} ledger predecessor changed",
        )
        add(
            f"ledgers/{mode}.jsonl",
            path,
            attestations["v1"]["predecessorLedgerSha256"][mode],
        )
    add(
        f"build-wheel/{WHEEL_NAME}",
        BUILD_WHEEL,
        attestations["v2"]["candidateWheelSha256"],
    )
    for key, (source, digest) in selected.items():
        require(sha256(source) == digest, f"input drift before seal: {key}")
    return selected


def seal() -> Path:
    require(
        not SEAL.exists() and not SEAL.is_symlink(),
        f"refusing to overwrite seal: {SEAL}",
    )
    selected = inputs()
    SEAL.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(
        prefix=".python-remote-kill-seal-", dir=SEAL.parent
    ) as staging_name:
        staging = Path(staging_name)
        entries: list[dict[str, Any]] = []
        for key, (source, expected) in sorted(selected.items()):
            target = staging / "inputs" / key
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)
            require(sha256(target) == expected, f"copy changed during seal: {key}")
            entries.append(
                {"path": key, "sha256": expected, "size": target.stat().st_size}
            )
        manifest = {
            "schema": 1,
            "kind": "python-remote-kill-v1-v2-input-seal",
            "classification": "official-derived-fixture-adapted-internal-diagnostic",
            "attestationSha256": ATTESTATION_SHA256,
            "entries": entries,
        }
        (staging / "seal.json").write_text(
            json.dumps(manifest, indent=2, sort_keys=True) + "\n"
        )
        staging.rename(SEAL)
    return SEAL / "seal.json"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("create",))
    args = parser.parse_args()
    if args.command == "create":
        manifest = seal()
        print(f"sealed {manifest}: sha256={sha256(manifest)}")


if __name__ == "__main__":
    main()
