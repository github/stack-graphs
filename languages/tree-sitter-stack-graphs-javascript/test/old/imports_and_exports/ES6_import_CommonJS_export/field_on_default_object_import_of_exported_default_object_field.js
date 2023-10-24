/*--- path: a.js ---*/

module.exports.foo = 1;

/*--- path: b.js ---*/

import { foo } from "./a.js";

/**/ foo;
//   ^ defined: 3, 7