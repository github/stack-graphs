/*--- path: a.js ---*/

export default function foo() { };

/*--- path: b.js ---*/

export { default as default } from "./a.js";

/*--- path: c.js ---*/

import bar from "./b.js";

/**/ bar;
//   ^ defined: 3, 7, 11
