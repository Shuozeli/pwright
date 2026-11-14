"""Attest the isolated v3 browserless RunServer version-mismatch pair.

This diagnostic reuses the sealed v2 manifest and URL/owner preloads. It sends
the unchanged official Core runner to a hash-pinned candidate package snapshot,
never to the live package or an earlier diagnostic's evidence directory. The
candidate package and addon are read-only inputs; the only outputs are new v3
result siblings. This remains a fixture-adapted diagnostic, not ledger credit.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any, cast

import run_playwright_core_runserver_url_owner_v2 as base
from verify_playwright_official_provenance import verify_provenance

ROOT = base.ROOT
SUITE = base.SUITE
MANIFEST = base.MANIFEST
PROFILE = base.PROFILE
ARTIFACT_ROOT = (
    ROOT
    / ".work/official-conformance/artifacts/core-server-connect-version-mismatch-v3-final-20261020"
)
ADDON = ARTIFACT_ROOT / "pwright.node"
CANDIDATE = ARTIFACT_ROOT / "candidate-package"
CANDIDATE_ARTIFACT = CANDIDATE / "native" / PROFILE / "pwright.node"
RESULT_ROOT = (
    ROOT
    / ".work/official-conformance/results/core-server-connect-version-mismatch-url-owner-v3"
)
SERVER_RUNTIME = ROOT / "tests/official/playwright-core-server-runtime.cjs"
CANDIDATE_PRELOAD = ROOT / "tests/official/playwright-core-candidate-preload.cjs"

# These inputs are pinned to the previously sealed v2 diagnostic, not merely
# re-read from its current files. The v3 addon is a new release-mode build.
PINNED_V2_INPUTS = {
    MANIFEST: "08b1be8a61b205b045ec6031874ba17cdbd32ee749d764ce18c4d916f49a77ee",
    Path(
        base.__file__
    ).resolve(): "a3f7906872cccc84e2441f253393a9e348c5461665548933c124ae9cb0fb07de",
    Path(
        base.base.__file__
    ).resolve(): "e70d672827b4ffcbd55e5714c385360623da1637eb396213ad59b34784980e76",
    base.base.ADAPTER: "f8ad0fcc3aa6fdd606c81bb6b6f72d5e645fa312eb4fdeef63b80728cdc7f75f",
    base.OWNER: "b7dc4e3be3aa4c07ac069bc07adfa5a2e65456efc6fc438e49a84a907ffc06ce",
}
EXPECTED_ADDON_SHA256 = (
    "7a2664ef66516bcd1ada56b962a9f4a85abb05add8bcf3905113490fa9f39938"
)
EXPECTED_SOURCE_SHA256 = (
    "a2bb47a7c22f88b1c8cbaff72fb107de0a015eec85f6a4101d94e0cbddbfca5a"
)

selected_result = base.base.selected_result
verify_owner_receipt = base.verify_owner_receipt


def digest(path: Path) -> str:
    return base.base.digest(path)


def _exact_children(directory: Path, names: set[str]) -> None:
    if directory.is_symlink() or not directory.is_dir():
        raise ValueError(f"snapshot directory missing or linked: {directory}")
    children = list(directory.iterdir())
    if (
        any(child.is_symlink() for child in children)
        or {child.name for child in children} != names
    ):
        raise ValueError(f"snapshot directory contents changed: {directory}")


def _snapshot_source_digest() -> str:
    """Reproduce the unchanged official runner's package.json + src digest."""
    package = CANDIDATE / "package.json"
    source = CANDIDATE / "src"
    if package.is_symlink() or not package.is_file():
        raise ValueError("snapshot package.json is missing or linked")
    if source.is_symlink() or not source.is_dir():
        raise ValueError("snapshot src is missing or linked")
    entries = list(source.rglob("*"))
    if not entries or any(
        entry.is_symlink() or not (entry.is_file() or entry.is_dir())
        for entry in entries
    ):
        raise ValueError("snapshot src contains a missing or linked entry")
    files = [package, *(entry for entry in entries if entry.is_file())]
    value = hashlib.sha256()
    for path in sorted(files, key=lambda item: item.relative_to(CANDIDATE).as_posix()):
        relative = path.relative_to(CANDIDATE).as_posix().encode()
        value.update(len(relative).to_bytes(8, "big"))
        value.update(relative)
        value.update(path.stat().st_size.to_bytes(8, "big"))
        with path.open("rb") as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                value.update(chunk)
    return value.hexdigest()


