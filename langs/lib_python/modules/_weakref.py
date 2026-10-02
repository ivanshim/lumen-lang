# The module CPython writes in C under this name. The kernels' weak
# holds are reached through the weakref module here; this adapter
# re-exports what that module offers under the names _weakref carries.
from weakref import ref, proxy, getweakrefcount, getweakrefs
