# Python tuple allocation and release

Both Python kernels keep sequence elements in an `Rc<Vec<Value>>`. Stack8's
`Items` and microcode7's `Sequence` wrap that reference so the final owner can
return dead tuple storage to a pool. Arrays and internal rows use the same
wrapper to preserve shared iterator and comparison references, but their
ordinary constructors do not enable recycling.

The existing `ext.builtin.tuple` language support enables the policy for a run,
including compilation and imports. Other language runs retain ordinary
allocation and release. Each kernel implements its own thread-local pool and
restores the previous policy when a nested run finishes.

## Ownership and identity

Cloning a sequence clones its real `Rc`; there is no extra live cache reference
and no adjusted reference count. Only `Rc::get_mut` succeeding at final release
allows recycling. Every element is destroyed before the empty allocation is
put in the pool. Destruction happens outside a pool borrow, allowing nested
tuples to release their storage safely. If an allocation has another strong
owner or a weak owner, it follows ordinary `Rc` destruction instead.

Acquisition removes an empty allocation from the pool, leaving exactly one
strong owner, and fills its existing vector without growing it. Selection uses
the smallest available capacity that can hold the new elements. Both the `Rc`
allocation and element buffer remain actual reused storage. Stack8 tuple
identity now uses the `Rc` allocation, matching its collector node identity;
microcode7 already used that allocation. The existing identity registries are
unchanged and receive these actual addresses. No identity is assigned based on
contents, names, paths, tuple lengths, or a desired test result.

Pools retain at most 256 empty allocations and at most 1 MiB of element
capacity per thread. These are resource bounds, not restrictions on tuple
creation. Tuples outside the reserve budget retain normal ownership and
allocation behavior. The pool is cleared when the outer run ends.

The collector still traverses tuple elements and instance/container edges.
Live sequence references are counted by the same real `Rc` as before. Empty
pooled storage has no Python references, so it cannot keep a payload, callback,
or cyclic graph alive. No generator ownership or resurrection behavior changes.

## Evidence

The rejected f005ea70f5 Lambda run failed tuple reuse in `test_genexps` on
stack8: nine observed tuple identities could be the same while the tenth was
different. Independent identity-registry growth and mixed allocation probes
reproduced this under the default allocator. Changing allocator cache counts
also exposed storage-reuse failures in the base version and microcode7.
Immediately before this correction, independent probes again reproduced
failures in final stack8 and microcode7 under those allocator settings. CPython
3.11 and 3.14 reference probes kept stable tuple reuse and correct live values.

The ownership unit tests check actual `Rc` strong counts and both allocation
addresses, retained live aliases, payload release, and dead-storage reuse.
Python probes cover distinct retained tuples, contents after recycling,
`tuple(existing)` aliases, formatting, equality, hashing, dict/set membership,
ordering, concatenation, repetition, slicing, iterator aliases, weak payload
callbacks, and nested tuple/container cycles. Pressure probes vary prior tuple,
string, object, and container allocations and identity-registry growth, using
both default allocation and allocator cache counts 0, 1, and 7 with freed-memory
perturbation. Probe inputs and logs are excluded from version control; CPython
fixtures are unchanged.

The corrected allocator matrix completed four processes per setting and kernel,
4,864 reuse sequences per kernel, without reuse failures. Retained tuple values
and all weak payload release/cycle checks stayed correct. Both kernels passed
the bounded whole-file genexps check and retained the required namespace test
progress lines. The 15 tuple scratch records and builtin scratch record matched
without edits. Focused PHP and Lumen array copy/alias probes matched base output.
