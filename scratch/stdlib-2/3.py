import re
print(re.match(r"a(\d+)b", "a12b").group(1), re.findall(r"\w+", "x y"), re.sub(r"\s+", "-", "a  b"))
import typing
from typing import Optional, List
def f(x: Optional[List[int]]) -> None:
    pass
print(f([1]))
