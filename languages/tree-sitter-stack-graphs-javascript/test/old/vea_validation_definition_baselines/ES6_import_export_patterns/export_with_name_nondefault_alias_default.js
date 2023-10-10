/*--- path: a.js ---*/

export function foo() { }

/*--- path: index.js ---*/

export { foo as default } from "./a.js";

/*--- path: index2.js ---*/

import bar from "./index.js";
//     ^ defined: 3, 7