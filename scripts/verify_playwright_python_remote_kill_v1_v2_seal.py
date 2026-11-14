#!/usr/bin/env python3
"""Replay the historical remote-kill diagnostics from a pinned input seal.

The original v1/v2 verifiers are loaded from their hash-checked sealed bytes.
Their JUnit, receipt, wheel-install, command, provenance and attestation checks
remain unchanged.  Only live source/harness reads are redirected to the seal.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
import types
from contextlib import contextmanager
from pathlib import Path, PurePosixPath
from typing import Any, Iterator

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / ".work/official-conformance"
SEAL = BASE / "seals/python-remote-kill-v1-v2-inputs-v1"
SEAL_SHA256 = "d72344cc44dd7507f7b6e8f036edf5c2763c160664645ed844d29aed0c1f3327"
ATTESTATION_SHA256 = {
    "v1": "6d75f73094c46e7127ee87cc7319bd4e1edadde2986dc4505601aadb37590a59",
    "v2": "bff60e731976671cd1af6eb7e2484884aace48e8a7c2be7cdea0bb3696e2a5ab",
}
RESULTS = {
    "v1": BASE / "results/python-connect-remote-kill-synthetic-v1-oct20-final",
    "v2": BASE / "results/python-connect-remote-kill-synthetic-v2-terminal-fix-oct20",
}
UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-python")
CORE = Path("/Users/cyuan/projects/thirdparty/playwright-v1.58.0")
BUILD_WHEEL = (
    BASE
    / "builds/python-remote-terminal-fix-oct20/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl"
)
VERIFIER_NAMES = {
    "v1": "verify_playwright_python_connect_remote_kill_synthetic_v1.py",
    "v2": "verify_playwright_python_connect_remote_kill_synthetic_v2.py",
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def regular(path: Path) -> Path:
    require(
        path.is_file() and not path.is_symlink(), f"missing regular sealed file: {path}"
    )
    return path


def safe_key(raw: str) -> PurePosixPath:
    key = PurePosixPath(raw)
    require(
        not key.is_absolute()
        and key.parts
        and all(part not in (".", "..") for part in key.parts),
        f"unsafe seal path: {raw}",
    )
    require(str(key) == raw, f"noncanonical seal path: {raw}")
    return key


def _attestation_entries(attestations: dict[str, dict[str, Any]]) -> dict[str, str]:
    expected: dict[str, str] = {}

    def add(key: str, digest: str) -> None:
        safe_key(key)
        prior = expected.get(key)
        require(prior is None or prior == digest, f"conflicting attested hash: {key}")
        expected[key] = digest

    for version, attestation in attestations.items():
        add(f"attestations/{version}.json", ATTESTATION_SHA256[version])
        for relative, digest in attestation["harnessInputs"].items():
            add(f"repo/{relative}", digest)
        for relative, digest in attestation.get("candidateSourceSha256", {}).items():
            add(f"repo/{relative}", digest)
        for relative, digest in attestation.get("upstreamSourceSha256", {}).items():
            add(f"upstream/{relative}", digest)
        add(
            f"result-wheels/{version}/{BUILD_WHEEL.name}",
            attestation["candidateWheelSha256"],
        )
    for mode in ("candidate", "oracle"):
        digest = attestations["v1"]["predecessorLedgerSha256"][mode]
        require(
            attestations["v2"]["predecessorLedgerSha256"][mode] == digest,
            f"{mode} predecessor mismatch",
        )
        add(f"ledgers/{mode}.jsonl", digest)
    add(f"build-wheel/{BUILD_WHEEL.name}", attestations["v2"]["candidateWheelSha256"])
    return expected


def verify_seal(seal: Path = SEAL) -> dict[str, dict[str, Any]]:
    require(
        seal.is_dir() and not seal.is_symlink(), "seal directory missing or symlinked"
    )
    manifest_path = regular(seal / "seal.json")
    require(sha256(manifest_path) == SEAL_SHA256, "seal manifest SHA-256 changed")
    manifest = json.loads(manifest_path.read_text())
    require(
        isinstance(manifest, dict)
        and set(manifest)
        == {"schema", "kind", "classification", "attestationSha256", "entries"},
        "seal manifest fields changed",
    )
    require(
        manifest["schema"] == 1
        and manifest["kind"] == "python-remote-kill-v1-v2-input-seal",
        "seal schema changed",
    )
    require(
        manifest["classification"]
        == "official-derived-fixture-adapted-internal-diagnostic"
        and manifest["attestationSha256"] == ATTESTATION_SHA256,
        "seal classification or attestation identity changed",
    )
    entries = manifest["entries"]
    require(isinstance(entries, list) and entries, "seal entries missing")
    inputs_root = seal / "inputs"
    require(
        inputs_root.is_dir() and not inputs_root.is_symlink(),
        "seal inputs directory missing or symlinked",
    )
    indexed: dict[str, str] = {}
    for entry in entries:
        require(
            isinstance(entry, dict) and set(entry) == {"path", "sha256", "size"},
            "invalid seal entry",
        )
        key = str(safe_key(entry["path"]))
        require(key not in indexed, f"duplicate seal entry: {key}")
        parent = inputs_root
        for component in PurePosixPath(key).parts[:-1]:
            parent /= component
            require(
                parent.is_dir() and not parent.is_symlink(),
                f"sealed directory missing or symlinked: {key}",
            )
        path = regular(inputs_root / key)
        require(
            path.resolve().is_relative_to(inputs_root.resolve()),
            f"seal entry escaped inputs: {key}",
        )
        require(
            path.stat().st_size == entry["size"] and sha256(path) == entry["sha256"],
            f"sealed bytes changed: {key}",
        )
        indexed[key] = entry["sha256"]
    require(list(indexed) == sorted(indexed), "seal entries are not canonical")
    actual = {
        str(path.relative_to(seal / "inputs"))
        for path in (seal / "inputs").rglob("*")
        if path.is_file() or path.is_symlink()
    }
    require(actual == set(indexed), "seal has missing or unexpected input files")
    attestations: dict[str, dict[str, Any]] = {}
    for version in ("v1", "v2"):
        snapshot = regular(seal / "inputs/attestations" / f"{version}.json")
        live = regular(RESULTS[version] / "attestation.json")
        require(
            sha256(snapshot) == sha256(live) == ATTESTATION_SHA256[version],
            f"{version} attestation changed",
        )
        value = json.loads(snapshot.read_text())
        require(
            isinstance(value, dict) and value.get("schema") == 1,
            f"{version} attestation schema changed",
        )
        require(
            value.get("classification")
            == "official-derived-fixture-adapted-internal-diagnostic"
            and value.get("publicApiParityEvidence") is False
            and value.get("promotedToPythonLedger") is False,
            f"{version} diagnostic classification changed",
        )
        attestations[version] = value
    require(
        attestations["v2"]["predecessorRedAttestationSha256"]
        == ATTESTATION_SHA256["v1"],
        "v2 RED predecessor changed",
    )
    require(
        indexed == _attestation_entries(attestations),
        "seal entries do not match original attestations",
    )
    return attestations


def _git_object_bytes(repository: Path, commit: str, path: str) -> bytes:
    return subprocess.check_output(
        ["git", "-C", str(repository), "show", f"{commit}:{path}"]
    )


def verify_upstream_objects(seal: Path, attestation: dict[str, Any]) -> None:
    commit = attestation["upstreamCommit"]
    core_commit = attestation["coreProtocolCommit"]
    subprocess.check_call(
        ["git", "-C", str(UPSTREAM), "cat-file", "-e", f"{commit}^{{commit}}"],
        stdout=subprocess.DEVNULL,
    )
    subprocess.check_call(
        ["git", "-C", str(CORE), "cat-file", "-e", f"{core_commit}^{{commit}}"],
        stdout=subprocess.DEVNULL,
    )
    for relative, digest in attestation["upstreamSourceSha256"].items():
        blob = _git_object_bytes(UPSTREAM, commit, relative)
        require(
            hashlib.sha256(blob).hexdigest() == digest
            and blob == (seal / "inputs/upstream" / relative).read_bytes(),
            f"upstream snapshot is not the pinned Git object: {relative}",
        )


@contextmanager
def replace(module: types.ModuleType, **values: Any) -> Iterator[None]:
    old = {name: getattr(module, name) for name in values}
    try:
        for name, value in values.items():
            setattr(module, name, value)
        yield
    finally:
        for name, value in old.items():
            setattr(module, name, value)


def load_sealed_verifiers(
    seal: Path, attestations: dict[str, dict[str, Any]]
) -> tuple[types.ModuleType, types.ModuleType]:
    paths = {
        version: seal / "inputs/repo/scripts" / filename
        for version, filename in VERIFIER_NAMES.items()
    }

    def load(path: Path, name: str, expected: str) -> types.ModuleType:
        source = regular(path).read_bytes()
        require(
            hashlib.sha256(source).hexdigest() == expected,
            f"sealed verifier changed before execution: {path.name}",
        )
        module = types.ModuleType(name)
        module.__file__ = str(path)
        exec(compile(source, str(path), "exec"), module.__dict__)
        return module

    v1 = load(
        paths["v1"],
        "sealed_remote_kill_v1",
        attestations["v1"]["harnessInputs"][f"scripts/{VERIFIER_NAMES['v1']}"],
    )
    import_name = "verify_playwright_python_connect_remote_kill_synthetic_v1"
    prior = sys.modules.get(import_name)
    sys.modules[import_name] = v1
    try:
        v2 = load(
            paths["v2"],
            "sealed_remote_kill_v2",
            attestations["v2"]["harnessInputs"][f"scripts/{VERIFIER_NAMES['v2']}"],
        )
    finally:
        if prior is None:
            sys.modules.pop(import_name, None)
        else:
            sys.modules[import_name] = prior
    return v1, v2


def replay(seal: Path = SEAL) -> dict[str, dict[str, Any]]:
    attestations = verify_seal(seal)
    verify_upstream_objects(seal, attestations["v1"])
    v1, v2 = load_sealed_verifiers(seal, attestations)
    sealed_repo = seal / "inputs/repo"
    sealed_upstream = seal / "inputs/upstream"
    original_v1_sources = v1.verify_sources
    original_v1_harness = v1.harness_hashes
    original_v2_sources = v2.verify_sources
    original_v2_harness = v2.harness_hashes

    def snapshot_git(checkout: Path, *args: str) -> str:
        if checkout == sealed_upstream and args == ("rev-parse", "HEAD"):
            return v1.UPSTREAM_COMMIT
        if checkout == CORE and args == ("rev-parse", "HEAD"):
            return v1.CORE_COMMIT
        if checkout == sealed_upstream and args == ("status", "--porcelain"):
            return ""
        if checkout == CORE and args == (
            "status",
            "--porcelain",
            "--untracked-files=no",
        ):
            return ""
        raise ValueError(f"unrecognized sealed Git query: {checkout} {args}")

    def v1_sources() -> dict[str, str]:
        ledgers = {
            mode: (seal / "inputs/ledgers" / f"{mode}.jsonl", digest)
            for mode, digest in attestations["v1"]["predecessorLedgerSha256"].items()
        }
        with replace(
            v1,
            UPSTREAM=sealed_upstream,
            LEGACY_DIR=sealed_repo
            / "packages/pwright-python/tests/official/connect_close_events_synthetic_v1",
            MANIFEST=sealed_repo
            / "packages/pwright-python/tests/official/connect_remote_kill_synthetic_v1/manifest.json",
            LEDGERS=ledgers,
            _git=snapshot_git,
        ):
            return original_v1_sources()

    def v1_harness() -> dict[str, str]:
        paths = tuple(
            sealed_repo / relative for relative in attestations["v1"]["harnessInputs"]
        )
        with replace(v1, ROOT=sealed_repo, HARNESS_INPUTS=paths):
            return original_v1_harness()

    def v2_sources() -> dict[str, str]:
        with replace(
            v2,
            ROOT=sealed_repo,
            MANIFEST=sealed_repo
            / "packages/pwright-python/tests/official/connect_remote_kill_synthetic_v2/manifest.json",
            BUILD_WHEEL=seal / "inputs/build-wheel" / BUILD_WHEEL.name,
        ):
            return original_v2_sources()

    def v2_harness() -> dict[str, str]:
        paths = tuple(
            sealed_repo / relative for relative in attestations["v2"]["harnessInputs"]
        )
        with replace(v2, ROOT=sealed_repo, HARNESS_INPUTS=paths):
            return original_v2_harness()

    v1.ROOT = ROOT
    v1.RESULT_PARENT = BASE / "results"
    v1.UPSTREAM = UPSTREAM
    v1.CORE = CORE
    v1.PLUGIN_DIR = (
        ROOT / "packages/pwright-python/tests/official/connect_remote_kill_synthetic_v1"
    )
    v1.LEGACY_DIR = (
        ROOT
        / "packages/pwright-python/tests/official/connect_close_events_synthetic_v1"
    )
    v1.RUNTIME = ROOT / "packages/pwright-python/tests/official/runtime"
    v1.SHIM = ROOT / "packages/pwright-python/tests/official/shim"
    v1.SERVER = (
        ROOT
        / "packages/pwright-python/tests/official/connect_close_events_synthetic_v1/server.cjs"
    )
    v1.verify_sources = v1_sources
    v1.harness_hashes = v1_harness
    v2.ROOT = ROOT
    v2.RESULT_PARENT = BASE / "results"
    v2.BUILD_WHEEL = BUILD_WHEEL
    v2.PREDECESSOR = RESULTS["v1"]
    v2.verify_sources = v2_sources
    v2.harness_hashes = v2_harness

    replayed_v1 = v1.verify_directory(RESULTS["v1"])
    require(
        replayed_v1 == attestations["v1"],
        "v1 raw evidence no longer matches attestation",
    )
    replayed_v2 = v2.verify_directory(RESULTS["v2"])
    require(
        replayed_v2 == attestations["v2"],
        "v2 raw evidence no longer matches attestation",
    )
    require(
        replayed_v1["lanes"]["oracle"]["passed"] == 4
        and replayed_v1["lanes"]["candidate"]["passed"] == 2,
        "v1 RED outcome changed",
    )
    require(
        replayed_v2["lanes"]["oracle"]["passed"] == 4
        and all(
            replayed_v2["lanes"][f"candidate-{index}"]["passed"] == 4
            for index in range(1, 6)
        ),
        "v2 repeated candidate outcome changed",
    )
    return {"v1": replayed_v1, "v2": replayed_v2}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--seal", type=Path, default=SEAL)
    args = parser.parse_args()
    outcomes = replay(args.seal)
    print(
        "verified sealed fixture-adapted remote kill: "
        f"v1 oracle {outcomes['v1']['lanes']['oracle']['passed']}/4, "
        f"candidate {outcomes['v1']['lanes']['candidate']['passed']}/4; "
        f"v2 oracle {outcomes['v2']['lanes']['oracle']['passed']}/4, "
        "candidate repetitions 4/4 x 5; no official ledger credit"
    )


if __name__ == "__main__":
    main()
