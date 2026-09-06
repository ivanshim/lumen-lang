// Ported from examples/lumen/constructs/string_equality.lm by scripts/port_examples.py; edit the Lumen original, not this file.
var x: string;
var y: string;
var z: string;

begin
    x := 'hello';
    y := 'hello';
    z := 'world';
    writeln(x = y);
    writeln(x = z);
    writeln(x <> z);
end.
