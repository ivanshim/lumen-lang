mv = memoryview(b"hello"); print(len(mv), mv[1], mv[1:3].tobytes(), mv.tolist()[:2], mv.readonly, bytes(mv) == b"hello")
