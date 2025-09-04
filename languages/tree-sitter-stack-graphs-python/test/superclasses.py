class A:
    def __init__(self):
        self.some_attr = 2

    def some_method(self):
        print self

class B(A):
    def method2(self):
        print self.some_attr, self.some_method()
        #          ^ defined: 3
        #                          ^ defined: 5, 14

    def some_method(self):
        pass

    def other(self):
        super().some_method()
        #       ^ defined: 5

    def __init_subclass__(cls, **kwargs):
        cls._c = kwargs.pop("c", True)
        super().__init_subclass__(**kwargs)

class C(B, c=False):
    def subclass(self):
        self.some_method()
        #    ^ defined: 5, 14
