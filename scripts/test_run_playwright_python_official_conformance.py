#!/usr/bin/env python3
"""Tests for guarded Playwright Python conformance inputs."""

from __future__ import annotations

import copy
import hashlib
import json
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from typing import TypedDict, cast
from unittest.mock import patch
from xml.etree import ElementTree as ET

from playwright_python_official_provenance import tree_sha256
from run_playwright_python_official_conformance import (
    Manifest,
    _active_cdp_pages,  # pyright: ignore[reportPrivateUsage]
    _collection_profile,  # pyright: ignore[reportPrivateUsage]
    _execution_requirements,  # pyright: ignore[reportPrivateUsage]
    _isolated_cases,  # pyright: ignore[reportPrivateUsage]
    _persistent_page_argument_browserless_mode,  # pyright: ignore[reportPrivateUsage]
    _proxy_bad_server_browserless_mode,  # pyright: ignore[reportPrivateUsage]
    _require_persistent_preflight_harness_sources,  # pyright: ignore[reportPrivateUsage]
    _require_persistent_preflight_runtime_sources,  # pyright: ignore[reportPrivateUsage]
    _require_proxy_preflight_runtime_sources,  # pyright: ignore[reportPrivateUsage]
    _run_isolated_cases,  # pyright: ignore[reportPrivateUsage]
    _selection_is_in_profile,  # pyright: ignore[reportPrivateUsage]
    _sync_lifecycle_loop_mode,  # pyright: ignore[reportPrivateUsage]
    _validate_loopback_endpoint,  # pyright: ignore[reportPrivateUsage]
    _validate_server_prefix,  # pyright: ignore[reportPrivateUsage]
    _validate_tailscale_fixture,  # pyright: ignore[reportPrivateUsage]
    _write_single_case,  # pyright: ignore[reportPrivateUsage]
)


class PersistentPreflightOptions(TypedDict):
    manifest_cases: bool
    manifest_api: str | None
    browsers: list[str]
    browser_channel: str | None
    launch_mode: str
    endpoint: str | None
    isolate_cases: bool
    collect_only: bool
    case_timeout: float | None
    server_prefix: str | None
    secure_http_fixture: bool


class PythonOfficialRunnerInputTests(unittest.TestCase):
    def test_proxy_preflight_rejects_changed_runtime_source(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            package = Path(directory) / "pwright"
            package.mkdir()
            (package / "async_api.py").write_text("changed launch implementation\n")

            # Act and Assert
            with self.assertRaisesRegex(ValueError, "package tree drifted"):
                _require_proxy_preflight_runtime_sources("candidate", package)
            with patch(
                "run_playwright_python_official_conformance.tree_sha256",
                return_value=("96ec860880d03b2a09432a7ec1aec14de3598aac5327735be25b8641742cbf4f"),
            ):
                with self.assertRaisesRegex(ValueError, "source drifted: async_api.py"):
                    _require_proxy_preflight_runtime_sources("candidate", package)

    def test_proxy_preflight_accepts_only_three_owned_async_engines(self) -> None:
        # Arrange
        manifest_path = (
            Path(__file__).parents[1]
            / "packages/pwright-python/tests/official/proxy-bad-server-browserless-manifest.json"
        )
        manifest = cast(Manifest, json.loads(manifest_path.read_text()))
        arguments = cast(
            PersistentPreflightOptions,
            dict(
                manifest_cases=True,
                manifest_api="async",
                browsers=["chromium", "firefox", "webkit"],
                browser_channel=None,
                launch_mode="owned",
                endpoint=None,
                isolate_cases=False,
                collect_only=False,
                case_timeout=30.0,
                server_prefix=None,
                secure_http_fixture=False,
            ),
        )

        # Act
        accepted = _proxy_bad_server_browserless_mode(manifest, **arguments)

        # Assert
        self.assertTrue(accepted)

    def test_proxy_preflight_rejects_browser_and_pytest_scope_expansion(self) -> None:
        # Arrange
        manifest_path = (
            Path(__file__).parents[1]
            / "packages/pwright-python/tests/official/proxy-bad-server-browserless-manifest.json"
        )
        manifest = cast(Manifest, json.loads(manifest_path.read_text()))
        arguments = cast(
            PersistentPreflightOptions,
            dict(
                manifest_cases=True,
                manifest_api="async",
                browsers=["chromium", "firefox", "webkit"],
                browser_channel=None,
                launch_mode="owned",
                endpoint=None,
                isolate_cases=False,
                collect_only=False,
                case_timeout=30.0,
                server_prefix=None,
                secure_http_fixture=False,
            ),
        )
        invalid: tuple[dict[str, object], ...] = (
            {"manifest_cases": False},
            {"manifest_api": "sync"},
            {"browsers": ["chromium"]},
            {"browsers": ["chromium", "webkit", "firefox"]},
            {"browsers": []},
            {"browser_channel": "chrome"},
            {"launch_mode": "cdp-adapter"},
            {"endpoint": "http://127.0.0.1:9222"},
            {"isolate_cases": True},
            {"collect_only": True},
            {"case_timeout": None},
            {"server_prefix": "https://example.ts.net"},
            {"secure_http_fixture": True},
        )

        # Act and Assert
        for change in invalid:
            with self.subTest(change=change):
                options = cast(PersistentPreflightOptions, arguments | change)
                with self.assertRaisesRegex(ValueError, "requires exactly the three"):
                    _proxy_bad_server_browserless_mode(manifest, **options)

    def test_persistent_preflight_rejects_changed_runtime_and_harness_sources(
        self,
    ) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            package = Path(directory) / "pwright"
            package.mkdir()
            (package / "async_api.py").write_text("changed preflight implementation\n")
            official = Path(directory) / "official"
            (official / "runtime").mkdir(parents=True)

            # Act and Assert
            with self.assertRaisesRegex(ValueError, "package tree drifted"):
                _require_persistent_preflight_runtime_sources("candidate", package)
            with patch(
                "run_playwright_python_official_conformance.tree_sha256",
                return_value=("7da9aab756d4f880e7789b652e1a3b171455eac02c58d99364157b4a44a6097b"),
            ):
                with self.assertRaisesRegex(ValueError, "source drifted: async_api.py"):
                    _require_persistent_preflight_runtime_sources("candidate", package)
            with self.assertRaisesRegex(ValueError, "harness source drifted: runtime"):
                _require_persistent_preflight_harness_sources(official, "oracle")

    def test_persistent_preflight_accepts_only_explicit_owned_webkit_or_firefox(
        self,
    ) -> None:
        # Arrange
        manifest_path = (
            Path(__file__).parents[1]
            / "packages/pwright-python/tests/official"
            / "persistent-context-page-argument-browserless-manifest.json"
        )
        manifest = cast(Manifest, json.loads(manifest_path.read_text()))
        arguments = cast(
            PersistentPreflightOptions,
            dict(
                manifest_cases=True,
                manifest_api="async",
                browsers=["webkit"],
                browser_channel=None,
                launch_mode="owned",
                endpoint=None,
                isolate_cases=False,
                collect_only=False,
                case_timeout=30.0,
                server_prefix=None,
                secure_http_fixture=False,
            ),
        )

        # Act and Assert
        for engine in ("webkit", "firefox"):
            with self.subTest(engine=engine):
                options = cast(PersistentPreflightOptions, arguments | {"browsers": [engine]})
                self.assertTrue(_persistent_page_argument_browserless_mode(manifest, **options))

    def test_persistent_preflight_rejects_browser_and_pytest_scope_expansion(
        self,
    ) -> None:
        # Arrange
        manifest_path = (
            Path(__file__).parents[1]
            / "packages/pwright-python/tests/official"
            / "persistent-context-page-argument-browserless-manifest.json"
        )
        manifest = cast(Manifest, json.loads(manifest_path.read_text()))
        arguments = cast(
            PersistentPreflightOptions,
            dict(
                manifest_cases=True,
                manifest_api="async",
                browsers=["webkit"],
                browser_channel=None,
                launch_mode="owned",
                endpoint=None,
                isolate_cases=False,
                collect_only=False,
                case_timeout=30.0,
                server_prefix=None,
                secure_http_fixture=False,
            ),
        )
        invalid: tuple[dict[str, object], ...] = (
            {"manifest_cases": False},
            {"manifest_api": "sync"},
            {"browsers": ["chromium"]},
            {"browsers": ["webkit", "firefox"]},
            {"browsers": []},
            {"browser_channel": "chrome"},
            {"launch_mode": "cdp-adapter"},
            {"endpoint": "http://127.0.0.1:9222"},
            {"isolate_cases": True},
            {"collect_only": True},
            {"case_timeout": None},
            {"server_prefix": "https://example.ts.net"},
            {"secure_http_fixture": True},
        )

        # Act and Assert
        for change in invalid:
            with self.subTest(change=change):
                options = cast(PersistentPreflightOptions, arguments | change)
                with self.assertRaisesRegex(ValueError, "requires one WebKit or Firefox"):
                    _persistent_page_argument_browserless_mode(manifest, **options)

    def test_persistent_preflight_rejects_manifest_policy_drift(self) -> None:
        # Arrange
        manifest_path = (
            Path(__file__).parents[1]
            / "packages/pwright-python/tests/official"
            / "persistent-context-page-argument-browserless-manifest.json"
        )
        manifest = cast(Manifest, json.loads(manifest_path.read_text()))
        changed = copy.deepcopy(manifest)
        changed["cases"] = []

        # Act and Assert
        with self.assertRaisesRegex(ValueError, "exact selection"):
            _persistent_page_argument_browserless_mode(
                changed,
                manifest_cases=True,
                manifest_api="async",
                browsers=["webkit"],
                browser_channel=None,
                launch_mode="owned",
                endpoint=None,
                isolate_cases=False,
                collect_only=False,
                case_timeout=30.0,
                server_prefix=None,
                secure_http_fixture=False,
            )

    def test_isolated_resume_rebuilds_complete_aggregate_from_validated_cases(
        self,
    ) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            result = root / "result.xml"
            result.write_text("old unrelated aggregate")
            node_id = "tests/test_item.py::test_item"
            cases = root / "result.xml.cases"
            cases.mkdir()
            case_path = cases / f"00001-{hashlib.sha256(node_id.encode()).hexdigest()[:16]}.xml"
            _write_single_case(
                ET.Element("testcase", {"classname": "tests.test_item", "name": "test_item"}),
                case_path,
            )
            with patch("run_playwright_python_official_conformance.subprocess.Popen") as child:
                # Act
                exit_code = _run_isolated_cases([], [node_id], root, root, {}, result, 1, True)

                # Assert
                child.assert_not_called()
            self.assertEqual(exit_code, 0)
            report_cases = list(ET.parse(result).getroot().iter("testcase"))
            self.assertEqual(len(report_cases), 1)
            self.assertEqual(report_cases[0].get("name"), "test_item")

    def test_isolated_empty_or_duplicate_selection_cannot_reuse_old_aggregate(
        self,
    ) -> None:
        for selection in ([], ["tests/test_item.py::test_item"] * 2):
            with self.subTest(selection=selection), TemporaryDirectory() as directory:
                # Arrange
                root = Path(directory)
                result = root / "result.xml"
                result.write_text("old aggregate")
                with patch("run_playwright_python_official_conformance.subprocess.Popen") as child:
                    # Act and Assert
                    with self.assertRaisesRegex(SystemExit, "non-empty unique"):
                        _run_isolated_cases([], selection, root, root, {}, result, 1, True)
                    child.assert_not_called()
                self.assertEqual(result.read_text(), "old aggregate")

    def test_invalid_resume_report_cannot_complete_or_replace_old_aggregate(
        self,
    ) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            result = root / "result.xml"
            result.write_text("old aggregate")
            node_id = "tests/test_item.py::test_item"
            cases = root / "result.xml.cases"
            cases.mkdir()
            case_path = cases / f"00001-{hashlib.sha256(node_id.encode()).hexdigest()[:16]}.xml"
            case_path.write_text("not valid XML")
            with patch("run_playwright_python_official_conformance.subprocess.Popen") as child:
                # Act and Assert
                with self.assertRaisesRegex(SystemExit, "invalid resumable isolated report"):
                    _run_isolated_cases([], [node_id], root, root, {}, result, 1, True)
                child.assert_not_called()
            self.assertEqual(result.read_text(), "old aggregate")

    def test_fresh_multi_testcase_report_preserves_raw_bytes_without_pass_credit(
        self,
    ) -> None:
        node_id = "tests/test_item.py::test_item"
        cases = (
            b'<testsuites><testsuite><testcase classname="tests.test_item" name="test_item"/>'
            b'<testcase classname="tests.test_item" name="test_item"/></testsuite></testsuites>',
            b'<testsuites><testsuite><testcase classname="tests.test_item" name="test_item"/>'
            b'<testcase classname="tests.test_item" name="other_item"/></testsuite></testsuites>',
        )
        for raw_report in cases:
            with self.subTest(raw_report=raw_report), TemporaryDirectory() as directory:
                # Arrange
                root = Path(directory)
                result = root / "result.xml"

                class CompletedChild:
                    def wait(self, timeout: float) -> int:
                        return 0

                def launch(command: list[str], **_kwargs: object) -> CompletedChild:
                    Path(command[-1].removeprefix("--junitxml=")).write_bytes(raw_report)
                    return CompletedChild()

                with patch(
                    "run_playwright_python_official_conformance.subprocess.Popen",
                    side_effect=launch,
                ):
                    # Act
                    exit_code = _run_isolated_cases([], [node_id], root, root, {}, result, 1, False)

                # Assert
                case_root = root / "result.xml.cases"
                case_path = next(case_root.glob("*.xml"))
                self.assertEqual(len(list(case_root.glob("*.xml"))), 1)
                raw_path = case_path.with_name(f"{case_path.name}.invalid.raw")
                self.assertEqual(raw_path.read_bytes(), raw_report)
                self.assertEqual(exit_code, 1)
                aggregate = ET.parse(result).getroot()
                self.assertEqual(len(list(aggregate.iter("testcase"))), 1)
                suite = aggregate.find(".//testsuite")
                assert suite is not None
                self.assertEqual(suite.get("errors"), "1")
                self.assertEqual(suite.get("failures"), "0")
                error = aggregate.find(".//testcase/error")
                self.assertIsNotNone(error)
                message = error.get("message", "") if error is not None else ""
                self.assertIn(str(raw_path), message)
                self.assertIn(f"sha256={hashlib.sha256(raw_report).hexdigest()}", message)
                bound_digest = tree_sha256(case_root)
                raw_path.write_bytes(raw_report + b"\n")
                self.assertNotEqual(tree_sha256(case_root), bound_digest)

    def test_valid_fresh_isolated_report_does_not_create_raw_artifact(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            result = root / "result.xml"
            node_id = "tests/test_item.py::test_item"
            raw_report = (
                b'<testsuites><testsuite><testcase classname="tests.test_item" '
                b'name="test_item"/></testsuite></testsuites>'
            )

            class CompletedChild:
                def wait(self, timeout: float) -> int:
                    return 0

            def launch(command: list[str], **_kwargs: object) -> CompletedChild:
                Path(command[-1].removeprefix("--junitxml=")).write_bytes(raw_report)
                return CompletedChild()

            with patch(
                "run_playwright_python_official_conformance.subprocess.Popen",
                side_effect=launch,
            ):
                # Act
                exit_code = _run_isolated_cases([], [node_id], root, root, {}, result, 1, False)

            # Assert
            self.assertEqual(exit_code, 0)
            case_root = root / "result.xml.cases"
            self.assertEqual(list(case_root.glob("*.invalid.raw")), [])
            self.assertEqual(len(list(ET.parse(result).getroot().iter("testcase"))), 1)

    def test_pinned_sync_lifecycle_manifest_enables_same_thread_loop(self) -> None:
        # Arrange
        manifest_path = (
            Path(__file__).parents[1]
            / "packages/pwright-python/tests/official/sync-lifecycle-running-loop-manifest.json"
        )
        manifest = cast(Manifest, json.loads(manifest_path.read_text()))

        # Act
        enabled = _sync_lifecycle_loop_mode(
            manifest,
            manifest_cases=True,
            manifest_api="sync",
            browsers=[],
            browser_channel=None,
            isolate_cases=False,
            collect_only=False,
        )
        collection_enabled = _sync_lifecycle_loop_mode(
            manifest,
            manifest_cases=True,
            manifest_api="sync",
            browsers=[],
            browser_channel=None,
            isolate_cases=False,
            collect_only=True,
        )

        # Assert
        self.assertTrue(enabled)
        self.assertFalse(collection_enabled)

    def test_sync_lifecycle_loop_rejects_browser_filter_and_case_drift(self) -> None:
        # Arrange
        manifest_path = (
            Path(__file__).parents[1]
            / "packages/pwright-python/tests/official/sync-lifecycle-running-loop-manifest.json"
        )
        manifest = cast(Manifest, json.loads(manifest_path.read_text()))

        # Act and Assert
        with self.assertRaisesRegex(ValueError, "all pinned sync manifest cases"):
            _sync_lifecycle_loop_mode(
                manifest,
                manifest_cases=True,
                manifest_api="sync",
                browsers=["chromium"],
                browser_channel=None,
                isolate_cases=False,
                collect_only=False,
            )
        manifest["cases"] = manifest["cases"][:1]
        with self.assertRaisesRegex(ValueError, "exactly two pinned lifecycle cases"):
            _sync_lifecycle_loop_mode(
                manifest,
                manifest_cases=True,
                manifest_api="sync",
                browsers=[],
                browser_channel=None,
                isolate_cases=False,
                collect_only=False,
            )

    def test_isolated_directory_selection_keeps_only_matching_browser_rows(
        self,
    ) -> None:
        # Arrange
        rows = [
            '{"file":"examples/todomvc/test_item.py","id":"examples/todomvc/test_item.py::test_item[chromium]"}',
            '{"file":"examples/todomvc/test_item.py","id":"examples/todomvc/test_item.py::test_item[firefox]"}',
            '{"file":"examples/todomvc-other/test_item.py","id":"examples/todomvc-other/test_item.py::test_item[chromium]"}',
        ]
        with TemporaryDirectory() as directory:
            inventory = Path(directory) / "inventory.jsonl"
            inventory.write_text("\n".join(rows) + "\n")

            # Act
            selected = _isolated_cases(inventory, ["examples/todomvc"], ["chromium"])

        # Assert
        self.assertEqual(
            selected,
            ["examples/todomvc/test_item.py::test_item[chromium]"],
        )

    def test_examples_profile_requires_browser_without_core_fixture_servers(
        self,
    ) -> None:
        # Arrange
        manifest = cast(Manifest, {"cases": []})

        # Act
        requirements = _execution_requirements(
            manifest,
            manifest_cases=False,
            collection_profile="examples",
        )

        # Assert
        self.assertEqual(requirements, (True, False))

    def test_collection_profile_is_resolved_from_manifest(self) -> None:
        # Arrange
        manifest = cast(
            Manifest,
            {
                "corpus": {
                    "collectionProfiles": [
                        {
                            "name": "examples",
                            "selection": "examples/todomvc",
                            "expandedTests": 25,
                        }
                    ]
                }
            },
        )

        # Act
        profile = _collection_profile(manifest, "examples")

        # Assert
        self.assertEqual(profile["selection"], "examples/todomvc")
        self.assertEqual(profile["expandedTests"], 25)

    def test_profile_selection_rejects_similar_prefix(self) -> None:
        # Arrange
        profile_selection = "examples/todomvc"

        # Act
        inside = _selection_is_in_profile(
            "examples/todomvc/mvctests/test_item.py::test_item", profile_selection
        )
        outside = _selection_is_in_profile(
            "examples/todomvc-other/test_item.py::test_item", profile_selection
        )

        # Assert
        self.assertTrue(inside)
        self.assertFalse(outside)

    def test_manifest_without_execution_policy_keeps_full_requirements(self) -> None:
        # Arrange
        manifest = cast(Manifest, {"cases": []})

        # Act
        requirements = _execution_requirements(manifest, manifest_cases=False)

        # Assert
        self.assertEqual(requirements, (True, True))

    def test_browserless_manifest_disables_only_browser_requirement(
        self,
    ) -> None:
        # Arrange
        manifest = cast(
            Manifest,
            {
                "execution": {
                    "requiresBrowser": False,
                    "requiresFixtureServers": True,
                },
                "cases": [
                    {
                        "id": "tests/sync/test_sync.py::test_lifecycle",
                        "capabilities": ["playwright.lifecycle.browserless"],
                    }
                ],
            },
        )

        # Act
        requirements = _execution_requirements(manifest, manifest_cases=True)

        # Assert
        self.assertEqual(requirements, (False, True))

    def test_manifest_cannot_disable_upstream_autouse_fixture_servers(self) -> None:
        # Arrange
        manifest = cast(
            Manifest,
            {
                "execution": {
                    "requiresBrowser": False,
                    "requiresFixtureServers": False,
                },
                "cases": [
                    {
                        "id": "tests/sync/test_sync.py::test_lifecycle",
                        "capabilities": ["playwright.lifecycle.browserless"],
                    }
                ],
            },
        )

        # Act and Assert
        with self.assertRaisesRegex(ValueError, "autouse start_server"):
            _execution_requirements(manifest, manifest_cases=True)

    def test_browserless_policy_requires_manifest_case_execution(self) -> None:
        # Arrange
        manifest = cast(
            Manifest,
            {
                "execution": {
                    "requiresBrowser": False,
                    "requiresFixtureServers": True,
                },
                "cases": [
                    {
                        "id": "tests/sync/test_sync.py::test_lifecycle",
                        "capabilities": ["playwright.lifecycle.browserless"],
                    }
                ],
            },
        )

        # Act and Assert
        with self.assertRaisesRegex(ValueError, "requires --manifest-cases"):
            _execution_requirements(manifest, manifest_cases=False)

    def test_browserless_policy_requires_every_case_capability(self) -> None:
        # Arrange
        manifest = cast(
            Manifest,
            {
                "execution": {
                    "requiresBrowser": False,
                    "requiresFixtureServers": True,
                },
                "cases": [
                    {
                        "id": "tests/sync/test_sync.py::test_lifecycle",
                        "capabilities": ["playwright.lifecycle"],
                    }
                ],
            },
        )

        # Act and Assert
        with self.assertRaisesRegex(ValueError, "every browserless"):
            _execution_requirements(manifest, manifest_cases=True)

    def test_blank_cdp_pages_are_ignored(self) -> None:
        # Arrange
        targets = [
            {"type": "page", "url": "about:blank", "title": ""},
            {"type": "page", "url": "chrome://newtab/", "title": "New Tab"},
            {"type": "service_worker", "url": "https://example.test/sw.js"},
        ]

        # Act
        pages = _active_cdp_pages(targets)

        # Assert
        self.assertEqual(pages, [])

    def test_nonblank_cdp_page_is_reported(self) -> None:
        # Arrange
        target = {"type": "page", "url": "https://example.test", "title": "Task"}

        # Act
        pages = _active_cdp_pages([target])

        # Assert
        self.assertEqual(pages, [target])

    def test_loopback_cdp_endpoint_is_accepted(self) -> None:
        # Arrange
        endpoint = "http://127.0.0.1:9222"

        # Act
        actual = _validate_loopback_endpoint(endpoint)

        # Assert
        self.assertEqual(actual, endpoint)

    def test_remote_cdp_endpoint_is_rejected(self) -> None:
        # Arrange
        endpoint = "http://browser-yuacx.example.ts.net:9222"

        # Act and Assert
        with self.assertRaisesRegex(ValueError, "loopback-only"):
            _validate_loopback_endpoint(endpoint)

    def test_tailscale_fixture_is_normalized(self) -> None:
        # Arrange
        ip = "100.64.0.1"
        host = "example-yuacx.tail8f3b66.ts.net."

        # Act
        actual = _validate_tailscale_fixture(ip, host)

        # Assert
        self.assertEqual(actual, (ip, "example-yuacx.tail8f3b66.ts.net"))

    def test_loopback_fixture_bind_is_rejected(self) -> None:
        # Arrange
        ip = "127.0.0.1"
        host = "example-yuacx.tail8f3b66.ts.net"

        # Act and Assert
        with self.assertRaisesRegex(ValueError, "non-loopback"):
            _validate_tailscale_fixture(ip, host)

    def test_non_magicdns_fixture_host_is_rejected(self) -> None:
        # Arrange
        ip = "100.64.0.1"
        host = "example.test"

        # Act and Assert
        with self.assertRaisesRegex(ValueError, "MagicDNS"):
            _validate_tailscale_fixture(ip, host)

    def test_magicdns_server_prefix_is_accepted(self) -> None:
        # Arrange
        prefix = "http://example-yuacx.tail8f3b66.ts.net:8080/assets/"

        # Act
        actual = _validate_server_prefix(prefix)

        # Assert
        self.assertEqual(actual, "http://example-yuacx.tail8f3b66.ts.net:8080/assets")

    def test_local_server_prefix_is_rejected(self) -> None:
        # Arrange
        prefix = "http://127.0.0.1:8080/assets"

        # Act and Assert
        with self.assertRaisesRegex(ValueError, "MagicDNS"):
            _validate_server_prefix(prefix)


if __name__ == "__main__":
    unittest.main()
