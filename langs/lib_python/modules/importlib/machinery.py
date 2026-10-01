# Import machinery available to the source interpreter.
from ._bootstrap import ModuleSpec
from ._bootstrap_external import SOURCE_SUFFIXES, BYTECODE_SUFFIXES, EXTENSION_SUFFIXES
from ._bootstrap_external import SourceFileLoader, SourcelessFileLoader, ExtensionFileLoader, FileFinder, PathFinder

def all_suffixes():
    return SOURCE_SUFFIXES + BYTECODE_SUFFIXES + EXTENSION_SUFFIXES
