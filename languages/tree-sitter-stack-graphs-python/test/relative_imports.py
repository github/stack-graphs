#------ path: foo/bar/main.py ------#

from . import a
from .b import B
from ..c import see
from ..c.d import D

print a.A
#     ^ defined: 3, 25
#       ^ defined: 26

print B.bee
#     ^ defined: 4, 31
#       ^ defined: 32

print see()
#     ^ defined: 5, 37


print D.d
#       ^ defined: 44

#------ path: foo/bar/a.py --------#

# module
A = "a"

#------ path: foo/bar/b.py --------#

# module
class B:
    bee = 1

#------ path: foo/c.py ------------#

# module
def see():
    pass

#------ path: foo/c/d.py ---------#

# module
class D:
    d = "d"

#------ path: foo/e/g.py ---#

# module
G = 1

#------ path: foo/e/__init__.py ---#

# module
from .g import G
#              ^ defined: 49

from ..c import see
#               ^ defined: 37

E = 1
