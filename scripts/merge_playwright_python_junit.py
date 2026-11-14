#!/usr/bin/env python3
"""Replace selected pytest JUnit cases while preserving the base report order."""

from __future__ import annotations

import argparse
import copy
import xml.etree.ElementTree as ET
from pathlib import Path

Key = tuple[str, str]


def _cases(path: Path) -> list[ET.Element]:
    return list(ET.parse(path).getroot().iter("testcase"))


def _key(case: ET.Element) -> Key:
    classname = case.get("classname")
    name = case.get("name")
    if classname is None or name is None:
        raise SystemExit("JUnit testcase lacks classname/name")
    return classname, name


def _count(cases: list[ET.Element], child: str) -> int:
    return sum(case.find(child) is not None for case in cases)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", type=Path, required=True)
    parser.add_argument("--replacement", type=Path, action="append", default=[])
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    base_cases = _cases(args.base)
    ordered_keys = [_key(case) for case in base_cases]
    if len(set(ordered_keys)) != len(ordered_keys):
        raise SystemExit("base JUnit report repeats a testcase")
    merged = {_key(case): copy.deepcopy(case) for case in base_cases}
    replaced: set[Key] = set()
    for report in args.replacement:
        report_seen: set[Key] = set()
        for case in _cases(report):
            key = _key(case)
            if key in report_seen:
                raise SystemExit(f"replacement JUnit report repeats a testcase: {key}")
            if key not in merged:
                raise SystemExit(
                    f"replacement testcase is absent from base report: {key}"
                )
            if key in replaced:
                raise SystemExit(
                    f"multiple replacement reports cover the same testcase: {key}"
                )
            report_seen.add(key)
            replaced.add(key)
            merged[key] = copy.deepcopy(case)

    cases = [merged[key] for key in ordered_keys]
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
        f"merged {len(cases)} cases with {len(replaced)} replacements: "
        f"failures={suite.get('failures')} errors={suite.get('errors')} "
        f"skipped={suite.get('skipped')}"
    )


if __name__ == "__main__":
    main()
