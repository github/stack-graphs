# --- path: foo/bar.py ---

def baz():
    pass

# --- path: test.py ---

from foo.bar import baz

baz()
# ^ defined: 3
