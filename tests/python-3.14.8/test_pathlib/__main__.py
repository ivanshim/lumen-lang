# Run the unchanged upstream pathlib package through its load_tests hook.
from test.test_pathlib import *
import unittest
if __name__ == '__main__':
    unittest.main()
