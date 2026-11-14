#!/usr/bin/env python3
"""Tests for Playwright Core result provenance verification."""

from __future__ import annotations

import copy
import hashlib
import json
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from typing import cast
from unittest.mock import patch

from run_playwright_official_conformance import (
    Manifest,
    Suite,
    _result_provenance,
    _tooling_sources,
    _verify_source_only_suite,
)
from test_run_playwright_official_conformance import _source_only_test_fixture
from verify_playwright_official_provenance import (
    ProvenanceValidationError,
    verify_provenance,
)


def _fixture(
    root: Path, *, schema2: bool = False
) -> tuple[dict[str, object], Path, Path, Path, Path]:
    upstream = root / "upstream"
    runner = upstream / "packages/playwright/cli.js"
    runner.parent.mkdir(parents=True)
    runner.write_text("'use strict';\n")
    suite = cast(
        Suite,
        {
            "id": "library",
            "role": "official-entrypoint",
            "runner": "packages/playwright/cli.js",
            "config": "tests/library/playwright.config.ts",
            "candidateAdapter": "test-mode-playwright-core",
            "oracleAdapter": "test-mode-playwright-core-cdp",
        },
    )
    manifest = cast(
        Manifest,
        {
            "schema": 1,
            "repository": "https://example.test/playwright.git",
            "tag": "v1.2.3",
            "commit": "0123456789abcdef",
            "suites": [suite],
        },
    )
    manifest_path = root / "playwright-core-manifest.json"
    manifest_path.write_text(json.dumps(manifest, sort_keys=True) + "\n")

    server_harness = root / "playwright-core-server-runtime.cjs"
    candidate_harness = root / "playwright-core-candidate-preload.cjs"
    server_harness.write_text("'use strict';\n")
    candidate_harness.write_text("'use strict';\n")

    candidate = root / "candidate"
    (candidate / "src").mkdir(parents=True)
    (candidate / "package.json").write_text('{"main":"src/index.cjs"}\n')
    (candidate / "src/index.cjs").write_text("module.exports = {};\n")
    artifact = candidate / "native/official/pwright.node"
    artifact.parent.mkdir(parents=True)
    artifact.write_bytes(b"native-test-artifact")

    result = root / "result.json"
    result.write_text('{"ok":true}\n')
    tooling_source_records = None
    if schema2:
        scripts = root / "scripts"
        scripts.mkdir()
        (scripts / "official_result_destination.py").write_text("guard = True\n")
        (scripts / "run_playwright_official_conformance.py").write_text("run = True\n")
        tooling_source_records = _tooling_sources(root)
    command = [
        "node",
        str(runner),
        "test",
        "--config=tests/library/playwright.config.ts",
        "--reporter=line,json",
        "--workers=1",
        "--project=chromium-page",
        "tests/page/example.spec.ts",
    ]
    provenance = _result_provenance(
        mode="candidate",
        manifest=manifest,
        manifest_path=manifest_path,
        suite=suite,
        upstream=upstream,
        project="chromium-page",
        filters=["tests/page/example.spec.ts"],
        grep=None,
        workers=1,
        headed=False,
        endpoint="http://127.0.0.1:9222",
        fixture_bind="100.64.0.1",
        fixture_host="example-yuacx.example.ts.net",
        candidate=candidate,
        candidate_profile="official",
        harness_inputs=[server_harness, candidate_harness],
        command=command,
        result=result,
        exit_code=0,
        tooling_sources=tooling_source_records,
    )
    provenance_path = result.with_suffix(".provenance.json")
    provenance_path.write_text(json.dumps(provenance, sort_keys=True) + "\n")
    return provenance, provenance_path, result, candidate, artifact


