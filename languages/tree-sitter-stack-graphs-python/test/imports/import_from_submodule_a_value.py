# --- path: foo/bar.py ---

BAR = 42

# --- path: test.py ---

from foo.bar import BAR

BAR
# ^ defined: 7, 3

foo
# ^ defined:

bar
# ^ defined:
