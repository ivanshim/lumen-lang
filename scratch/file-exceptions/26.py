def read_decorated_classes():
    @support.force_not_colorized_test_class
    class SyntaxErrorTests(unittest.TestCase):
        maxDiff = None

print("read decorated classes")