def _pinned_full_file_fixture(
    root: Path,
) -> tuple[dict[str, object], Path, Path, Path]:
    provenance, provenance_path, result, _, _ = _fixture(root, schema2=True)
    upstream = root / "upstream"
    source = upstream / "tests/page/example.spec.ts"
    source.parent.mkdir(parents=True)
    source.write_text("test('pinned', () => {});\n")
    manifest_path = Path(cast(str, provenance["manifest"]))
    manifest = json.loads(manifest_path.read_text())
    suite = manifest["suites"][0]
    suite.update(
        {
            "role": "browser-backed-official-file",
            "project": "chromium-page",
            "requiresBrowser": True,
            "args": ["tests/page/example.spec.ts"],
            "expectedFiles": 1,
            "expectedTestsPerProject": 1,
            "upstreamTestSourceSha256": hashlib.sha256(source.read_bytes()).hexdigest(),
            "expectedSpecIdsSha256": hashlib.sha256(b"pinned-id\n").hexdigest(),
        }
    )
    manifest_path.write_text(json.dumps(manifest, sort_keys=True) + "\n")
    result.write_text(
        json.dumps(
            {
                "suites": [
                    {
                        "specs": [
                            {
                                "id": "pinned-id",
                                "file": "page/example.spec.ts",
                                "tests": [
                                    {
                                        "projectId": "chromium-page",
                                        "status": "unexpected",
                                        "results": [{"status": "failed"}],
                                    }
                                ],
                            }
                        ]
                    }
                ],
            }
        )
    )
    provenance["suiteRole"] = suite["role"]
    provenance["manifestSha256"] = hashlib.sha256(
        manifest_path.read_bytes()
    ).hexdigest()
    provenance["resultSha256"] = hashlib.sha256(result.read_bytes()).hexdigest()
    provenance_path.write_text(json.dumps(provenance, sort_keys=True) + "\n")
    return provenance, provenance_path, result, source


def _pinned_one_row_fixture(
    root: Path,
) -> tuple[dict[str, object], Path, Path]:
    provenance, provenance_path, result, _ = _pinned_full_file_fixture(root)
    manifest_path = Path(cast(str, provenance["manifest"]))
    manifest = json.loads(manifest_path.read_text())
    grep = "pinned"
    manifest["suites"][0]["pinnedGrep"] = grep
    manifest_path.write_text(json.dumps(manifest, sort_keys=True) + "\n")
    provenance["manifestSha256"] = hashlib.sha256(
        manifest_path.read_bytes()
    ).hexdigest()
    provenance["grep"] = grep
    command = cast(list[str], provenance["command"])
    command.insert(-1, f"--grep={grep}")
    provenance_path.write_text(json.dumps(provenance, sort_keys=True) + "\n")
    return provenance, provenance_path, result


def _pinned_public_subset_fixture(
    root: Path, *, manifest_name: str, source_name: str
) -> tuple[dict[str, object], Path, Path, Path, str]:
    provenance, provenance_path, result, _, _ = _fixture(root, schema2=True)
    upstream = root / "upstream"
    source = upstream / f"tests/page/{source_name}"
    source.parent.mkdir(parents=True)
    source.write_text("test('public subset', () => {});\n")
    source_digest = hashlib.sha256(source.read_bytes()).hexdigest()
    pinned = json.loads(
        (Path(__file__).parents[1] / "tests/official" / manifest_name).read_text()
    )
    suite = pinned["suites"][0]
    suite["upstreamTestSourceSha256"] = source_digest
    manifest_path = Path(cast(str, provenance["manifest"]))
    manifest = json.loads(manifest_path.read_text())
    manifest["suites"] = [suite]
    manifest_path.write_text(json.dumps(manifest, sort_keys=True) + "\n")
    result.write_text(
        json.dumps(
            {
                "suites": [
                    {
                        "specs": [
                            {
                                "id": spec_id,
                                "file": f"page/{source_name}",
                                "tests": [
                                    {
                                        "projectId": "chromium-page",
                                        "status": "expected",
                                        "results": [{"status": "passed"}],
                                    }
                                ],
                            }
                            for spec_id in suite["expectedSpecIds"]
                        ]
                    }
                ],
            }
        )
    )
    provenance["suite"] = suite["id"]
    provenance["suiteRole"] = suite["role"]
    provenance["filters"] = suite["args"]
    provenance["grepInvert"] = suite["pinnedGrepInvert"]
    provenance["command"] = [
        "node",
        str(upstream / suite["runner"]),
        "test",
        f"--config={suite['config']}",
        "--reporter=line,json",
        "--workers=1",
        "--project=chromium-page",
        f"--grep-invert={suite['pinnedGrepInvert']}",
        *suite["args"],
    ]
    provenance["manifestSha256"] = hashlib.sha256(
        manifest_path.read_bytes()
    ).hexdigest()
    provenance["resultSha256"] = hashlib.sha256(result.read_bytes()).hexdigest()
    provenance_path.write_text(json.dumps(provenance, sort_keys=True) + "\n")
    return provenance, provenance_path, result, source, source_digest


