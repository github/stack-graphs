#------ path: one/two.py ------#

a = 1
b = 2

#------ path: one/three.py ------#

b = 3
c = 4

#------ path: main.py ---------#

from one.two import *
from one.three import *

print a
#     ^ defined: 3
print b
#     ^ defined: 8
print c
#     ^ defined: 9
