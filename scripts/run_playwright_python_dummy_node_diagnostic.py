#!/usr/bin/env python3
"""Run one pinned Python launch-failure test in an exec/network-denying sandbox.

This is diagnostic evidence, not a general official-corpus result. The test's
own Error assertion is too broad to prove that the dummy executable ran, so
both the public-call and PATH-interposed dummy-process receipts are required.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import cast

from official_result_destination import prepare_result_destination
from playwright_python_official_provenance import file_sha256, tree_sha256
from run_playwright_official_conformance import _tailscale_runtime

ROOT = Path(__file__).resolve().parents[1]
OFFICIAL = ROOT / "packages/pwright-python/tests/official"
MANIFEST = OFFICIAL / "dummy-node-launch-diagnostic-manifest.json"
MANIFEST_SHA256 = "7f191e7c75bf0decd1a19f065f54a96bff5d5dd57bd43db6513fe242bae9c50b"
GUARD_DIR = OFFICIAL / "dummy_guard"
GUARD_PLUGIN = GUARD_DIR / "pwright_official_dummy_guard.py"
NODE_WRAPPER = GUARD_DIR / "node"
ASSET = "tests/assets/dummy_bad_browser_executable.js"
COMMIT = "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
POLICY = "macos-sandbox-dummy-node-diagnostic-v1"
PYTHON_SOURCES = (
    "scripts/run_playwright_python_dummy_node_diagnostic.py",
    "scripts/official_result_destination.py",
    "scripts/playwright_python_official_provenance.py",
    "scripts/run_playwright_official_conformance.py",
    "packages/pwright-python/tests/official/dummy_guard/pwright_official_dummy_guard.py",
    "packages/pwright-python/tests/official/dummy_guard/node",
)
FORBIDDEN_AMBIENT = (
    "CI",
    "NODE_OPTIONS",
    "NODE_PATH",
    "PYTHONHOME",
    "PYTHONINSPECT",
    "PYTHONPATH",
    "PYTHONUSERBASE",
    "PYTEST_ADDOPTS",
    "PYTEST_PLUGINS",
    "HEADFUL",
    "PLAYWRIGHT_BROWSERS_PATH",
    "PWRIGHT_CDP_ENDPOINT",
    "PWRIGHT_CDP_WS_URL",
    "PWRIGHT_OFFICIAL_LAUNCH_MODE",
    "PWDEBUG",
)


def _git(upstream: Path, *arguments: str) -> str:
    return subprocess.run(
        ["git", "-C", str(upstream), *arguments],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def _pinned_manifest(upstream: Path) -> dict[str, object]:
    if MANIFEST.is_symlink() or file_sha256(MANIFEST) != MANIFEST_SHA256:
        raise ValueError(
            "dummy-node diagnostic manifest differs from its pinned policy"
        )
    manifest = cast(dict[str, object], json.loads(MANIFEST.read_text()))
    if (
        set(manifest)
        != {
            "schema",
            "repository",
            "tag",
            "commit",
            "executionPolicy",
            "evidenceScope",
            "browser",
            "requiresBrowser",
            "requiresFixtureServers",
            "tests",
            "upstreamSources",
            "runtimeTrees",
            "candidateNativeSha256",
            "oracleDriverNodeSha256",
            "systemNodeSha256",
        }
        or manifest["schema"] != 2
        or manifest["commit"] != COMMIT
        or manifest["executionPolicy"] != POLICY
        or manifest["browser"] != "chromium"
        or manifest["requiresBrowser"] is not False
        or manifest["requiresFixtureServers"] is not True
        or manifest["evidenceScope"] != "diagnostic-only-no-general-conformance-credit"
    ):
        raise ValueError("dummy-node diagnostic manifest scope differs")
    if _git(upstream, "rev-parse", "HEAD") != COMMIT or _git(
        upstream, "status", "--porcelain"
    ):
        raise ValueError(
            "dummy-node diagnostic requires a clean pinned upstream checkout"
        )
    sources = cast(dict[str, str], manifest["upstreamSources"])
    for relative, digest in sources.items():
        path = upstream / relative
        if path.is_symlink() or not path.is_file() or file_sha256(path) != digest:
            raise ValueError(
                f"dummy-node diagnostic upstream source drifted: {relative}"
            )
    asset = upstream / ASSET
    if (
        not os.access(asset, os.X_OK)
        or asset.read_bytes() != b"#!/usr/bin/env node\n\nprocess.exit(1);\n"
    ):
        raise ValueError("dummy-node asset is not the inert pinned program")
    return manifest


def _oracle_package(python: Path) -> Path:
    probe = subprocess.run(
        [
            str(python),
            "-c",
            (
                "import playwright; from pathlib import Path; "
                "print(Path(playwright.__file__).parent.resolve())"
            ),
        ],
        check=True,
        capture_output=True,
        text=True,
        env={"PATH": "/usr/bin:/bin:/opt/homebrew/bin", "PYTHONNOUSERSITE": "1"},
    )
    package = Path(probe.stdout.strip())
    if not package.is_absolute() or package.is_symlink() or not package.is_dir():
        raise ValueError("oracle Playwright package path is invalid")
    return package


def _runtime_sources(manifest: dict[str, object], oracle: Path, node: Path) -> None:
    trees = cast(dict[str, str], manifest["runtimeTrees"])
    paths = {
        "oracle": oracle,
        "candidate": ROOT / "packages/pwright-python/python/pwright",
        "shim": OFFICIAL / "shim",
        "runtime": OFFICIAL / "runtime",
    }
    for label, path in paths.items():
        if path.is_symlink() or not path.is_dir() or tree_sha256(path) != trees[label]:
            raise ValueError(f"dummy-node {label} runtime tree drifted")
    native = paths["candidate"] / "_native.abi3.so"
    driver = oracle / "driver/node"
    if (
        native.is_symlink()
        or file_sha256(native) != manifest["candidateNativeSha256"]
        or driver.is_symlink()
        or file_sha256(driver) != manifest["oracleDriverNodeSha256"]
        or node.is_symlink()
        or file_sha256(node) != manifest["systemNodeSha256"]
    ):
        raise ValueError("dummy-node native artifact or Node executable drifted")


def _sandbox_profile(allow: list[Path]) -> str:
    if len(allow) != len(set(allow)) or not all(path.is_absolute() for path in allow):
        raise ValueError("dummy-node sandbox allowlist is not exact")
    clauses = [
        "(version 1)",
        "(allow default)",
        "(deny process-exec)",
        "(deny network-outbound)",
    ]
    clauses.extend(
        f"(allow process-exec (literal {json.dumps(str(path))}))" for path in allow
    )
    return "\n".join(clauses)


def _python_paths(mode: str, upstream: Path) -> list[Path]:
    if mode == "oracle":
        return [OFFICIAL / "runtime", GUARD_DIR]
    if mode == "candidate":
        return [
            OFFICIAL / "shim",
            ROOT / "packages/pwright-python/python",
            upstream,
            OFFICIAL / "runtime",
            GUARD_DIR,
        ]
    raise ValueError("dummy-node diagnostic mode is invalid")


def _probe_sandbox(profile: str, python: Path) -> None:
    # Arrange: only the selected Python process is started in the sandbox.
    probe = (
        "import errno, socket, subprocess\n"
        "try:\n"
        "    subprocess.run(['/usr/bin/true'], check=False)\n"
        "except PermissionError:\n"
        "    pass\n"
        "else:\n"
        "    raise SystemExit('exec-deny policy is ineffective')\n"
        "with socket.socket() as connection:\n"
        "    if connection.connect_ex(('127.0.0.1', 9)) != errno.EPERM:\n"
        "        raise SystemExit('network-deny policy is ineffective')\n"
    )
    # Act: probe an unlisted benign binary and an unused loopback port.
    completed = subprocess.run(
        ["/usr/bin/sandbox-exec", "-p", profile, str(python), "-c", probe],
        check=False,
        capture_output=True,
        text=True,
    )
    # Assert: both denied operations must be observed before pytest may start.
    if completed.returncode:
        raise ValueError(
            f"dummy-node sandbox preflight failed: {completed.stderr or completed.stdout}"
        )


def _receipt(path: Path, expected: list[dict[str, object]]) -> None:
    if path.is_symlink() or not path.is_file():
        raise ValueError(f"dummy-node receipt is missing or linked: {path}")
    observed = [json.loads(line) for line in path.read_text().splitlines() if line]
    if observed != expected:
        raise ValueError(f"dummy-node receipt differs: {path}: {observed}")


def run(args: argparse.Namespace) -> None:
    if any(key in os.environ for key in FORBIDDEN_AMBIENT) or any(
        key.startswith(("DYLD_", "LD_", "PWTEST_", "PLAYWRIGHT_", "PWRIGHT_"))
        for key in os.environ
    ):
        raise ValueError(
            "dummy-node diagnostic forbids inherited test/browser overrides"
        )
    upstream = args.upstream.resolve()
    python = args.python.absolute()
    if python.is_symlink() and not python.resolve().is_file():
        raise ValueError("dummy-node Python interpreter link is invalid")
    if not python.is_file() or not os.access(python, os.X_OK):
        raise ValueError("dummy-node Python interpreter is missing")
    real_python = python.resolve()
    node_command = shutil.which("node", path="/opt/homebrew/bin:/usr/bin:/bin")
    if node_command is None:
        raise ValueError("dummy-node diagnostic requires installed Node")
    node = Path(node_command).resolve()
    if not node.is_absolute() or not node.is_file():
        raise ValueError("dummy-node system Node is invalid")
    manifest = _pinned_manifest(upstream)
    oracle = _oracle_package(python)
    _runtime_sources(manifest, oracle, node)
    if (
        NODE_WRAPPER.is_symlink()
        or not NODE_WRAPPER.is_file()
        or not os.access(NODE_WRAPPER, os.X_OK)
    ):
        raise ValueError("dummy-node PATH wrapper is not an executable regular file")
    local_sources = {
        relative: file_sha256(ROOT / relative) for relative in PYTHON_SOURCES
    }
    bind, host = _tailscale_runtime(os.environ.copy())
    test = cast(dict[str, str], manifest["tests"])[args.api]
    asset = upstream / ASSET
    allow = [
        python,
        real_python,
        asset,
        NODE_WRAPPER,
        Path("/usr/bin/env"),
        Path("/usr/bin/python3"),
        Path("/Library/Developer/CommandLineTools/usr/bin/python3"),
        Path(
            "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/bin/python3.9"
        ),
        Path(
            "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/Resources/Python.app/Contents/MacOS/Python"
        ),
        node,
    ]
    if args.mode == "oracle":
        allow.append(oracle / "driver/node")
    profile = _sandbox_profile(allow)
    _probe_sandbox(profile, python)
    result = prepare_result_destination(args.result)
    public_receipt = result.with_suffix(".public-launch.jsonl")
    process_receipt = result.with_suffix(".dummy-process.jsonl")
    for receipt in (public_receipt, process_receipt):
        if receipt.is_symlink() or receipt.exists():
            raise ValueError(
                f"dummy-node receipt destination already exists: {receipt}"
            )
        receipt.parent.mkdir(parents=True, exist_ok=True)
        receipt.open("x").close()
    env = os.environ.copy()
    for key in FORBIDDEN_AMBIENT:
        env.pop(key, None)
    env.update(
        {
            "PATH": f"{GUARD_DIR}:/opt/homebrew/bin:/usr/bin:/bin",
            "PYTHONNOUSERSITE": "1",
            "PYTEST_DISABLE_PLUGIN_AUTOLOAD": "1",
            "PYTHONPATH": os.pathsep.join(
                str(path) for path in _python_paths(args.mode, upstream)
            ),
            "PWRIGHT_OFFICIAL_MODE": args.mode,
            "PWRIGHT_OFFICIAL_LAUNCH_MODE": "owned",
            "PWRIGHT_OFFICIAL_BROWSER_CHANNEL": "",
            "PWRIGHT_OFFICIAL_UPSTREAM_TESTS": str(upstream / "tests"),
            "PWRIGHT_OFFICIAL_TAILSCALE_IP": bind,
            "PWRIGHT_OFFICIAL_TAILSCALE_HOST": host,
            "PWRIGHT_PYTHON_DUMMY_UPSTREAM": str(upstream),
            "PWRIGHT_PYTHON_DUMMY_API": args.api,
            "PWRIGHT_PYTHON_DUMMY_LAUNCH_RECEIPT": str(public_receipt),
            "PWRIGHT_PYTHON_DUMMY_PROCESS_RECEIPT": str(process_receipt),
            "PWRIGHT_PYTHON_DUMMY_REAL_NODE": str(node),
        }
    )
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
    ]
    command.extend(
        ["-p", "pwright_candidate_runtime"]
        if args.mode == "candidate"
        else ["-p", "pwright_official_oracle_runtime"]
    )
    command.extend(
        [
            "-p",
            "pwright_official_dummy_guard",
            "--browser=chromium",
            "--timeout=20",
            "--timeout-method=signal",
            str(upstream / test),
            f"--junitxml={result}",
        ]
    )
    completed = subprocess.run(command, cwd=ROOT, env=env, check=False, timeout=60)
    if not result.is_file() or result.is_symlink():
        raise ValueError("dummy-node pytest produced no regular JUnit result")
    cases = ET.parse(result).findall(".//testcase")
    if len(cases) != 1 or not cases[0].get("name", "").startswith(test.split("::")[1]):
        raise ValueError("dummy-node JUnit did not select exactly one pinned test")
    expected_common: dict[str, object] = {
        "api": args.api,
        "mode": args.mode,
        "executablePath": str(asset),
        "assetSha256": cast(dict[str, str], manifest["upstreamSources"])[ASSET],
    }
    _receipt(
        public_receipt,
        [
            {"event": "public-launch-invoked", **expected_common},
            {"event": "public-launch-failed", **expected_common},
        ],
    )
    _receipt(
        process_receipt,
        [{"event": "dummy-node-exited", "exitCode": 1, **expected_common}],
    )
    if _pinned_manifest(upstream) != manifest:
        raise ValueError("dummy-node upstream or manifest changed during execution")
    _runtime_sources(manifest, oracle, node)
    if {
        relative: file_sha256(ROOT / relative) for relative in PYTHON_SOURCES
    } != local_sources:
        raise ValueError("dummy-node local diagnostic source changed during execution")
    status = (
        "passed"
        if completed.returncode == 0 and not list(cases[0])
        else "skipped"
        if cases[0].find("skipped") is not None
        else "failed"
    )
    provenance = {
        "schema": 1,
        "evidenceScope": "diagnostic-only-no-general-conformance-credit",
        "mode": args.mode,
        "api": args.api,
        "browser": "chromium",
        "executionPolicy": POLICY,
        "manifest": str(MANIFEST),
        "manifestSha256": MANIFEST_SHA256,
        "upstream": str(upstream),
        "commit": COMMIT,
        "test": test,
        "python": str(python),
        "oraclePackage": str(oracle),
        "runtimeTrees": manifest["runtimeTrees"],
        "candidateNativeSha256": manifest["candidateNativeSha256"],
        "oracleDriverNodeSha256": manifest["oracleDriverNodeSha256"],
        "systemNode": str(node),
        "systemNodeSha256": manifest["systemNodeSha256"],
        "localSources": local_sources,
        "sandboxProfileSha256": hashlib.sha256(profile.encode()).hexdigest(),
        "sandboxAllowedExecutables": [str(path) for path in allow],
        "sandboxNetworkOutbound": "denied",
        "fixtureBind": bind,
        "fixtureHost": host,
        "command": command,
        "exitCode": completed.returncode,
        "status": status,
        "result": str(result),
        "resultSha256": file_sha256(result),
        "publicReceipt": str(public_receipt),
        "publicReceiptSha256": file_sha256(public_receipt),
        "processReceipt": str(process_receipt),
        "processReceiptSha256": file_sha256(process_receipt),
    }
    sidecar = result.with_suffix(".provenance.json")
    with sidecar.open("x") as stream:
        json.dump(provenance, stream, indent=2, sort_keys=True)
        stream.write("\n")
    print(
        f"dummy-node Python diagnostic {args.mode}/{args.api}: {status}; result={result}"
    )
    raise SystemExit(completed.returncode)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("oracle", "candidate"))
    parser.add_argument("api", choices=("async", "sync"))
    parser.add_argument("upstream", type=Path)
    parser.add_argument(
        "--python",
        type=Path,
        default=ROOT / ".work/official-conformance/venvs/python-oracle/bin/python",
    )
    parser.add_argument("--result", type=Path, required=True)
    args = parser.parse_args()
    try:
        run(args)
    except (
        OSError,
        ValueError,
        subprocess.CalledProcessError,
        subprocess.TimeoutExpired,
        json.JSONDecodeError,
    ) as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
