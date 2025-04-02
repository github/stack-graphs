class A:
    def __init_subclass__(cls, foo):
        pass

    def __init__(self):
        self.some_attr = 2

    def some_method(self):
        print self

class B(A, foo="Bar"):
    def method2(self):
        print self.some_attr, self.some_method()
        #          ^ defined: 6
        #                          ^ defined: 8, 17

    def some_method(self):
        pass

    def other(self):
        super().some_method()
        #       ^ defined: 8
