"""Reference entry points: flat modules and executable test packages."""

def python_tests(root, directory=None):
    directory = root / directory if directory is not None else root / "tests" / "python-3.14.8"
    return sorted([*directory.glob("*.py"),
                   *(p / "__main__.py" for p in directory.iterdir()
                     if p.is_dir() and (p / "__init__.py").is_file()
                     and (p / "__main__.py").is_file())])


def python_test_name(path):
    return path.parent.name if path.name == "__main__.py" else path.stem


def python_test_path(root, name, directory=None):
    directory = root / directory if directory is not None else root / "tests" / "python-3.14.8"
    module = directory / (name + ".py")
    return module if module.is_file() else directory / name / "__main__.py"

def python_test_module(path):
    """The library module an executable test package runs under: a
    reference test package lives in the library's `test` namespace."""
    return "test." + path.parent.name if path.name == "__main__.py" else None


def python_test_args(path, only=None, runner=None):
    """Import flat reference modules so load_tests sees their real namespace."""
    if path.name == "__main__.py":
        return [str(path), *(only.split(",") if only else [])]
    module = "test." + path.stem
    if runner is None:
        from pathlib import Path
        runner = Path(__file__).with_name("run_reference.py")
    return [str(runner), module, str(path), *(only.split(",") if only else [])]
