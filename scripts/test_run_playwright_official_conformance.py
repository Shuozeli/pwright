#!/usr/bin/env python3
"""Tests for official Core runner profile selection."""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
import shutil
import subprocess
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from typing import cast
from unittest.mock import patch

from run_playwright_official_conformance import (
    Manifest,
    Suite,
    _cross_origin_forward_enabled,
    _install_secure_certificate,
    _pinned_full_file_source,
    _pinned_grep,
    _pinned_grep_invert,
    _positive_int,
    _require_tooling_unchanged,
    _resolve_candidate_profile,
    _result_provenance,
    _secure_fixture_source_digests,
    _secure_spec_ids,
    _tooling_sources,
    _verify_pinned_full_file_result_selection,
    _verify_secure_result_selection,
    _verify_source_only_guard_receipt,
    _verify_source_only_result_selection,
    _verify_source_only_suite,
)
from run_playwright_official_conformance import (
    main as run_official_main,
)
from run_playwright_official_image_tools_source_only import _verify_result_selection


def _secure_suite() -> Suite:
    manifest = cast(
        Manifest,
        json.loads(
            (
                Path(__file__).parents[1]
                / "tests/official/playwright-core-page-dispatch-event-secure-manifest.json"
            ).read_text()
        ),
    )
    return manifest["suites"][0]


def _secure_specs(suite: Suite) -> list[dict[str, object]]:
    return [
        {
            "id": spec_id,
            "file": "page/page-dispatchevent.spec.ts",
            "tests": [{"projectId": "chromium-page"}],
        }
        for spec_id in suite["expectedSpecIds"]
    ]


def _source_only_test_fixture(directory: Path) -> tuple[Path, Path, Suite]:
    root = Path(__file__).parents[1]
    typescript = root / "packages/pwright/node_modules/typescript"
    if not typescript.is_dir():
        raise unittest.SkipTest("pnpm TypeScript dependency is not installed")
    upstream = directory / "upstream"
    node_modules = upstream / "node_modules"
    node_modules.mkdir(parents=True)
    (node_modules / "typescript").symlink_to(
        typescript.resolve(), target_is_directory=True
    )
    source = upstream / "tests/library/browsercontext-viewport.spec.ts"
    source.parent.mkdir(parents=True)
    source.write_text(
        "\n" * 21
        + "import { deviceDescriptors } from 'packages/playwright-core/lib/server/deviceDescriptors';\n"
        + "\n" * 27
        + "it('landscape viewport should have width larger than height', async () => {\n"
        + "  if (!deviceDescriptors) throw new Error('missing');\n"
        + "});\n"
    )
    original = cast(
        Manifest,
        json.loads(
            (
                root
                / "tests/official/playwright-core-internal-source-only-manifest.json"
            ).read_text()
        ),
    )
    suite = original["suites"][0]
    suite["upstreamTestSourceSha256"] = hashlib.sha256(source.read_bytes()).hexdigest()
    original["suites"] = [suite]
    manifest = directory / "manifest.json"
    manifest.write_text(json.dumps(original) + "\n")
    return upstream, manifest, suite


def _image_tools_source_fixture(directory: Path) -> tuple[Path, Path, Suite]:
    root = Path(__file__).parents[1]
    checkout = Path(
        os.environ.get(
            "PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM",
            "/Users/cyuan/projects/thirdparty/playwright-v1.58.0",
        )
    )
    if not (checkout / "tests/image_tools/unit.spec.ts").is_file():
        raise unittest.SkipTest("pinned Playwright v1.58.0 checkout is unavailable")
    typescript = checkout / "node_modules/typescript"
    if not typescript.is_dir():
        raise unittest.SkipTest("upstream TypeScript dependency is unavailable")
    manifest = root / "tests/official/playwright-core-internal-source-only-image-tools-v1-manifest.json"
    suite = cast(Manifest, json.loads(manifest.read_text()))["suites"][0]
    upstream = directory / "upstream"
    source_files = [
        *suite["args"],
        "tests/image_tools/utils.ts",
        "tests/playwright-test/playwright.config.ts",
        "tests/playwright-test/stable-test-runner/index.js",
        "tests/playwright-test/stable-test-runner/package-lock.json",
    ]
    for relative in source_files:
        destination = upstream / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(checkout / relative, destination)
    shutil.copytree(
        checkout / "tests/image_tools/fixtures",
        upstream / "tests/image_tools/fixtures",
    )
    (upstream / "node_modules").mkdir()
    (upstream / "node_modules/typescript").symlink_to(
        typescript.resolve(), target_is_directory=True
    )
    return upstream, manifest, suite