def _pinned_set_content_public_fixture(
    root: Path,
) -> tuple[dict[str, object], Path, Path, Path, str]:
    return _pinned_public_subset_fixture(
        root,
        manifest_name="playwright-core-page-set-content-public-manifest.json",
        source_name="page-set-content.spec.ts",
    )


def _pinned_page_click_public_fixture(
    root: Path,
) -> tuple[dict[str, object], Path, Path, Path, str]:
    return _pinned_public_subset_fixture(
        root,
        manifest_name="playwright-core-page-click-public-manifest.json",
        source_name="page-click.spec.ts",
    )


def _secure_fixture(
    root: Path, *, schema2: bool = False
) -> tuple[dict[str, object], Path, Path]:
    # Arrange
    pinned_manifest = cast(
        Manifest,
        json.loads(
            (
                Path(__file__).parents[1]
                / "tests/official/playwright-core-page-dispatch-event-secure-manifest.json"
            ).read_text()
        ),
    )
    spec_ids = pinned_manifest["suites"][0]["expectedSpecIds"]
    upstream = root / "upstream"
    runner = upstream / "packages/playwright/cli.js"
    test_source = upstream / "tests/page/page-dispatchevent.spec.ts"
    fixture_source = upstream / "tests/config/testserver/index.ts"
    server_fixtures_source = upstream / "tests/config/serverFixtures.ts"
    for source in (runner, test_source, fixture_source, server_fixtures_source):
        source.parent.mkdir(parents=True, exist_ok=True)
        source.write_text(f"source {source.name}\n")
    source_digests = {
        "tests/page/page-dispatchevent.spec.ts": hashlib.sha256(
            test_source.read_bytes()
        ).hexdigest(),
        "tests/config/testserver/index.ts": hashlib.sha256(
            fixture_source.read_bytes()
        ).hexdigest(),
        "tests/config/serverFixtures.ts": hashlib.sha256(
            server_fixtures_source.read_bytes()
        ).hexdigest(),
    }
    suite = cast(
        Suite,
        {
            "id": "library-page-dispatch-event-chromium-secure",
            "role": "browser-backed-official-file",
            "runner": "packages/playwright/cli.js",
            "config": "tests/library/playwright.config.ts",
            "candidateAdapter": "test-mode-playwright-core",
            "oracleAdapter": "test-mode-playwright-core-cdp",
            "project": "chromium-page",
            "requiresBrowser": True,
            "args": ["tests/page/page-dispatchevent.spec.ts"],
            "expectedTestsPerProject": 17,
            "expectedFiles": 1,
            "expectedSpecIds": spec_ids,
            "fixtureMode": "trusted-tailscale-tls-dual-port",
            "upstreamTestSourceSha256": source_digests[
                "tests/page/page-dispatchevent.spec.ts"
            ],
            "upstreamFixtureSourceSha256": source_digests[
                "tests/config/testserver/index.ts"
            ],
            "upstreamServerFixturesSourceSha256": source_digests[
                "tests/config/serverFixtures.ts"
            ],
        },
    )
    manifest = cast(
        Manifest,
        {
            "schema": 1,
            "repository": "https://example.test/playwright.git",
            "tag": "v1.2.3",
            "commit": "0123456789abcdef",
            "suites": [suite],
        },
    )
    manifest_path = root / "secure-manifest.json"
    manifest_path.write_text(json.dumps(manifest, sort_keys=True) + "\n")
    server_harness = root / "playwright-core-server-runtime.cjs"
    secure_harness = root / "playwright-core-secure-server-runtime.cjs"
    oracle_harness = root / "playwright-core-oracle-preload.cjs"
    for harness in (server_harness, secure_harness, oracle_harness):
        harness.write_text("'use strict';\n")
    result = root / "secure-result.json"
    specs = [
        {
            "id": spec_id,
            "file": "page/page-dispatchevent.spec.ts",
            "tests": [{"projectId": "chromium-page"}],
        }
        for spec_id in spec_ids
    ]
    result.write_text(json.dumps({"suites": [{"specs": specs, "suites": []}]}))
    tooling_source_records = None
    if schema2:
        scripts = root / "scripts"
        scripts.mkdir()
        (scripts / "official_result_destination.py").write_text("guard = True\n")
        (scripts / "run_playwright_official_conformance.py").write_text("run = True\n")
        tooling_source_records = _tooling_sources(root)
    command = [
        "node",
        str(runner),
        "test",
        "--config=tests/library/playwright.config.ts",
        "--reporter=line,json",
        "--workers=1",
        "--project=chromium-page",
        "tests/page/page-dispatchevent.spec.ts",
    ]
    provenance = _result_provenance(
        mode="oracle",
        manifest=manifest,
        manifest_path=manifest_path,
        suite=suite,
        upstream=upstream,
        project="chromium-page",
        filters=["tests/page/page-dispatchevent.spec.ts"],
        grep=None,
        workers=1,
        headed=False,
        endpoint="http://127.0.0.1:9222",
        fixture_bind="100.64.0.1",
        fixture_host="example-yuacx.example.ts.net",
        candidate=None,
        candidate_profile=None,
        harness_inputs=[server_harness, secure_harness, oracle_harness],
        command=command,
        result=result,
        exit_code=0,
        secure_fixture_sources=source_digests,
        tooling_sources=tooling_source_records,
    )
    provenance_path = result.with_suffix(".provenance.json")
    provenance_path.write_text(json.dumps(provenance, sort_keys=True) + "\n")
    return provenance, provenance_path, fixture_source


