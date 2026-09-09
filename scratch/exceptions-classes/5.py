def fail():
    try:
        for i in [1, 2]:
            1 / 0
    finally:
        print("function finally")
try:
    fail()
except Exception:
    print("caught across call")
else:
    print("wrong")
finally:
    print("outer finally")
try:
    try:
        raise ValueError("again")
    except Exception:
        raise
except ValueError as e:
    print("reraised", str(e))
try:
    pass
except Exception:
    print("wrong")
else:
    print("else")
try:
    try:
        pass
    except Exception:
        print("wrong")
    else:
        1 / 0
    finally:
        print("else finally")
        raise
except ZeroDivisionError:
    print("else fault")
