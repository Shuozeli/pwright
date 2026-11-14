#!/usr/bin/env python3
"""Verify a Playwright Core conformance result provenance sidecar."""

from __future__ import annotations

import argparse
import hashlib
import ipaddress
import json
import re
import subprocess
from pathlib import Path
from typing import TYPE_CHECKING, cast
from urllib.parse import urlsplit

if TYPE_CHECKING:
    from run_playwright_official_conformance import Suite


class ProvenanceValidationError(ValueError):
    """Raised when a sidecar cannot prove the result it describes."""


_SHA256_PATTERN = re.compile(r"^[0-9a-f]{64}$")
_EXPECTED_KEYS = frozenset(
    {
        "schema",
        "mode",
        "repository",
        "tag",
        "commit",
        "manifest",
        "manifestSha256",
        "upstream",
        "suite",
        "suiteRole",
        "adapter",
        "project",
        "filters",
        "grep",
        "workers",
        "headed",
        "endpoint",
        "fixtureBind",
        "fixtureHost",
        "candidate",
        "candidateProfile",
        "candidateSourceSha256",
        "candidateArtifact",
        "candidateArtifactSha256",
        "harnessInputs",
        "command",
        "exitCode",
        "result",
        "resultSha256",
    }
)
_SECURE_EXPECTED_KEYS = _EXPECTED_KEYS | {"secureFixture"}
_TOOLING_KEY = "toolingSources"
_TOOLING_SOURCE_PATHS = (
    "scripts/official_result_destination.py",
    "scripts/run_playwright_official_conformance.py",
)
_SOURCE_ONLY_GUARD_SOURCE = "tests/official/playwright-core-internal-source-only-guard.cjs"
_SECURE_FIXTURE_MODE = "trusted-tailscale-tls-dual-port"
_SOURCE_ONLY_FIXTURE_MODE = "pinned-private-source-only-no-io"


def _fail(message: str) -> None:
    raise ProvenanceValidationError(message)


def _string(payload: dict[str, object], key: str) -> str:
    value = payload[key]
    if not isinstance(value, str) or not value:
        _fail(f"{key} must be a non-empty string")
    return value


def _nullable_string(payload: dict[str, object], key: str) -> str | None:
    value = payload[key]
    if value is not None and (not isinstance(value, str) or not value):
        _fail(f"{key} must be null or a non-empty string")
    return value if isinstance(value, str) else None


def _sha256(payload: dict[str, object], key: str, *, nullable: bool = False) -> str | None:
    value = payload[key]
    if nullable and value is None:
        return None
    if not isinstance(value, str) or not _SHA256_PATTERN.fullmatch(value):
        _fail(f"{key} must be a lowercase SHA-256 digest")
    return value


def _verify_file_digest(path: Path, expected: str, label: str) -> None:
    if not path.is_file():
        _fail(f"{label} is missing: {path}")
    actual = hashlib.sha256(path.read_bytes()).hexdigest()
    if actual != expected:
        _fail(f"{label} SHA-256 mismatch: expected {expected}, got {actual}")


def _verify_tooling_sources(raw: object, root: Path, *, source_only: bool = False) -> None:
    sources = _TOOLING_SOURCE_PATHS + ((_SOURCE_ONLY_GUARD_SOURCE,) if source_only else ())
    if not isinstance(raw, list) or len(raw) != len(sources):
        _fail("toolingSources must contain exactly the fixed Core capture sources")
    if (root / "scripts").is_symlink():
        _fail("toolingSources source directory is a symlink")
    for index, relative in enumerate(sources):
        record = raw[index]
        if not isinstance(record, dict) or set(record) != {"path", "sha256"}:
            _fail(f"toolingSources[{index}] must contain only path and sha256")
        if record["path"] != relative:
            _fail(f"toolingSources[{index}] has the wrong fixed source path")
        digest = record["sha256"]
        if not isinstance(digest, str) or not _SHA256_PATTERN.fullmatch(digest):
            _fail(f"toolingSources[{index}].sha256 is invalid")
        path = root / relative
        if (
            path.is_symlink()
            or not path.is_file()
            or not path.resolve().is_relative_to(root)
        ):
            _fail(f"toolingSources[{index}] source is missing or a symlink: {relative}")
        _verify_file_digest(path, digest, f"toolingSources[{index}]")


