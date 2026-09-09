try:
    raise "boom"
except 42:
    print("wrong")
except:
    print("wrong")
finally:
    print("finally")
