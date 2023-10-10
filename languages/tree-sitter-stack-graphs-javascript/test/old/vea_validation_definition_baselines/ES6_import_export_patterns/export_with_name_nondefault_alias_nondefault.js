/*--- path: a.js ---*/

export function foo() { }

/*--- path: index.js ---*/

export { foo as bar } from "./a.js";

/*--- path: index2.js ---*/

import { bar } from "./index";
//       ^ defined: 3, 7