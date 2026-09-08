// PHP's exception classes, written in PHP because they are PHP's and not
// the kernel's: any kernel that reads the class labels gets them from
// here. Hand-written; scripts/port_examples.py leaves native/ alone.
// Throwable stands where PHP has an interface, so that catching it takes
// both lines and catching one line takes only that line.

class Throwable {
    public $message = "";
    public $code = 0;
    public $previous = null;
    public $file = "";
    public $line = 0;
    public $trace = array();
    public function __construct($message = "", $code = 0, $previous = null) {
        $this->message = $message;
        $this->code = $code;
        $this->previous = $previous;
        // The calls a fault was raised under are the calls as they stood
        // before it was made: the making itself, and the making of every
        // class it is built on, are none of them. A maker belonging to
        // some other class is a call like any other and stays.
        $under = __calls();
        $made_at = null;
        while (count($under) > 0) {
            $frame = $under[0];
            if ($frame['function'] !== '__construct' || !isset($frame['class'])) { break; }
            $of = $frame['class'];
            if (!($this instanceof $of)) { break; }
            $made_at = array_shift($under);
        }
        // Where a fault says it was raised is where it was made, which
        // is where the outermost of those makers was called from.
        if ($made_at !== null) {
            $this->file = isset($made_at['file']) ? $made_at['file'] : '';
            $this->line = isset($made_at['line']) ? $made_at['line'] : 0;
        }
        $this->trace = $under;
    }
    public function getMessage() { return $this->message; }
    public function getCode() { return $this->code; }
    public function getPrevious() { return $this->previous; }
    public function getFile() { return $this->file; }
    public function getLine() { return $this->line; }
    public function getTrace() { return $this->trace; }
    public function getTraceAsString() {
        $lines = array();
        $at = 0;
        foreach ($this->trace as $frame) {
            $lines[] = '#' . $at . ' ' . __frame_told($frame);
            $at = $at + 1;
        }
        $lines[] = '#' . $at . ' {main}';
        return implode("\n", $lines);
    }
    public function __toString() { return $this->message; }
}

class Exception extends Throwable {}
class Error extends Throwable {}
class TypeError extends Error {}
class ValueError extends Error {}
class ArithmeticError extends Error {}
class DivisionByZeroError extends ArithmeticError {}
class ArgumentCountError extends TypeError {}
class AssertionError extends Error {}
class UnhandledMatchError extends Error {}
class ErrorException extends Exception {}
class RuntimeException extends Exception {}
class LogicException extends Exception {}
class InvalidArgumentException extends LogicException {}
class DomainException extends LogicException {}
class LengthException extends LogicException {}
class OutOfRangeException extends LogicException {}
class OutOfBoundsException extends RuntimeException {}
class RangeException extends RuntimeException {}
class OverflowException extends RuntimeException {}
class UnderflowException extends RuntimeException {}
class UnexpectedValueException extends RuntimeException {}
class JsonException extends Exception {}

// The class a program makes a plain thing of, with whatever properties
// it writes into it and nothing of its own.
class stdClass {}
