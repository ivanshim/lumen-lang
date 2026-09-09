try:
    raise "outer"
except:
    try:
        raise "inner"
    except:
        pass
    raise
