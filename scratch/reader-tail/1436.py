# tests/python/test_decorators.py:201
if False:
    def test_expressions(self):
        for expr in (
            "(x,)", "(x, y)", "x := y", "(x := y)", "x @y", "(x @ y)", "x[0]",
            "w[x].y.z", "w + x - (y + z)", "x(y)()(z)", "[w, x, y][z]", "x.y",
        ):
            compile(f"@{expr}\ndef f(): pass", "test", "exec")


print('read')
