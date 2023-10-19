/*--- path: index.js ---*/

module.exports = function foo() { };

/*--- path: index2.js ---*/

import bar from "./index.js";

/**/ bar;
//   ^ defined: 3, 7

/*--- path: index3.js ---*/

let bar = await import("./index.js");

/**/ bar.default;
//   ^ defined: 14
//       ^ defined: 3
