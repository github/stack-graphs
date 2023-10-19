/*--- path: a.js ---*/

export default function foo() { }

/*--- path: index.js ---*/

import { default as bar } from "./a.js";

export { bar };

/*--- path: index2.js ---*/

let { bar } = await import("./index.js");

/**/ bar;
//   ^ defined: 3, 7, 9, 13
