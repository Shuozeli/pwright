"""Verify the two-case, two-lane trusted-HTTPS Service Worker diagnostic."""

from __future__ import annotations

import ast
import copy
import hashlib
import json
import subprocess
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[1]
UPSTREAM = Path.home() / "projects/thirdparty/playwright-python"
OFFICIAL = ROOT / "packages/pwright-python/tests/official"
MANIFEST = OFFICIAL / "sync-network-service-worker-secure-chromium-paired-v1-manifest.json"
ORIGINAL = OFFICIAL / "sync-network-full-v1-manifest.json"
ORACLE_ONLY = (
    OFFICIAL / "sync-network-service-worker-secure-chromium-oracle-v1-manifest.json"
)
INVENTORY = ROOT / ".work/official-conformance/playwright-python-v1.58.0-profiled.jsonl"
SOURCE = "tests/sync/test_network.py"
COMMIT = "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
NAMES = (
    "test_should_report_if_response_was_from_service_worker",
    "test_should_report_service_worker_request",
)
CASES = [
    {"id": f"{SOURCE}::{NAMES[0]}", "capabilities": ["response-service-worker", "sync-api"]},
    {"id": f"{SOURCE}::{NAMES[1]}", "capabilities": ["request-service-worker", "sync-api"]},
]
EXPANDED_IDS = [f"{case['id']}[chromium]" for case in CASES]
FILES = {
    "tests/conftest.py": "c4236e98ff1731fff1f7ef1c00b37fe3a0cb1e4a3fd07e89a491dc7129ea93cc",
    "tests/sync/conftest.py": "9ec5a65a98c312de67590792e80d3d9ea45312d56188f45101e71df6f7773d0b",
    "tests/server.py": "2cf1a6b805f99c1c82c5351b67df995f06f2a9a18d21dbd48d4d4dc6079c3030",
    "tests/utils.py": "60b7d0915ffdaec4dda729fc38658d55bb216ec8baaf9087d77cef4c3c34abea",
    SOURCE: "db831f837f94b2ee4eefb9dd6b271e7af42c573a59e4c3f49c20d83c572128ff",
}
ASSETS = {
    "tests/assets/empty.html": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "tests/assets/simple.json": "1a30b52dacd02b3698f473e5d9df017c54b1e006846363e878dcc2ade9624712",
    "tests/assets/serviceworkers/fetch/sw.html": "450261e64a2d7bf9b054638c11ebbaf0481394e0e5284350cbf0c601ccefd0b0",
    "tests/assets/serviceworkers/fetch/sw.js": "1d42856cb41a94fbbfc321d92b39d152e52d357cbe9cbd0f54d75f8b710690ac",
    "tests/assets/serviceworkers/fetch/style.css": "aae2798dbe4c81fb3b467df66899ef72fed0d732490cff8cf0df4717ac76697e",
    "tests/testserver/cert.pem": "402a4ecc40aba35f8143673ac7c20d02c742954a79dc94523b06a5bd9c5fb18e",
    "tests/testserver/key.pem": "3302e5f7eb8485e832a10f54a63cc049f98e98a5a2d55671c9ed9b440c59053a",
}
EXPERIMENT = {
    "scope": "paired-trusted-tailscale-https-fixture-diagnostic",
    "lanes": ["candidate", "oracle"],
    "api": "sync",
    "browser": "chromium",
    "fixtureFlag": "--secure-http-fixture",
    "selectionFlags": ["--manifest-cases", "--manifest-api", "sync", "--browser", "chromium"],
    "ledgerPolicy": "diagnostic-only-no-ledger-promotion-preserve-original-six-case-http-results",
    "hypothesis": (
        "Trusted HTTPS may enable service-worker registration and activation; "
        "comparing both lanes does not by itself establish the root cause of "
        "the original HTTP failures."
    ),
}


def _read_json(path: Path) -> dict[str, object]:
    return json.loads(path.read_text())


def _id_hash(ids: list[str]) -> str:
    return hashlib.sha256("".join(f"{item}\n" for item in sorted(ids)).encode()).hexdigest()


def _validate_contract(
    manifest: dict[str, object], original: dict[str, object], oracle_only: dict[str, object]
) -> None:
    expected: dict[str, object] = {
        "schema": 1,
        "repository": "https://github.com/microsoft/playwright-python.git",
        "tag": "v1.58.0",
        "commit": COMMIT,
        "corpus": original["corpus"],
        "bootstrapSelectionOnly": False,
        "execution": {"requiresBrowser": True, "requiresFixtureServers": True},
        "experiment": EXPERIMENT,
        "files": FILES,
        "assets": ASSETS,
        "expandedChromiumIds": EXPANDED_IDS,
        "expandedChromiumIdsSha256": _id_hash(EXPANDED_IDS),
        "cases": CASES,
    }
    if set(manifest) != set(expected):
        raise ValueError("manifest fields changed")
    for field, value in expected.items():
        if manifest[field] != value:
            raise ValueError(f"manifest {field} changed")
    for field in ("repository", "tag", "commit", "corpus", "files", "assets"):
        if original[field] != expected[field]:
            raise ValueError(f"six-case source {field} changed")
        if oracle_only[field] != expected[field]:
            raise ValueError(f"Oracle-only source {field} changed")
    if original["cases"][-2:] != CASES or oracle_only["cases"] != CASES:
        raise ValueError("selected cases differ from the historical manifests")
    if oracle_only["expandedChromiumIds"] != EXPANDED_IDS:
        raise ValueError("Oracle-only expanded IDs changed")


