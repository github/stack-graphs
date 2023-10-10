/*--- path: a.js ---*/

export default function foo() { }

/*--- path: index.js ---*/

import { default as foo } from "./a.js";

export { foo };

/*--- path: index2.js ---*/

import { foo } from "./index.js";
//       ^ defined: 3, 7, 9