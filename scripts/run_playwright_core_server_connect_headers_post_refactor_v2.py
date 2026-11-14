"""Capture the frozen connect-header suite against the N-API arity refactor."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
from pathlib import Path

import run_playwright_core_server_connect_headers_strict_v1 as base

ROOT = Path(__file__).parents[1].resolve()
ARTIFACT_SHA256 = "0b2553d3137a77f162836d8fadeacbf1d5e47038b24ae97fe9d1c9519d068194"
SCOPE = (
    "official-derived-core-server-connect-headers-fixture-host-adapted-"
    "internal-post-refactor-v2"
)
RESULT_PARENT = (
    ROOT
    / ".work/official-conformance/results/core-server-connect-headers-strict-v1/post-refactor-v2"
)
VERIFIER = (
    ROOT / "scripts/verify_playwright_core_server_connect_headers_post_refactor_v2.py"
)
TESTS = (
    ROOT
    / "scripts/test_verify_playwright_core_server_connect_headers_post_refactor_v2.py"
)
INPUT_FILES = (*base.INPUT_FILES, Path(__file__).resolve(), VERIFIER, TESTS)
ADDON_ENVIRONMENT = ROOT / "crates/pwright-node-addon/src/environment.rs"
ADDON_ENVIRONMENT_SHA256 = (
    "53379d2dd4ed44f25050c9acb1d0ae64cea0b14e3d302f116a96e37456884995"
)
ADDON_LIB = ROOT / "crates/pwright-node-addon/src/lib.rs"
ADDON_LIB_SHA256 = "45dafc4f07e89fb5ae413a5fbb6e73334d09e3487f3be3b6345c05552b8bbaca"
CANDIDATE_INDEX_SHA256 = (
    "510bec0e66c277d53bef5620b57b47068013ac82425f7374c18ed9d72dcf133f"
)
CANDIDATE_NATIVE_ARTIFACT_SHA256 = (
    "48c39527e49b5b5489b557f084a41ec1f46ab97c01919070c2e1d8e819d65c11"
)
ORIGINAL_WRITE_JSON_EXCLUSIVE = base.write_json_exclusive


def expected_abi_shape() -> dict:
    return {
        "schema": 1,
        "contract": "five-argument-connect-with-three-string-runtime-identity-tuple-v2",
        "markerExport": "playwrightServerConnectHeadersAbiShape",
        "markerValue": 2,
        "enabledExport": "playwrightServerConnectHeadersEnabled",
        "port": "NativeEnvironment.prototype.connectPlaywrightServerHeaders",
        "runtimeIdentityFields": ["platform", "arch", "release"],
        "runtimeIdentityMaxUtf8Bytes": [64, 64, 128],
        "addonEnvironmentSha256": ADDON_ENVIRONMENT_SHA256,
        "addonLibSha256": ADDON_LIB_SHA256,
        "candidateIndexSha256": CANDIDATE_INDEX_SHA256,
        "candidateNativeArtifactSha256": CANDIDATE_NATIVE_ARTIFACT_SHA256,
        "artifactSha256": ARTIFACT_SHA256,
    }


def probe_artifact_abi(artifact: Path) -> dict:
    environment = os.environ.copy()
    environment.pop("NODE_OPTIONS", None)
    script = (
        "const b=require(process.argv[1]);process.stdout.write(JSON.stringify({"
        "enabled:b.playwrightServerConnectHeadersEnabled?.(),"
        "shape:b.playwrightServerConnectHeadersAbiShape?.(),"
        "port:typeof b.NativeEnvironment?.prototype?.connectPlaywrightServerHeaders"
        "}))"
    )
    output = subprocess.check_output(
        [
            "node",
            "-e",
            script,
            str(artifact),
        ],
        env=environment,
        text=True,
    )
    return json.loads(output)


def check_abi_sources() -> None:
    assert base.sha256(ADDON_ENVIRONMENT) == ADDON_ENVIRONMENT_SHA256
    assert base.sha256(ADDON_LIB) == ADDON_LIB_SHA256
    index = base.SOURCE_PACKAGE / "src/index.cjs"
    native_artifact = base.SOURCE_PACKAGE / "src/native-artifact.cjs"
    assert base.sha256(index) == CANDIDATE_INDEX_SHA256
    assert base.sha256(native_artifact) == CANDIDATE_NATIVE_ARTIFACT_SHA256
    assert "runtime_identity: Unknown<'env>," in ADDON_ENVIRONMENT.read_text()
    assert "values.get_array_length()? != 3" in ADDON_ENVIRONMENT.read_text()
    assert (
        "bounded_server_connect_runtime_identity(runtime_identity)?"
        in ADDON_ENVIRONMENT.read_text()
    )
    assert (
        "pub fn playwright_server_connect_headers_abi_shape() -> u32 {\n    2\n}"
        in ADDON_LIB.read_text()
    )
    assert "[os.platform(), os.arch(), os.release()]" in index.read_text()
    assert (
        "serverConnectHeadersAbiShape !== (serverConnectHeaders ? 2 : 0)"
        in native_artifact.read_text()
    )


def write_v2_sidecar(destination: Path, value: dict) -> None:
    value["abiShape"] = expected_abi_shape()
    ORIGINAL_WRITE_JSON_EXCLUSIVE(destination, value)


def configure(*, check_live: bool = True) -> None:
    if check_live:
        check_abi_sources()
        artifact = base.SOURCE_PACKAGE / "native" / base.PROFILE / "pwright.node"
        assert base.sha256(artifact) == ARTIFACT_SHA256
        assert probe_artifact_abi(artifact) == {
            "enabled": True,
            "shape": 2,
            "port": "function",
        }
    base.ARTIFACT_SHA256 = ARTIFACT_SHA256
    base.SCOPE = SCOPE
    base.INPUT_FILES = INPUT_FILES
    base.write_json_exclusive = write_v2_sidecar


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("oracle", "candidate"))
    args = parser.parse_args()
    configure()
    strict = base.run(args.mode, RESULT_PARENT)
    print(
        f"Core connect headers post-refactor diagnostic {args.mode}: "
        f"{[(item['kind'], item['headerCase'], item['status']) for item in strict['selected']]}"
    )


if __name__ == "__main__":
    main()
