// Ported from examples/lumen/constructs/scope_call.lm by scripts/port_examples.py; edit the Lumen original, not this file.
var k: integer;

procedure show();
begin
    writeln(k);
end;

procedure caller();
var k: integer;
begin
    k := 5;
    show();
    writeln(k);
end;

begin
    k := 1;
    caller();
    writeln(k);
    show();
end.
