<?php
// The tests under sapi/cli are written for the command-line way of
// running a program, and say nothing about any other. A run that is not
// that one passes them over.
if (php_sapi_name() != "cli") {
    die("skip CLI only");
}
