def foo():
    def bar():
        pass
    bar()
    # ^ defined: 2

foo()
# ^ defined: 1

bar()
# ^ defined: 
