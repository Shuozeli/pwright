#!/usr/bin/env python3
"""Check the pinned full Core source/suite inventory without executing tests.

This verifies source identity and the manifest's declared coverage. It cannot
verify expanded test IDs, candidate/oracle outcomes, or browser compatibility.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path, PurePosixPath
from typing import cast

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "tests/official/playwright-core-manifest.json"
MANIFEST_SHA256 = "b0345cbfeec1aafa02265759720b6bb8991fa72c238d798e497d2ace42f52c16"
COMMIT = "961381ec73a9fa2661d92504eab1a6bd6eaffc75"
SOURCE_COUNTS = {
    "specTs": 518,
    "specTsx": 25,
    "specJs": 15,
    "total": 558,
    "testsTree": 510,
    "examplesTree": 48,
}
SUITE_IDS = frozenset(
    {
        "android",
        "bidi",
        "component-react-vite",
        "component-react17",
        "component-svelte-vite",
        "component-svelte",
        "component-vue-cli",
        "component-vue-vite",
        "components-aggregate",
        "electron",
        "example-github-api",
        "example-mock-battery",
        "example-mock-filesystem",
        "example-svgomg",
        "example-todomvc",
        "installation",
        "library",
        "mcp",
        "playwright-test",
        "stress",
    }
)
NON_COLLECTED = frozenset(
    {
        "tests/installation/fixture-scripts/docker.spec.js",
        "tests/installation/fixture-scripts/failing.spec.js",
        "tests/installation/fixture-scripts/plugin.spec.ts",
        "tests/installation/fixture-scripts/sample.spec.js",
    }
)
GENERATED_RUNNER = (
    "tests/playwright-test/stable-test-runner/node_modules/@playwright/test/cli.js"
)
GENERATED_RUNNER_PACKAGE = "tests/playwright-test/stable-test-runner/package.json"


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _git(upstream: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(upstream), *args],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def _pinned_specs(upstream: Path) -> list[str]:
    _require(_git(upstream, "rev-parse", "HEAD") == COMMIT, "upstream commit differs")
    _require(
        not _git(upstream, "status", "--porcelain", "--untracked-files=no"),
        "upstream tracked source is dirty",
    )
    return sorted(
        path
        for path in _git(
            upstream, "ls-files", "*spec.ts", "*spec.tsx", "*spec.js"
        ).splitlines()
        if path
    )


def _safe_relative(raw: object) -> str:
    _require(isinstance(raw, str) and bool(raw), "source path is not text")
    path = PurePosixPath(cast(str, raw))
    _require(
        not path.is_absolute()
        and path.as_posix() == raw
        and all(part not in ("", ".", "..") for part in cast(str, raw).split("/"))
        and "\\" not in cast(str, raw),
        f"unsafe source path: {raw}",
    )
    return cast(str, raw)


def _tracked_regular(upstream: Path, relative: object, tracked: set[str]) -> str:
    name = _safe_relative(relative)
    path = upstream / name
    _require(
        name in tracked
        and path.is_file()
        and not path.is_symlink()
        and path.resolve().is_relative_to(upstream.resolve()),
        f"tracked regular source is missing: {name}",
    )
    return name


def _manifest_header(manifest: dict[str, object]) -> None:
    _require(
        set(manifest)
        == {
            "schema",
            "repository",
            "tag",
            "commit",
            "sourceInventory",
            "nonCollectedSpecs",
            "suites",
        }
        and manifest.get("schema") == 1
        and manifest.get("repository") == "https://github.com/microsoft/playwright.git"
        and manifest.get("tag") == "v1.58.0"
        and manifest.get("commit") == COMMIT
        and manifest.get("sourceInventory") == SOURCE_COUNTS,
        "full Core manifest provenance or source counts differ",
    )


def validate_source_inventory(
    manifest: dict[str, object],
    specs: list[str],
    tracked: set[str],
    upstream: Path,
) -> dict[str, object]:
    """Validate complete pinned source accounting using a Git-tracked file set."""
    _manifest_header(manifest)
    _require(specs == sorted(set(specs)), "tracked spec list is not sorted and unique")
    _require(all(_safe_relative(path) == path for path in specs), "unsafe spec path")
    _require(set(specs) <= tracked, "tracked spec list contains an untracked source")
    observed = {
        "specTs": sum(path.endswith(".spec.ts") for path in specs),
        "specTsx": sum(path.endswith(".spec.tsx") for path in specs),
        "specJs": sum(path.endswith(".spec.js") for path in specs),
        "total": len(specs),
        "testsTree": sum(path.startswith("tests/") for path in specs),
        "examplesTree": sum(path.startswith("examples/") for path in specs),
    }
    _require(observed == SOURCE_COUNTS, "tracked Core spec source counts differ")
    _require(
        all(path.startswith(("tests/", "examples/")) for path in specs),
        "tracked Core spec is outside tests/examples",
    )

    raw_non_collected = manifest["nonCollectedSpecs"]
    _require(isinstance(raw_non_collected, list), "non-collected specs are not a list")
    non_collected: set[str] = set()
    for entry in cast(list[object], raw_non_collected):
        _require(isinstance(entry, dict), "non-collected spec is malformed")
        item = cast(dict[str, object], entry)
        _require(
            set(item) == {"path", "classification", "parentSuite", "reason"}
            and item.get("classification") == "nested-fixture-script"
            and item.get("parentSuite") == "installation"
            and isinstance(item.get("reason"), str)
            and bool(item["reason"]),
            "non-collected spec disposition differs",
        )
        name = _safe_relative(item["path"])
        _require(name not in non_collected, f"duplicate non-collected spec: {name}")
        non_collected.add(name)
    _require(
        non_collected == NON_COLLECTED and non_collected <= set(specs),
        "non-collected fixture spec set differs from tracked sources",
    )

    raw_suites = manifest["suites"]
    _require(isinstance(raw_suites, list), "Core suites are not a list")
    suite_ids: set[str] = set()
    declared_tests = 0
    for entry in cast(list[object], raw_suites):
        _require(isinstance(entry, dict), "Core suite entry is malformed")
        suite = cast(dict[str, object], entry)
        _require(
            {"id", "role", "runner", "expectedTests", "expectedFiles"} <= set(suite)
            and set(suite)
            <= {
                "id",
                "role",
                "runner",
                "config",
                "args",
                "expectedTests",
                "expectedFiles",
                "candidateAdapter",
                "oracleAdapter",
            },
            "Core suite fields differ",
        )
        suite_id = suite["id"]
        _require(
            isinstance(suite_id, str)
            and suite_id in SUITE_IDS
            and suite_id not in suite_ids,
            f"duplicate or unknown Core suite: {suite_id}",
        )
        suite_ids.add(suite_id)
        if suite_id == "library":
            _require(
                suite.get("candidateAdapter") == "test-mode-playwright-core"
                and suite.get("oracleAdapter") == "test-mode-playwright-core-cdp",
                "library candidate/oracle adapter declarations differ",
            )
        else:
            _require(
                "candidateAdapter" not in suite and "oracleAdapter" not in suite,
                "non-library suite unexpectedly declares an adapter",
            )
        _require(
            suite["role"]
            in {"official-entrypoint", "nested-component-corpus", "official-example"}
            and type(suite["expectedTests"]) is int
            and cast(int, suite["expectedTests"]) > 0
            and type(suite["expectedFiles"]) is int
            and 0 < cast(int, suite["expectedFiles"]) <= SOURCE_COUNTS["total"],
            f"invalid Core suite role or counts: {suite_id}",
        )
        declared_tests += cast(int, suite["expectedTests"])
        runner = _safe_relative(suite["runner"])
        if runner == GENERATED_RUNNER:
            _require(
                suite_id == "playwright-test",
                "generated runner belongs only to playwright-test suite",
            )
            _tracked_regular(upstream, GENERATED_RUNNER_PACKAGE, tracked)
        else:
            _tracked_regular(upstream, runner, tracked)
        if "config" in suite:
            _tracked_regular(upstream, suite["config"], tracked)
        if "args" in suite:
            args = suite["args"]
            _require(isinstance(args, list) and bool(args), "suite args are malformed")
            for arg in cast(list[object], args):
                name = _tracked_regular(upstream, arg, tracked)
                _require(name in specs, f"suite arg is not a tracked spec: {name}")
    _require(
        suite_ids == SUITE_IDS and declared_tests == 48_477,
        "full Core suite set or declared invocation count differs",
    )
    return {
        "schema": 1,
        "scope": "pinned-upstream-source-and-declared-suite-inventory-only",
        "trackedSpecFiles": len(specs),
        "nonCollectedFixtureSpecs": len(non_collected),
        "suites": len(suite_ids),
        "declaredTestInvocations": declared_tests,
        "deferredGeneratedRunner": {
            "suite": "playwright-test",
            "runner": GENERATED_RUNNER,
            "trackedPackageSource": GENERATED_RUNNER_PACKAGE,
            "runnerBinaryVerified": False,
        },
        "testsExecuted": False,
        "candidateOracleOutcomesVerified": False,
    }


def verify(upstream: Path, manifest_path: Path = MANIFEST) -> dict[str, object]:
    _require(
        manifest_path.is_file()
        and not manifest_path.is_symlink()
        and _sha256(manifest_path) == MANIFEST_SHA256,
        "pinned full Core manifest file or SHA-256 differs",
    )
    raw = json.loads(manifest_path.read_text(encoding="utf-8"))
    _require(isinstance(raw, dict), "full Core manifest is not an object")
    manifest = cast(dict[str, object], raw)
    _manifest_header(manifest)
    _require(
        upstream.is_dir() and not upstream.is_symlink(), "upstream checkout missing"
    )
    specs = _pinned_specs(upstream)
    tracked = set(_git(upstream, "ls-files").splitlines())
    report = validate_source_inventory(manifest, specs, tracked, upstream)
    report["upstreamCommit"] = COMMIT
    report["manifestSha256"] = MANIFEST_SHA256
    return report


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("upstream", type=Path)
    args = parser.parse_args()
    try:
        report = verify(args.upstream)
    except (
        OSError,
        ValueError,
        KeyError,
        TypeError,
        subprocess.CalledProcessError,
        SystemExit,
    ) as error:
        parser.exit(2, f"full Core source inventory rejected: {error}\n")
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
