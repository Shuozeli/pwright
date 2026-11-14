#!/usr/bin/env python3
"""Tests for Playwright Python result provenance construction."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import sysconfig
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from typing import TypedDict, cast

from playwright_python_official_provenance import (
    RuntimeIdentity,
    browser_registry_diagnostic_selected,
    browser_registry_snapshot,
    build_provenance,
    probe_collection_profile_identity,
    probe_official_browsers_json,
    probe_runtime_identity,
    require_browser_registry_unchanged,
    require_tooling_unchanged,
    string_list_sha256,
    tooling_sources,
    tree_sha256,
)


class RegistryExecutable(TypedDict):
    revision: str
    path: str
    exists: bool


def _registry_executables(snapshot: dict[str, object]) -> list[RegistryExecutable]:
    return cast(list[RegistryExecutable], snapshot["executables"])


class PythonOfficialProvenanceTests(unittest.TestCase):
    def test_browser_registry_probe_ignores_upstream_distribution_shadow(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            upstream = root / "upstream"
            metadata = upstream / "playwright.egg-info"
            metadata.mkdir(parents=True)
            (metadata / "PKG-INFO").write_text(
                "Metadata-Version: 2.1\nName: playwright\nVersion: 1.58.0\n"
            )
            shadow = upstream / "playwright/driver/package/browsers.json"
            shadow.parent.mkdir(parents=True)
            shadow.write_text("{}\n")
            environment = os.environ.copy()
            environment["PYTHONPATH"] = str(upstream)
            visible_distribution = subprocess.run(
                [
                    sys.executable,
                    "-c",
                    "from importlib.metadata import distribution; "
                    "print(distribution('playwright').locate_file('playwright/driver/package/browsers.json'))",
                ],
                cwd=root,
                env=environment,
                check=True,
                capture_output=True,
                text=True,
            )

            # Act
            resolved = probe_official_browsers_json(sys.executable, cwd=root, env=environment)

            # Assert
            self.assertEqual(Path(visible_distribution.stdout.strip()), shadow)
            self.assertEqual(
                resolved,
                (
                    Path(sysconfig.get_path("purelib")) / "playwright/driver/package/browsers.json"
                ).resolve(),
            )
            self.assertNotEqual(resolved, shadow)

    def test_browser_registry_selection_is_limited_to_executable_execution(
        self,
    ) -> None:
        # Arrange
        manifest: dict[str, object] = {"tag": "v1.58.0"}
        selections = ["tests/async/test_launcher.py::test_browser_type_executable_path_should_work"]

        # Act
        selected = browser_registry_diagnostic_selected(
            manifest, selections, manifest_cases=True, collect_only=False
        )
        collection = browser_registry_diagnostic_selected(
            manifest, selections, manifest_cases=True, collect_only=True
        )
        ad_hoc = browser_registry_diagnostic_selected(
            manifest, selections, manifest_cases=False, collect_only=False
        )

        # Assert
        self.assertTrue(selected)
        self.assertFalse(collection)
        self.assertFalse(ad_hoc)

    def test_browser_registry_zero_path_uses_mode_specific_package_root(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            pwright_package = root / "site/pwright"
            pwright_package.mkdir(parents=True)
            runtime: RuntimeIdentity = {
                "pythonExecutable": sys.executable,
                "pythonVersion": "3.12",
                "playwrightPackage": str(root / "shim/playwright"),
                "pwrightPackage": str(pwright_package),
                "pwrightArtifact": None,
            }
            environment = os.environ.copy()
            environment["PLAYWRIGHT_BROWSERS_PATH"] = "0"
            browsers_json = probe_official_browsers_json(sys.executable, cwd=root, env=environment)

            # Act
            candidate = browser_registry_snapshot(
                mode="candidate",
                runtime=runtime,
                cwd=root,
                env=environment,
                engines=["chromium"],
                browsers_json_path=browsers_json,
                os_name="darwin",
                architecture="arm64",
            )
            oracle = browser_registry_snapshot(
                mode="oracle",
                runtime=runtime,
                cwd=root,
                env=environment,
                engines=["chromium"],
                browsers_json_path=browsers_json,
                os_name="darwin",
                architecture="arm64",
            )

            # Assert
            self.assertEqual(candidate["registryRoot"], str(pwright_package / ".local-browsers"))
            self.assertEqual(oracle["registryRoot"], str(browsers_json.parent / ".local-browsers"))
            self.assertNotEqual(candidate["registryRoot"], oracle["registryRoot"])
            self.assertEqual(_registry_executables(candidate)[0]["revision"], "1208")

    def test_browser_registry_relative_path_and_webkit_override_are_recorded(
        self,
    ) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            runtime: RuntimeIdentity = {
                "pythonExecutable": sys.executable,
                "pythonVersion": "3.12",
                "playwrightPackage": str(root / "shim/playwright"),
                "pwrightPackage": str(root / "site/pwright"),
                "pwrightArtifact": None,
            }
            environment = os.environ.copy()
            environment["PLAYWRIGHT_BROWSERS_PATH"] = "relative-cache"
            environment["INIT_CWD"] = str(root)
            environment["PLAYWRIGHT_HOST_PLATFORM_OVERRIDE"] = "debian11-arm64"
            browsers_json = probe_official_browsers_json(sys.executable, cwd=root, env=environment)

            # Act
            snapshot = browser_registry_snapshot(
                mode="candidate",
                runtime=runtime,
                cwd=root,
                env=environment,
                engines=["webkit"],
                browsers_json_path=browsers_json,
                os_name="linux",
                architecture="aarch64",
                os_release="ID=ubuntu\nVERSION_ID=22.04\n",
            )

            # Assert
            self.assertEqual(snapshot["registryRoot"], str(root / "relative-cache"))
            self.assertEqual(_registry_executables(snapshot)[0]["revision"], "2105")
            self.assertIn(
                "webkit_debian11_arm64_special-2105",
                _registry_executables(snapshot)[0]["path"],
            )

    def test_browser_registry_host_override_changes_executable_platform_only(
        self,
    ) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            runtime: RuntimeIdentity = {
                "pythonExecutable": sys.executable,
                "pythonVersion": "3.12",
                "playwrightPackage": str(root / "shim/playwright"),
                "pwrightPackage": str(root / "site/pwright"),
                "pwrightArtifact": None,
            }
            environment = os.environ.copy()
            environment["PLAYWRIGHT_HOST_PLATFORM_OVERRIDE"] = "win64"
            environment.pop("PLAYWRIGHT_BROWSERS_PATH", None)
            environment["HOME"] = str(root)
            browsers_json = probe_official_browsers_json(sys.executable, cwd=root, env=environment)

            # Act
            snapshot = browser_registry_snapshot(
                mode="candidate",
                runtime=runtime,
                cwd=root,
                env=environment,
                engines=["chromium"],
                browsers_json_path=browsers_json,
                os_name="darwin",
                architecture="arm64",
            )

            # Assert
            self.assertEqual(snapshot["cachePlatform"], "mac-arm64")
            self.assertEqual(snapshot["shortPlatform"], "win64")
            self.assertEqual(snapshot["registryRoot"], str(root / "Library/Caches/ms-playwright"))
            self.assertTrue(
                _registry_executables(snapshot)[0]["path"].endswith("chrome-win64/chrome.exe")
            )

    def test_browser_registry_default_cache_honors_platform_cache_env_without_home(
        self,
    ) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            runtime: RuntimeIdentity = {
                "pythonExecutable": sys.executable,
                "pythonVersion": "3.12",
                "playwrightPackage": str(root / "shim/playwright"),
                "pwrightPackage": str(root / "site/pwright"),
                "pwrightArtifact": None,
            }
            browsers_json = probe_official_browsers_json(
                sys.executable, cwd=root, env=os.environ.copy()
            )
            linux_env = {"XDG_CACHE_HOME": str(root / "xdg")}
            windows_env = {"LOCALAPPDATA": str(root / "local")}

            # Act
            linux = browser_registry_snapshot(
                mode="candidate",
                runtime=runtime,
                cwd=root,
                env=linux_env,
                engines=["chromium"],
                browsers_json_path=browsers_json,
                os_name="linux",
                architecture="x86_64",
                os_release="ID=ubuntu\nVERSION_ID=22.04\n",
            )
            windows = browser_registry_snapshot(
                mode="candidate",
                runtime=runtime,
                cwd=root,
                env=windows_env,
                engines=["chromium"],
                browsers_json_path=browsers_json,
                os_name="win32",
                architecture="AMD64",
            )

            # Assert
            self.assertEqual(linux["registryRoot"], str(root / "xdg/ms-playwright"))
            self.assertEqual(linux["hostPlatform"], "ubuntu22.04-x64")
            self.assertEqual(windows["registryRoot"], str(root / "local/ms-playwright"))

    def test_browser_registry_ubuntu18_webkit_keeps_default_revision(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            runtime: RuntimeIdentity = {
                "pythonExecutable": sys.executable,
                "pythonVersion": "3.12",
                "playwrightPackage": str(root / "shim/playwright"),
                "pwrightPackage": str(root / "site/pwright"),
                "pwrightArtifact": None,
            }
            environment = {"HOME": str(root)}
            browsers_json = probe_official_browsers_json(
                sys.executable, cwd=root, env=os.environ.copy()
            )

            # Act
            snapshot = browser_registry_snapshot(
                mode="candidate",
                runtime=runtime,
                cwd=root,
                env=environment,
                engines=["webkit"],
                browsers_json_path=browsers_json,
                os_name="linux",
                architecture="x86_64",
                os_release="ID=ubuntu\nVERSION_ID=18.04\n",
            )

            # Assert
            self.assertEqual(snapshot["hostPlatform"], "ubuntu18.04-x64")
            self.assertEqual(_registry_executables(snapshot)[0]["revision"], "2248")
            self.assertIn("webkit-2248/pw_run.sh", _registry_executables(snapshot)[0]["path"])

    def test_browser_registry_snapshot_detects_exact_executable_appearance(
        self,
    ) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            cache = root / "cache"
            runtime: RuntimeIdentity = {
                "pythonExecutable": sys.executable,
                "pythonVersion": "3.12",
                "playwrightPackage": str(root / "shim/playwright"),
                "pwrightPackage": str(root / "site/pwright"),
                "pwrightArtifact": None,
            }
            environment = os.environ.copy()
            environment["PLAYWRIGHT_BROWSERS_PATH"] = str(cache)
            browsers_json = probe_official_browsers_json(sys.executable, cwd=root, env=environment)
            before = browser_registry_snapshot(
                mode="candidate",
                runtime=runtime,
                cwd=root,
                env=environment,
                engines=["firefox"],
                browsers_json_path=browsers_json,
                os_name="darwin",
                architecture="arm64",
            )
            executable = Path(_registry_executables(before)[0]["path"])

            # Act
            executable.parent.mkdir(parents=True)
            executable.write_bytes(b"")
            after = browser_registry_snapshot(
                mode="candidate",
                runtime=runtime,
                cwd=root,
                env=environment,
                engines=["firefox"],
                browsers_json_path=browsers_json,
                os_name="darwin",
                architecture="arm64",
            )

            # Assert
            self.assertFalse(_registry_executables(before)[0]["exists"])
            self.assertTrue(_registry_executables(after)[0]["exists"])
            self.assertNotEqual(before, after)
            with self.assertRaisesRegex(ValueError, "executable existence changed"):
                require_browser_registry_unchanged(
                    before,
                    mode="candidate",
                    python=sys.executable,
                    runtime=runtime,
                    cwd=root,
                    env=environment,
                    engines=["firefox"],
                    browsers_json_path=browsers_json,
                    os_name="darwin",
                    architecture="arm64",
                )

    def test_browser_registry_rejects_unpinned_browsers_json(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            browsers_json = root / "browsers.json"
            browsers_json.write_text('{"browsers": []}\n')
            runtime: RuntimeIdentity = {
                "pythonExecutable": sys.executable,
                "pythonVersion": "3.12",
                "playwrightPackage": str(root / "playwright"),
                "pwrightPackage": str(root / "pwright"),
                "pwrightArtifact": None,
            }

            # Act and Assert
            with self.assertRaisesRegex(ValueError, "SHA-256 differs"):
                browser_registry_snapshot(
                    mode="candidate",
                    runtime=runtime,
                    cwd=root,
                    env=os.environ.copy(),
                    engines=["chromium"],
                    browsers_json_path=browsers_json,
                    os_name="darwin",
                    architecture="arm64",
                )

    def test_tooling_snapshot_rejects_midrun_source_drift(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            scripts = root / "scripts"
            scripts.mkdir()
            for name in (
                "check_playwright_python_official_source.py",
                "official_result_destination.py",
                "playwright_python_official_provenance.py",
                "run_playwright_python_official_conformance.py",
            ):
                (scripts / name).write_text(f"source = {name!r}\n")
            snapshot = tooling_sources(root)

            # Act
            (scripts / "check_playwright_python_official_source.py").write_text("changed\n")

            # Assert
            with self.assertRaisesRegex(ValueError, "changed during the run"):
                require_tooling_unchanged(root, snapshot)

    def test_tooling_snapshot_rejects_symlink_source(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            scripts = root / "scripts"
            scripts.mkdir()
            for name in (
                "check_playwright_python_official_source.py",
                "official_result_destination.py",
                "playwright_python_official_provenance.py",
            ):
                (scripts / name).write_text(f"source = {name!r}\n")
            (scripts / "run_playwright_python_official_conformance.py").symlink_to(
                scripts / "official_result_destination.py"
            )

            # Act and Assert
            with self.assertRaisesRegex(ValueError, "symlink"):
                tooling_sources(root)

    def test_collection_profile_probe_verifies_frozen_runtime(self) -> None:
        # Arrange
        profile: dict[str, object] = {
            "name": "core",
            "selection": "tests",
            "expandedTests": 6501,
            "pytestVersion": "8.4.1",
        }

        # Act
        identity = probe_collection_profile_identity(
            sys.executable,
            profile,
            tag="v1.58.0",
            root=Path.cwd(),
        )

        # Assert
        self.assertEqual(identity["name"], "core")
        self.assertEqual(identity["pytestVersion"], "8.4.1")
        self.assertIsNone(identity["pytestPlaywrightVersion"])

    def test_tree_digest_ignores_python_bytecode_cache(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "module.py").write_text("value = 1\n")
            cache = root / "__pycache__"
            cache.mkdir()
            (cache / "module.cpython-39.pyc").write_bytes(b"first")
            expected = tree_sha256(root)
            (cache / "module.cpython-39.pyc").write_bytes(b"second")

            # Act
            actual = tree_sha256(root)

            # Assert
            self.assertEqual(actual, expected)

    def test_isolated_case_digest_preserves_order(self) -> None:
        # Arrange
        forward = ["tests/test_a.py::test_a", "tests/test_b.py::test_b"]
        reverse = list(reversed(forward))

        # Act
        forward_digest = string_list_sha256(forward)
        reverse_digest = string_list_sha256(reverse)

        # Assert
        self.assertNotEqual(forward_digest, reverse_digest)

    def test_candidate_provenance_hashes_runtime_and_case_inputs(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            manifest_path = root / "manifest.json"
            manifest: dict[str, object] = {
                "schema": 1,
                "repository": "https://example.test/playwright-python.git",
                "tag": "v1.2.3",
                "commit": "0123456789abcdef",
                "corpus": {"expandedTests": 2},
                "cases": [],
            }
            manifest_path.write_text(json.dumps(manifest) + "\n")
            playwright_package = root / "playwright"
            pwright_package = root / "pwright"
            harness = root / "runtime"
            for package in (playwright_package, pwright_package, harness):
                package.mkdir()
                (package / "__init__.py").write_text("\n")
            artifact = pwright_package / "_native.abi3.so"
            artifact.write_bytes(b"native")
            inventory = root / "inventory.jsonl"
            inventory.write_text('{"id":"tests/test_a.py::test_a"}\n')
            result = root / "result.xml"
            result.write_text('<testsuite tests="1"/>\n')
            case_reports = root / "result.xml.cases"
            case_reports.mkdir()
            (case_reports / "00001.xml").write_text('<testcase name="test_a"/>\n')
            runtime: RuntimeIdentity = {
                "pythonExecutable": "/usr/bin/python3",
                "pythonVersion": "3.9.6",
                "playwrightPackage": str(playwright_package),
                "pwrightPackage": str(pwright_package),
                "pwrightArtifact": str(artifact),
            }
            scripts = root / "scripts"
            scripts.mkdir()
            for name in (
                "check_playwright_python_official_source.py",
                "official_result_destination.py",
                "playwright_python_official_provenance.py",
                "run_playwright_python_official_conformance.py",
            ):
                (scripts / name).write_text(f"source = {name!r}\n")
            tooling_source_records = tooling_sources(root)

            # Act
            provenance = build_provenance(
                mode="candidate",
                manifest=manifest,
                manifest_path=manifest_path,
                upstream=root / "upstream",
                scope="selected",
                browsers=["chromium"],
                browser_channel="chrome",
                selections=["tests/test_a.py::test_a"],
                manifest_cases=False,
                manifest_api=None,
                bootstrap_only=False,
                collect_only=False,
                endpoint="http://127.0.0.1:9222",
                fixture_bind="100.64.0.1",
                fixture_host="example-yuacx.example.ts.net",
                server_prefix=None,
                secure_http_fixture=False,
                case_timeout=30.0,
                isolate_cases=True,
                resume_isolated=False,
                inventory=inventory,
                isolated_node_ids=["tests/test_a.py::test_a"],
                command=["/usr/bin/python3", "-m", "pytest", "-q"],
                runtime=runtime,
                harness_inputs=[harness],
                result=result,
                exit_code=0,
                tooling_source_records=tooling_source_records,
            )

            # Assert
            self.assertEqual(provenance["mode"], "candidate")
            self.assertEqual(provenance["schema"], 2)
            self.assertEqual(provenance["toolingSources"], tooling_source_records)
            self.assertEqual(provenance["isolatedCaseCount"], 1)
            self.assertEqual(provenance["pwrightArtifact"], str(artifact))
            self.assertEqual(provenance["caseReports"], str(case_reports))
            self.assertEqual(len(str(provenance["resultSha256"])), 64)

    def test_runtime_probe_resolves_candidate_modules(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            for package in ("playwright", "pwright"):
                package_path = root / package
                package_path.mkdir()
                (package_path / "__init__.py").write_text("\n")
            (root / "pwright/_native.py").write_text("\n")
            environment = os.environ.copy()
            environment["PYTHONPATH"] = str(root)

            # Act
            identity = probe_runtime_identity(
                sys.executable,
                "candidate",
                cwd=root,
                env=environment,
            )

            # Assert
            self.assertEqual(identity["playwrightPackage"], str((root / "playwright").resolve()))
            self.assertEqual(identity["pwrightPackage"], str((root / "pwright").resolve()))
            self.assertEqual(
                identity["pwrightArtifact"],
                str((root / "pwright/_native.py").resolve()),
            )


if __name__ == "__main__":
    unittest.main()