def _source_only_fixture(
    root: Path, *, mode: str = "oracle"
) -> tuple[dict[str, object], Path, Path]:
    repository = Path(__file__).parents[1]
    upstream, manifest_path, suite = _source_only_test_fixture(root)
    runner = upstream / suite["runner"]
    runner.parent.mkdir(parents=True)
    runner.write_text("'use strict';\n")
    manifest = cast(Manifest, json.loads(manifest_path.read_text()))
    source = _verify_source_only_suite(suite, upstream, manifest_path, repository)
    result = root / "source-only-result.json"
    specs = [
        {
            "id": spec_id,
            "file": "library/browsercontext-viewport.spec.ts",
            "tests": [
                {
                    "projectId": project_id,
                    "status": "expected",
                    "results": [{"status": "passed"}],
                }
            ],
        }
        for spec_id, project_id in zip(
            source["expectedSpecIds"], source["expectedProjectIds"], strict=True
        )
    ]
    result.write_text(json.dumps({"suites": [{"specs": specs, "suites": []}]}))
    receipt = result.with_suffix(".guard-receipt.jsonl")
    receipt.write_text(
        "\n".join(
            json.dumps(record)
            for record in [
                {
                    "kind": "runner-no-io",
                    "pid": 101,
                    "threadId": 0,
                    "suite": suite["id"],
                    "case": source["case"],
                },
                {
                    "kind": "worker-no-io",
                    "pid": 102,
                    "threadId": 0,
                    "suite": suite["id"],
                    "case": source["case"],
                },
            ]
        )
        + "\n"
    )
    evidence = {
        "scope": "upstream-private-source-only",
        "candidateRuntimeAttested": False,
        "publicApiParityEvidence": False,
        "source": source,
        "reportedTests": len(specs),
        "guardReceipt": str(receipt),
        "guardReceiptSha256": hashlib.sha256(receipt.read_bytes()).hexdigest(),
        "guardedWorkerProcesses": 1,
    }
    candidate = None
    harness_inputs = [
        repository / "tests/official/playwright-core-internal-source-only-guard.cjs"
    ]
    if mode == "candidate":
        candidate = root / "candidate"
        (candidate / "src").mkdir(parents=True)
        (candidate / "package.json").write_text('{"main":"src/index.cjs"}\n')
        (candidate / "src/index.cjs").write_text("module.exports = {};\n")
        artifact = candidate / "native/official/pwright.node"
        artifact.parent.mkdir(parents=True)
        artifact.write_bytes(b"native-test-artifact")
        harness_inputs.append(
            repository / "tests/official/playwright-core-candidate-preload.cjs"
        )
    provenance = _result_provenance(
        mode=mode,
        manifest=manifest,
        manifest_path=manifest_path,
        suite=suite,
        upstream=upstream,
        project=None,
        filters=suite["args"],
        grep=None,
        workers=1,
        headed=False,
        endpoint="http://127.0.0.1:9222",
        fixture_bind="",
        fixture_host="",
        candidate=candidate,
        candidate_profile="official" if candidate is not None else None,
        harness_inputs=harness_inputs,
        command=[
            "node",
            str(runner),
            "test",
            f"--config={suite['config']}",
            "--reporter=line,json",
            "--workers=1",
            *suite["args"],
        ],
        result=result,
        exit_code=0,
        tooling_sources=_tooling_sources(repository, source_only=True),
        source_only_execution=evidence,
    )
    provenance_path = result.with_suffix(".provenance.json")
    provenance_path.write_text(json.dumps(provenance, sort_keys=True) + "\n")
    return provenance, provenance_path, receipt


