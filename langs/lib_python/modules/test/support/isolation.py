# An isolated process needs a host service this runtime does not provide.
import unittest

def runInSubprocess():
    return unittest.skip('isolated subprocesses are not supported')
