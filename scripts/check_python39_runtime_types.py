#!/usr/bin/env python3
"""Reject runtime PEP 604 expressions hidden inside typing.cast calls."""

from __future__ import annotations

import argparse
import ast
from pathlib import Path


def _is_cast(call: ast.Call) -> bool:
    return (isinstance(call.func, ast.Name) and call.func.id == "cast") or (
        isinstance(call.func, ast.Attribute) and call.func.attr == "cast"
    )


def _contains_runtime_union(node: ast.AST) -> bool:
    return any(
        isinstance(child, ast.BinOp) and isinstance(child.op, ast.BitOr) for child in ast.walk(node)
    )


def runtime_union_cast_lines(source: str) -> list[int]:
    tree = ast.parse(source)
    return sorted(
        call.lineno
        for call in ast.walk(tree)
        if isinstance(call, ast.Call)
        and _is_cast(call)
        and call.args
        and _contains_runtime_union(call.args[0])
    )


def _python_files(paths: list[Path]) -> list[Path]:
    files: list[Path] = []
    for path in paths:
        if path.is_dir():
            files.extend(path.rglob("*.py"))
        elif path.is_file():
            files.append(path)
        else:
            raise ValueError(f"Python source path is missing: {path}")
    return sorted(set(files))


def main() -> None:
    root = Path(__file__).parents[1]
    parser = argparse.ArgumentParser()
    parser.add_argument("path", nargs="*", type=Path)
    args = parser.parse_args()
    paths = args.path or [
        root / "packages/pwright-python/python",
        root / "scripts/playwright_python_official_provenance.py",
        root / "scripts/run_playwright_python_official_conformance.py",
        root / "scripts/verify_playwright_python_official_provenance.py",
        root / "scripts/audit_playwright_python_official_invocations.py",
    ]
    try:
        files = _python_files(paths)
    except ValueError as error:
        parser.error(str(error))

    findings = [
        (path, line) for path in files for line in runtime_union_cast_lines(path.read_text())
    ]
    if findings:
        for path, line in findings:
            print(
                f"{path}:{line}: typing.cast evaluates its first argument; "
                "quote PEP 604 types for Python 3.9"
            )
        raise SystemExit(1)
    print(f"Python 3.9 runtime cast audit passed ({len(files)} files)")


if __name__ == "__main__":
    main()
