// What a stream is opened with beside its name: what each wrapper is to
// be told, and what the program wants to hear back while the stream is
// read. Written in PHP because they are PHP's, not the kernel's.
// Hand-written; scripts/port_examples.py leaves native/ alone.

// A context is the number it is filed under, drawn from the same count a
// stream's handle is drawn from, so that a number stands for a stream or
// for a context and never for both. What is held under it is what each
// wrapper was told, under the wrapper's name, and beside that whatever
// else the program set: the one thing a program may set there is who to
// tell of what happens, which nothing in a run of this kind sends for.
$__contexts = array();
// The context every stream opened without one is opened with. It is made
// the first time the program asks after it and stands for the whole run.
$__context_stands = null;

// The short name the reference gives a kind when it says what it was
// handed, which is not always the name gettype gives it.
function __kind_briefly($value) {
    $kind = gettype($value);
    if ($kind === "integer") { return "int"; }
    if ($kind === "double") { return "float"; }
    if ($kind === "boolean") { return "bool"; }
    if ($kind === "NULL") { return "null"; }
    if ($kind === "object") { return get_class($value); }
    return $kind;
}

// A context of its own, holding what it was made with and nothing else.
function __context_made($options) {
    global $__contexts, $__stream_next;
    $which = $__stream_next;
    $__stream_next = $which + 1;
    $__contexts[$which] = array("options" => $options, "told" => array());
    return $which;
}
function __context_is($which) {
    global $__contexts;
    if (!is_int($which)) { return false; }
    return array_key_exists($which, $__contexts);
}
// The context a resource stands for. A program may name a context
// outright or name a stream, and a stream that has been asked after
// keeps a context of its own from then on, empty until something is set
// in it: no wrapper a run of this kind has looks in one.
function __context_named($said, $where, $which) {
    if (__context_is($which)) { return $which; }
    $stream = __stream_ready($which);
    if ($stream !== null) {
        if (array_key_exists("context", $stream)) { return $stream["context"]; }
        $made = __context_made(array());
        __stream_put($which, "context", $made);
        return $made;
    }
    throw new TypeError($said . '(): Argument #1 ($' . $where . ') must be of type resource, ' . __kind_briefly($which) . ' given');
}
// The context a call was handed where it was handed one, and nothing
// where it was handed none. A stream is a resource but not this one, and
// is turned down in words of its own.
function __context_given($said, $where, $number, $which) {
    if ($which === null) { return null; }
    if (__context_is($which)) { return $which; }
    if (__stream_at($which) !== null) {
        throw new TypeError($said . '(): supplied resource is not a valid Stream-Context resource');
    }
    throw new TypeError($said . '(): Argument #' . $number . ' ($' . $where . ') must be of type resource or null, ' . __kind_briefly($which) . ' given');
}

// Options as the reference will have them: a wrapper's name against the
// things that wrapper is to be told, each under a name of its own. A
// wrapper named by anything but a word, or told anything but a list of
// things, is turned down and nothing at all is set. A thing named by
// anything but a word is quietly let be, and a wrapper left with nothing
// to be told is not written down at all.
function __context_options_told($options) {
    $out = array();
    foreach ($options as $wrapper => $told) {
        if (!is_string($wrapper) || !is_array($told)) {
            throw new ValueError('Options should have the form ["wrappername"]["optionname"] = $value');
        }
        $kept = array();
        foreach ($told as $name => $value) {
            if (is_string($name)) { $kept[$name] = $value; }
        }
        if (count($kept) > 0) { $out[$wrapper] = $kept; }
    }
    return $out;
}
// Options set in a context, beside whatever it was told before: a
// wrapper already spoken for keeps what it holds save where the same
// thing is named again.
function __context_options_put($which, $options) {
    global $__contexts;
    $context = $__contexts[$which];
    $held = $context["options"];
    foreach ($options as $wrapper => $told) {
        $already = array_key_exists($wrapper, $held) ? $held[$wrapper] : array();
        foreach ($told as $name => $value) { $already[$name] = $value; }
        $held[$wrapper] = $already;
    }
    $context["options"] = $held;
    $__contexts[$which] = $context;
    return true;
}
// What a program sets in a context beside the wrappers' own options.
// Options named here are set as they would be set outright; who to tell
// of what happens is written down and answered with again; anything else
// is nothing this run knows of and is quietly let be.
function __context_told_put($which, $params) {
    global $__contexts;
    foreach ($params as $name => $value) {
        if ($name === "options") {
            if (!is_array($value)) { throw new TypeError('Invalid stream/context parameter'); }
            __context_options_put($which, __context_options_told($value));
        } elseif ($name === "notification") {
            $context = $__contexts[$which];
            $held = $context["told"];
            $held["notification"] = $value;
            $context["told"] = $held;
            $__contexts[$which] = $context;
        }
    }
    return true;
}
// The context every stream opened without one is opened with.
function __context_standing() {
    global $__context_stands;
    if ($__context_stands === null) { $__context_stands = __context_made(array()); }
    return $__context_stands;
}

