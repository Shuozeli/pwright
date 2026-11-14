"""Expose only the pinned upstream test package to spawned test children."""

from __future__ import annotations

import os
from pathlib import Path

_configured = os.environ.get("PWRIGHT_OFFICIAL_UPSTREAM_TESTS")
if _configured is None:
    raise ImportError("PWRIGHT_OFFICIAL_UPSTREAM_TESTS is not configured")

_upstream_tests = Path(_configured).resolve()
if not _upstream_tests.is_dir():
    raise ImportError(f"upstream Playwright tests directory is missing: {_upstream_tests}")

__path__ = [str(_upstream_tests)]

if (
    os.environ.get("PWRIGHT_OFFICIAL_MODE") == "oracle"
    and os.environ.get("PWRIGHT_OFFICIAL_LAUNCH_MODE", "cdp-adapter") == "cdp-adapter"
):
    from pwright_official_oracle_runtime import install_oracle_launch_adapter

    install_oracle_launch_adapter()
