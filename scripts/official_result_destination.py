"""Fail-closed destination checks before launching an official test process."""

from pathlib import Path


def prepare_result_destination(path: Path, *, resume_isolated: bool = False) -> Path:
    # Resolve the parent only: resolving the final component would hide an
    # existing (including dangling) symlink before the freshness check.
    destination = path.parent.resolve() / path.name
    for output in (destination, destination.with_suffix(".provenance.json")):
        if output.is_symlink() or (output.exists() and not output.is_file()):
            raise SystemExit(f"result destination is not a regular file path: {output}")
        if output.exists() and not resume_isolated:
            raise SystemExit(f"result destination already exists: {output}; choose a new --result")
    return destination
