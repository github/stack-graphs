/*--- path: a.js ---*/

export let foo = 1;

/*--- path: b.js ---*/

/**/ bar;
//   ^ defined: 3, 10

import { foo as bar } from "./a.js";