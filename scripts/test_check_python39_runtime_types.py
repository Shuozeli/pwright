#!/usr/bin/env python3
"""Tests for the Python 3.9 runtime-type source audit."""

from __future__ import annotations

import unittest

from check_python39_runtime_types import runtime_union_cast_lines


class RuntimeUnionCastAuditTests(unittest.TestCase):
    def test_unquoted_pep604_cast_is_reported(self) -> None:
        # Arrange
        source = "from typing import cast\nvalue = cast(str | None, raw)\n"

        # Act
        lines = runtime_union_cast_lines(source)

        # Assert
        self.assertEqual(lines, [2])

    def test_quoted_pep604_cast_is_allowed(self) -> None:
        # Arrange
        source = 'from typing import cast\nvalue = cast("str | None", raw)\n'

        # Act
        lines = runtime_union_cast_lines(source)

        # Assert
        self.assertEqual(lines, [])

    def test_postponed_annotation_is_not_a_runtime_cast(self) -> None:
        # Arrange
        source = "from __future__ import annotations\nvalue: str | None = None\n"

        # Act
        lines = runtime_union_cast_lines(source)

        # Assert
        self.assertEqual(lines, [])


if __name__ == "__main__":
    unittest.main()
