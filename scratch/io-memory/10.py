import io
raw = io.BytesIO()
buffer = io.BufferedReader(raw)
text = io.TextIOWrapper(buffer, encoding='utf-8')
raw.name = 'stream'
text.mode = 'r'
print(repr(text))
buffer.detach()
print(repr(text))
class BadName(io.BytesIO):
    @property
    def name(self):
        raise ValueError('name unavailable')
print(repr(io.TextIOWrapper(BadName(), encoding='ascii')))
class BadMode(io.TextIOWrapper):
    @property
    def mode(self):
        raise ValueError('mode unavailable')
try:
    repr(BadMode(io.BytesIO(), encoding='ascii'))
except ValueError as error:
    print(type(error).__name__, str(error))
