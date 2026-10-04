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
