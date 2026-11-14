"""Run the explicit source-only adapter without changing the v1 receipt tooling."""

from pathlib import Path

import run_playwright_official_image_tools_source_only as lane

_ROOT = Path(__file__).parents[1]
lane._MANIFEST = _ROOT / "tests/official/playwright-core-internal-source-only-image-tools-v2-manifest.json"
lane._GUARD = _ROOT / "tests/official/playwright-core-image-tools-source-only-v2-guard.cjs"
lane._TOOLING = (
    Path("scripts/official_result_destination.py"),
    Path("scripts/run_playwright_official_image_tools_source_only.py"),
    Path("scripts/run_playwright_official_image_tools_source_only_v2.py"),
    Path("tests/official/playwright-core-internal-source-only-image-tools-v1-manifest.json"),
    Path("tests/official/playwright-core-image-tools-source-only-guard.cjs"),
    Path("tests/official/playwright-core-image-tools-source-only-v2-guard.cjs"),
)

if __name__ == "__main__":
    lane.main()
