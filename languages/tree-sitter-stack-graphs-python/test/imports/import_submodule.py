# --- path: foo/bar.py ---

BAR = 42

# --- path: test.py ---

import foo.bar

foo.bar.BAR
# ^ defined: 7
#     ^ defined: 7, 3
#         ^ defined: 3

bar
# ^ defined:
