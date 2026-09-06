// Ported from examples/lumen/constructs/functions_basic.lm by scripts/port_examples.py; edit the Lumen original, not this file.
function square(x: integer): integer;
begin
    square := x * x;
end;

function add(a: integer; b: integer): integer;
begin
    add := a + b;
end;

function greet(name: string): string;
begin
    greet := 'Hello, ' + name;
end;

function get_constant(): integer;
begin
    get_constant := 42;
end;

function compute(x: integer; y: integer): integer;
var sum: integer;
var product: integer;
begin
    sum := x + y;
    product := x * y;
    compute := sum + product;
end;

begin
    writeln('Test: Basic Functions');
    writeln(square(5));
    writeln(add(10, 20));
    writeln(greet('Lumen'));
    writeln(get_constant());
    writeln(compute(3, 4));
end.
