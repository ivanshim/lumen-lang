# The common reference sequence tests are not carried in this module.
# Refuse an instance rather than claim their absent checks succeeded.
import unittest

class CommonTest(unittest.TestCase):
    def __init__(self, methodName='runTest'):
        raise 'NotImplementedError: common sequence tests are not available'
