try:
    raise "boom"
except E as e:
    print(e)
except:
    print("caught")
finally:
    print("finally")
try:
    print("body")
except E as e:
    print(e)
else:
    print("else")
finally:
    print("last")
