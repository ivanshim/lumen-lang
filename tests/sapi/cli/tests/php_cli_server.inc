<?php
// Starting this implementation as a web server, so that a test may ask
// it something over a connection and read what it answers. A test that
// wants one requires this file and calls php_cli_server_start; what it
// gets back is a server already answering, an address written down under
// PHP_CLI_SERVER_ADDRESS, and a promise that the server will be stopped
// when the run ends however the run ends.
//
// This is the same support the reference implementation keeps under the
// same name in its own tree, written again for this one. It knows
// nothing of any particular test and asks nothing of one.
//
// THE PLACE LEFT FOR THE KERNEL.
//
// Two things here are the kernel's to give, and until they are given
// php_cli_server_start says plainly that it cannot start a server and
// answers false. Neither is written yet.
//
//   label      ext.builtin.run.begin
//   name       __run_begin          (what langs/php.json binds it to)
//   handed     $program  the program to start, as bytes
//              $args     the words to hand it, a list of bytes
//              $named    names to set in its environment, a map of a
//                        name to what it is to be set to
//   answers    a whole number standing for the run, by which it may be
//              stopped again; false where the program could not be
//              started at all. What the run writes goes nowhere: this
//              is for starting a thing that answers on a connection,
//              not for reading a program's output.
//
//   label      ext.builtin.run.end
//   name       __run_end
//   handed     $which    the number a run was begun under
//   answers    true where the run was stopped, false where there was no
//              such run to stop
//
// Waiting until the server answers is done with __net_ask, which
// langs/lib_php/native/network.php asks for under the label
// ext.builtin.net.ask and says the shape of.

// How many ports to try before giving up, and the first of them. A port
// may be named outright in the environment, as the reference lets one be.
const PHP_CLI_SERVER_HOSTNAME = "127.0.0.1";
const PHP_CLI_SERVER_PORT_FIRST = 8964;
const PHP_CLI_SERVER_PORT_TRIES = 100;

$php_cli_server_running = null;

// Whether the run can start a server and reach one. Both are wanted:
// starting one that cannot be reached tells a test nothing.
function php_cli_server_available() {
    return function_exists("__run_begin") && function_exists("__run_end") && function_exists("__net_ask");
}

// Whether something is already answering on a port. A port nothing
// answers on is one this run may take.
function php_cli_server_answers($port) {
    $sent = "GET / HTTP/1.1\r\nHost: " . PHP_CLI_SERVER_HOSTNAME . ":" . $port . "\r\nConnection: close\r\n\r\n";
    return __net_ask(PHP_CLI_SERVER_HOSTNAME, $port, $sent, 1) !== false;
}

// A port nothing is answering on, looked for from the first upward. A
// port named in the environment is taken as it stands, since whoever
// named it meant that one.
function php_cli_server_free_port() {
    $said = getenv("PHP_CLI_SERVER_PORT");
    if ($said !== false && $said !== "") { return (int)$said; }
    $port = PHP_CLI_SERVER_PORT_FIRST;
    $tried = 0;
    while ($tried < PHP_CLI_SERVER_PORT_TRIES) {
        if (!php_cli_server_answers($port)) { return $port; }
        $port = $port + 1;
        $tried = $tried + 1;
    }
    return $port;
}

// The settings a test asks for, written as the command line of the
// reference writes them — a -d before each, and a name=value after it —
// turned into the names this implementation reads them under. A word
// that is none of these is let be, since a test may hand along whatever
// the reference would have taken.
function php_cli_server_settings($args) {
    $named = array();
    if (!is_array($args)) { return $named; }
    $at = 0;
    $count = count($args);
    while ($at < $count) {
        if ($args[$at] === "-d" && $at + 1 < $count) {
            $said = (string)$args[$at + 1];
            $split = strpos($said, "=");
            if ($split !== false) {
                $named["PHP_INI_" . substr($said, 0, $split)] = substr($said, $split + 1);
            }
            $at = $at + 2;
            continue;
        }
        $at = $at + 1;
    }
    return $named;
}

// Start a server answering the given program, and wait until it answers.
// The program is written to a file beside the test, as the reference
// writes it beside its own, and the file is named .php so that the run
// reads it as PHP without being told to.
//
// Answers true where a server is up and answering, and false where none
// could be started — having said why.
// The switches the run was started with beyond the program itself, as
// the runner named them. Nothing where it named none.
function php_cli_server_passed_on() {
    $said = getenv("TEST_PHP_ARGS");
    if ($said === false || trim($said) === "") { return array(); }
    $out = array();
    foreach (preg_split('/\s+/', trim($said)) as $one) {
        if ($one !== "") { $out[] = trim($one, "'\""); }
    }
    return $out;
}

function php_cli_server_start($code = 'echo "Hello world";', $router = null, $args = array()) {
    global $php_cli_server_running;
    if (!php_cli_server_available()) {
        echo "cannot start a server: this run has no way to begin one, or no way to reach one\n";
        return false;
    }
    $where = __DIR__ . "/php_cli_server_served.php";
    if ($router !== null && $router !== "") { $where = __DIR__ . "/" . $router; }
    file_put_contents($where, "<?php\n" . $code . "\n");
    $port = php_cli_server_free_port();
    $address = PHP_CLI_SERVER_HOSTNAME . ":" . $port;
    $named = php_cli_server_settings($args);
    // The switches this run was itself started with are handed on, so
    // that the server reads its program under the same definition. The
    // reference's own runner passes its extra switches along the same
    // way, under the same name.
    $carried = array("--serve", $address);
    foreach (php_cli_server_passed_on() as $one) { $carried[] = $one; }
    $carried[] = $where;
    $begun = __run_begin(PHP_BINARY, $carried, $named);
    if ($begun === false) {
        echo "cannot start a server: the program would not begin\n";
        return false;
    }
    $php_cli_server_running = array($begun, $where);
    // A server takes a moment to take its port. It is asked after until
    // it answers, and given up on where it never does.
    $tries = 0;
    while ($tries < 200) {
        if (php_cli_server_answers($port)) {
            if (!defined("PHP_CLI_SERVER_ADDRESS")) { define("PHP_CLI_SERVER_ADDRESS", $address); }
            if (!defined("PHP_CLI_SERVER_PORT")) { define("PHP_CLI_SERVER_PORT", $port); }
            if (function_exists("register_shutdown_function")) {
                register_shutdown_function("php_cli_server_stop");
            }
            return true;
        }
        usleep(25000);
        $tries = $tries + 1;
    }
    php_cli_server_stop();
    echo "cannot start a server: it never answered on " . $address . "\n";
    return false;
}

// Stop the server this run started, and take away the file it was
// serving. Stopping one that was never started is nothing at all, so
// that this may be called at the end of a run whatever happened.
function php_cli_server_stop() {
    global $php_cli_server_running;
    if ($php_cli_server_running === null) { return true; }
    $running = $php_cli_server_running;
    $php_cli_server_running = null;
    __run_end($running[0]);
    if (file_exists($running[1])) { unlink($running[1]); }
    return true;
}
