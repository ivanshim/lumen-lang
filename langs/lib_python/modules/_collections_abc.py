# The reference's modules ask for the container kinds by the name
# _collections_abc; this library keeps them in collections.abc. The one
# kind is answered by either name, so a subclass spelled under either
# stands as a subclass of the one kind.
from collections.abc import *
from collections.abc import __all__
