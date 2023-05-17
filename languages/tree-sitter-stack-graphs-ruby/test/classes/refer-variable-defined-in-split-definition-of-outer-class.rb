class A
    FOO = 42
end

class A
    class B
        BAR = FOO
        #     ^ defined: 2
    end
end

  A::FOO
# ^ defined: 1, 5
#    ^ defined: 2

  A::B::BAR
# ^ defined: 1, 5
#    ^ defined: 6
#       ^ defined: 7
