"""Invocation checks for reference modules and package entry points."""
from pathlib import Path
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


if __name__ == "__main__":
    test_invocation()
    print("reference invocation: 4 checks passed")
