def foo():
    pass

def bar():
    def foo():
        pass
    foo()
    # ^ defined: 5

foo()
# ^ defined: 1
