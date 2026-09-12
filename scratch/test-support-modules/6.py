from test import list_tests, mapping_tests
from test.support import testcase, numbers, isolation, threading_helper
import sys
print(hasattr(list_tests, 'CommonTest'), hasattr(mapping_tests, 'BasicTestMappingProtocol'))
print(hasattr(testcase, 'FloatsAreIdenticalMixin'), len(numbers.VALID_UNDERSCORE_LITERALS) > 0)
print(sys.intern('kept'), sys.getsizeof(1) > 0, sys.byteorder, sys.maxunicode)