class SyncNetworkServiceWorkerSecurePairedManifestTests(unittest.TestCase):
    def test_exact_paired_scope_and_pins(self) -> None:
        # Arrange
        manifest = _read_json(MANIFEST)
        original = _read_json(ORIGINAL)
        oracle_only = _read_json(ORACLE_ONLY)

        # Act
        _validate_contract(manifest, original, oracle_only)

        # Assert
        self.assertEqual(len(manifest["cases"]), 2)
        self.assertEqual(manifest["experiment"], EXPERIMENT)

    def test_selected_source_fixture_assets_and_commit_match_upstream(self) -> None:
        if not UPSTREAM.is_dir():
            self.skipTest("pinned upstream checkout is unavailable")

        # Arrange
        manifest = _read_json(MANIFEST)
        tree = ast.parse((UPSTREAM / SOURCE).read_bytes(), filename=SOURCE)

        # Act
        head = subprocess.run(
            ["git", "-C", str(UPSTREAM), "rev-parse", "HEAD"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        hashes = {
            relative: hashlib.sha256((UPSTREAM / relative).read_bytes()).hexdigest()
            for relative in {**FILES, **ASSETS}
        }
        functions = {
            node.name: node
            for node in tree.body
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
        }

        # Assert
        self.assertEqual(head, COMMIT)
        self.assertEqual(hashes, {**manifest["files"], **manifest["assets"]})
        self.assertEqual(set(NAMES), set(functions).intersection(NAMES))
        self.assertTrue(all(isinstance(functions[name], ast.FunctionDef) for name in NAMES))

    def test_inventory_expands_exactly_two_public_chromium_ids(self) -> None:
        if not INVENTORY.is_file():
            self.skipTest("pinned upstream inventory is unavailable")

        # Arrange
        manifest = _read_json(MANIFEST)
        base_ids = {case["id"] for case in CASES}

        # Act
        with INVENTORY.open() as stream:
            rows = [
                row
                for line in stream
                if (row := json.loads(line))["id"].split("[", 1)[0] in base_ids
            ]
        chromium_rows = [row for row in rows if row["id"].endswith("[chromium]")]
        observed_ids = [row["id"] for row in chromium_rows]

        # Assert
        self.assertEqual(len(rows), 6)
        self.assertEqual(
            {row["id"] for row in rows},
            {f"{base}[{browser}]" for base in base_ids for browser in ("chromium", "firefox", "webkit")},
        )
        self.assertEqual(observed_ids, manifest["expandedChromiumIds"])
        self.assertEqual(_id_hash(observed_ids), manifest["expandedChromiumIdsSha256"])
        self.assertTrue(all(row["api"] == "sync" for row in rows))
        self.assertTrue(all(row["sourceClassification"] == "public-api-source" for row in rows))
        self.assertTrue(all(row["privateImports"] == [] for row in rows))

    def test_mutations_of_lane_policy_pins_and_ids_are_rejected(self) -> None:
        # Arrange
        manifest = _read_json(MANIFEST)
        original = _read_json(ORIGINAL)
        oracle_only = _read_json(ORACLE_ONLY)
        mutations = {
            "lane": ("experiment", "lanes", ["oracle"]),
            "ledger": ("experiment", "ledgerPolicy", "promote"),
            "fixture": ("experiment", "fixtureFlag", "--other-fixture"),
            "source": ("files", SOURCE, "0" * 64),
            "asset": ("assets", "tests/assets/serviceworkers/fetch/sw.js", "0" * 64),
            "case": ("cases", 0, {"id": "wrong", "capabilities": []}),
            "expanded ID": ("expandedChromiumIds", 0, "wrong[chromium]"),
            "ID digest": ("expandedChromiumIdsSha256", None, "0" * 64),
        }

        for name, (field, key, replacement) in mutations.items():
            with self.subTest(name=name):
                # Act
                changed = copy.deepcopy(manifest)
                if key is None:
                    changed[field] = replacement
                else:
                    changed[field][key] = replacement

                # Assert
                with self.assertRaises(ValueError):
                    _validate_contract(changed, original, oracle_only)


if __name__ == "__main__":
    unittest.main()
