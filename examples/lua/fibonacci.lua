function fib(n)
  local a = 0
  local b = 1
  local i = 0
  while i < n do
    local c = a + b
    a = b
    b = c
    i = i + 1
  end
  return a
end

local i = 0
while i < 10 do
  print(fib(i))
  i = i + 1
end
