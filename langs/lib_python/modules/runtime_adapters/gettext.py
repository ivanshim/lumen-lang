# Accelerate ordinary catalog discovery while retaining the release implementation.
# No discovery result is cached: each call reads the environment and stats paths.
def _install_catalog_discovery():
    original = globals()['find']
    _all = all
    installed = [None]
    # Initialize the optional acceleration at the first lookup, not at import.
    def find(domain, localedir=None, languages=None, all=False):
        if installed[0] is not None:
            return installed[0](domain, localedir, languages, all)
        if (languages is not None or type(domain) is not str
                or (localedir is not None and type(localedir) is not str)
                or type(all) is not bool or type(os.environ) is not os._Environ):
            return original(domain, localedir, languages, all)
        result = original(domain, localedir, languages, all)
        import locale
        import posix
        expand = _expand_lang
        normalizer = locale.normalize
        host = posix._host
        aliases = locale.locale_alias
        encodings = locale.locale_encoding_alias
        alias_revision = host('catalog_revision', aliases)
        encoding_revision = host('catalog_revision', encodings)
        environment = os.environ
        mapping = type(environment)
        get = mapping.get
        getitem = mapping.__getitem__
        encode = environment.encodekey
        decode = environment.decodevalue
        join = os.path.join
        exists = os.path.exists
        stat = os.stat
        if not _all(hasattr(fn, '__code__') for fn in (expand, normalizer, get, getitem, encode, decode, join, exists, stat, posix._path)):
            globals()['find'] = original
            installed[0] = original
            return result
        import builtins
        range_function = builtins.range
        key_error = builtins.KeyError
        namespace = globals()
        # A dependency already replaced before import must keep the interpreted path.
        ready = (mapping is os._Environ and type(aliases) is dict and type(encodings) is dict
                 and range_function.__module__ == 'builtins' and not hasattr(range_function, '__code__')
                 and key_error.__module__ == 'builtins' and key_error.__name__ == 'KeyError'
                 and type(os) is type(posix) and type(os.path) is type(posix)
                 and type(locale) is type(posix)
                 and _all(type(key) is str and type(value) is str for key, value in aliases.items())
                 and _all(type(key) is str and type(value) is str for key, value in encodings.items())
                 and stat.__module__ == 'posix' and stat.__code__.co_name == 'stat'
                 and stat.__code__.co_filename.endswith('posix.py')
                 and join.__module__ == 'posixpath' and join.__code__.co_name == 'join'
                 and join.__code__.co_filename.endswith('posixpath.py')
                 and exists.__module__ == 'genericpath' and exists.__code__.co_name == 'exists'
                 and exists.__code__.co_filename.endswith('genericpath.py')
                 and get.__code__.co_name == 'get' and get.__code__.co_filename.endswith('_collections_abc.py')
                 and getitem.__code__.co_name == '__getitem__' and getitem.__code__.co_filename.endswith('os.py')
                 and expand.__code__.co_name == '_expand_lang' and expand.__code__.co_filename.endswith('gettext.py')
                 and encode.__module__ == 'os' and encode.__code__.co_name == 'encode'
                 and encode.__code__.co_filename.endswith('os.py')
                 and decode.__module__ == 'os' and decode.__code__.co_name == 'decode'
                 and decode.__code__.co_filename.endswith('os.py')
                 and normalizer.__module__ == 'locale' and normalizer.__code__.co_name == 'normalize'
                 and normalizer.__code__.co_filename.endswith('locale.py'))
        if not ready:
            namespace['find'] = original
            installed[0] = original
            return result
        import sys
        guards = ((os, 'environ', environment), (os, 'path', os.path),
                  (os.path, 'join', join), (os.path, 'exists', exists),
                  (os, 'stat', stat), (namespace, '_expand_lang', expand),
                  (locale, 'normalize', normalizer),
                  (mapping, 'get', get), (mapping, '__getitem__', getitem),
                  (environment, 'encodekey', encode), (environment, 'decodevalue', decode),
                  (posix, '_host', host), (posix, '_path', posix._path),
                  (posix, 'fspath', posix.fspath),
                  (locale, 'locale_alias', aliases), (locale, 'locale_encoding_alias', encodings),
                  (sys.modules, 'locale', locale),
                  (builtins, 'range', range_function), (builtins, 'KeyError', key_error))
        functions = (expand, normalizer, join, exists, stat, get, getitem, encode, decode,
                     posix._path, posix.fspath)
        owners = (os, os.path, locale, posix, environment)
        getters = tuple(type(owner).__getattribute__ for owner in owners)
        guards += tuple((type(owner), '__getattribute__', getter) for owner, getter in zip(owners, getters))
        functions += tuple(getter for getter in getters if hasattr(getter, '__code__'))
        codec_module = locale.encodings
        codec_aliases = codec_module.aliases
        codec_names = codec_aliases.aliases
        helpers = (locale._replace_encoding, locale._append_modifier, codec_module.normalize_encoding)
        # Only unchanged release helpers and plain alias tables can be memoized.
        if (not _all(hasattr(helper, '__code__') for helper in helpers)
                or helpers[0].__code__.co_name != '_replace_encoding'
                or not helpers[0].__code__.co_filename.endswith('locale.py')
                or helpers[1].__code__.co_name != '_append_modifier'
                or not helpers[1].__code__.co_filename.endswith('locale.py')
                or helpers[2].__code__.co_name != 'normalize_encoding'
                or not helpers[2].__code__.co_filename.endswith('encodings/__init__.py')):
            namespace['find'] = original
            installed[0] = original
            return result
        guards += ((locale, 'encodings', codec_module),
                   (codec_module, 'aliases', codec_aliases), (codec_aliases, 'aliases', codec_names),
                   (locale, '_replace_encoding', helpers[0]), (locale, '_append_modifier', helpers[1]),
                   (codec_module, 'normalize_encoding', helpers[2]))
        functions += helpers
        revisions = ((aliases, alias_revision), (encodings, encoding_revision),
                     (codec_names, host('catalog_revision', codec_names)))
        normalization_cache = {}
        state = (namespace, environment, expand, guards, functions, revisions, normalizer, normalization_cache)

        # Let the kernels check dependency identities without interpreted metadata reads.
        def find(domain, localedir=None, languages=None, all=False):
            if ready:
                handled, result = host('catalog_find', domain, localedir, languages, all, state)
                if handled:
                    return result
            return original(domain, localedir, languages, all)
        find.__name__ = 'find'
        find.__qualname__ = 'find'
        find.__doc__ = original.__doc__
        namespace['find'] = find
        installed[0] = find
        return result
    find.__name__ = 'find'
    find.__qualname__ = 'find'
    find.__doc__ = original.__doc__
    globals()['find'] = find

_install_catalog_discovery()
del _install_catalog_discovery
