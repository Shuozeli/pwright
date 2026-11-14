"""AAA checks for fail-closed strict dummy-node evidence verification."""

from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path

import pytest
from playwright_python_official_provenance import file_sha256, tree_sha256
from summarize_playwright_python_official_results import _verified_generic_result
from verify_playwright_python_dummy_node_strict import (
    ASSET,
    COMMIT,
    KIND,
    POLICY,
    ROOT,
    SCOPE,
    TEST_NAME,
    StrictEvidenceError,
    verify_evidence,
    verify_provenance,
)


def _write(path: Path, content: str | bytes) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(content.encode() if isinstance(content, str) else content)
    return path


def _jsonl(path: Path, rows: list[dict[str, object]]) -> str:
    _write(path, "".join(json.dumps(row, sort_keys=True) + "\n" for row in rows))
    return file_sha256(path)


def _sample(tmp_path: Path, mode: str, api: str) -> tuple[dict, dict, Path, Path, Path]:
    # Arrange: a tiny, self-contained model of the pinned artifact topology.
    policy = copy.deepcopy(json.loads(POLICY.read_text()))
    policy["captureReady"] = True
    upstream = tmp_path / "upstream"
    for relative in policy["upstreamSources"]:
        source = _write(
            upstream / relative,
            (
                b"#!/usr/bin/env node\n\nprocess.exit(1);\n"
                if relative == ASSET
                else f"pinned {relative}\n"
            ),
        )
        policy["upstreamSources"][relative] = file_sha256(source)
    (upstream / ASSET).chmod(0o755)

    tooling = tmp_path / "tooling"
    for relative in policy["captureSourcePaths"]:
        source = _write(tooling / relative, f"strict source {relative}\n")
        policy["captureSourceSha256"][relative] = file_sha256(source)
    for relative in policy["supportTrees"]:
        _write(tooling / relative / "plugin.py", f"support {relative}\n")
        policy["supportTrees"][relative] = tree_sha256(tooling / relative)

    python = tmp_path / mode / "bin/python"
    real_python = _write(tmp_path / "cpython3.12", b"python executable\n")
    python.parent.mkdir(parents=True)
    python.symlink_to(real_python)
    policy["pythonRuntimeSha256"] = file_sha256(real_python)
    site = python.parent.parent / "lib/python3.12/site-packages"
    official = _write(site / "playwright/__init__.py", "official playwright\n").parent
    driver = _write(official / "driver/node", b"official driver node\n")
    policy["oraclePackageTreeSha256"] = tree_sha256(official)
    policy["oracleDriverNodeSha256"] = file_sha256(driver)
    wheel = _write(tmp_path / "abi119.whl", b"abi119 wheel\n")
    policy["candidateWheelSha256"] = file_sha256(wheel)
    node = _write(tmp_path / "node", b"system node\n")
    policy["systemNodeSha256"] = file_sha256(node)

    artifact_site = (
        site if mode == "candidate" else tmp_path / "candidate-artifact-site"
    )
    package = artifact_site / "pwright"
    native = _write(package / "_native.abi3.so", b"native abi119\n")
    _write(package / "__init__.py", "candidate public package\n")
    policy["candidatePackageTreeSha256"] = tree_sha256(package)
    policy["candidateNativeSha256"] = file_sha256(native)
    dylib = _write(
        artifact_site / "shuozeli_pwright.dylibs/libssl-test.dylib", b"ssl\n"
    )
    policy["candidateRuntimeLibraries"] = {dylib.name: file_sha256(dylib)}

    manifest = _write(
        tmp_path / "strict-policy.json", json.dumps(policy, sort_keys=True)
    )
    result = _write(
        tmp_path / "one-official-case.xml",
        '<testsuites><testsuite tests="1" failures="0" errors="0" skipped="0">'
        f'<testcase classname="tests.{api}.test_launcher" name="{TEST_NAME}[chromium]" />'
        "</testsuite></testsuites>",
    )
    sidecar = result.with_suffix(".provenance.json")
    public = result.with_suffix(".public-launch.jsonl")
    process = result.with_suffix(".dummy-process.jsonl")
    guard = tooling / "packages/pwright-python/tests/official/dummy_guard_strict"
    nonce = "0123456789abcdef0123456789abcdef"
    common = {
        "api": api,
        "mode": mode,
        "runNonce": nonce,
        "executablePath": str(upstream / ASSET),
        "assetSha256": policy["upstreamSources"][ASSET],
    }
    process_hash = _jsonl(
        process,
        [
            {
                "event": "dummy-node-exited",
                "exitCode": 1,
                "childPid": 4242,
                "node": str(node),
                "nodeSha256": file_sha256(node),
                "argv": [str(node), str(upstream / ASSET)],
                **common,
            }
        ],
    )
    public_hash = _jsonl(
        public,
        [
            {"event": "public-launch-invoked", **common},
            {
                "event": "public-launch-failed",
                "errorClass": "Error" if mode == "candidate" else "TargetClosedError",
                "errorMessage": "Browser process exited with code 1",
                "processReceiptSha256": process_hash,
                **common,
            },
        ],
    )

    allowed = [
        str(python),
        str(real_python),
        str(upstream / ASSET),
        str(guard / "node"),
        "/usr/bin/env",
        "/usr/bin/python3",
        "/Library/Developer/CommandLineTools/usr/bin/python3",
        "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/bin/python3.9",
        "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/Resources/Python.app/Contents/MacOS/Python",
        str(node),
    ]
    if mode == "oracle":
        allowed.append(str(driver))
    profile = "\n".join(
        [
            "(version 1)",
            "(allow default)",
            "(deny process-exec)",
            "(deny network-outbound)",
        ]
        + [f"(allow process-exec (literal {json.dumps(path)}))" for path in allowed]
    )
    official_tooling = tooling / "packages/pwright-python/tests/official"
    python_paths = (
        [official_tooling / "shim", upstream, official_tooling / "runtime", guard]
        if mode == "candidate"
        else [official_tooling / "runtime", guard]
    )
    environment = {
        "PATH": f"{guard}:/opt/homebrew/bin:/usr/bin:/bin",
        "PYTHONPATH": ":".join(map(str, python_paths)),
        "PYTHONNOUSERSITE": "1",
        "PYTEST_DISABLE_PLUGIN_AUTOLOAD": "1",
        "PWRIGHT_OFFICIAL_MODE": mode,
        "PWRIGHT_OFFICIAL_LAUNCH_MODE": "owned",
        "PWRIGHT_OFFICIAL_BROWSER_CHANNEL": "",
        "PWRIGHT_OFFICIAL_UPSTREAM_TESTS": str(upstream / "tests"),
        "PWRIGHT_OFFICIAL_TAILSCALE_IP": "100.64.0.1",
        "PWRIGHT_OFFICIAL_TAILSCALE_HOST": "test.tail8f3b66.ts.net",
        "PWRIGHT_PYTHON_DUMMY_UPSTREAM": str(upstream),
        "PWRIGHT_PYTHON_DUMMY_API": api,
        "PWRIGHT_PYTHON_DUMMY_LAUNCH_RECEIPT": str(public),
        "PWRIGHT_PYTHON_DUMMY_PROCESS_RECEIPT": str(process),
        "PWRIGHT_PYTHON_DUMMY_REAL_NODE": str(node),
        "PWRIGHT_PYTHON_DUMMY_NONCE": nonce,
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
        if mode == "candidate"
        else "pwright_official_oracle_runtime",
        "-p",
        "pwright_official_dummy_guard",
        "--browser=chromium",
        "--timeout=20",
        "--timeout-method=signal",
        str(upstream / f"tests/{api}/test_launcher.py") + f"::{TEST_NAME}",
        f"--junitxml={result}",
    ]
    payload = {
        "schema": 2,
        "kind": KIND,
        "evidenceScope": SCOPE,
        "mode": mode,
        "api": api,
        "testId": policy["tests"][api],
        "manifest": str(manifest),
        "manifestSha256": file_sha256(manifest),
        "upstream": str(upstream),
        "commit": COMMIT,
        "toolingRoot": str(tooling),
        "toolingSources": policy["captureSourceSha256"],
        "pythonExecutable": str(python),
        "pythonVersion": "3.12.14",
        "wheelPath": str(wheel),
        "wheelSha256": file_sha256(wheel),
        "playwrightPackage": str(official),
        "playwrightPackageSha256": policy["oraclePackageTreeSha256"],
        "pwrightPackage": str(package) if mode == "candidate" else None,
        "pwrightPackageSha256": (
            policy["candidatePackageTreeSha256"] if mode == "candidate" else None
        ),
        "nativePath": str(native) if mode == "candidate" else None,
        "nativeSha256": policy["candidateNativeSha256"]
        if mode == "candidate"
        else None,
        "runtimeLibraries": policy["candidateRuntimeLibraries"]
        if mode == "candidate"
        else {},
        "driverNode": str(driver),
        "driverNodeSha256": file_sha256(driver),
        "systemNode": str(node),
        "systemNodeSha256": file_sha256(node),
        "launchMode": "owned",
        "endpoint": None,
        "collectOnly": False,
        "bootstrapOnly": False,
        "fixtureBind": "100.64.0.1",
        "fixtureHost": "test.tail8f3b66.ts.net",
        "environment": environment,
        "sandboxProfile": profile,
        "sandboxProfileSha256": hashlib.sha256(profile.encode()).hexdigest(),
        "sandboxAllowedExecutables": allowed,
        "sandboxProbe": {"execDenied": True, "networkDenied": True},
        "command": command,
        "runNonce": nonce,
        "exitCode": 0,
        "result": str(result),
        "resultSha256": file_sha256(result),
        "publicReceipt": str(public),
        "publicReceiptSha256": public_hash,
        "processReceipt": str(process),
        "processReceiptSha256": process_hash,
        "verifierSha256": file_sha256(
            ROOT / "scripts/verify_playwright_python_dummy_node_strict.py"
        ),
    }
    return payload, policy, manifest, sidecar, tooling


