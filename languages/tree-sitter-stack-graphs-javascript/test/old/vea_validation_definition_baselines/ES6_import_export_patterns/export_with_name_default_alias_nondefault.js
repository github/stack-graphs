/*--- path: a.js ---*/

export default function foo() { }

/*--- path: index.js ---*/

export { default as bar } from "./a.js";

// --- path: index2.js ---

import { bar } from "./index.js";
//       ^ defined: 3, 7