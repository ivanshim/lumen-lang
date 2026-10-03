# From CPython v3.14.8 (8e6e75d9102e), Lib/test/typinganndata/ann_module6.py; PSF License.
# Tests that top-level ClassVar is not allowed

from __future__ import annotations

from typing import ClassVar

wrong: ClassVar[int] = 1
