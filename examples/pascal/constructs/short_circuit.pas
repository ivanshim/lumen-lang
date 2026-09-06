// Ported from examples/lumen/constructs/short_circuit.lm by scripts/port_examples.py; edit the Lumen original, not this file.
var result: boolean;
var x: integer;

function is_even(x: integer): boolean;
begin
    writeln('Checking if ');
    writeln(x);
    writeln(' is even');
    is_even := x mod 2 = 0;
end;

function is_positive(x: integer): boolean;
begin
    writeln('Checking if ');
    writeln(x);
    writeln(' is positive');
    is_positive := x > 0;
end;

procedure safe_check(value: integer);
begin
    if (value <> nil) and (value > 10) then begin
        writeln('Value is not null and greater than 10');
    end else begin
        writeln('Value is null or not greater than 10');
    end;
end;

begin
    writeln('Test: Short-Circuit Evaluation');
    writeln('false and is_even(10):');
    result := false and is_even(10);
    writeln(result);
    writeln('true and is_even(10):');
    result := true and is_even(10);
    writeln(result);
    writeln('true or is_positive(5):');
    result := true or is_positive(5);
    writeln(result);
    writeln('false or is_positive(5):');
    result := false or is_positive(5);
    writeln(result);
    writeln('Testing division by zero avoidance:');
    x := 0;
    if (x <> 0) and (10 / x > 5) then begin
        writeln('Result is greater than 5');
    end else begin
        writeln('x is zero or result is not greater than 5');
    end;
    safe_check(15);
    safe_check(5);
end.