def check_artifact_snapshot() -> dict[str, str]:
    """Fail closed if any copied package source or either addon has drifted."""
    _exact_children(ARTIFACT_ROOT, {"pwright.node", "candidate-package"})
    _exact_children(CANDIDATE, {"package.json", "src", "native"})
    _exact_children(CANDIDATE / "native", {PROFILE})
    _exact_children(CANDIDATE / "native" / PROFILE, {"pwright.node"})
    for path in (ADDON, CANDIDATE_ARTIFACT):
        if path.is_symlink() or not path.is_file():
            raise ValueError(f"snapshot addon missing or linked: {path}")
    addon_sha = digest(ADDON)
    candidate_artifact_sha = digest(CANDIDATE_ARTIFACT)
    source_sha = _snapshot_source_digest()
    if (
        addon_sha != EXPECTED_ADDON_SHA256
        or candidate_artifact_sha != EXPECTED_ADDON_SHA256
        or source_sha != EXPECTED_SOURCE_SHA256
    ):
        raise ValueError("v3 candidate addon or source snapshot SHA changed")
    return {
        "artifactRoot": str(ARTIFACT_ROOT),
        "addon": str(ADDON),
        "addonSha256": addon_sha,
        "candidate": str(CANDIDATE),
        "candidateSourceSha256": source_sha,
        "candidateArtifact": str(CANDIDATE_ARTIFACT),
        "candidateArtifactSha256": candidate_artifact_sha,
    }


def check_manifest(upstream: Path) -> str:
    for path, expected in PINNED_V2_INPUTS.items():
        base.base.require_file(path, ROOT)
        if digest(path) != expected:
            raise ValueError(f"sealed v2 input changed: {path}")
    return base.check_manifest(upstream)


def check_ambient(environment: dict[str, str]) -> None:
    base.check_ambient(environment)
    if (
        environment.get("NODE_PATH", "").strip()
        or any(key.startswith("PWRIGHT_OFFICIAL_") for key in environment)
        or environment.get("PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM", "").strip()
    ):
        raise ValueError("ambient Node or official-runner overrides are forbidden")


def node_options(mode: str) -> str:
    return base.node_options(mode)


def expected_raw_harness_inputs(mode: str) -> list[dict[str, str]]:
    if mode not in {"candidate", "oracle"}:
        raise ValueError("v3 mode must be candidate or oracle")
    paths = [SERVER_RUNTIME]
    if mode == "candidate":
        paths.append(CANDIDATE_PRELOAD)
    return [{"path": str(path), "sha256": digest(path)} for path in paths]


def harness_inputs() -> list[dict[str, str]]:
    paths = (
        Path(__file__).resolve(),
        Path(base.__file__).resolve(),
        Path(base.base.__file__).resolve(),
        MANIFEST,
        base.base.ADAPTER,
        base.OWNER,
        base.base.RUNNER,
        SERVER_RUNTIME,
        CANDIDATE_PRELOAD,
        ROOT / "scripts/verify_playwright_official_provenance.py",
    )
    return [{"path": str(path), "sha256": digest(path)} for path in paths]


def receipt_path(result: Path) -> Path:
    return base.base.receipt_path(result)


def owner_receipt_path(result: Path) -> Path:
    return base.owner_receipt_path(result)


def envelope_path(result: Path) -> Path:
    return result.with_suffix(".runserver-url-owner-v3-attestation.json")


def result_siblings(result: Path) -> tuple[Path, ...]:
    return (
        result,
        result.with_suffix(".provenance.json"),
        receipt_path(result),
        owner_receipt_path(result),
        envelope_path(result),
    )


def check_result_location(mode: str, result: Path) -> None:
    if mode not in {"candidate", "oracle"}:
        raise ValueError("v3 mode must be candidate or oracle")
    if result.is_symlink() or result.absolute() != result.resolve():
        raise ValueError("v3 result path must not contain a symlink")
    if result.name != f"{mode}.json" or result.parent.parent != RESULT_ROOT:
        raise ValueError("v3 result must be a named child of the isolated result root")


def check_immutable_destination(mode: str, result: Path) -> None:
    check_result_location(mode, result)
    for path in result_siblings(result):
        if path.exists() or path.is_symlink():
            raise ValueError(
                f"v3 immutable evidence destination already exists: {path}"
            )


