// Ported from examples/lumen/constructs/pipe_operator.lm by scripts/port_examples.py; edit the Lumen original, not this file.
var result: integer;
var x: integer;

function double(x: integer): integer;
begin
    double := x * 2;
end;

function add_one(x: integer): integer;
begin
    add_one := x + 1;
end;

function square(x: integer): integer;
begin
    square := x * x;
end;

function multiply(a: integer; b: integer): integer;
begin
    multiply := a * b;
end;

begin
    writeln('Test: Pipe Operator');
    writeln('Without pipe: square(add_one(double(5)))');
    writeln(square(add_one(double(5))));
    writeln('With pipe: 5 |> double() |> add_one() |> square()');
    result := square(add_one(double(5)));
    writeln(result);
    writeln('10 |> double():');
    writeln(double(10));
    writeln('3 |> double():');
    x := double(3);
    writeln(multiply(x, 2));
end.
