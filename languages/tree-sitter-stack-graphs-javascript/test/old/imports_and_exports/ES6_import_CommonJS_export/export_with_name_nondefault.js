/*--- path: index.js ---*/

exports.foo = function () { };

/*--- path: index2.js ---*/

import { foo } from "./index.js";

/**/ foo;
//   ^ defined: 3, 7

/*--- path: index3.js ---*/

let { foo } = await import("./index.js");

/**/ foo;
//   ^ defined: 3, 14
