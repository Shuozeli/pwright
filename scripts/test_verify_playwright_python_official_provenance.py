#!/usr/bin/env python3
"""Tests for Playwright Python provenance verification."""

from __future__ import annotations

import copy
import json
import os
import platform
import shutil
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from typing import TypedDict, cast
from unittest.mock import patch

from playwright_python_official_provenance import (
    RuntimeIdentity,
    browser_registry_snapshot,
    build_provenance,
    file_sha256,
    probe_collection_profile_identity,
    probe_official_browsers_json,
    tooling_sources,
    tree_sha256,
)
from run_playwright_python_official_conformance import (
    SYNC_LIFECYCLE_CASE_IDS,
    SYNC_LIFECYCLE_COMMIT,
    SYNC_LIFECYCLE_LOOP_MODE,
    SYNC_LIFECYCLE_LOOP_WRAPPER,
    SYNC_LIFECYCLE_LOOP_WRAPPER_SHA256,
    SYNC_LIFECYCLE_SOURCE_SHA256,
)
from verify_playwright_python_official_provenance import (
    ProvenanceValidationError,
    verify_provenance,
)


class RegistryExecutable(TypedDict):
    revision: str
    exists: bool


def _registry_executables(snapshot: dict[str, object]) -> list[RegistryExecutable]:
    return cast(list[RegistryExecutable], snapshot["executables"])


def _registry_environment(snapshot: dict[str, object]) -> dict[str, str | None]:
    return cast(dict[str, str | None], snapshot["environment"])


def _fixture(
    root: Path,
    *,
    browserless: bool = False,
    profile: bool = False,
    schema2: bool = False,
    schema5: bool = False,
    launch_mode: str | None = None,
    source_package: bool = False,
) -> tuple[dict[str, object], Path, Path, Path]:
    official = root / "packages/pwright-python/tests/official"
    runtime_harness = official / "runtime"
    candidate_harness = official / "shim"
    runtime_harness.mkdir(parents=True)
    candidate_harness.mkdir()
    (runtime_harness / "pwright_official_server_runtime.py").write_text("\n")
    (candidate_harness / "pwright_candidate_runtime.py").write_text("\n")

    upstream = root / "upstream"
    upstream.mkdir()
    (upstream / "pyproject.toml").write_text("[tool.pytest.ini_options]\n")
    source = upstream / "tests/test_page.py"
    source.parent.mkdir()
    source.write_text("def test_example():\n    pass\n")
    selection = "tests/test_page.py::test_example"
    manifest: dict[str, object] = {
        "schema": 1,
        "repository": "https://example.test/playwright-python.git",
        "tag": "v1.2.3",
        "commit": "0123456789abcdef",
        "corpus": {"expandedTests": 1},
        "cases": [],
    }
    profile_identity = None
    profile_manifest: dict[str, object] | None = None
    if profile:
        manifest["tag"] = "v1.58.0"
        profile_manifest = {
            "name": "examples",
            "selection": "tests",
            "expandedTests": 1,
            "pythonVersion": ".".join(platform.python_version().split(".")[:2]),
            "pytestVersion": "8.4.1",
        }
        cast(dict[str, object], manifest["corpus"])["collectionProfiles"] = [profile_manifest]
    if browserless:
        manifest["execution"] = {
            "requiresBrowser": False,
            "requiresFixtureServers": True,
        }
        manifest["cases"] = [
            {
                "id": selection,
                "capabilities": ["playwright.lifecycle.browserless"],
            }
        ]
    manifest_path = official / "manifest.json"
    manifest_path.write_text(json.dumps(manifest) + "\n")
    if profile_manifest is not None:
        profile_identity = probe_collection_profile_identity(
            sys.executable,
            profile_manifest,
            tag="v1.58.0",
            root=root,
        )

    playwright_package = root / "site/playwright"
    pwright_package = (
        root / "packages/pwright-python/python/pwright" if source_package else root / "site/pwright"
    )
    playwright_package.mkdir(parents=True)
    pwright_package.mkdir(parents=True)
    (playwright_package / "__init__.py").write_text("\n")
    (pwright_package / "__init__.py").write_text("\n")
    artifact = pwright_package / "_native.abi3.so"
    artifact.write_bytes(b"native")
    if schema5:
        libraries = pwright_package.parent / "shuozeli_pwright.dylibs"
        libraries.mkdir()
        (libraries / "libcrypto-a12805a1.3.dylib").write_bytes(b"crypto")
        (libraries / "libssl-ffd8ac69.3.dylib").write_bytes(b"ssl")
    runtime: RuntimeIdentity = {
        "pythonExecutable": str(Path(sys.executable).absolute()),
        "pythonVersion": platform.python_version(),
        "playwrightPackage": str(playwright_package),
        "pwrightPackage": str(pwright_package),
        "pwrightArtifact": str(artifact),
    }
    result = root / "result.xml"
    launch_property = (
        '<properties><property name="pwright.official.launchMode" '
        f'value="{launch_mode}"/></properties>'
        if launch_mode is not None
        else ""
    )
    result.write_text(
        f'<testsuite tests="1">{launch_property}<testcase name="test_example"/></testsuite>\n'
    )
    command = [
        sys.executable,
        "-m",
        "pytest",
        "-q",
        "--import-mode=importlib",
        "-c",
        str(upstream / "pyproject.toml"),
    ]
    if not profile:
        command.extend(
            [
                "-p",
                "pwright_official_server_runtime",
                "-p",
                "pwright_candidate_runtime",
            ]
        )
    command.extend(
        [
            "--browser=chromium",
            f"{source.resolve()}::test_example",
            f"--junitxml={result}",
        ]
    )
    tooling_source_records = None
    if schema2:
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
    provenance = build_provenance(
        mode="candidate",
        manifest=manifest,
        manifest_path=manifest_path,
        upstream=upstream,
        scope=(
            "manifest-cases-all" if browserless else "profile-examples" if profile else "selected"
        ),
        browsers=["chromium"],
        browser_channel=None,
        selections=[selection],
        manifest_cases=browserless,
        manifest_api=None,
        bootstrap_only=False,
        collect_only=False,
        endpoint=None if launch_mode == "owned" else "http://127.0.0.1:9222",
        fixture_bind=None if profile else "100.64.0.1",
        fixture_host=None if profile else "example-yuacx.example.ts.net",
        server_prefix=None,
        secure_http_fixture=False,
        case_timeout=None,
        isolate_cases=False,
        resume_isolated=False,
        inventory=None,
        isolated_node_ids=[],
        command=command,
        runtime=runtime,
        harness_inputs=[candidate_harness, runtime_harness],
        result=result,
        exit_code=0,
        collection_profile=profile_identity,
        tooling_source_records=tooling_source_records,
        launch_mode=launch_mode,
        capture_runtime_libraries=schema5,
    )
    provenance_path = result.with_suffix(".provenance.json")
    provenance_path.write_text(json.dumps(provenance, sort_keys=True) + "\n")
    return provenance, provenance_path, result, pwright_package


