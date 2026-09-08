// The classes of method names a thing answers to when it walks itself,
// written in PHP because they are PHP's and not the kernel's. The kernel
// is told only which class stands for each of them and what the methods
// of the walk are called; the classes themselves belong here.
// Hand-written; scripts/port_examples.py leaves native/ alone.

interface Traversable {}

// A thing that is its own walk: wound back once, then asked whether
// there is more, what stands here, what it is called, and to step on.
interface Iterator extends Traversable {
    function current();
    function key();
    function next();
    function rewind();
    function valid();
}

// A thing that hands over another to be walked in its stead.
interface IteratorAggregate extends Traversable {
    function getIterator();
}
