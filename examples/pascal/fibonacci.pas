var a: integer;
var b: integer;
var c: integer;
var i: integer;
begin
  a := 0;
  b := 1;
  i := 0;
  while i < 10 do begin
    writeln(a);
    c := a + b;
    a := b;
    b := c;
    i := i + 1;
  end;
end.
