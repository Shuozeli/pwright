"""Verify exact upstream marker skips without awarding public-behavior parity."""

from __future__ import annotations

import argparse
import ast
import hashlib
import ipaddress
import json
import re
import subprocess
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import Any, NoReturn

from verify_playwright_python_connect_refused_strict_v1 import (
    verify_provenance as verify_identity_anchor,
)
from verify_playwright_python_ws_transport_strict import sha256_file

ROOT = Path(__file__).resolve().parents[1]
OFFICIAL = ROOT / "packages/pwright-python/tests/official"
POLICY = OFFICIAL / "upstream-skip-markers-strict-v1-manifest.json"
GUARD = OFFICIAL / "upstream_skip_guard_strict_v1"
RUNNER = ROOT / "scripts/run_playwright_python_upstream_skip_markers_strict_v1.py"
KIND = "playwright-python-upstream-skip-markers-strict-v1"
SCOPE = "upstream-marker-skip-accounting-only-no-public-behavior-credit"
COMMIT = "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
_CDP_ASYNC = "tests/async/test_browsertype_connect_cdp.py"
_CDP_SYNC = "tests/sync/test_browsertype_connect_cdp.py"
_ISSUES = "tests/async/test_issues.py"
_HEADFUL = "tests/async/test_headful.py"
_CDP_ASYNC_NAMES = (
    "test_conect_over_a_ws_endpoint",
    "test_connect_over_cdp_passing_header_works",
    "test_connect_to_an_existing_cdp_session",
    "test_connect_to_an_existing_cdp_session_twice",
    "test_should_print_custom_ws_close_error",
)
TEST_IDS = (
    tuple(
        f"{_CDP_ASYNC}::{name}[{engine}]"
        for name in _CDP_ASYNC_NAMES
        for engine in ("firefox", "webkit")
    )
    + tuple(
        f"{_CDP_SYNC}::test_connect_to_an_existing_cdp_session[{engine}]"
        for engine in ("firefox", "webkit")
    )
    + tuple(f"{_ISSUES}::test_issue_189[{engine}]" for engine in ("firefox", "webkit"))
    + (f"{_HEADFUL}::test_should_not_override_viewport_size_when_passed_null[webkit]",)
)
_SOURCES = frozenset(
    {
        "pyproject.toml",
        "tests/conftest.py",
        "tests/async/conftest.py",
        "tests/sync/conftest.py",
        "tests/server.py",
        _CDP_ASYNC,
        _CDP_SYNC,
        _ISSUES,
        _HEADFUL,
    }
)
_ANCHORS = {
    "candidate": {
        "path": ".work/official-conformance/results/python-connect-refused-strict-v1/candidate-async-chromium.provenance.json",
        "sha256": "d31d2b5981be5bb56d5b8f8db8c10747c1c67584b3cfa05437350e19a120f396",
    },
    "oracle": {
        "path": ".work/official-conformance/results/python-connect-refused-strict-v1/oracle-async-chromium.provenance.json",
        "sha256": "941596b17a1ca433f2c6005a77b464f2df4976a3f665329a5a0bb5cffa291e26",
    },
}
_POLICY_KEYS = frozenset(
    {
        "schema",
        "kind",
        "evidenceScope",
        "repository",
        "tag",
        "commit",
        "requiresBrowser",
        "requiresFixtureServers",
        "fixtureAdaptation",
        "expectedOutcome",
        "tests",
        "upstreamSourceSha256",
        "identityAnchors",
    }
)
_ARTIFACT_KEYS = frozenset(
    {
        "pythonExecutable",
        "pythonRuntimeSha256",
        "pythonVersion",
        "wheelPath",
        "wheelSha256",
        "playwrightPackage",
        "playwrightPackageSha256",
        "driverNode",
        "driverNodeSha256",
        "pwrightPackage",
        "pwrightPackageSha256",
        "nativePath",
        "nativeSha256",
        "runtimeLibraries",
    }
)
_EVIDENCE_KEYS = frozenset(
    {
        "schema",
        "kind",
        "evidenceScope",
        "mode",
        "testId",
        "manifest",
        "manifestSha256",
        "upstream",
        "commit",
        "identityAnchor",
        "identityAnchorSha256",
        "artifact",
        "fixtureBind",
        "fixtureHost",
        "environment",
        "identityProbeBefore",
        "identityProbeAfter",
        "sandboxProfile",
        "sandboxProfileSha256",
        "sandboxAllowedExecutables",
        "command",
        "runNonce",
        "exitCode",
        "outcome",
        "result",
        "resultSha256",
        "callReceipt",
        "callReceiptSha256",
        "guardSha256",
        "runnerSha256",
        "verifierSha256",
    }
)
_DIGEST = re.compile(r"[0-9a-f]{64}\Z")
_NONCE = re.compile(r"[0-9a-f]{32}\Z")
_HOST = re.compile(r"[a-z0-9-]+(?:\.[a-z0-9-]+)*\.ts\.net\Z")


class StrictUpstreamSkipEvidenceError(ValueError):
    """The selected official skip has insufficient source-bound evidence."""


def _fail(message: str) -> NoReturn:
    raise StrictUpstreamSkipEvidenceError(message)


def _regular(path: Path, label: str) -> Path:
    if not path.is_absolute() or path.is_symlink() or not path.is_file():
        _fail(f"{label} is not an absolute regular file")
    return path


def _digest_file(path: Path, digest: str, label: str) -> None:
    _regular(path, label)
    if not isinstance(digest, str) or not _DIGEST.fullmatch(digest):
        _fail(f"{label} digest is invalid")
    if sha256_file(path) != digest:
        _fail(f"{label} digest differs")


def _marker(node: ast.AST, kind: str, value: str) -> bool:
    return (
        isinstance(node, ast.Call)
        and isinstance(node.func, ast.Attribute)
        and isinstance(node.func.value, ast.Attribute)
        and isinstance(node.func.value.value, ast.Name)
        and node.func.value.value.id == "pytest"
        and node.func.value.attr == "mark"
        and node.func.attr == kind
        and len(node.args) == 1
        and isinstance(node.args[0], ast.Constant)
        and node.args[0].value == value
        and not node.keywords
    )


def _source_marker(upstream: Path, test_id: str) -> None:
    source, selection = test_id.split("::", 1)
    name = selection.split("[", 1)[0]
    tree = ast.parse((upstream / source).read_text(encoding="utf-8"))
    function = next(
        (
            node
            for node in tree.body
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
            and node.name == name
        ),
        None,
    )
    if function is None:
        _fail(f"upstream test function is absent: {test_id}")
    args = {arg.arg for arg in function.args.args}
    if args - {"browser_type", "launch_arguments", "server"}:
        _fail(f"skip-only test has a browser-launching or unknown fixture: {test_id}")
    if source in {_CDP_ASYNC, _CDP_SYNC}:
        module_markers = [
            node.value
            for node in tree.body
            if isinstance(node, ast.Assign)
            and any(
                isinstance(target, ast.Name) and target.id == "pytestmark"
                for target in node.targets
            )
        ]
        valid = len(module_markers) == 1 and _marker(
            module_markers[0], "only_browser", "chromium"
        )
    elif source == _ISSUES:
        valid = any(
            _marker(node, "only_browser", "chromium")
            for node in function.decorator_list
        )
    else:
        valid = source == _HEADFUL and any(
            _marker(node, "skip_browser", "webkit") for node in function.decorator_list
        )
    if not valid:
        _fail(f"upstream skip marker differs: {test_id}")


def load_policy() -> dict[str, Any]:
    _regular(POLICY, "skip policy")
    raw = json.loads(POLICY.read_text(encoding="utf-8"))
    if not isinstance(raw, dict) or set(raw) != _POLICY_KEYS:
        _fail("skip policy schema differs")
    if (
        raw["schema"] != 1
        or raw["kind"] != KIND
        or raw["evidenceScope"] != SCOPE
        or raw["repository"] != "https://github.com/microsoft/playwright-python.git"
        or raw["tag"] != "v1.58.0"
        or raw["commit"] != COMMIT
        or raw["requiresBrowser"] is not False
        or raw["requiresFixtureServers"] is not True
        or raw["fixtureAdaptation"]
        != "unchanged-upstream-tests-with-tailscale-only-fixture-runtime"
        or raw["expectedOutcome"] != "skipped"
        or raw["tests"] != list(TEST_IDS)
        or raw["identityAnchors"] != _ANCHORS
    ):
        _fail("skip policy changed its exact source, identity, or outcome scope")
    sources = raw["upstreamSourceSha256"]
    if not isinstance(sources, dict) or set(sources) != _SOURCES:
        _fail("skip policy upstream source set differs")
    if any(
        not isinstance(value, str) or not _DIGEST.fullmatch(value)
        for value in sources.values()
    ):
        _fail("skip policy has an invalid source digest")
    return raw


