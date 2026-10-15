# Run the unchanged upstream pathlib module with its package context.
import unittest
from test.test_pathlib import test_join_windows as reference
if __name__ == "__main__":
    unittest.main(module=reference)
