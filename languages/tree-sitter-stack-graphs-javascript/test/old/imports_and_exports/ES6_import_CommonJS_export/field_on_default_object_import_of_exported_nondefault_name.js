/*--- path: index.js ---*/

exports.foo = function () { };

/*--- path: index2.js ---*/

import mod from "./index.js";

mod.foo;
//  ^ defined: 3
