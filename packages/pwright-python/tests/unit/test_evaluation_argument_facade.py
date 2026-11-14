# pyright: reportPrivateUsage=false
"""The facade preserves special values for Rust-owned evaluation encoding."""

from typing import cast
from urllib.parse import ParseResult, urlparse

from pwright.async_api import _native_evaluation_argument


def test_url_parse_result_reaches_native_codec_with_its_type() -> None:
    # Arrange
    parsed = urlparse("https://user:pass@example.com/?foo=bar#hi")

    # Act
    converted = _native_evaluation_argument(parsed)

    # Assert
    assert converted is parsed
    assert isinstance(converted, ParseResult)


def test_nested_url_parse_result_reaches_native_codec_with_its_type() -> None:
    # Arrange
    parsed = urlparse("https://example.com/a?q=1")
    argument = {"urls": [parsed]}

    # Act
    converted = cast(dict[str, list[object]], _native_evaluation_argument(argument))

    # Assert
    assert converted == argument
    assert converted["urls"][0] is parsed


def test_plain_tuple_still_becomes_native_codec_array_argument() -> None:
    # Arrange
    argument = ("first", "second")

    # Act
    converted = _native_evaluation_argument(argument)

    # Assert
    assert converted == ["first", "second"]
    assert type(converted) is list
