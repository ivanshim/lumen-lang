# From CPython v3.14.8 (8e6e75d9102e), Lib/test/test_dataclasses/dataclass_textanno.py; PSF License.
from __future__ import annotations

import dataclasses


class Foo:
    pass


@dataclasses.dataclass
class Bar:
    foo: Foo
