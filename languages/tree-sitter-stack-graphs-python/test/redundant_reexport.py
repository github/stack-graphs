#--- path: a/__init__.py ---#

from . import child

#--- path: a/child.py ----

def f():
    pass

#--- path: main.py ---#

import a

print a.child.f()
#             ^ defined: 7
