class G:
    def __setitem__(self, key, value):
        print(key, value)
    def __delitem__(self, key):
        print(key)
g = G()
g[1:2, 3] = 9
g[...] = 8
g[()] = 7
del g[1:2]
del g[1, 2]
del g[...]