def check_raw_identity(raw: dict[str, Any], envelope: dict[str, Any]) -> None:
    mode = envelope["mode"]
    snapshot = envelope["snapshot"]
    candidate = mode == "candidate"
    expected = {
        "mode": mode,
        "suite": SUITE,
        "suiteRole": "browser-independent-runserver-url-only-owner-fixture-diagnostic",
        "manifest": str(MANIFEST),
        "manifestSha256": digest(MANIFEST),
        "upstream": envelope["upstream"],
        "result": envelope["result"],
        "resultSha256": envelope["resultSha256"],
        "exitCode": envelope["runnerExitCode"],
        "fixtureBind": envelope["bindIp"],
        "fixtureHost": envelope["magicDnsHost"],
        "candidateProfile": PROFILE if candidate else None,
        "candidate": snapshot["candidate"] if candidate else None,
        "candidateSourceSha256": (
            snapshot["candidateSourceSha256"] if candidate else None
        ),
        "candidateArtifact": snapshot["candidateArtifact"] if candidate else None,
        "candidateArtifactSha256": (
            snapshot["candidateArtifactSha256"] if candidate else None
        ),
        "harnessInputs": expected_raw_harness_inputs(mode),
    }
    if any(raw.get(key) != value for key, value in expected.items()):
        raise ValueError(
            "raw provenance bind, owner, candidate snapshot, or input differs"
        )


def verify_envelope(path: Path) -> dict[str, Any]:
    if path.is_symlink() or not path.is_file():
        raise ValueError("v3 attestation is missing or linked")
    envelope = json.loads(path.read_text())
    expected_keys = {
        "schema",
        "mode",
        "suite",
        "manifest",
        "manifestSha256",
        "upstream",
        "sourceFileSha256",
        "result",
        "resultSha256",
        "rawProvenance",
        "rawProvenanceSha256",
        "runnerExitCode",
        "nodeOptions",
        "harnessInputs",
        "receipt",
        "receiptSha256",
        "ownerReceipt",
        "ownerReceiptSha256",
        "ownerRecordCount",
        "bindIp",
        "magicDnsHost",
        "candidateProfile",
        "snapshot",
        "specId",
        "status",
    }
    if not isinstance(envelope, dict) or set(envelope) != expected_keys:
        raise ValueError("v3 envelope shape changed")
    mode = envelope["mode"]
    if mode not in {"candidate", "oracle"}:
        raise ValueError("v3 mode changed")
    result = Path(cast(str, envelope["result"]))
    upstream = Path(cast(str, envelope["upstream"]))
    check_result_location(mode, result)
    if path.resolve() != envelope_path(result):
        raise ValueError("v3 envelope is not the immutable result sibling")
    source_sha = check_manifest(upstream)
    snapshot = check_artifact_snapshot()
    if (
        envelope["schema"] != 3
        or envelope["suite"] != SUITE
        or envelope["manifest"] != str(MANIFEST)
        or envelope["manifestSha256"] != digest(MANIFEST)
        or envelope["sourceFileSha256"] != source_sha
        or envelope["nodeOptions"] != node_options(mode)
        or envelope["harnessInputs"] != harness_inputs()
        or envelope["snapshot"] != snapshot
        or envelope["candidateProfile"] != (PROFILE if mode == "candidate" else None)
    ):
        raise ValueError("v3 source, snapshot, profile, or executable inputs changed")
    raw_path = result.with_suffix(".provenance.json")
    receipt = receipt_path(result)
    for key, actual in (
        ("resultSha256", result),
        ("rawProvenanceSha256", raw_path),
        ("receiptSha256", receipt),
    ):
        base.base.require_file(actual, RESULT_ROOT)
        if envelope[key] != digest(actual):
            raise ValueError(f"v3 evidence hash changed: {key}")
    if envelope["rawProvenance"] != str(raw_path) or envelope["receipt"] != str(
        receipt
    ):
        raise ValueError("v3 evidence sibling path changed")
    raw = verify_provenance(json.loads(raw_path.read_text()), provenance_path=raw_path)
    check_raw_identity(raw, envelope)
    bind, host = cast(str, envelope["bindIp"]), cast(str, envelope["magicDnsHost"])
    base.base.verified_receipt(receipt, bind, host)
    owner = owner_receipt_path(result)
    if mode == "candidate":
        base.base.require_file(owner, RESULT_ROOT)
        if envelope["ownerReceipt"] != str(owner) or envelope[
            "ownerReceiptSha256"
        ] != digest(owner):
            raise ValueError("v3 candidate owner receipt changed")
        if envelope["ownerRecordCount"] != verify_owner_receipt(
            owner, cast(str, raw["candidate"])
        ):
            raise ValueError("v3 candidate owner receipt count changed")
    elif (
        any(
            envelope[key] is not None
            for key in ("ownerReceipt", "ownerReceiptSha256", "ownerRecordCount")
        )
        or owner.exists()
        or owner.is_symlink()
    ):
        raise ValueError("v3 oracle unexpectedly loaded candidate owner fixture")
    spec_id, status = selected_result(result)
    if envelope["specId"] != spec_id or envelope["status"] != status:
        raise ValueError("v3 unchanged official row differs")
    return envelope