function stream_context_create($options = null, $params = null) {
    if ($options !== null && !is_array($options)) {
        throw new TypeError('stream_context_create(): Argument #1 ($options) must be of type ?array, ' . __kind_briefly($options) . ' given');
    }
    if ($params !== null && !is_array($params)) {
        throw new TypeError('stream_context_create(): Argument #2 ($params) must be of type ?array, ' . __kind_briefly($params) . ' given');
    }
    $told = $options === null ? array() : __context_options_told($options);
    $which = __context_made($told);
    if ($params !== null) { __context_told_put($which, $params); }
    return $which;
}
function stream_context_get_options($which) {
    global $__contexts;
    $context = __context_named("stream_context_get_options", "stream_or_context", $which);
    return $__contexts[$context]["options"];
}
// A whole list of things the wrappers are to be told, set at once.
function stream_context_set_options($which, $options) {
    $context = __context_named("stream_context_set_options", "context", $which);
    if (!is_array($options)) {
        throw new TypeError('stream_context_set_options(): Argument #2 ($options) must be of type array, ' . __kind_briefly($options) . ' given');
    }
    return __context_options_put($context, __context_options_told($options));
}
// One thing a wrapper is to be told, or a whole list of them at once.
// Where the program hands a list it may hand nothing else, and is told
// that the other call is the one for that now; where it names a wrapper
// it must name the thing and give its value both.
function stream_context_set_option($which, $wrapper_or_options, $option = null, $value = null) {
    $given = func_num_args();
    $context = __context_named("stream_context_set_option", "context", $which);
    if ($given == 2) {
        __complaint_say(__complaint_word(E_DEPRECATED), "Calling stream_context_set_option() with 2 arguments is deprecated, use stream_context_set_options() instead");
    }
    if (is_array($wrapper_or_options)) {
        if ($option !== null) {
            throw new ValueError('stream_context_set_option(): Argument #3 ($option_name) must be null when argument #2 ($wrapper_or_options) is an array');
        }
        if ($given > 3) {
            throw new ValueError('stream_context_set_option(): Argument #4 ($value) cannot be provided when argument #2 ($wrapper_or_options) is an array');
        }
        return __context_options_put($context, __context_options_told($wrapper_or_options));
    }
    if ($option === null) {
        throw new ValueError('stream_context_set_option(): Argument #3 ($option_name) cannot be null when argument #2 ($wrapper_or_options) is a string');
    }
    if ($given < 4) {
        throw new ValueError('stream_context_set_option(): Argument #4 ($value) must be provided when argument #2 ($wrapper_or_options) is a string');
    }
    return __context_options_put($context, array((string)$wrapper_or_options => array((string)$option => $value)));
}
function stream_context_set_params($which, $params) {
    $context = __context_named("stream_context_set_params", "context", $which);
    if (!is_array($params)) {
        throw new TypeError('stream_context_set_params(): Argument #2 ($params) must be of type array, ' . __kind_briefly($params) . ' given');
    }
    return __context_told_put($context, $params);
}
// What a context holds, options and all: who to tell of what happens
// stands first where the program ever set one, as the reference has it
// stand, and the wrappers' options last under a name of their own.
function stream_context_get_params($which) {
    global $__contexts;
    $context = __context_named("stream_context_get_params", "context", $which);
    $held = $__contexts[$context];
    $out = $held["told"];
    $out["options"] = $held["options"];
    return $out;
}
// The standing context, with whatever the program named set in it. Both
// of these set beside what is already there rather than in its place,
// which is what the reference does with them.
function stream_context_get_default($options = null) {
    if ($options !== null && !is_array($options)) {
        throw new TypeError('stream_context_get_default(): Argument #1 ($options) must be of type ?array, ' . __kind_briefly($options) . ' given');
    }
    $which = __context_standing();
    if ($options !== null) { __context_options_put($which, __context_options_told($options)); }
    return $which;
}
function stream_context_set_default($options) {
    if (!is_array($options)) {
        throw new TypeError('stream_context_set_default(): Argument #1 ($options) must be of type array, ' . __kind_briefly($options) . ' given');
    }
    $which = __context_standing();
    __context_options_put($which, __context_options_told($options));
    return $which;
}
