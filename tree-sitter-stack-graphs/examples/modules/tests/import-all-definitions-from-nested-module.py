# --- path: foo/bar.py ---

def baz():
    pass

# --- path: test.py ---

from foo.bar import *

baz()
# ^ defined: 3
