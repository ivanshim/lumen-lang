class P:
    count = 1
    count = count + 1
    def __init__(me, x=4):
        me.x = x
    def get(me):
        return me.x

class R: pass
class Q(P, R,):
    def __init__(me, x=4):
        super().__init__(x + 1)
    def append(me, x):
        me.x = me.x + x
        return me.x

Q.count = 7
q = Q()
bound = q.get
print(P.get(q), bound(), q.count, P.count, Q.count)
print(q.append(3), q.x)
class Outer:
    class Inner:
        value = 9
    saved = Inner
print(Outer.saved.value)
xs = []
print(xs.append(2), xs)
