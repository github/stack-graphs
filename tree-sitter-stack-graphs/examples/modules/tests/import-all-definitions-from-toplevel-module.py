# --- path: foo.py ---

def bar():
    pass

# --- path: test.py ---

from foo import *

bar()
# ^ defined: 3