class VerifyProvenanceTests(unittest.TestCase):
    def test_schema2_page_click_public_accepts_only_fixed_81_case_selection(
        self,
    ) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, _, digest = (
                _pinned_page_click_public_fixture(root)
            )

            # Act
            with patch(
                "run_playwright_official_conformance._PAGE_CLICK_SOURCE_SHA256", digest
            ):
                verified = verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    check_checkout=False,
                )

            # Assert
            self.assertIn("should not wait with noAutoWaiting", verified["grepInvert"])

    def test_schema2_page_click_public_rejects_changed_filter_id_or_source(
        self,
    ) -> None:
        for change in ("grepInvert", "missingInvert", "id", "source", "excluded"):
            with self.subTest(change=change), TemporaryDirectory() as directory:
                # Arrange
                root = Path(directory)
                provenance, provenance_path, result, source, digest = (
                    _pinned_page_click_public_fixture(root)
                )
                if change == "grepInvert":
                    provenance["grepInvert"] = ".*"
                elif change == "missingInvert":
                    provenance.pop("grepInvert")
                elif change == "id":
                    report = json.loads(result.read_text())
                    report["suites"][0]["specs"][0]["id"] = "substituted-id"
                    result.write_text(json.dumps(report))
                    provenance["resultSha256"] = hashlib.sha256(
                        result.read_bytes()
                    ).hexdigest()
                elif change == "source":
                    source.write_text("test('changed', () => {});\n")
                else:
                    manifest_path = Path(cast(str, provenance["manifest"]))
                    manifest = json.loads(manifest_path.read_text())
                    manifest["suites"][0]["excludedSpecIds"] = []
                    manifest_path.write_text(
                        json.dumps(manifest, sort_keys=True) + "\n"
                    )
                    provenance["manifestSha256"] = hashlib.sha256(
                        manifest_path.read_bytes()
                    ).hexdigest()

                # Act and Assert
                with (
                    patch(
                        "run_playwright_official_conformance._PAGE_CLICK_SOURCE_SHA256",
                        digest,
                    ),
                    self.assertRaises(ProvenanceValidationError),
                ):
                    verify_provenance(
                        provenance,
                        provenance_path=provenance_path,
                        tooling_root=root,
                        check_checkout=False,
                    )

    def test_schema2_set_content_public_accepts_only_fixed_15_case_selection(
        self,
    ) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, _, digest = (
                _pinned_set_content_public_fixture(root)
            )

            # Act
            with patch(
                "run_playwright_official_conformance._SET_CONTENT_SOURCE_SHA256", digest
            ):
                verified = verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    check_checkout=False,
                )

            # Assert
            self.assertEqual(
                verified["grepInvert"], "should handle timeout properly(?: 2)?$"
            )

    def test_schema2_set_content_public_rejects_changed_filter_id_or_source(
        self,
    ) -> None:
        for change in ("grepInvert", "missingInvert", "id", "source"):
            with self.subTest(change=change), TemporaryDirectory() as directory:
                # Arrange
                root = Path(directory)
                provenance, provenance_path, result, source, digest = (
                    _pinned_set_content_public_fixture(root)
                )
                if change == "grepInvert":
                    provenance["grepInvert"] = ".*"
                elif change == "missingInvert":
                    provenance.pop("grepInvert")
                elif change == "id":
                    report = json.loads(result.read_text())
                    report["suites"][0]["specs"][0]["id"] = "substituted-id"
                    result.write_text(json.dumps(report))
                    provenance["resultSha256"] = hashlib.sha256(
                        result.read_bytes()
                    ).hexdigest()
                else:
                    source.write_text("test('changed', () => {});\n")

                # Act and Assert
                with (
                    patch(
                        "run_playwright_official_conformance._SET_CONTENT_SOURCE_SHA256",
                        digest,
                    ),
                    self.assertRaises(ProvenanceValidationError),
                ):
                    verify_provenance(
                        provenance,
                        provenance_path=provenance_path,
                        tooling_root=root,
                        check_checkout=False,
                    )

    def test_schema2_pinned_one_row_accepts_only_manifest_owned_grep(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _ = _pinned_one_row_fixture(root)

            # Act
            verified = verify_provenance(
                provenance,
                provenance_path=provenance_path,
                tooling_root=root,
                check_checkout=False,
            )

            # Assert
            self.assertEqual(verified["grep"], "pinned")
            provenance["grep"] = "other"
            with self.assertRaisesRegex(ProvenanceValidationError, "selection differs"):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    check_checkout=False,
                )

    def test_schema2_pinned_full_file_verifies_exact_selection_and_source(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, _ = _pinned_full_file_fixture(root)

            # Act
            verified = verify_provenance(
                provenance,
                provenance_path=provenance_path,
                tooling_root=root,
                check_checkout=False,
            )

            # Assert
            self.assertEqual(verified["resultSha256"], provenance["resultSha256"])

    def test_schema2_pinned_full_file_rejects_valid_digest_with_wrong_id(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, result, _ = _pinned_full_file_fixture(root)
            report = json.loads(result.read_text())
            report["suites"][0]["specs"][0]["id"] = "substituted-id"
            result.write_text(json.dumps(report))
            provenance["resultSha256"] = hashlib.sha256(result.read_bytes()).hexdigest()

            # Act and Assert
            with self.assertRaisesRegex(ProvenanceValidationError, "expected 1 unique"):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    check_checkout=False,
                )

    def test_schema2_pinned_full_file_rejects_valid_digest_without_execution(
        self,
    ) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, result, _ = _pinned_full_file_fixture(root)
            report = json.loads(result.read_text())
            test = report["suites"][0]["specs"][0]["tests"][0]
            test["results"] = []
            result.write_text(json.dumps(report))
            provenance["resultSha256"] = hashlib.sha256(result.read_bytes()).hexdigest()

            # Act and Assert
            with self.assertRaisesRegex(ProvenanceValidationError, "executed attempt"):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    check_checkout=False,
                )

    def test_schema2_pinned_full_file_rejects_live_source_drift(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, source = _pinned_full_file_fixture(root)
            source.write_text("test('changed', () => {});\n")

            # Act and Assert
            with self.assertRaisesRegex(ProvenanceValidationError, "source drifted"):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    check_checkout=False,
                )

    def test_schema2_pinned_full_file_rejects_narrowed_filters(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, _ = _pinned_full_file_fixture(root)
            provenance["grep"] = "pinned"

            # Act and Assert
            with self.assertRaisesRegex(ProvenanceValidationError, "selection differs"):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    check_checkout=False,
                )

    def test_schema3_source_only_candidate_verifies_without_public_parity(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            provenance, provenance_path, _ = _source_only_fixture(
                Path(directory), mode="candidate"
            )

            # Act
            verified = verify_provenance(
                provenance,
                provenance_path=provenance_path,
                check_checkout=False,
            )

            # Assert
            self.assertEqual(verified["mode"], "candidate")
            self.assertFalse(
                verified["sourceOnlyExecution"]["candidateRuntimeAttested"]
            )

    def test_schema3_source_only_oracle_verifies_bounded_evidence(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            provenance, provenance_path, _ = _source_only_fixture(Path(directory))

            # Act
            verified = verify_provenance(
                provenance,
                provenance_path=provenance_path,
                check_checkout=False,
            )

            # Assert
            self.assertEqual(verified["schema"], 3)
            self.assertFalse(verified["sourceOnlyExecution"]["publicApiParityEvidence"])

    def test_schema3_rejects_forged_public_api_parity_claim(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            provenance, provenance_path, _ = _source_only_fixture(Path(directory))
            provenance["sourceOnlyExecution"]["publicApiParityEvidence"] = True

            # Act and Assert
            with self.assertRaisesRegex(
                ProvenanceValidationError, "bounded source-only run"
            ):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    check_checkout=False,
                )

    def test_schema3_rejects_forged_candidate_runtime_claim(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            provenance, provenance_path, _ = _source_only_fixture(
                Path(directory), mode="candidate"
            )
            provenance["sourceOnlyExecution"]["candidateRuntimeAttested"] = True

            # Act and Assert
            with self.assertRaisesRegex(
                ProvenanceValidationError, "bounded source-only run"
            ):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    check_checkout=False,
                )

    def test_schema3_rejects_guard_receipt_without_worker(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            provenance, provenance_path, receipt = _source_only_fixture(Path(directory))
            receipt.write_text(receipt.read_text().splitlines()[0] + "\n")
            provenance["sourceOnlyExecution"]["guardReceiptSha256"] = hashlib.sha256(
                receipt.read_bytes()
            ).hexdigest()

            # Act and Assert
            with self.assertRaisesRegex(ProvenanceValidationError, "never activated"):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    check_checkout=False,
                )

    def test_schema2_candidate_verifies_tooling_in_archived_root(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, _, _ = _fixture(root, schema2=True)

            # Act
            verified = verify_provenance(
                provenance,
                provenance_path=provenance_path,
                tooling_root=root,
                check_checkout=False,
            )

            # Assert
            self.assertEqual(verified["schema"], 2)
            self.assertEqual(len(verified["toolingSources"]), 2)

    def test_schema2_secure_oracle_verifies_tooling(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _ = _secure_fixture(root, schema2=True)

            # Act
            verified = verify_provenance(
                provenance,
                provenance_path=provenance_path,
                tooling_root=root,
                check_checkout=False,
            )

            # Assert
            self.assertEqual(verified["schema"], 2)
            self.assertEqual(verified["secureFixture"]["reportedTests"], 17)

    def test_schema2_rejects_tooling_drift_even_without_checkout_check(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, _, _ = _fixture(root, schema2=True)
            (root / "scripts/run_playwright_official_conformance.py").write_text(
                "changed\n"
            )

            # Act and Assert
            with self.assertRaisesRegex(
                ProvenanceValidationError, "toolingSources.*SHA-256 mismatch"
            ):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    check_checkout=False,
                )

    def test_schema2_rejects_missing_or_reordered_tooling_records(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, _, _ = _fixture(root, schema2=True)
            original = copy.deepcopy(provenance["toolingSources"])

            # Act and Assert
            for invalid in (
                original[:1],
                list(reversed(original)),
                original + [original[0]],
            ):
                with self.subTest(invalid=invalid):
                    provenance["toolingSources"] = invalid
                    with self.assertRaisesRegex(
                        ProvenanceValidationError, "toolingSources"
                    ):
                        verify_provenance(
                            provenance,
                            provenance_path=provenance_path,
                            tooling_root=root,
                            check_checkout=False,
                        )

    def test_schema2_rejects_invalid_digest_and_missing_source(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, _, _ = _fixture(root, schema2=True)
            records = cast(list[dict[str, str]], provenance["toolingSources"])
            records[0]["sha256"] = "ABC"

            # Act and Assert
            with self.assertRaisesRegex(ProvenanceValidationError, "sha256 is invalid"):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    check_checkout=False,
                )
            records[0]["sha256"] = _tooling_sources(root)[0]["sha256"]
            (root / records[0]["path"]).unlink()
            with self.assertRaisesRegex(
                ProvenanceValidationError, "missing or a symlink"
            ):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    check_checkout=False,
                )

    def test_schema1_rejects_unversioned_tooling_field(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            provenance, provenance_path, _, _, _ = _fixture(Path(directory))
            provenance["toolingSources"] = []

            # Act and Assert
            with self.assertRaisesRegex(ProvenanceValidationError, "fields differ"):
                verify_provenance(
                    provenance, provenance_path=provenance_path, check_checkout=False
                )

    def test_secure_sidecar_rejects_duplicate_spec_despite_matching_digest(
        self,
    ) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            provenance, provenance_path, _ = _secure_fixture(Path(directory))
            result = Path(cast(str, provenance["result"]))
            report = json.loads(result.read_text())
            specs = report["suites"][0]["specs"]
            specs[-1] = specs[0].copy()
            result.write_text(json.dumps(report))
            provenance["resultSha256"] = hashlib.sha256(result.read_bytes()).hexdigest()

            # Act and Assert
            with self.assertRaisesRegex(ProvenanceValidationError, "17 unique pinned"):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    check_checkout=False,
                )

    def test_generated_candidate_sidecar_verifies(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, _, _ = _fixture(root)

            # Act
            verified = verify_provenance(
                provenance,
                provenance_path=provenance_path,
                check_checkout=False,
            )

            # Assert
            self.assertEqual(verified["mode"], "candidate")
            self.assertEqual(verified["suite"], "library")

    def test_result_tampering_is_rejected(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, result, _, _ = _fixture(root)
            result.write_text('{"ok":false}\n')

            # Act and Assert
            with self.assertRaisesRegex(
                ProvenanceValidationError, "result SHA-256 mismatch"
            ):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    check_checkout=False,
                )

    def test_adapter_drift_is_rejected(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, _, _ = _fixture(root)
            provenance["adapter"] = "official-implementation"

            # Act and Assert
            with self.assertRaisesRegex(ProvenanceValidationError, "adapter differs"):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    check_checkout=False,
                )

    def test_candidate_artifact_tampering_is_rejected(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, _, artifact = _fixture(root)
            artifact.write_bytes(b"different-native-artifact")

            # Act and Assert
            with self.assertRaisesRegex(
                ProvenanceValidationError, "candidate artifact SHA-256 mismatch"
            ):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    check_checkout=False,
                )

    def test_secure_oracle_sidecar_verifies_exact_sources_and_selection(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            provenance, provenance_path, _ = _secure_fixture(Path(directory))

            # Act
            verified = verify_provenance(
                provenance,
                provenance_path=provenance_path,
                check_checkout=False,
            )

            # Assert
            self.assertEqual(verified["secureFixture"]["reportedTests"], 17)

    def test_secure_sidecar_rejects_upstream_fixture_source_drift(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            provenance, provenance_path, fixture_source = _secure_fixture(
                Path(directory)
            )
            fixture_source.write_text("drifted fixture\n")

            # Act and Assert
            with self.assertRaisesRegex(
                ProvenanceValidationError, "pinned source drifted"
            ):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    check_checkout=False,
                )

    def test_secure_sidecar_rejects_incorrect_cross_origin_port_offset(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            provenance, provenance_path, _ = _secure_fixture(Path(directory))
            provenance["secureFixture"]["crossProcessPortOffset"] = 0

            # Act and Assert
            with self.assertRaisesRegex(
                ProvenanceValidationError, "secureFixture evidence differs"
            ):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    check_checkout=False,
                )


if __name__ == "__main__":
    unittest.main()
