"""Seal a browserless Core diagnostic with URL and candidate fixture-owner adapters.

The v1 URL adapter and the pre-existing BrowserType owner adapter are both
hash-pinned. The original Core runner and the sealed v1 evidence stay unchanged.
This diagnostic is not eligible for public-API ledger credit on its own.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any, cast

import run_playwright_core_runserver_url_only as base
from verify_playwright_official_provenance import verify_provenance

ROOT = base.ROOT
SUITE = base.SUITE
MANIFEST = (
    ROOT
    / "tests/official/playwright-core-server-connect-version-mismatch-url-owner-v2-manifest.json"
)
OWNER = ROOT / "tests/official/playwright-core-browser-type-owner-v2-preload.cjs"
PROFILE = "playwright-server-connect-session"
FORBIDDEN_AMBIENT = (
    "NODE_OPTIONS",
    "PWRIGHT_NODE_CANDIDATE",
    "PWRIGHT_NATIVE_PROFILE",
    "PWRIGHT_CANDIDATE_CDP_ENDPOINT",
    "PWRIGHT_ORACLE_CDP_ENDPOINT",
    "PWRIGHT_CDP_ENDPOINT",
    "PWRIGHT_CDP_WS_URL",
    "PWRIGHT_CORE_BROWSER_TYPE_OWNER_RECEIPT",
)


def check_manifest(upstream: Path) -> str:
    base.require_file(MANIFEST, ROOT / "tests/official")
    base.require_file(OWNER, ROOT / "tests/official")
    base.require_file(base.ADAPTER, ROOT / "tests/official")
    source = upstream / base.SOURCE
    base.require_file(source, upstream)
    source_sha = base.digest(source)
    expected = {
        "schema": 1,
        "repository": "https://github.com/microsoft/playwright.git",
        "tag": "v1.58.0",
        "commit": "961381ec73a9fa2661d92504eab1a6bd6eaffc75",
        "suites": [
            {
                "id": SUITE,
                "role": "browser-independent-runserver-url-only-owner-fixture-diagnostic",
                "runner": "packages/playwright/cli.js",
                "config": "tests/library/playwright.config.ts",
                "args": [base.SELECTOR],
                "candidateAdapter": "test-mode-playwright-core",
                "project": "chromium-library",
                "requiresBrowser": False,
                "expectedTestsPerProject": 1,
                "expectedFiles": 1,
                "sourceFileSha256": source_sha,
                "urlOnlyPreload": {
                    "path": str(base.ADAPTER.relative_to(ROOT)),
                    "sha256": base.digest(base.ADAPTER),
                },
                "ownerPreload": {
                    "path": str(OWNER.relative_to(ROOT)),
                    "sha256": base.digest(OWNER),
                },
            }
        ],
    }
    if json.loads(MANIFEST.read_text()) != expected:
        raise ValueError("v2 manifest or hash-pinned preload selection changed")
    return source_sha


def owner_receipt_path(result: Path) -> Path:
    return result.with_suffix(".owner-receipt.jsonl")


def envelope_path(result: Path) -> Path:
    return result.with_suffix(".runserver-url-owner-v2-attestation.json")


def node_options(mode: str) -> str:
    options = f"--require={base.ADAPTER}"
    if mode == "candidate":
        options += f" --require={OWNER}"
    elif mode != "oracle":
        raise ValueError("v2 mode must be candidate or oracle")
    return options


def check_ambient(environment: dict[str, str]) -> None:
    if any(environment.get(key, "").strip() for key in FORBIDDEN_AMBIENT) or any(
        key.startswith("PWRIGHT_OFFICIAL_RUNSERVER_") for key in environment
    ):
        raise ValueError(
            "ambient Node, CDP, candidate, or RunServer overrides are forbidden"
        )


def check_raw_identity(raw: dict[str, Any], envelope: dict[str, Any]) -> None:
    if (
        raw["mode"] != envelope["mode"]
        or raw["suite"] != SUITE
        or raw["suiteRole"]
        != "browser-independent-runserver-url-only-owner-fixture-diagnostic"
        or raw["manifest"] != str(MANIFEST)
        or raw["upstream"] != envelope["upstream"]
        or raw["result"] != envelope["result"]
        or raw["exitCode"] != envelope["runnerExitCode"]
        or raw["fixtureBind"] != envelope["bindIp"]
        or raw["fixtureHost"] != envelope["magicDnsHost"]
        or raw["candidateProfile"]
        != (PROFILE if envelope["mode"] == "candidate" else None)
        or raw["candidate"]
        != (str(ROOT / "packages/pwright") if envelope["mode"] == "candidate" else None)
    ):
        raise ValueError("raw provenance bind, host, profile, or selection differs")


def verify_owner_receipt(path: Path, candidate: str) -> int:
    base.require_file(path, ROOT / ".work/official-conformance")
    records = [json.loads(line) for line in path.read_text().splitlines()]
    if not records or not any(record.get("workerIndex") == "0" for record in records):
        raise ValueError("candidate worker did not attest BrowserType fixture owner")
    for record in records:
        if (
            not isinstance(record, dict)
            or set(record) != {"pid", "workerIndex", "candidate", "ownerBound"}
            or isinstance(record["pid"], bool)
            or not isinstance(record["pid"], int)
            or record["pid"] <= 0
            or record["workerIndex"] not in {None, "0"}
            or record["candidate"] != candidate
            or record["ownerBound"] is not True
        ):
            raise ValueError("BrowserType fixture-owner receipt changed")
    return len(records)


def harness_inputs() -> list[dict[str, str]]:
    paths = (
        Path(__file__).resolve(),
        Path(base.__file__).resolve(),
        base.ADAPTER,
        OWNER,
        ROOT / "scripts/verify_playwright_official_provenance.py",
    )
    return [{"path": str(path), "sha256": base.digest(path)} for path in paths]


def verify_envelope(path: Path) -> dict[str, Any]:
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
        "specId",
        "status",
    }
    if not isinstance(envelope, dict) or set(envelope) != expected_keys:
        raise ValueError("v2 envelope shape changed")
    result = Path(cast(str, envelope["result"])).resolve()
    upstream = Path(cast(str, envelope["upstream"])).resolve()
    if path.resolve() != envelope_path(result):
        raise ValueError("v2 envelope is not the immutable result sibling")
    source_sha = check_manifest(upstream)
    mode = cast(str, envelope["mode"])
    if (
        envelope["schema"] != 2
        or mode not in {"candidate", "oracle"}
        or envelope["suite"] != SUITE
        or envelope["manifest"] != str(MANIFEST)
        or envelope["manifestSha256"] != base.digest(MANIFEST)
        or envelope["sourceFileSha256"] != source_sha
        or envelope["nodeOptions"] != node_options(mode)
        or envelope["harnessInputs"] != harness_inputs()
        or envelope["candidateProfile"] != (PROFILE if mode == "candidate" else None)
    ):
        raise ValueError("v2 source, profile, or executable harness inputs changed")
    base.require_file(result, ROOT / ".work/official-conformance")
    raw_path = result.with_suffix(".provenance.json")
    receipt = base.receipt_path(result)
    for key, actual in (
        ("resultSha256", result),
        ("rawProvenanceSha256", raw_path),
        ("receiptSha256", receipt),
    ):
        if envelope[key] != base.digest(actual):
            raise ValueError(f"v2 evidence hash changed: {key}")
    if envelope["rawProvenance"] != str(raw_path) or envelope["receipt"] != str(
        receipt
    ):
        raise ValueError("v2 evidence sibling path changed")
    raw = verify_provenance(json.loads(raw_path.read_text()), provenance_path=raw_path)
    check_raw_identity(raw, envelope)
    bind, host = cast(str, envelope["bindIp"]), cast(str, envelope["magicDnsHost"])
    base.verified_receipt(receipt, bind, host)
    owner = owner_receipt_path(result)
    if mode == "candidate":
        if envelope["ownerReceipt"] != str(owner) or envelope[
            "ownerReceiptSha256"
        ] != base.digest(owner):
            raise ValueError("v2 candidate owner receipt changed")
        if envelope["ownerRecordCount"] != verify_owner_receipt(
            owner, cast(str, raw["candidate"])
        ):
            raise ValueError("v2 candidate owner receipt count changed")
    elif (
        any(
            envelope[key] is not None
            for key in ("ownerReceipt", "ownerReceiptSha256", "ownerRecordCount")
        )
        or owner.exists()
    ):
        raise ValueError("v2 oracle unexpectedly loaded candidate owner fixture")
    spec_id, status = base.selected_result(result)
    if envelope["specId"] != spec_id or envelope["status"] != status:
        raise ValueError("v2 unchanged official row differs")
    return envelope


def run(mode: str, upstream: Path, result: Path) -> int:
    check_ambient(dict(os.environ))
    upstream, result = upstream.resolve(), result.resolve()
    check_manifest(upstream)
    bind, host = base.tailscale_identity()
    siblings = (
        result,
        result.with_suffix(".provenance.json"),
        base.receipt_path(result),
        owner_receipt_path(result),
        envelope_path(result),
    )
    if any(path.exists() or path.is_symlink() for path in siblings):
        raise ValueError("v2 immutable evidence destination already exists")
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
            "PWRIGHT_OFFICIAL_RUNSERVER_RECEIPT": str(base.receipt_path(result)),
        }
    )
    if mode == "candidate":
        env["PWRIGHT_CORE_BROWSER_TYPE_OWNER_RECEIPT"] = str(owner_receipt_path(result))
    command = [
        sys.executable,
        str(base.RUNNER),
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
        command.extend(["--candidate-profile", PROFILE])
    completed = subprocess.run(command, cwd=ROOT, env=env, check=False)
    source_sha = check_manifest(upstream)
    if base.tailscale_identity() != (bind, host):
        raise ValueError("Tailscale identity changed during the v2 diagnostic")
    raw_path = result.with_suffix(".provenance.json")
    raw = verify_provenance(json.loads(raw_path.read_text()), provenance_path=raw_path)
    base.verified_receipt(base.receipt_path(result), bind, host)
    spec_id, status = base.selected_result(result)
    owner = owner_receipt_path(result)
    owner_count = (
        verify_owner_receipt(owner, cast(str, raw["candidate"]))
        if mode == "candidate"
        else None
    )
    envelope: dict[str, Any] = {
        "schema": 2,
        "mode": mode,
        "suite": SUITE,
        "manifest": str(MANIFEST),
        "manifestSha256": base.digest(MANIFEST),
        "upstream": str(upstream),
        "sourceFileSha256": source_sha,
        "result": str(result),
        "resultSha256": base.digest(result),
        "rawProvenance": str(raw_path),
        "rawProvenanceSha256": base.digest(raw_path),
        "runnerExitCode": completed.returncode,
        "nodeOptions": node_options(mode),
        "harnessInputs": harness_inputs(),
        "receipt": str(base.receipt_path(result)),
        "receiptSha256": base.digest(base.receipt_path(result)),
        "ownerReceipt": str(owner) if mode == "candidate" else None,
        "ownerReceiptSha256": base.digest(owner) if mode == "candidate" else None,
        "ownerRecordCount": owner_count,
        "bindIp": bind,
        "magicDnsHost": host,
        "candidateProfile": PROFILE if mode == "candidate" else None,
        "specId": spec_id,
        "status": status,
    }
    check_raw_identity(raw, envelope)
    with envelope_path(result).open("x") as stream:
        json.dump(envelope, stream, indent=2, sort_keys=True)
        stream.write("\n")
    verify_envelope(envelope_path(result))
    print(
        f"verified v2 diagnostic {mode} status={status} envelope={envelope_path(result)}"
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
                raise ValueError("verify accepts only the immutable envelope path")
            verified = verify_envelope(args.upstream_or_envelope)
            print(
                f"verified v2 diagnostic {verified['mode']} status={verified['status']}"
            )
            return
        if args.result is None:
            raise ValueError("v2 diagnostic requires --result")
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
