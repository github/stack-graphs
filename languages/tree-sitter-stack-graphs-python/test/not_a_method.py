class Foo:
    a = 1

    # Self can be named anything
    def first_method(actually_self):
        return actually_self.a
        #                    ^ defined: 2

    def second_method(self):
        # First argument here is not self
        def not_a_method(not_self):
            return not_self.a
            #               ^ defined:
