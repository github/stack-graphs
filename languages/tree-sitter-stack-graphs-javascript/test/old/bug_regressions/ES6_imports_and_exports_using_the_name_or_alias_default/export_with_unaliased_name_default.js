/*--- path: a.js ---*/

export default 1;

/*--- path: b.js ---*/

export { default } from "./a.js";

/*--- path: c.js ---*/

import foo from "./b.js";

/**/ foo;
//   ^ defined: 3, 7, 11
