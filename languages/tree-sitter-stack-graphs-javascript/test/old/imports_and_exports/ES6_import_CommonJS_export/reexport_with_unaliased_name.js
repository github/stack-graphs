/*--- path: a.js ---*/

exports.foo = function () { };
/*--- path: index.js ---*/

let mod = require("./a.js");
exports.foo = mod.foo;

/*--- path: index2.js ---*/

import { foo } from "./index.js";

/**/ foo;
//   ^ defined: 3, 7, 11

/*--- path: index3.js ---*/

let { foo } = await import("./index.js");

/**/ foo;
//   ^ defined: 3, 7, 18
