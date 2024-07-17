class A
    class B
        FOO = 42
    end
end

class A
    class B
        BAR = 11
    end
end

  A::B::FOO
# ^ defined: 1, 7
#    ^ defined: 2, 8
#       ^ defined: 3

  A::B::BAR
# ^ defined: 1, 7
#    ^ defined: 2, 8
#       ^ defined: 9
