# Source: CPython v3.14.8 / 8e6e75d9102e, Lib/test/test_warnings/data/package_helper.py; PSF License.
# helper to the helper for testing skip_file_prefixes.

import os

package_path = os.path.dirname(__file__)

def inner_api(message, *, stacklevel, warnings_module):
    warnings_module.warn(
            message, stacklevel=stacklevel,
            skip_file_prefixes=(package_path,))
