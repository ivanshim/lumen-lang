# tests/python/test_opcodes.py:39
if False:
    def test_use_existing_annotations(self):
        ns = {'__annotations__': {1: 2}}
        exec('x: int', ns)
        self.assertEqual(ns['__annotations__'], {1: 2})


print('read')
