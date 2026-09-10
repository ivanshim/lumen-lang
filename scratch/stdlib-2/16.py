from dataclasses import dataclass
@dataclass()
class Point:
    value: int
print(Point(3), Point(3) == Point(3))