class ResolveCandidateProfileTests(unittest.TestCase):
    def test_source_only_image_tools_guard_pins_png_inputs_and_fixture_scope(self) -> None:
        # Arrange
        root = Path(__file__).parents[1]
        guard = root / "tests/official/playwright-core-image-tools-source-only-guard.cjs"
        with TemporaryDirectory() as directory:
            temporary = Path(directory)
            upstream, manifest, suite = _image_tools_source_fixture(temporary)
            command = [
                "node", str(guard), "--check", str(manifest), suite["id"], str(upstream)
            ]
            env = os.environ.copy()
            for key in ("IMAGE_TOOLS_FIXTURES", "GITHUB_ACTIONS", "GITLAB_CI", "JENKINS_URL", "BUILD_URL"):
                env.pop(key, None)

            # Act
            checked = subprocess.run(command, env=env, capture_output=True, text=True, check=False)

            # Assert
            self.assertEqual(checked.returncode, 0, checked.stderr)
            source = json.loads(checked.stdout)
            self.assertEqual(source["case"], "private-image-tools-batch-v1")
            self.assertEqual(source["expectedTestsPerProject"], 25)
            self.assertEqual(source["sourceSha256"], suite["upstreamSourceSetSha256"])
            self.assertEqual(source["expectedInvocationIdsSha256"], suite["expectedInvocationIdsSha256"])
            self.assertEqual(source["expectedFileSpecIdsSha256"], suite["expectedFileSpecIdsSha256"])

            png = next((upstream / "tests/image_tools/fixtures").rglob("*-actual.png"))
            png.write_bytes(png.read_bytes() + b"drift")
            drifted = subprocess.run(command, env=env, capture_output=True, text=True, check=False)
            self.assertNotEqual(drifted.returncode, 0)
            self.assertIn("pinned source or PNG SHA-256 mismatch", drifted.stderr)

            env["IMAGE_TOOLS_FIXTURES"] = str(temporary / "external")
            custom = subprocess.run(command, env=env, capture_output=True, text=True, check=False)
            self.assertNotEqual(custom.returncode, 0)
            self.assertIn("forbids custom IMAGE_TOOLS_FIXTURES", custom.stderr)
            env.pop("IMAGE_TOOLS_FIXTURES")

            dotenv = upstream / ".env"
            dotenv.write_text("IMAGE_TOOLS_FIXTURES=/unapproved\n")
            overridden = subprocess.run(command, env=env, capture_output=True, text=True, check=False)
            self.assertNotEqual(overridden.returncode, 0)
            self.assertIn("forbids upstream .env overrides", overridden.stderr)
            dotenv.unlink()

            env["GITHUB_ACTIONS"] = "true"
            ci = subprocess.run(command, env=env, capture_output=True, text=True, check=False)
            self.assertNotEqual(ci.returncode, 0)
            self.assertIn("forbids CI Git metadata subprocesses", ci.stderr)
            env.pop("GITHUB_ACTIONS")

            fixture_directory = upstream / "tests/image_tools/fixtures/should-match"
            relocated = temporary / "relocated-fixtures"
            fixture_directory.rename(relocated)
            fixture_directory.symlink_to(relocated, target_is_directory=True)
            symlinked = subprocess.run(command, env=env, capture_output=True, text=True, check=False)
            self.assertNotEqual(symlinked.returncode, 0)
            self.assertIn("forbids symlink fixtures", symlinked.stderr)

    def test_source_only_image_tools_guard_blocks_process_network_and_candidate_load(self) -> None:
        # Arrange
        root = Path(__file__).parents[1]
        guard = root / "tests/official/playwright-core-image-tools-source-only-guard.cjs"
        with TemporaryDirectory() as directory:
            temporary = Path(directory)
            upstream, manifest, suite = _image_tools_source_fixture(temporary)
            candidate = temporary / "candidate"
            candidate.mkdir()
            (candidate / "package.json").write_text('{"main":"index.js"}\n')
            (candidate / "index.js").write_text('module.exports = { loaded: true };\n')
            receipt = temporary / "guard.jsonl"
            env = os.environ.copy()
            for key in ("IMAGE_TOOLS_FIXTURES", "GITHUB_ACTIONS", "GITLAB_CI", "JENKINS_URL", "BUILD_URL"):
                env.pop(key, None)
            env.update({
                "NODE_OPTIONS": f"--require={guard}",
                "PWRIGHT_CORE_SOURCE_ONLY_ACTIVE": "1",
                "PWRIGHT_CORE_SOURCE_ONLY_TEST_WORKER": "1",
                "PWRIGHT_CORE_SOURCE_ONLY_MANIFEST": str(manifest),
                "PWRIGHT_CORE_SOURCE_ONLY_SUITE": suite["id"],
                "PWRIGHT_CORE_SOURCE_ONLY_GUARD_RECEIPT": str(receipt),
                "PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM": str(upstream),
            })
            script = """
const assert = require('node:assert/strict');
const child = require('node:child_process');
const net = require('node:net');
assert.throws(() => child.spawn('forbidden'), /forbids child_process.spawn/);
assert.throws(() => child.fork('forbidden'), /forbids child_process.fork/);
assert.throws(() => net.connect(1234), /forbids net connect/);
assert.throws(() => net.createServer().listen(1234), /forbids socket listen/);
assert.throws(() => fetch('http://127.0.0.1/'), /forbids fetch/);
"""

            # Act
            completed = subprocess.run(
                ["node", "-e", script], env=env, cwd=root,
                capture_output=True, text=True, check=False,
            )

            # Assert
            self.assertEqual(completed.returncode, 0, completed.stderr)
            self.assertEqual(json.loads(receipt.read_text())["kind"], "worker-no-io")
            env["PWRIGHT_NODE_CANDIDATE"] = str(candidate)
            blocked = subprocess.run(
                ["node", "-e", "require(process.env.PWRIGHT_NODE_CANDIDATE)"],
                env=env, cwd=root, capture_output=True, text=True, check=False,
            )
            self.assertNotEqual(blocked.returncode, 0)
            self.assertIn("forbids loading the Pwright candidate runtime", blocked.stderr)

    def test_source_only_image_tools_result_requires_exact_25_ids_and_attempts(self) -> None:
        # Arrange
        pairs = [(f"{index:020x}-{index + 100:020x}", "image_tools") for index in range(25)]
        digest = hashlib.sha256(
            "".join(
                f"playwright-test\t{spec_id}\t{project_id}\n"
                for spec_id, project_id in sorted(pairs)
            ).encode()
        ).hexdigest()
        file_spec_digest = hashlib.sha256(
            "".join(
                f"tests/image_tools/{'fixtures' if index < 21 else 'unit'}.spec.ts\t{spec_id}\n"
                for index, (spec_id, _) in enumerate(pairs)
            ).encode()
        ).hexdigest()
        source = {
            "case": "private-image-tools-batch-v1",
            "sourceFiles": ["tests/image_tools/fixtures.spec.ts", "tests/image_tools/unit.spec.ts"],
            "sourceSha256": "0" * 64,
            "expectedInvocationIdsSha256": digest,
            "expectedFileSpecIdsSha256": file_spec_digest,
            "expectedProjectIds": ["image_tools"],
            "expectedTestsPerProject": 25,
        }
        specs = [
            {
                "id": spec_id,
                "file": "../image_tools/fixtures.spec.ts" if index < 21 else "../image_tools/unit.spec.ts",
                "tests": [{
                    "projectId": project_id,
                    "expectedStatus": "passed",
                    "status": "unexpected" if index == 0 else "expected",
                    "results": [{"status": "failed" if index == 0 else "passed"}],
                }],
            }
            for index, (spec_id, project_id) in enumerate(pairs)
        ]
        with TemporaryDirectory() as directory:
            result = Path(directory) / "result.json"
            result.write_text(json.dumps({"suites": [{"specs": specs}]}))

            # Act
            count = _verify_result_selection(result, source)

            # Assert
            self.assertEqual(count, 25)
            specs[0]["tests"][0]["results"] = []
            result.write_text(json.dumps({"suites": [{"specs": specs}]}))
            with self.assertRaisesRegex(ValueError, "no valid attempt"):
                _verify_result_selection(result, source)
            specs[0]["tests"][0]["results"] = [{"status": "failed"}]
            specs[0]["id"] = "f" * 20 + "-" + "f" * 20
            result.write_text(json.dumps({"suites": [{"specs": specs}]}))
            with self.assertRaisesRegex(ValueError, "changed exact invocation IDs"):
                _verify_result_selection(result, source)

    def test_source_only_batch_preserves_failure_and_rejects_missing_attempt(self) -> None:
        # Arrange
        projects = ["chromium-library", "firefox-library", "webkit-library"]
        files = [f"tests/library/pinned-{index}.spec.ts" for index in range(26)]
        pairs: list[tuple[str, str]] = []
        specs: list[dict[str, object]] = []
        for project_index, project in enumerate(projects):
            for case_index in range(284):
                spec_id = f"{case_index:020x}-{project_index:020x}"
                pairs.append((spec_id, project))
                status = "failed" if (project_index, case_index) == (0, 0) else "passed"
                specs.append({
                    "id": spec_id,
                    "file": files[case_index % 26].removeprefix("tests/"),
                    "tests": [{
                        "projectId": project,
                        "expectedStatus": "passed",
                        "status": "unexpected" if status == "failed" else "expected",
                        "results": [{"status": status}],
                    }],
                })
        digest = hashlib.sha256(
            "".join(
                f"library\t{spec_id}\t{project}\n"
                for spec_id, project in sorted(pairs)
            ).encode()
        ).hexdigest()
        source = {
            "case": "private-internal-batch-v1",
            "sourceFiles": files,
            "sourceSha256": "0" * 64,
            "expectedInvocationIdsSha256": digest,
            "expectedProjectIds": projects,
            "expectedTestsPerProject": 284,
        }
        suite = cast(Suite, {"config": "tests/library/playwright.config.ts"})
        with TemporaryDirectory() as directory:
            result = Path(directory) / "result.json"
            result.write_text(json.dumps({"suites": [{"specs": specs}]}))

            # Act
            count = _verify_source_only_result_selection(result, suite, source)

            # Assert
            self.assertEqual(count, 852)
            specs[0]["tests"][0]["results"] = []
            result.write_text(json.dumps({"suites": [{"specs": specs}]}))
            with self.assertRaisesRegex(ValueError, "missing or malformed attempt"):
                _verify_source_only_result_selection(result, suite, source)

    def test_source_only_worker_preload_blocks_spawn_connect_listen_and_fetch(
        self,
    ) -> None:
        # Arrange
        root = Path(__file__).parents[1]
        guard = root / "tests/official/playwright-core-internal-source-only-guard.cjs"
        with TemporaryDirectory() as directory:
            temporary = Path(directory)
            upstream, manifest, _ = _source_only_test_fixture(temporary)
            receipt = temporary / "guard.jsonl"
            env = os.environ.copy()
            env.update(
                {
                    "NODE_OPTIONS": f"--require={guard}",
                    "PWRIGHT_CORE_SOURCE_ONLY_ACTIVE": "1",
                    "PWRIGHT_CORE_SOURCE_ONLY_TEST_WORKER": "1",
                    "PWRIGHT_CORE_SOURCE_ONLY_MANIFEST": str(manifest),
                    "PWRIGHT_CORE_SOURCE_ONLY_SUITE": "core455-internal-viewport-library",
                    "PWRIGHT_CORE_SOURCE_ONLY_GUARD_RECEIPT": str(receipt),
                    "PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM": str(upstream),
                }
            )
            script = """
const assert = require('node:assert/strict');
const child = require('node:child_process');
const net = require('node:net');
assert.throws(() => child.spawn('forbidden'), /forbids child_process.spawn/);
assert.throws(() => child.fork('forbidden'), /forbids child_process.fork/);
assert.throws(() => net.connect(1234), /forbids net connect/);
assert.throws(() => net.createServer().listen(1234), /forbids socket listen/);
assert.throws(() => fetch('http://127.0.0.1/'), /forbids fetch/);
"""

            # Act
            completed = subprocess.run(
                ["node", "-e", script],
                env=env,
                cwd=root,
                capture_output=True,
                text=True,
                check=False,
            )

            # Assert
            self.assertEqual(completed.returncode, 0, completed.stderr)
            self.assertEqual(json.loads(receipt.read_text())["kind"], "worker-no-io")
            parent = subprocess.run(
                ["node", "-e", "process.stdout.write('parent-unmodified')"],
                cwd=root,
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(parent.stdout, "parent-unmodified")

    def test_source_only_result_requires_exact_passing_spec_project_pairs(self) -> None:
        # Arrange
        root = Path(__file__).parents[1]
        with TemporaryDirectory() as directory:
            temporary = Path(directory)
            upstream, manifest_path, suite = _source_only_test_fixture(temporary)
            source = _verify_source_only_suite(suite, upstream, manifest_path, root)
            result = temporary / "result.json"
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

            # Act
            count = _verify_source_only_result_selection(result, suite, source)

            # Assert
            self.assertEqual(count, 3)
            specs[0]["tests"][0]["projectId"] = "android-page"
            result.write_text(json.dumps({"suites": [{"specs": specs}]}))
            with self.assertRaisesRegex(ValueError, "unpinned spec or project"):
                _verify_source_only_result_selection(result, suite, source)

    def test_source_only_receipt_requires_runner_and_worker_guard(self) -> None:
        # Arrange
        root = Path(__file__).parents[1]
        with TemporaryDirectory() as directory:
            temporary = Path(directory)
            upstream, manifest_path, suite = _source_only_test_fixture(temporary)
            source = _verify_source_only_suite(suite, upstream, manifest_path, root)
            receipt = temporary / "receipt.jsonl"
            records = [
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
            receipt.write_text(
                "\n".join(json.dumps(record) for record in records) + "\n"
            )

            # Act
            worker_count = _verify_source_only_guard_receipt(receipt, suite, source)

            # Assert
            self.assertEqual(worker_count, 1)
            receipt.write_text(json.dumps(records[0]) + "\n")
            with self.assertRaisesRegex(ValueError, "never activated"):
                _verify_source_only_guard_receipt(receipt, suite, source)

    def test_tooling_snapshot_rejects_midrun_source_drift(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            scripts = root / "scripts"
            scripts.mkdir()
            destination = scripts / "official_result_destination.py"
            runner = scripts / "run_playwright_official_conformance.py"
            destination.write_text("guard = True\n")
            runner.write_text("run = True\n")
            snapshot = _tooling_sources(root)

            # Act
            runner.write_text("run = False\n")

            # Assert
            with self.assertRaisesRegex(ValueError, "changed during the run"):
                _require_tooling_unchanged(root, snapshot)

    def test_tooling_snapshot_rejects_symlink_source(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            scripts = root / "scripts"
            scripts.mkdir()
            (scripts / "official_result_destination.py").write_text("guard = True\n")
            (scripts / "run_playwright_official_conformance.py").symlink_to(
                scripts / "official_result_destination.py"
            )

            # Act and Assert
            with self.assertRaisesRegex(ValueError, "symlink"):
                _tooling_sources(root)

    def test_worker_count_rejects_zero(self) -> None:
        # Arrange
        raw = "0"

        # Act and Assert
        with self.assertRaisesRegex(argparse.ArgumentTypeError, "positive integer"):
            _positive_int(raw)

    def test_explicit_profile_wins_over_environment(self) -> None:
        # Arrange
        environment = {"PWRIGHT_NATIVE_PROFILE": "default"}

        # Act
        selected = _resolve_candidate_profile(
            "candidate", False, "official-conformance-page-bindings", environment
        )

        # Assert
        self.assertEqual(selected, "official-conformance-page-bindings")

    def test_environment_profile_is_accepted(self) -> None:
        # Arrange
        environment = {"PWRIGHT_NATIVE_PROFILE": "api-request"}

        # Act
        selected = _resolve_candidate_profile("candidate", False, None, environment)

        # Assert
        self.assertEqual(selected, "api-request")

    def test_missing_candidate_profile_fails_closed(self) -> None:
        # Arrange
        environment: dict[str, str] = {}

        # Act and Assert
        with self.assertRaisesRegex(ValueError, "candidate execution requires"):
            _resolve_candidate_profile("candidate", False, None, environment)

    def test_oracle_does_not_select_candidate_profile(self) -> None:
        # Arrange
        environment = {"PWRIGHT_NATIVE_PROFILE": "default"}

        # Act
        selected = _resolve_candidate_profile("oracle", False, None, environment)

        # Assert
        self.assertIsNone(selected)

    def test_candidate_profile_rejects_path_injection(self) -> None:
        # Arrange
        environment: dict[str, str] = {}

        # Act and Assert
        with self.assertRaisesRegex(ValueError, "lowercase hyphen-separated"):
            _resolve_candidate_profile("candidate", False, "../../foreign", environment)

    def test_result_provenance_records_profile_and_report_digest(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            result = root / "result.json"
            result.write_text('{"ok":true}\n')
            manifest_path = root / "manifest.json"
            harness = root / "preload.cjs"
            harness.write_text("'use strict';\n")
            candidate = root / "candidate"
            (candidate / "src").mkdir(parents=True)
            (candidate / "package.json").write_text('{"main":"src/index.cjs"}\n')
            (candidate / "src/index.cjs").write_text("module.exports = {};\n")
            artifact = candidate / "native/official/pwright.node"
            artifact.parent.mkdir(parents=True)
            artifact.write_bytes(b"native-test-artifact")
            manifest = cast(
                Manifest,
                {
                    "schema": 1,
                    "repository": "https://example.test/playwright.git",
                    "tag": "v1.2.3",
                    "commit": "0123456789abcdef",
                    "suites": [],
                },
            )
            manifest_path.write_text(json.dumps(manifest) + "\n")
            suite = cast(
                Suite,
                {
                    "id": "library",
                    "role": "official-entrypoint",
                    "runner": "cli.js",
                    "candidateAdapter": "candidate-adapter",
                },
            )
            scripts = root / "scripts"
            scripts.mkdir()
            (scripts / "official_result_destination.py").write_text("guard = True\n")
            (scripts / "run_playwright_official_conformance.py").write_text(
                "run = True\n"
            )
            tooling_source_records = _tooling_sources(root)

            # Act
            provenance = _result_provenance(
                mode="candidate",
                manifest=manifest,
                manifest_path=manifest_path,
                suite=suite,
                upstream=root,
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
                harness_inputs=[harness],
                command=["node", "cli.js", "test"],
                result=result,
                exit_code=0,
                tooling_sources=tooling_source_records,
            )

            # Assert
            self.assertEqual(provenance["candidateProfile"], "official")
            self.assertEqual(provenance["schema"], 2)
            self.assertEqual(provenance["toolingSources"], tooling_source_records)
            self.assertEqual(provenance["adapter"], "candidate-adapter")
            self.assertEqual(provenance["candidateArtifact"], str(artifact))
            self.assertEqual(provenance["manifest"], str(manifest_path))
            self.assertEqual(
                provenance["harnessInputs"],
                [
                    {
                        "path": str(harness),
                        "sha256": (
                            "e65f3cca9a4637c34593c5b4dc5698d6368b91ce43f6d26273a70c430a66b71f"
                        ),
                    }
                ],
            )
            self.assertEqual(
                provenance["resultSha256"],
                "e5f1eb4d806641698a35efe20e098efd20d7d57a9b90ee69079d5bb650920726",
            )

    def test_secure_fixture_requires_unchanged_pinned_sources(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            upstream = Path(directory)
            test_source = upstream / "tests/page/page-dispatchevent.spec.ts"
            fixture_source = upstream / "tests/config/testserver/index.ts"
            server_fixtures_source = upstream / "tests/config/serverFixtures.ts"
            test_source.parent.mkdir(parents=True)
            fixture_source.parent.mkdir(parents=True)
            test_source.write_text("official test\n")
            fixture_source.write_text("official fixture\n")
            server_fixtures_source.write_text("official server fixtures\n")
            suite = cast(
                Suite,
                {
                    "id": "secure",
                    "role": "browser-backed-official-file",
                    "runner": "packages/playwright/cli.js",
                    "requiresBrowser": True,
                    "project": "chromium-page",
                    "args": ["tests/page/page-dispatchevent.spec.ts"],
                    "expectedTestsPerProject": 17,
                    "expectedFiles": 1,
                    "expectedSpecIds": _secure_suite()["expectedSpecIds"],
                    "fixtureMode": "trusted-tailscale-tls-dual-port",
                    "upstreamTestSourceSha256": hashlib.sha256(
                        test_source.read_bytes()
                    ).hexdigest(),
                    "upstreamFixtureSourceSha256": hashlib.sha256(
                        fixture_source.read_bytes()
                    ).hexdigest(),
                    "upstreamServerFixturesSourceSha256": hashlib.sha256(
                        server_fixtures_source.read_bytes()
                    ).hexdigest(),
                },
            )

            # Act
            verified = _secure_fixture_source_digests(suite, upstream)

            # Assert
            self.assertEqual(len(verified), 3)
            server_fixtures_source.write_text("changed server fixtures\n")
            with self.assertRaisesRegex(ValueError, "pinned source drifted"):
                _secure_fixture_source_digests(suite, upstream)

    def test_secure_result_accepts_exact_pinned_ids_in_any_report_order(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            result = Path(directory) / "result.json"
            suite = _secure_suite()
            specs = list(reversed(_secure_specs(suite)))
            result.write_text(json.dumps({"suites": [{"specs": specs, "suites": []}]}))

            # Act
            count = _verify_secure_result_selection(result, "chromium-page", suite)

            # Assert
            self.assertEqual(count, 17)

    def test_secure_result_rejects_missing_spec(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            result = Path(directory) / "result.json"
            suite = _secure_suite()
            specs = _secure_specs(suite)
            specs.pop()
            result.write_text(json.dumps({"suites": [{"specs": specs, "suites": []}]}))

            # Act and Assert
            with self.assertRaisesRegex(ValueError, "exactly 17"):
                _verify_secure_result_selection(result, "chromium-page", suite)

    def test_secure_result_rejects_duplicate_spec_with_unchanged_count(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            result = Path(directory) / "result.json"
            suite = _secure_suite()
            specs = _secure_specs(suite)
            specs[-1] = specs[0].copy()
            result.write_text(json.dumps({"suites": [{"specs": specs}]}))

            # Act and Assert
            with self.assertRaisesRegex(ValueError, "17 unique pinned"):
                _verify_secure_result_selection(result, "chromium-page", suite)

    def test_secure_result_rejects_substituted_spec_with_unchanged_count(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            result = Path(directory) / "result.json"
            suite = _secure_suite()
            specs = _secure_specs(suite)
            specs[-1]["id"] = "abcdd35b469348a13783-00000000000000000000"
            result.write_text(json.dumps({"suites": [{"specs": specs}]}))

            # Act and Assert
            with self.assertRaisesRegex(ValueError, "17 unique pinned"):
                _verify_secure_result_selection(result, "chromium-page", suite)

    def test_secure_result_rejects_wrong_project_even_with_pinned_ids(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            result = Path(directory) / "result.json"
            suite = _secure_suite()
            specs = _secure_specs(suite)
            specs[-1]["tests"] = [{"projectId": "firefox-page"}]
            result.write_text(json.dumps({"suites": [{"specs": specs}]}))

            # Act and Assert
            with self.assertRaisesRegex(ValueError, "one pinned chromium-page"):
                _verify_secure_result_selection(result, "chromium-page", suite)

    def test_secure_manifest_rejects_missing_and_duplicate_spec_ids(self) -> None:
        for invalid in ([], ["duplicate"] * 17):
            with self.subTest(ids=invalid):
                # Arrange
                suite = _secure_suite()
                suite["expectedSpecIds"] = invalid

                # Act and Assert
                with self.assertRaisesRegex(ValueError, "17 unique pinned spec IDs"):
                    _secure_spec_ids(suite)

    def test_failed_certificate_issuance_removes_task_owned_directory(self) -> None:
        # Arrange
        with patch("run_playwright_official_conformance.subprocess.run") as run:
            run.return_value.returncode = 1
            run.return_value.stderr = "ACME control plane error"
            run.return_value.stdout = ""

            # Act and Assert
            with self.assertRaisesRegex(RuntimeError, "ACME control plane error"):
                _install_secure_certificate("100.64.0.1", "host.tailnet.ts.net")
            self.assertEqual(run.call_count, 1)
            certificate_path = Path(run.call_args.args[0][3])
            self.assertFalse(certificate_path.parent.exists())


class PinnedFullFileTests(unittest.TestCase):
    def test_page_click_public_selection_pins_exact_81_plus_6_ids(self) -> None:
        # Arrange
        root = Path(__file__).parents[1]
        manifest = cast(
            Manifest,
            json.loads(
                (
                    root
                    / "tests/official/playwright-core-page-click-public-manifest.json"
                ).read_text()
            ),
        )
        suite = manifest["suites"][0]

        # Act
        grep_invert = _pinned_grep_invert(suite)

        # Assert
        self.assertIn("should not wait with noAutoWaiting", grep_invert)
        self.assertEqual(len(suite["expectedSpecIds"]), 81)
        self.assertEqual(len(suite["excludedSpecIds"]), 6)

    def test_page_click_public_selection_rejects_changed_filter_or_ids(self) -> None:
        root = Path(__file__).parents[1]
        original = cast(
            Manifest,
            json.loads(
                (
                    root
                    / "tests/official/playwright-core-page-click-public-manifest.json"
                ).read_text()
            ),
        )["suites"][0]
        for field, replacement in (
            ("pinnedGrepInvert", ".*"),
            ("expectedTestsPerProject", 80),
            ("upstreamTestSourceSha256", "0" * 64),
            ("excludedSpecIds", []),
            ("expectedSpecIds", [*original["expectedSpecIds"][:-1], "substituted-id"]),
        ):
            with self.subTest(field=field):
                # Arrange
                suite = cast(Suite, {**original, field: replacement})

                # Act and Assert
                with self.assertRaisesRegex(
                    ValueError, "page-click public selection|page-click public IDs"
                ):
                    _pinned_grep_invert(suite)

    def test_set_content_public_selection_pins_exact_15_plus_2_ids(self) -> None:
        # Arrange
        root = Path(__file__).parents[1]
        manifest = cast(
            Manifest,
            json.loads(
                (
                    root
                    / "tests/official/playwright-core-page-set-content-public-manifest.json"
                ).read_text()
            ),
        )
        suite = manifest["suites"][0]

        # Act
        grep_invert = _pinned_grep_invert(suite)

        # Assert
        self.assertEqual(grep_invert, "should handle timeout properly(?: 2)?$")
        self.assertEqual(len(suite["expectedSpecIds"]), 15)
        self.assertEqual(len(suite["excludedSpecIds"]), 2)

    def test_set_content_public_selection_rejects_changed_private_or_public_ids(
        self,
    ) -> None:
        root = Path(__file__).parents[1]
        original = cast(
            Manifest,
            json.loads(
                (
                    root
                    / "tests/official/playwright-core-page-set-content-public-manifest.json"
                ).read_text()
            ),
        )["suites"][0]
        for field, replacement in (
            ("pinnedGrepInvert", ".*"),
            ("expectedTestsPerProject", 14),
            ("excludedSpecIds", []),
            ("expectedSpecIds", [*original["expectedSpecIds"][:-1], "substituted-id"]),
        ):
            with self.subTest(field=field):
                # Arrange
                suite = cast(Suite, {**original, field: replacement})

                # Act and Assert
                with self.assertRaisesRegex(
                    ValueError, "set-content public selection|public IDs"
                ):
                    _pinned_grep_invert(suite)

    def test_set_content_public_rejects_ad_hoc_grep_invert_option(self) -> None:
        # Arrange
        runner = Path(__file__).parent / "run_playwright_official_conformance.py"

        # Act
        completed = subprocess.run(
            [
                sys.executable,
                str(runner),
                "oracle",
                "/nonexistent/upstream",
                "--grep-invert=.*",
            ],
            capture_output=True,
            text=True,
            check=False,
        )

        # Assert
        self.assertEqual(completed.returncode, 2)
        self.assertIn("unrecognized arguments: --grep-invert=.*", completed.stderr)

    def test_pinned_one_row_grep_requires_one_expected_case(self) -> None:
        # Arrange
        suite = cast(
            Suite,
            {
                "pinnedGrep": "should not leak listeners during 20 waitForNavigation",
                "expectedTestsPerProject": 1,
            },
        )

        # Act
        grep = _pinned_grep(suite)

        # Assert
        self.assertEqual(grep, suite["pinnedGrep"])
        suite["expectedTestsPerProject"] = 2
        with self.assertRaisesRegex(ValueError, "one exact expected test"):
            _pinned_grep(suite)

    def test_cross_origin_forward_is_limited_to_pinned_goto_suite(self) -> None:
        # Arrange
        root = Path(__file__).parents[1]
        manifest = cast(
            Manifest,
            json.loads(
                (
                    root / "tests/official/playwright-core-page-goto-manifest.json"
                ).read_text()
            ),
        )
        suite = manifest["suites"][0]
        legacy_suite = cast(
            Suite,
            {key: value for key, value in suite.items() if key != "crossOriginForward"},
        )
        suite["crossOriginForward"] = True

        # Act
        enabled = _cross_origin_forward_enabled(suite)
        legacy_enabled = _cross_origin_forward_enabled(legacy_suite)

        # Assert
        self.assertTrue(enabled)
        self.assertFalse(legacy_enabled)
        suite["args"] = ["tests/page/other.spec.ts"]
        with self.assertRaisesRegex(ValueError, "limited to the pinned page-goto"):
            _cross_origin_forward_enabled(suite)

    def test_cross_origin_forward_accepts_only_versioned_route_manifest(self) -> None:
        # Arrange
        root = Path(__file__).parents[1]
        old_manifest = cast(
            Manifest,
            json.loads(
                (
                    root
                    / "tests/official/playwright-core-page-route-public-manifest.json"
                ).read_text()
            ),
        )
        new_manifest = cast(
            Manifest,
            json.loads(
                (
                    root
                    / "tests/official/playwright-core-page-route-public-origin-v1-manifest.json"
                ).read_text()
            ),
        )
        old_suite = old_manifest["suites"][0]
        new_suite = new_manifest["suites"][0]

        # Act
        enabled = _cross_origin_forward_enabled(new_suite)
        legacy_enabled = _cross_origin_forward_enabled(old_suite)

        # Assert
        self.assertEqual(len(old_manifest["suites"]), 1)
        self.assertEqual(len(new_manifest["suites"]), 1)
        self.assertEqual(
            {key: value for key, value in old_manifest.items() if key != "suites"},
            {key: value for key, value in new_manifest.items() if key != "suites"},
        )
        self.assertEqual(new_suite, {**old_suite, "crossOriginForward": True})
        self.assertEqual(new_suite["expectedTestsPerProject"], 55)
        self.assertTrue(enabled)
        self.assertFalse(legacy_enabled)

    def test_cross_origin_forward_rejects_route_selection_and_metadata_drift(
        self,
    ) -> None:
        # Arrange
        root = Path(__file__).parents[1]
        manifest = cast(
            Manifest,
            json.loads(
                (
                    root
                    / "tests/official/playwright-core-page-route-public-origin-v1-manifest.json"
                ).read_text()
            ),
        )
        suite = manifest["suites"][0]
        drifted_fields: dict[str, object] = {
            "args": ["tests/page/other.spec.ts"],
            "project": "chromium-context",
            "expectedTestsPerProject": 54,
            "expectedFiles": 2,
            "upstreamTestSourceSha256": "0" * 64,
            "expectedSpecIdsSha256": "0" * 64,
            "candidateAdapter": "test-mode-playwright-core-cdp",
            "oracleAdapter": "test-mode-playwright-core",
            "crossOriginForward": False,
            "pinnedGrep": "one test",
        }

        # Act and Assert
        for field, value in drifted_fields.items():
            with self.subTest(field=field):
                drifted = cast(Suite, {**suite, field: value})
                with self.assertRaisesRegex(ValueError, "exact pinned page-route"):
                    _cross_origin_forward_enabled(drifted)

    def test_cross_origin_forward_accepts_exact_history_file_manifest(self) -> None:
        # Arrange
        root = Path(__file__).parents[1]
        manifest = cast(
            Manifest,
            json.loads(
                (
                    root
                    / "tests/official/playwright-core-page-history-public-chromium-v1-manifest.json"
                ).read_text()
            ),
        )
        suite = manifest["suites"][0]

        # Act
        enabled = _cross_origin_forward_enabled(suite)

        # Assert
        self.assertEqual(len(manifest["suites"]), 1)
        self.assertEqual(suite["expectedTestsPerProject"], 16)
        self.assertEqual(suite["project"], "chromium-page")
        self.assertTrue(enabled)

    def test_cross_origin_forward_rejects_history_selection_and_metadata_drift(
        self,
    ) -> None:
        # Arrange
        root = Path(__file__).parents[1]
        manifest = cast(
            Manifest,
            json.loads(
                (
                    root
                    / "tests/official/playwright-core-page-history-public-chromium-v1-manifest.json"
                ).read_text()
            ),
        )
        suite = manifest["suites"][0]
        drifted_fields: dict[str, object] = {
            "args": ["tests/page/other.spec.ts"],
            "project": "firefox-page",
            "expectedTestsPerProject": 15,
            "expectedFiles": 2,
            "upstreamTestSourceSha256": "0" * 64,
            "expectedSpecIdsSha256": "0" * 64,
            "candidateAdapter": "test-mode-playwright-core-cdp",
            "oracleAdapter": "test-mode-playwright-core",
            "crossOriginForward": False,
            "pinnedGrep": "one test",
        }

        # Act and Assert
        for field, value in drifted_fields.items():
            with self.subTest(field=field):
                drifted = cast(Suite, {**suite, field: value})
                with self.assertRaisesRegex(ValueError, "exact pinned page-history"):
                    _cross_origin_forward_enabled(drifted)
        missing_flag = cast(
            Suite, {key: value for key, value in suite.items() if key != "crossOriginForward"}
        )
        with self.assertRaisesRegex(ValueError, "exact pinned page-history"):
            _cross_origin_forward_enabled(missing_flag)

    def test_history_file_source_pin_matches_upstream(self) -> None:
        # Arrange
        root = Path(__file__).parents[1]
        upstream = Path(
            os.environ.get(
                "PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM",
                "/Users/cyuan/projects/thirdparty/playwright-v1.58.0",
            )
        )
        if not (upstream / "tests/page/page-history.spec.ts").is_file():
            self.skipTest("pinned upstream Playwright checkout is unavailable")
        manifest = cast(
            Manifest,
            json.loads(
                (
                    root
                    / "tests/official/playwright-core-page-history-public-chromium-v1-manifest.json"
                ).read_text()
            ),
        )

        # Act
        source = _pinned_full_file_source(manifest["suites"][0], upstream)

        # Assert
        self.assertEqual(source[:3], ("page/page-history.spec.ts", 16, "chromium-page"))

    def test_history_file_result_verifier_preserves_red(self) -> None:
        # Arrange
        root = Path(__file__).parents[1]
        upstream = Path(
            os.environ.get(
                "PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM",
                "/Users/cyuan/projects/thirdparty/playwright-v1.58.0",
            )
        )
        if not (upstream / "tests/page/page-history.spec.ts").is_file():
            self.skipTest("pinned upstream Playwright checkout is unavailable")
        manifest = cast(
            Manifest,
            json.loads(
                (
                    root
                    / "tests/official/playwright-core-page-history-public-chromium-v1-manifest.json"
                ).read_text()
            ),
        )
        suite = manifest["suites"][0]
        ids = (
            "34c3de86d93245a3d399-6afc0abb5612468ce13d",
            "34c3de86d93245a3d399-4dfffa636cc215010883",
            "34c3de86d93245a3d399-e8c171d7d1bab10f52e4",
            "34c3de86d93245a3d399-72f45706d7dc5f2834b1",
            "34c3de86d93245a3d399-94501e40733c80c18118",
            "34c3de86d93245a3d399-c126322edb449fc1d8ce",
            "34c3de86d93245a3d399-884c389b8f2a36fbbd4d",
            "34c3de86d93245a3d399-26e2f404dbbae81c1b33",
            "34c3de86d93245a3d399-2e235a3d50d52c9643a7",
            "34c3de86d93245a3d399-da3eef3821ac9e00159f",
            "34c3de86d93245a3d399-62e435ae63d00a9364b3",
            "34c3de86d93245a3d399-ca5c817a3c8a357dfe20",
            "34c3de86d93245a3d399-1584065d84619c0cde95",
            "34c3de86d93245a3d399-d8e4611f384d9260c715",
            "34c3de86d93245a3d399-ee5cb1eba1cdc63e5a23",
            "34c3de86d93245a3d399-f87c743fe5c65722a574",
        )
        specs = [
            {
                "id": spec_id,
                "file": "page/page-history.spec.ts",
                "tests": [
                    {
                        "projectId": "chromium-page",
                        "status": "unexpected" if index == 0 else "expected",
                        "results": [
                            {"status": "failed" if index == 0 else "passed"}
                        ],
                    }
                ],
            }
            for index, spec_id in enumerate(ids)
        ]

        # Act
        with TemporaryDirectory() as directory:
            result = Path(directory) / "history-result.json"
            result.write_text(json.dumps({"suites": [{"specs": specs}]}))
            count = _verify_pinned_full_file_result_selection(result, suite, upstream)

        # Assert
        self.assertEqual(count, 16)

    def test_route_cross_origin_manifest_rejects_ad_hoc_case_filters(self) -> None:
        # Arrange
        root = Path(__file__).parents[1]
        manifest = (
            root
            / "tests/official/playwright-core-page-route-public-origin-v1-manifest.json"
        )
        upstream = root / "nonexistent-official-core-upstream"
        pinned_commit = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"

        # Act and Assert
        for filter_args in (
            ["--test", "tests/page/other.spec.ts"],
            ["--grep", "one test"],
        ):
            with self.subTest(filter_args=filter_args):
                argv = [
                    "run_playwright_official_conformance.py",
                    "oracle",
                    str(upstream),
                    "--manifest",
                    str(manifest),
                    "--suite",
                    "library-page-route-public-chromium",
                    "--collect-only",
                    *filter_args,
                ]
                stderr = io.StringIO()
                with (
                    patch.object(sys, "argv", argv),
                    patch.object(sys, "stderr", stderr),
                    patch(
                        "run_playwright_official_conformance._git",
                        side_effect=[pinned_commit, ""],
                    ),
                    self.assertRaises(SystemExit) as exit_status,
                ):
                    run_official_main()
                self.assertEqual(exit_status.exception.code, 2)
                self.assertIn(
                    "pinned full-file Core suite forbids ad-hoc --test and --grep filters",
                    stderr.getvalue(),
                )

    @staticmethod
    def _fixture(directory: Path) -> tuple[Path, Path, Suite, list[dict[str, object]]]:
        upstream = directory / "upstream"
        source = upstream / "tests/page/example.spec.ts"
        source.parent.mkdir(parents=True)
        source.write_text("it('first', () => {});\nit('second', () => {});\n")
        spec_ids = ["first-id", "second-id"]
        suite = cast(
            Suite,
            {
                "id": "library-example-chromium",
                "role": "browser-backed-official-file",
                "runner": "packages/playwright/cli.js",
                "args": ["tests/page/example.spec.ts"],
                "project": "chromium-page",
                "requiresBrowser": True,
                "expectedFiles": 1,
                "expectedTestsPerProject": 2,
                "upstreamTestSourceSha256": hashlib.sha256(
                    source.read_bytes()
                ).hexdigest(),
                "expectedSpecIdsSha256": hashlib.sha256(
                    "".join(f"{spec_id}\n" for spec_id in sorted(spec_ids)).encode()
                ).hexdigest(),
            },
        )
        specs: list[dict[str, object]] = [
            {
                "id": spec_id,
                "file": "page/example.spec.ts",
                "tests": [
                    {
                        "projectId": "chromium-page",
                        "status": "unexpected" if index == 0 else "skipped",
                        "results": [{"status": "failed" if index == 0 else "skipped"}],
                    }
                ],
            }
            for index, spec_id in enumerate(spec_ids)
        ]
        result = directory / "result.json"
        result.write_text(json.dumps({"suites": [{"specs": specs, "suites": []}]}))
        return upstream, result, suite, specs

    def test_pinned_full_file_accepts_failed_and_skipped_test_outcomes(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            upstream, result, suite, _ = self._fixture(Path(directory))

            # Act
            source = _pinned_full_file_source(suite, upstream)
            count = _verify_pinned_full_file_result_selection(result, suite, upstream)

            # Assert
            self.assertEqual(source[:3], ("page/example.spec.ts", 2, "chromium-page"))
            self.assertEqual(count, 2)

    def test_pinned_full_file_rejects_source_drift(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            upstream, _, suite, _ = self._fixture(Path(directory))
            (upstream / "tests/page/example.spec.ts").write_text(
                "it('changed', () => {});\n"
            )

            # Act and Assert
            with self.assertRaisesRegex(ValueError, "source drifted"):
                _pinned_full_file_source(suite, upstream)

    def test_pinned_full_file_rejects_missing_and_substituted_ids(self) -> None:
        for replacement in (None, "other-id", "first-id"):
            with (
                self.subTest(replacement=replacement),
                TemporaryDirectory() as directory,
            ):
                # Arrange
                upstream, result, suite, specs = self._fixture(Path(directory))
                if replacement is None:
                    specs.pop()
                else:
                    specs[-1]["id"] = replacement
                result.write_text(
                    json.dumps({"suites": [{"specs": specs, "suites": []}]})
                )

                # Act and Assert
                with self.assertRaisesRegex(ValueError, "expected 2 unique"):
                    _verify_pinned_full_file_result_selection(result, suite, upstream)

    def test_pinned_full_file_rejects_wrong_file_or_project(self) -> None:
        for field in ("file", "project"):
            with self.subTest(field=field), TemporaryDirectory() as directory:
                # Arrange
                upstream, result, suite, specs = self._fixture(Path(directory))
                if field == "file":
                    specs[-1]["file"] = "page/other.spec.ts"
                else:
                    tests = cast(list[dict[str, object]], specs[-1]["tests"])
                    tests[0]["projectId"] = "firefox-page"
                result.write_text(
                    json.dumps({"suites": [{"specs": specs, "suites": []}]})
                )

                # Act and Assert
                with self.assertRaisesRegex(ValueError, "wrong file, ID or project"):
                    _verify_pinned_full_file_result_selection(result, suite, upstream)

    def test_pinned_full_file_rejects_collected_but_unexecuted_cases(self) -> None:
        for change in (
            "missingStatus",
            "missingResults",
            "emptyResults",
            "invalidAttempt",
        ):
            with self.subTest(change=change), TemporaryDirectory() as directory:
                # Arrange
                upstream, result, suite, specs = self._fixture(Path(directory))
                test = cast(list[dict[str, object]], specs[0]["tests"])[0]
                if change == "missingStatus":
                    test.pop("status")
                elif change == "missingResults":
                    test.pop("results")
                elif change == "emptyResults":
                    test["results"] = []
                else:
                    test["results"] = [{"status": "unknown"}]
                result.write_text(
                    json.dumps({"suites": [{"specs": specs, "suites": []}]})
                )

                # Act and Assert
                with self.assertRaisesRegex(ValueError, "requires an executed attempt"):
                    _verify_pinned_full_file_result_selection(result, suite, upstream)


if __name__ == "__main__":
    unittest.main()
