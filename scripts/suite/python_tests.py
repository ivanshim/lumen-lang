"""Reference entry points: scripts, caller-module doctests and test packages."""
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

def python_test_module(path):
    """Import packages and suites that ask doctest to inspect their caller."""
    if path.name == "__main__.py":
        return "test." + path.parent.name
    try:
        tree = ast.parse(path.read_text())
    except SyntaxError:
        # shortcut: Newer syntax keeps its script entry; upgrade the host parser if it declares caller-module doctests.
        return None
    modules, suites = set(), set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            modules.update(alias.asname or alias.name for alias in node.names if alias.name == "doctest")
        elif isinstance(node, ast.ImportFrom) and node.module == "doctest":
            suites.update(alias.asname or alias.name for alias in node.names if alias.name == "DocTestSuite")
    hooks = [node for node in tree.body if isinstance(node, ast.FunctionDef) and node.name == "load_tests"]
    for node in (item for hook in hooks for item in ast.walk(hook)):
        if not isinstance(node, ast.Call):
            continue
        function = node.func
        direct = isinstance(function, ast.Name) and function.id in suites
        qualified = (isinstance(function, ast.Attribute) and function.attr == "DocTestSuite"
                     and isinstance(function.value, ast.Name) and function.value.id in modules)
        if not (direct or qualified):
            continue
        module = node.args[0] if node.args else next(
            (kw.value for kw in node.keywords if kw.arg == "module"), None)
        if module is None or isinstance(module, ast.Constant) and module.value is None:
            return "test." + path.stem
    return None
