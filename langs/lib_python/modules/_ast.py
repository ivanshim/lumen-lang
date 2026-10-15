# Native syntax-tree class namespace shared with the ast module.
import ast as _ast_module
__all__ = []
for _name, _value in vars(_ast_module).items():
    if isinstance(_value, type) and issubclass(_value, _ast_module.AST):
        globals()[_name] = _value
        __all__.append(_name)
