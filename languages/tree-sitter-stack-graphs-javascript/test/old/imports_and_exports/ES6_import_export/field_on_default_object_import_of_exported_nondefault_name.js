/*--- path: index.js ---*/

export function foo() { }

/*--- path: index2.js ---*/

import mod from "./index.js";

mod.foo;
//  ^ defined:
