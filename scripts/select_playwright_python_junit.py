#!/usr/bin/env python3
"""Select exact pytest node IDs from one official Python JUnit report."""

from __future__ import annotations

import argparse
import copy
import xml.etree.ElementTree as ET
from pathlib import Path

Key = tuple[str, str]


def _junit_key(node_id: str) -> Key:
    source, *segments = node_id.split("::")
    if not segments or not source.endswith(".py"):
        raise SystemExit(f"unsupported pytest node ID: {node_id}")
    module = source.removesuffix(".py").replace("/", ".")
    classname = ".".join((module, *segments[:-1]))
    return classname, segments[-1]


def _case_key(case: ET.Element) -> Key:
    classname = case.get("classname")
    name = case.get("name")
    if classname is None or name is None:
        raise SystemExit("JUnit testcase lacks classname/name")
    return classname, name


def _count(cases: list[ET.Element], child: str) -> int:
    return sum(case.find(child) is not None for case in cases)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--test", action="append", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    requested = [_junit_key(node_id) for node_id in args.test]
    if len(set(requested)) != len(requested):
        raise SystemExit("requested pytest node IDs are not unique")

    available: dict[Key, ET.Element] = {}
    for case in ET.parse(args.input).getroot().iter("testcase"):
        key = _case_key(case)
        if key in available:
            raise SystemExit(f"input JUnit report repeats a testcase: {key}")
        available[key] = case

    missing = [key for key in requested if key not in available]
    if missing:
        raise SystemExit(f"requested testcases are absent from input report: {missing}")

    cases = [copy.deepcopy(available[key]) for key in requested]
    suite = ET.Element(
        "testsuite",
        {
            "name": "pytest",
            "errors": str(_count(cases, "error")),
            "failures": str(_count(cases, "failure")),
            "skipped": str(_count(cases, "skipped")),
            "tests": str(len(cases)),
            "time": f"{sum(float(case.get('time', '0')) for case in cases):.3f}",
        },
    )
    suite.extend(cases)
    root = ET.Element("testsuites", {"name": "pytest tests"})
    root.append(suite)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    ET.ElementTree(root).write(args.output, encoding="utf-8", xml_declaration=True)
    print(
        f"selected {len(cases)} of {len(available)} cases: "
        f"failures={suite.get('failures')} errors={suite.get('errors')} "
        f"skipped={suite.get('skipped')}"
    )


if __name__ == "__main__":
    main()
