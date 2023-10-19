let x = 1;

// Flow Out

import * as y from "mod";
import { z } from "mod";
import { y as w } from "mod";
import q from "mod";

/**/ y;
//   ^ defined: 5

/**/ z;
//   defined: 6

/**/ w;
//   defined: 7

/**/ q;
//   defined: 8

// Flow Around

/**/ x;
//   ^ defined: 1