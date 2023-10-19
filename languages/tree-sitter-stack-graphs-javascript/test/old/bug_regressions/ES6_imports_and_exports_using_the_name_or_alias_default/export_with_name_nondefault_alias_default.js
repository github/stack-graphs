/*--- path: a.js ---*/

export let foo = 1;

/*--- path: b.js ---*/

export { foo as default } from "./a.js";

/*--- path: c.js ---*/

import bar from "./b.js";

/**/ bar;
//   ^ defined: 3, 7, 11