@pytest.mark.parametrize(
    "mode,api",
    [
        ("candidate", "async"),
        ("candidate", "sync"),
        ("oracle", "async"),
        ("oracle", "sync"),
    ],
)
def test_exact_guarded_result_is_eligible_only_under_ready_test_policy(
    tmp_path: Path, mode: str, api: str
) -> None:
    # Arrange
    payload, policy, manifest, sidecar, tooling = _sample(tmp_path, mode, api)

    # Act
    result = verify_evidence(
        payload,
        policy=policy,
        policy_path=manifest,
        sidecar_path=sidecar,
        tooling_root=tooling,
        check_checkout=False,
    )

    # Assert
    assert result is payload


def test_production_policy_rejects_an_unstructured_new_sidecar(
    tmp_path: Path,
) -> None:
    # Arrange
    sidecar = _write(tmp_path / "result.provenance.json", "{}")

    # Act
    with pytest.raises(StrictEvidenceError, match="fields differ"):
        verify_provenance(sidecar)

    # Assert
    assert json.loads(POLICY.read_text())["captureReady"] is True


def test_generic_ledger_verifier_refuses_guarded_dummy_evidence(tmp_path: Path) -> None:
    # Arrange
    payload, _policy, _manifest, sidecar, _tooling = _sample(
        tmp_path, "candidate", "async"
    )
    _write(sidecar, json.dumps(payload, sort_keys=True))

    # Act
    with pytest.raises(SystemExit, match="provenance verification failed"):
        _verified_generic_result(Path(payload["result"]), "candidate")


def test_generic_ledger_verifier_refuses_historical_diagnostic(
    tmp_path: Path,
) -> None:
    # Arrange: the older diagnostic scope and schema are never ordinary
    # source-bound execution provenance, even beside a report file.
    report = _write(tmp_path / "dummy.xml", "<testsuites />\n")
    sidecar = report.with_suffix(".provenance.json")
    _write(
        sidecar,
        json.dumps(
            {
                "schema": 1,
                "evidenceScope": "diagnostic-only-no-general-conformance-credit",
                "mode": "candidate",
                "result": str(report),
                "resultSha256": file_sha256(report),
            }
        ),
    )

    # Act
    with pytest.raises(SystemExit, match="provenance verification failed"):
        _verified_generic_result(report, "candidate")

    # Assert
    assert report.is_file() and sidecar.is_file()


def _move_process_receipt_to_public_path(payload: dict) -> None:
    payload["processReceipt"] = payload["publicReceipt"]
    payload["environment"]["PWRIGHT_PYTHON_DUMMY_PROCESS_RECEIPT"] = payload[
        "publicReceipt"
    ]


@pytest.mark.parametrize(
    "mutate,reason",
    [
        (lambda p: p.update(schema=1), "scope"),
        (
            lambda p: p.update(
                evidenceScope="diagnostic-only-no-general-conformance-credit"
            ),
            "scope",
        ),
        (
            lambda p: p.update(testId="tests/async/test_launcher.py::other[chromium]"),
            "scope",
        ),
        (lambda p: p.update(collectOnly=True), "scope"),
        (lambda p: p.update(endpoint="http://127.0.0.1:9222"), "scope"),
        (lambda p: p.update(nativeSha256="0" * 64), "native"),
        (
            lambda p: p["environment"].update(
                PWRIGHT_CDP_ENDPOINT="http://127.0.0.1:9222"
            ),
            "environment",
        ),
        (lambda p: p["command"].append("-k=other"), "command"),
        (lambda p: p["sandboxProbe"].update(networkDenied=False), "sandbox"),
        (lambda p: p.update(verifierSha256="0" * 64), "verifier"),
        (_move_process_receipt_to_public_path, "JUnit siblings"),
    ],
)
def test_forged_provenance_is_rejected(tmp_path: Path, mutate, reason: str) -> None:
    # Arrange
    payload, policy, manifest, sidecar, tooling = _sample(
        tmp_path, "candidate", "async"
    )
    mutate(payload)

    # Act
    with pytest.raises(StrictEvidenceError, match=reason):
        verify_evidence(
            payload,
            policy=policy,
            policy_path=manifest,
            sidecar_path=sidecar,
            tooling_root=tooling,
            check_checkout=False,
        )

    # Assert
    assert payload["mode"] == "candidate"


