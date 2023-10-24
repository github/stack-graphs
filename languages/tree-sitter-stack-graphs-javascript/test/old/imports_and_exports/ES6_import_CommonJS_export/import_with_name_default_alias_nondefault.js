/*--- path: a.js ---*/

module.exports = function foo() { };

/*--- path: index.js ---*/

let mod = require("./a.js");
exports.bar = mod;

/*--- path: index2.js ---*/

import { bar } from "./index.js";

/**/ bar;
//   ^ defined: 3, 7, 8, 12

/*--- path: index3.js ---*/

let { bar } = await import("./index.js");

/**/ bar;
//   ^ defined: 3, 7, 8, 19
