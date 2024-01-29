# --- path: foo/__init__.py ---

FOO = 42

# --- path: foo/bar/test.py ---

from .. import FOO

FOO
# ^ defined: 7, 3
