module M
    FOO = 42
end

module M
    BAR = FOO
    #     ^ defined: 2
end

  M::FOO
# ^ defined: 1, 5
#    ^ defined: 2

  M::BAR
# ^ defined: 1, 5
#    ^ defined: 6
