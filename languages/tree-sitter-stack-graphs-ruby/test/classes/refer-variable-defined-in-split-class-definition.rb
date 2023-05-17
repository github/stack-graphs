class A
    FOO = 42
end

class A
    BAR = FOO
    #     ^ defined: 2
end

  A::FOO
# ^ defined: 1, 5
#    ^ defined: 2

  A::BAR
# ^ defined: 1, 5
#    ^ defined: 6
