#------ path: a.py ------#

def foo(x):
    return x

#------ path: b.py ------#

class A:
    bar = 1

#------ path: main.py ---------#

from a import *
from b import *

foo(A).bar
#      ^ defined: 9