@pytest.mark.parametrize(
    "change,reason",
    [
        ("missing-process", "process receipt"),
        ("wrong-exit", "exit-code-1"),
        ("wrong-nonce", "process receipt"),
        ("wrong-public-sequence", "public launch"),
        ("skipped-junit", "official JUnit"),
    ],
)
def test_false_positive_artifact_is_rejected_after_rehash(
    tmp_path: Path, change: str, reason: str
) -> None:
    # Arrange
    payload, policy, manifest, sidecar, tooling = _sample(tmp_path, "candidate", "sync")
    if change == "skipped-junit":
        result = Path(payload["result"])
        _write(
            result,
            result.read_text()
            .replace("</testcase>", "<skipped /></testcase>")
            .replace(" />", "><skipped /></testcase>"),
        )
        payload["resultSha256"] = file_sha256(result)
    else:
        process = Path(payload["processReceipt"])
        public = Path(payload["publicReceipt"])
        if change == "wrong-public-sequence":
            rows = [json.loads(line) for line in public.read_text().splitlines()]
            payload["publicReceiptSha256"] = _jsonl(public, list(reversed(rows)))
        else:
            rows = [json.loads(line) for line in process.read_text().splitlines()]
            if change == "missing-process":
                rows = []
            elif change == "wrong-exit":
                rows[0]["exitCode"] = 0
            else:
                rows[0]["runNonce"] = "f" * 32
            payload["processReceiptSha256"] = _jsonl(process, rows)
            public_rows = [json.loads(line) for line in public.read_text().splitlines()]
            public_rows[1]["processReceiptSha256"] = payload["processReceiptSha256"]
            payload["publicReceiptSha256"] = _jsonl(public, public_rows)

    # Act
    with pytest.raises(StrictEvidenceError, match=reason):
        verify_evidence(
            payload,
            policy=policy,
            policy_path=manifest,
            sidecar_path=sidecar,
            tooling_root=tooling,
            check_checkout=False,
        )

    # Assert
    assert payload["resultSha256"] == file_sha256(Path(payload["result"]))


def test_changed_support_plugin_is_rejected(tmp_path: Path) -> None:
    # Arrange
    payload, policy, manifest, sidecar, tooling = _sample(
        tmp_path, "candidate", "async"
    )
    _write(
        tooling / "packages/pwright-python/tests/official/shim/plugin.py",
        "a changed shim can fake a public launch\n",
    )

    # Act
    with pytest.raises(StrictEvidenceError, match="support tree"):
        verify_evidence(
            payload,
            policy=policy,
            policy_path=manifest,
            sidecar_path=sidecar,
            tooling_root=tooling,
            check_checkout=False,
        )

    # Assert
    assert policy["supportTrees"][
        "packages/pwright-python/tests/official/shim"
    ] != tree_sha256(tooling / "packages/pwright-python/tests/official/shim")
