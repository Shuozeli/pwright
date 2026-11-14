"""Exact package/native ABI handshake."""

from __future__ import annotations

from . import _native

NATIVE_ABI_VERSION = 131

_observed_abi = _native.native_abi_version()
if _observed_abi != NATIVE_ABI_VERSION:
    raise ImportError(
        "incompatible Pwright native extension: "
        f"expected ABI {NATIVE_ABI_VERSION}, observed ABI {_observed_abi}"
    )


def native_capabilities() -> dict[str, object]:
    """Return immutable build identity as a new Python value."""

    return {
        "abi": _observed_abi,
        "version": _native.package_version(),
        "profile": _native.build_profile(),
        "features": list(_native.selected_features()),
        "limits": dict(_native.resource_limits()),
    }
