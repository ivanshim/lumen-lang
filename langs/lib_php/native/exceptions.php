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
    public function __construct($message = "", $code = 0, $previous = null) {
        $this->message = $message;
        $this->code = $code;
        $this->previous = $previous;
    }
    public function getMessage() { return $this->message; }
    public function getCode() { return $this->code; }
    public function getPrevious() { return $this->previous; }
    public function getFile() { return $this->file; }
    public function getLine() { return $this->line; }
    public function getTrace() { return array(); }
    public function getTraceAsString() { return "#0 {main}"; }
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
