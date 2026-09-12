from dataclasses import dataclass
class Base:
    pass
@dataclass
class Child(Base):
    value: int
