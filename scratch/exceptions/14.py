try:
    raise "boom"
except ():
    print("wrong")
except (E,):
    print("wrong")
except:
    print("caught")