def _expected_command(
    payload: dict[str, object], suite: Suite, recorded_upstream: Path
) -> list[str]:
    command = ["node", str(recorded_upstream / suite["runner"]), "test"]
    if config := suite.get("config"):
        command.append(f"--config={config}")
    command.extend(["--reporter=line,json", f"--workers={payload['workers']}"])
    project = payload["project"]
    if project is not None:
        command.append(f"--project={project}")
    if payload["headed"]:
        command.append("--headed")
    grep = payload["grep"]
    if grep is not None:
        command.append(f"--grep={grep}")
    grep_invert = payload.get("grepInvert")
    if grep_invert is not None:
        command.append(f"--grep-invert={grep_invert}")
    command.extend(cast(list[str], payload["filters"]))
    return command


def verify_provenance(
    raw: object,
    *,
    provenance_path: Path,
    manifest_override: Path | None = None,
    result_override: Path | None = None,
    upstream_override: Path | None = None,
    candidate_override: Path | None = None,
    tooling_root: Path | None = None,
    check_checkout: bool = True,
) -> dict[str, object]:
    """Validate one sidecar and all local evidence that it fingerprints."""

    if not isinstance(raw, dict) or not all(isinstance(key, str) for key in raw):
        _fail("provenance root must be a JSON object with string keys")
    payload = cast(dict[str, object], raw)
    actual_keys = frozenset(payload)
    schema = payload.get("schema")
    if isinstance(schema, bool) or schema not in {1, 2, 3}:
        _fail(f"unsupported provenance schema: {schema!r}")
    tooling_key = {_TOOLING_KEY} if schema in {2, 3} else set()
    source_only_key = {"sourceOnlyExecution"} if schema == 3 else set()
    expected_keys = _EXPECTED_KEYS | tooling_key | source_only_key
    secure_expected_keys = _SECURE_EXPECTED_KEYS | tooling_key
    allowed_keys = {expected_keys} if schema == 3 else {expected_keys, secure_expected_keys}
    if schema == 2:
        allowed_keys.add(expected_keys | {"grepInvert"})
    if actual_keys not in allowed_keys:
        missing = sorted(expected_keys - actual_keys)
        extra = sorted(actual_keys - (expected_keys | secure_expected_keys))
        _fail(f"provenance fields differ: missing={missing}, extra={extra}")
    if schema in {2, 3}:
        _verify_tooling_sources(
            payload[_TOOLING_KEY],
            (tooling_root or Path(__file__).parents[1]).resolve(),
            source_only=schema == 3,
        )
    # Import capture helpers only after schema-2 tooling identity has passed.
    from run_playwright_official_conformance import (
        Manifest,
        Suite,
        _candidate_artifact_path,
        _candidate_source_sha256,
        _cross_origin_forward_enabled,
        _git,
        _pinned_full_file_source,
        _pinned_grep,
        _pinned_grep_invert,
        _secure_fixture_source_digests,
        _verify_pinned_full_file_result_selection,
        _verify_secure_result_selection,
        _verify_source_only_guard_receipt,
        _verify_source_only_result_selection,
        _verify_source_only_suite,
    )

    mode = _string(payload, "mode")
    if mode not in {"candidate", "oracle"}:
        _fail("mode must be candidate or oracle")
    repository = _string(payload, "repository")
    tag = _string(payload, "tag")
    commit = _string(payload, "commit")
    manifest_digest = cast(str, _sha256(payload, "manifestSha256"))
    recorded_manifest = Path(_string(payload, "manifest"))
    actual_manifest = (manifest_override or recorded_manifest).resolve()
    _verify_file_digest(actual_manifest, manifest_digest, "manifest")
    parsed_manifest = json.loads(actual_manifest.read_text())
    if not isinstance(parsed_manifest, dict):
        _fail("manifest root must be a JSON object")
    manifest = cast(Manifest, parsed_manifest)
    if manifest.get("schema") != 1:
        _fail("manifest schema must be 1")
    if (
        manifest.get("repository") != repository
        or manifest.get("tag") != tag
        or manifest.get("commit") != commit
    ):
        _fail("manifest repository, tag or commit differs from provenance")
    manifest_suites = manifest.get("suites")
    if not isinstance(manifest_suites, list):
        _fail("manifest suites must be a list")

    recorded_upstream = Path(_string(payload, "upstream"))
    actual_upstream = (upstream_override or recorded_upstream).resolve()
    suite_id = _string(payload, "suite")
    suites = {
        suite["id"]: suite
        for suite in manifest_suites
        if isinstance(suite, dict) and isinstance(suite.get("id"), str)
    }
    if suite_id not in suites:
        _fail(f"suite {suite_id!r} is absent from the pinned manifest")
    suite = cast(Suite, suites[suite_id])
    if not isinstance(suite.get("role"), str) or not isinstance(suite.get("runner"), str):
        _fail("suite role and runner must be strings")
    if payload["suiteRole"] != suite.get("role"):
        _fail("suiteRole differs from the pinned manifest")
    adapter_key = "candidateAdapter" if mode == "candidate" else "oracleAdapter"
    if payload["adapter"] != suite.get(adapter_key):
        _fail("adapter differs from the pinned manifest")
    secure_fixture = suite.get("fixtureMode") == _SECURE_FIXTURE_MODE
    source_only = suite.get("fixtureMode") == _SOURCE_ONLY_FIXTURE_MODE
    if suite.get("fixtureMode") not in (None, _SECURE_FIXTURE_MODE, _SOURCE_ONLY_FIXTURE_MODE):
        _fail("unknown official Core fixture mode")
    if source_only != (schema == 3):
        _fail("sourceOnlyExecution schema does not match the pinned fixture mode")
    if secure_fixture != (actual_keys == secure_expected_keys):
        _fail("secureFixture field does not match the pinned fixture mode")

    project = _nullable_string(payload, "project")
    fixed_project = suite.get("project")
    if fixed_project is not None and project != fixed_project:
        _fail("project differs from the suite's fixed project")
    filters = payload["filters"]
    if not isinstance(filters, list) or not all(isinstance(item, str) and item for item in filters):
        _fail("filters must be a list of non-empty strings")
    if not suite.get("requiresBrowser", True) and filters != suite.get("args", []):
        _fail("browser-independent suite filters differ from the manifest")
    grep = _nullable_string(payload, "grep")
    if secure_fixture and (filters != suite.get("args", []) or grep is not None):
        _fail("secure Core fixture selection differs from the pinned 17-case file")
    pinned_full_file = (
        not secure_fixture
        and not source_only
        and suite.get("upstreamTestSourceSha256") is not None
    )
    if not pinned_full_file and (
        cast(dict[str, object], suite).get("pinnedGrep") is not None
        or cast(dict[str, object], suite).get("pinnedGrepInvert") is not None
    ):
        _fail("pinned Core grep requires a source-bound file selection")
    grep_invert = _nullable_string(payload, "grepInvert") if "grepInvert" in payload else None
    if pinned_full_file:
        try:
            _pinned_full_file_source(suite, actual_upstream)
            fixed_grep = _pinned_grep(suite)
            fixed_grep_invert = _pinned_grep_invert(suite)
        except (OSError, ValueError) as error:
            raise ProvenanceValidationError(str(error)) from error
        if (
            filters != suite.get("args", [])
            or grep != fixed_grep
            or grep_invert != fixed_grep_invert
            or ("grepInvert" in payload) != (fixed_grep_invert is not None)
        ):
            _fail("pinned full-file Core selection differs from its manifest")
    elif grep_invert is not None:
        _fail("grepInvert requires a source-bound pinned public selection")
    try:
        cross_origin_forward = _cross_origin_forward_enabled(suite)
    except ValueError as error:
        raise ProvenanceValidationError(str(error)) from error
    if cross_origin_forward and not pinned_full_file:
        _fail("cross-origin forward requires an exact pinned full-file selection")
    workers = payload["workers"]
    if isinstance(workers, bool) or not isinstance(workers, int) or workers < 1:
        _fail("workers must be a positive integer")
    if not isinstance(payload["headed"], bool):
        _fail("headed must be a boolean")
    if source_only and (
        project is not None or grep is not None or payload["headed"] or workers != 1
        or suite.get("requiresBrowser") is not False
        or filters != suite.get("args", [])
    ):
        _fail("source-only selection must be fixed, one-worker, headless, and browserless")

    endpoint = _string(payload, "endpoint")
    parsed_endpoint = urlsplit(endpoint)
    if parsed_endpoint.scheme not in {"http", "https", "ws", "wss"} or (
        parsed_endpoint.hostname not in {"127.0.0.1", "localhost", "::1"}
    ):
        _fail("endpoint must be a loopback HTTP(S) or WS(S) URL")
    if source_only:
        if payload["fixtureBind"] != "" or payload["fixtureHost"] != "":
            _fail("source-only execution must not configure a fixture server")
        fixture_bind = fixture_host = ""
    else:
        fixture_bind = _string(payload, "fixtureBind")
        try:
            fixture_address = ipaddress.ip_address(fixture_bind)
        except ValueError as error:
            raise ProvenanceValidationError("fixtureBind must be an IPv4 address") from error
        if (
            fixture_address.version != 4
            or fixture_address.is_loopback
            or fixture_address.is_unspecified
        ):
            _fail("fixtureBind must be a non-loopback IPv4 address")
        fixture_host = _string(payload, "fixtureHost").rstrip(".")
        if not fixture_host.endswith(".ts.net") or fixture_host in {
            "localhost",
            fixture_bind,
        }:
            _fail("fixtureHost must be a full MagicDNS hostname")
    if secure_fixture:
        fixture = payload["secureFixture"]
        if not isinstance(fixture, dict) or set(fixture) != {
            "mode",
            "fixtureHost",
            "ordinaryScheme",
            "crossProcessScheme",
            "crossProcessPortOffset",
            "isolationGuarantee",
            "reportedTests",
            "upstreamSources",
            "trustedTlsPreflight",
        }:
            _fail("secureFixture must contain exactly the pinned TLS evidence fields")
        try:
            expected_sources = _secure_fixture_source_digests(suite, actual_upstream)
        except ValueError as error:
            raise ProvenanceValidationError(str(error)) from error
        if fixture != {
            "mode": _SECURE_FIXTURE_MODE,
            "fixtureHost": fixture_host,
            "ordinaryScheme": "https",
            "crossProcessScheme": "https",
            "crossProcessPortOffset": 1,
            "isolationGuarantee": "distinct-origin-same-site-possible",
            "reportedTests": 17,
            "upstreamSources": expected_sources,
            "trustedTlsPreflight": True,
        }:
            _fail("secureFixture evidence differs from the pinned trusted TLS lane")

    command = payload["command"]
    if (
        not isinstance(command, list)
        or not command
        or not all(isinstance(item, str) and item for item in command)
    ):
        _fail("command must be a non-empty list of non-empty strings")
    expected_command = _expected_command(payload, suite, recorded_upstream)
    if command != expected_command:
        _fail("command does not match the pinned suite and recorded selection")
    exit_code = payload["exitCode"]
    if isinstance(exit_code, bool) or not isinstance(exit_code, int):
        _fail("exitCode must be an integer")

    result_digest = cast(str, _sha256(payload, "resultSha256"))
    recorded_result = Path(_string(payload, "result"))
    actual_result = (result_override or recorded_result).resolve()
    _verify_file_digest(actual_result, result_digest, "result")
    if secure_fixture:
        try:
            _verify_secure_result_selection(actual_result, cast(str, project), suite)
        except (OSError, TypeError, ValueError, json.JSONDecodeError) as error:
            raise ProvenanceValidationError(str(error)) from error
    if pinned_full_file:
        try:
            _verify_pinned_full_file_result_selection(actual_result, suite, actual_upstream)
        except (OSError, TypeError, ValueError, json.JSONDecodeError) as error:
            raise ProvenanceValidationError(str(error)) from error
    if source_only:
        try:
            source = _verify_source_only_suite(
                suite,
                actual_upstream,
                actual_manifest,
                (tooling_root or Path(__file__).parents[1]).resolve(),
            )
            reported_tests = _verify_source_only_result_selection(actual_result, suite, source)
        except (OSError, TypeError, ValueError, json.JSONDecodeError) as error:
            raise ProvenanceValidationError(str(error)) from error
        evidence = payload["sourceOnlyExecution"]
        if not isinstance(evidence, dict) or set(evidence) != {
            "scope", "candidateRuntimeAttested", "publicApiParityEvidence", "source",
            "reportedTests", "guardReceipt", "guardReceiptSha256", "guardedWorkerProcesses",
        }:
            _fail("sourceOnlyExecution must contain exactly the bounded evidence fields")
        receipt_recorded = recorded_result.with_suffix(".guard-receipt.jsonl")
        if evidence["guardReceipt"] != str(receipt_recorded):
            _fail("sourceOnlyExecution receipt path is not the result's sibling")
        receipt_actual = actual_result.with_suffix(".guard-receipt.jsonl")
        receipt_digest = evidence["guardReceiptSha256"]
        if not isinstance(receipt_digest, str) or not _SHA256_PATTERN.fullmatch(receipt_digest):
            _fail("sourceOnlyExecution receipt digest is invalid")
        _verify_file_digest(receipt_actual, receipt_digest, "source-only guard receipt")
        try:
            worker_count = _verify_source_only_guard_receipt(receipt_actual, suite, source)
        except (OSError, TypeError, ValueError, json.JSONDecodeError) as error:
            raise ProvenanceValidationError(str(error)) from error
        if evidence != {
            "scope": "upstream-private-source-only",
            "candidateRuntimeAttested": False,
            "publicApiParityEvidence": False,
            "source": source,
            "reportedTests": reported_tests,
            "guardReceipt": str(receipt_recorded),
            "guardReceiptSha256": receipt_digest,
            "guardedWorkerProcesses": worker_count,
        }:
            _fail("sourceOnlyExecution evidence differs from the bounded source-only run")
    if result_override is None:
        expected_sidecar = recorded_result.with_suffix(".provenance.json").resolve()
        if provenance_path.resolve() != expected_sidecar:
            _fail("provenance sidecar is not the recorded result's sibling")

    harness_inputs = payload["harnessInputs"]
    requires_browser = suite.get("requiresBrowser", True)
    expected_harness_count = (1 + int(mode == "candidate")) if source_only else (
        (2 if mode == "candidate" or requires_browser else 1) + int(secure_fixture)
    )
    if not isinstance(harness_inputs, list) or len(harness_inputs) != expected_harness_count:
        _fail(f"harnessInputs must contain exactly {expected_harness_count} records")
    expected_harness_names = (
        ["playwright-core-internal-source-only-guard.cjs"] if source_only else
        ["playwright-core-server-runtime.cjs"]
    )
    if secure_fixture:
        expected_harness_names.append("playwright-core-secure-server-runtime.cjs")
    if mode == "candidate":
        expected_harness_names.append("playwright-core-candidate-preload.cjs")
    elif requires_browser:
        expected_harness_names.append("playwright-core-oracle-preload.cjs")
    for index, expected_name in enumerate(expected_harness_names):
        record = harness_inputs[index]
        if not isinstance(record, dict) or set(record) != {"path", "sha256"}:
            _fail(f"harnessInputs[{index}] must contain only path and sha256")
        path_value = record["path"]
        digest_value = record["sha256"]
        if not isinstance(path_value, str) or Path(path_value).name != expected_name:
            _fail(f"harnessInputs[{index}] has the wrong harness path")
        if not isinstance(digest_value, str) or not _SHA256_PATTERN.fullmatch(digest_value):
            _fail(f"harnessInputs[{index}].sha256 is invalid")
        _verify_file_digest(Path(path_value), digest_value, f"harnessInputs[{index}]")

    candidate = _nullable_string(payload, "candidate")
    candidate_profile = _nullable_string(payload, "candidateProfile")
    source_digest = _sha256(payload, "candidateSourceSha256", nullable=True)
    recorded_artifact = _nullable_string(payload, "candidateArtifact")
    artifact_digest = _sha256(payload, "candidateArtifactSha256", nullable=True)
    if mode == "oracle":
        if any(
            value is not None
            for value in (
                candidate,
                candidate_profile,
                source_digest,
                recorded_artifact,
                artifact_digest,
            )
        ):
            _fail("oracle provenance must not contain candidate inputs")
    else:
        if candidate is None or candidate_profile is None or source_digest is None:
            _fail("candidate provenance must identify its package, profile and source")
        recorded_candidate = Path(candidate)
        expected_recorded_artifact = _candidate_artifact_path(recorded_candidate, candidate_profile)
        if recorded_artifact != str(expected_recorded_artifact):
            _fail("candidateArtifact does not match candidateProfile")
        actual_candidate = (candidate_override or recorded_candidate).resolve()
        try:
            actual_source_digest = _candidate_source_sha256(actual_candidate)
        except ValueError as error:
            raise ProvenanceValidationError(str(error)) from error
        if actual_source_digest != source_digest:
            _fail("candidate source SHA-256 mismatch")
        actual_artifact = _candidate_artifact_path(actual_candidate, candidate_profile)
        if artifact_digest is None:
            if actual_artifact.is_file():
                _fail("candidate artifact appeared after a missing-artifact run")
            if exit_code == 0 and not source_only:
                _fail("successful candidate run cannot omit its artifact digest")
        else:
            _verify_file_digest(actual_artifact, artifact_digest, "candidate artifact")

    if check_checkout:
        if _git(actual_upstream, "rev-parse", "HEAD") != commit:
            _fail("upstream checkout no longer matches the pinned commit")
        if _git(actual_upstream, "status", "--porcelain", "--untracked-files=no"):
            _fail("upstream checkout has tracked source changes")
    runner = actual_upstream / suite["runner"]
    if not runner.is_file():
        _fail(f"suite runner is missing: {runner}")
    return payload


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("provenance", type=Path)
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--result", type=Path)
    parser.add_argument("--upstream", type=Path)
    parser.add_argument("--candidate", type=Path)
    parser.add_argument("--tooling-root", type=Path)
    args = parser.parse_args()

    try:
        raw = json.loads(args.provenance.read_text())
        payload = verify_provenance(
            raw,
            provenance_path=args.provenance,
            manifest_override=args.manifest,
            result_override=args.result,
            upstream_override=args.upstream,
            candidate_override=args.candidate,
            tooling_root=args.tooling_root,
        )
    except (
        OSError,
        json.JSONDecodeError,
        ProvenanceValidationError,
        subprocess.CalledProcessError,
    ) as error:
        parser.error(str(error))
    print(
        "verified Playwright Core provenance "
        f"mode={payload['mode']} suite={payload['suite']} "
        f"tooling={'source-bound' if payload['schema'] in (2, 3) else 'legacy-unbound'} "
        f"resultSha256={payload['resultSha256']}"
    )


if __name__ == "__main__":
    main()
