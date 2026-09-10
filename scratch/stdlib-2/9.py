from dataclasses import dataclass, field
@dataclass
class Note:
    text: str
    marks: list
print(Note("a'b", ['x', 'y']))
from enum import Enum
class Word(Enum):
    HELLO = 'hello'
print(list(Word))
print(str(Word.HELLO), 1 == Note('a', []), Note('a', []) == 1)
