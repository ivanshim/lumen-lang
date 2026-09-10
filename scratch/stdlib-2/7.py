from enum import Enum, auto
class Colour(Enum):
    RED = auto()
    GREEN = auto()
    ALIAS = 1
print(Colour.RED is Colour.ALIAS, Colour(1) is Colour.RED, Colour.RED == Colour.GREEN)
from dataclasses import dataclass, field, asdict
def empty_items():
    return []
@dataclass
class Box:
    value: int
    items: list = field(default_factory=empty_items)
a = Box(2)
b = Box(value=2)
print(a == b, asdict(a))
a.items = [3]
print(a == b, b.items)
from typing import Optional, cast, TYPE_CHECKING
print(Optional[1] is Optional, cast(None, 9), TYPE_CHECKING)
from abc import ABC, abstractmethod
@abstractmethod
def answer():
    return 4
print(answer())
import time
first = time.monotonic()
time.sleep(0)
print(time.monotonic() >= first, time.time() > 0, time.perf_counter() >= 0)
print([member.value for member in Colour])