def identity_anchor(mode: str, policy: dict[str, Any]) -> dict[str, Any]:
    if mode not in _ANCHORS:
        _fail("skip mode differs")
    pin = policy["identityAnchors"][mode]
    path = ROOT / pin["path"]
    _digest_file(path, pin["sha256"], "strict identity anchor")
    payload = verify_identity_anchor(path)
    if payload["mode"] != mode or payload["commit"] != COMMIT:
        _fail("strict identity anchor mode or upstream commit differs")
    return payload


def verify_upstream(upstream: Path, policy: dict[str, Any]) -> None:
    if not upstream.is_absolute() or upstream.is_symlink() or not upstream.is_dir():
        _fail("upstream checkout must be an absolute non-symlink directory")
    head = subprocess.run(
        ["git", "-C", str(upstream), "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
        timeout=10,
    ).stdout.strip()
    dirty = subprocess.run(
        ["git", "-C", str(upstream), "status", "--porcelain"],
        check=True,
        capture_output=True,
        text=True,
        timeout=10,
    ).stdout.strip()
    if head != COMMIT or dirty:
        _fail("upstream checkout commit or clean status differs")
    for source, digest in policy["upstreamSourceSha256"].items():
        _digest_file(upstream / source, digest, f"upstream {source}")
    for test_id in TEST_IDS:
        _source_marker(upstream, test_id)


def sandbox_profile(artifact: dict[str, Any], mode: str) -> tuple[str, list[str]]:
    python = Path(artifact["pythonExecutable"])
    allowed = [str(python), str(python.resolve())]
    if mode == "oracle":
        allowed.append(artifact["driverNode"])
    profile = "\n".join(
        [
            "(version 1)",
            "(allow default)",
            "(deny process-exec)",
            "(deny network-outbound)",
        ]
        + [f"(allow process-exec (literal {json.dumps(path)}))" for path in allowed]
    )
    return profile, allowed


def environment(
    mode: str,
    upstream: Path,
    bind: str,
    host: str,
    test_id: str,
    receipt: Path,
    nonce: str,
) -> dict[str, str]:
    paths = (
        [OFFICIAL / "shim", upstream, OFFICIAL / "runtime", GUARD]
        if mode == "candidate"
        else [OFFICIAL / "runtime", GUARD]
    )
    return {
        "PATH": "/opt/homebrew/bin:/usr/bin:/bin",
        "PYTHONPATH": ":".join(map(str, paths)),
        "PYTHONNOUSERSITE": "1",
        "PYTEST_DISABLE_PLUGIN_AUTOLOAD": "1",
        "PWRIGHT_OFFICIAL_MODE": mode,
        "PWRIGHT_OFFICIAL_LAUNCH_MODE": "owned",
        "PWRIGHT_OFFICIAL_BROWSER_CHANNEL": "",
        "PWRIGHT_OFFICIAL_UPSTREAM_TESTS": str(upstream / "tests"),
        "PWRIGHT_OFFICIAL_TAILSCALE_IP": bind,
        "PWRIGHT_OFFICIAL_TAILSCALE_HOST": host,
        "PWRIGHT_PYTHON_SKIP_V1_EXPECTED_TEST": test_id,
        "PWRIGHT_PYTHON_SKIP_V1_UPSTREAM": str(upstream),
        "PWRIGHT_PYTHON_SKIP_V1_RECEIPT": str(receipt),
        "PWRIGHT_PYTHON_SKIP_V1_NONCE": nonce,
    }


def identity_probe(
    mode: str,
    artifact: dict[str, Any],
    env: dict[str, str],
    profile: str,
) -> dict[str, str | None]:
    script = (
        "import json,pathlib,playwright; "
        "root=str(pathlib.Path(playwright.__file__).resolve()); "
        "p=None; "
        "exec('import pwright; p=str(pathlib.Path(pwright.__file__).resolve())') "
        "if " + repr(mode == "candidate") + " else None; "
        "print(json.dumps({'playwright':root,'pwright':p}))"
    )
    completed = subprocess.run(
        [
            "/usr/bin/sandbox-exec",
            "-p",
            profile,
            artifact["pythonExecutable"],
            "-c",
            script,
        ],
        env=env,
        check=True,
        capture_output=True,
        text=True,
        timeout=20,
    )
    result = json.loads(completed.stdout.strip())
    expected = {
        "playwright": str(
            (OFFICIAL / "shim/playwright/__init__.py").resolve()
            if mode == "candidate"
            else (Path(artifact["playwrightPackage"]) / "__init__.py").resolve()
        ),
        "pwright": (
            str((Path(artifact["pwrightPackage"]) / "__init__.py").resolve())
            if mode == "candidate"
            else None
        ),
    }
    if result != expected:
        _fail("runtime did not import the pinned candidate shim or oracle package")
    return result


def command(
    mode: str,
    artifact: dict[str, Any],
    profile: str,
    upstream: Path,
    test_id: str,
    result: Path,
) -> list[str]:
    source, selection = test_id.split("::", 1)
    name, engine = selection.rsplit("[", 1)
    engine = engine.removesuffix("]")
    return [
        "/usr/bin/sandbox-exec",
        "-p",
        profile,
        artifact["pythonExecutable"],
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
        if mode == "candidate"
        else "pwright_official_oracle_runtime",
        "-p",
        "pwright_official_upstream_skip_guard_v1",
        f"--browser={engine}",
        "--timeout=20",
        "--timeout-method=signal",
        f"{upstream / source}::{name}",
        f"--junitxml={result}",
    ]


def _receipt(payload: dict[str, Any]) -> None:
    path = Path(payload["callReceipt"])
    _digest_file(path, payload["callReceiptSha256"], "skip guard receipt")
    data = path.read_bytes()
    if not data.endswith(b"\n"):
        _fail("skip guard receipt is not newline terminated")
    lines = data.splitlines()
    if len(lines) != 2:
        _fail("skip guard did not record exactly collection and setup")
    events = [json.loads(line) for line in lines]
    common = {
        "schema": 1,
        "mode": payload["mode"],
        "testId": payload["testId"],
        "runNonce": payload["runNonce"],
    }
    if events != [
        {**common, "event": "collection", "selected": [payload["testId"]]},
        {
            **common,
            "event": "setup",
            "selected": payload["testId"],
            "outcome": "skipped",
        },
    ]:
        _fail("skip guard receipt does not prove pre-body upstream skip")


def _junit(payload: dict[str, Any]) -> None:
    result = Path(payload["result"])
    _digest_file(result, payload["resultSha256"], "upstream skip JUnit")
    root = ET.fromstring(result.read_bytes())
    suites = root.findall("testsuite") if root.tag == "testsuites" else []
    if len(suites) != 1 or len(list(root.iter("testcase"))) != 1:
        _fail("upstream skip JUnit must contain exactly one case")
    suite = suites[0]
    cases = suite.findall("testcase")
    source, selection = payload["testId"].split("::", 1)
    engine = selection.rsplit("[", 1)[1].removesuffix("]")
    skipped = cases[0].find("skipped") if len(cases) == 1 else None
    if (
        suite.get("tests") != "1"
        or suite.get("skipped") != "1"
        or suite.get("failures") != "0"
        or suite.get("errors") != "0"
        or cases[0].get("classname") != source.removesuffix(".py").replace("/", ".")
        or cases[0].get("name") != selection
        or skipped is None
        or skipped.get("type") != "pytest.skip"
        or skipped.get("message") != f"skipped for this browser: {engine}"
        or cases[0].find("failure") is not None
        or cases[0].find("error") is not None
        or payload["exitCode"] != 0
        or payload["outcome"] != "skipped"
    ):
        _fail("upstream skip JUnit identity, marker reason, or exit differs")


def verify_provenance(path: Path) -> dict[str, Any]:
    policy = load_policy()
    payload = json.loads(_regular(path, "skip provenance").read_text(encoding="utf-8"))
    if not isinstance(payload, dict) or set(payload) != _EVIDENCE_KEYS:
        _fail("skip provenance schema differs")
    if (
        payload["schema"] != 1
        or payload["kind"] != KIND
        or payload["evidenceScope"] != SCOPE
        or payload["mode"] not in _ANCHORS
        or payload["testId"] not in TEST_IDS
        or payload["outcome"] != "skipped"
        or payload["commit"] != COMMIT
        or payload["manifest"] != str(POLICY)
        or payload["manifestSha256"] != sha256_file(POLICY)
    ):
        _fail("skip provenance identity differs")
    anchor_pin = policy["identityAnchors"][payload["mode"]]
    anchor_path = ROOT / anchor_pin["path"]
    if (
        payload["identityAnchor"] != str(anchor_path)
        or payload["identityAnchorSha256"] != anchor_pin["sha256"]
    ):
        _fail("skip provenance identity anchor differs")
    anchor = identity_anchor(payload["mode"], policy)
    artifact = payload["artifact"]
    if not isinstance(artifact, dict) or set(artifact) != _ARTIFACT_KEYS:
        _fail("skip provenance artifact schema differs")
    if any(artifact[key] != anchor[key] for key in _ARTIFACT_KEYS):
        _fail("skip runtime package differs from the validated identity anchor")
    upstream = Path(payload["upstream"])
    if upstream != Path(anchor["upstream"]):
        _fail("skip upstream differs from anchor")
    verify_upstream(upstream, policy)
    try:
        bind = ipaddress.ip_address(payload["fixtureBind"])
    except ValueError as error:
        raise StrictUpstreamSkipEvidenceError("skip fixture bind is invalid") from error
    if (
        bind not in ipaddress.ip_network("100.64.0.0/10")
        or not isinstance(payload["fixtureHost"], str)
        or not _HOST.fullmatch(payload["fixtureHost"])
        or not isinstance(payload["runNonce"], str)
        or not _NONCE.fullmatch(payload["runNonce"])
    ):
        _fail("skip fixture address or nonce differs")
    result = Path(payload["result"])
    receipt = Path(payload["callReceipt"])
    if (
        not result.is_absolute()
        or result.suffix != ".xml"
        or path != result.with_suffix(".provenance.json")
        or receipt != result.with_suffix(".call.jsonl")
    ):
        _fail("skip output path relationship differs")
    expected_env = environment(
        payload["mode"],
        upstream,
        payload["fixtureBind"],
        payload["fixtureHost"],
        payload["testId"],
        receipt,
        payload["runNonce"],
    )
    if payload["environment"] != expected_env:
        _fail("skip runtime environment contains an override")
    profile, allowed = sandbox_profile(artifact, payload["mode"])
    if (
        payload["sandboxProfile"] != profile
        or payload["sandboxProfileSha256"]
        != hashlib.sha256(profile.encode()).hexdigest()
        or payload["sandboxAllowedExecutables"] != allowed
        or payload["command"]
        != command(
            payload["mode"], artifact, profile, upstream, payload["testId"], result
        )
    ):
        _fail("skip sandbox or exact command differs")
    before = identity_probe(payload["mode"], artifact, expected_env, profile)
    if (
        payload["identityProbeBefore"] != before
        or payload["identityProbeAfter"] != before
    ):
        _fail("skip runtime import identity differs")
    _digest_file(RUNNER, payload["runnerSha256"], "skip capture runner")
    _digest_file(
        GUARD / "pwright_official_upstream_skip_guard_v1.py",
        payload["guardSha256"],
        "skip pre-body guard",
    )
    _digest_file(Path(__file__), payload["verifierSha256"], "skip verifier")
    _receipt(payload)
    _junit(payload)
    return payload


def verify_pair(candidate: Path, oracle: Path) -> tuple[dict[str, Any], dict[str, Any]]:
    candidate_payload = verify_provenance(candidate)
    oracle_payload = verify_provenance(oracle)
    if (
        candidate_payload["mode"] != "candidate"
        or oracle_payload["mode"] != "oracle"
        or candidate_payload["testId"] != oracle_payload["testId"]
        or candidate_payload["manifestSha256"] != oracle_payload["manifestSha256"]
        or candidate_payload["fixtureBind"] != oracle_payload["fixtureBind"]
        or candidate_payload["fixtureHost"] != oracle_payload["fixtureHost"]
    ):
        _fail("candidate and oracle are not an exact paired upstream marker skip")
    return candidate_payload, oracle_payload


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("oracle", type=Path)
    args = parser.parse_args()
    try:
        candidate, _oracle = verify_pair(args.candidate, args.oracle)
    except (
        OSError,
        ValueError,
        subprocess.TimeoutExpired,
        subprocess.CalledProcessError,
        ET.ParseError,
    ) as error:
        parser.error(str(error))
    print(
        f"verified strict upstream skip only: {candidate['testId']} candidate=skipped oracle=skipped"
    )


if __name__ == "__main__":
    main()
