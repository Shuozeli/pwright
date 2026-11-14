"""Capture one exact official Python upstream skip in isolated paired lanes.

Every selected test is upstream-marked to skip on the selected engine. The
guard aborts before test-body execution if that expectation ever changes; the
process sandbox also denies browser execution and outbound connections. This
lane records skips only and never awards public-behavior compatibility credit.
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
    _selected_python,
    _tailscale_fixture,
)
from verify_playwright_python_upstream_skip_callable_fixtures_strict_v2 import (
    COMMIT,
    GUARD,
    KIND,
    POLICY,
    ROOT,
    RUNNER,
    SCOPE,
    TEST_IDS,
    StrictUpstreamSkipEvidenceError,
    _junit,
    _receipt,
    command,
    environment,
    identity_anchor,
    identity_probe,
    load_policy,
    sandbox_profile,
    sha256_file,
    verify_provenance,
    verify_upstream,
)


def _fail(message: str) -> NoReturn:
    raise StrictUpstreamSkipEvidenceError(message)


def _fresh_result(path: Path) -> tuple[Path, Path, Path]:
    result = path.absolute()
    if result.suffix != ".xml" or result.is_symlink():
        _fail("strict skip result must be a fresh absolute XML destination")
    provenance = result.with_suffix(".provenance.json")
    receipt = result.with_suffix(".call.jsonl")
    for output in (result, provenance, receipt):
        if output.is_symlink() or output.exists():
            _fail(f"strict skip output already exists: {output}")
    result.parent.mkdir(parents=True, exist_ok=True)
    result.open("xb").close()
    receipt.open("xb").close()
    return result, provenance, receipt


def run(mode: str, case: int, result_path: Path) -> Path:
    if sys.platform != "darwin" or not Path("/usr/bin/sandbox-exec").is_file():
        _fail("strict skip capture requires the macOS process/network sandbox")
    _clean_ambient()
    policy = load_policy()
    policy_sha = sha256_file(POLICY)
    if case < 0 or case >= len(TEST_IDS):
        _fail("strict skip case index is outside the 11 pinned upstream nodes")
    test_id = TEST_IDS[case]
    anchor = identity_anchor(mode, policy)
    upstream = Path(anchor["upstream"])
    verify_upstream(upstream, policy)
    python, _, site = _selected_python(
        Path(anchor["pythonExecutable"]), anchor["pythonRuntimeSha256"]
    )
    artifact = _artifact(mode, python, site, Path(anchor["wheelPath"]))
    if any(artifact[key] != anchor[key] for key in artifact):
        _fail("strict skip runtime package differs from the validated anchor")
    bind, host = _tailscale_fixture()
    profile, allowed = sandbox_profile(artifact, mode)
    result, provenance, receipt = _fresh_result(result_path)
    nonce = secrets.token_hex(16)
    env = environment(mode, upstream, bind, host, test_id, receipt, nonce)
    before = identity_probe(mode, artifact, env, profile)
    selection = command(mode, artifact, profile, upstream, test_id, result)
    completed = subprocess.run(
        selection,
        cwd=ROOT,
        env=env,
        check=False,
        capture_output=True,
        text=True,
        timeout=90,
    )
    if not result.is_file() or not receipt.is_file():
        _fail("strict upstream skip did not produce JUnit and guard receipt")
    if sha256_file(POLICY) != policy_sha:
        _fail("strict upstream skip policy changed during capture")
    verify_upstream(upstream, policy)
    after_artifact = _artifact(mode, python, site, Path(anchor["wheelPath"]))
    if any(after_artifact[key] != anchor[key] for key in artifact):
        _fail("strict upstream skip runtime package changed during capture")
    after = identity_probe(mode, artifact, env, profile)
    payload = {
        "schema": 1,
        "kind": KIND,
        "evidenceScope": SCOPE,
        "mode": mode,
        "testId": test_id,
        "manifest": str(POLICY),
        "manifestSha256": policy_sha,
        "upstream": str(upstream),
        "commit": COMMIT,
        "identityAnchor": str(ROOT / policy["identityAnchors"][mode]["path"]),
        "identityAnchorSha256": policy["identityAnchors"][mode]["sha256"],
        "artifact": artifact,
        "fixtureBind": bind,
        "fixtureHost": host,
        "environment": env,
        "identityProbeBefore": before,
        "identityProbeAfter": after,
        "sandboxProfile": profile,
        "sandboxProfileSha256": hashlib.sha256(profile.encode()).hexdigest(),
        "sandboxAllowedExecutables": allowed,
        "command": selection,
        "runNonce": nonce,
        "exitCode": completed.returncode,
        "outcome": "skipped",
        "result": str(result),
        "resultSha256": sha256_file(result),
        "callReceipt": str(receipt),
        "callReceiptSha256": sha256_file(receipt),
        "guardSha256": sha256_file(
            GUARD / "pwright_official_upstream_skip_guard_v1.py"
        ),
        "runnerSha256": sha256_file(RUNNER),
        "verifierSha256": sha256_file(
            ROOT
            / "scripts/verify_playwright_python_upstream_skip_callable_fixtures_strict_v2.py"
        ),
    }
    # No evidence sidecar is written until the independently parsable JUnit
    # and pre-body guard receipt match the exact marker skip.
    _receipt(payload)
    _junit(payload)
    with provenance.open("x", encoding="utf-8") as stream:
        json.dump(payload, stream, indent=2, sort_keys=True)
        stream.write("\n")
    verify_provenance(provenance)
    print(f"verified upstream skip only {mode}/{case}: {test_id}")
    return provenance


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("candidate", "oracle"))
    parser.add_argument("case", type=int, choices=tuple(range(len(TEST_IDS))))
    parser.add_argument("--result", type=Path, required=True)
    args = parser.parse_args()
    try:
        run(args.mode, args.case, args.result)
    except (
        OSError,
        ValueError,
        subprocess.TimeoutExpired,
        subprocess.CalledProcessError,
    ) as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
