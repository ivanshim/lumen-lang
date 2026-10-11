"""Reference entry points: flat modules and executable test packages."""
import ast


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


def _main_guard(test):
    return (isinstance(test, ast.Compare) and isinstance(test.left, ast.Name)
            and test.left.id == "__name__" and len(test.ops) == 1
            and isinstance(test.ops[0], ast.Eq) and len(test.comparators) == 1
            and isinstance(test.comparators[0], ast.Constant)
            and test.comparators[0].value == "__main__")


def _imported_name(tree, word):
    for node in tree.body:
        if isinstance(node, ast.ImportFrom) and node.module:
            for alias in node.names:
                if (alias.asname or alias.name) == word:
                    return node.module + "." + alias.name
        elif isinstance(node, ast.Import):
            for alias in node.names:
                if (alias.asname or alias.name.split(".")[0]) == word:
                    return alias.name
    return None


def _delegated_module(path):
    """The library module an entry's main guard hands its tests to.

    A reference driver can hold no tests of its own and run another module's
    suite through ``unittest.main(module=<name>)``. Reading that name from the
    driver's own imports keeps the driver on the imported-module entry, so its
    main block is not the collection point."""
    try:
        tree = ast.parse(path.read_text())
    except (SyntaxError, OSError, ValueError):
        return None
    for node in tree.body:
        if not (isinstance(node, ast.If) and _main_guard(node.test)):
            continue
        for call in ast.walk(node):
            if not (isinstance(call, ast.Call) and isinstance(call.func, ast.Attribute)
                    and call.func.attr == "main"):
                continue
            for keyword in call.keywords:
                if keyword.arg == "module" and isinstance(keyword.value, ast.Name):
                    return _imported_name(tree, keyword.value.id)
    return None


def python_test_module(path):
    """The library module a reference entry is imported as, the rule regrtest
    applies to every test: an executable package under its own name, a flat
    file under its own name, or the module its main guard delegates to."""
    if path.name == "__main__.py":
        return "test." + path.parent.name
    return _delegated_module(path) or ("test." + path.stem)
