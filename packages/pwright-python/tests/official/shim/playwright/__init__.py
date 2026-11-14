"""Candidate-only public namespace adapter for unmodified upstream tests.

The official suite's own fixtures import ``playwright._impl`` helpers.  Extend
the package search path so those fixture-only internals continue to come from
the pinned oracle installation while ``async_api`` and ``sync_api`` resolve to
the Rust-backed candidate adapters in this directory.
"""

from pkgutil import extend_path

__path__ = extend_path(__path__, __name__)
