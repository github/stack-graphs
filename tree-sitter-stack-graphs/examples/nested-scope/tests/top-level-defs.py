def foo():
    bar()
    # ^ defined: 5

def bar():
    pass

foo()
# ^ defined: 1
