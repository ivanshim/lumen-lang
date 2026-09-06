print(1 + 2 * 3)

local x = 0
local y = 5

if x < y and y == 5 then
  print(100)
elseif x == y then
  print(150)
else
  print(200)
end

local i = 0
local sum = 0

while i < 10 do
  if i ~= 5 then
    if i == 8 then
      break
    end
    sum = sum + i
    print(sum)
  end
  i = i + 1
end

print(sum)
print(true)
print(false)
print(not false)
print(-10 + 3)
print(0x1F)
print(7 // 2)
print(2 ^ 10)
print("con" .. "cat")
--[[ A block
     comment. ]]
print("done")
