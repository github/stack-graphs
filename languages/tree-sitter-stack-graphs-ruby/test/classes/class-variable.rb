class A
    FOO = 42
end

class FOO
end

  A::FOO
# ^ defined: 1
#    ^ defined: 2

  FOO
# ^ defined: 5
