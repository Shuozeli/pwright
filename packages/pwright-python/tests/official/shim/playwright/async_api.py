# pyright: reportPrivateUsage=false
"""Candidate async namespace used by the unmodified official test corpus."""

import os
import sys
from types import ModuleType
from typing import cast

from pwright import Error as Error
from pwright import TimeoutError as TimeoutError
from pwright import UnsupportedError as UnsupportedError
from pwright.async_api import (
    APIRequest as APIRequest,
)
from pwright.async_api import (
    APIRequestContext as APIRequestContext,
)
from pwright.async_api import (
    APIResponse as APIResponse,
)
from pwright.async_api import (
    Browser as Browser,
)
from pwright.async_api import (
    BrowserContext as BrowserContext,
)
from pwright.async_api import (
    BrowserType as BrowserType,
)
from pwright.async_api import (
    ClientCertificate as ClientCertificate,
)
from pwright.async_api import (
    ConsoleMessage as ConsoleMessage,
)
from pwright.async_api import (
    Download as Download,
)
from pwright.async_api import (
    ElementHandle as ElementHandle,
)
from pwright.async_api import (
    FileChooser as FileChooser,
)
from pwright.async_api import (
    Frame as Frame,
)
from pwright.async_api import (
    JSHandle as JSHandle,
)
from pwright.async_api import (
    Locator as Locator,
)
from pwright.async_api import (
    Page as Page,
)
from pwright.async_api import (
    Playwright as Playwright,
)
from pwright.async_api import (
    Request as Request,
)
from pwright.async_api import (
    Response as Response,
)
from pwright.async_api import (
    Selectors as Selectors,
)
from pwright.async_api import (
    SourceLocation as SourceLocation,
)
from pwright.async_api import (
    StorageState as StorageState,
)
from pwright.async_api import (
    StorageStateCookie as StorageStateCookie,
)
from pwright.async_api import Worker as Worker
from pwright.async_api import (
    async_playwright as async_playwright,
)
from pwright.async_api import expect as expect

from ._public_types import install_public_type_placeholders

install_public_type_placeholders(globals())


class _LaunchedBrowser(Browser):
    _official_fixture_context_ids: set[str]

    @property
    def contexts(self) -> list[BrowserContext]:
        return [
            context
            for context in super().contexts
            if context._object_id not in self._official_fixture_context_ids
        ]


async def _candidate_launch(self: BrowserType, **options: object) -> Browser:
    if os.environ.get("PWRIGHT_OFFICIAL_LAUNCH_MODE", "cdp-adapter") != "cdp-adapter":
        raise UnsupportedError("official-suite CDP launch adapter is disabled in owned mode")
    unsupported = sorted(set(options) - {"headless", "handle_sigint", "channel"})
    if unsupported:
        raise UnsupportedError(
            "official-suite launch adapter cannot apply options: " + ", ".join(unsupported)
        )
    if "handle_sigint" in options and options["handle_sigint"] is not False:
        raise UnsupportedError("official-suite CDP attach adapter requires handle_sigint=False")
    expected_channel = os.environ.get("PWRIGHT_OFFICIAL_BROWSER_CHANNEL")
    if "channel" in options and options["channel"] != expected_channel:
        raise UnsupportedError(
            "official-suite CDP attach adapter requires the configured browser channel"
        )
    if self.name != "chromium":
        raise UnsupportedError(
            f"official-suite launch adapter cannot attach the {self.name} engine"
        )
    endpoint = os.environ["PWRIGHT_CDP_ENDPOINT"]
    print(
        f"[pwright-candidate] BrowserType.launch fixture adapter -> connect_over_cdp({endpoint})",
        file=sys.stderr,
    )
    browser = await self.connect_over_cdp(endpoint)
    hidden = {context._object_id for context in browser.contexts}
    browser.__class__ = _LaunchedBrowser
    launched = cast(_LaunchedBrowser, browser)
    launched._official_fixture_context_ids = hidden
    return launched


_launch_mode = os.environ.get("PWRIGHT_OFFICIAL_LAUNCH_MODE", "cdp-adapter")
if _launch_mode == "cdp-adapter":
    setattr(BrowserType, "launch", _candidate_launch)
elif _launch_mode == "owned":
    if not callable(getattr(BrowserType, "launch", None)):
        raise RuntimeError("owned launch requires pwright.async_api.BrowserType.launch")
else:
    raise RuntimeError(f"unsupported official launch mode: {_launch_mode!r}")

# One upstream test imports Browser from the generated module solely for an
# annotation.  Publish the candidate Browser there without copying any oracle
# runtime behavior into the candidate facade.
_generated = ModuleType(f"{__name__}._generated")
setattr(_generated, "Browser", Browser)
sys.modules[_generated.__name__] = _generated
__path__: list[str] = []
