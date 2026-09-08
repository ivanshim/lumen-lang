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

// A walk over an array: the keys are taken once, at the start, so what
// the walk hands out is what the array held when the walk began.
class ArrayIterator implements Iterator {
    private $items;
    private $keys;
    private $at;
    function __construct($array = array()) {
        $this->items = $array;
        $this->keys = array_keys($array);
        $this->at = 0;
    }
    function current() { return $this->items[$this->keys[$this->at]]; }
    function key() { return $this->keys[$this->at]; }
    function next() { $this->at = $this->at + 1; }
    function rewind() { $this->at = 0; }
    function valid() { return $this->at < count($this->keys); }
}

// A walk that never ends: when the walk it stands on runs out, it is
// wound back and goes round again.
class InfiniteIterator implements Iterator {
    private $inner;
    function __construct($inner) { $this->inner = $inner; }
    function current() { return $this->inner->current(); }
    function key() { return $this->inner->key(); }
    function next() {
        $this->inner->next();
        if (!$this->inner->valid()) { $this->inner->rewind(); }
    }
    function rewind() { $this->inner->rewind(); }
    function valid() { return $this->inner->valid(); }
}

