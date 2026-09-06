// Ported from examples/lumen/constructs/let_keyword.lm by scripts/port_examples.py; edit the Lumen original, not this file.
var x: integer;
var y: integer;
var result: integer;

function test_let(): integer;
var a: integer;
var b: integer;
begin
    a := 42;
    b := 10;
    b := 50;
    test_let := a + b;
end;

begin
    writeln('Test: let and let mut Keywords');
    x := 10;
    writeln('let x = 10');
    writeln(x);
    y := 5;
    writeln('let mut y = 5');
    writeln(y);
    y := 20;
    writeln('After y = 20:');
    writeln(y);
    x := 100;
    writeln('After let x = 100 (shadowing):');
    writeln(x);
    result := x + y;
    writeln('let result = x + y');
    writeln(result);
    writeln('test_let():');
    writeln(test_let());
end.
