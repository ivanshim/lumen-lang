from enum import Enum
class C(Enum):
    R = 1
    G = 2
print(C.R, C.R.value, C(2).name, list(C))
