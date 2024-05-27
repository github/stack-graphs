class Foo:
    a = 1

    def first_method(self):
        self.a
        #    ^ defined: 2

    def second_method(self):
        self.a
        #    ^ defined: 2

        # First argument here is not self
        def not_a_method(not_self):
            return not_self.a
            #               ^ defined:


def function(self):
    self.a
    #    ^ defined:
