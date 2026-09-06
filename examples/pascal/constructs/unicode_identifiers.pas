// Ported from examples/lumen/constructs/unicode_identifiers.lm by scripts/port_examples.py; edit the Lumen original, not this file.
var café: integer;
var π: real;
var 数: integer;

function größe(x: integer): integer;
begin
    größe := x + 1;
end;

begin
    café := 3;
    π := 22 / 7;
    数 := café * 2;
    writeln(café);
    writeln(π);
    writeln(数);
    writeln(größe(数));
end.
