class G:
    def __getitem__(self, key):
        print(key)
    def __setitem__(self, key, value):
        print(key, value)
    def __delitem__(self, key):
        print(key)
g = G()
g[:42, ..., :24:, 24, 100]
g[:42, ..., :24:, 24, 100] = "Strange"
del g[:42, ..., :24:, 24, 100]
g[1:2,]
g[::0]
g[...:...:...]
