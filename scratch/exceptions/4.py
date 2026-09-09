try:
    try:
        raise "first"
    except:
        print("handler")
        raise "second"
    finally:
        print("finally")
except:
    print("outer")
try:
    try:
        print("body")
    except:
        print("wrong")
    else:
        raise "else"
    finally:
        print("else-finally")
except:
    print("else-outer")
