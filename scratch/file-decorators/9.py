def unread_until_called():
    class BaseStrTest:
        @bigmemtest(size=_2G // 5 + 1,
                   memuse=ucs2_char_size + pointer_size)
        def test_center(self, size):
            lpadsize = rpadsize = size // 2
            return lpadsize + rpadsize

print('bigmem method read')
