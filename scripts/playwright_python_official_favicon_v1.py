"""Capture or verify the opt-in, source-bound offscreen favicon fixture variant.

The historical schema-5 runner, verifier, and official/runtime tree remain
untouched. A second receipt binds the opt-in plugin and real ICO response.
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
import subprocess
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
VARIANT_DIR = (
    ROOT / "packages/pwright-python/tests/official/fixture_variants/offscreen_favicon_v1"
)
PLUGIN = VARIANT_DIR / "pwright_official_favicon_v1.py"
ICON = VARIANT_DIR / "favicon.ico.b64"
RUNNER = ROOT / "scripts/run_playwright_python_official_conformance.py"
BASE_VERIFIER = ROOT / "scripts/verify_playwright_python_official_provenance.py"
MANIFEST = VARIANT_DIR / "manifest.json"
VARIANT_ID = "offscreen-favicon-v1"
CASE_SELECTION = "tests/async/test_click.py::test_click_offscreen_buttons"
CASE_NAME = "test_click_offscreen_buttons[chromium]"
CASE_ID = f"tests/async/test_click.py::{CASE_NAME}"
ICON_ROUTE = "/favicon.ico"
PLUGIN_ADDOPTS = "-p pwright_official_favicon_v1"
PROPERTY_VARIANT = "pwright.fixtureVariant"
PROPERTY_ICON_SHA256 = "pwright.fixtureVariantIconSha256"
PROPERTY_SERVED = "pwright.fixtureVariantServed"
SCHEMA = 1
RECEIPT_KEYS = frozenset(
    {
        "schema",
        "variant",
        "mode",
        "caseId",
        "pythonExecutable",
        "upstream",
        "fixtureBind",
        "fixtureHost",
        "result",
        "resultSha256",
        "baseProvenance",
        "baseProvenanceSha256",
        "manifestSha256",
        "pluginSha256",
        "iconAssetSha256",
        "iconBytesSha256",
        "captureToolSha256",
        "runnerSha256",
        "baseVerifierSha256",
        "runnerCommand",
        "runnerExitCode",
        "pluginPythonPath",
        "pluginPytestAddopts",
    }
)


class FixtureVariantError(ValueError):
    """A capture or receipt does not prove the exact fixture variant."""


def _file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _source_sha256(path: Path) -> str:
    if path.is_symlink() or not path.is_file() or not path.resolve().is_relative_to(ROOT):
        raise FixtureVariantError(f"variant source is missing or a symlink: {path}")
    return _file_sha256(path)


def _icon_bytes() -> bytes:
    _source_sha256(ICON)
    try:
        icon = base64.b64decode(ICON.read_bytes().strip(), validate=True)
    except (OSError, ValueError) as error:
        raise FixtureVariantError("versioned icon asset is not valid base64") from error
    if (
        len(icon) != 70
        or icon[:6] != b"\x00\x00\x01\x00\x01\x00"
        or icon[6:10] != b"\x01\x01\x00\x00"
        or int.from_bytes(icon[14:18], "little") != 48
        or int.from_bytes(icon[18:22], "little") != 22
    ):
        raise FixtureVariantError("versioned icon asset is not the expected ICO")
    return icon


def _runner_command(
    *, python: Path, mode: str, upstream: Path, fixture_bind: str,
    fixture_host: str, result: Path,
) -> list[str]:
    return [
        str(python), str(RUNNER), mode, str(upstream),
        "--manifest", str(MANIFEST),
        "--manifest-cases",
        "--manifest-api", "async",
        "--browser", "chromium",
        "--browser-channel", "chrome",
        "--endpoint", "http://127.0.0.1:9222",
        "--case-timeout", "30",
        "--tailscale-ip", fixture_bind,
        "--tailscale-host", fixture_host,
        "--result", str(result),
    ]


def _case_properties(result: Path) -> dict[str, str]:
    try:
        tree = ET.parse(result)
    except (OSError, ET.ParseError) as error:
        raise FixtureVariantError("variant JUnit result is missing or malformed") from error
    cases = list(tree.getroot().iter("testcase"))
    if len(cases) != 1:
        raise FixtureVariantError("variant JUnit must contain exactly one testcase")
    case = cases[0]
    if case.get("classname") != "tests.async.test_click" or case.get("name") != CASE_NAME:
        raise FixtureVariantError("variant JUnit contains the wrong upstream testcase")
    properties = case.find("properties")
    if properties is None:
        raise FixtureVariantError("variant JUnit is missing testcase properties")
    values: dict[str, str] = {}
    for entry in properties.findall("property"):
        name, value = entry.get("name"), entry.get("value")
        if name in {PROPERTY_VARIANT, PROPERTY_ICON_SHA256, PROPERTY_SERVED}:
            if value is None or name in values:
                raise FixtureVariantError("variant JUnit marker is duplicated or empty")
            values[name] = value
    if set(values) != {PROPERTY_VARIANT, PROPERTY_ICON_SHA256, PROPERTY_SERVED}:
        raise FixtureVariantError("plugin or ICO response was not observed in JUnit")
    return values


def _require_junit_variant(result: Path, icon_bytes_sha256: str) -> None:
    properties = _case_properties(result)
    if properties != {
        PROPERTY_VARIANT: VARIANT_ID,
        PROPERTY_ICON_SHA256: icon_bytes_sha256,
        PROPERTY_SERVED: ICON_ROUTE,
    }:
        raise FixtureVariantError("JUnit fixture marker or served ICO digest differs")


def _receipt_path(result: Path) -> Path:
    return result.with_suffix(".fixture-variant.json")


def _capture(args: argparse.Namespace) -> int:
    # Keep the venv entrypoint path: resolving its symlink would lose venv site-packages.
    python = args.python.absolute()
    upstream = args.upstream.resolve()
    result = args.result.resolve()
    if not python.is_file() or not upstream.is_dir():
        raise FixtureVariantError("Python interpreter or upstream checkout is missing")
    if result.exists() or result.with_suffix(".provenance.json").exists() or _receipt_path(result).exists():
        raise FixtureVariantError("refusing to overwrite an existing result or receipt")
    icon_bytes_sha256 = hashlib.sha256(_icon_bytes()).hexdigest()
    source_hashes = {
        "manifestSha256": _source_sha256(MANIFEST),
        "pluginSha256": _source_sha256(PLUGIN),
        "iconAssetSha256": _source_sha256(ICON),
        "captureToolSha256": _source_sha256(Path(__file__)),
        "runnerSha256": _source_sha256(RUNNER),
        "baseVerifierSha256": _source_sha256(BASE_VERIFIER),
    }
    command = _runner_command(
        python=python, mode=args.mode, upstream=upstream,
        fixture_bind=args.tailscale_ip, fixture_host=args.tailscale_host,
        result=result,
    )
    env = os.environ.copy()
    env["PYTHONPATH"] = str(VARIANT_DIR)
    env["PYTEST_ADDOPTS"] = PLUGIN_ADDOPTS
    env.pop("PYTEST_PLUGINS", None)
    env["PYTHONNOUSERSITE"] = "1"
    completed = subprocess.run(command, cwd=ROOT, env=env, check=False)
    base_provenance = result.with_suffix(".provenance.json")
    if not result.is_file() or not base_provenance.is_file():
        raise FixtureVariantError(
            f"base runner did not produce both result and sidecar (exit={completed.returncode})"
        )
    if source_hashes != {
        "manifestSha256": _source_sha256(MANIFEST),
        "pluginSha256": _source_sha256(PLUGIN),
        "iconAssetSha256": _source_sha256(ICON),
        "captureToolSha256": _source_sha256(Path(__file__)),
        "runnerSha256": _source_sha256(RUNNER),
        "baseVerifierSha256": _source_sha256(BASE_VERIFIER),
    }:
        raise FixtureVariantError("fixture variant sources changed during capture")
    _require_junit_variant(result, icon_bytes_sha256)
    receipt: dict[str, Any] = {
        "schema": SCHEMA,
        "variant": VARIANT_ID,
        "mode": args.mode,
        "caseId": CASE_ID,
        "pythonExecutable": str(python),
        "upstream": str(upstream),
        "fixtureBind": args.tailscale_ip,
        "fixtureHost": args.tailscale_host,
        "result": str(result),
        "resultSha256": _file_sha256(result),
        "baseProvenance": str(base_provenance),
        "baseProvenanceSha256": _file_sha256(base_provenance),
        "iconBytesSha256": icon_bytes_sha256,
        "runnerCommand": command,
        "runnerExitCode": completed.returncode,
        "pluginPythonPath": str(VARIANT_DIR),
        "pluginPytestAddopts": PLUGIN_ADDOPTS,
        **source_hashes,
    }
    _verify_payload(receipt, _receipt_path(result), verify_base=True)
    _receipt_path(result).write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n")
    print(f"fixture-variant provenance={_receipt_path(result)}", flush=True)
    return completed.returncode


def _verify_payload(raw: object, receipt_path: Path, *, verify_base: bool) -> dict[str, Any]:
    if not isinstance(raw, dict) or set(raw) != RECEIPT_KEYS:
        raise FixtureVariantError("fixture variant receipt fields differ from schema 1")
    payload: dict[str, Any] = raw
    if type(payload["schema"]) is not int or payload["schema"] != SCHEMA or payload["variant"] != VARIANT_ID:
        raise FixtureVariantError("wrong fixture variant schema or identity")
    if payload["mode"] not in {"oracle", "candidate"} or payload["caseId"] != CASE_ID:
        raise FixtureVariantError("wrong fixture variant mode or exact case")
    for key in (
        "pythonExecutable", "upstream", "fixtureBind", "fixtureHost", "result",
        "baseProvenance", "resultSha256", "baseProvenanceSha256",
        "manifestSha256",
        "pluginSha256", "iconAssetSha256", "iconBytesSha256",
        "captureToolSha256", "runnerSha256", "baseVerifierSha256",
        "pluginPythonPath", "pluginPytestAddopts",
    ):
        if not isinstance(payload[key], str) or not payload[key]:
            raise FixtureVariantError(f"fixture variant {key} must be a non-empty string")
    if (
        type(payload["runnerExitCode"]) is not int
        or payload["runnerExitCode"] < 0
        or not isinstance(payload["runnerCommand"], list)
        or not all(isinstance(part, str) for part in payload["runnerCommand"])
    ):
        raise FixtureVariantError("fixture variant runner command or exit code is invalid")
    result = Path(payload["result"])
    base_provenance = Path(payload["baseProvenance"])
    python = Path(payload["pythonExecutable"])
    upstream = Path(payload["upstream"])
    if (
        not result.is_absolute()
        or not base_provenance.is_absolute()
        or not python.is_absolute()
        or not upstream.is_absolute()
        or receipt_path.is_symlink()
        or result.is_symlink()
        or base_provenance.is_symlink()
        or not python.is_file()
        or not upstream.is_dir()
        or receipt_path.resolve() != _receipt_path(result).resolve()
        or base_provenance != result.with_suffix(".provenance.json")
    ):
        raise FixtureVariantError("fixture variant paths are invalid or mismatched")
    for key, path in (
        ("manifestSha256", MANIFEST),
        ("pluginSha256", PLUGIN),
        ("iconAssetSha256", ICON),
        ("captureToolSha256", Path(__file__)),
        ("runnerSha256", RUNNER),
        ("baseVerifierSha256", BASE_VERIFIER),
    ):
        if payload[key] != _source_sha256(path):
            raise FixtureVariantError(f"fixture variant source hash drifted: {key}")
    icon_bytes_sha256 = hashlib.sha256(_icon_bytes()).hexdigest()
    if payload["iconBytesSha256"] != icon_bytes_sha256:
        raise FixtureVariantError("decoded ICO digest drifted")
    for key, path in (
        ("resultSha256", result),
        ("baseProvenanceSha256", base_provenance),
    ):
        if not path.is_file() or payload[key] != _file_sha256(path):
            raise FixtureVariantError(f"fixture variant artifact hash drifted: {key}")
    if (
        payload["pluginPythonPath"] != str(VARIANT_DIR)
        or payload["pluginPytestAddopts"] != PLUGIN_ADDOPTS
    ):
        raise FixtureVariantError("fixture plugin environment differs")
    expected_command = _runner_command(
        python=python, mode=payload["mode"], upstream=upstream,
        fixture_bind=payload["fixtureBind"], fixture_host=payload["fixtureHost"],
        result=result,
    )
    if payload["runnerCommand"] != expected_command:
        raise FixtureVariantError("runner command differs from fixed variant invocation")
    _require_junit_variant(result, icon_bytes_sha256)
    try:
        base_payload = json.loads(base_provenance.read_text())
    except (OSError, json.JSONDecodeError) as error:
        raise FixtureVariantError("base schema-5 sidecar is malformed") from error
    if not isinstance(base_payload, dict):
        raise FixtureVariantError("base schema-5 sidecar is not an object")
    if (
        base_payload.get("schema") != 5
        or base_payload.get("mode") != payload["mode"]
        or base_payload.get("selections") != [CASE_SELECTION]
        or base_payload.get("manifestCases") is not True
        or base_payload.get("manifestApi") != "async"
        or base_payload.get("browsers") != ["chromium"]
        or base_payload.get("browserChannel") != "chrome"
        or base_payload.get("manifest") != str(MANIFEST)
        or base_payload.get("result") != str(result)
        or base_payload.get("exitCode") != payload["runnerExitCode"]
        or base_payload.get("fixtureBind") != payload["fixtureBind"]
        or base_payload.get("fixtureHost") != payload["fixtureHost"]
        or base_payload.get("pythonExecutable") != str(python)
    ):
        raise FixtureVariantError("base sidecar does not describe the exact fixture variant run")
    if verify_base:
        completed = subprocess.run(
            [str(python), str(BASE_VERIFIER), str(base_provenance)],
            cwd=ROOT, capture_output=True, text=True, check=False,
        )
        if completed.returncode:
            raise FixtureVariantError(
                "base schema-5 provenance failed verification: "
                + (completed.stderr or completed.stdout).strip()
            )
    return payload


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="action", required=True)
    capture = subparsers.add_parser("capture")
    capture.add_argument("mode", choices=("oracle", "candidate"))
    capture.add_argument("upstream", type=Path)
    capture.add_argument("--python", type=Path, required=True)
    capture.add_argument("--tailscale-ip", required=True)
    capture.add_argument("--tailscale-host", required=True)
    capture.add_argument("--result", type=Path, required=True)
    verify = subparsers.add_parser("verify")
    verify.add_argument("receipt", type=Path)
    args = parser.parse_args()
    try:
        if args.action == "capture":
            raise SystemExit(_capture(args))
        receipt = args.receipt.resolve()
        raw = json.loads(receipt.read_text())
        payload = _verify_payload(raw, receipt, verify_base=True)
        print(
            f"verified fixture variant={payload['variant']} mode={payload['mode']} "
            f"resultSha256={payload['resultSha256']}"
        )
    except (OSError, ValueError, json.JSONDecodeError) as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
