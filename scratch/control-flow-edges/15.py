class Pair:
    def __enter__(self):
        print("enter")
        return [1, 2]
    def __exit__(self, kind, value, trace):
        print("exit")
        return False
with Pair() as (a, b):
    print(a, b)