def _loop_fixture(root: Path, *, schema2: bool = False) -> tuple[dict[str, object], Path, Path]:
    provenance, provenance_path, result, _ = _fixture(root, browserless=True, schema2=schema2)
    manifest_path = Path(cast(str, provenance["manifest"]))
    manifest = cast(dict[str, object], json.loads(manifest_path.read_text()))
    manifest["repository"] = "https://github.com/microsoft/playwright-python.git"
    manifest["tag"] = "v1.58.0"
    manifest["commit"] = SYNC_LIFECYCLE_COMMIT
    manifest["files"] = {"tests/sync/test_sync.py": SYNC_LIFECYCLE_SOURCE_SHA256}
    manifest["executionMode"] = SYNC_LIFECYCLE_LOOP_MODE
    manifest["cases"] = [
        {"id": case_id, "capabilities": ["playwright.lifecycle.browserless"]}
        for case_id in SYNC_LIFECYCLE_CASE_IDS
    ]
    manifest_path.write_text(json.dumps(manifest) + "\n")
    upstream = Path(cast(str, provenance["upstream"]))
    source = upstream / "tests/sync/test_sync.py"
    source.parent.mkdir(exist_ok=True)
    source.write_text("\n")
    rows = [
        (
            '<testcase classname="tests.sync.test_sync" '
            f'name="{case_id.rsplit("::", 1)[1]}[{browser}]"/>'
        )
        for case_id in SYNC_LIFECYCLE_CASE_IDS
        for browser in ("chromium", "firefox", "webkit")
    ]
    result.write_text('<testsuite tests="6">' + "".join(rows) + "</testsuite>\n")
    provenance.update(
        {
            "repository": manifest["repository"],
            "tag": manifest["tag"],
            "commit": manifest["commit"],
            "manifestSha256": file_sha256(manifest_path),
            "scope": "manifest-cases-sync",
            "browsers": [],
            "selections": list(SYNC_LIFECYCLE_CASE_IDS),
            "manifestApi": "sync",
            "resultSha256": file_sha256(result),
            "executionMode": SYNC_LIFECYCLE_LOOP_MODE,
            "executionWrapperSha256": SYNC_LIFECYCLE_LOOP_WRAPPER_SHA256,
            "command": [
                sys.executable,
                "-c",
                SYNC_LIFECYCLE_LOOP_WRAPPER,
                "-q",
                "--import-mode=importlib",
                "-c",
                str(upstream / "pyproject.toml"),
                "-p",
                "pwright_official_server_runtime",
                "-p",
                "pwright_candidate_runtime",
                str(source.resolve()) + "::test_sync_playwright_multiple_times",
                str(source.resolve()) + "::test_sync_playwright_stop_multiple_times",
                f"--junitxml={result}",
            ],
        }
    )
    provenance_path.write_text(json.dumps(provenance, sort_keys=True) + "\n")
    return provenance, provenance_path, result