def run(mode: str, upstream: Path, result: Path) -> int:
    check_ambient(dict(os.environ))
    upstream, result = upstream.resolve(), result.absolute()
    check_result_location(mode, result)
    check_immutable_destination(mode, result)
    source_sha = check_manifest(upstream)
    snapshot = check_artifact_snapshot()
    bind, host = base.base.tailscale_identity()
    result.parent.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    env.update(
        {
            "TAILSCALE_IP": bind,
            "TAILSCALE_HOST": host,
            "NODE_OPTIONS": node_options(mode),
            "PWRIGHT_OFFICIAL_RUNSERVER_URL_SUITE": SUITE,
            "PWRIGHT_OFFICIAL_RUNSERVER_EXPECT_BIND": bind,
            "PWRIGHT_OFFICIAL_RUNSERVER_EXPECT_HOST": host,
            "PWRIGHT_OFFICIAL_RUNSERVER_RECEIPT": str(receipt_path(result)),
        }
    )
    if mode == "candidate":
        env["PWRIGHT_CORE_BROWSER_TYPE_OWNER_RECEIPT"] = str(owner_receipt_path(result))
    command = [
        sys.executable,
        str(base.base.RUNNER),
        mode,
        str(upstream),
        "--manifest",
        str(MANIFEST),
        "--suite",
        SUITE,
        "--result",
        str(result),
    ]
    if mode == "candidate":
        command.extend(["--candidate", str(CANDIDATE), "--candidate-profile", PROFILE])
    completed = subprocess.run(command, cwd=ROOT, env=env, check=False)
    if base.base.tailscale_identity() != (bind, host):
        raise ValueError("Tailscale identity changed during the v3 diagnostic")
    if check_manifest(upstream) != source_sha or check_artifact_snapshot() != snapshot:
        raise ValueError("v3 input changed during execution")
    raw_path = result.with_suffix(".provenance.json")
    raw = verify_provenance(json.loads(raw_path.read_text()), provenance_path=raw_path)
    base.base.verified_receipt(receipt_path(result), bind, host)
    spec_id, status = selected_result(result)
    owner = owner_receipt_path(result)
    owner_count = (
        verify_owner_receipt(owner, cast(str, raw["candidate"]))
        if mode == "candidate"
        else None
    )
    envelope: dict[str, Any] = {
        "schema": 3,
        "mode": mode,
        "suite": SUITE,
        "manifest": str(MANIFEST),
        "manifestSha256": digest(MANIFEST),
        "upstream": str(upstream),
        "sourceFileSha256": source_sha,
        "result": str(result),
        "resultSha256": digest(result),
        "rawProvenance": str(raw_path),
        "rawProvenanceSha256": digest(raw_path),
        "runnerExitCode": completed.returncode,
        "nodeOptions": node_options(mode),
        "harnessInputs": harness_inputs(),
        "receipt": str(receipt_path(result)),
        "receiptSha256": digest(receipt_path(result)),
        "ownerReceipt": str(owner) if mode == "candidate" else None,
        "ownerReceiptSha256": digest(owner) if mode == "candidate" else None,
        "ownerRecordCount": owner_count,
        "bindIp": bind,
        "magicDnsHost": host,
        "candidateProfile": PROFILE if mode == "candidate" else None,
        "snapshot": snapshot,
        "specId": spec_id,
        "status": status,
    }
    check_raw_identity(raw, envelope)
    with envelope_path(result).open("x") as stream:
        json.dump(envelope, stream, indent=2, sort_keys=True)
        stream.write("\n")
    verify_envelope(envelope_path(result))
    print(
        f"verified v3 diagnostic {mode} status={status} envelope={envelope_path(result)}",
        flush=True,
    )
    return completed.returncode


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=("candidate", "oracle", "verify"))
    parser.add_argument("upstream_or_envelope", type=Path)
    parser.add_argument("--result", type=Path)
    args = parser.parse_args()
    try:
        if args.mode == "verify":
            if args.result is not None:
                raise ValueError("verify accepts only the immutable v3 envelope path")
            envelope = verify_envelope(args.upstream_or_envelope)
            print(
                f"verified v3 diagnostic {envelope['mode']} status={envelope['status']}"
            )
            return
        if args.result is None:
            raise ValueError("v3 diagnostic requires --result")
        raise SystemExit(run(args.mode, args.upstream_or_envelope, args.result))
    except (
        OSError,
        TypeError,
        ValueError,
        json.JSONDecodeError,
        subprocess.CalledProcessError,
    ) as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
