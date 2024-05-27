def outer(a):
    def inner(b):
        pass

    inner(1)
    # ^ defined: 2