def _registry_fixture(
    root: Path, *, mode: str = "candidate", schema5: bool = False
) -> tuple[dict[str, object], Path]:
    provenance, provenance_path, _, _ = _fixture(
        root,
        browserless=True,
        schema2=True,
        schema5=schema5,
        launch_mode="owned" if schema5 else None,
    )
    manifest_path = Path(cast(str, provenance["manifest"]))
    manifest = cast(dict[str, object], json.loads(manifest_path.read_text()))
    case_id = "tests/async/test_launcher.py::test_browser_type_executable_path_should_work"
    manifest["tag"] = "v1.58.0"
    manifest["cases"] = [{"id": case_id, "capabilities": ["playwright.lifecycle.browserless"]}]
    manifest_path.write_text(json.dumps(manifest) + "\n")
    upstream = Path(cast(str, provenance["upstream"]))
    source = upstream / "tests/async/test_launcher.py"
    source.parent.mkdir(parents=True, exist_ok=True)
    source.write_text("def test_browser_type_executable_path_should_work():\n    pass\n")
    command = cast(list[str], provenance["command"])
    command[-2] = str(source.resolve()) + "::test_browser_type_executable_path_should_work"
    if mode == "oracle":
        provenance["mode"] = "oracle"
        provenance["pwrightPackage"] = None
        provenance["pwrightPackageSha256"] = None
        provenance["pwrightArtifact"] = None
        provenance["pwrightArtifactSha256"] = None
        provenance["harnessInputs"] = cast(list[dict[str, str]], provenance["harnessInputs"])[1:]
        command[command.index("pwright_candidate_runtime")] = "pwright_official_oracle_runtime"
        if schema5:
            provenance["pwrightRuntimeLibraries"] = []
    provenance["manifestSha256"] = file_sha256(manifest_path)
    provenance["tag"] = "v1.58.0"
    provenance["selections"] = [case_id]
    runtime: RuntimeIdentity = {
        "pythonExecutable": cast(str, provenance["pythonExecutable"]),
        "pythonVersion": cast(str, provenance["pythonVersion"]),
        "playwrightPackage": cast(str, provenance["playwrightPackage"]),
        "pwrightPackage": cast(str | None, provenance["pwrightPackage"]),
        "pwrightArtifact": cast(str | None, provenance["pwrightArtifact"]),
    }
    environment = os.environ.copy()
    browsers_json = probe_official_browsers_json(sys.executable, cwd=root, env=environment)
    provenance["browserRegistry"] = browser_registry_snapshot(
        mode=mode,
        runtime=runtime,
        cwd=root,
        env=environment,
        engines=["chromium"],
        browsers_json_path=browsers_json,
    )
    provenance["schema"] = 5 if schema5 else 3
    provenance["exitCode"] = 1
    result = Path(cast(str, provenance["result"]))
    launch_property = (
        '<properties><property name="pwright.official.launchMode" value="owned"/></properties>'
        if schema5
        else ""
    )
    result.write_text(
        '<testsuite tests="1" failures="1">'
        f'{launch_property}<testcase name="test_browser_type_executable_path_should_work">'
        "<failure>diagnostic fixture</failure></testcase></testsuite>\n"
    )
    provenance["resultSha256"] = file_sha256(result)
    provenance_path.write_text(json.dumps(provenance, sort_keys=True) + "\n")
    return provenance, provenance_path


