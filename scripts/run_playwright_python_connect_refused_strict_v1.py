"""Capture one official, fixture-adapted closed-endpoint connect assertion.

The official test body is unchanged. A guarded replacement for its
``launch_server`` RemoteServer starts a real Tailscale-bound TCP listener and
closes it before the official public connect call. This does not launch a
browser, nor does it prove that a remote Browser can be constructed.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import secrets
import subprocess
import sys
from pathlib import Path
from typing import NoReturn

from run_playwright_python_ws_transport_strict import (
    _artifact,
    _clean_ambient,
    _fixture_port,
    _probe_sandbox,
    _selected_python,
    _tailscale_fixture,
)
from verify_playwright_python_connect_refused_strict_v1 import (
    COMMIT,
    GUARD,
    KIND,
    NAME,
    OFFICIAL,
    POLICY,
    POLICY_SHA256,
    ROOT,
    SCOPE,
    TEST_IDS,
    StrictConnectRefusedV1EvidenceError,
    _candidate_feature_probe,
    _method_probe,
    _policy,
    _receipt_events,
    _runtime,
    _sources,
    _upstream,
    report_outcome,
    sandbox_profile,
    sha256_file,
    verify_provenance,
)


def _fail(message: str) -> NoReturn:
    raise StrictConnectRefusedV1EvidenceError(message)


def _fresh_result(path: Path) -> tuple[Path, Path, Path]:
    result = path.parent.resolve() / path.name
    if not result.is_absolute() or result.suffix != ".xml":
        _fail("strict connect-refused result must be an absolute XML destination")
    provenance = result.with_suffix(".provenance.json")
    receipt = result.with_suffix(".call.jsonl")
    for output in (result, provenance, receipt):
        if output.is_symlink() or output.exists():
            _fail(f"strict connect-refused output already exists: {output}")
    result.parent.mkdir(parents=True, exist_ok=True)
    result.open("xb").close()
    receipt.open("xb").close()
    return result, provenance, receipt


def run(args: argparse.Namespace) -> Path:
    if sys.platform != "darwin" or not Path("/usr/bin/sandbox-exec").is_file():
        _fail("strict connect-refused capture requires the macOS sandbox")
    _clean_ambient()
    if (
        POLICY.is_symlink()
        or not POLICY.is_file()
        or sha256_file(POLICY) != POLICY_SHA256
    ):
        _fail("strict connect-refused policy is missing or not frozen")
    policy = _policy(json.loads(POLICY.read_text()))
    upstream = args.upstream.resolve()
    _upstream(policy, upstream)
    _sources(ROOT, policy["supportSources"], "support")
    _sources(ROOT, policy["captureSourceSha256"], "capture")
    test_id = TEST_IDS[args.case]
    source, _, selected = test_id.partition("::")
    engine = selected.rsplit("[", 1)[1].removesuffix("]")
    python, _, site = _selected_python(args.python, policy["pythonRuntimeSha256"])
    wheel = args.wheel.absolute()
    artifact = _artifact(args.mode, python, site, wheel)
    if artifact["wheelSha256"] != policy["candidateWheelSha256"]:
        _fail("strict connect-refused capture requires the frozen candidate wheel")
    _runtime(policy, {**artifact, "mode": args.mode})
    if args.mode == "candidate" and (
        artifact["pwrightPackageSha256"] != policy["candidatePackageTreeSha256"]
        or artifact["nativeSha256"] != policy["candidateNativeSha256"]
        or artifact["runtimeLibraries"] != policy["candidateRuntimeLibraries"]
    ):
        _fail("strict connect-refused installed wheel differs from policy")
    probe_payload = {"pythonExecutable": str(python), "mode": args.mode}
    if not _method_probe(probe_payload) or not _candidate_feature_probe(
        probe_payload, policy
    ):
        _fail(
            "strict connect-refused capture requires ABI123 public BrowserType.connect"
        )
    bind, host = _tailscale_fixture()
    port = _fixture_port(bind)
    profile, allowed = sandbox_profile(
        python, Path(artifact["driverNode"]), args.mode, port
    )
    probe = _probe_sandbox(profile, python, bind, host, port)
    result, provenance, receipt = _fresh_result(args.result)
    nonce = secrets.token_hex(16)
    paths = (
        [OFFICIAL / "shim", upstream, OFFICIAL / "runtime", GUARD]
        if args.mode == "candidate"
        else [OFFICIAL / "runtime", GUARD]
    )
    environment = {
        "PATH": "/opt/homebrew/bin:/usr/bin:/bin",
        "PYTHONPATH": ":".join(map(str, paths)),
        "PYTHONNOUSERSITE": "1",
        "PYTEST_DISABLE_PLUGIN_AUTOLOAD": "1",
        "PWRIGHT_OFFICIAL_MODE": args.mode,
        "PWRIGHT_OFFICIAL_LAUNCH_MODE": "owned",
        "PWRIGHT_OFFICIAL_BROWSER_CHANNEL": "",
        "PWRIGHT_OFFICIAL_UPSTREAM_TESTS": str(upstream / "tests"),
        "PWRIGHT_OFFICIAL_TAILSCALE_IP": bind,
        "PWRIGHT_OFFICIAL_TAILSCALE_HOST": host,
        "PWRIGHT_PYTHON_REFUSED_V1_EXPECTED_TEST": test_id,
        "PWRIGHT_PYTHON_REFUSED_V1_UPSTREAM": str(upstream),
        "PWRIGHT_PYTHON_REFUSED_V1_PORT": str(port),
        "PWRIGHT_PYTHON_REFUSED_V1_RECEIPT": str(receipt),
        "PWRIGHT_PYTHON_REFUSED_V1_NONCE": nonce,
    }
    command = [
        "/usr/bin/sandbox-exec",
        "-p",
        profile,
        str(python),
        "-m",
        "pytest",
        "-q",
        "--import-mode=importlib",
        "-c",
        str(upstream / "pyproject.toml"),
        "-p",
        "pytest_asyncio.plugin",
        "-p",
        "pytest_timeout",
        "-p",
        "pwright_official_server_runtime",
        "-p",
        "pwright_candidate_runtime"
        if args.mode == "candidate"
        else "pwright_official_oracle_runtime",
        "-p",
        "pwright_official_connect_refused_guard_v1",
        f"--browser={engine}",
        "--timeout=20",
        "--timeout-method=signal",
        f"{upstream / source}::{NAME}",
        f"--junitxml={result}",
    ]
    completed = subprocess.run(
        command, cwd=ROOT, env=environment, check=False, timeout=75
    )
    if not result.is_file() or result.is_symlink():
        _fail("strict connect-refused capture produced no regular JUnit")
    if sha256_file(POLICY) != POLICY_SHA256:
        _fail("strict connect-refused policy changed during execution")
    _upstream(policy, upstream)
    _sources(ROOT, policy["supportSources"], "support")
    _sources(ROOT, policy["captureSourceSha256"], "capture")
    if _artifact(args.mode, python, site, wheel) != artifact:
        _fail("strict connect-refused installed wheel changed during execution")
    _runtime(policy, {**artifact, "mode": args.mode})
    payload = {
        "schema": 1,
        "kind": KIND,
        "evidenceScope": SCOPE,
        "mode": args.mode,
        "testId": test_id,
        "manifest": str(POLICY),
        "manifestSha256": POLICY_SHA256,
        "upstream": str(upstream),
        "commit": COMMIT,
        "toolingRoot": str(ROOT),
        "toolingSources": policy["captureSourceSha256"],
        **artifact,
        "publicConnectPresent": True,
        "fixtureBind": bind,
        "fixtureHost": host,
        "fixturePort": port,
        "fixtureAdaptation": policy["fixtureAdaptation"],
        "environment": environment,
        "sandboxProfile": profile,
        "sandboxProfileSha256": hashlib.sha256(profile.encode()).hexdigest(),
        "sandboxAllowedExecutables": allowed,
        "sandboxProbe": probe,
        "command": command,
        "runNonce": nonce,
        "exitCode": completed.returncode,
        "outcome": "pending",
        "result": str(result),
        "resultSha256": sha256_file(result),
        "callReceipt": str(receipt),
        "callReceiptSha256": sha256_file(receipt),
        "verifierSha256": sha256_file(
            ROOT / "scripts/verify_playwright_python_connect_refused_strict_v1.py"
        ),
    }
    payload["outcome"] = report_outcome(payload)
    has_required_prefix = _receipt_events(payload)
    if has_required_prefix != (payload["outcome"] == "passed"):
        _fail(
            "strict connect-refused error prefix differs from official assertion outcome"
        )
    with provenance.open("x", encoding="utf-8") as stream:
        json.dump(payload, stream, indent=2, sort_keys=True)
        stream.write("\n")
    verify_provenance(provenance)
    print(
        f"verified strict connect-refused v1 {args.mode}/{test_id}: {payload['outcome']}"
    )
    return provenance


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("candidate", "oracle"))
    parser.add_argument("case", type=int, choices=tuple(range(len(TEST_IDS))))
    parser.add_argument("upstream", type=Path)
    parser.add_argument("--python", type=Path, required=True)
    parser.add_argument("--wheel", type=Path, required=True)
    parser.add_argument("--result", type=Path, required=True)
    args = parser.parse_args()
    try:
        run(args)
    except (
        OSError,
        ValueError,
        subprocess.TimeoutExpired,
        subprocess.CalledProcessError,
    ) as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
