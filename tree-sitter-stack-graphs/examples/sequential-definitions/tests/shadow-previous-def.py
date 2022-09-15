def foo():
    pass

bar()
# ^ defined:

def bar():
    pass

foo()
# ^ defined: 1

def foo():
    pass

foo()
# ^ defined: 13
