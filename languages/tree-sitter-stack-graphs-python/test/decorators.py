from a import deprecated
from b import ignore_warnings

class A:
    @deprecated
    # ^ defined: 1
    @ignore_warnings.all
    # ^ defined: 2
    def b(self):
        pass
