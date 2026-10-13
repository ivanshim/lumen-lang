import codecs
import io
import _pyio
codecs.lookup('utf-8')
class C:
    def __del__(self):
        for module in (io, _pyio):
            for options in ({}, {'encoding': 'utf-8', 'errors': 'strict'}):
                module.TextIOWrapper(module.BytesIO(), **options)
                print('ok')
c = C()
