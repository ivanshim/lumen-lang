"""Invocation checks for reference modules and package entry points."""
from pathlib import Path
import os
import subprocess
import sys
import tempfile
from python_tests import python_test_args


def test_invocation():
    module = Path("suite/example.py")
    assert python_test_args(module, runner="runner.py") == ["runner.py", "test.example", str(module)]
    assert python_test_args(module, "Case,Case.method", "runner.py") == [
        "runner.py", "test.example", str(module), "Case", "Case.method"]
    package = Path("suite/example/__main__.py")
    assert python_test_args(package) == [str(package)]
    assert python_test_args(package, "Case.method") == [
        str(package), "Case.method"]


def test_source_import():
    runner = Path(__file__).with_name("run_reference.py").resolve()
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        package = root / "example_package"
        package.mkdir()
        (package / "__init__.py").write_text("")
        (package / "subject.py").write_text("raise AssertionError('wrong source')\n")
        requested = root / "requested"
        requested.mkdir()
        source = requested / "subject.py"
        source.write_text(
            "import unittest\n"
            "hook_called = False\n"
            "class Case(unittest.TestCase):\n"
            "    def test_namespace(self):\n"
            "        self.assertTrue(hook_called)\n"
            "        self.assertEqual(__name__, 'example_package.subject')\n"
            "        self.assertEqual(__package__, 'example_package')\n"
            "def load_tests(loader, tests, pattern):\n"
            "    global hook_called\n"
            "    hook_called = True\n"
            "    assert __name__ == 'example_package.subject'\n"
            "    return tests\n")
        environment = dict(os.environ, PYTHONPATH=str(root))
        result = subprocess.run([sys.executable, "-B", str(runner),
                                 "example_package.subject", str(source)],
                                env=environment, capture_output=True, text=True)
        assert result.returncode == 0, result.stderr
        assert "Ran 1 test" in result.stderr and "\nOK\n" in result.stderr, result.stderr

if __name__ == "__main__":
    test_invocation()
    test_source_import()
    print("reference invocation: 4 argument checks and source-import check passed")
