"""Attest one browserless upstream RunServer URL-only Tailscale adaptation.

This wrapper leaves the existing Core runner and its sealed provenance format
unchanged. Its separate envelope binds the extra preload, actual socket bind,
reported endpoint, original runner evidence, and unchanged official test source.
"""

from __future__ import annotations

import argparse
import hashlib
import ipaddress
import json
import os
import socket
import subprocess
import sys
from pathlib import Path
from typing import cast
from urllib.parse import urlsplit

from run_playwright_official_conformance import _tailscale_runtime
from verify_playwright_official_provenance import verify_provenance

ROOT = Path(__file__).parents[1].resolve()
SUITE = "library-server-connect-version-mismatch-chromium-url-v1"
MANIFEST = (
    ROOT
    / "tests/official/playwright-core-server-connect-version-mismatch-url-v1-manifest.json"
)
RUNNER = ROOT / "scripts/run_playwright_official_conformance.py"
SOURCE = "tests/library/browsertype-connect.spec.ts"
SELECTOR = f"{SOURCE}:1046"
TITLE = "should refuse connecting when versions do not match"
ADAPTER = ROOT / "tests/official/playwright-core-runserver-url-only-v1-preload.cjs"


def digest(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def require_file(path: Path, parent: Path) -> None:
    if (
        path.is_symlink()
        or not path.is_file()
        or not path.resolve().is_relative_to(parent)
    ):
        raise ValueError(f"missing, linked, or out-of-scope evidence file: {path}")


def pinned_suite(upstream: Path) -> tuple[dict[str, object], str]:
    require_file(MANIFEST, ROOT / "tests/official")
    require_file(ADAPTER, ROOT / "tests/official")
    manifest = json.loads(MANIFEST.read_text())
    if (
        manifest.get("schema") != 1
        or manifest.get("commit") != "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
        or not isinstance(manifest.get("suites"), list)
        or len(manifest["suites"]) != 1
    ):
        raise ValueError("RunServer URL-only manifest identity changed")
    suite = cast(dict[str, object], manifest["suites"][0])
    expected = {
        "id": SUITE,
        "role": "browser-independent-runserver-url-only-diagnostic",
        "runner": "packages/playwright/cli.js",
        "config": "tests/library/playwright.config.ts",
        "args": [SELECTOR],
        "candidateAdapter": "test-mode-playwright-core",
        "project": "chromium-library",
        "requiresBrowser": False,
        "expectedTestsPerProject": 1,
        "expectedFiles": 1,
    }
    if any(suite.get(key) != value for key, value in expected.items()):
        raise ValueError("RunServer URL-only selection changed")
    preload = suite.get("urlOnlyPreload")
    if preload != {
        "path": str(ADAPTER.relative_to(ROOT)),
        "sha256": digest(ADAPTER),
    }:
        raise ValueError("RunServer URL-only preload path or SHA changed")
    source = upstream / SOURCE
    require_file(source, upstream)
    source_sha = digest(source)
    if suite.get("sourceFileSha256") != source_sha:
        raise ValueError("upstream RunServer test source changed")
    return suite, source_sha


def tailscale_identity() -> tuple[str, str]:
    # Do not trust ambient TAILSCALE_IP/HOST overrides for the bound socket.
    bind, host = _tailscale_runtime({})
    addresses = {
        value[4][0]
        for value in socket.getaddrinfo(host, None, socket.AF_INET, socket.SOCK_STREAM)
    }
    if bind not in addresses:
        raise ValueError("full Tailscale MagicDNS name does not resolve to the bind IP")
    return bind, host


def selected_result(result: Path) -> tuple[str, str]:
    report = json.loads(result.read_text())
    found: list[dict[str, object]] = []

    def visit(suite: object) -> None:
        if not isinstance(suite, dict):
            raise TypeError("malformed official suite")
        for spec in suite.get("specs", []):
            if not isinstance(spec, dict):
                raise TypeError("malformed official spec")
            found.append(spec)
        for child in suite.get("suites", []):
            visit(child)

    if not isinstance(report, dict) or not isinstance(report.get("suites"), list):
        raise TypeError("official result has no suites")
    for suite in report["suites"]:
        visit(suite)
    if len(found) != 1:
        raise ValueError(f"expected exactly one official spec, got {len(found)}")
    spec = found[0]
    tests = spec.get("tests")
    if (
        spec.get("file") != "library/browsertype-connect.spec.ts"
        or spec.get("line") != 1046
        or spec.get("title") != TITLE
        or not isinstance(spec.get("id"), str)
        or not isinstance(tests, list)
        or len(tests) != 1
        or not isinstance(tests[0], dict)
        or tests[0].get("projectId") != "chromium-library"
    ):
        raise ValueError("official result selection differs from the unchanged row")
    test = tests[0]
    attempts = test.get("results")
    if (
        test.get("status") not in {"expected", "unexpected", "flaky", "skipped"}
        or not isinstance(attempts, list)
        or len(attempts) != 1
        or not isinstance(attempts[0], dict)
        or attempts[0].get("status")
        not in {"passed", "failed", "timedOut", "skipped", "interrupted"}
    ):
        raise ValueError("official row has no exact executed attempt")
    return cast(str, spec["id"]), cast(str, test["status"])


def verified_receipt(path: Path, bind: str, host: str) -> dict[str, object]:
    receipt = json.loads(path.read_text())
    if not isinstance(receipt, dict) or set(receipt) != {
        "schema",
        "suite",
        "pid",
        "boundAddress",
        "boundPort",
        "sourceEndpoint",
        "adaptedEndpoint",
    }:
        raise ValueError("RunServer bind/URL receipt shape changed")
    port = receipt["boundPort"]
    if (
        receipt["schema"] != 1
        or receipt["suite"] != SUITE
        or isinstance(receipt["pid"], bool)
        or not isinstance(receipt["pid"], int)
        or receipt["pid"] < 1
        or isinstance(port, bool)
        or not isinstance(port, int)
        or not 1 <= port <= 65535
        or receipt["boundAddress"] != bind
        or receipt["sourceEndpoint"] != f"ws://localhost:{port}/"
        or receipt["adaptedEndpoint"] != f"ws://{host}:{port}/"
        or ipaddress.ip_address(bind).is_loopback
    ):
        raise ValueError(
            "RunServer was not proven bound to Tailscale with URL-only adaptation"
        )
    if urlsplit(cast(str, receipt["adaptedEndpoint"])).hostname != host:
        raise ValueError("RunServer adapted endpoint is not full MagicDNS")
    return receipt


def envelope_path(result: Path) -> Path:
    return result.with_suffix(".runserver-url-attestation.json")


def receipt_path(result: Path) -> Path:
    return result.with_suffix(".runserver-url-receipt.json")


def verify_envelope(path: Path) -> dict[str, object]:
    envelope = json.loads(path.read_text())
    if not isinstance(envelope, dict) or set(envelope) != {
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
        "bindIp",
        "magicDnsHost",
        "specId",
        "status",
    }:
        raise ValueError("RunServer attestation envelope shape changed")
    result = Path(cast(str, envelope["result"])).resolve()
    if path.resolve() != envelope_path(result):
        raise ValueError("attestation is not the result's immutable sibling")
    upstream = Path(cast(str, envelope["upstream"])).resolve()
    _, source_sha = pinned_suite(upstream)
    if (
        envelope["schema"] != 1
        or envelope["suite"] != SUITE
        or envelope["mode"] not in {"candidate", "oracle"}
        or envelope["manifest"] != str(MANIFEST)
        or envelope["manifestSha256"] != digest(MANIFEST)
        or envelope["sourceFileSha256"] != source_sha
        or envelope["nodeOptions"] != f"--require={ADAPTER}"
    ):
        raise ValueError("RunServer manifest, source, or clean NODE_OPTIONS differs")
    expected_inputs = [
        {
            "path": str(Path(__file__).resolve()),
            "sha256": digest(Path(__file__).resolve()),
        },
        {"path": str(ADAPTER), "sha256": digest(ADAPTER)},
    ]
    if envelope["harnessInputs"] != expected_inputs:
        raise ValueError("RunServer wrapper/preload harness input SHA changed")
    require_file(result, ROOT / ".work/official-conformance")
    raw_path = result.with_suffix(".provenance.json")
    receipt = receipt_path(result)
    for value, actual, label in (
        ("resultSha256", result, "result"),
        ("rawProvenanceSha256", raw_path, "raw provenance"),
        ("receiptSha256", receipt, "bind/URL receipt"),
    ):
        if envelope[value] != digest(actual):
            raise ValueError(f"{label} SHA changed")
    if envelope["rawProvenance"] != str(raw_path) or envelope["receipt"] != str(
        receipt
    ):
        raise ValueError("RunServer evidence sibling path changed")
    raw = verify_provenance(json.loads(raw_path.read_text()), provenance_path=raw_path)
    if (
        raw["mode"] != envelope["mode"]
        or raw["suite"] != SUITE
        or raw["manifest"] != str(MANIFEST)
        or raw["upstream"] != str(upstream)
        or raw["result"] != str(result)
        or raw["exitCode"] != envelope["runnerExitCode"]
    ):
        raise ValueError("RunServer raw runner provenance differs from the envelope")
    bind = cast(str, envelope["bindIp"])
    host = cast(str, envelope["magicDnsHost"])
    verified_receipt(receipt, bind, host)
    spec_id, status = selected_result(result)
    if envelope["specId"] != spec_id or envelope["status"] != status:
        raise ValueError("RunServer selected official outcome changed")
    return envelope


def run(mode: str, upstream: Path, profile: str | None, result: Path) -> int:
    if os.environ.get("NODE_OPTIONS", "").strip():
        raise ValueError("ambient NODE_OPTIONS is forbidden for this strict suite")
    if any(key.startswith("PWRIGHT_OFFICIAL_RUNSERVER_") for key in os.environ):
        raise ValueError("ambient RunServer attestation variables are forbidden")
    upstream = upstream.resolve()
    result = result.resolve()
    pinned_suite(upstream)
    bind, host = tailscale_identity()
    for path in (
        result,
        result.with_suffix(".provenance.json"),
        receipt_path(result),
        envelope_path(result),
    ):
        if path.exists() or path.is_symlink():
            raise ValueError(f"immutable result destination already exists: {path}")
    if mode == "candidate" and not profile:
        raise ValueError("candidate run needs an explicit installed native profile")
    if mode == "oracle" and profile:
        raise ValueError("oracle run must not select a candidate profile")
    env = os.environ.copy()
    env["TAILSCALE_IP"] = bind
    env["TAILSCALE_HOST"] = host
    env["NODE_OPTIONS"] = f"--require={ADAPTER}"
    env["PWRIGHT_OFFICIAL_RUNSERVER_URL_SUITE"] = SUITE
    env["PWRIGHT_OFFICIAL_RUNSERVER_EXPECT_BIND"] = bind
    env["PWRIGHT_OFFICIAL_RUNSERVER_EXPECT_HOST"] = host
    env["PWRIGHT_OFFICIAL_RUNSERVER_RECEIPT"] = str(receipt_path(result))
    command = [
        sys.executable,
        str(RUNNER),
        mode,
        str(upstream),
        "--manifest",
        str(MANIFEST),
        "--suite",
        SUITE,
        "--result",
        str(result),
    ]
    if profile:
        command.extend(["--candidate-profile", profile])
    completed = subprocess.run(command, cwd=ROOT, env=env, check=False)
    if not result.is_file() or not result.with_suffix(".provenance.json").is_file():
        raise ValueError("unchanged Core runner did not produce result and provenance")
    _, source_sha = pinned_suite(upstream)
    if (fresh := tailscale_identity()) != (bind, host):
        raise ValueError(f"Tailscale identity changed during the strict run: {fresh}")
    raw_path = result.with_suffix(".provenance.json")
    raw = verify_provenance(json.loads(raw_path.read_text()), provenance_path=raw_path)
    if raw["exitCode"] != completed.returncode:
        raise ValueError("raw runner exit code differs from executed process")
    verified_receipt(receipt_path(result), bind, host)
    spec_id, status = selected_result(result)
    envelope = {
        "schema": 1,
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
        "nodeOptions": f"--require={ADAPTER}",
        "harnessInputs": [
            {
                "path": str(Path(__file__).resolve()),
                "sha256": digest(Path(__file__).resolve()),
            },
            {"path": str(ADAPTER), "sha256": digest(ADAPTER)},
        ],
        "receipt": str(receipt_path(result)),
        "receiptSha256": digest(receipt_path(result)),
        "bindIp": bind,
        "magicDnsHost": host,
        "specId": spec_id,
        "status": status,
    }
    path = envelope_path(result)
    with path.open("x") as stream:
        json.dump(envelope, stream, indent=2, sort_keys=True)
        stream.write("\n")
    verify_envelope(path)
    print(
        f"verified RunServer URL-only attestation={path} outcome={status}", flush=True
    )
    return completed.returncode


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=("candidate", "oracle", "verify"))
    parser.add_argument("upstream_or_envelope", type=Path)
    parser.add_argument("--result", type=Path)
    parser.add_argument("--candidate-profile")
    args = parser.parse_args()
    try:
        if args.mode == "verify":
            if args.result or args.candidate_profile:
                raise ValueError("verification takes only the immutable envelope path")
            envelope = verify_envelope(args.upstream_or_envelope)
            print(
                f"verified RunServer URL-only attestation mode={envelope['mode']} status={envelope['status']}"
            )
            return
        if args.result is None:
            raise ValueError("strict run requires --result")
        raise SystemExit(
            run(
                args.mode,
                args.upstream_or_envelope,
                args.candidate_profile,
                args.result,
            )
        )
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
