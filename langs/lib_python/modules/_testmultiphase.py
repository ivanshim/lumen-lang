# Stand-in for the CPython test extension this runtime does not build.
# It stands only so that a support module which imports it while it is
# read (test.test_importlib.util) can be read; asking it for anything
# fails as it would for an extension that is not loaded.
