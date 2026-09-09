try:
    try:
        raise "again"
    except:
        print("inner")
        raise
except:
    print("outer")
