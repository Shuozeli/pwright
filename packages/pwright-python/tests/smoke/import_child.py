import json

import pwright
from pwright import _native

print(
    json.dumps(
        {
            "abi": _native.native_abi_version(),
            "capabilities": pwright.native_capabilities(),
            "module": _native.__file__,
        },
        sort_keys=True,
    )
)
