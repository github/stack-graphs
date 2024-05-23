def outer(a):
    def inner(b, c):
        pass

    inner(1)
    # ^ defined: 2


class A:
    def method(a):
        pass
