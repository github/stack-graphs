# --- path: foo/bar.py ---

BAR = 42

# --- path: test.py ---

from foo import bar

bar.BAR
# ^ defined: 7, 3
#     ^ defined: 3

foo
# ^ defined:
