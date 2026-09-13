class Q:
    j: int
    J: float
    jj: int
    j2: int = 4
    def j(self):
        return 2
print(Q.j2, Q().j())
class R:
    e: int
    j: int = 7
print(R.j)
j = 1j
J = j * 2
print(j, J, type(J).__name__)
