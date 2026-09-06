// Ported from examples/lumen/constructs/comments.lm by scripts/port_examples.py; edit the Lumen original, not this file.
var x: integer;
var result: integer;
var value: integer;
var counter: integer;

function add_numbers(a: integer; b: integer): integer;
begin
    add_numbers := a + b;
end;

begin
    writeln('Test: Comments Support');
    x := 42;
    writeln(x);
    result := x * 2;
    writeln(result);
    value := add_numbers(10, 20);
    writeln(value);
    if value > 20 then begin
        writeln('Value is greater than 20');
    end else begin
        writeln('Value is 20 or less');
    end;
    counter := 0;
    while counter < 3 do begin
        writeln(counter);
        counter := counter + 1;
    end;
    writeln('Done testing comments');
end.