class VerifyPythonOfficialProvenanceTests(unittest.TestCase):
    def test_schema5_registry_verifier_matches_capture_with_upstream_shadow(
        self,
    ) -> None:
        for mode in ("candidate", "oracle"):
            with self.subTest(mode=mode), TemporaryDirectory() as directory:
                # Arrange
                root = Path(directory)
                upstream = root / "shadow"
                metadata = upstream / "playwright.egg-info"
                metadata.mkdir(parents=True)
                (metadata / "PKG-INFO").write_text(
                    "Metadata-Version: 2.1\nName: playwright\nVersion: 1.58.0\n"
                )
                shadow = upstream / "playwright/driver/package/browsers.json"
                shadow.parent.mkdir(parents=True)
                shadow.write_text("{}\n")
                with patch.dict(os.environ, {"PYTHONPATH": str(upstream)}):
                    provenance, provenance_path = _registry_fixture(root, mode=mode, schema5=True)

                    # Act
                    verified = verify_provenance(
                        provenance,
                        provenance_path=provenance_path,
                        tooling_root=root,
                        check_checkout=False,
                    )

                # Assert
                self.assertEqual(verified["schema"], 5)
                self.assertEqual(verified["mode"], mode)
                self.assertEqual(verified["browserRegistry"], provenance["browserRegistry"])
                self.assertNotEqual(
                    cast(dict[str, object], verified["browserRegistry"])["browsersJsonPath"],
                    str(shadow),
                )

    def test_schema5_candidate_binds_both_adjacent_runtime_libraries(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, package = _fixture(
                root,
                schema2=True,
                schema5=True,
                launch_mode="owned",
                source_package=True,
            )
            libraries = package.parent / "shuozeli_pwright.dylibs"

            # Act
            verified = verify_provenance(
                provenance,
                provenance_path=provenance_path,
                tooling_root=root,
                check_checkout=False,
            )

            # Assert
            self.assertEqual(verified["schema"], 5)
            self.assertEqual(
                verified["pwrightRuntimeLibraries"],
                [
                    {"path": str(path), "sha256": file_sha256(path)}
                    for path in sorted(libraries.iterdir())
                ],
            )

    def test_schema5_rejects_runtime_library_content_and_membership_drift(self) -> None:
        # Arrange
        for drift in ("content", "extra", "missing"):
            with self.subTest(drift=drift), TemporaryDirectory() as directory:
                root = Path(directory)
                provenance, provenance_path, _, package = _fixture(
                    root,
                    schema2=True,
                    schema5=True,
                    launch_mode="owned",
                    source_package=True,
                )
                libraries = package.parent / "shuozeli_pwright.dylibs"
                crypto = libraries / "libcrypto-a12805a1.3.dylib"
                if drift == "content":
                    crypto.write_bytes(b"changed")
                    message = "SHA-256 mismatch"
                elif drift == "extra":
                    (libraries / "libextra.dylib").write_bytes(b"extra")
                    message = "library set differs"
                else:
                    crypto.unlink()
                    message = "library set differs"

                # Act and Assert
                with self.assertRaisesRegex(ProvenanceValidationError, message):
                    verify_provenance(
                        provenance,
                        provenance_path=provenance_path,
                        tooling_root=root,
                        check_checkout=False,
                    )

    def test_schema5_rejects_relabelled_or_symlinked_runtime_library(self) -> None:
        # Arrange
        for drift in ("path", "symlink"):
            with self.subTest(drift=drift), TemporaryDirectory() as directory:
                root = Path(directory)
                provenance, provenance_path, _, package = _fixture(
                    root,
                    schema2=True,
                    schema5=True,
                    launch_mode="owned",
                    source_package=True,
                )
                libraries = package.parent / "shuozeli_pwright.dylibs"
                if drift == "path":
                    records = cast(list[dict[str, str]], provenance["pwrightRuntimeLibraries"])
                    records[0]["path"] = str(root / "unrelated/libcrypto.dylib")
                    message = "wrong adjacent path"
                else:
                    crypto = libraries / "libcrypto-a12805a1.3.dylib"
                    outside = root / "outside.dylib"
                    outside.write_bytes(crypto.read_bytes())
                    crypto.unlink()
                    crypto.symlink_to(outside)
                    message = "not a regular adjacent dylib"

                # Act and Assert
                with self.assertRaisesRegex(ProvenanceValidationError, message):
                    verify_provenance(
                        provenance,
                        provenance_path=provenance_path,
                        tooling_root=root,
                        check_checkout=False,
                    )

    def test_schema5_archive_requires_exact_sibling_runtime_libraries(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, live_package = _fixture(
                root,
                schema2=True,
                schema5=True,
                launch_mode="owned",
                source_package=True,
            )
            archived_package = root / "archive/pwright"
            archived_libraries = archived_package.parent / "shuozeli_pwright.dylibs"
            shutil.copytree(live_package, archived_package)
            shutil.copytree(live_package.parent / "shuozeli_pwright.dylibs", archived_libraries)
            (live_package / "__init__.py").write_text("changed\n")
            (live_package.parent / "shuozeli_pwright.dylibs/libssl-ffd8ac69.3.dylib").write_bytes(
                b"changed"
            )

            # Act
            verified = verify_provenance(
                provenance,
                provenance_path=provenance_path,
                tooling_root=root,
                package_root=archived_package,
                check_checkout=False,
            )

            # Assert
            self.assertEqual(verified["schema"], 5)
            self.assertEqual(len(cast(list[object], verified["pwrightRuntimeLibraries"])), 2)

            # Act and Assert: an archive with an altered sibling is not equivalent.
            (archived_libraries / "libssl-ffd8ac69.3.dylib").write_bytes(b"changed")
            with self.assertRaisesRegex(ProvenanceValidationError, "SHA-256 mismatch"):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    package_root=archived_package,
                    check_checkout=False,
                )

    def test_schema5_archive_rejects_missing_sibling_library_directory(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, live_package = _fixture(
                root,
                schema2=True,
                schema5=True,
                launch_mode="owned",
                source_package=True,
            )
            archived_package = root / "archive/pwright"
            shutil.copytree(live_package, archived_package)

            # Act and Assert
            with self.assertRaisesRegex(ProvenanceValidationError, "library set differs"):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    package_root=archived_package,
                    check_checkout=False,
                )

    def test_schema5_oracle_requires_an_empty_runtime_library_list(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, _ = _fixture(
                root, schema2=True, schema5=True, launch_mode="owned"
            )
            provenance["mode"] = "oracle"
            for key in (
                "pwrightPackage",
                "pwrightPackageSha256",
                "pwrightArtifact",
                "pwrightArtifactSha256",
            ):
                provenance[key] = None
            provenance["pwrightRuntimeLibraries"] = []
            provenance["harnessInputs"] = cast(list[object], provenance["harnessInputs"])[1:]
            command = cast(list[str], provenance["command"])
            candidate_plugin = command.index("pwright_candidate_runtime")
            command[candidate_plugin] = "pwright_official_oracle_runtime"

            # Act
            verified = verify_provenance(
                provenance,
                provenance_path=provenance_path,
                tooling_root=root,
                check_checkout=False,
            )

            # Assert
            self.assertEqual(verified["pwrightRuntimeLibraries"], [])

            # Act and Assert: oracle cannot claim candidate-owned libraries.
            provenance["pwrightRuntimeLibraries"] = [
                {"path": str(root / "libssl.dylib"), "sha256": "0" * 64}
            ]
            with self.assertRaisesRegex(ProvenanceValidationError, "oracle provenance"):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    check_checkout=False,
                )

    def test_schema5_requires_runtime_library_field(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, _ = _fixture(
                root, schema2=True, schema5=True, launch_mode="owned"
            )
            del provenance["pwrightRuntimeLibraries"]

            # Act and Assert
            with self.assertRaisesRegex(ProvenanceValidationError, "fields differ"):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    check_checkout=False,
                )

    def test_schema4_owned_launch_verifies_without_cdp_endpoint(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, _ = _fixture(root, schema2=True, launch_mode="owned")

            # Act
            verified = verify_provenance(
                provenance,
                provenance_path=provenance_path,
                tooling_root=root,
                check_checkout=False,
            )

            # Assert
            self.assertEqual(verified["schema"], 4)
            self.assertEqual(verified["launchMode"], "owned")
            self.assertIsNone(verified["endpoint"])

    def test_schema4_rejects_launch_mode_relabeling_and_junit_drift(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, result, _ = _fixture(
                root, schema2=True, launch_mode="owned"
            )

            # Act and Assert
            provenance["launchMode"] = "cdp-adapter"
            with self.assertRaisesRegex(ProvenanceValidationError, "endpoint"):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    check_checkout=False,
                )
            provenance["endpoint"] = "http://127.0.0.1:9222"
            with self.assertRaisesRegex(ProvenanceValidationError, "result launch mode differs"):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    check_checkout=False,
                )
            provenance["launchMode"] = "owned"
            provenance["endpoint"] = None
            result.write_text('<testsuite tests="1"><testcase name="test_example"/></testsuite>\n')
            provenance["resultSha256"] = file_sha256(result)
            with self.assertRaisesRegex(ProvenanceValidationError, "result launch mode differs"):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    check_checkout=False,
                )

    def test_schema4_adapter_launch_verifies_as_distinct_mode(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, _ = _fixture(
                root, schema2=True, launch_mode="cdp-adapter"
            )

            # Act
            verified = verify_provenance(
                provenance,
                provenance_path=provenance_path,
                tooling_root=root,
                check_checkout=False,
            )

            # Assert
            self.assertEqual(verified["schema"], 4)
            self.assertEqual(verified["launchMode"], "cdp-adapter")
            self.assertEqual(verified["endpoint"], "http://127.0.0.1:9222")

    def test_historical_harness_can_be_verified_from_exact_archive(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, _ = _fixture(root, schema2=True)
            archive = root / "archive"
            archive.mkdir()
            shutil.copytree(root / "packages/pwright-python/tests/official/shim", archive / "shim")
            shutil.copytree(
                root / "packages/pwright-python/tests/official/runtime",
                archive / "runtime",
            )
            (
                root / "packages/pwright-python/tests/official/shim/pwright_candidate_runtime.py"
            ).write_text("changed\n")

            # Act
            verified = verify_provenance(
                provenance,
                provenance_path=provenance_path,
                tooling_root=root,
                harness_root=archive,
                check_checkout=False,
            )

            # Assert
            self.assertEqual(verified["schema"], 2)

    def test_historical_candidate_shim_package_uses_exact_archive(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, _ = _fixture(root, schema2=True)
            shim = root / "packages/pwright-python/tests/official/shim"
            shim_package = shim / "playwright"
            shim_package.mkdir()
            (shim_package / "__init__.py").write_text("old\n")
            provenance["playwrightPackage"] = str(shim_package)
            provenance["playwrightPackageSha256"] = tree_sha256(shim_package)
            harness_inputs = cast(list[dict[str, str]], provenance["harnessInputs"])
            harness_inputs[0]["sha256"] = tree_sha256(shim)
            archive = root / "archive"
            archive.mkdir()
            shutil.copytree(shim, archive / "shim")
            shutil.copytree(
                root / "packages/pwright-python/tests/official/runtime",
                archive / "runtime",
            )
            (shim_package / "__init__.py").write_text("new\n")

            # Act
            verified = verify_provenance(
                provenance,
                provenance_path=provenance_path,
                tooling_root=root,
                harness_root=archive,
                check_checkout=False,
            )

            # Assert
            self.assertEqual(
                verified["playwrightPackageSha256"],
                tree_sha256(archive / "shim/playwright"),
            )

    def test_historical_candidate_package_uses_exact_archive_and_artifact(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, live_package = _fixture(
                root, schema2=True, source_package=True
            )
            archive = root / "archive/pwright"
            shutil.copytree(live_package, archive)
            (live_package / "__init__.py").write_text("changed\n")
            (live_package / "_native.abi3.so").write_bytes(b"changed native")

            # Act
            verified = verify_provenance(
                provenance,
                provenance_path=provenance_path,
                tooling_root=root,
                package_root=archive,
                check_checkout=False,
            )

            # Assert
            self.assertEqual(verified["pwrightPackageSha256"], tree_sha256(archive))
            self.assertEqual(
                verified["pwrightArtifactSha256"],
                file_sha256(archive / "_native.abi3.so"),
            )

    def test_historical_candidate_package_rejects_mismatched_archive(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, live_package = _fixture(
                root, schema2=True, source_package=True
            )
            archive = root / "archive/pwright"
            shutil.copytree(live_package, archive)
            (archive / "__init__.py").write_text("not captured\n")

            # Act and Assert
            with self.assertRaisesRegex(
                ProvenanceValidationError, "pwright package SHA-256 mismatch"
            ):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    package_root=archive,
                    check_checkout=False,
                )

    def test_historical_candidate_package_rechecks_original_artifact_digest(
        self,
    ) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, live_package = _fixture(
                root, schema2=True, source_package=True
            )
            archive = root / "archive/pwright"
            shutil.copytree(live_package, archive)
            (archive / "_native.abi3.so").write_bytes(b"not captured")
            provenance["pwrightPackageSha256"] = tree_sha256(archive)

            # Act and Assert
            with self.assertRaisesRegex(
                ProvenanceValidationError, "pwright artifact SHA-256 mismatch"
            ):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    package_root=archive,
                    check_checkout=False,
                )

    def test_historical_candidate_package_rejects_unrecorded_source_root(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, live_package = _fixture(root, schema2=True)
            archive = root / "archive/pwright"
            shutil.copytree(live_package, archive)

            # Act and Assert
            with self.assertRaisesRegex(ProvenanceValidationError, "exact recorded source package"):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    package_root=archive,
                    check_checkout=False,
                )

    def test_historical_candidate_package_rejects_artifact_outside_recorded_root(
        self,
    ) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, live_package = _fixture(
                root, schema2=True, source_package=True
            )
            archive = root / "archive/pwright"
            shutil.copytree(live_package, archive)
            provenance["pwrightArtifact"] = str(root / "unrelated/_native.abi3.so")

            # Act and Assert
            with self.assertRaisesRegex(ProvenanceValidationError, "inside the recorded package"):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    package_root=archive,
                    check_checkout=False,
                )

    def test_historical_candidate_package_rejects_symlink_root(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, live_package = _fixture(
                root, schema2=True, source_package=True
            )
            archive = root / "archive/pwright"
            shutil.copytree(live_package, archive)
            linked_root = root / "linked-pwright"
            linked_root.symlink_to(archive, target_is_directory=True)

            # Act and Assert
            with self.assertRaisesRegex(ProvenanceValidationError, "symlink"):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    package_root=linked_root,
                    check_checkout=False,
                )

    def test_historical_candidate_package_rejects_symlink_escape(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, live_package = _fixture(
                root, schema2=True, source_package=True
            )
            archive = root / "archive/pwright"
            shutil.copytree(live_package, archive)
            outside = root / "outside.py"
            outside.write_text("outside\n")
            (archive / "escape.py").symlink_to(outside)

            # Act and Assert
            with self.assertRaisesRegex(ProvenanceValidationError, "symlink or escape"):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    package_root=archive,
                    check_checkout=False,
                )

    def test_historical_candidate_package_override_rejects_oracle_mode(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, live_package = _fixture(
                root, schema2=True, source_package=True
            )
            archive = root / "archive/pwright"
            shutil.copytree(live_package, archive)
            provenance["mode"] = "oracle"

            # Act and Assert
            with self.assertRaisesRegex(ProvenanceValidationError, "only valid for candidate"):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    package_root=archive,
                    check_checkout=False,
                )

    def test_schema3_executable_diagnostic_verifies_without_browser_launch(
        self,
    ) -> None:
        # Arrange
        for mode in ("candidate", "oracle"):
            with self.subTest(mode=mode), TemporaryDirectory() as directory:
                root = Path(directory)
                provenance, provenance_path = _registry_fixture(root, mode=mode)

                # Act
                verified = verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    check_checkout=False,
                )

                # Assert
                self.assertEqual(verified["schema"], 3)
                self.assertEqual(verified["mode"], mode)
                registry = cast(dict[str, object], verified["browserRegistry"])
                self.assertEqual(_registry_executables(registry)[0]["revision"], "1208")

    def test_schema3_rejects_changed_environment_and_executable_evidence(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path = _registry_fixture(root)
            original = cast(dict[str, object], copy.deepcopy(provenance["browserRegistry"]))

            # Act and Assert
            for key, value in (
                ("registryRoot", str(root / "wrong-cache")),
                ("browsersJsonSha256", "0" * 64),
            ):
                with self.subTest(key=key):
                    altered = copy.deepcopy(original)
                    altered[key] = value
                    provenance["browserRegistry"] = altered
                    with self.assertRaisesRegex(
                        ProvenanceValidationError, "browserRegistry differs"
                    ):
                        verify_provenance(
                            provenance,
                            provenance_path=provenance_path,
                            tooling_root=root,
                            check_checkout=False,
                        )
            altered = copy.deepcopy(original)
            _registry_executables(altered)[0]["exists"] = not _registry_executables(altered)[0][
                "exists"
            ]
            provenance["browserRegistry"] = altered
            with self.assertRaisesRegex(ProvenanceValidationError, "browserRegistry differs"):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    check_checkout=False,
                )
            altered = copy.deepcopy(original)
            _registry_environment(altered)["PLAYWRIGHT_BROWSERS_PATH"] = "changed-cache"
            provenance["browserRegistry"] = altered
            with self.assertRaisesRegex(ProvenanceValidationError, "browserRegistry differs"):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    check_checkout=False,
                )
            altered = copy.deepcopy(original)
            _registry_executables(altered)[0]["revision"] = "1234"
            provenance["browserRegistry"] = altered
            with self.assertRaisesRegex(ProvenanceValidationError, "browserRegistry differs"):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    check_checkout=False,
                )

    def test_schema3_rejects_current_environment_drift_after_capture(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            with patch.dict(os.environ, {"PLAYWRIGHT_BROWSERS_PATH": str(root / "captured-cache")}):
                provenance, provenance_path = _registry_fixture(root)

            # Act and Assert
            with (
                patch.dict(os.environ, {"PLAYWRIGHT_BROWSERS_PATH": str(root / "later-cache")}),
                self.assertRaisesRegex(ProvenanceValidationError, "browserRegistry differs"),
            ):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    check_checkout=False,
                )

    def test_schema3_rejects_unselected_manifest(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, _ = _fixture(root, schema2=True)
            provenance["schema"] = 3
            provenance["browserRegistry"] = {}

            # Act and Assert
            with self.assertRaisesRegex(
                ProvenanceValidationError, "limited to pinned executable_path"
            ):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    check_checkout=False,
                )

    def test_schema3_rejects_success_with_missing_executable(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            with patch.dict(os.environ, {"PLAYWRIGHT_BROWSERS_PATH": str(root / "empty-cache")}):
                provenance, provenance_path = _registry_fixture(root)
                registry = cast(dict[str, object], provenance["browserRegistry"])
                self.assertFalse(
                    cast(list[dict[str, object]], registry["executables"])[0]["exists"]
                )
                provenance["exitCode"] = 0

                # Act and Assert
                with self.assertRaisesRegex(ProvenanceValidationError, "missing executable"):
                    verify_provenance(
                        provenance,
                        provenance_path=provenance_path,
                        tooling_root=root,
                        check_checkout=False,
                    )

    def test_schema2_candidate_verifies_archived_tooling(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, _ = _fixture(root, schema2=True)

            # Act
            verified = verify_provenance(
                provenance,
                provenance_path=provenance_path,
                tooling_root=root,
                check_checkout=False,
            )

            # Assert
            self.assertEqual(verified["schema"], 2)
            self.assertEqual(len(cast(list[dict[str, str]], verified["toolingSources"])), 4)

    def test_schema2_profile_and_loop_variants_verify(self) -> None:
        # Arrange and Act and Assert
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, _ = _fixture(root, profile=True, schema2=True)
            verified = verify_provenance(
                provenance,
                provenance_path=provenance_path,
                tooling_root=root,
                check_checkout=False,
            )
            self.assertEqual(verified["schema"], 2)
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _ = _loop_fixture(root, schema2=True)
            verified = verify_provenance(
                provenance,
                provenance_path=provenance_path,
                tooling_root=root,
                check_checkout=False,
            )
            self.assertEqual(verified["executionMode"], SYNC_LIFECYCLE_LOOP_MODE)

    def test_schema2_rejects_tooling_drift_even_without_checkout_check(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, _ = _fixture(root, schema2=True)
            (root / "scripts/check_playwright_python_official_source.py").write_text("changed\n")

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

    def test_schema2_rejects_missing_reordered_or_extra_records(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            provenance, provenance_path, _, _ = _fixture(root, schema2=True)
            original = cast(list[dict[str, str]], copy.deepcopy(provenance["toolingSources"]))

            # Act and Assert
            for invalid in (
                original[:3],
                list(reversed(original)),
                original + [original[0]],
            ):
                with self.subTest(invalid=invalid):
                    provenance["toolingSources"] = invalid
                    with self.assertRaisesRegex(ProvenanceValidationError, "toolingSources"):
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
            provenance, provenance_path, _, _ = _fixture(root, schema2=True)
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
            records[0]["sha256"] = tooling_sources(root)[0]["sha256"]
            (root / records[0]["path"]).unlink()
            with self.assertRaisesRegex(ProvenanceValidationError, "missing or a symlink"):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    tooling_root=root,
                    check_checkout=False,
                )

    def test_schema1_rejects_unversioned_tooling_field(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            provenance, provenance_path, _, _ = _fixture(Path(directory))
            provenance["toolingSources"] = []

            # Act and Assert
            with self.assertRaisesRegex(ProvenanceValidationError, "fields differ"):
                verify_provenance(provenance, provenance_path=provenance_path, check_checkout=False)

    def test_pinned_sync_lifecycle_loop_sidecar_verifies_exact_six_rows(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            provenance, provenance_path, _ = _loop_fixture(Path(directory))

            # Act
            verified = verify_provenance(
                provenance, provenance_path=provenance_path, check_checkout=False
            )

            # Assert
            self.assertEqual(verified["executionMode"], SYNC_LIFECYCLE_LOOP_MODE)

    def test_sync_lifecycle_loop_wrapper_digest_tampering_is_rejected(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            provenance, provenance_path, _ = _loop_fixture(Path(directory))
            provenance["executionWrapperSha256"] = "0" * 64

            # Act and Assert
            with self.assertRaisesRegex(ProvenanceValidationError, "wrapper source"):
                verify_provenance(provenance, provenance_path=provenance_path, check_checkout=False)

    def test_sync_lifecycle_loop_command_tampering_is_rejected(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            provenance, provenance_path, _ = _loop_fixture(Path(directory))
            command = list(cast(list[str], provenance["command"]))
            command[2] = command[2].replace("asyncio.run(main())", "main()")
            provenance["command"] = command

            # Act and Assert
            with self.assertRaisesRegex(ProvenanceValidationError, "command does not match"):
                verify_provenance(provenance, provenance_path=provenance_path, check_checkout=False)

    def test_sync_lifecycle_loop_mode_tampering_is_rejected(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            provenance, provenance_path, _ = _loop_fixture(Path(directory))
            provenance["executionMode"] = "ordinary-pytest"

            # Act and Assert
            with self.assertRaisesRegex(ProvenanceValidationError, "executionMode differs"):
                verify_provenance(provenance, provenance_path=provenance_path, check_checkout=False)

    def test_sync_lifecycle_loop_rejects_missing_result_row(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            provenance, provenance_path, result = _loop_fixture(Path(directory))
            result.write_text(
                result.read_text().replace(
                    '<testcase classname="tests.sync.test_sync" '
                    'name="test_sync_playwright_multiple_times[chromium]"/>',
                    "",
                )
            )
            provenance["resultSha256"] = file_sha256(result)

            # Act and Assert
            with self.assertRaisesRegex(ProvenanceValidationError, "exactly six pinned rows"):
                verify_provenance(provenance, provenance_path=provenance_path, check_checkout=False)

    def test_generated_profile_candidate_sidecar_verifies(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            provenance, provenance_path, _, _ = _fixture(Path(directory), profile=True)

            # Act
            verified = verify_provenance(
                provenance,
                provenance_path=provenance_path,
                check_checkout=False,
            )

            # Assert
            self.assertEqual(verified["scope"], "profile-examples")
            self.assertIsNone(verified["fixtureBind"])

    def test_generated_candidate_sidecar_verifies(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            provenance, provenance_path, _, _ = _fixture(Path(directory))

            # Act
            verified = verify_provenance(
                provenance,
                provenance_path=provenance_path,
                check_checkout=False,
            )

            # Assert
            self.assertEqual(verified["mode"], "candidate")
            self.assertEqual(verified["scope"], "selected")

    def test_generated_browserless_candidate_sidecar_verifies(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            provenance, provenance_path, _, _ = _fixture(Path(directory), browserless=True)

            # Act
            verified = verify_provenance(
                provenance,
                provenance_path=provenance_path,
                check_checkout=False,
            )

            # Assert
            self.assertEqual(verified["mode"], "candidate")
            self.assertEqual(verified["scope"], "manifest-cases-all")

    def test_result_tampering_is_rejected(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            provenance, provenance_path, result, _ = _fixture(Path(directory))
            result.write_text('<testsuite tests="0"/>\n')

            # Act and Assert
            with self.assertRaisesRegex(ProvenanceValidationError, "result SHA-256 mismatch"):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    check_checkout=False,
                )

    def test_candidate_package_tampering_is_rejected(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            provenance, provenance_path, _, pwright_package = _fixture(Path(directory))
            (pwright_package / "__init__.py").write_text("tampered = True\n")

            # Act and Assert
            with self.assertRaisesRegex(
                ProvenanceValidationError, "pwright package SHA-256 mismatch"
            ):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    check_checkout=False,
                )

    def test_command_drift_is_rejected(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            provenance, provenance_path, _, _ = _fixture(Path(directory))
            command = list(cast(list[str], provenance["command"]))
            command.append("--headed")
            provenance["command"] = command

            # Act and Assert
            with self.assertRaisesRegex(ProvenanceValidationError, "command does not match"):
                verify_provenance(
                    provenance,
                    provenance_path=provenance_path,
                    check_checkout=False,
                )


if __name__ == "__main__":
    unittest.main()
