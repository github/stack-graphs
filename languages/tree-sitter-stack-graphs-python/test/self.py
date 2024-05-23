class Foo:
    a = 1

    def mathod_1(self):
        return self.a
        #           ^ defined: 2

    # Self can be named anything
    def method_2(actually_self):
        return actually_self.a
        #                    ^ defined: 2
