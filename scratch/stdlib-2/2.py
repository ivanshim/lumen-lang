from dataclasses import dataclass
@dataclass
class P:
    x: int
    y: int = 0
print(P(1), P(1) == P(1, 0))
import contextlib
with contextlib.suppress(KeyError):
    {}["k"]
print("ok")
