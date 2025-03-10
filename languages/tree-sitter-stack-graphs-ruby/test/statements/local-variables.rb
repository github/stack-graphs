foo = 42

module M
    bar = foo
    #     ^ defined: 1
    bar
    # ^ defined: 4
end

foo
# ^ defined: 1

bar
# ^ defined:
